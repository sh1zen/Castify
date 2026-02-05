use crate::decoder::{AudioPlayer, FfmpegDecoder, H264Depacketizer, VideoFrame};
use crate::pipeline::clock::MediaClock;
use crate::pipeline::health::PipelineHealth;
use crate::pipeline::state::PipelineState;
use crate::utils::net::common::find_caster;
use crate::utils::net::webrtc::WebRTCReceiver;
use crate::utils::sos::SignalOfStop;
use crate::utils::{SendResult, try_send};
use crate::workers::WorkerClose;
use crate::workers::save_stream::{SavePacket, SaveStream};
use log::{error, info};
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::time::Instant;
use tokio::sync::{Mutex, mpsc};

/// Return true if the H.264 access unit contains an IDR (nal type 5) or SPS/PPS (7/8).
fn au_contains_idr_or_sps(au: &[u8]) -> bool {
    const START_CODE: &[u8] = &[0, 0, 0, 1];
    let mut i = 0usize;
    while i + 4 <= au.len() {
        // find next start code
        if &au[i..i + 4] == START_CODE {
            let nal_start = i + 4;
            if nal_start >= au.len() {
                break;
            }
            let nal_type = au[nal_start] & 0x1F;
            if nal_type == 5 || nal_type == 7 || nal_type == 8 {
                return true;
            }
            i = nal_start;
        } else {
            i += 1;
        }
    }
    false
}

pub struct Receiver {
    is_streaming: Arc<AtomicBool>,
    audio_muted: Arc<AtomicBool>,
    save_stream: Option<SaveStream>,
    caster_addr: Option<SocketAddr>,
    /// Canale usato dal SaveStream per ricevere copie dei frame
    save_rx: Option<Arc<Mutex<mpsc::Receiver<SavePacket>>>>,
    local_sos: SignalOfStop,
    handler: Arc<WebRTCReceiver>,

    // Pipeline integration
    clock: MediaClock,
    health: Arc<PipelineHealth>,
    pipeline_state: PipelineState,
    /// Audio playback position for A/V sync tracking
    audio_position: Arc<AtomicI64>,
}

impl Receiver {
    pub fn new(sos: SignalOfStop) -> Self {
        let clock = MediaClock::new();
        let health = Arc::new(PipelineHealth::new());

        Self {
            is_streaming: Arc::new(AtomicBool::new(false)),
            audio_muted: Arc::new(AtomicBool::new(false)),
            save_stream: None,
            caster_addr: None,
            save_rx: None,
            local_sos: sos,
            handler: Arc::new(WebRTCReceiver::new()),
            clock,
            health,
            pipeline_state: PipelineState::Idle,
            audio_position: Arc::new(AtomicI64::new(0)),
        }
    }

    /// Get the media clock for timestamp correlation
    pub fn clock(&self) -> &MediaClock {
        &self.clock
    }

    /// Get the pipeline health metrics
    pub fn health(&self) -> &Arc<PipelineHealth> {
        &self.health
    }

    /// Get the current pipeline state
    pub fn pipeline_state(&self) -> &PipelineState {
        &self.pipeline_state
    }

    /// Get the audio playback position for A/V sync tracking
    pub fn audio_position(&self) -> i64 {
        self.audio_position.load(Ordering::Relaxed)
    }

    pub fn set_caster_addr(&mut self, addr: SocketAddr) {
        self.caster_addr = Some(addr);
    }

    /// Avvia la connessione al caster e ritorna il canale con i frame
    /// video da renderizzare (al posto della vecchia Pipeline GStreamer).
    pub fn launch(&mut self, auto: bool) -> Option<mpsc::Receiver<VideoFrame>> {
        self.pipeline_state = PipelineState::Initializing;

        // Canale principale: WebRTC → display
        // Increased capacity to prevent blocking when GUI is temporarily slow
        // At 30fps: 1024 frames = ~34 second buffer
        let (video_tx, video_rx) = mpsc::channel::<VideoFrame>(1024);
        // Canale per il salvataggio stream (largo per evitare drop quando il muxer è lento)
        let (save_tx, save_rx) = mpsc::channel::<SavePacket>(2048);

        self.save_rx = Some(Arc::new(Mutex::new(save_rx)));

        let is_streaming = Arc::clone(&self.is_streaming);
        let audio_muted = Arc::clone(&self.audio_muted);
        let mut caster_addr = self.caster_addr;
        let handler = Arc::clone(&self.handler);
        let health = self.health.clone();
        let audio_position = self.audio_position.clone();

        // Task di connessione + ricezione
        tokio::spawn(async move {
            // IMPORTANT: Set up receive channels BEFORE connecting
            // This ensures on_track handler is registered before SDP negotiation
            // Increased RTP channel to 512 packets to handle network bursts
            let (raw_tx, mut raw_rx) = mpsc::channel::<(Vec<u8>, bool, u16, u32)>(512);
            // Audio channel now includes RTP timestamp for proper timing
            let (audio_tx, mut audio_rx) = mpsc::channel::<(Vec<u8>, u32)>(1024);

            // Register channels first - this sets up the on_track handler
            handler.receive_video(raw_tx, audio_tx).await;

            // Auto-discovery del caster se necessario
            if auto {
                if caster_addr.is_none() {
                    caster_addr = find_caster();
                }

                if let Some(socket_addr) = caster_addr {
                    let addr = format!("ws://{}", socket_addr);
                    info!("Connecting to caster at {}", addr);

                    if let Err(e) = handler.connect(&addr).await {
                        error!("Failed to connect to caster: {}", e);
                        return;
                    }
                } else {
                    error!("No caster found");
                    return;
                }
            }

            if !handler.is_connected().await {
                error!("Not connected to caster");
                return;
            }

            is_streaming.store(true, Ordering::Relaxed);
            info!("Streaming started");

            let save_tx_video = save_tx.clone();
            let health_video = health.clone();
            // Share first video playout origin with audio task for sync
            let (first_video_start_tx, mut first_video_start_rx) = mpsc::channel::<Instant>(1);

            // Frame reordering pool
            let video_task = tokio::spawn(async move {
                log::info!("=== RECEIVER: Video processing task STARTED ===");

                let mut depacketizer = H264Depacketizer::new();
                let mut decoder = match FfmpegDecoder::new() {
                    Ok(d) => d,
                    Err(e) => {
                        error!("Failed to create H.264 decoder: {}", e);
                        return;
                    }
                };

                let mut consecutive_failures: u32 = 0;
                // Start rendering only after we received a keyframe (IDR) or SPS/PPS
                let mut waiting_for_keyframe = true;
                // Track first RTP timestamp for proper timestamp normalization
                let mut first_rtp_timestamp: Option<u32> = None;

                const MAX_REORDERING: u16 = 30; // Maximum expected packet reordering
                const MAX_BUFFER_SIZE: usize = 60; // Maximum buffered packets before cleanup (reduced from 200)
                // Buffer now stores: (payload, marker, rtp_timestamp)
                let mut frame_buffer =
                    std::collections::HashMap::<u16, (Vec<u8>, bool, u32)>::new();
                let mut expected_seq = None;
                let mut last_packet_time = Instant::now();
                let mut total_packets_received = 0u64;
                let mut last_stats_log = Instant::now();

                loop {
                    // Check for stream timeout every second
                    let recv_result =
                        tokio::time::timeout(std::time::Duration::from_secs(1), raw_rx.recv())
                            .await;

                    let (payload, marker, seq_num, rtp_timestamp) = match recv_result {
                        Ok(Some(packet)) => {
                            last_packet_time = Instant::now();
                            total_packets_received += 1;

                            // Log first packet
                            if total_packets_received == 1 {
                                log::info!(
                                    "RECEIVER: First video packet received! Seq: {}",
                                    packet.2
                                );
                            }

                            packet
                        }
                        Ok(None) => {
                            // Channel closed
                            error!("RECEIVER: Video packet channel CLOSED (raw_rx returned None)!");
                            break;
                        }
                        Err(_) => {
                            // Timeout - check if stream has stalled
                            let elapsed = last_packet_time.elapsed().as_secs();
                            if elapsed >= 5 && elapsed.is_multiple_of(5) {
                                error!(
                                    "RECEIVER: No video packets for {} seconds! Buffer: {}, expected: {:?}, total: {}",
                                    elapsed,
                                    frame_buffer.len(),
                                    expected_seq,
                                    total_packets_received
                                );
                            }
                            continue;
                        }
                    };

                    // Log stats every 30 seconds
                    if last_stats_log.elapsed().as_secs() >= 30 {
                        info!(
                            "Video receiver stats: {} packets received, buffer size: {}, expected_seq: {:?}",
                            total_packets_received,
                            frame_buffer.len(),
                            expected_seq
                        );
                        last_stats_log = Instant::now();
                    }
                    frame_buffer.insert(seq_num, (payload, marker, rtp_timestamp));

                    // Determine expected sequence number
                    if expected_seq.is_none() {
                        expected_seq = Some(seq_num);
                    }

                    // Cleanup stale entries if buffer grows too large
                    if frame_buffer.len() > MAX_BUFFER_SIZE
                        && let Some(exp) = expected_seq
                    {
                        // Remove packets that are too old (beyond reordering window)
                        frame_buffer.retain(|&seq, _| {
                            let diff = seq.wrapping_sub(exp);
                            // Keep packets within the reordering window ahead or behind expected
                            diff <= MAX_REORDERING || diff >= (u16::MAX - MAX_REORDERING)
                        });
                        log::warn!(
                            "Buffer overflow: cleaned up stale packets, {} remain",
                            frame_buffer.len()
                        );
                    }

                    // Process in-order packets
                    let mut processed_count = 0;
                    while let Some(exp) = expected_seq {
                        if let Some((data, marker_bit, rtp_ts)) = frame_buffer.remove(&exp) {
                            // Depacketize and reassemble frames
                            if let Some(h264_au) = depacketizer.push(&data, marker_bit) {
                                // Use RTP timestamp for proper timing
                                // Initialize first timestamp on first keyframe
                                if first_rtp_timestamp.is_none() && au_contains_idr_or_sps(&h264_au)
                                {
                                    first_rtp_timestamp = Some(rtp_ts);
                                    // Send first video start instant to audio task for sync.
                                    // Audio and video RTP clocks are independent, so use a shared local origin.
                                    let _ = first_video_start_tx.send(Instant::now()).await;
                                }

                                // Calculate normalized timestamp relative to first frame
                                // RTP video clock is typically 90kHz
                                let ts_us = if let Some(first_ts) = first_rtp_timestamp {
                                    // Convert RTP timestamp difference to microseconds
                                    // RTP video clock = 90kHz, so 1 tick = 11.111... microseconds
                                    let diff = rtp_ts.wrapping_sub(first_ts) as i64;
                                    (diff * 1_000_000) / 90_000
                                } else {
                                    // Before first keyframe, skip this frame entirely
                                    // Don't send to save channel or decode
                                    expected_seq = Some(exp.wrapping_add(1));
                                    continue;
                                };

                                // If we're waiting for a keyframe, skip non-key AUs to avoid
                                // showing partial/garbled frames at stream start.
                                if waiting_for_keyframe {
                                    if au_contains_idr_or_sps(&h264_au) {
                                        waiting_for_keyframe = false;
                                    } else {
                                        // still waiting, skip decode/display
                                        expected_seq = Some(exp.wrapping_add(1));
                                        continue;
                                    }
                                }

                                // Send to save channel only after we have a valid timestamp
                                // Use try_send to avoid blocking the video processing loop
                                if try_send(
                                    &save_tx_video,
                                    SavePacket::Video(h264_au.clone(), ts_us),
                                )
                                .is_closed()
                                {
                                    log::warn!("Save channel closed (video)");
                                }

                                // Decode H.264 → packed YUV420p (GPU converts to RGB)
                                if let Some((yuv, w, h)) = decoder.decode(&h264_au) {
                                    consecutive_failures = 0;
                                    let is_key = au_contains_idr_or_sps(&h264_au);
                                    health_video.record_frame(yuv.len(), is_key);
                                    let frame = VideoFrame {
                                        data: yuv,
                                        width: w as u32,
                                        height: h as u32,
                                    };
                                    // Use try_send to avoid blocking the processing loop
                                    // If the display channel is full, drop the frame rather than stall the pipeline
                                    match try_send(&video_tx, frame) {
                                        SendResult::Sent => {}
                                        SendResult::Full => {
                                            // Channel full, drop frame - this is better than blocking
                                            // the entire decode pipeline
                                            log::warn!(
                                                "Video display channel full, dropping frame"
                                            );
                                        }
                                        SendResult::Closed => {
                                            info!("Video display channel closed, stopping");
                                            break;
                                        }
                                    }
                                } else {
                                    consecutive_failures += 1;
                                    health_video.record_decode_failure();
                                    if consecutive_failures >= 10 {
                                        log::warn!(
                                            "10 consecutive decode failures, resetting depacketizer (waiting for IDR)"
                                        );
                                        depacketizer.reset();
                                        consecutive_failures = 0;
                                        waiting_for_keyframe = true;
                                    }
                                }
                            }
                            expected_seq = Some(exp.wrapping_add(1));
                            processed_count += 1;

                            // Yield periodically to prevent blocking on batch processing
                            if processed_count >= 10 {
                                break;
                            }
                        } else {
                            // Check if we should skip this packet (lost or beyond reordering window)
                            let diff = seq_num.wrapping_sub(exp);

                            // If current packet is far ahead, consider expected packet lost
                            if diff > 0 && diff <= MAX_REORDERING {
                                // Current packet is ahead but within window - keep waiting
                                break;
                            } else if diff > MAX_REORDERING && diff < (u16::MAX - MAX_REORDERING) {
                                // Packet lost or severely delayed - skip it
                                log::warn!(
                                    "Packet lost or delayed: expected {}, got {} (diff={})",
                                    exp,
                                    seq_num,
                                    diff
                                );
                                expected_seq = Some(exp.wrapping_add(1));
                                // Continue to try next sequence number
                            } else {
                                // seq_num is behind expected (wrapped or delayed) - shouldn't happen normally
                                break;
                            }
                        }
                    }
                }

                info!(
                    "Video processing loop exited after {} packets",
                    total_packets_received
                );
            });

            // Audio playback: decode Opus and play via cpal
            let audio_player = match AudioPlayer::new() {
                Ok(p) => Some(p),
                Err(e) => {
                    error!("Failed to create audio player: {}", e);
                    None
                }
            };
            let save_tx_audio = save_tx.clone();
            let audio_pos_ref = audio_position;
            // Get first video start instant for sync
            let audio_task = tokio::spawn(async move {
                let mut player = audio_player;
                let mut first_video_start: Option<Instant> = None;
                let mut first_audio_rtp_ts: Option<u32> = None;
                let mut first_audio_anchor_us: Option<i64> = None;

                loop {
                    tokio::select! {
                        // Wait for audio packet
                        Some((audio_data, rtp_timestamp)) = audio_rx.recv() => {
                            // Get first video start instant if not already received
                            if first_video_start.is_none() {
                                match first_video_start_rx.try_recv() {
                                    Ok(start) => first_video_start = Some(start),
                                    Err(mpsc::error::TryRecvError::Empty) => {
                                        // Still waiting for first video packet - skip this audio packet
                                        continue;
                                    }
                                    Err(mpsc::error::TryRecvError::Disconnected) => {
                                        // Video task closed - exit audio task
                                        break;
                                    }
                                }
                            }

                            // Initialize first audio RTP timestamp and anchor on video timeline
                            if first_audio_rtp_ts.is_none() {
                                first_audio_rtp_ts = Some(rtp_timestamp);
                                if let Some(video_start) = first_video_start {
                                    first_audio_anchor_us = Some(video_start.elapsed().as_micros() as i64);
                                }
                            }

                            // Audio RTP uses a different clock domain than video RTP.
                            // Build audio timeline from audio RTP deltas and align it to video start.
                            let ts_us = match (first_audio_rtp_ts, first_audio_anchor_us) {
                                (Some(audio_first_rtp), Some(anchor_us)) => {
                                    let diff = rtp_timestamp.wrapping_sub(audio_first_rtp) as i64;
                                    anchor_us + (diff * 1_000_000) / 48_000
                                }
                                _ => 0,
                            };

                            // Ensure timestamp is not negative
                            let ts_us = ts_us.max(0);

                            // Update audio position for A/V sync tracking
                            audio_pos_ref.store(ts_us, Ordering::Relaxed);

                            // Prefer non-blocking send, but don't drop audio packets while saving.
                            let save_packet = SavePacket::Audio(audio_data.clone(), ts_us);
                            match save_tx_audio.try_send(save_packet) {
                                Ok(()) => {}
                                Err(mpsc::error::TrySendError::Full(packet)) => {
                                    if save_tx_audio.send(packet).await.is_err() {
                                        log::warn!("Save channel closed (audio)");
                                    }
                                }
                                Err(mpsc::error::TrySendError::Closed(_)) => {
                                    log::warn!("Save channel closed (audio)");
                                }
                            }

                            // Play audio (if not muted)
                            if !audio_muted.load(Ordering::Relaxed)
                                && let Some(ref mut p) = player
                                    && let Err(e) = p.play(&audio_data) {
                                        log::warn!("Audio playback error: {}", e);
                                    }
                        },
                        // Wait for first video start origin if we haven't received any audio yet
                        Some(start) = first_video_start_rx.recv() => {
                            first_video_start = Some(start);
                            log::info!("Audio task received first video origin");
                        }
                    }
                }
            });

            // Wait for either video or audio task to complete (which indicates stream ended)
            tokio::select! {
                _ = video_task => {
                    info!("Video processing task ended");
                }
                _ = audio_task => {
                    info!("Audio processing task ended");
                }
            }

            is_streaming.store(false, Ordering::Relaxed);
            info!("Streaming ended. Final health: {}", health.summary());
        });

        self.pipeline_state = PipelineState::Running {
            started_at: Instant::now(),
        };
        Some(video_rx)
    }

    // ── Stato ───────────────────────────────────────────────────

    pub fn is_streaming(&self) -> bool {
        self.is_streaming.load(Ordering::Relaxed)
    }

    pub fn is_saving(&self) -> bool {
        self.save_stream.as_ref().is_some_and(|s| s.is_saving())
    }

    // ── Audio mute ──────────────────────────────────────────────

    pub fn is_audio_muted(&self) -> bool {
        self.audio_muted.load(Ordering::Relaxed)
    }

    pub fn toggle_audio_mute(&mut self) {
        let muted = !self.audio_muted.load(Ordering::Relaxed);
        self.audio_muted.store(muted, Ordering::Relaxed);
        info!("Receiver audio muted: {}", muted);
    }

    // ── Salvataggio stream ──────────────────────────────────────

    pub fn save_stream(&mut self, path: String) {
        if let Some(saver_channel) = &self.save_rx {
            let mut stream_saver = SaveStream::new(Arc::clone(saver_channel));
            stream_saver.start(path);
            self.save_stream = Some(stream_saver);
        }
    }

    pub fn save_stop(&mut self) {
        if let Some(mut save_stream) = self.save_stream.take() {
            save_stream.stop();
        }
    }

    // ── WebRTC ──────────────────────────────────────────────────

    pub fn get_connection_handler(&self) -> Arc<WebRTCReceiver> {
        Arc::clone(&self.handler)
    }
}

// ── Cleanup ─────────────────────────────────────────────────────

impl WorkerClose for Receiver {
    fn close(&mut self) {
        self.pipeline_state = PipelineState::Stopping;
        self.save_stop();
        let handler = Arc::clone(&self.handler);
        tokio::spawn(async move {
            handler.close().await;
        });
        self.is_streaming.store(false, Ordering::Relaxed);
        self.local_sos.cancel();
        self.pipeline_state = PipelineState::Stopped;
        info!("Receiver closed (pipeline state: {})", self.pipeline_state);
    }
}

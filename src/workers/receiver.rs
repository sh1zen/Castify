use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use log::{info, error};
use tokio::sync::{mpsc, Mutex};
use crate::assets::FRAME_RATE;
use crate::decoder::{H264Depacketizer, FfmpegDecoder, VideoFrame};
use crate::utils::net::common::find_caster;
use crate::utils::net::webrtc::WebRTCReceiver;
use crate::utils::sos::SignalOfStop;
use crate::workers::save_stream::SaveStream;
use crate::workers::WorkerClose;

pub struct Receiver {
    is_streaming: Arc<AtomicBool>,
    audio_muted: Arc<AtomicBool>,
    save_stream: Option<SaveStream>,
    caster_addr: Option<SocketAddr>,
    /// Canale dove arrivano i frame decodificati dal WebRTC
    frame_rx: Option<Arc<Mutex<mpsc::Receiver<VideoFrame>>>>,
    /// Canale usato dal SaveStream per ricevere copie dei frame
    save_rx: Option<Arc<Mutex<mpsc::Receiver<Vec<u8>>>>>,
    local_sos: SignalOfStop,
    handler: Arc<WebRTCReceiver>,
}

impl Receiver {
    pub fn new(sos: SignalOfStop) -> Self {
        Self {
            is_streaming: Arc::new(AtomicBool::new(false)),
            audio_muted: Arc::new(AtomicBool::new(false)),
            save_stream: None,
            caster_addr: None,
            frame_rx: None,
            save_rx: None,
            local_sos: sos,
            handler: Arc::new(WebRTCReceiver::new()),
        }
    }

    pub fn set_caster_addr(&mut self, addr: SocketAddr) {
        self.caster_addr = Some(addr);
    }

    /// Avvia la connessione al caster e ritorna il canale con i frame
    /// video da renderizzare (al posto della vecchia Pipeline GStreamer).
    pub fn launch(&mut self, auto: bool) -> Option<mpsc::Receiver<VideoFrame>> {
        // Canale principale: WebRTC → display
        let (video_tx, video_rx) = mpsc::channel::<VideoFrame>(FRAME_RATE as usize);
        // Canale per il salvataggio stream
        let (save_tx, save_rx) = mpsc::channel::<Vec<u8>>(FRAME_RATE as usize);

        self.save_rx = Some(Arc::new(Mutex::new(save_rx)));

        let is_streaming = Arc::clone(&self.is_streaming);
        let mut caster_addr = self.caster_addr;
        let handler = Arc::clone(&self.handler);

        // Task di connessione + ricezione
        tokio::spawn(async move {
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

            // Canale interno dal WebRTC handler (RTP packets with marker bit)
            let (raw_tx, mut raw_rx) = mpsc::channel::<(Vec<u8>, bool)>(FRAME_RATE as usize * 4);
            let (audio_tx, mut audio_rx) = mpsc::channel::<Vec<u8>>(128);
            handler.receive_video(raw_tx, audio_tx).await;

            // Audio playback: forward received Opus packets to log (actual decode requires opus decoder)
            // For now, we receive and discard audio packets - full decode/playback can be added later
            tokio::spawn(async move {
                while let Some(_audio_data) = audio_rx.recv().await {
                    // Audio packets received - playback integration point
                }
            });

            let mut depacketizer = H264Depacketizer::new();
            let mut decoder = match FfmpegDecoder::new() {
                Ok(d) => d,
                Err(e) => {
                    error!("Failed to create H.264 decoder: {}", e);
                    return;
                }
            };

            let mut consecutive_failures: u32 = 0;

            while let Some((payload, marker)) = raw_rx.recv().await {
                // Reassemble RTP packets into H.264 access units
                if let Some(h264_au) = depacketizer.push(&payload, marker) {
                    // Save raw H.264 data (best-effort)
                    let _ = save_tx.try_send(h264_au.clone());

                    // Decode H.264 → RGBA
                    if let Some((rgba, w, h)) = decoder.decode(&h264_au) {
                        consecutive_failures = 0;
                        let frame = VideoFrame { data: rgba, width: w as u32, height: h as u32 };
                        if video_tx.send(frame).await.is_err() {
                            info!("Video display channel closed, stopping");
                            break;
                        }
                    } else {
                        consecutive_failures += 1;
                        if consecutive_failures >= 10 {
                            log::warn!("10 consecutive decode failures, resetting depacketizer (waiting for IDR)");
                            depacketizer.reset();
                            consecutive_failures = 0;
                        }
                    }
                }
            }

            is_streaming.store(false, Ordering::Relaxed);
            info!("Streaming ended");
        });

        Some(video_rx)
    }

    // ── Stato ───────────────────────────────────────────────────

    pub fn is_streaming(&self) -> bool {
        self.is_streaming.load(Ordering::Relaxed)
    }

    pub fn is_saving(&self) -> bool {
        self.save_stream
            .as_ref()
            .map_or(false, |s| s.is_saving())
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
        self.save_stop();
        let handler = Arc::clone(&self.handler);
        tokio::spawn(async move {
            handler.close().await;
        });
        self.is_streaming.store(false, Ordering::Relaxed);
        self.local_sos.cancel();
        info!("Receiver closed");
    }
}
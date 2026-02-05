use log::{error, info};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;
use tokio::sync::mpsc::Receiver;

use ac_ffmpeg::codec::audio::frame::get_sample_format;
use ac_ffmpeg::codec::audio::{AudioDecoder, AudioEncoder, AudioFrameMut, ChannelLayout};
use ac_ffmpeg::codec::video::VideoDecoder;
use ac_ffmpeg::codec::{CodecParameters, Decoder, Encoder};
use ac_ffmpeg::format::io::IO;
use ac_ffmpeg::format::muxer::{Muxer, OutputFormat};
use ac_ffmpeg::packet::PacketMut;
use ac_ffmpeg::time::TimeBase;

#[derive(Clone, Debug)]
pub enum SavePacket {
    Video(Vec<u8>, i64), // H.264 Annex B access unit + timestamp_us
    Audio(Vec<u8>, i64), // Opus packet + timestamp_us
}

#[derive(Debug)]
pub struct SaveStream {
    saver_channel: Arc<Mutex<Receiver<SavePacket>>>,
    is_saving: Arc<AtomicBool>,
    stop_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

/// Extract SPS and PPS NAL units from the first Annex B access unit.
/// Returns them concatenated with 4-byte start codes, suitable for codec extradata.
fn extract_sps_pps_extradata(annex_b: &[u8]) -> Option<Vec<u8>> {
    const START_CODE: &[u8] = &[0, 0, 0, 1];

    // Find all NAL unit boundaries (positions right after a 4-byte start code)
    let mut nal_starts: Vec<usize> = Vec::new();
    for i in 0..annex_b.len().saturating_sub(3) {
        if annex_b[i..i + 4] == *START_CODE {
            nal_starts.push(i + 4);
        }
    }

    let mut sps: Option<&[u8]> = None;
    let mut pps: Option<&[u8]> = None;

    for (idx, &start) in nal_starts.iter().enumerate() {
        if start >= annex_b.len() {
            continue;
        }
        let nal_type = annex_b[start] & 0x1F;
        let end = if idx + 1 < nal_starts.len() {
            // NAL ends where the next start code begins (4 bytes before next NAL start)
            nal_starts[idx + 1] - 4
        } else {
            annex_b.len()
        };

        match nal_type {
            7 if sps.is_none() => sps = Some(&annex_b[start..end]),
            8 if pps.is_none() => pps = Some(&annex_b[start..end]),
            _ => {}
        }

        if sps.is_some() && pps.is_some() {
            break;
        }
    }

    let (sps, pps) = (sps?, pps?);

    let mut extradata = Vec::with_capacity(8 + sps.len() + pps.len());
    extradata.extend_from_slice(START_CODE);
    extradata.extend_from_slice(sps);
    extradata.extend_from_slice(START_CODE);
    extradata.extend_from_slice(pps);
    Some(extradata)
}

/// Replace NaN/Inf with silence; pass through all finite values unchanged.
fn sanitize_sample(s: f32) -> f32 {
    if s.is_finite() { s } else { 0.0 }
}

/// Holds state for Opus→AAC transcoding when saving to MP4/MOV.
struct AudioTranscoder {
    decoder: AudioDecoder,
    encoder: AudioEncoder,
    buf_left: Vec<f32>,
    buf_right: Vec<f32>,
    /// Sequential packet counter for simple timing (like video frames)
    packet_count: u64,
    last_audio_dts: i64,
    next_pts_samples: Option<i64>,
}

impl AudioTranscoder {
    fn new() -> anyhow::Result<Self> {
        let decoder = AudioDecoder::new("libopus").or_else(|e| {
            log::warn!(
                "libopus decoder not available ({}), trying built-in opus decoder",
                e
            );
            AudioDecoder::new("opus")
        })?;
        let encoder = AudioEncoder::builder("aac")?
            .sample_rate(48000)
            .channel_layout(ChannelLayout::from_channels(2).unwrap())
            .sample_format(get_sample_format("fltp"))
            .set_option("profile", "aac_low")
            .set_option("bit_rate", "128000")
            .set_option("strict", "experimental")
            .build()?;
        Ok(Self {
            decoder,
            encoder,
            buf_left: Vec::new(),
            buf_right: Vec::new(),
            packet_count: 0,
            last_audio_dts: -1,
            next_pts_samples: None,
        })
    }

    /// Decode an Opus packet and buffer the resulting PCM samples.
    fn decode_and_buffer(&mut self, opus_data: &[u8]) -> anyhow::Result<()> {
        // Validate input size (Opus packets: typically 6-1275 bytes)
        if opus_data.is_empty() || opus_data.len() > 2000 {
            log::warn!("Invalid Opus packet size: {}, skipping", opus_data.len());
            return Ok(()); // Skip, don't fail
        }

        let packet = PacketMut::from(opus_data).freeze();
        match self.decoder.try_push(packet) {
            Ok(()) => {}
            Err(e) => {
                if e.is_again() {
                    self.drain_decoder();
                    // Retry after draining
                    let retry = PacketMut::from(opus_data).freeze();
                    if let Err(e) = self.decoder.try_push(retry) {
                        log::warn!("Audio decoder retry failed: {}, skipping packet", e);
                        return Ok(()); // Skip this packet, don't fail
                    }
                } else {
                    log::warn!("Audio decoder error: {}, skipping packet", e);
                    return Ok(()); // Skip this packet, don't fail
                }
            }
        }
        self.drain_decoder();
        Ok(())
    }

    /// Drain decoded frames from the Opus decoder into sample buffers.
    /// Detects sample format (float vs int16) based on plane data size.
    fn drain_decoder(&mut self) {
        while let Ok(Some(frame)) = self.decoder.take() {
            let planes = frame.planes();
            let sample_count = frame.samples();
            let channels = frame.channel_layout().channels() as usize;
            let sample_fmt = frame.sample_format().name();

            if sample_count == 0 || channels == 0 || planes.is_empty() {
                continue;
            }

            match sample_fmt {
                "fltp" => {
                    let left_plane = planes[0].data();
                    let right_plane = if channels > 1 && planes.len() > 1 {
                        planes[1].data()
                    } else {
                        planes[0].data()
                    };
                    let needed = sample_count * 4;
                    if left_plane.len() < needed || right_plane.len() < needed {
                        continue;
                    }
                    let left: &[f32] = unsafe {
                        std::slice::from_raw_parts(left_plane.as_ptr() as *const f32, sample_count)
                    };
                    let right: &[f32] = unsafe {
                        std::slice::from_raw_parts(right_plane.as_ptr() as *const f32, sample_count)
                    };
                    self.buf_left
                        .extend(left.iter().map(|&s| sanitize_sample(s)));
                    self.buf_right
                        .extend(right.iter().map(|&s| sanitize_sample(s)));
                }
                "s16p" => {
                    let left_plane = planes[0].data();
                    let right_plane = if channels > 1 && planes.len() > 1 {
                        planes[1].data()
                    } else {
                        planes[0].data()
                    };
                    let needed = sample_count * 2;
                    if left_plane.len() < needed || right_plane.len() < needed {
                        continue;
                    }
                    let left: &[i16] = unsafe {
                        std::slice::from_raw_parts(left_plane.as_ptr() as *const i16, sample_count)
                    };
                    let right: &[i16] = unsafe {
                        std::slice::from_raw_parts(right_plane.as_ptr() as *const i16, sample_count)
                    };
                    self.buf_left
                        .extend(left.iter().map(|&s| sanitize_sample(s as f32 / 32768.0)));
                    self.buf_right
                        .extend(right.iter().map(|&s| sanitize_sample(s as f32 / 32768.0)));
                }
                "flt" => {
                    let plane = planes[0].data();
                    let needed = sample_count * channels * 4;
                    if plane.len() < needed {
                        continue;
                    }
                    let samples: &[f32] = unsafe {
                        std::slice::from_raw_parts(
                            plane.as_ptr() as *const f32,
                            sample_count * channels,
                        )
                    };
                    for frame in samples.chunks_exact(channels) {
                        let l = frame[0];
                        let r = if channels > 1 { frame[1] } else { frame[0] };
                        self.buf_left.push(sanitize_sample(l));
                        self.buf_right.push(sanitize_sample(r));
                    }
                }
                "s16" => {
                    let plane = planes[0].data();
                    let needed = sample_count * channels * 2;
                    if plane.len() < needed {
                        continue;
                    }
                    let samples: &[i16] = unsafe {
                        std::slice::from_raw_parts(
                            plane.as_ptr() as *const i16,
                            sample_count * channels,
                        )
                    };
                    for frame in samples.chunks_exact(channels) {
                        let l = frame[0] as f32 / 32768.0;
                        let r = if channels > 1 {
                            frame[1] as f32 / 32768.0
                        } else {
                            l
                        };
                        self.buf_left.push(sanitize_sample(l));
                        self.buf_right.push(sanitize_sample(r));
                    }
                }
                other => {
                    log::warn!(
                        "Unsupported decoded audio sample format '{}', skipping frame",
                        other
                    );
                }
            }
        }
    }

    /// Ensure packet DTS is strictly monotonically increasing and PTS >= DTS.
    fn enforce_monotonic_dts(
        &mut self,
        pkt: ac_ffmpeg::packet::Packet,
    ) -> ac_ffmpeg::packet::Packet {
        let dts = pkt.dts().timestamp();
        let pts = pkt.pts().timestamp();

        // Ensure DTS is strictly increasing
        let fixed_dts = if dts <= self.last_audio_dts {
            self.last_audio_dts + 1
        } else {
            dts
        };

        // Ensure PTS >= DTS
        let fixed_pts = if pts < fixed_dts { fixed_dts } else { pts };

        self.last_audio_dts = fixed_dts;
        pkt.with_raw_dts(fixed_dts).with_raw_pts(fixed_pts)
    }

    fn ensure_pts_anchor(&mut self, relative_ts_us: i64) {
        if self.next_pts_samples.is_none() {
            let start_pts = (relative_ts_us as f64 * 48000.0 / 1_000_000.0) as i64;
            self.next_pts_samples = Some(start_pts.max(0));
        }
    }

    /// Encode buffered samples into AAC frames and mux them with proper timestamps.
    /// Returns encoded AAC packets ready for muxing.
    fn encode_buffered_with_timestamp(
        &mut self,
        audio_idx: usize,
        audio_tb: TimeBase,
        relative_ts_us: i64,
    ) -> anyhow::Result<Vec<ac_ffmpeg::packet::Packet>> {
        let aac_frame_size = self.encoder.samples_per_frame().unwrap_or(1024);
        let mut packets = Vec::new();
        self.ensure_pts_anchor(relative_ts_us);
        let mut pts_samples = self.next_pts_samples.unwrap_or(0);

        while self.buf_left.len() >= aac_frame_size && self.buf_right.len() >= aac_frame_size {
            let mut frame = AudioFrameMut::silence(
                self.encoder.codec_parameters().channel_layout(),
                self.encoder.codec_parameters().sample_format(),
                self.encoder.codec_parameters().sample_rate(),
                aac_frame_size,
            );

            let out_fmt = frame.sample_format().name();
            match out_fmt {
                "fltp" => {
                    {
                        let mut planes = frame.planes_mut();
                        let plane_data = planes[0].data_mut();
                        let dst: &mut [f32] = unsafe {
                            std::slice::from_raw_parts_mut(
                                plane_data.as_mut_ptr() as *mut f32,
                                plane_data.len() / 4,
                            )
                        };
                        let copy_len = aac_frame_size.min(dst.len());
                        for (i, &s) in self.buf_left.iter().enumerate().take(copy_len) {
                            dst[i] = if s.is_finite() {
                                s.clamp(-1.0, 1.0)
                            } else {
                                0.0
                            };
                        }
                    }
                    {
                        let mut planes = frame.planes_mut();
                        let plane_data = planes[1].data_mut();
                        let dst: &mut [f32] = unsafe {
                            std::slice::from_raw_parts_mut(
                                plane_data.as_mut_ptr() as *mut f32,
                                plane_data.len() / 4,
                            )
                        };
                        let copy_len = aac_frame_size.min(dst.len());
                        for (i, &s) in self.buf_right.iter().enumerate().take(copy_len) {
                            dst[i] = if s.is_finite() {
                                s.clamp(-1.0, 1.0)
                            } else {
                                0.0
                            };
                        }
                    }
                }
                "s16p" => {
                    {
                        let mut planes = frame.planes_mut();
                        let plane_data = planes[0].data_mut();
                        let dst: &mut [i16] = unsafe {
                            std::slice::from_raw_parts_mut(
                                plane_data.as_mut_ptr() as *mut i16,
                                plane_data.len() / 2,
                            )
                        };
                        let copy_len = aac_frame_size.min(dst.len());
                        for (i, &s) in self.buf_left.iter().enumerate().take(copy_len) {
                            let v = if s.is_finite() {
                                s.clamp(-1.0, 1.0)
                            } else {
                                0.0
                            };
                            dst[i] = (v * 32767.0) as i16;
                        }
                    }
                    {
                        let mut planes = frame.planes_mut();
                        let plane_data = planes[1].data_mut();
                        let dst: &mut [i16] = unsafe {
                            std::slice::from_raw_parts_mut(
                                plane_data.as_mut_ptr() as *mut i16,
                                plane_data.len() / 2,
                            )
                        };
                        let copy_len = aac_frame_size.min(dst.len());
                        for (i, &s) in self.buf_right.iter().enumerate().take(copy_len) {
                            let v = if s.is_finite() {
                                s.clamp(-1.0, 1.0)
                            } else {
                                0.0
                            };
                            dst[i] = (v * 32767.0) as i16;
                        }
                    }
                }
                other => {
                    log::error!(
                        "Unsupported AAC encoder sample format '{}', skipping frame",
                        other
                    );
                    self.buf_left.drain(..aac_frame_size);
                    self.buf_right.drain(..aac_frame_size);
                    self.packet_count += 1;
                    pts_samples += aac_frame_size as i64;
                    continue;
                }
            }

            let pts = ac_ffmpeg::time::Timestamp::new(pts_samples, audio_tb);
            let frame = frame.with_pts(pts).freeze();

            match self.encoder.try_push(frame) {
                Ok(()) => {}
                Err(e) => {
                    log::error!("Audio encoder push failed: {}, skipping frame", e);
                    self.buf_left.drain(..aac_frame_size);
                    self.buf_right.drain(..aac_frame_size);
                    self.packet_count += 1;
                    pts_samples += aac_frame_size as i64;
                    continue;
                }
            }

            self.buf_left.drain(..aac_frame_size);
            self.buf_right.drain(..aac_frame_size);
            self.packet_count += 1;

            while let Some(pkt) = self.encoder.take()? {
                let pkt = self.enforce_monotonic_dts(pkt);
                packets.push(pkt.with_stream_index(audio_idx));
            }

            pts_samples += aac_frame_size as i64;
        }

        self.next_pts_samples = Some(pts_samples);
        Ok(packets)
    }

    /// Flush remaining buffered samples (pad with silence if needed) and drain encoder.
    fn flush(
        &mut self,
        audio_idx: usize,
        audio_tb: TimeBase,
    ) -> anyhow::Result<Vec<ac_ffmpeg::packet::Packet>> {
        let aac_frame_size = self.encoder.samples_per_frame().unwrap_or(1024);
        let mut packets = Vec::new();

        // Pad remaining samples to a full frame and encode
        if !self.buf_left.is_empty() || !self.buf_right.is_empty() {
            let remaining = self.buf_left.len().max(self.buf_right.len());
            self.buf_left.resize(aac_frame_size.max(remaining), 0.0);
            self.buf_right.resize(aac_frame_size.max(remaining), 0.0);

            // Encode all remaining (now padded to at least one full frame)
            self.ensure_pts_anchor(0);
            let flushed = self.encode_buffered_with_timestamp(audio_idx, audio_tb, 0)?;
            packets.extend(flushed);
        }

        // Flush the AAC encoder
        self.encoder.flush()?;
        while let Some(pkt) = self.encoder.take()? {
            let pkt = self.enforce_monotonic_dts(pkt);
            packets.push(pkt.with_stream_index(audio_idx));
        }

        Ok(packets)
    }
}

impl SaveStream {
    pub fn new(saver_channel: Arc<Mutex<Receiver<SavePacket>>>) -> Self {
        Self {
            saver_channel,
            is_saving: Arc::new(AtomicBool::new(false)),
            stop_tx: None,
        }
    }

    pub fn start(&mut self, path: String) {
        self.is_saving.store(true, Ordering::Release);

        let is_saving = Arc::clone(&self.is_saving);
        let saver_channel = Arc::clone(&self.saver_channel);
        let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
        self.stop_tx = Some(stop_tx);

        tokio::spawn(async move {
            if let Err(e) =
                Self::run_muxer(saver_channel, Arc::clone(&is_saving), stop_rx, path).await
            {
                error!("SaveStream muxer error: {}", e);
            }
            is_saving.store(false, Ordering::Release);
        });
    }

    async fn run_muxer(
        saver_channel: Arc<Mutex<Receiver<SavePacket>>>,
        is_saving: Arc<AtomicBool>,
        mut stop_rx: tokio::sync::oneshot::Receiver<()>,
        path: String,
    ) -> anyhow::Result<()> {
        // Drain stale packets from the channel before starting.
        // The channel may contain old packets from before save was requested.
        {
            let mut rx = saver_channel.lock().await;
            let mut drained = 0;
            while rx.try_recv().is_ok() {
                drained += 1;
            }
            if drained > 0 {
                info!("Drained {} stale packets from save channel", drained);
            }
        }

        // Wait for the first fresh Video packet (needed to init muxer)
        let first_video: Vec<u8>;
        let first_video_ts: i64;
        let mut buffered_audio: Vec<(Vec<u8>, i64)> = Vec::new();

        loop {
            let pkt = {
                let mut rx = saver_channel.lock().await;
                tokio::select! {
                    frame = rx.recv() => frame,
                    _ = &mut stop_rx => {
                        info!("SaveStream stop before first video frame");
                        return Ok(());
                    }
                }
            };

            let Some(pkt) = pkt else {
                info!("Saver channel closed before first video frame");
                return Ok(());
            };

            match pkt {
                SavePacket::Video(data, ts) => {
                    // Only use this AU as the starting point if it contains SPS+PPS;
                    // otherwise the muxer can't determine video dimensions.
                    if extract_sps_pps_extradata(&data).is_some() {
                        log::debug!(
                            "SaveStream: First video packet received with timestamp: {} us",
                            ts
                        );
                        first_video = data;
                        first_video_ts = ts;
                        break;
                    } else {
                        log::debug!("Skipping video AU without SPS/PPS while waiting for keyframe");
                    }
                }
                SavePacket::Audio(data, ts) => {
                    buffered_audio.push((data, ts));
                }
            }
        }

        // Initialize muxer with codec parameters derived from first video AU
        let (mut muxer, video_idx, audio_idx, mut transcoder) =
            tokio::task::block_in_place(|| -> anyhow::Result<_> {
                // Video codec parameters: extract SPS/PPS from first AU for container header
                let sps_pps = extract_sps_pps_extradata(&first_video);
                if sps_pps.is_none() {
                    log::warn!("Could not extract SPS/PPS from first video AU");
                }

                let mut vdec_builder =
                    VideoDecoder::builder("h264")?.time_base(TimeBase::new(1, 90_000));
                if let Some(ref extradata) = sps_pps {
                    vdec_builder = vdec_builder.extradata(Some(extradata));
                }
                let mut vdec = vdec_builder.build()?;
                let pkt = PacketMut::from(&first_video[..]).freeze();
                let _ = vdec.try_push(pkt);
                let _ = vdec.take(); // parse headers
                let video_params: CodecParameters = vdec.codec_parameters().into();

                // Try AAC transcoding first (best compatibility), fall back to raw Opus passthrough
                let (audio_params, transcoder) = match AudioTranscoder::new() {
                    Ok(tc) => {
                        let params: CodecParameters = tc.encoder.codec_parameters().into();
                        (params, Some(tc))
                    }
                    Err(e) => {
                        log::warn!(
                            "AAC transcoder unavailable ({}), falling back to Opus passthrough",
                            e
                        );
                        let aenc = AudioEncoder::builder("libopus")?
                            .sample_rate(48000)
                            .channel_layout(ChannelLayout::from_channels(2).unwrap())
                            .sample_format(get_sample_format("flt"))
                            .set_option("frame_duration", "10")
                            .build()?;
                        let params: CodecParameters = aenc.codec_parameters().into();
                        (params, None)
                    }
                };

                // Create muxer - use MP4 format for better compatibility
                let file = std::fs::File::create(&path)?;
                let io = IO::from_seekable_write_stream(file);
                let format = OutputFormat::guess_from_file_name(&path)
                    .or_else(|| OutputFormat::find_by_name("mp4"))
                    .ok_or_else(|| anyhow::anyhow!("No output format found"))?;

                let mut builder = Muxer::builder();
                let video_idx = builder.add_stream(&video_params)?;
                let audio_idx = builder.add_stream(&audio_params)?;
                let muxer = builder
                    .interleaved(true) // Interleave packets for MP4 compatibility
                    .build(io, format)?;

                Ok((muxer, video_idx, audio_idx, transcoder))
            })?;

        // Audio time base: 1/48000 for sample-accurate AAC timestamps
        let audio_tb = TimeBase::new(1, 48000);

        let audio_mode = if transcoder.is_some() { "AAC" } else { "Opus" };
        info!("SaveStream started → {} (audio: {})", path, audio_mode);

        // Video time base: 1/90000 for H.264 timestamps
        const VIDEO_TIMEBASE: i64 = 90_000; // 90kHz
        let video_tb = TimeBase::new(1, 90_000);

        // Track first packet timestamp to use as time origin (0)
        let first_video_ts_us = first_video_ts;
        let mut video_frame_count = 0i64;

        // Write first video packet with PTS/DTS=0
        tokio::task::block_in_place(|| -> anyhow::Result<()> {
            log::info!("SaveStream: Writing first video frame with PTS/DTS=0");
            let pkt = PacketMut::from(&first_video[..])
                .with_stream_index(video_idx)
                .with_pts(ac_ffmpeg::time::Timestamp::new(0, video_tb))
                .with_dts(ac_ffmpeg::time::Timestamp::new(0, video_tb))
                .freeze();
            muxer.push(pkt)?;
            video_frame_count += 1;
            Ok(())
        })?;

        // Process any buffered audio packets (non-fatal: log errors and continue)
        let mut audio_packet_count = 0u64;
        for (audio_data, audio_ts) in buffered_audio {
            audio_packet_count += 1;
            tokio::task::block_in_place(|| {
                if let Some(ref mut tc) = transcoder {
                    if let Err(e) = tc.decode_and_buffer(&audio_data) {
                        log::warn!("Audio decode error (buffered): {}", e);
                        return;
                    }

                    // Calculate relative timestamp for buffered audio using video's time origin
                    let relative_ts_us = audio_ts - first_video_ts;
                    // Ensure timestamp is not negative (audio before video starts)
                    let relative_ts_us = relative_ts_us.max(0);

                    match tc.encode_buffered_with_timestamp(audio_idx, audio_tb, relative_ts_us) {
                        Ok(packets) => {
                            log::debug!("Audio (buffered): {} AAC packets encoded", packets.len());
                            for pkt in packets {
                                if let Err(e) = muxer.push(pkt) {
                                    log::warn!("Audio mux error (buffered): {}", e);
                                }
                            }
                        }
                        Err(e) => log::warn!("Audio encode error (buffered): {}", e),
                    }
                } else {
                    // Opus passthrough: would need timestamp handling
                    log::warn!("Opus passthrough not implemented for buffered audio");
                }
            });
        }
        log::info!(
            "Processed {} buffered audio packets before video start",
            audio_packet_count
        );

        // Main loop: mux incoming packets
        // Batch packets to reduce block_in_place overhead
        const BATCH_SIZE: usize = 10;
        let mut packet_batch = Vec::with_capacity(BATCH_SIZE);
        let mut check_counter = 0u32;
        let mut audio_packets_received = 0u64;
        let mut audio_packets_encoded = 0u64;

        loop {
            // Collect a batch of packets
            packet_batch.clear();

            // Always wait for at least one packet
            let first_packet = {
                let mut rx = saver_channel.lock().await;
                tokio::select! {
                    frame = rx.recv() => frame,
                    _ = &mut stop_rx => {
                        info!("SaveStream stop signal received");
                        break;
                    }
                }
            };

            let Some(first_packet) = first_packet else {
                info!("Saver channel closed");
                break;
            };

            packet_batch.push(first_packet);

            // Try to collect more packets (non-blocking)
            {
                let mut rx = saver_channel.lock().await;
                while packet_batch.len() < BATCH_SIZE {
                    match rx.try_recv() {
                        Ok(pkt) => packet_batch.push(pkt),
                        Err(_) => break, // No more packets available
                    }
                }
            }

            // Log batch size every 100 batches for diagnostics
            if check_counter.is_multiple_of(100) {
                log::debug!(
                    "SaveStream: processed {} batches, current batch size: {}",
                    check_counter,
                    packet_batch.len()
                );
            }

            // Check is_saving only occasionally to reduce lock contention
            check_counter += 1;
            if check_counter.is_multiple_of(30) && !is_saving.load(Ordering::Acquire) {
                info!("is_saving flag set to false, stopping");
                break;
            }

            // Process entire batch in one block_in_place call
            tokio::task::block_in_place(|| {
                for data in packet_batch.iter() {
                    match data {
                        SavePacket::Video(bytes, ts_us) => {
                            // Calculate relative timestamp using first video packet as origin
                            let relative_ts_us = ts_us - first_video_ts_us;

                            // Convert microseconds to 90kHz ticks
                            let pts_val = (relative_ts_us as f64 * VIDEO_TIMEBASE as f64
                                / 1_000_000.0) as i64;

                            let pkt = PacketMut::from(&bytes[..])
                                .with_stream_index(video_idx)
                                .with_pts(ac_ffmpeg::time::Timestamp::new(pts_val, video_tb))
                                .with_dts(ac_ffmpeg::time::Timestamp::new(pts_val, video_tb))
                                .freeze();
                            if let Err(e) = muxer.push(pkt) {
                                log::warn!("Video mux error: {}, skipping packet", e);
                            }
                            video_frame_count += 1;
                        }
                        SavePacket::Audio(bytes, ts_us) => {
                            audio_packets_received += 1;

                            // Calculate relative timestamp using video's time origin
                            let relative_ts_us = ts_us - first_video_ts;
                            // Ensure timestamp is not negative (audio before video starts)
                            let relative_ts_us = relative_ts_us.max(0);

                            if let Some(ref mut tc) = transcoder {
                                if let Err(e) = tc.decode_and_buffer(bytes) {
                                    log::warn!("Audio decode error: {}", e);
                                    continue; // Skip this packet, process next
                                }

                                // Encode with timestamp information
                                match tc.encode_buffered_with_timestamp(
                                    audio_idx,
                                    audio_tb,
                                    relative_ts_us,
                                ) {
                                    Ok(packets) => {
                                        audio_packets_encoded += packets.len() as u64;
                                        for pkt in packets {
                                            if let Err(e) = muxer.push(pkt) {
                                                log::warn!(
                                                    "Audio mux error: {}, skipping packet",
                                                    e
                                                );
                                            }
                                        }
                                    }
                                    Err(e) => log::warn!("Audio encode error: {}", e),
                                }
                            } else {
                                // Opus passthrough: would need timestamp handling
                                log::warn!("Opus passthrough not fully implemented");
                            }
                        }
                    }
                }
            });
        }

        // Flush transcoder and close muxer
        tokio::task::block_in_place(|| -> anyhow::Result<()> {
            if let Some(ref mut tc) = transcoder {
                match tc.flush(audio_idx, audio_tb) {
                    Ok(packets) => {
                        for pkt in packets {
                            if let Err(e) = muxer.push(pkt) {
                                log::warn!("Audio mux error (flush): {}", e);
                            }
                        }
                    }
                    Err(e) => log::warn!("Audio flush error: {}", e),
                }
            }
            muxer.flush()?;
            let _ = muxer.close()?;
            Ok(())
        })?;

        // Already tracked video_frame_count variable
        info!(
            "SaveStream finished → {} ({} video frames, {} audio packets received, {} audio packets encoded)",
            path, video_frame_count, audio_packets_received, audio_packets_encoded
        );
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
    }

    pub fn is_saving(&self) -> bool {
        self.is_saving.load(Ordering::Acquire)
    }
}

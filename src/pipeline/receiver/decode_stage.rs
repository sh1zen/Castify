//! Decode stage for the receiver pipeline
//!
//! Wraps H264Depacketizer + FfmpegDecoder for video and AudioPlayer for audio,
//! producing decoded frames for A/V sync.

use crate::decoder::{FfmpegDecoder, H264Depacketizer, VideoFrame};
use crate::pipeline::PipelineStage;
use crate::pipeline::clock::MediaClock;
use crate::pipeline::health::PipelineHealth;
use crate::pipeline::receiver::reorder_stage::RtpPacket;
use crate::pipeline::types::Timestamp;
use anyhow::Result;
use async_trait::async_trait;
use log::{info, warn};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;

/// Decoded video frame with timing information
#[derive(Debug, Clone)]
pub struct TimedVideoFrame {
    pub frame: VideoFrame,
    pub pts: Timestamp,
    pub correlation_id: u64,
    pub is_keyframe: bool,
}

/// Decode stage: depacketizes RTP and decodes H.264 into raw video frames
pub struct DecodeStage {
    clock: MediaClock,
    health: Arc<PipelineHealth>,
    input_rx: Option<mpsc::Receiver<RtpPacket>>,
    output_tx: Option<mpsc::Sender<TimedVideoFrame>>,
}

/// Return true if the H.264 access unit contains an IDR (nal type 5) or SPS/PPS (7/8).
fn au_contains_idr_or_sps(au: &[u8]) -> bool {
    const START_CODE: &[u8] = &[0, 0, 0, 1];
    let mut i = 0usize;
    while i + 4 <= au.len() {
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

impl DecodeStage {
    /// Create a new decode stage
    pub fn new(clock: MediaClock, health: Arc<PipelineHealth>) -> Self {
        Self {
            clock,
            health,
            input_rx: None,
            output_tx: None,
        }
    }

    /// Set the input channel (reordered RTP packets)
    pub fn set_input(&mut self, rx: mpsc::Receiver<RtpPacket>) {
        self.input_rx = Some(rx);
    }

    /// Get the output channel for decoded video frames
    pub fn take_output(&mut self) -> mpsc::Receiver<TimedVideoFrame> {
        let (tx, rx) = mpsc::channel::<TimedVideoFrame>(8);
        self.output_tx = Some(tx);
        rx
    }
}

#[async_trait]
impl PipelineStage for DecodeStage {
    async fn run(&mut self) -> Result<()> {
        let mut input_rx = self
            .input_rx
            .take()
            .ok_or_else(|| anyhow::anyhow!("No input channel"))?;
        let output_tx = self
            .output_tx
            .take()
            .ok_or_else(|| anyhow::anyhow!("No output channel"))?;

        let mut depacketizer = H264Depacketizer::new();
        let mut decoder =
            FfmpegDecoder::new().map_err(|e| anyhow::anyhow!("Failed to create decoder: {}", e))?;

        let mut consecutive_failures: u32 = 0;
        let mut waiting_for_keyframe = true;
        let _start_time = Instant::now();
        let mut total_frames = 0u64;
        let mut decoded_frames = 0u64;

        info!("DecodeStage: started");

        while let Some(packet) = input_rx.recv().await {
            total_frames += 1;

            // Depacketize RTP into H.264 access units
            if let Some(h264_au) = depacketizer.push(&packet.payload, packet.marker) {
                // Wait for first keyframe
                if waiting_for_keyframe {
                    if au_contains_idr_or_sps(&h264_au) {
                        waiting_for_keyframe = false;
                        info!("DecodeStage: received first keyframe");
                    } else {
                        continue;
                    }
                }

                // Decode H.264 to YUV420p
                if let Some((yuv, w, h)) = decoder.decode(&h264_au) {
                    consecutive_failures = 0;
                    decoded_frames += 1;

                    let pts = self.clock.timestamp_from_instant(packet.received_at);
                    let correlation_id = self.clock.next_correlation_id();
                    let is_keyframe = au_contains_idr_or_sps(&h264_au);

                    self.health.record_frame(yuv.len(), is_keyframe);

                    let timed_frame = TimedVideoFrame {
                        frame: VideoFrame {
                            data: yuv,
                            width: w as u32,
                            height: h as u32,
                        },
                        pts,
                        correlation_id,
                        is_keyframe,
                    };

                    if output_tx.send(timed_frame).await.is_err() {
                        info!("DecodeStage: output channel closed");
                        break;
                    }
                } else {
                    consecutive_failures += 1;
                    self.health.record_decode_failure();

                    if consecutive_failures >= 10 {
                        warn!("DecodeStage: 10 consecutive failures, resetting (waiting for IDR)");
                        depacketizer.reset();
                        consecutive_failures = 0;
                        waiting_for_keyframe = true;
                    }
                }
            }
        }

        info!(
            "DecodeStage: finished ({} packets, {} decoded frames)",
            total_frames, decoded_frames
        );
        Ok(())
    }

    fn name(&self) -> &'static str {
        "DecodeStage"
    }
}

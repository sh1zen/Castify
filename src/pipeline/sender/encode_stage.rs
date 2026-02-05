//! Encode stage for the sender pipeline
//!
//! Wraps FfmpegEncoder and consumes raw frames, producing encoded H.264 packets.

use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use log::info;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tokio::sync::mpsc;

use crate::pipeline::PipelineStage;
use crate::pipeline::clock::MediaClock;
use crate::pipeline::health::PipelineHealth;
use crate::pipeline::types::{MediaFrame, MediaKind};

/// Encode stage: transforms captured frames into encoded H.264 data
///
/// This stage wraps the encoder and adds MediaClock-based timestamp correlation
/// to the output MediaFrames.
pub struct EncodeStage {
    /// Force IDR flag shared with the encoder
    pub force_idr: Arc<AtomicBool>,
    /// Media clock for timestamp correlation
    clock: MediaClock,
    /// Health metrics
    health: Arc<PipelineHealth>,
    /// Input channel (raw H.264 from capture)
    input_rx: Option<mpsc::Receiver<Bytes>>,
    /// Output channel (MediaFrame with PTS)
    output_tx: Option<mpsc::Sender<MediaFrame>>,
}

impl EncodeStage {
    /// Create a new encode stage
    pub fn new(clock: MediaClock, health: Arc<PipelineHealth>) -> Self {
        Self {
            force_idr: Arc::new(AtomicBool::new(false)),
            clock,
            health,
            input_rx: None,
            output_tx: None,
        }
    }

    /// Set the input channel (encoded frames from CaptureStage)
    pub fn set_input(&mut self, rx: mpsc::Receiver<Bytes>) {
        self.input_rx = Some(rx);
    }

    /// Get the output channel for MediaFrames
    pub fn take_output(&mut self) -> mpsc::Receiver<MediaFrame> {
        let (tx, rx) = mpsc::channel::<MediaFrame>(32);
        self.output_tx = Some(tx);
        rx
    }

    /// Process a single encoded frame: wrap with timestamps and correlation IDs
    fn wrap_frame(&self, data: Bytes) -> MediaFrame {
        let pts = self.clock.video_now();
        let correlation_id = self.clock.next_correlation_id();

        // Detect keyframes by scanning for IDR NAL units
        let is_keyframe = contains_idr(&data);

        if is_keyframe {
            self.health.record_frame(data.len(), true);
        } else {
            self.health.record_frame(data.len(), false);
        }

        MediaFrame {
            kind: MediaKind::Video,
            data,
            pts,
            dts: pts,
            correlation_id,
            is_keyframe,
            width: None,
            height: None,
            sample_rate: None,
            channels: None,
        }
    }
}

/// Check if H.264 Annex B data contains an IDR NAL unit (type 5)
fn contains_idr(data: &[u8]) -> bool {
    let start_code: &[u8] = &[0, 0, 0, 1];
    let mut i = 0;
    while i + 4 < data.len() {
        if &data[i..i + 4] == start_code {
            if i + 4 < data.len() && (data[i + 4] & 0x1F) == 5 {
                return true;
            }
            i += 4;
        } else {
            i += 1;
        }
    }
    false
}

#[async_trait]
impl PipelineStage for EncodeStage {
    async fn run(&mut self) -> Result<()> {
        let mut input_rx = self
            .input_rx
            .take()
            .ok_or_else(|| anyhow::anyhow!("No input channel"))?;
        let output_tx = self
            .output_tx
            .take()
            .ok_or_else(|| anyhow::anyhow!("No output channel"))?;

        info!("EncodeStage: started");
        let mut total_frames = 0u64;

        while let Some(raw_data) = input_rx.recv().await {
            let media_frame = self.wrap_frame(raw_data);
            total_frames += 1;

            if output_tx.send(media_frame).await.is_err() {
                info!("EncodeStage: output channel closed");
                break;
            }
        }

        info!("EncodeStage: finished after {} frames", total_frames);
        Ok(())
    }

    fn name(&self) -> &'static str {
        "EncodeStage"
    }
}

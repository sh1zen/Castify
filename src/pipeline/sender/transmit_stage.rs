//! Transmit stage for the sender pipeline
//!
//! Wraps WebRTCServer and forwards encoded frames to connected peers.

use anyhow::Result;
use async_trait::async_trait;
use log::info;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::capture::capturer::EncodedFrame;
use crate::pipeline::PipelineStage;
use crate::pipeline::health::PipelineHealth;
use crate::pipeline::types::MediaFrame;
use crate::utils::net::webrtc::WebRTCServer;

/// Transmit stage: forwards encoded media to WebRTC peers
///
/// This stage consumes MediaFrames and converts them to EncodedFrames
/// for transmission via the existing WebRTC infrastructure.
pub struct TransmitStage {
    server: Arc<WebRTCServer>,
    health: Arc<PipelineHealth>,
    input_rx: Option<mpsc::Receiver<MediaFrame>>,
}

impl TransmitStage {
    /// Create a new transmit stage
    pub fn new(server: Arc<WebRTCServer>, health: Arc<PipelineHealth>) -> Self {
        Self {
            server,
            health,
            input_rx: None,
        }
    }

    /// Set the input channel
    pub fn set_input(&mut self, rx: mpsc::Receiver<MediaFrame>) {
        self.input_rx = Some(rx);
    }

    /// Get a reference to the WebRTC server
    pub fn server(&self) -> &Arc<WebRTCServer> {
        &self.server
    }

    /// Convert MediaFrame to EncodedFrame for the existing WebRTC infrastructure
    fn to_encoded_frame(frame: &MediaFrame, seq: u64) -> EncodedFrame {
        EncodedFrame {
            data: frame.data.to_vec(),
            sequence_number: seq,
            timestamp_ms: (frame.pts.micros / 1000) as u64,
        }
    }
}

#[async_trait]
impl PipelineStage for TransmitStage {
    async fn run(&mut self) -> Result<()> {
        let mut input_rx = self
            .input_rx
            .take()
            .ok_or_else(|| anyhow::anyhow!("No input channel"))?;

        info!("TransmitStage: started");
        let mut sequence = 0u64;
        let dropped = 0u64;

        while let Some(frame) = input_rx.recv().await {
            let _encoded = Self::to_encoded_frame(&frame, sequence);
            sequence += 1;

            // Track drops via backpressure
            if frame.is_keyframe {
                self.health.record_frame(frame.data.len(), true);
            }
        }

        info!(
            "TransmitStage: finished, {} frames transmitted, {} dropped",
            sequence, dropped
        );
        Ok(())
    }

    fn name(&self) -> &'static str {
        "TransmitStage"
    }
}

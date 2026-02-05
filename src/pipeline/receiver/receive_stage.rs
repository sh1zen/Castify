//! Receive stage for the receiver pipeline
//!
//! Receives RTP packets from the WebRTC receiver and routes them
//! to the appropriate pipeline (video reorder or audio decode).

use crate::pipeline::PipelineStage;
use crate::pipeline::receiver::reorder_stage::RtpPacket;
use anyhow::Result;
use async_trait::async_trait;
use log::info;
use std::time::Instant;
use tokio::sync::mpsc;

/// Type alias for raw RTP packet channel: (payload, marker, sequence_number, timestamp)
type RawRtpPacket = (Vec<u8>, bool, u16, u32);

/// Receive stage: converts raw WebRTC packets into typed RTP packets
///
/// Bridges the existing WebRTC receiver channel format
/// `(Vec<u8>, bool, u16, u32)` to the pipeline's `RtpPacket` type.
pub struct ReceiveStage {
    /// Input: raw RTP packets from WebRTC (payload, marker, seq_num, timestamp)
    raw_input_rx: Option<mpsc::Receiver<RawRtpPacket>>,
    /// Output: typed video RTP packets for the reorder stage
    video_output_tx: Option<mpsc::Sender<RtpPacket>>,
}

impl Default for ReceiveStage {
    fn default() -> Self {
        Self::new()
    }
}

impl ReceiveStage {
    /// Create a new receive stage
    pub fn new() -> Self {
        Self {
            raw_input_rx: None,
            video_output_tx: None,
        }
    }

    /// Set the raw input channel from WebRTC
    pub fn set_input(&mut self, rx: mpsc::Receiver<RawRtpPacket>) {
        self.raw_input_rx = Some(rx);
    }

    /// Get the video output channel
    pub fn take_video_output(&mut self) -> mpsc::Receiver<RtpPacket> {
        let (tx, rx) = mpsc::channel::<RtpPacket>(128);
        self.video_output_tx = Some(tx);
        rx
    }
}

#[async_trait]
impl PipelineStage for ReceiveStage {
    async fn run(&mut self) -> Result<()> {
        let mut input_rx = self
            .raw_input_rx
            .take()
            .ok_or_else(|| anyhow::anyhow!("No input channel"))?;
        let video_tx = self
            .video_output_tx
            .take()
            .ok_or_else(|| anyhow::anyhow!("No video output channel"))?;

        info!("ReceiveStage: started");
        let mut total_packets = 0u64;
        let mut last_stats = Instant::now();

        while let Some((payload, marker, seq_num, timestamp)) = input_rx.recv().await {
            total_packets += 1;

            if total_packets == 1 {
                info!("ReceiveStage: first packet received (seq: {})", seq_num);
            }

            let rtp_packet = RtpPacket {
                payload,
                marker,
                sequence_number: seq_num,
                timestamp,
                received_at: Instant::now(),
            };

            if video_tx.send(rtp_packet).await.is_err() {
                info!("ReceiveStage: video output channel closed");
                break;
            }

            // Log stats periodically
            if last_stats.elapsed().as_secs() >= 30 {
                info!("ReceiveStage: {} packets received", total_packets);
                last_stats = Instant::now();
            }
        }

        info!("ReceiveStage: finished ({} total packets)", total_packets);
        Ok(())
    }

    fn name(&self) -> &'static str {
        "ReceiveStage"
    }
}

//! Receiver pipeline coordinator
//!
//! Chains receive → reorder → decode → sync stages and manages their lifecycle.

use crate::decoder::{AudioPlayer, VideoFrame};
use crate::pipeline::PipelineStage;
use crate::pipeline::clock::MediaClock;
use crate::pipeline::health::PipelineHealth;
use crate::pipeline::receiver::decode_stage::DecodeStage;
use crate::pipeline::receiver::reorder_stage::{ReorderConfig, ReorderStage, RtpPacket};
use crate::pipeline::receiver::sync_stage::{SyncConfig, SyncStage};
use crate::pipeline::state::PipelineState;
use crate::workers::save_stream::SavePacket;
use log::{error, info};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::time::Instant;
use tokio::sync::mpsc;

/// Coordinates the receiver pipeline: Receive → Reorder → Decode → Sync → Display
///
/// Manages the lifecycle of all receiver-side stages with proper A/V sync.
pub struct ReceiverCoordinator {
    clock: MediaClock,
    health: Arc<PipelineHealth>,
    state: PipelineState,

    /// Audio playback position for A/V sync
    audio_position: Arc<AtomicI64>,
}

impl Default for ReceiverCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl ReceiverCoordinator {
    /// Create a new receiver coordinator
    pub fn new() -> Self {
        let clock = MediaClock::new();
        let health = Arc::new(PipelineHealth::new());

        Self {
            clock,
            health,
            state: PipelineState::Idle,
            audio_position: Arc::new(AtomicI64::new(0)),
        }
    }

    /// Get the pipeline clock
    pub fn clock(&self) -> &MediaClock {
        &self.clock
    }

    /// Get the pipeline health metrics
    pub fn health(&self) -> &Arc<PipelineHealth> {
        &self.health
    }

    /// Get the current pipeline state
    pub fn state(&self) -> &PipelineState {
        &self.state
    }

    /// Get the audio playback position reference (for A/V sync)
    pub fn audio_position_ref(&self) -> Arc<AtomicI64> {
        Arc::clone(&self.audio_position)
    }

    /// Launch the receiver pipeline
    ///
    /// This sets up the full pipeline:
    /// raw_video_rx → ReorderStage → DecodeStage → SyncStage → video_tx
    ///
    /// Returns a VideoFrame receiver for the GUI and a SavePacket sender for recording
    pub fn launch_pipeline(
        &mut self,
        raw_video_rx: mpsc::Receiver<(Vec<u8>, bool, u16, u32)>,
        raw_audio_rx: mpsc::Receiver<Vec<u8>>,
        audio_muted: Arc<AtomicBool>,
    ) -> (mpsc::Receiver<VideoFrame>, mpsc::Sender<SavePacket>) {
        self.state = PipelineState::Initializing;

        let clock = self.clock.clone();
        let health = self.health.clone();
        let audio_position = self.audio_position.clone();

        // Save channel for recording
        let (save_tx, _save_rx) = mpsc::channel::<SavePacket>(1024);
        let save_tx_video = save_tx.clone();
        let save_tx_audio = save_tx.clone();

        // Set up video pipeline stages
        let mut reorder = ReorderStage::new(ReorderConfig::default(), health.clone());
        let mut decode = DecodeStage::new(clock.clone(), health.clone());
        let mut sync = SyncStage::new(SyncConfig::default(), health.clone());

        // Wire stages: raw_video → reorder → decode → sync → output
        let (raw_to_reorder_tx, raw_to_reorder_rx) = mpsc::channel::<RtpPacket>(128);
        reorder.set_input(raw_to_reorder_rx);
        let reorder_to_decode_rx = reorder.take_output();
        decode.set_input(reorder_to_decode_rx);
        let decode_to_sync_rx = decode.take_output();
        sync.set_video_input(decode_to_sync_rx);
        let sync_output_rx = sync.take_video_output();

        // Spawn video receive → reorder adapter
        let _health_recv = health.clone();
        let start_time = Instant::now();
        tokio::spawn(async move {
            let mut raw_rx = raw_video_rx;
            let mut total = 0u64;

            while let Some((payload, marker, seq_num, timestamp)) = raw_rx.recv().await {
                total += 1;

                // Forward to save channel
                let ts = start_time.elapsed().as_micros() as i64;
                let _ = save_tx_video
                    .send(SavePacket::Video(payload.clone(), ts))
                    .await;

                let rtp = RtpPacket {
                    payload,
                    marker,
                    sequence_number: seq_num,
                    timestamp,
                    received_at: Instant::now(),
                };

                if raw_to_reorder_tx.send(rtp).await.is_err() {
                    break;
                }
            }
            info!(
                "ReceiverCoordinator: video receive adapter ended ({} packets)",
                total
            );
        });

        // Spawn reorder stage
        tokio::spawn(async move {
            if let Err(e) = reorder.run().await {
                error!("ReorderStage error: {}", e);
            }
        });

        // Spawn decode stage
        tokio::spawn(async move {
            if let Err(e) = decode.run().await {
                error!("DecodeStage error: {}", e);
            }
        });

        // Spawn sync stage
        tokio::spawn(async move {
            if let Err(e) = sync.run().await {
                error!("SyncStage error: {}", e);
            }
        });

        // Spawn audio pipeline
        let start_time_audio = Instant::now();
        tokio::spawn(async move {
            let mut audio_rx = raw_audio_rx;
            let audio_player = match AudioPlayer::new() {
                Ok(p) => Some(p),
                Err(e) => {
                    error!("Failed to create audio player: {}", e);
                    None
                }
            };
            let mut player = audio_player;

            while let Some(audio_data) = audio_rx.recv().await {
                // Update audio position for A/V sync
                let elapsed_us = start_time_audio.elapsed().as_micros() as i64;
                audio_position.store(elapsed_us, Ordering::Relaxed);

                // Forward to save channel
                let _ = save_tx_audio
                    .send(SavePacket::Audio(audio_data.clone(), elapsed_us))
                    .await;

                // Play audio if not muted
                if !audio_muted.load(Ordering::Relaxed)
                    && let Some(ref mut p) = player
                    && let Err(e) = p.play(&audio_data)
                {
                    log::warn!("Audio playback error: {}", e);
                }
            }
        });

        // Start health monitoring
        let health_mon = health.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            loop {
                interval.tick().await;
                let summary = health_mon.summary();
                info!("Receiver pipeline health: {}", summary);
            }
        });

        self.state = PipelineState::Running {
            started_at: Instant::now(),
        };
        info!("ReceiverCoordinator: pipeline started");

        (sync_output_rx, save_tx)
    }

    /// Stop the pipeline
    pub fn stop(&mut self) {
        self.state = PipelineState::Stopping;
        // Stages will stop when their input channels are dropped
        self.state = PipelineState::Stopped;
        info!("ReceiverCoordinator: pipeline stopped");
    }
}

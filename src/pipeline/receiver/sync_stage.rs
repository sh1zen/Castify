//! A/V synchronization stage for the receiver pipeline
//!
//! Implements PTS-based playout scheduling using audio as the reference clock.
//! Video frames are released when their PTS is within tolerance of the audio
//! playback position.

use anyhow::Result;
use async_trait::async_trait;
use log::info;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

use crate::decoder::VideoFrame;
use crate::pipeline::PipelineStage;
use crate::pipeline::health::PipelineHealth;
use crate::pipeline::receiver::decode_stage::TimedVideoFrame;

/// Configuration for A/V synchronization
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Initial playout delay (buffering time before starting playback)
    pub playout_delay: Duration,
    /// Maximum A/V drift tolerance before correction
    pub max_drift: Duration,
    /// Frame tolerance: how far ahead of audio a video frame can be released
    pub frame_tolerance: Duration,
    /// Maximum number of video frames to buffer
    pub max_video_queue: usize,
    /// Maximum number of audio frames to buffer
    pub max_audio_queue: usize,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            playout_delay: Duration::from_millis(200), // Increased for better buffering
            max_drift: Duration::from_millis(100),     // Increased tolerance
            frame_tolerance: Duration::from_millis(66), // 2 frames at 30fps
            max_video_queue: 120,                      // Doubled video buffer
            max_audio_queue: 240,                      // Doubled audio buffer
        }
    }
}

/// Tracks audio playback position as the reference clock
///
/// Audio is the reference clock because:
/// - Audio glitches are more perceptible than video glitches
/// - Audio plays at a constant rate (sample-rate driven)
/// - Standard approach in media players (VLC, FFmpeg, etc.)
pub struct AudioPlaybackTracker {
    /// Current audio playback position in microseconds
    position_us: Arc<AtomicI64>,
    /// Whether audio playback has started
    started: bool,
}

impl Default for AudioPlaybackTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioPlaybackTracker {
    /// Create a new audio playback tracker
    pub fn new() -> Self {
        Self {
            position_us: Arc::new(AtomicI64::new(0)),
            started: false,
        }
    }

    /// Get a shared reference to the position atomic
    pub fn position_ref(&self) -> Arc<AtomicI64> {
        Arc::clone(&self.position_us)
    }

    /// Update the audio playback position
    pub fn update_position(&self, micros: i64) {
        self.position_us.store(micros, Ordering::Relaxed);
    }

    /// Get the current audio playback position
    pub fn position(&self) -> i64 {
        self.position_us.load(Ordering::Relaxed)
    }

    /// Mark audio as started
    pub fn mark_started(&mut self) {
        self.started = true;
    }

    /// Check if audio has started
    pub fn is_started(&self) -> bool {
        self.started
    }
}

/// A/V synchronization stage
///
/// Uses audio playback position as the reference clock to schedule
/// video frame release. Video frames are buffered and released when
/// their PTS is within tolerance of the audio position.
///
/// # Algorithm
///
/// 1. Buffer incoming video frames in a PTS-ordered queue
/// 2. Track audio playback position via AtomicI64
/// 3. On each tick, check if the front video frame's PTS <= audio_position + frame_tolerance
/// 4. If yes, release the video frame
/// 5. If video is too far behind audio, drop video frames to catch up
/// 6. If video is too far ahead of audio, wait
pub struct SyncStage {
    /// Video frame queue ordered by PTS
    video_queue: VecDeque<TimedVideoFrame>,
    /// Audio playback position tracker
    audio_tracker: AudioPlaybackTracker,
    /// Configuration
    config: SyncConfig,
    /// Health metrics
    health: Arc<PipelineHealth>,
    /// Input: decoded video frames
    video_input_rx: Option<mpsc::Receiver<TimedVideoFrame>>,
    /// Output: synchronized video frames
    video_output_tx: Option<mpsc::Sender<VideoFrame>>,
    /// Playout start time
    playout_start: Option<Instant>,
    /// Statistics
    frames_released: u64,
    frames_dropped: u64,
}

impl SyncStage {
    /// Create a new sync stage
    pub fn new(config: SyncConfig, health: Arc<PipelineHealth>) -> Self {
        Self {
            video_queue: VecDeque::with_capacity(config.max_video_queue),
            audio_tracker: AudioPlaybackTracker::new(),
            config,
            health,
            video_input_rx: None,
            video_output_tx: None,
            playout_start: None,
            frames_released: 0,
            frames_dropped: 0,
        }
    }

    /// Get a shared reference to the audio position tracker
    pub fn audio_position_ref(&self) -> Arc<AtomicI64> {
        self.audio_tracker.position_ref()
    }

    /// Set the video input channel
    pub fn set_video_input(&mut self, rx: mpsc::Receiver<TimedVideoFrame>) {
        self.video_input_rx = Some(rx);
    }

    /// Get the video output channel
    pub fn take_video_output(&mut self) -> mpsc::Receiver<VideoFrame> {
        let (tx, rx) = mpsc::channel::<VideoFrame>(3);
        self.video_output_tx = Some(tx);
        rx
    }

    /// Process video queue: release frames whose PTS is ready
    fn process_video_queue(&mut self) -> Vec<VideoFrame> {
        let mut output = Vec::new();
        let audio_pos_us = self.audio_tracker.position();

        // If audio hasn't started yet, check playout delay
        if !self.audio_tracker.is_started()
            && let Some(start) = self.playout_start
            && start.elapsed() < self.config.playout_delay
        {
            return output; // Still buffering
        }
        // After playout delay, release frames based on wall clock

        let tolerance_us = self.config.frame_tolerance.as_micros() as i64;
        let max_drift_us = self.config.max_drift.as_micros() as i64;

        while let Some(front) = self.video_queue.front() {
            let video_pts_us = front.pts.micros;

            if self.audio_tracker.is_started() {
                // Audio-based sync
                if video_pts_us <= audio_pos_us + tolerance_us {
                    // Frame is ready to display
                    let frame = self.video_queue.pop_front().unwrap();

                    // Check if frame is too old (behind audio by more than max_drift)
                    if audio_pos_us - video_pts_us > max_drift_us {
                        // Frame is too late, drop it
                        self.frames_dropped += 1;
                        self.health.record_frame_drop();
                        continue; // Check next frame
                    }

                    self.frames_released += 1;
                    output.push(frame.frame);
                    break; // Release one frame per tick
                } else {
                    // Video is ahead of audio, wait
                    break;
                }
            } else {
                // No audio reference - release immediately (passthrough mode)
                let frame = self.video_queue.pop_front().unwrap();
                self.frames_released += 1;
                output.push(frame.frame);
                break;
            }
        }

        // Drop excess frames if queue is too large
        while self.video_queue.len() > self.config.max_video_queue {
            self.video_queue.pop_front();
            self.frames_dropped += 1;
            self.health.record_frame_drop();
        }

        output
    }
}

#[async_trait]
impl PipelineStage for SyncStage {
    async fn run(&mut self) -> Result<()> {
        let mut video_input = self
            .video_input_rx
            .take()
            .ok_or_else(|| anyhow::anyhow!("No video input channel"))?;
        let video_output = self
            .video_output_tx
            .take()
            .ok_or_else(|| anyhow::anyhow!("No video output channel"))?;

        info!(
            "SyncStage: started (playout delay: {:?})",
            self.config.playout_delay
        );
        self.playout_start = Some(Instant::now());

        let mut last_stats_log = Instant::now();
        let sync_tick = Duration::from_millis(5); // Check sync every 5ms

        loop {
            tokio::select! {
                frame = video_input.recv() => {
                    match frame {
                        Some(timed_frame) => {
                            self.video_queue.push_back(timed_frame);

                            // Process queue
                            for vf in self.process_video_queue() {
                                if video_output.send(vf).await.is_err() {
                                    info!("SyncStage: output channel closed");
                                    return Ok(());
                                }
                            }
                        }
                        None => {
                            info!("SyncStage: video input closed");
                            // Flush remaining
                            while let Some(front) = self.video_queue.pop_front() {
                                let _ = video_output.send(front.frame).await;
                            }
                            break;
                        }
                    }
                }
                _ = tokio::time::sleep(sync_tick) => {
                    // Periodic sync check
                    for vf in self.process_video_queue() {
                        if video_output.send(vf).await.is_err() {
                            return Ok(());
                        }
                    }
                }
            }

            // Log stats periodically
            if last_stats_log.elapsed().as_secs() >= 30 {
                let audio_pos = self.audio_tracker.position();
                let video_queue_len = self.video_queue.len();
                let front_pts = self.video_queue.front().map(|f| f.pts.micros).unwrap_or(0);
                let drift_us = if audio_pos > 0 && front_pts > 0 {
                    (front_pts - audio_pos).abs()
                } else {
                    0
                };

                info!(
                    "SyncStage: {} released, {} dropped, queue: {}, drift: {}µs, audio_pos: {}µs",
                    self.frames_released, self.frames_dropped, video_queue_len, drift_us, audio_pos,
                );
                last_stats_log = Instant::now();
            }
        }

        info!(
            "SyncStage: finished ({} released, {} dropped)",
            self.frames_released, self.frames_dropped
        );
        Ok(())
    }

    fn name(&self) -> &'static str {
        "SyncStage"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::Timestamp;

    fn make_timed_frame(pts_us: i64, w: u32, h: u32) -> TimedVideoFrame {
        TimedVideoFrame {
            frame: VideoFrame {
                data: vec![0u8; (w * h * 3 / 2) as usize],
                width: w,
                height: h,
            },
            pts: Timestamp::from_micros(pts_us),
            correlation_id: 0,
            is_keyframe: false,
        }
    }

    #[test]
    fn test_sync_passthrough_no_audio() {
        let config = SyncConfig {
            playout_delay: Duration::from_millis(0),
            ..Default::default()
        };
        let health = Arc::new(PipelineHealth::new());
        let mut stage = SyncStage::new(config, health);
        stage.playout_start = Some(Instant::now());

        stage.video_queue.push_back(make_timed_frame(0, 320, 240));
        stage
            .video_queue
            .push_back(make_timed_frame(33000, 320, 240));

        // Without audio, frames should be released immediately
        let output = stage.process_video_queue();
        assert_eq!(output.len(), 1);
    }

    #[test]
    fn test_sync_with_audio_reference() {
        let config = SyncConfig {
            playout_delay: Duration::from_millis(0),
            frame_tolerance: Duration::from_millis(33),
            max_drift: Duration::from_millis(50),
            ..Default::default()
        };
        let health = Arc::new(PipelineHealth::new());
        let mut stage = SyncStage::new(config, health);
        stage.playout_start = Some(Instant::now());
        stage.audio_tracker.mark_started();

        // Audio at 100ms
        stage.audio_tracker.update_position(100_000);

        // Video at 50ms - should be released (behind audio, within tolerance)
        stage
            .video_queue
            .push_back(make_timed_frame(50_000, 320, 240));

        // Video at 200ms - should NOT be released (ahead of audio)
        stage
            .video_queue
            .push_back(make_timed_frame(200_000, 320, 240));

        let output = stage.process_video_queue();
        assert_eq!(output.len(), 1);

        // Queue should still have the 200ms frame
        assert_eq!(stage.video_queue.len(), 1);
    }

    #[test]
    fn test_sync_drops_late_frames() {
        let config = SyncConfig {
            playout_delay: Duration::from_millis(0),
            frame_tolerance: Duration::from_millis(33),
            max_drift: Duration::from_millis(50),
            ..Default::default()
        };
        let health = Arc::new(PipelineHealth::new());
        let mut stage = SyncStage::new(config, health);
        stage.playout_start = Some(Instant::now());
        stage.audio_tracker.mark_started();

        // Audio at 500ms
        stage.audio_tracker.update_position(500_000);

        // Video at 100ms - should be dropped (too far behind audio, > 50ms drift)
        stage
            .video_queue
            .push_back(make_timed_frame(100_000, 320, 240));
        // Video at 480ms - should be released (within tolerance)
        stage
            .video_queue
            .push_back(make_timed_frame(480_000, 320, 240));

        let output = stage.process_video_queue();
        assert_eq!(output.len(), 1);
        assert_eq!(stage.frames_dropped, 1);
    }
}

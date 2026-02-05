//! Health monitoring and metrics for pipeline

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::mpsc;

/// Health metrics for a pipeline
///
/// Tracks various counters and timestamps to monitor pipeline health.
/// All fields use atomic operations for thread-safe access.
pub struct PipelineHealth {
    /// Number of frames dropped due to backpressure or errors
    pub frame_drops: AtomicU64,

    /// Number of decode failures
    pub decode_failures: AtomicU64,

    /// Number of network errors
    pub network_errors: AtomicU64,

    /// Timestamp (as Unix microseconds) of the last successfully processed frame
    pub last_frame_time: AtomicU64,

    /// Number of frames successfully processed
    pub frames_processed: AtomicU64,

    /// Total bytes of data processed
    pub bytes_processed: AtomicU64,

    /// Number of keyframes processed
    pub keyframes_processed: AtomicU64,
}

impl PipelineHealth {
    /// Create a new health metrics instance
    pub fn new() -> Self {
        let now_micros = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;
        Self {
            frame_drops: AtomicU64::new(0),
            decode_failures: AtomicU64::new(0),
            network_errors: AtomicU64::new(0),
            last_frame_time: AtomicU64::new(now_micros),
            frames_processed: AtomicU64::new(0),
            bytes_processed: AtomicU64::new(0),
            keyframes_processed: AtomicU64::new(0),
        }
    }

    /// Record a dropped frame
    pub fn record_frame_drop(&self) {
        self.frame_drops.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a decode failure
    pub fn record_decode_failure(&self) {
        self.decode_failures.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a network error
    pub fn record_network_error(&self) {
        self.network_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a successfully processed frame
    pub fn record_frame(&self, size: usize, is_keyframe: bool) {
        let now_micros = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;
        self.last_frame_time.store(now_micros, Ordering::Relaxed);
        self.frames_processed.fetch_add(1, Ordering::Relaxed);
        self.bytes_processed
            .fetch_add(size as u64, Ordering::Relaxed);
        if is_keyframe {
            self.keyframes_processed.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Get the number of frame drops
    pub fn frame_drops(&self) -> u64 {
        self.frame_drops.load(Ordering::Relaxed)
    }

    /// Get the number of decode failures
    pub fn decode_failures(&self) -> u64 {
        self.decode_failures.load(Ordering::Relaxed)
    }

    /// Get the number of network errors
    pub fn network_errors(&self) -> u64 {
        self.network_errors.load(Ordering::Relaxed)
    }

    /// Get the timestamp of the last frame (Unix microseconds)
    pub fn last_frame_time(&self) -> u64 {
        self.last_frame_time.load(Ordering::Relaxed)
    }

    /// Get the number of frames processed
    pub fn frames_processed(&self) -> u64 {
        self.frames_processed.load(Ordering::Relaxed)
    }

    /// Get the total bytes processed
    pub fn bytes_processed(&self) -> u64 {
        self.bytes_processed.load(Ordering::Relaxed)
    }

    /// Get the number of keyframes processed
    pub fn keyframes_processed(&self) -> u64 {
        self.keyframes_processed.load(Ordering::Relaxed)
    }

    /// Calculate the frame drop rate as a percentage
    pub fn frame_drop_rate(&self) -> f64 {
        let drops = self.frame_drops();
        let processed = self.frames_processed();
        if processed == 0 {
            return 0.0;
        }
        (drops as f64 / processed as f64) * 100.0
    }

    /// Check if the pipeline has stalled (no frames for given duration)
    pub fn is_stalled(&self, threshold: Duration) -> bool {
        let last_frame = self.last_frame_time();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;
        let elapsed_micros = now.saturating_sub(last_frame);
        elapsed_micros > threshold.as_micros() as u64
    }

    /// Get a summary of health metrics
    pub fn summary(&self) -> HealthSummary {
        HealthSummary {
            frames_processed: self.frames_processed(),
            frame_drops: self.frame_drops(),
            decode_failures: self.decode_failures(),
            network_errors: self.network_errors(),
            bytes_processed: self.bytes_processed(),
            keyframes_processed: self.keyframes_processed(),
            frame_drop_rate: self.frame_drop_rate(),
        }
    }
}

impl Default for PipelineHealth {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of health metrics
#[derive(Debug, Clone)]
pub struct HealthSummary {
    pub frames_processed: u64,
    pub frame_drops: u64,
    pub decode_failures: u64,
    pub network_errors: u64,
    pub bytes_processed: u64,
    pub keyframes_processed: u64,
    pub frame_drop_rate: f64,
}

impl std::fmt::Display for HealthSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Health: {} frames ({} drops, {:.2}%), {} decode failures, {} network errors, {} bytes, {} keyframes",
            self.frames_processed,
            self.frame_drops,
            self.frame_drop_rate,
            self.decode_failures,
            self.network_errors,
            self.bytes_processed,
            self.keyframes_processed
        )
    }
}

/// Health alert types
#[derive(Debug, Clone)]
pub enum HealthAlert {
    /// Pipeline has stalled (no frames for threshold duration)
    Stalled { duration: Duration },

    /// High frame drop rate detected
    HighDropRate { rate: f64 },

    /// Multiple decode failures
    DecodeFailures { count: u64 },

    /// Network errors detected
    NetworkErrors { count: u64 },
}

impl std::fmt::Display for HealthAlert {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthAlert::Stalled { duration } => {
                write!(f, "Pipeline stalled for {:?}", duration)
            }
            HealthAlert::HighDropRate { rate } => {
                write!(f, "High frame drop rate: {:.2}%", rate)
            }
            HealthAlert::DecodeFailures { count } => {
                write!(f, "Decode failures: {}", count)
            }
            HealthAlert::NetworkErrors { count } => {
                write!(f, "Network errors: {}", count)
            }
        }
    }
}

/// Health monitoring service
///
/// Periodically checks pipeline health and sends alerts when issues are detected.
pub struct HealthMonitor {
    health: Arc<PipelineHealth>,
    alert_tx: mpsc::Sender<HealthAlert>,
    check_interval: Duration,
    stall_threshold: Duration,
    drop_rate_threshold: f64,
}

impl HealthMonitor {
    /// Create a new health monitor
    pub fn new(health: Arc<PipelineHealth>, alert_tx: mpsc::Sender<HealthAlert>) -> Self {
        Self {
            health,
            alert_tx,
            check_interval: Duration::from_secs(5),
            stall_threshold: Duration::from_secs(5),
            drop_rate_threshold: 10.0, // 10% drop rate
        }
    }

    /// Configure the check interval
    pub fn with_check_interval(mut self, interval: Duration) -> Self {
        self.check_interval = interval;
        self
    }

    /// Configure the stall threshold
    pub fn with_stall_threshold(mut self, threshold: Duration) -> Self {
        self.stall_threshold = threshold;
        self
    }

    /// Configure the drop rate threshold
    pub fn with_drop_rate_threshold(mut self, threshold: f64) -> Self {
        self.drop_rate_threshold = threshold;
        self
    }

    /// Run the health monitor (blocking loop)
    pub async fn run(&self) {
        let mut interval = tokio::time::interval(self.check_interval);
        let mut last_decode_failures = 0u64;
        let mut last_network_errors = 0u64;

        loop {
            interval.tick().await;

            // Check for stall
            if self.health.is_stalled(self.stall_threshold) {
                let _ = self
                    .alert_tx
                    .send(HealthAlert::Stalled {
                        duration: self.stall_threshold,
                    })
                    .await;
            }

            // Check for high drop rate
            let drop_rate = self.health.frame_drop_rate();
            if drop_rate > self.drop_rate_threshold {
                let _ = self
                    .alert_tx
                    .send(HealthAlert::HighDropRate { rate: drop_rate })
                    .await;
            }

            // Check for new decode failures
            let decode_failures = self.health.decode_failures();
            if decode_failures > last_decode_failures {
                let _ = self
                    .alert_tx
                    .send(HealthAlert::DecodeFailures {
                        count: decode_failures - last_decode_failures,
                    })
                    .await;
                last_decode_failures = decode_failures;
            }

            // Check for new network errors
            let network_errors = self.health.network_errors();
            if network_errors > last_network_errors {
                let _ = self
                    .alert_tx
                    .send(HealthAlert::NetworkErrors {
                        count: network_errors - last_network_errors,
                    })
                    .await;
                last_network_errors = network_errors;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_metrics() {
        let health = PipelineHealth::new();

        // Record some frames
        health.record_frame(1000, false);
        health.record_frame(2000, true);
        health.record_frame(1500, false);

        assert_eq!(health.frames_processed(), 3);
        assert_eq!(health.bytes_processed(), 4500);
        assert_eq!(health.keyframes_processed(), 1);
        assert_eq!(health.frame_drops(), 0);

        // Record some drops
        health.record_frame_drop();
        health.record_frame_drop();

        assert_eq!(health.frame_drops(), 2);
        assert!(health.frame_drop_rate() > 0.0);
    }

    #[test]
    fn test_stall_detection() {
        let health = PipelineHealth::new();

        // Should not be stalled immediately
        assert!(!health.is_stalled(Duration::from_secs(1)));

        // Record a frame to update last_frame_time
        health.record_frame(1000, false);

        // Simulate stall by not recording frames
        std::thread::sleep(Duration::from_millis(150));

        // Should be stalled after 150ms if threshold is 100ms
        assert!(health.is_stalled(Duration::from_millis(100)));
    }
}

//! Media clock for audio-video synchronization

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{Duration, Instant};

use super::types::Timestamp;

/// Media clock for timestamp correlation
///
/// Provides a unified time base for audio and video streams to enable
/// proper synchronization. The clock maintains:
/// - A base instant (when the clock started)
/// - Separate offsets for audio and video to compensate for pipeline delays
/// - Methods to convert between wall-clock time and media timestamps
///
/// # Design
///
/// The clock uses a base `Instant` as the reference point, and all timestamps
/// are calculated relative to this base. Each media type (audio/video) can
/// have an offset to account for different pipeline latencies.
///
/// # Thread Safety
///
/// The clock is thread-safe and can be cloned via Arc. Offsets use atomic
/// operations for lock-free updates.
#[derive(Clone)]
pub struct MediaClock {
    /// Base instant when the clock started
    base: Arc<Instant>,

    /// Offset for video timestamps (microseconds)
    video_offset: Arc<AtomicI64>,

    /// Offset for audio timestamps (microseconds)
    audio_offset: Arc<AtomicI64>,

    /// Correlation ID counter
    correlation_counter: Arc<AtomicI64>,
}

impl MediaClock {
    /// Create a new media clock starting now
    pub fn new() -> Self {
        Self {
            base: Arc::new(Instant::now()),
            video_offset: Arc::new(AtomicI64::new(0)),
            audio_offset: Arc::new(AtomicI64::new(0)),
            correlation_counter: Arc::new(AtomicI64::new(0)),
        }
    }

    /// Create a new media clock with a specific base instant
    pub fn with_base(base: Instant) -> Self {
        Self {
            base: Arc::new(base),
            video_offset: Arc::new(AtomicI64::new(0)),
            audio_offset: Arc::new(AtomicI64::new(0)),
            correlation_counter: Arc::new(AtomicI64::new(0)),
        }
    }

    /// Get the base instant
    pub fn base(&self) -> Instant {
        *self.base
    }

    /// Get the current timestamp for video
    pub fn video_now(&self) -> Timestamp {
        let elapsed = self.base.elapsed();
        let offset = self.video_offset.load(Ordering::Relaxed);
        Timestamp::from_micros(elapsed.as_micros() as i64 + offset)
    }

    /// Get the current timestamp for audio
    pub fn audio_now(&self) -> Timestamp {
        let elapsed = self.base.elapsed();
        let offset = self.audio_offset.load(Ordering::Relaxed);
        Timestamp::from_micros(elapsed.as_micros() as i64 + offset)
    }

    /// Get a timestamp relative to the clock base
    pub fn timestamp_from_instant(&self, instant: Instant) -> Timestamp {
        Timestamp::from_instant(instant, *self.base)
    }

    /// Get a timestamp from a duration since the clock started
    pub fn timestamp_from_duration(&self, duration: Duration) -> Timestamp {
        Timestamp::from_duration(duration)
    }

    /// Set the video offset
    pub fn set_video_offset(&self, offset: Duration) {
        self.video_offset
            .store(offset.as_micros() as i64, Ordering::Relaxed);
    }

    /// Set the audio offset
    pub fn set_audio_offset(&self, offset: Duration) {
        self.audio_offset
            .store(offset.as_micros() as i64, Ordering::Relaxed);
    }

    /// Adjust the video offset by a delta
    pub fn adjust_video_offset(&self, delta: Duration, subtract: bool) {
        let delta_micros = delta.as_micros() as i64;
        let adjustment = if subtract {
            -delta_micros
        } else {
            delta_micros
        };
        self.video_offset.fetch_add(adjustment, Ordering::Relaxed);
    }

    /// Adjust the audio offset by a delta
    pub fn adjust_audio_offset(&self, delta: Duration, subtract: bool) {
        let delta_micros = delta.as_micros() as i64;
        let adjustment = if subtract {
            -delta_micros
        } else {
            delta_micros
        };
        self.audio_offset.fetch_add(adjustment, Ordering::Relaxed);
    }

    /// Get the current video offset
    pub fn video_offset(&self) -> Duration {
        let offset = self.video_offset.load(Ordering::Relaxed);
        if offset >= 0 {
            Duration::from_micros(offset as u64)
        } else {
            Duration::from_micros(0)
        }
    }

    /// Get the current audio offset
    pub fn audio_offset(&self) -> Duration {
        let offset = self.audio_offset.load(Ordering::Relaxed);
        if offset >= 0 {
            Duration::from_micros(offset as u64)
        } else {
            Duration::from_micros(0)
        }
    }

    /// Generate a new correlation ID
    ///
    /// Correlation IDs are used to match audio and video frames that were
    /// captured at the same instant. Frames with the same correlation ID
    /// should be synchronized.
    pub fn next_correlation_id(&self) -> u64 {
        self.correlation_counter.fetch_add(1, Ordering::Relaxed) as u64
    }

    /// Calculate the A/V sync offset (video timestamp - audio timestamp)
    ///
    /// Positive values mean video is ahead of audio.
    /// Negative values mean audio is ahead of video.
    pub fn av_sync_offset(&self) -> Duration {
        let video_offset = self.video_offset.load(Ordering::Relaxed);
        let audio_offset = self.audio_offset.load(Ordering::Relaxed);
        let diff = video_offset - audio_offset;

        if diff >= 0 {
            Duration::from_micros(diff as u64)
        } else {
            Duration::from_micros((-diff) as u64)
        }
    }

    /// Check if A/V sync is within tolerance
    pub fn is_synced(&self, tolerance: Duration) -> bool {
        self.av_sync_offset() <= tolerance
    }

    /// Synchronize video to audio by adjusting video offset
    ///
    /// This adjusts the video timestamp to match the audio timestamp,
    /// bringing them back into sync.
    pub fn sync_video_to_audio(&self) {
        let audio_offset = self.audio_offset.load(Ordering::Relaxed);
        self.video_offset.store(audio_offset, Ordering::Relaxed);
    }

    /// Synchronize audio to video by adjusting audio offset
    ///
    /// This adjusts the audio timestamp to match the video timestamp,
    /// bringing them back into sync.
    pub fn sync_audio_to_video(&self) {
        let video_offset = self.video_offset.load(Ordering::Relaxed);
        self.audio_offset.store(video_offset, Ordering::Relaxed);
    }

    /// Reset both offsets to zero
    pub fn reset_offsets(&self) {
        self.video_offset.store(0, Ordering::Relaxed);
        self.audio_offset.store(0, Ordering::Relaxed);
    }
}

impl Default for MediaClock {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for MediaClock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MediaClock")
            .field("elapsed", &self.base.elapsed())
            .field("video_offset", &self.video_offset())
            .field("audio_offset", &self.audio_offset())
            .field("av_sync_offset", &self.av_sync_offset())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_clock_basic() {
        let clock = MediaClock::new();

        // Small delay to ensure some time has passed
        thread::sleep(Duration::from_millis(10));

        let video_ts = clock.video_now();
        let audio_ts = clock.audio_now();

        // Both should be positive and relatively close
        assert!(video_ts.micros > 0);
        assert!(audio_ts.micros > 0);
        // No offsets initially, should be within 1ms of each other
        let diff = (video_ts.micros - audio_ts.micros).abs();
        assert!(diff < 1000, "Timestamps differ by {} microseconds", diff);
    }

    #[test]
    fn test_clock_offsets() {
        let clock = MediaClock::new();

        // Set different offsets
        clock.set_video_offset(Duration::from_millis(100));
        clock.set_audio_offset(Duration::from_millis(50));

        let video_ts = clock.video_now();
        let audio_ts = clock.audio_now();

        // Video should be 50ms ahead of audio
        let diff = video_ts.micros - audio_ts.micros;
        assert!((diff - 50_000).abs() < 1000); // Within 1ms tolerance
    }

    #[test]
    fn test_correlation_id() {
        let clock = MediaClock::new();

        let id1 = clock.next_correlation_id();
        let id2 = clock.next_correlation_id();
        let id3 = clock.next_correlation_id();

        // IDs should be sequential
        assert_eq!(id1 + 1, id2);
        assert_eq!(id2 + 1, id3);
    }

    #[test]
    fn test_av_sync() {
        let clock = MediaClock::new();

        // Start in sync
        assert!(clock.is_synced(Duration::from_millis(1)));

        // Create offset
        clock.set_video_offset(Duration::from_millis(100));

        // Should not be synced
        assert!(!clock.is_synced(Duration::from_millis(10)));

        // Sync video to audio (both at 0)
        clock.sync_video_to_audio();

        // Should be in sync again
        assert!(clock.is_synced(Duration::from_millis(1)));
    }

    #[test]
    fn test_adjust_offset() {
        let clock = MediaClock::new();

        clock.set_video_offset(Duration::from_millis(100));
        assert_eq!(clock.video_offset(), Duration::from_millis(100));

        // Add 50ms
        clock.adjust_video_offset(Duration::from_millis(50), false);
        assert_eq!(clock.video_offset(), Duration::from_millis(150));

        // Subtract 30ms
        clock.adjust_video_offset(Duration::from_millis(30), true);
        assert_eq!(clock.video_offset(), Duration::from_millis(120));
    }
}

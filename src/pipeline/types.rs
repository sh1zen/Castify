//! Core types for the pipeline system

use bytes::Bytes;
use std::time::{Duration, Instant};

/// Timestamp representation for media frames
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp {
    /// Microseconds since pipeline start
    pub micros: i64,
}

impl Timestamp {
    /// Create a new timestamp from microseconds
    pub fn from_micros(micros: i64) -> Self {
        Self { micros }
    }

    /// Create a timestamp from duration since epoch
    pub fn from_duration(duration: Duration) -> Self {
        Self {
            micros: duration.as_micros() as i64,
        }
    }

    /// Create a timestamp from instant relative to base
    pub fn from_instant(instant: Instant, base: Instant) -> Self {
        let duration = instant.saturating_duration_since(base);
        Self::from_duration(duration)
    }

    /// Convert to duration
    pub fn as_duration(&self) -> Duration {
        Duration::from_micros(self.micros as u64)
    }

    /// Add a duration to this timestamp
    pub fn add(&self, duration: Duration) -> Self {
        Self {
            micros: self.micros + duration.as_micros() as i64,
        }
    }

    /// Subtract a duration from this timestamp
    pub fn sub(&self, duration: Duration) -> Self {
        Self {
            micros: self.micros - duration.as_micros() as i64,
        }
    }

    /// Calculate the difference between two timestamps
    pub fn diff(&self, other: Timestamp) -> Duration {
        let diff_micros = (self.micros - other.micros).abs();
        Duration::from_micros(diff_micros as u64)
    }
}

impl std::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}µs", self.micros)
    }
}

/// Kind of media data
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    /// Video frame data
    Video,
    /// Audio sample data
    Audio,
}

impl std::fmt::Display for MediaKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MediaKind::Video => write!(f, "Video"),
            MediaKind::Audio => write!(f, "Audio"),
        }
    }
}

/// Unified media frame with timestamp correlation
///
/// This structure represents a single frame of media data (video or audio)
/// with associated timing information for synchronization.
#[derive(Clone)]
pub struct MediaFrame {
    /// Kind of media (video or audio)
    pub kind: MediaKind,

    /// Encoded or raw media data
    pub data: Bytes,

    /// Presentation timestamp - when this frame should be displayed/played
    pub pts: Timestamp,

    /// Decode timestamp - when this frame should be decoded
    /// For video: may differ from PTS due to B-frames
    /// For audio: typically same as PTS
    pub dts: Timestamp,

    /// Correlation ID to match audio and video frames from the same capture instant
    /// Frames captured at the same time will have the same correlation_id
    pub correlation_id: u64,

    /// Whether this is a keyframe (for video) or important sync point
    pub is_keyframe: bool,

    /// Frame width (for video only)
    pub width: Option<u32>,

    /// Frame height (for video only)
    pub height: Option<u32>,

    /// Sample rate (for audio only)
    pub sample_rate: Option<u32>,

    /// Number of channels (for audio only)
    pub channels: Option<u16>,
}

impl MediaFrame {
    /// Create a new video frame
    pub fn video(
        data: Bytes,
        pts: Timestamp,
        dts: Timestamp,
        correlation_id: u64,
        is_keyframe: bool,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            kind: MediaKind::Video,
            data,
            pts,
            dts,
            correlation_id,
            is_keyframe,
            width: Some(width),
            height: Some(height),
            sample_rate: None,
            channels: None,
        }
    }

    /// Create a new audio frame
    pub fn audio(
        data: Bytes,
        pts: Timestamp,
        correlation_id: u64,
        sample_rate: u32,
        channels: u16,
    ) -> Self {
        Self {
            kind: MediaKind::Audio,
            data,
            pts,
            dts: pts, // Audio DTS = PTS
            correlation_id,
            is_keyframe: false,
            width: None,
            height: None,
            sample_rate: Some(sample_rate),
            channels: Some(channels),
        }
    }

    /// Get the size of the frame data in bytes
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

impl std::fmt::Debug for MediaFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_struct("MediaFrame");
        debug
            .field("kind", &self.kind)
            .field("pts", &self.pts)
            .field("dts", &self.dts)
            .field("correlation_id", &self.correlation_id)
            .field("is_keyframe", &self.is_keyframe)
            .field("size", &self.size());

        if let Some(width) = self.width {
            debug.field("width", &width);
        }
        if let Some(height) = self.height {
            debug.field("height", &height);
        }
        if let Some(sample_rate) = self.sample_rate {
            debug.field("sample_rate", &sample_rate);
        }
        if let Some(channels) = self.channels {
            debug.field("channels", &channels);
        }

        debug.finish()
    }
}

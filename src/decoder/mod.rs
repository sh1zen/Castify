//! Video and audio decoding module
//!
//! Provides H.264 video decoding via FFmpeg and audio playback via cpal.

mod depacketizer;
mod ffmpeg;

pub mod audio;

/// Decoded video frame with raw pixel data.
#[derive(Debug, Clone)]
pub struct VideoFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub use audio::AudioPlayer;
pub use depacketizer::H264Depacketizer;
pub use ffmpeg::FfmpegDecoder;

// Re-export FrameData from encoder for convenience
pub use crate::encoder::FrameData;

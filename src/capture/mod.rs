//! Screen capture module
//!
//! Provides cross-platform screen capture functionality through platform-specific
//! implementations (Windows Graphics Capture on Windows, generic fallback elsewhere).

#[cfg(target_os = "windows")]
mod wgc;
#[cfg(target_os = "windows")]
pub use wgc::WGCScreenCapture as ScreenCaptureImpl;

#[cfg(not(target_os = "windows"))]
mod generic;
#[cfg(not(target_os = "windows"))]
pub use generic::GenericScreenCapture as ScreenCaptureImpl;

pub mod audio;
pub mod capturer;
pub mod display;
mod traits;
#[cfg(target_os = "windows")]
mod yuv_convert;

pub struct YUVFrame {
    pub display_time: u64,
    pub width: i32,
    pub height: i32,
    pub luminance_bytes: Vec<u8>,
    pub luminance_stride: i32,
    pub chrominance_bytes: Vec<u8>,
    pub chrominance_stride: i32,
}

unsafe impl Send for YUVFrame {}

pub struct NV12FrameRef<'a> {
    pub luminance_bytes: &'a [u8],
    pub luminance_stride: i32,
    pub chrominance_bytes: &'a [u8],
    pub chrominance_stride: i32,
}

pub use capturer::{CaptureOpts, CropRect};
pub use traits::{DisplayInfo, ScreenCapture};
#[cfg(target_os = "windows")]
pub use yuv_convert::YuvConverter;

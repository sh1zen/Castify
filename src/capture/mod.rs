use async_trait::async_trait;

#[async_trait]
pub trait ScreenCapture {
    fn new_default() -> Result<ScreenCaptureImpl, anyhow::Error>;

    fn display(&self) -> &dyn DisplayInfo;

    async fn start_capture(
        &mut self,
        encoder: FfmpegEncoder,
        output: tokio::sync::mpsc::Sender<bytes::Bytes>,
    ) -> Result<(), anyhow::Error>;

    async fn stop_capture(&mut self) -> Result<(), anyhow::Error>;
}

pub trait DisplayInfo {
    /// Get the resolution of the display in (width, height)
    fn resolution(&self) -> (u32, u32);
    /// Get the DPI factor for input handling
    fn dpi_conversion_factor(&self) -> f64;
}

use crate::encoder::FfmpegEncoder;

#[cfg(target_os = "windows")]
mod wgc;
#[cfg(target_os = "windows")]
pub use wgc::WGCScreenCapture as ScreenCaptureImpl;

pub mod capturer;
mod frame;
#[cfg(target_os = "macos")]
mod macos;

pub use frame::YUVFrame;
#[cfg(target_os = "macos")]
pub use macos::MacOSCapture as ScreenCaptureImpl;

mod audio;
pub mod display;
mod yuv_convert;

#[allow(unused_imports)]
pub use yuv_convert::YuvConverter;
//! Traits for screen capture functionality

use crate::capture::ScreenCaptureImpl;
use crate::encoder::FfmpegEncoder;
use async_trait::async_trait;
use tokio::sync::watch;

/// Trait for screen capture implementations
#[async_trait]
pub trait ScreenCapture {
    fn new_default() -> Result<ScreenCaptureImpl, anyhow::Error>;

    fn display(&self) -> &dyn DisplayInfo;

    async fn start_capture(
        &mut self,
        encoder: FfmpegEncoder,
        output: tokio::sync::mpsc::Sender<bytes::Bytes>,
        opts_rx: watch::Receiver<super::capturer::CaptureOpts>,
    ) -> Result<(), anyhow::Error>;

    async fn stop_capture(&mut self) -> Result<(), anyhow::Error>;
}

/// Trait for display information
pub trait DisplayInfo {
    /// Get the resolution of the display in (width, height)
    fn resolution(&self) -> (u32, u32);
    /// Get the DPI factor for input handling
    fn dpi_conversion_factor(&self) -> f64;
}

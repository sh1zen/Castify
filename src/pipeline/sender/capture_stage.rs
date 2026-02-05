//! Capture stage for the sender pipeline
//!
//! Wraps ScreenCaptureImpl and produces raw frames for encoding.

use crate::assets::FRAME_RATE;
use crate::capture::capturer::CaptureOpts;
use crate::capture::display::DisplaySelector;
use crate::capture::{ScreenCapture, ScreenCaptureImpl};
use crate::encoder::FfmpegEncoder;
use crate::pipeline::PipelineStage;
use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use log::{error, info};
use std::sync::Arc;
use tokio::sync::{mpsc, watch};

/// Capture stage: wraps screen capture and encoder into a pipeline stage
///
/// This stage owns the screen capture implementation and the encoder,
/// producing encoded H.264 frames from captured screen content.
pub struct CaptureStage {
    capture: Arc<tokio::sync::Mutex<ScreenCaptureImpl>>,
    opts_tx: watch::Sender<CaptureOpts>,
    opts_rx: watch::Receiver<CaptureOpts>,
    output_tx: Option<mpsc::Sender<Bytes>>,
    is_running: bool,
}

impl CaptureStage {
    /// Create a new capture stage
    pub fn new() -> Result<Self> {
        let display_capture = ScreenCaptureImpl::new_default()
            .map_err(|e| anyhow::anyhow!("Failed to create screen capture: {}", e))?;

        let default_opts = CaptureOpts {
            blank_screen: false,
            crop: None,
            paused: false,
            max_fps: FRAME_RATE,
        };
        let (opts_tx, opts_rx) = watch::channel(default_opts);

        Ok(Self {
            capture: Arc::new(tokio::sync::Mutex::new(display_capture)),
            opts_tx,
            opts_rx,
            output_tx: None,
            is_running: false,
        })
    }

    /// Get the output channel for encoded frames
    pub fn take_output(&mut self) -> Option<mpsc::Receiver<Bytes>> {
        let (tx, rx) = mpsc::channel::<Bytes>(16);
        self.output_tx = Some(tx);
        Some(rx)
    }

    /// Start capture with the given encoder dimensions
    pub async fn start_capture(&mut self, enc_w: u32, enc_h: u32) -> Result<()> {
        let encoder = FfmpegEncoder::new(enc_w, enc_h);
        let output_tx = self
            .output_tx
            .take()
            .ok_or_else(|| anyhow::anyhow!("No output channel configured"))?;

        let mut cap = self.capture.lock().await;
        cap.start_capture(encoder, output_tx, self.opts_rx.clone())
            .await?;
        self.is_running = true;
        info!("CaptureStage: started capture ({}x{})", enc_w, enc_h);
        Ok(())
    }

    /// Stop the capture
    pub async fn stop_capture(&mut self) -> Result<()> {
        if self.is_running {
            let mut cap = self.capture.lock().await;
            cap.stop_capture().await?;
            self.is_running = false;
            info!("CaptureStage: stopped capture");
        }
        Ok(())
    }

    /// Set blank screen option
    pub fn set_blank_screen(&self, blank: bool) {
        self.opts_tx.send_modify(|o| o.blank_screen = blank);
    }

    /// Set crop rectangle
    pub fn set_crop(&self, crop: Option<crate::capture::CropRect>) {
        self.opts_tx.send_modify(|o| o.crop = crop);
    }

    /// Get current capture resolution
    pub async fn resolution(&self) -> (u32, u32) {
        let opts = self.opts_rx.borrow();
        if let Some(crop) = &opts.crop {
            let w = crop.w + (crop.w % 2);
            let h = crop.h + (crop.h % 2);
            (w, h)
        } else {
            let cap = self.capture.lock().await;
            cap.display().resolution()
        }
    }

    /// Get available displays
    pub fn available_displays(&self) -> Vec<<ScreenCaptureImpl as DisplaySelector>::Display> {
        match self.capture.try_lock() {
            Ok(mut cap) => cap.available_displays().unwrap_or_default(),
            Err(_) => {
                error!("Cannot list displays while capture is locked");
                Vec::new()
            }
        }
    }

    /// Select a display
    pub fn select_display(&self, display: <ScreenCaptureImpl as DisplaySelector>::Display) {
        match self.capture.try_lock() {
            Ok(mut cap) => {
                if let Err(e) = cap.select_display(&display) {
                    error!("Failed to select display: {}", e);
                }
            }
            Err(_) => {
                error!("Cannot change display while capture is running");
            }
        }
    }

    /// Get selected display
    pub fn selected_display(&self) -> Option<<ScreenCaptureImpl as DisplaySelector>::Display> {
        match self.capture.try_lock() {
            Ok(cap) => cap.selected_display().unwrap_or(None),
            Err(_) => None,
        }
    }
}

#[async_trait]
impl PipelineStage for CaptureStage {
    async fn run(&mut self) -> Result<()> {
        // CaptureStage is driven externally via start_capture/stop_capture
        // The internal capture loop runs in a spawned task
        Ok(())
    }

    fn name(&self) -> &'static str {
        "CaptureStage"
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.stop_capture().await
    }
}

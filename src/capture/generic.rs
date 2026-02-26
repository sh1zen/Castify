use anyhow::{Result, anyhow};
use async_trait::async_trait;
use bytes::Bytes;
use display_info::DisplayInfo as OsDisplayInfo;
use std::time::{Duration, Instant};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

use crate::assets::FRAME_RATE;
use crate::capture::display::DisplaySelector;
use crate::capture::{
    CaptureOpts, CropRect, DisplayInfo, ScreenCapture, ScreenCaptureImpl, YUVFrame,
};
use crate::encoder::{FfmpegEncoder, FrameData};

#[derive(Clone, Debug)]
pub struct GenericDisplay {
    pub id: u32,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
}

impl PartialEq for GenericDisplay {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.name == other.name
            && self.width == other.width
            && self.height == other.height
            && self.scale_factor.to_bits() == other.scale_factor.to_bits()
    }
}

impl Eq for GenericDisplay {}

unsafe impl Send for GenericDisplay {}

impl ToString for GenericDisplay {
    fn to_string(&self) -> String {
        self.name.clone()
    }
}

impl DisplayInfo for GenericDisplay {
    fn resolution(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn dpi_conversion_factor(&self) -> f64 {
        self.scale_factor
    }
}

pub struct GenericScreenCapture {
    selected_display: GenericDisplay,
    cancel_token: Option<CancellationToken>,
}

impl GenericScreenCapture {
    fn load_displays() -> Result<Vec<GenericDisplay>> {
        let displays =
            OsDisplayInfo::all().map_err(|e| anyhow!("Failed to enumerate displays: {}", e))?;

        let mut out = Vec::with_capacity(displays.len());
        for d in displays {
            out.push(GenericDisplay {
                id: d.id,
                name: format!("{} ({} x {})", d.name, d.width, d.height),
                width: d.width,
                height: d.height,
                scale_factor: d.scale_factor as f64,
            });
        }
        if out.is_empty() {
            return Err(anyhow!("No displays found"));
        }
        Ok(out)
    }

    fn black_frame(width: u32, height: u32) -> YUVFrame {
        let w = width + (width % 2);
        let h = height + (height % 2);
        YUVFrame {
            display_time: 0,
            width: w as i32,
            height: h as i32,
            luminance_bytes: vec![0u8; (w * h) as usize],
            luminance_stride: w as i32,
            chrominance_bytes: vec![128u8; (w * h / 2) as usize],
            chrominance_stride: w as i32,
        }
    }
}

#[async_trait]
impl ScreenCapture for GenericScreenCapture {
    fn new_default() -> Result<ScreenCaptureImpl, anyhow::Error> {
        let displays = Self::load_displays()?;
        let selected_display = displays[0].clone();
        Ok(Self {
            selected_display,
            cancel_token: None,
        })
    }

    fn display(&self) -> &dyn DisplayInfo {
        &self.selected_display
    }

    async fn start_capture(
        &mut self,
        mut encoder: FfmpegEncoder,
        output: tokio::sync::mpsc::Sender<Bytes>,
        opts_rx: watch::Receiver<CaptureOpts>,
    ) -> Result<(), anyhow::Error> {
        if self.cancel_token.is_some() {
            return Err(anyhow!("Capture already running"));
        }

        let cancel = CancellationToken::new();
        self.cancel_token = Some(cancel.clone());

        let (dw, dh) = self.selected_display.resolution();
        tokio::spawn(async move {
            let mut opts_rx = opts_rx;
            let mut current_crop: Option<CropRect> = opts_rx.borrow().crop;
            let mut black_frame = if let Some(c) = current_crop {
                GenericScreenCapture::black_frame(c.w, c.h)
            } else {
                GenericScreenCapture::black_frame(dw, dh)
            };
            let mut current_fps = opts_rx.borrow().max_fps.clamp(15, FRAME_RATE.max(15));
            let mut pressure_score: u32 = 0;
            let started = Instant::now();

            loop {
                if cancel.is_cancelled() {
                    break;
                }

                let frame_start = Instant::now();
                let opts = opts_rx.borrow().clone();
                if opts.paused {
                    tokio::time::sleep(Duration::from_millis(8)).await;
                    continue;
                }

                let max_fps = opts.max_fps.clamp(15, FRAME_RATE.max(15));
                if current_fps > max_fps {
                    current_fps = max_fps;
                }

                if opts.crop != current_crop {
                    current_crop = opts.crop;
                    let (w, h) = if let Some(c) = current_crop {
                        (c.w, c.h)
                    } else {
                        (dw, dh)
                    };
                    black_frame = GenericScreenCapture::black_frame(w, h);
                    encoder =
                        FfmpegEncoder::new(black_frame.width as u32, black_frame.height as u32);
                }

                let frame_data = if opts.blank_screen {
                    FrameData::NV12(&black_frame)
                } else {
                    // Generic fallback backend: keep deterministic timing and format.
                    FrameData::NV12(&black_frame)
                };
                match encoder.encode(frame_data, started.elapsed().as_micros() as i64) {
                    Ok(encoded) => {
                        if output.try_send(encoded).is_err() {
                            pressure_score = (pressure_score + 3).min(100);
                            current_fps = current_fps.saturating_sub(5).max(15);
                        } else {
                            pressure_score = pressure_score.saturating_sub(1);
                        }
                    }
                    Err(e) => {
                        log::warn!("Generic capture encode failed: {}", e);
                    }
                }

                let elapsed_ms = frame_start.elapsed().as_millis() as u64;
                let budget_ms = (1000 / current_fps.max(1)) as u64;
                if elapsed_ms > budget_ms || pressure_score > 20 {
                    current_fps = current_fps.saturating_sub(3).max(15);
                } else if elapsed_ms < (budget_ms * 55 / 100)
                    && pressure_score < 5
                    && current_fps < max_fps
                {
                    current_fps = (current_fps + 1).min(max_fps);
                }

                let remaining = budget_ms.saturating_sub(elapsed_ms);
                if remaining > 1 {
                    tokio::select! {
                        _ = cancel.cancelled() => break,
                        _ = tokio::time::sleep(Duration::from_millis(remaining)) => {}
                    }
                }
            }
        });

        Ok(())
    }

    async fn stop_capture(&mut self) -> Result<(), anyhow::Error> {
        if let Some(cancel) = self.cancel_token.take() {
            cancel.cancel();
        }
        Ok(())
    }
}

impl DisplaySelector for GenericScreenCapture {
    type Display = GenericDisplay;

    fn available_displays(&mut self) -> Result<Vec<Self::Display>> {
        Self::load_displays()
    }

    fn select_display(&mut self, display: &Self::Display) -> Result<()> {
        self.selected_display = display.clone();
        Ok(())
    }

    fn selected_display(&self) -> Result<Option<Self::Display>> {
        Ok(Some(self.selected_display.clone()))
    }
}

use crate::capture::display::DisplaySelector;
use crate::capture::wgc::d3d;
use crate::capture::wgc::display::Display;
use crate::capture::{CaptureOpts, CropRect, DisplayInfo, ScreenCapture, ScreenCaptureImpl, YUVFrame, YuvConverter};
use crate::encoder::{FfmpegEncoder, FrameData};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::select;
use tokio::sync::watch;
use windows::core::IInspectable;
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Capture::{
    Direct3D11CaptureFrame, Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession,
};
use windows::Graphics::DirectX::DirectXPixelFormat;

pub struct WGCScreenCapture {
    engine: Option<CaptureEngine>,
    selected_display: Display,
    session: Option<GraphicsCaptureSession>,
    item: GraphicsCaptureItem,
}

struct CaptureEngine {
    frame_pool: Direct3D11CaptureFramePool,
    duplicator: YuvConverter,
}

impl CaptureEngine {
    fn new(item: &GraphicsCaptureItem) -> Self {
        let item_size = item.Size().unwrap();
        let (device, d3d_device, d3d_context) = d3d::create_direct3d_devices_and_context().unwrap();
        let device = Arc::new(device);
        let d3d_context = Arc::new(d3d_context);
        let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            &d3d_device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            3,
            item_size,
        )
            .unwrap();
        let duplicator = YuvConverter::new(
            device,
            d3d_context,
            (item_size.Width as u32, item_size.Height as u32),
        )
            .unwrap();
        Self {
            frame_pool,
            duplicator,
        }
    }
}

#[async_trait]
impl ScreenCapture for WGCScreenCapture {
    fn new_default() -> Result<ScreenCaptureImpl, anyhow::Error> {
        let selected_display = Display::online().unwrap()[0].clone();
        let item = selected_display.select()?;
        Ok(Self {
            engine: None,
            selected_display,
            session: None,
            item,
        })
    }

    fn display(&self) -> &dyn DisplayInfo {
        &self.item
    }

    async fn start_capture(
        &mut self,
        mut encoder: FfmpegEncoder,
        output: tokio::sync::mpsc::Sender<bytes::Bytes>,
        opts_rx: watch::Receiver<CaptureOpts>,
    ) -> Result<(), anyhow::Error> {
        let engine = CaptureEngine::new(&self.item);
        let item_size = self.item.Size()?;

        let session = engine.frame_pool.CreateCaptureSession(&self.item)?;

        let (sender, mut receiver) = tokio::sync::mpsc::channel::<Direct3D11CaptureFrame>(1);

        engine.frame_pool.FrameArrived(&TypedEventHandler::<
            Direct3D11CaptureFramePool,
            IInspectable,
        >::new({
            move |frame_pool, _| {
                let frame_pool = frame_pool.as_ref().unwrap();
                let frame = frame_pool.TryGetNextFrame()?;
                let _ = sender.try_send(frame);
                Ok(())
            }
        }))?;

        session.StartCapture()?;
        self.session.replace(session);

        let mut duplicator = engine.duplicator.clone();

        // Read crop once at start (frozen for this session)
        let initial_crop = opts_rx.borrow().crop;

        tokio::spawn(async move {
            loop {
                select! {
                    Some(frame) = receiver.recv() => {
                        let frame_time = frame.SystemRelativeTime().unwrap().Duration;

                        // Check blank_screen dynamically
                        let opts = opts_rx.borrow().clone();

                        if opts.blank_screen {
                            // Encode a black NV12 frame at encoder dimensions
                            let (enc_w, enc_h) = if let Some(ref c) = initial_crop {
                                (c.w + (c.w % 2), c.h + (c.h % 2))
                            } else {
                                (item_size.Width as u32, item_size.Height as u32)
                            };
                            let black = YUVFrame {
                                display_time: 0,
                                width: enc_w as i32,
                                height: enc_h as i32,
                                luminance_bytes: vec![0u8; (enc_w * enc_h) as usize],
                                luminance_stride: enc_w as i32,
                                chrominance_bytes: vec![128u8; (enc_w * enc_h / 2) as usize],
                                chrominance_stride: enc_w as i32,
                            };
                            match encoder.encode(FrameData::NV12(&black), frame_time) {
                                Ok(encoded) => {
                                    if output.send(encoded).await.is_err() { break; }
                                }
                                Err(e) => {
                                    log::error!("Encode blank frame failed: {}", e);
                                }
                            }
                            continue;
                        }

                        let yuv_frame = {
                            duplicator
                                .capture(d3d::get_d3d_interface_from_object(&frame.Surface().unwrap()).unwrap()).unwrap()
                        };

                        // True crop extraction: extract only the crop region
                        let frame_to_encode = if let Some(ref crop) = initial_crop {
                            extract_crop_nv12(&yuv_frame, crop)
                        } else {
                            yuv_frame
                        };

                        match encoder.encode(FrameData::NV12(&frame_to_encode), frame_time) {
                            Ok(encoded) => {
                                if output.send(encoded).await.is_err() { break; }
                            }
                            Err(e) => {
                                log::error!("Encode frame failed: {}", e);
                            }
                        }
                    }
                    else => break,
                }
            }
        });

        self.engine.replace(engine);

        Ok(())
    }

    async fn stop_capture(&mut self) -> Result<(), anyhow::Error> {
        if let Some(session) = self.session.take() {
            session.Close()?;
        }
        self.engine.take();
        Ok(())
    }
}

impl DisplaySelector for WGCScreenCapture {
    type Display = Display;

    fn available_displays(&mut self) -> Result<Vec<Display>, anyhow::Error> {
        Display::online()
    }

    fn select_display(&mut self, display: &Display) -> Result<(), anyhow::Error> {
        self.engine = Some(CaptureEngine::new(&display.select()?));
        self.selected_display = display.clone();
        Ok(())
    }

    fn selected_display(&self) -> Result<Option<Self::Display>, anyhow::Error> {
        Ok(Some(self.selected_display.clone()))
    }
}

/// Extract a crop region from an NV12 YUVFrame, producing a smaller YUVFrame.
fn extract_crop_nv12(frame: &YUVFrame, crop: &CropRect) -> YUVFrame {
    // Ensure even alignment for NV12 chroma
    let cx = crop.x & !1; // round down to even
    let cw = (crop.w + (crop.w % 2)) as usize; // round up to even
    let cy = crop.y & !1;
    let ch = (crop.h + (crop.h % 2)) as usize;

    let src_y_stride = frame.luminance_stride as usize;
    let src_uv_stride = frame.chrominance_stride as usize;

    // Extract Y plane
    let mut y_buf = vec![0u8; cw * ch];
    for row in 0..ch {
        let src_row = (cy as usize) + row;
        if src_row >= frame.height as usize { break; }
        let src_start = src_row * src_y_stride + cx as usize;
        let dst_start = row * cw;
        let copy_len = cw.min(frame.luminance_bytes.len().saturating_sub(src_start));
        if copy_len > 0 {
            y_buf[dst_start..dst_start + copy_len]
                .copy_from_slice(&frame.luminance_bytes[src_start..src_start + copy_len]);
        }
    }

    // Extract UV plane (NV12: interleaved UV at half vertical res, same horizontal byte width)
    let uv_h = ch / 2;
    let mut uv_buf = vec![128u8; cw * uv_h];
    for row in 0..uv_h {
        let src_row = (cy as usize / 2) + row;
        if src_row >= (frame.height as usize / 2) { break; }
        // In NV12, UV pairs start at even x positions; cx is already even
        let src_start = src_row * src_uv_stride + cx as usize;
        let dst_start = row * cw;
        let copy_len = cw.min(frame.chrominance_bytes.len().saturating_sub(src_start));
        if copy_len > 0 {
            uv_buf[dst_start..dst_start + copy_len]
                .copy_from_slice(&frame.chrominance_bytes[src_start..src_start + copy_len]);
        }
    }

    YUVFrame {
        display_time: frame.display_time,
        width: cw as i32,
        height: ch as i32,
        luminance_bytes: y_buf,
        luminance_stride: cw as i32,
        chrominance_bytes: uv_buf,
        chrominance_stride: cw as i32,
    }
}
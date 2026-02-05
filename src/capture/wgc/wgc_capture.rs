use crate::assets::FRAME_RATE;
use crate::capture::display::DisplaySelector;
use crate::capture::wgc::d3d;
use crate::capture::wgc::display::Display;
use crate::capture::{
    CaptureOpts, CropRect, DisplayInfo, ScreenCapture, ScreenCaptureImpl, YUVFrame, YuvConverter,
};
use crate::encoder::{FfmpegEncoder, FrameData};
use crate::utils::perf::PipelineStats;
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::select;
use tokio::sync::watch;
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Capture::{
    Direct3D11CaptureFrame, Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession,
};
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::core::IInspectable;

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

        // Increased capacity to prevent frame drops when pipeline is under load
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<Direct3D11CaptureFrame>(8);

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

        // Track current crop dynamically — read from opts_rx each frame
        let mut current_crop: Option<CropRect> = opts_rx.borrow().crop;

        // Share the force_idr flag from the encoder
        let force_idr = encoder.force_idr.clone();

        // Pipeline stats for periodic logging
        let stats = Arc::new(PipelineStats::new(encoder.codec_name.clone()));
        let stats_clone = Arc::clone(&stats);

        // Periodic stats logger
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                stats_clone.log_summary();
            }
        });

        tokio::spawn(async move {
            log::info!("=== WGC Capture loop STARTED ===");

            // Cache the black frame to avoid per-frame allocation
            let mut cached_black_frame: Option<YUVFrame> = None;

            // Pre-allocated crop buffers — reused across frames to avoid per-frame allocation
            let mut crop_y_buf: Vec<u8> = Vec::new();
            let mut crop_uv_buf: Vec<u8> = Vec::new();

            let mut max_fps = opts_rx.borrow().max_fps.clamp(15, FRAME_RATE.max(15));
            let mut current_fps: u32 = max_fps;
            let mut pressure_score: u32 = 0;

            let mut frame_count = 0u64;
            let mut last_frame_log = std::time::Instant::now();

            loop {
                select! {
                    Some(frame) = receiver.recv() => {
                        frame_count += 1;

                        // Log heartbeat every 5 seconds
                        if last_frame_log.elapsed().as_secs() >= 5 {
                            log::info!("WGC Capture: {} frames processed, FPS: {}", frame_count, current_fps);
                            last_frame_log = std::time::Instant::now();
                        }

                        let frame_start = std::time::Instant::now();
                        let frame_time = frame.SystemRelativeTime().unwrap().Duration;

                        // Read opts dynamically each frame (blank_screen + crop + paused)
                        let opts = opts_rx.borrow().clone();
                        max_fps = opts.max_fps.clamp(15, FRAME_RATE.max(15));
                        if current_fps > max_fps {
                            current_fps = max_fps;
                        }

                        // If paused, skip encoding and continue to next frame
                        if opts.paused {
                            continue;
                        }

                        // Detect crop change and recreate encoder if needed
                        if opts.crop != current_crop {
                            let (enc_w, enc_h) = if let Some(ref c) = opts.crop {
                                (c.w + (c.w % 2), c.h + (c.h % 2))
                            } else {
                                (item_size.Width as u32, item_size.Height as u32)
                            };
                            encoder = FfmpegEncoder::new(enc_w, enc_h);
                            encoder.force_idr = force_idr.clone();
                            force_idr.store(true, Ordering::Relaxed);
                            cached_black_frame = None;
                            current_crop = opts.crop;
                            log::info!("Crop changed → encoder recreated at {}x{}", enc_w, enc_h);
                        }

                        if opts.blank_screen {
                            // Encode a black NV12 frame at encoder dimensions
                            let (enc_w, enc_h) = if let Some(ref c) = current_crop {
                                (c.w + (c.w % 2), c.h + (c.h % 2))
                            } else {
                                (item_size.Width as u32, item_size.Height as u32)
                            };
                            let black = cached_black_frame.get_or_insert_with(|| {
                                YUVFrame {
                                    display_time: 0,
                                    width: enc_w as i32,
                                    height: enc_h as i32,
                                    luminance_bytes: vec![0u8; (enc_w * enc_h) as usize],
                                    luminance_stride: enc_w as i32,
                                    chrominance_bytes: vec![128u8; (enc_w * enc_h / 2) as usize],
                                    chrominance_stride: enc_w as i32,
                                }
                            });
                            match encoder.encode(FrameData::NV12(black), frame_time) {
                                Ok(encoded) => {
                                    if output.try_send(encoded).is_err() {
                                        stats.frames_skipped.fetch_add(1, Ordering::Relaxed);
                                    }
                                }
                                Err(e) => {
                                    log::error!("Encode blank frame failed: {}", e);
                                }
                            }
                            continue;
                        }

                        let surface =
                            d3d::get_d3d_interface_from_object(&frame.Surface().unwrap()).unwrap();

                        let encoded_result = if current_crop.is_none() {
                            // Fast path: map NV12 planes and encode directly, avoiding YUVFrame allocation/copy.
                            let t_capture = std::time::Instant::now();
                            duplicator.capture_with_nv12_view(surface, |nv12_view| {
                                stats.capture_us.fetch_add(
                                    t_capture.elapsed().as_micros() as u64,
                                    Ordering::Relaxed,
                                );
                                let t_encode = std::time::Instant::now();
                                let encoded = encoder.encode(FrameData::NV12Ref(nv12_view), frame_time);
                                stats
                                    .encode_us
                                    .fetch_add(t_encode.elapsed().as_micros() as u64, Ordering::Relaxed);
                                encoded
                            })
                        } else {
                            let t_capture = std::time::Instant::now();
                            let yuv_frame = duplicator.capture(surface).unwrap();
                            stats
                                .capture_us
                                .fetch_add(t_capture.elapsed().as_micros() as u64, Ordering::Relaxed);

                            // Crop extraction: reuse pre-allocated buffers, swap instead of clone
                            let frame_to_encode = extract_crop_nv12_reuse(
                                &yuv_frame,
                                current_crop.as_ref().unwrap(),
                                &mut crop_y_buf,
                                &mut crop_uv_buf,
                            );

                            let t_encode = std::time::Instant::now();
                            let encoded = encoder.encode(FrameData::NV12(&frame_to_encode), frame_time);
                            stats
                                .encode_us
                                .fetch_add(t_encode.elapsed().as_micros() as u64, Ordering::Relaxed);
                            encoded
                        };

                        match encoded_result {
                            Ok(encoded) => {
                                stats.frames_encoded.fetch_add(1, Ordering::Relaxed);

                                let t_send = std::time::Instant::now();
                                // Use try_send to prevent blocking the capture pipeline
                                match output.try_send(encoded) {
                                    Ok(_) => {
                                        pressure_score = pressure_score.saturating_sub(1);
                                        stats.send_us.fetch_add(t_send.elapsed().as_micros() as u64, Ordering::Relaxed);
                                    }
                                    Err(_) => {
                                        // Output channel full - drop this frame and request keyframe
                                        pressure_score = (pressure_score + 3).min(100);
                                        current_fps = current_fps.saturating_sub(6).max(15);
                                        stats.frames_skipped.fetch_add(1, Ordering::Relaxed);
                                        force_idr.store(true, Ordering::Relaxed);
                                        log::warn!("Encoder output channel full, frame dropped");
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("Encode frame failed: {}", e);
                            }
                        }

                        // Adaptive frame rate
                        let elapsed_ms = frame_start.elapsed().as_millis() as u64;
                        let budget_ms = 1000 / current_fps as u64;

                        if elapsed_ms > budget_ms || pressure_score > 20 {
                            current_fps = current_fps.saturating_sub(4).max(15);
                            stats.frames_skipped.fetch_add(1, Ordering::Relaxed);
                        } else if elapsed_ms < budget_ms * 55 / 100
                            && pressure_score < 5
                            && current_fps < max_fps
                        {
                            current_fps = (current_fps + 1).min(max_fps);
                        }

                        stats.current_fps.store(current_fps as u64, Ordering::Relaxed);

                        let remaining = budget_ms.saturating_sub(elapsed_ms);
                        if remaining > 1 {
                            tokio::time::sleep(std::time::Duration::from_millis(remaining)).await;
                        }
                    }
                    else => {
                        log::error!("WGC Capture: receiver.recv() returned None - channel closed!");
                        break;
                    }
                }
            }

            log::error!(
                "=== WGC Capture loop EXITED after {} frames ===",
                frame_count
            );
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
        // Switching display while an active session is running is not supported.
        // The caller should stop capture first, select display, then start again.
        if self.session.is_some() {
            return Err(anyhow::anyhow!(
                "Cannot switch display while capture is running"
            ));
        }

        // IMPORTANT: update the actual capture item, not only internal engine state.
        self.item = display.select()?;
        self.engine = None;
        self.selected_display = display.clone();
        Ok(())
    }

    fn selected_display(&self) -> Result<Option<Self::Display>, anyhow::Error> {
        Ok(Some(self.selected_display.clone()))
    }
}

/// Extract a crop region from an NV12 YUVFrame, reusing pre-allocated buffers.
/// Returns a YUVFrame whose luminance/chrominance data lives in the provided buffers.
fn extract_crop_nv12_reuse<'a>(
    frame: &YUVFrame,
    crop: &CropRect,
    y_buf: &'a mut Vec<u8>,
    uv_buf: &'a mut Vec<u8>,
) -> YUVFrame {
    let cx = crop.x & !1;
    let cw = (crop.w + (crop.w % 2)) as usize;
    let cy = crop.y & !1;
    let ch = (crop.h + (crop.h % 2)) as usize;

    let src_y_stride = frame.luminance_stride as usize;
    let src_uv_stride = frame.chrominance_stride as usize;

    // Resize buffers (no-op after first frame if dimensions are constant)
    y_buf.resize(cw * ch, 0);
    let uv_h = ch / 2;
    uv_buf.resize(cw * uv_h, 128);

    // Extract Y plane
    for row in 0..ch {
        let src_row = (cy as usize) + row;
        if src_row >= frame.height as usize {
            break;
        }
        let src_start = src_row * src_y_stride + cx as usize;
        let dst_start = row * cw;
        let copy_len = cw.min(frame.luminance_bytes.len().saturating_sub(src_start));
        if copy_len > 0 {
            y_buf[dst_start..dst_start + copy_len]
                .copy_from_slice(&frame.luminance_bytes[src_start..src_start + copy_len]);
        }
    }

    // Extract UV plane
    for row in 0..uv_h {
        let src_row = (cy as usize / 2) + row;
        if src_row >= (frame.height as usize / 2) {
            break;
        }
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
        luminance_bytes: std::mem::replace(y_buf, Vec::with_capacity(cw * ch)),
        luminance_stride: cw as i32,
        chrominance_bytes: std::mem::replace(uv_buf, Vec::with_capacity(cw * uv_h)),
        chrominance_stride: cw as i32,
    }
}

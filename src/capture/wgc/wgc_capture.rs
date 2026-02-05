use crate::capture::display::DisplaySelector;
use crate::capture::wgc::d3d;
use crate::capture::wgc::display::Display;
use crate::capture::{DisplayInfo, ScreenCapture, ScreenCaptureImpl, YuvConverter};
use crate::encoder::{FfmpegEncoder, FrameData};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::select;
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
    ) -> Result<(), anyhow::Error> {
        let engine = CaptureEngine::new(&self.item);

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

        tokio::spawn(async move {
            loop {
                select! {
                    Some(frame) = receiver.recv() => {
                        let frame_time = frame.SystemRelativeTime().unwrap().Duration;
                        let yuv_frame = {
                            duplicator
                                .capture(d3d::get_d3d_interface_from_object(&frame.Surface().unwrap()).unwrap()).unwrap()
                        };

                        let encoded = encoder
                            .encode(FrameData::NV12(&yuv_frame), frame_time)
                            .unwrap();

                        if output.send(encoded).await.is_err() {
                            break;
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
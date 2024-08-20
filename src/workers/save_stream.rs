use crate::gui::resource::FRAME_RATE;
use crate::utils::gist::create_save_pipeline;
use glib::prelude::*;
use gstreamer::prelude::{ElementExt, GstBinExt};
use gstreamer::{ClockTime, MessageView, Pipeline};
use gstreamer_app::{gst, AppSrc};
use once_cell::sync::Lazy;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;
use xcap::image::RgbaImage;

#[derive(Debug, Clone)]
pub struct SaveStream {
    pub(crate) is_saving: bool,
    is_available: bool,
    pipeline: Pipeline,
    appsrc: Option<AppSrc>,
    frame_i: u64,
}

impl SaveStream {
    pub fn new() -> Self {
        Self {
            is_saving: false,
            is_available: false,
            pipeline: Pipeline::new(),
            appsrc: None,
            frame_i: 0,
        }
    }

    pub fn start(&mut self) {
        if !self.is_saving {
            self.is_saving = true;
            self.frame_i = 0;
            tokio::spawn(async move {
                let pipeline = create_save_pipeline().unwrap();
                let appsrc = Some(
                    pipeline
                        .by_name("image-to-file")
                        .and_then(|elem| elem.downcast::<AppSrc>().ok())
                        .unwrap()
                );
                pipeline.set_state(gst::State::Playing).expect("Failed start save_pipeline");
                let _ = pipeline.state(ClockTime::from_seconds(3));

                let binding = get_instance();
                let mut self_lock = binding.lock().unwrap();
                self_lock.pipeline = pipeline;
                self_lock.appsrc = appsrc;
                self_lock.is_available = true;
            });
        }
    }

    pub fn send_frame(&mut self, frame: RgbaImage) {
        if self.is_available {
            match &self.appsrc {
                Some(appsrc) => {
                    // Convert the image buffer into raw byte data
                    let raw_data: Vec<u8> = frame.into_raw();

                    // Create a GStreamer buffer from the raw data slice
                    let mut buffer = gst::Buffer::from_slice(raw_data);
                    {
                        let buffer_ref = buffer.get_mut().unwrap();

                        // Calculate PTS and duration based on frame rate
                        let pts = ClockTime::from_mseconds(1000 * self.frame_i / FRAME_RATE as u64);
                        let duration = ClockTime::from_mseconds(1000 * (1 / FRAME_RATE) as u64);

                        buffer_ref.set_pts(pts);
                        buffer_ref.set_dts(pts);
                        buffer_ref.set_duration(duration);
                    }

                    match appsrc.push_buffer(buffer) {
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("{}", e);
                            self.stop()
                        }
                    }

                    self.frame_i += 1;
                }
                _ => {}
            }
        }
    }

    pub fn stop(&mut self) {
        self.is_available = false;

        match &self.appsrc {
            Some(appsrc) => {
                appsrc.end_of_stream().expect("Failed to send EOS");
                // Check for pipeline state changes, errors, etc.
                let bus = self.pipeline.bus().unwrap();
                for msg in bus.iter() {
                    match msg.view() {
                        MessageView::Eos(_) => {
                            self.pipeline.set_state(gstreamer::State::Null).unwrap();
                            break;
                        }
                        _ => {
                            // fix to allow pipeline to prepare for gstreamer::State::Null
                            sleep(Duration::from_millis(2));
                        }
                    }
                }
            }
            _ => {}
        }

        self.appsrc = None;
        self.pipeline = Pipeline::new();
        self.is_saving = false;
    }
}

static INSTANCE: Lazy<Arc<Mutex<SaveStream>>> = Lazy::new(|| Arc::new(Mutex::new(SaveStream::new())));

pub(crate) fn get_instance() -> Arc<Mutex<SaveStream>> {
    INSTANCE.clone()
}
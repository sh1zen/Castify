use crate::gui::resource::USE_WEBRTC;
use crate::utils::gist::{create_rtp_save_pipeline, create_save_pipeline};
use glib::prelude::*;
use gstreamer::prelude::{ElementExt, GstBinExt};
use gstreamer::{MessageView, Pipeline};
use gstreamer_app::{gst, AppSrc};
use once_cell::sync::Lazy;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

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
            pipeline: Default::default(),
            appsrc: None,
            frame_i: 0,
        }
    }

    pub fn start(&mut self) {
        if !self.is_saving {
            self.is_saving = true;
            self.frame_i = 0;
            tokio::spawn(async move {
                let pipeline = if USE_WEBRTC {
                    create_rtp_save_pipeline().unwrap()
                } else {
                    create_save_pipeline().unwrap()
                };
                let appsrc = Some(
                    pipeline.by_name("appsrc")
                        .and_then(|elem| elem.downcast::<AppSrc>().ok())
                        .unwrap()
                );
                pipeline.set_state(gst::State::Playing).expect("Failed start save_pipeline");
                get_instance().lock().unwrap().pipeline = pipeline;
                get_instance().lock().unwrap().appsrc = appsrc;
                get_instance().lock().unwrap().is_available = true;
            });
        }
    }

    pub fn send_frame(&mut self, buffer: gst::Buffer) {
        if self.is_available {
            match &self.appsrc {
                Some(appsrc) => {
                    if let Err(_) = appsrc.push_buffer(buffer) {
                        self.stop()
                    }
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
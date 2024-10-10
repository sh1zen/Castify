use crate::utils::gist::create_save_pipeline;
use glib::prelude::*;
use gstreamer::prelude::{ElementExt, GstBinExt};
use gstreamer::{FlowSuccess, MessageView, Pipeline};
use gstreamer_app::{gst, AppSrc};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Receiver;
use tokio::time::sleep;

#[derive(Debug)]
pub struct SaveStream {
    saver_channel: Arc<tokio::sync::Mutex<Receiver<gstreamer::Buffer>>>,
    is_saving: Arc<tokio::sync::Mutex<bool>>,
    pipeline: Arc<tokio::sync::Mutex<Pipeline>>,
}

impl SaveStream {
    pub fn new(saver_channel: Arc<tokio::sync::Mutex<Receiver<gstreamer::Buffer>>>) -> Self {
        Self {
            saver_channel,
            is_saving: Arc::new(tokio::sync::Mutex::new(false)),
            pipeline: Default::default(),
        }
    }

    pub fn start(&mut self) {
        let pipeline = create_save_pipeline().unwrap();

        if let Some(appsrc) = pipeline.by_name("appsrc").and_then(|elem| elem.downcast::<AppSrc>().ok()) {
            pipeline.set_state(gst::State::Playing).expect("Failed starting save_pipeline");
            *self.pipeline.blocking_lock() = pipeline;
            *self.is_saving.blocking_lock() = true;

            let available = Arc::clone(&self.is_saving);
            let saver_channel = Arc::clone(&self.saver_channel);
            let pipeline = Arc::clone(&self.pipeline);

            tokio::spawn(async move {
                loop {
                    if !*available.lock().await {
                        break;
                    }

                    let buffer = saver_channel.lock().await.recv().await;
                    if let Some(buffer) = buffer {
                        if appsrc.push_buffer(buffer).is_err() {
                            *available.lock().await = false;
                            appsrc.end_of_stream().unwrap_or(FlowSuccess::Ok);
                            pipeline.lock().await.set_state(gstreamer::State::Null).unwrap();
                            break;
                        }
                    }
                }
            });
        }
    }

    pub fn stop(&mut self) {
        let is_saving = Arc::clone(&self.is_saving);
        let pipeline = Arc::clone(&self.pipeline);

        tokio::spawn(async move {
            let mut pipeline = pipeline.lock().await;
            match &pipeline
                .by_name("appsrc").and_then(|elem| elem.downcast::<AppSrc>().ok()) {
                Some(appsrc) => {
                    appsrc.end_of_stream().unwrap_or(FlowSuccess::Ok);
                    if let Some(bus) = pipeline.bus()
                    {
                        for msg in bus.iter() {
                            match msg.view() {
                                MessageView::Eos(_) => {
                                    pipeline.set_state(gstreamer::State::Null).unwrap();
                                    break;
                                }
                                _ => {
                                    // fix to allow pipeline to prepare for gstreamer::State::Null
                                    sleep(Duration::from_millis(2)).await;
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
            *is_saving.lock().await = false;
            *pipeline = Pipeline::new();
        });
    }

    pub fn is_saving(&self) -> bool {
        *self.is_saving.blocking_lock()
    }
}
use crate::utils::gist::create_save_pipeline;
use gstreamer::prelude::{Cast, ElementExt, GstBinExt};
use gstreamer::{ClockTime, FlowSuccess, MessageView, Pipeline, StateChangeSuccess};
use gstreamer_app::{gst, AppSrc};
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;

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

    pub fn start(&mut self, path: String) {
        let pipeline = create_save_pipeline(path).unwrap();

        if let Some(appsrc) = pipeline.by_name("appsrc").and_then(|elem| elem.downcast::<AppSrc>().ok()) {
            if pipeline.set_state(gst::State::Playing).is_err() {
                return;
            }
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
                        for msg in bus.iter_timed(Some(ClockTime::from_mseconds(10))) {
                            match msg.view() {
                                MessageView::Eos(_) => {
                                    pipeline.set_state(gstreamer::State::Null).unwrap_or(StateChangeSuccess::NoPreroll);
                                    break;
                                }
                                _ => {}
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
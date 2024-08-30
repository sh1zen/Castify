use crate::capture::{Capture, Streamer};
use crate::gui::resource::USE_WEBRTC;
use crate::utils::gist::create_stream_pipeline;
use crate::utils::net::WebRTCServer;
use gstreamer::prelude::ElementExt;
use gstreamer::Pipeline;
use gstreamer_app::gst;
use once_cell::sync::Lazy;
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct Caster {
    pub streaming: bool,
    blank_screen: bool,
    init: bool,
    monitor: u32,
    capture: Arc<tokio::sync::Mutex<Capture>>,
    pipeline: Arc<tokio::sync::Mutex<Pipeline>>,
}

impl Caster {
    pub fn new() -> Self {
        let cap = Capture::new();
        Self {
            streaming: false,
            blank_screen: false,
            init: false,
            monitor: Capture::get_main(),
            capture: Arc::new(tokio::sync::Mutex::new(cap)),
            pipeline: Default::default(),
        }
    }

    pub fn resize_rec_area(&mut self, x: i32, y: i32, width: u32, height: u32) {
        let monitor = self.monitor;
        let cap = Arc::clone(&self.capture);
        tokio::spawn(async move {
            cap.lock().await.resize(monitor, x, y, width, height);
        });
    }

    pub fn full_screen(&mut self) {
        self.resize_rec_area(0, 0, 0, 0);
    }

    pub fn change_monitor(&mut self, id: u32) {
        self.monitor = id;
    }
    pub fn current_monitor(&self) -> u32 {
        self.monitor
    }

    pub fn cast_screen(&mut self) {
        self.streaming = true;

        if !self.init {
            self.init = true;

            let (tx_raw, rx_raw) = tokio::sync::mpsc::channel(1);
            let (tx_processed, rx_processed) = tokio::sync::mpsc::channel(1);

            // generate frames
            let capture = Arc::clone(&self.capture);
            tokio::spawn(async move {
                Streamer::stream(capture, tx_raw).await;
            });

            // process screens
            self.pipeline = Arc::new(
                tokio::sync::Mutex::new(
                    create_stream_pipeline(rx_raw, tx_processed, false).unwrap()
                )
            );

            let pipeline = Arc::clone(&self.pipeline);

            tokio::spawn(async move {
                pipeline.lock().await.set_state(gst::State::Playing).unwrap();

                // used for auto caster discovery
                crate::utils::net::caster_discover_service();

                if USE_WEBRTC {
                    let calla = WebRTCServer::new();
                    calla.send_video_frames(rx_processed).await.expect("send_video_frames webrtc error");
                } else {
                    crate::utils::net::net::caster(rx_processed).await;
                }
            });
        }
    }

    pub fn pause(&mut self) {
        self.streaming = false;
    }

    pub fn toggle_blank_screen(&mut self) {
        self.blank_screen = !self.blank_screen;
    }

    pub fn is_blank_screen(&self) -> bool {
        self.blank_screen.clone()
    }
}

static INSTANCE: Lazy<Arc<Mutex<Caster>>> = Lazy::new(|| Arc::new(Mutex::new(Caster::new())));

pub(crate) fn get_instance() -> Arc<Mutex<Caster>> {
    INSTANCE.clone()
}
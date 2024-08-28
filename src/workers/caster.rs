use crate::capture::Capture;
use crate::gui::resource::{FRAME_RATE, USE_WEBRTC};
use crate::utils::gist::create_stream_pipeline;
use crate::utils::net::WebRTCServer;
use gstreamer::prelude::ElementExt;
use gstreamer::{ClockTime, Pipeline};
use gstreamer_app::gst;
use once_cell::sync::Lazy;
use std::sync::{Arc, Mutex};
use crate::gui::types::appbase::CaptureMode;

#[derive(Debug, Clone)]
pub struct Caster {
    pub streaming: bool,
    blank_screen: bool,
    init: bool,
    pub monitor: u32,
    capture: Capture,
    pipeline: Pipeline,
    pub capture_mode: CaptureMode,
}

impl Caster {
    pub fn new() -> Self {
        let mut cap = Capture::new();
        cap.set_framerate(FRAME_RATE as f32);
        Self {
            streaming: false,
            blank_screen: false,
            init: false,
            monitor: Capture::get_main(),
            capture: cap,
            pipeline: Default::default(),
            capture_mode: CaptureMode::FullScreen,
        }
    }

    pub fn cast_screen(&mut self, capture_mode: CaptureMode) {
        self.streaming = true;

        if !self.init {
            self.init = true;

            if USE_WEBRTC {
                let mut ff = self.clone();

                tokio::spawn(async move {
                    ff.get_stream(capture_mode).await;
                });
            } else {
                let (tx, rx) = tokio::sync::mpsc::channel(3);

                let mut selfc = self.clone();

                // generate frames
                tokio::spawn(async move {
                    match capture_mode {
                        CaptureMode::FullScreen => {
                            println!("Capture Mode: Full Screen");
                            selfc.capture.stream(0, tx).await;
                        },
                        CaptureMode::Area => {
                            let area = (1000, 500, 500, 500); //valori di esempio per il momento
                            println!(
                                "Capture Mode: Area Selected\nCoordinates: x = {}, y = {}, width = {}, height = {}",
                                area.0, area.1, area.2, area.3
                            );
                            selfc.capture.stream_area(0, area, tx).await;
                        },
                    }
                });

                // send frames over the local network
                tokio::spawn(async move {
                    crate::utils::net::net::caster(rx).await;
                });
            }
        }
    }

    async fn get_stream(&mut self, capture_mode: CaptureMode) {
        let (tx_raw, mut rx_raw) = tokio::sync::mpsc::channel(1);
        let (tx_processed, mut rx_processed) = tokio::sync::mpsc::channel(1);
        let mut selfc = self.clone();

        // capture screens
        tokio::spawn(async move {
            match capture_mode {
                CaptureMode::FullScreen => {
                    selfc.capture.stream(0, tx_raw).await;
                },
                CaptureMode::Area => {
                    let area = (100, 100, 500, 500); // valori d'esempio
                    selfc.capture.stream_area(0, area, tx_raw).await;
                },
            }
        });

        // process screens
        self.pipeline = create_stream_pipeline(rx_raw, tx_processed).unwrap();
        self.pipeline.set_state(gst::State::Playing).unwrap();
        let _ = self.pipeline.state(ClockTime::from_seconds(1));

        let calla = WebRTCServer::new();

        tokio::spawn(async move {
            calla.send_video_frames(rx_processed).await.expect("send_video_frames webrtc error");
        });

        // test save
        /*tokio::spawn(async move {
            let pipeline = create_ss_save_pipeline(rx_processed).unwrap();

            pipeline.set_state(gst::State::Playing).unwrap();
            let _ = pipeline.state(ClockTime::from_seconds(1));
        });*/
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
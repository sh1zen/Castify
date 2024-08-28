use crate::capture::{Capture, Streamer};
use crate::gui::resource::{FRAME_RATE, USE_WEBRTC};
use crate::utils::gist::create_stream_pipeline;
use gstreamer::prelude::ElementExt;
use gstreamer::{ClockTime, Pipeline};
use gstreamer_app::gst;
use once_cell::sync::Lazy;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

#[derive(Debug, Clone)]
pub struct Caster {
    pub streaming: bool,
    blank_screen: bool,
    init: bool,
    monitor: u32,
    capture: Arc<tokio::sync::Mutex<Capture>>,
    pipeline: Pipeline,
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
        let mut cap = Arc::clone(&self.capture);
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

            if USE_WEBRTC {
                let mut ff = self.clone();

                tokio::spawn(async move {
                    ff.get_stream().await;
                });
            } else {
                let (tx, rx) = tokio::sync::mpsc::channel(1);

                // generate frames
                let capture = Arc::clone(&self.capture);
                tokio::spawn(async move {
                    Streamer::stream(capture, tx).await;
                });

                // send frames over the local network
                tokio::spawn(async move {
                    crate::utils::net::net::caster(rx).await;
                });
            }
        }
    }

    async fn get_stream(&mut self) {
        let (tx_raw, mut rx_raw) = tokio::sync::mpsc::channel(1);
        let (tx_processed, mut rx_processed) = tokio::sync::mpsc::channel(1);

        // capture screens
        let capture = Arc::clone(&self.capture);
        tokio::spawn(async move {
            Streamer::stream(capture, tx_raw).await;
        });

        // process screens
        self.pipeline = create_stream_pipeline(rx_raw, tx_processed).unwrap();
        self.pipeline.set_state(gst::State::Playing).unwrap();
        let _ = self.pipeline.state(ClockTime::from_seconds(2));

        // test save
        thread::spawn(move || {
            while let Some(x) = rx_processed.blocking_recv() {
                println!("{:?}", x);
            }
            /*let pipeline = create_test_save_pipeline(rx_processed).unwrap();

            pipeline.set_state(gst::State::Playing).unwrap();
            let _ = pipeline.state(ClockTime::from_seconds(2));*/
        });


        /*let bus =  self.pipeline.bus();
        tokio::spawn(async move {
            let bus = bus.unwrap();
            for msg in bus.iter() {
                match msg.view() {
                    MessageView::Error(err) => {
                        println!(
                            "Errore ricevuto da {:?}: {:?}", err.debug(), err.message()
                        );
                        break;
                    }
                    MessageView::Eos(_) => {
                        println!("gstreamer received eos");
                        break;
                    }
                    _ => {}
                }
            }
        });*/

        /*let calla = WebRTCServer::new();

        tokio::spawn(async move {
            calla.send_video_frames(rx_processed).await.expect("send_video_frames webrtc error");
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
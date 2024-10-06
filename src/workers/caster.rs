use crate::assets::{FRAME_RATE, USE_WEBRTC};
use crate::gui::common::datastructure::ScreenRect;
use crate::utils::gist::create_stream_pipeline;
use crate::workers::WorkerClose;
use glib::prelude::ObjectExt;
use gstreamer::prelude::{ElementExt, ElementExtManual, GObjectExtManualGst, GstBinExt};
use gstreamer::Pipeline;
use gstreamer_app::gst;
use display_info::DisplayInfo;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crate::utils::net::webrtc::WebRTCServer;

#[derive(Debug, Clone)]
pub struct XMonitor {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    primary: bool,
    device_identifier: String,
}

unsafe impl Send for XMonitor {}

#[derive(Debug, Clone)]
pub struct Caster {
    init: bool,
    pub streaming_time: u64,
    pipeline: Pipeline,
    streaming: bool,
    blank_screen: bool,
    monitor: u32,
    monitors: HashMap<u32, XMonitor>,
    running: Arc<AtomicBool>,
}

impl WorkerClose for Caster {
    fn close(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if self.init {
            self.pause();
            let _ = self.pipeline.set_state(gst::State::Null).is_err();
        }
        self.init = false;
    }
}


impl Caster {
    pub fn new() -> Self {
        let (monitors, main) = Self::setup_monitors();
        Self {
            init: false,
            streaming_time: 0,
            pipeline: Default::default(),
            streaming: false,
            monitors,
            blank_screen: false,
            monitor: main,
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn resize_rec_area(&mut self, rect: ScreenRect) -> bool {
        self.lazy_init();

        let state = self.pipeline.current_state();

        if self.pipeline.set_state(gst::State::Paused).is_err() {
            return false;
        }

        let start_pos_x = rect.x as i32;
        let start_pos_y = rect.y as i32;

        let mon = self.monitors.get_mut(&self.monitor).unwrap();

        let right = if rect.width > 0.0 {
            mon.width as i32 - start_pos_x - rect.width as i32
        } else {
            0
        };

        let bottom = if rect.height > 0.0 {
            mon.height as i32 - start_pos_y - rect.height as i32
        } else {
            0
        };

        let videobox = self.pipeline.by_name("videobox").unwrap();

        videobox.set_property("left", start_pos_x);
        videobox.set_property("top", start_pos_y);
        videobox.set_property("right", right);
        videobox.set_property("bottom", bottom);

        self.pipeline.set_state(state).is_err()
    }

    pub fn change_monitor(&mut self, id: u32) -> bool {
        if !self.has_monitor(id) {
            return false;
        }

        let state = self.pipeline.current_state();

        if self.pipeline.set_state(gst::State::Paused).is_err() {
            return false;
        }

        self.monitor = id;

        self.pipeline.set_state(state).is_err()
    }

    pub fn current_monitor(&self) -> u32 {
        self.monitor
    }

    pub fn cast(&mut self) {
        self.lazy_init();
        self.streaming = !self.pipeline.set_state(gst::State::Playing).is_err();
    }

    fn lazy_init(&mut self) {
        if !self.init {
            self.init = true;

            let (tx_processed, rx_processed) = tokio::sync::mpsc::channel(FRAME_RATE as usize);

            // process screens
            self.pipeline = create_stream_pipeline(&(self.monitors.get(&self.monitor).unwrap().device_identifier), tx_processed, false).unwrap();

            let running = Arc::clone(&self.running);

            tokio::spawn(async move {
                //crate::utils::net::common::port_forwarding().expect("Failed to setup port forwarding.");
                // used for auto caster discovery
                crate::utils::net::common::caster_discover_service();

                if USE_WEBRTC {
                    let calla = WebRTCServer::new();
                    calla.send_video_frames(rx_processed, running).await;
                } else {
                    crate::utils::net::xgp::caster(rx_processed, running).await;
                }
            });
        }
    }

    pub fn pause(&mut self) -> bool {
        self.streaming = false;
        self.pipeline.set_state(gst::State::Paused).is_err()
    }

    pub fn toggle_blank_screen(&mut self) -> bool {
        let state = self.pipeline.current_state();

        if self.pipeline.set_state(gst::State::Paused).is_err() {
            return false;
        }

        self.blank_screen = !self.blank_screen;

        let videobox = self.pipeline.by_name("videobox").unwrap();

        if self.blank_screen {
            let mon = self.monitors.get(&self.monitor).unwrap();

            videobox.set_property("left", mon.width as i32);
            videobox.set_property("top", mon.height as i32);
            videobox.set_property("right", mon.width as i32);
            videobox.set_property("bottom", mon.height as i32);
            videobox.set_property_from_str("fill", "5");
        } else {
            videobox.set_property("left", 0i32);
            videobox.set_property("top", 0i32);
            videobox.set_property("right", 0i32);
            videobox.set_property("bottom", 0i32);
            videobox.set_property_from_str("fill", "0");
        }

        self.pipeline.set_state(state).is_err()
    }

    pub fn toggle_streaming(&mut self) {
        if self.streaming {
            self.pause();
        } else {
            self.cast();
        }
    }
    pub fn has_monitor(&self, id: u32) -> bool {
        self.monitors.contains_key(&id)
    }

    pub fn get_monitors(&self) -> Vec<u32> {
        let mut monitors = Vec::new();

        for x in self.monitors.iter() {
            monitors.push(x.0.clone());
        }

        monitors
    }

    pub fn is_streaming(&self) -> bool {
        self.streaming
    }

    fn setup_monitors() -> (HashMap<u32, XMonitor>, u32) {
        let mut monitors = HashMap::new();
        let mut main = 0;

        if let Ok(vec_display) = DisplayInfo::all() {
            for display in vec_display {
                monitors.insert(display.id, XMonitor {
                    x: display.x,
                    y: display.y,
                    height: display.height,
                    width: display.width,
                    primary: display.is_primary,
                    #[cfg(target_os = "windows")]
                    device_identifier: display.raw_handle.0.to_string(),
                    #[cfg(target_os = "macos")]
                    device_identifier: display.raw_handle.id.to_string(),
                    #[cfg(target_os = "linux")]
                    device_identifier: display.name.to_string(),
                });

                if display.is_primary {
                    main = display.id;
                }
            }
        }

        (monitors, main)
    }
}
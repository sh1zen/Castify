use crate::assets::{FRAME_RATE, TARGET_OS};
use crate::gui::common::datastructure::ScreenRect;
use crate::utils::gist::create_stream_pipeline;
use crate::utils::monitors::{Monitors, XMonitor};
use crate::utils::net::webrtc::WebRTCServer;
use crate::utils::sos::SignalOfStop;
use crate::workers::WorkerClose;
use glib::prelude::ObjectExt;
use gstreamer::prelude::{ElementExt, ElementExtManual, GObjectExtManualGst, GstBinExt};
use gstreamer::{Pipeline, State};
use gstreamer_app::gst;
use std::sync::Arc;

pub struct Caster {
    init: bool,
    pub streaming_time: u64,
    pipeline: Pipeline,
    streaming: bool,
    blank_screen: bool,
    monitors: Monitors,
    server: Arc<WebRTCServer>,
    sos: SignalOfStop,
    locked: bool,
}

impl Caster {
    pub fn new(sos: SignalOfStop) -> Self {
        Self {
            init: false,
            streaming_time: 0,
            pipeline: Default::default(),
            streaming: false,
            monitors: Monitors::new(),
            blank_screen: false,
            sos,
            server: WebRTCServer::new(),
            locked: false,
        }
    }

    pub fn resize_rec_area(&mut self, rect: ScreenRect) -> bool {
        self.lazy_init();

        if let Ok(state) = self.lock() {
            let start_pos_x = rect.x as i32;
            let start_pos_y = rect.y as i32;

            let mon = self.monitors.get_monitor().unwrap();

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

            videobox.set_property("left", start_pos_x as i32);
            videobox.set_property("top",start_pos_y as i32);
            videobox.set_property("right", right as i32);
            videobox.set_property("bottom", bottom as i32);

            self.unlock(state)
        } else {
            false
        }
    }

    pub fn get_monitor(&self) -> Option<&XMonitor> {
        self.monitors.get_monitor()
    }

    pub fn get_monitors(&self) -> Vec<u32> {
        self.monitors.get_monitors()
    }

    pub fn change_monitor(&mut self, id: u32) -> bool {
        self.lazy_init();

        if !self.monitors.has_monitor(id) {
            return false;
        }

        if let Ok(state) = self.hard_lock() {
            self.monitors.change_monitor(id);

            let element = self.pipeline.by_name("src").unwrap();
            let mon = self.monitors.get_monitor().unwrap();

            println!("changing mon: {:?}", mon);

            match TARGET_OS {
                "windows" => {
                    element.set_property_from_str("monitor-handle", &*mon.dev_id);
                }
                "macos" => {
                    element.set_property_from_str("device-index", &*mon.dev_id);
                }
                "linux" => {
                    element.set_property("startx", mon.x as u32);
                    element.set_property("starty", mon.y as u32);
                    element.set_property("endx", mon.width + mon.x as u32 - 1);
                    element.set_property("endy", mon.height + mon.y as u32 - 1);
                }
                _ => { unreachable!("TargetOS not supported") }
            }

            self.unlock(state)
        } else {
            false
        }
    }

    pub fn current_monitor_id(&self) -> u32 {
        self.monitors.get_monitor_id()
    }

    pub fn cast(&mut self) {
        self.lazy_init();
        self.streaming = self.pipeline.set_state(State::Playing).is_ok();
    }

    fn lazy_init(&mut self) {
        if !self.init {
            self.init = true;

            let (tx_processed, rx_processed) = tokio::sync::mpsc::channel(FRAME_RATE as usize);

            // process screens
            self.pipeline = create_stream_pipeline(self.monitors.get_monitor().unwrap(), tx_processed).unwrap();

            self.sos.spawn(async move {
                // used for auto caster discovery
                match crate::utils::net::common::caster_discover_service() {
                    Ok(_) => {
                        println!("Caster running and registered on mDNS");
                    }
                    Err(e) => {
                        println!("mDNS Error:: {}", e);
                    }
                }

                if let Err(e) = crate::utils::net::common::port_forwarding() {
                    println!("Failed to setup port forwarding: {}", e)
                }
            });

            Arc::clone(&self.server).run();
            self.server.get_handler().send_video_frames(rx_processed);
        }
    }

    pub fn pause(&mut self) -> bool {
        self.streaming = false;
        self.pipeline.set_state(State::Paused).is_ok()
    }

    pub fn toggle_blank_screen(&mut self) -> bool {
        if let Ok(state) = self.lock() {
            self.blank_screen = !self.blank_screen;

            let videobox = self.pipeline.by_name("videofilter").unwrap();

            if self.blank_screen {
                videobox.set_property("saturation", 0f64);
                videobox.set_property("contrast", 0f64);
                videobox.set_property("brightness", 1f64);
                videobox.set_property("hue", 0f64);
            } else {
                videobox.set_property("saturation", 1f64);
                videobox.set_property("contrast", 1f64);
                videobox.set_property("brightness", 0f64);
                videobox.set_property("hue", 0f64);
            }

            self.unlock(state)
        } else {
            false
        }
    }

    pub fn toggle_streaming(&mut self) {
        if self.streaming {
            self.pause();
        } else {
            self.cast();
        }
    }

    pub fn is_streaming(&self) -> bool {
        self.streaming
    }

    fn lock(&mut self) -> Result<State, bool> {
        if self.locked {
            return Err(false);
        }
        self.locked = true;
        let state = self.pipeline.current_state();

        if self.pipeline.set_state(State::Paused).is_err() {
            Err(false)
        } else {
            Ok(state)
        }
    }

    fn hard_lock(&mut self) -> Result<State, bool> {
        if let Ok(state) = self.lock() {
            if self.pipeline.set_state(State::Ready).is_err() {
                Err(false)
            } else {
                Ok(state)
            }
        } else {
            Err(false)
        }
    }

    fn unlock(&mut self, state: State) -> bool {
        let res = self.pipeline.set_state(state).is_err();
        self.locked = false;
        res
    }

    pub fn get_connection_handler(&self) -> Arc<WebRTCServer> {
        Arc::clone(&self.server)
    }
}

impl WorkerClose for Caster {
    fn close(&mut self) {
        if self.init {
            self.pause();
            self.server.close();
            let _ = self.pipeline.set_state(gst::State::Null).is_err();
            self.init = false;
            self.blank_screen = false;
            self.pipeline = Default::default();
        }
    }
}
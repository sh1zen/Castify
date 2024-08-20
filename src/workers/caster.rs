use crate::capture::Capture;
use crate::gui::resource::FRAME_RATE;
use once_cell::sync::Lazy;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub struct Caster {
    pub streaming: bool,
    blank_screen: bool,
    init: bool,
    pub monitor: u32,
}

impl Caster {
    pub fn new() -> Self {
        Self {
            streaming: false,
            blank_screen: false,
            init: false,
            monitor: Capture::get_main(),
        }
    }

    pub fn cast_screen(&mut self) {
        self.streaming = true;

        if !self.init {
            self.init = true;

            let (tx, rx) = tokio::sync::mpsc::channel(3);

            // generate frames
            tokio::spawn(async move {
                let mut capture = Capture::new();
                capture.set_framerate(FRAME_RATE as f32);
                capture.stream(0, tx).await;
            });

            // send frames over the local network
            tokio::spawn(async move {
                crate::utils::net::caster(rx).await;
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
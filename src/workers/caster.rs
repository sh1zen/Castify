use std::sync::Arc;
use log::{info, error};
use crate::capture::capturer::{Capturer, CropRect};
use crate::capture::display::DisplaySelector;
use crate::capture::ScreenCaptureImpl;
use crate::gui::common::datastructure::ScreenRect;
use crate::utils::net::webrtc::WebRTCServer;
use crate::utils::sos::SignalOfStop;

pub struct Caster {
    init: bool,
    pub streaming_time: u64,
    streaming: bool,
    blank_screen: bool,
    capturer: Capturer,
    server: Arc<WebRTCServer>,
    sos: SignalOfStop,
}

impl Caster {
    pub fn new(fps: u32, sos: SignalOfStop) -> Self {
        Self {
            init: false,
            streaming_time: 0,
            streaming: false,
            blank_screen: false,
            capturer: Capturer::new(fps),
            server: WebRTCServer::new(),
            sos,
        }
    }

    /// Inizializza il caster: avvia mDNS, port forwarding, WebRTC server
    /// e collega il canale video dal Capturer al server.
    fn lazy_init(&mut self) {
        if self.init {
            return;
        }
        self.init = true;

        // Avvia la cattura e ottieni il canale con i frame H.264
        let handle = tokio::runtime::Handle::current();
        let rx = match tokio::task::block_in_place(|| handle.block_on(self.capturer.start())) {
            Ok(rx) => rx,
            Err(e) => {
                error!("Failed to start capturer: {}", e);
                self.init = false;
                return;
            }
        };

        // mDNS discovery + port forwarding in background
        self.sos.spawn(async move {
            match crate::utils::net::common::caster_discover_service() {
                Ok(_) => info!("Caster running and registered on mDNS"),
                Err(e) => error!("mDNS Error: {}", e),
            }
            if let Err(e) = crate::utils::net::common::port_forwarding() {
                error!("Failed to setup port forwarding: {}", e);
            }
        });

        // Avvia il server WebRTC e inoltra i frame
        Arc::clone(&self.server).run();
        self.server.get_handler().send_video_frames(rx);
    }

    // ── Streaming control ───────────────────────────────────────

    pub fn cast(&mut self) {
        self.lazy_init();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.capturer.play())
        });
        self.streaming = true;
    }

    pub fn pause(&mut self) -> bool {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.capturer.pause())
        });
        self.streaming = false;
        true
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

    // ── Display / monitor management ────────────────────────────

    pub fn get_displays(
        &self,
    ) -> Vec<<ScreenCaptureImpl as DisplaySelector>::Display> {
        self.capturer.available_displays()
    }

    pub fn change_display(
        &mut self,
        display: <ScreenCaptureImpl as DisplaySelector>::Display,
    ) {
        self.capturer.select_display(display);
    }

    // ── Blank screen ────────────────────────────────────────────

    pub fn is_blank_screen(&self) -> bool {
        self.blank_screen
    }

    pub fn toggle_blank_screen(&mut self) {
        self.blank_screen = !self.blank_screen;
        self.capturer.set_blank_screen(self.blank_screen);
    }

    // ── Resize recording area ───────────────────────────────────

    pub fn resize_rec_area(&mut self, rect: ScreenRect) -> bool {
        let crop = if rect.width > 0.0 && rect.height > 0.0 {
            Some(CropRect::from(&rect))
        } else {
            None
        };
        self.capturer.set_crop(crop);
        true
    }

    // ── WebRTC ──────────────────────────────────────────────────

    pub fn get_connection_handler(&self) -> Arc<WebRTCServer> {
        Arc::clone(&self.server)
    }
}

// ── Cleanup ─────────────────────────────────────────────────────

impl Caster {
    pub fn close(&mut self) {
        if self.init {
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(self.capturer.stop())
            });
            self.server.close();
            self.init = false;
            self.streaming = false;
            self.blank_screen = false;
            info!("Caster closed");
        }
    }
}
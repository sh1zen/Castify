use crate::capture::ScreenCaptureImpl;
use crate::capture::audio::AudioCapture;
use crate::capture::capturer::{Capturer, CropRect};
use crate::capture::display::DisplaySelector;
use crate::gui::common::datastructure::ScreenRect;
use crate::pipeline::clock::MediaClock;
use crate::pipeline::health::PipelineHealth;
use crate::pipeline::state::PipelineState;
use crate::utils::net::webrtc::WebRTCServer;
use crate::utils::sos::SignalOfStop;
use log::{error, info};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio_util::sync::CancellationToken;

pub struct Caster {
    init: bool,
    pub streaming_time: u64,
    streaming: bool,
    blank_screen: bool,
    audio_muted: Arc<AtomicBool>,
    audio_cancel: Option<CancellationToken>,
    capturer: Capturer,
    server: Arc<WebRTCServer>,
    sos: SignalOfStop,

    // Pipeline integration
    clock: MediaClock,
    health: Arc<PipelineHealth>,
    pipeline_state: PipelineState,
}

impl Caster {
    pub fn new(fps: u32, sos: SignalOfStop) -> Self {
        let clock = MediaClock::new();
        let health = Arc::new(PipelineHealth::new());

        Self {
            init: false,
            streaming_time: 0,
            streaming: false,
            blank_screen: false,
            audio_muted: Arc::new(AtomicBool::new(false)),
            audio_cancel: None,
            capturer: Capturer::new(fps),
            server: WebRTCServer::new(),
            sos,
            clock,
            health,
            pipeline_state: PipelineState::Idle,
        }
    }

    /// Get the media clock for timestamp correlation
    pub fn clock(&self) -> &MediaClock {
        &self.clock
    }

    /// Get the pipeline health metrics
    pub fn health(&self) -> &Arc<PipelineHealth> {
        &self.health
    }

    /// Get the current pipeline state
    pub fn pipeline_state(&self) -> &PipelineState {
        &self.pipeline_state
    }

    /// Inizializza il caster: avvia mDNS, port forwarding, WebRTC server
    /// e collega il canale video dal Capturer al server.
    fn lazy_init(&mut self) {
        if self.init {
            return;
        }
        self.init = true;
        self.pipeline_state = PipelineState::Initializing;

        // Avvia la cattura e ottieni il canale con i frame H.264
        let handle = tokio::runtime::Handle::current();
        let rx = match tokio::task::block_in_place(|| handle.block_on(self.capturer.start())) {
            Ok(rx) => rx,
            Err(e) => {
                error!("Failed to start capturer: {}", e);
                self.init = false;
                self.pipeline_state = PipelineState::Idle;
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

        // Link the encoder's force_idr flag to the server so new peers trigger IDR
        self.server.set_force_idr(self.capturer.force_idr());

        // Avvia il server WebRTC e inoltra i frame
        Arc::clone(&self.server).run();
        self.server.get_handler().send_video_frames(rx);

        self.start_audio_capture(true);

        // Start health monitoring
        let health = self.health.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            loop {
                interval.tick().await;
                let summary = health.summary();
                info!("Sender pipeline health: {}", summary);
            }
        });

        self.pipeline_state = PipelineState::Running {
            started_at: std::time::Instant::now(),
        };
        info!("Pipeline state: {}", self.pipeline_state);
    }

    // ── Streaming control ───────────────────────────────────────

    pub fn cast(&mut self) {
        self.lazy_init();
        self.capturer.play();
        self.streaming = true;
    }

    pub fn pause(&mut self) -> bool {
        self.capturer.pause();
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

    pub fn get_displays(&self) -> Vec<<ScreenCaptureImpl as DisplaySelector>::Display> {
        self.capturer.available_displays()
    }

    pub fn change_display(&mut self, display: <ScreenCaptureImpl as DisplaySelector>::Display) {
        self.capturer.select_display(display);
    }

    pub fn get_selected_display(&self) -> Option<<ScreenCaptureImpl as DisplaySelector>::Display> {
        self.capturer.selected_display()
    }

    // ── Blank screen ────────────────────────────────────────────

    pub fn is_blank_screen(&self) -> bool {
        self.blank_screen
    }

    pub fn toggle_blank_screen(&mut self) {
        self.blank_screen = !self.blank_screen;
        self.capturer.set_blank_screen(self.blank_screen);
    }

    // ── Audio mute ──────────────────────────────────────────────

    pub fn is_audio_muted(&self) -> bool {
        self.audio_muted.load(Ordering::Relaxed)
    }

    pub fn toggle_audio_mute(&mut self) {
        let muted = !self.audio_muted.load(Ordering::Relaxed);
        self.audio_muted.store(muted, Ordering::Relaxed);
        if muted {
            self.stop_audio_capture();
            info!("Audio muted");
        } else {
            self.start_audio_capture(false);
        }
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
            self.pipeline_state = PipelineState::Stopping;
            self.stop_audio_capture();
            self.capturer.stop();
            self.server.close();
            self.init = false;
            self.streaming = false;
            self.blank_screen = false;
            self.pipeline_state = PipelineState::Stopped;
            info!("Caster closed (pipeline state: {})", self.pipeline_state);
        }
    }
}

impl Caster {
    fn start_audio_capture(&mut self, initial_start: bool) {
        let audio_cancel = CancellationToken::new();
        match AudioCapture::start(audio_cancel.clone()) {
            Ok(audio_rx) => {
                self.server.get_handler().send_audio_frames(audio_rx);
                if initial_start {
                    info!("Audio capture started");
                } else {
                    info!("Audio unmuted");
                }
            }
            Err(e) => {
                if initial_start {
                    error!("Failed to start audio capture: {}", e);
                } else {
                    error!("Failed to restart audio capture: {}", e);
                }
            }
        }
        self.audio_cancel = Some(audio_cancel);
    }

    fn stop_audio_capture(&mut self) {
        if let Some(cancel) = self.audio_cancel.take() {
            cancel.cancel();
        }
    }
}

impl crate::workers::WorkerClose for Caster {
    fn close(&mut self) {
        Caster::close(self);
    }
}

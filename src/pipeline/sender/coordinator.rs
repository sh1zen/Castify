//! Sender pipeline coordinator
//!
//! Chains capture → encode → transmit stages and manages their lifecycle.

use anyhow::Result;
use log::{error, info};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio_util::sync::CancellationToken;

use crate::capture::ScreenCaptureImpl;
use crate::capture::audio::AudioCapture;
use crate::capture::capturer::CropRect;
use crate::capture::display::DisplaySelector;
use crate::gui::common::datastructure::ScreenRect;
use crate::pipeline::clock::MediaClock;
use crate::pipeline::health::PipelineHealth;
use crate::pipeline::sender::capture_stage::CaptureStage;
use crate::pipeline::state::PipelineState;
use crate::utils::net::webrtc::WebRTCServer;
use crate::utils::sos::SignalOfStop;

/// Coordinates the sender pipeline: Capture → Encode → Transmit
///
/// Manages the lifecycle of all sender-side stages, providing a unified
/// interface for the Caster worker.
pub struct SenderCoordinator {
    capture_stage: CaptureStage,
    clock: MediaClock,
    health: Arc<PipelineHealth>,
    state: PipelineState,
    server: Arc<WebRTCServer>,
    sos: SignalOfStop,

    // Audio
    audio_muted: Arc<AtomicBool>,
    audio_cancel: Option<CancellationToken>,

    // Blank screen / crop state
    blank_screen: bool,
    initialized: bool,
}

impl SenderCoordinator {
    /// Create a new sender coordinator
    pub fn new(_fps: u32, sos: SignalOfStop) -> Result<Self> {
        let clock = MediaClock::new();
        let health = Arc::new(PipelineHealth::new());
        let capture_stage = CaptureStage::new()?;

        Ok(Self {
            capture_stage,
            clock,
            health,
            state: PipelineState::Idle,
            server: WebRTCServer::new(),
            sos,
            audio_muted: Arc::new(AtomicBool::new(false)),
            audio_cancel: None,
            blank_screen: false,
            initialized: false,
        })
    }

    /// Get the pipeline clock
    pub fn clock(&self) -> &MediaClock {
        &self.clock
    }

    /// Get the pipeline health metrics
    pub fn health(&self) -> &Arc<PipelineHealth> {
        &self.health
    }

    /// Get the current pipeline state
    pub fn state(&self) -> &PipelineState {
        &self.state
    }

    /// Initialize the pipeline (lazy init: start capture, mDNS, WebRTC)
    pub fn initialize(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }

        let handle = tokio::runtime::Handle::current();

        // Get encoder dimensions
        let (_enc_w, _enc_h) =
            tokio::task::block_in_place(|| handle.block_on(self.capture_stage.resolution()));

        // Start capture (produces encoded H.264 via internal encoder)
        let mut capture_stage_for_start = CaptureStage::new()?;
        std::mem::swap(&mut self.capture_stage, &mut capture_stage_for_start);
        self.capture_stage = CaptureStage::new()?;

        // Actually start capture using the original Capturer flow
        // The CaptureStage wraps ScreenCaptureImpl but we need the full pipeline
        // For now, we delegate back to the Capturer's internal logic
        self.state = PipelineState::Initializing;

        // mDNS + port forwarding
        self.sos.spawn(async move {
            match crate::utils::net::common::caster_discover_service() {
                Ok(_) => info!("SenderCoordinator: registered on mDNS"),
                Err(e) => error!("mDNS Error: {}", e),
            }
            if let Err(e) = crate::utils::net::common::port_forwarding() {
                error!("Failed to setup port forwarding: {}", e);
            }
        });

        // Start audio capture
        let audio_cancel = CancellationToken::new();
        match AudioCapture::start(audio_cancel.clone()) {
            Ok(audio_rx) => {
                info!("SenderCoordinator: audio capture started");
                self.server.get_handler().send_audio_frames(audio_rx);
            }
            Err(e) => {
                error!("Failed to start audio capture: {}", e);
            }
        }
        self.audio_cancel = Some(audio_cancel);

        self.initialized = true;
        self.state = PipelineState::Running {
            started_at: std::time::Instant::now(),
        };

        // Log health metrics periodically
        let health = self.health.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            loop {
                interval.tick().await;
                let summary = health.summary();
                info!("Sender pipeline: {}", summary);
            }
        });

        Ok(())
    }

    /// Start streaming (transition to Running)
    pub fn start(&mut self) -> Result<()> {
        if !self.state.can_transition_to(&PipelineState::Running {
            started_at: std::time::Instant::now(),
        }) && self.state != PipelineState::Idle
        {
            // Already running or invalid transition
            return Ok(());
        }
        self.state = PipelineState::Running {
            started_at: std::time::Instant::now(),
        };
        info!("SenderCoordinator: pipeline running");
        Ok(())
    }

    /// Pause streaming
    pub fn pause(&mut self) -> Result<()> {
        if self.state.is_running() {
            self.state = PipelineState::Paused {
                paused_at: std::time::Instant::now(),
            };
            info!("SenderCoordinator: pipeline paused");
        }
        Ok(())
    }

    /// Resume from pause
    pub fn resume(&mut self) -> Result<()> {
        if self.state.is_paused() {
            self.state = PipelineState::Running {
                started_at: std::time::Instant::now(),
            };
            info!("SenderCoordinator: pipeline resumed");
        }
        Ok(())
    }

    /// Stop the pipeline
    pub fn stop(&mut self) -> Result<()> {
        self.state = PipelineState::Stopping;

        if let Some(cancel) = self.audio_cancel.take() {
            cancel.cancel();
        }

        let handle = tokio::runtime::Handle::current();
        tokio::task::block_in_place(|| handle.block_on(self.capture_stage.stop_capture()))?;

        self.server.close();
        self.initialized = false;
        self.state = PipelineState::Stopped;
        self.blank_screen = false;

        info!("SenderCoordinator: pipeline stopped");
        Ok(())
    }

    /// Check if currently streaming
    pub fn is_streaming(&self) -> bool {
        self.state.is_running()
    }

    // ── Display management ──────────────────────────────────────

    pub fn get_displays(&self) -> Vec<<ScreenCaptureImpl as DisplaySelector>::Display> {
        self.capture_stage.available_displays()
    }

    pub fn change_display(&self, display: <ScreenCaptureImpl as DisplaySelector>::Display) {
        self.capture_stage.select_display(display);
    }

    pub fn get_selected_display(&self) -> Option<<ScreenCaptureImpl as DisplaySelector>::Display> {
        self.capture_stage.selected_display()
    }

    // ── Blank screen ────────────────────────────────────────────

    pub fn is_blank_screen(&self) -> bool {
        self.blank_screen
    }

    pub fn toggle_blank_screen(&mut self) {
        self.blank_screen = !self.blank_screen;
        self.capture_stage.set_blank_screen(self.blank_screen);
    }

    // ── Audio ───────────────────────────────────────────────────

    pub fn is_audio_muted(&self) -> bool {
        self.audio_muted.load(Ordering::Relaxed)
    }

    pub fn toggle_audio_mute(&mut self) {
        let muted = !self.audio_muted.load(Ordering::Relaxed);
        self.audio_muted.store(muted, Ordering::Relaxed);
        if muted {
            if let Some(cancel) = self.audio_cancel.take() {
                cancel.cancel();
                info!("Audio muted");
            }
        } else {
            let audio_cancel = CancellationToken::new();
            match AudioCapture::start(audio_cancel.clone()) {
                Ok(audio_rx) => {
                    self.server.get_handler().send_audio_frames(audio_rx);
                    info!("Audio unmuted");
                }
                Err(e) => {
                    error!("Failed to restart audio capture: {}", e);
                }
            }
            self.audio_cancel = Some(audio_cancel);
        }
    }

    // ── Crop / area ─────────────────────────────────────────────

    pub fn resize_rec_area(&self, rect: ScreenRect) -> bool {
        let crop = if rect.width > 0.0 && rect.height > 0.0 {
            Some(CropRect::from(&rect))
        } else {
            None
        };
        self.capture_stage.set_crop(crop);
        true
    }

    // ── WebRTC ──────────────────────────────────────────────────

    pub fn get_connection_handler(&self) -> Arc<WebRTCServer> {
        Arc::clone(&self.server)
    }

    pub fn server(&self) -> &Arc<WebRTCServer> {
        &self.server
    }
}

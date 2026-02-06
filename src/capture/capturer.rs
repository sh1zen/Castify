use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::{
    select,
    sync::{mpsc, watch, Mutex, Notify},
};
use log::{info, error};

use crate::capture::{ScreenCapture, ScreenCaptureImpl};
use crate::capture::display::DisplaySelector;
use crate::gui::common::datastructure::ScreenRect;
use crate::encoder::FfmpegEncoder;

// ── Stato interno ───────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CaptureState {
    Playing,
    Paused,
    Stopped,
}

/// Opzioni dinamiche che possono cambiare a runtime.
/// Vengono lette dal loop di cattura ad ogni frame tramite `watch`.
#[derive(Debug, Clone)]
pub struct CaptureOpts {
    pub blank_screen: bool,
    pub crop: Option<CropRect>,
}

#[derive(Debug, Clone, Copy)]
pub struct CropRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl From<&ScreenRect> for CropRect {
    fn from(r: &ScreenRect) -> Self {
        Self {
            x: r.x.max(0.0) as u32,
            y: r.y.max(0.0) as u32,
            w: r.width.max(0.0) as u32,
            h: r.height.max(0.0) as u32,
        }
    }
}

// ── Capturer ────────────────────────────────────────────────────

pub struct Capturer {
    capture: Arc<Mutex<ScreenCaptureImpl>>,
    state: Arc<Mutex<CaptureState>>,
    pause_notify: Arc<Notify>,
    stop_notify: Arc<Notify>,
    opts_tx: watch::Sender<CaptureOpts>,
    opts_rx: watch::Receiver<CaptureOpts>,
    force_idr: Arc<AtomicBool>,
}

impl Capturer {
    pub fn new(_fps: u32) -> Self {
        let display_capture = ScreenCaptureImpl::new_default()
            .expect("Failed to create screen capture");

        let default_opts = CaptureOpts {
            blank_screen: false,
            crop: None,
        };
        let (opts_tx, opts_rx) = watch::channel(default_opts);

        Self {
            capture: Arc::new(Mutex::new(display_capture)),
            state: Arc::new(Mutex::new(CaptureState::Stopped)),
            pause_notify: Arc::new(Notify::new()),
            stop_notify: Arc::new(Notify::new()),
            opts_tx,
            opts_rx,
            force_idr: Arc::new(AtomicBool::new(false)),
        }
    }

    // ── Avvio cattura ───────────────────────────────────────────

    /// Avvia la cattura e ritorna il canale con i frame H.264 codificati.
    pub async fn start(&mut self) -> Result<mpsc::Receiver<Vec<u8>>, anyhow::Error> {
        {
            let mut state = self.state.lock().await;
            if *state != CaptureState::Stopped {
                return Err(anyhow::anyhow!("Capturer already running"));
            }
            *state = CaptureState::Playing;
        }

        // Crop-aware encoder resolution: if crop is set, use crop dimensions
        let (enc_w, enc_h) = {
            let opts = self.opts_rx.borrow();
            if let Some(crop) = &opts.crop {
                // Ensure even dimensions for NV12 chroma alignment
                let w = crop.w + (crop.w % 2);
                let h = crop.h + (crop.h % 2);
                (w, h)
            } else {
                let cap = self.capture.lock().await;
                cap.display().resolution()
            }
        };

        let (tx, rx) = mpsc::channel::<Vec<u8>>(4);
        let (frame_tx, mut frame_rx) = mpsc::channel::<bytes::Bytes>(2);

        let capture = self.capture.clone();
        let pause_notify = self.pause_notify.clone();
        let stop_notify = self.stop_notify.clone();
        let state_ref = self.state.clone();
        let opts_rx = self.opts_rx.clone();

        // Create encoder and capture its force_idr before moving it
        let encoder = FfmpegEncoder::new(enc_w, enc_h);
        self.force_idr = encoder.force_idr.clone();
        let force_idr = self.force_idr.clone();

        tokio::spawn(async move {
            // Avvia la cattura interna (scrive frame codificati in frame_tx)
            {
                let mut cap = capture.lock().await;
                if let Err(e) = cap.start_capture(encoder, frame_tx, opts_rx).await {
                    error!("Capture start failed: {}", e);
                    return;
                }
            }

            info!("Capture loop started");

            loop {
                select! {
                    frame = frame_rx.recv() => {
                        let Some(raw) = frame else {
                            info!("Frame channel closed, stopping");
                            break;
                        };

                        let s = *state_ref.lock().await;
                        match s {
                            CaptureState::Paused => {
                                pause_notify.notified().await;
                                continue;
                            }
                            CaptureState::Stopped => break,
                            CaptureState::Playing => {}
                        }

                        // Use try_send to avoid backpressure blocking the capture thread
                        if tx.try_send(Vec::from(raw)).is_err() {
                            log::warn!("Encoded frame dropped (channel full), requesting IDR");
                            force_idr.store(true, Ordering::Relaxed);
                        }
                    }

                    _ = stop_notify.notified() => {
                        info!("Capture stopped via signal");
                        break;
                    }
                }
            }

            // Cleanup
            let mut cap = capture.lock().await;
            if let Err(e) = cap.stop_capture().await {
                error!("Capture stop failed: {}", e);
            }
            info!("Capture loop exited");
        });

        Ok(rx)
    }

    // ── Controllo stato ─────────────────────────────────────────

    pub async fn play(&self) {
        let mut state = self.state.lock().await;
        if *state == CaptureState::Paused {
            *state = CaptureState::Playing;
            self.pause_notify.notify_waiters();
            info!("Capture resumed");
        }
    }

    pub async fn pause(&self) {
        let mut state = self.state.lock().await;
        if *state == CaptureState::Playing {
            *state = CaptureState::Paused;
            info!("Capture paused");
        }
    }

    pub async fn stop(&self) {
        let mut state = self.state.lock().await;
        if *state != CaptureState::Stopped {
            *state = CaptureState::Stopped;
            self.stop_notify.notify_waiters();
            info!("Capture fully stopped");
        }
    }

    pub async fn is_playing(&self) -> bool {
        *self.state.lock().await == CaptureState::Playing
    }

    // ── Force IDR ─────────────────────────────────────────────

    /// Get the force_idr flag (shared with the encoder).
    /// Setting this to true will make the next encoded frame an IDR/keyframe.
    pub fn force_idr(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.force_idr)
    }

    // ── Opzioni dinamiche ───────────────────────────────────────

    /// Attiva/disattiva lo schermo nero (sostituisce i frame con dati vuoti).
    pub fn set_blank_screen(&self, blank: bool) {
        self.opts_tx.send_modify(|o| o.blank_screen = blank);
        info!("Blank screen: {}", blank);
    }

    /// Imposta (o rimuove) l'area di crop.
    pub fn set_crop(&self, rect: Option<CropRect>) {
        self.opts_tx.send_modify(|o| o.crop = rect);
        info!("Crop: {:?}", rect);
    }

    // ── Display management ──────────────────────────────────────

    pub fn available_displays(&self) -> Vec<<ScreenCaptureImpl as DisplaySelector>::Display> {
        // `try_lock` è ok qui: chiamato solo quando NON siamo nel loop di cattura
        match self.capture.try_lock() {
            Ok(mut cap) => cap.available_displays().unwrap_or_default(),
            Err(_) => {
                error!("Cannot list displays while capture is locked");
                Vec::new()
            }
        }
    }

    pub fn select_display(&self, display: <ScreenCaptureImpl as DisplaySelector>::Display) {
        match self.capture.try_lock() {
            Ok(mut cap) => {
                if let Err(e) = cap.select_display(&display) {
                    error!("Failed to select display: {}", e);
                }
            }
            Err(_) => {
                error!("Cannot change display while capture is running");
            }
        }
    }

    pub fn selected_display(&self) -> Option<<ScreenCaptureImpl as DisplaySelector>::Display> {
        match self.capture.try_lock() {
            Ok(cap) => cap.selected_display().unwrap_or(None),
            Err(_) => None,
        }
    }
}


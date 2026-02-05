use std::sync::Arc;
use std::time::Duration;
use tokio::{
    select,
    sync::{mpsc, watch, Mutex, Notify},
    time::interval,
};
use log::{info, error};

use crate::assets::FRAME_RATE;
use crate::capture::{ScreenCapture, ScreenCaptureImpl};
use crate::capture::display::DisplaySelector;
use crate::config::Config;
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
struct CaptureOpts {
    blank_screen: bool,
    crop: Option<CropRect>,
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
    fps: u32,
    capture: Arc<Mutex<ScreenCaptureImpl>>,
    state: Arc<Mutex<CaptureState>>,
    pause_notify: Arc<Notify>,
    stop_notify: Arc<Notify>,
    opts_tx: watch::Sender<CaptureOpts>,
    opts_rx: watch::Receiver<CaptureOpts>,
}

impl Capturer {
    pub fn new(fps: u32) -> Self {
        let display_capture = ScreenCaptureImpl::new_default()
            .expect("Failed to create screen capture");

        let default_opts = CaptureOpts {
            blank_screen: false,
            crop: None,
        };
        let (opts_tx, opts_rx) = watch::channel(default_opts);

        Self {
            capture: Arc::new(Mutex::new(display_capture)),
            fps,
            state: Arc::new(Mutex::new(CaptureState::Stopped)),
            pause_notify: Arc::new(Notify::new()),
            stop_notify: Arc::new(Notify::new()),
            opts_tx,
            opts_rx,
        }
    }

    // ── Avvio cattura ───────────────────────────────────────────

    /// Avvia la cattura e ritorna il canale con i frame H.264 codificati.
    pub async fn start(&self) -> Result<mpsc::Receiver<Vec<u8>>, anyhow::Error> {
        {
            let mut state = self.state.lock().await;
            if *state != CaptureState::Stopped {
                return Err(anyhow::anyhow!("Capturer already running"));
            }
            *state = CaptureState::Playing;
        }

        // Leggiamo la risoluzione prima dello spawn per evitare problemi con `self`
        let resolution = {
            let cap = self.capture.lock().await;
            cap.display().resolution()
        };

        let (tx, rx) = mpsc::channel::<Vec<u8>>(FRAME_RATE as usize);
        let (frame_tx, mut frame_rx) = mpsc::channel::<bytes::Bytes>(FRAME_RATE as usize);

        let capture = self.capture.clone();
        let pause_notify = self.pause_notify.clone();
        let stop_notify = self.stop_notify.clone();
        let state_ref = self.state.clone();
        let mut opts_rx = self.opts_rx.clone();
        let fps = self.fps;

        tokio::spawn(async move {
            // Avvia la cattura interna (scrive frame codificati in frame_tx)
            {
                let mut cap = capture.lock().await;
                let encoder = FfmpegEncoder::new(resolution.0, resolution.1);
                if let Err(e) = cap.start_capture(encoder, frame_tx).await {
                    error!("Capture start failed: {}", e);
                    return;
                }
            }

            let mut ticker = interval(Duration::from_millis(1000 / fps as u64));
            info!("Capture loop started @ {} fps", fps);

            loop {
                select! {
                    // Frame codificato in arrivo dal layer di cattura
                    frame = frame_rx.recv() => {
                        let Some(raw) = frame else {
                            info!("Frame channel closed, stopping");
                            break;
                        };

                        // Controlla stato
                        let s = *state_ref.lock().await;
                        match s {
                            CaptureState::Paused => {
                                pause_notify.notified().await;
                                continue;
                            }
                            CaptureState::Stopped => break,
                            CaptureState::Playing => {}
                        }

                        // Leggi opzioni correnti
                        let opts = opts_rx.borrow_and_update().clone();

                        let output = if opts.blank_screen {
                            blank_frame(raw.len())
                        } else if let Some(crop) = &opts.crop {
                            crop_frame(&raw, crop)
                        } else {
                            Vec::from(raw)
                        };

                        let _ = tx.try_send(output);
                        ticker.tick().await;
                    }

                    // Segnale di stop esterno
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
}

// ── Helpers frame ───────────────────────────────────────────────

/// Genera un frame nero (tutti zeri) della dimensione data.
fn blank_frame(size: usize) -> Vec<u8> {
    vec![0u8; size]
}

/// Applica il crop al frame raw.
/// NB: Placeholder — l'implementazione reale dipende dal formato pixel
/// (NV12, I420, BGRA …). Per ora restituisce il frame intero.
fn crop_frame(raw: &[u8], _crop: &CropRect) -> Vec<u8> {
    // TODO: implementare crop reale in base al pixel format
    raw.to_vec()
}
use log::{error, info};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use tokio::{
    select,
    sync::{Mutex, Notify, mpsc, watch},
};

use crate::assets::FRAME_RATE;
use crate::capture::display::DisplaySelector;
use crate::capture::{ScreenCapture, ScreenCaptureImpl};
use crate::encoder::FfmpegEncoder;
use crate::gui::common::datastructure::ScreenRect;

// ── Stato interno ───────────────────────────────────────────────

/// Capture state values for atomic access.
/// Using u8 allows for efficient lock-free operations across threads.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CaptureState {
    Playing = 0,
    Paused = 1,
    Stopped = 2,
}

impl CaptureState {
    /// Convert from u8 value. Returns Stopped for invalid values.
    #[inline]
    fn from_u8(value: u8) -> Self {
        match value {
            0 => CaptureState::Playing,
            1 => CaptureState::Paused,
            _ => CaptureState::Stopped,
        }
    }
}

/// Opzioni dinamiche che possono cambiare a runtime.
/// Vengono lette dal loop di cattura ad ogni frame tramite `watch`.
#[derive(Debug, Clone)]
pub struct CaptureOpts {
    pub blank_screen: bool,
    pub crop: Option<CropRect>,
    pub paused: bool,
    pub max_fps: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
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
    /// Uses atomic u8 for lock-free state access across threads.
    state: Arc<AtomicU8>,
    pause_notify: Arc<Notify>,
    stop_notify: Arc<Notify>,
    opts_tx: watch::Sender<CaptureOpts>,
    opts_rx: watch::Receiver<CaptureOpts>,
    force_idr: Arc<AtomicBool>,
}

#[derive(Debug, Clone)]
pub struct EncodedFrame {
    pub data: Vec<u8>,
    pub sequence_number: u64,
    pub timestamp_ms: u64,
}

impl Capturer {
    pub fn new(_fps: u32) -> Self {
        let display_capture =
            ScreenCaptureImpl::new_default().expect("Failed to create screen capture");

        let default_opts = CaptureOpts {
            blank_screen: false,
            crop: None,
            paused: false,
            max_fps: FRAME_RATE,
        };
        let (opts_tx, opts_rx) = watch::channel(default_opts);

        Self {
            capture: Arc::new(Mutex::new(display_capture)),
            state: Arc::new(AtomicU8::new(CaptureState::Stopped as u8)),
            pause_notify: Arc::new(Notify::new()),
            stop_notify: Arc::new(Notify::new()),
            opts_tx,
            opts_rx,
            force_idr: Arc::new(AtomicBool::new(false)),
        }
    }

    // ── Avvio cattura ───────────────────────────────────────────

    /// Avvia la cattura e ritorna il canale con i frame H.264 codificati.
    pub async fn start(&mut self) -> Result<mpsc::Receiver<EncodedFrame>, anyhow::Error> {
        // Use atomic compare_exchange for lock-free state transition
        let current = self.state.load(Ordering::Acquire);
        if current != CaptureState::Stopped as u8 {
            return Err(anyhow::anyhow!("Capturer already running"));
        }
        self.state
            .store(CaptureState::Playing as u8, Ordering::Release);

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

        // Increased channel capacity to prevent frame dropping under load
        // At 30fps: 256 frames = ~8 second buffer for network jitter
        let (tx, rx) = mpsc::channel::<EncodedFrame>(256);
        let (frame_tx, mut frame_rx) = mpsc::channel::<bytes::Bytes>(128);

        let capture = self.capture.clone();
        let pause_notify = self.pause_notify.clone();
        let stop_notify = self.stop_notify.clone();
        let state_ref = Arc::clone(&self.state);
        let opts_rx = self.opts_rx.clone();

        // Create encoder and capture its force_idr before moving it
        let encoder = FfmpegEncoder::new(enc_w, enc_h);
        self.force_idr = encoder.force_idr.clone();
        let force_idr = self.force_idr.clone();

        let mut sequence_number = 0u64;
        let start_time = std::time::Instant::now();
        let mut total_frames = 0u64;
        let mut dropped_frames = 0u64;
        let mut last_stats_log = std::time::Instant::now();

        tokio::spawn(async move {
            info!("=== CAPTURER: Spawn started ===");

            // Avvia la cattura interna (scrive frame codificati in frame_tx)
            {
                let mut cap = capture.lock().await;
                if let Err(e) = cap.start_capture(encoder, frame_tx, opts_rx).await {
                    error!("Capture start failed: {}", e);
                    return;
                }
            }

            info!("=== CAPTURER: Capture loop started, waiting for frames ===");

            loop {
                select! {
                    frame = frame_rx.recv() => {
                        let Some(raw) = frame else {
                            error!("CAPTURER: Frame channel closed (frame_rx returned None)!");
                            break;
                        };

                        // Log first frame
                        if total_frames == 0 {
                            info!("CAPTURER: First frame received! Size: {} bytes", raw.len());
                        }

                        let s = CaptureState::from_u8(state_ref.load(Ordering::Acquire));
                        match s {
                            CaptureState::Paused => {
                                pause_notify.notified().await;
                                continue;
                            }
                            CaptureState::Stopped => break,
                            CaptureState::Playing => {}
                        }

                        let timestamp_ms = start_time.elapsed().as_millis() as u64;
                        let frame_size = raw.len();
                        let encoded_frame = EncodedFrame {
                            data: Vec::from(raw),
                            sequence_number,
                            timestamp_ms,
                        };
                        sequence_number += 1;
                        total_frames += 1;

                        // Try to send frame, track drops
                        match tx.try_send(encoded_frame) {
                            Ok(_) => {
                                // Success - log stats periodically
                                if last_stats_log.elapsed().as_secs() >= 10 {
                                    let drop_rate = if total_frames > 0 {
                                        (dropped_frames as f64 / total_frames as f64) * 100.0
                                    } else {
                                        0.0
                                    };
                                    log::info!(
                                        "Encoder stats: {} frames sent, {} dropped ({:.1}%), last frame: {} bytes",
                                        total_frames - dropped_frames,
                                        dropped_frames,
                                        drop_rate,
                                        frame_size
                                    );
                                    last_stats_log = std::time::Instant::now();
                                }
                            }
                            Err(_) => {
                                dropped_frames += 1;
                                if dropped_frames % 30 == 1 {
                                    // Log every 30 drops to avoid spam
                                    log::warn!(
                                        "Frame {} dropped (channel full, {} total drops), requesting IDR",
                                        sequence_number - 1,
                                        dropped_frames
                                    );
                                }
                                force_idr.store(true, Ordering::Relaxed);
                            }
                        }
                    }

                    _ = stop_notify.notified() => {
                        info!("CAPTURER: Stop signal received");
                        break;
                    }
                }
            }

            error!(
                "=== CAPTURER: Loop exited! Total frames: {}, dropped: {} ===",
                total_frames, dropped_frames
            );

            // Cleanup
            let mut cap = capture.lock().await;
            if let Err(e) = cap.stop_capture().await {
                error!("Capture stop failed: {}", e);
            }
            info!("Capture cleanup completed");
        });

        Ok(rx)
    }

    // ── Controllo stato ─────────────────────────────────────────

    pub fn play(&self) {
        let current = self.state.load(Ordering::Acquire);
        if current == CaptureState::Paused as u8 {
            self.state
                .store(CaptureState::Playing as u8, Ordering::Release);
            self.opts_tx.send_modify(|o| o.paused = false);
            self.pause_notify.notify_waiters();
            info!("Capture resumed");
        }
    }

    pub fn pause(&self) {
        let current = self.state.load(Ordering::Acquire);
        if current == CaptureState::Playing as u8 {
            self.state
                .store(CaptureState::Paused as u8, Ordering::Release);
            self.opts_tx.send_modify(|o| o.paused = true);
            info!("Capture paused");
        }
    }

    pub fn stop(&self) {
        let current = self.state.load(Ordering::Acquire);
        if current != CaptureState::Stopped as u8 {
            self.state
                .store(CaptureState::Stopped as u8, Ordering::Release);
            self.stop_notify.notify_waiters();
            info!("Capture fully stopped");
        }
    }

    pub fn is_playing(&self) -> bool {
        self.state.load(Ordering::Acquire) == CaptureState::Playing as u8
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

    /// Imposta lo stato di pausa per il loop di cattura.
    pub fn set_paused(&self, paused: bool) {
        self.opts_tx.send_modify(|o| o.paused = paused);
        info!("Capture paused flag: {}", paused);
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
        if self.is_playing() {
            error!("Cannot change display while capture is running");
            return;
        }

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

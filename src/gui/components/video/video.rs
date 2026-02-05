use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::mpsc;
use log::error;

/// Internal state for the video renderer.
pub struct Internal {
    pub id: u64,

    pub width: i32,
    pub height: i32,
    pub framerate: f64,

    pub frame: Arc<Mutex<Vec<u8>>>,
    pub upload_frame: Arc<AtomicBool>,
    pub is_eos_flag: Arc<AtomicBool>,
    pub paused: bool,
    pub next_redraw: Instant,
    is_eos: bool
}

/// Video component: riceve frame H.264 (o raw RGBA) da un canale Tokio
/// e li espone come buffer per il rendering sulla GUI.
pub struct Video(pub RefCell<Internal>);

impl Video {
    pub fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);

        Video(RefCell::new(Internal {
            id,
            width: 0,
            height: 0,
            framerate: 0.0,
            frame: Arc::new(Mutex::new(Vec::new())),
            upload_frame: Arc::new(AtomicBool::new(false)),
            is_eos_flag: Arc::new(AtomicBool::new(false)),
            paused: false,
            next_redraw: Instant::now(),
            is_eos: false,
        }))
    }

    /// Collega uno stream video (canale Tokio) al componente.
    ///
    /// Spawna un task che legge i frame dal canale e li copia nel buffer
    /// condiviso, segnalando `upload_frame` per il rendering.
    pub fn set_stream(
        &mut self,
        mut rx: mpsc::Receiver<Vec<u8>>,
        width: i32,
        height: i32,
        fps: u32,
    ) {
        let frame_buf = vec![0u8; (width * height * 4) as usize];
        let frame = Arc::new(Mutex::new(frame_buf));
        let frame_ref = Arc::clone(&frame);

        let upload_frame = Arc::new(AtomicBool::new(false));
        let upload_frame_ref = Arc::clone(&upload_frame);

        // Aggiorna lo stato interno
        {
            let mut inner = self.0.borrow_mut();
            inner.width = width;
            inner.height = height;
            inner.framerate = fps as f64;
            inner.frame = frame;
            inner.upload_frame = upload_frame;
            inner.paused = false;
            inner.is_eos = false;
        }

        // Flag EOS condiviso col widget
        let is_eos = Arc::new(AtomicBool::new(false));
        let is_eos_ref = Arc::clone(&is_eos);
        self.0.borrow_mut().is_eos_flag = Arc::clone(&is_eos);

        // Task di lettura dal canale
        tokio::spawn(async move {
            while let Some(data) = rx.recv().await {
                match frame_ref.lock() {
                    Ok(mut buf) => {
                        let len = buf.len().min(data.len());
                        buf[..len].copy_from_slice(&data[..len]);
                        upload_frame_ref.store(true, Ordering::SeqCst);
                    }
                    Err(e) => {
                        error!("Frame lock poisoned: {}", e);
                        break;
                    }
                }
            }
            // Il canale si è chiuso → end of stream
            is_eos_ref.store(true, Ordering::SeqCst);
        });
    }

    /// Get the size/resolution of the video as `(width, height)`.
    #[inline(always)]
    pub fn size(&self) -> (i32, i32) {
        let inner = self.0.borrow();
        (inner.width, inner.height)
    }

    /// Get the framerate of the video as frames per second.
    #[inline(always)]
    pub fn framerate(&self) -> f64 {
        self.0.borrow().framerate
    }

    /// Set if the media is paused or not.
    pub fn set_paused(&mut self, paused: bool) {
        self.0.borrow_mut().paused = paused;
    }

    /// Get if the media is paused or not.
    #[inline(always)]
    pub fn paused(&self) -> bool {
        self.0.borrow().paused
    }

    /// Get if the stream ended (channel closed).
    #[inline(always)]
    pub fn eos(&self) -> bool {
        self.0.borrow().is_eos_flag.load(Ordering::SeqCst)
    }
}
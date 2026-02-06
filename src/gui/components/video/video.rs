use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::mpsc;
use log::error;
use crate::decoder::VideoFrame;

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
    is_eos: bool,

    /// Dynamic width/height updated by the reader task (for dynamic resolution)
    pub dyn_width: Option<Arc<std::sync::atomic::AtomicI32>>,
    pub dyn_height: Option<Arc<std::sync::atomic::AtomicI32>>,
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
            dyn_width: None,
            dyn_height: None,
        }))
    }

    /// Collega uno stream video (canale Tokio) al componente.
    ///
    /// Spawna un task che legge i frame dal canale e li copia nel buffer
    /// condiviso, segnalando `upload_frame` per il rendering.
    /// Dimensions are derived from the first received frame.
    pub fn set_stream(
        &mut self,
        mut rx: mpsc::Receiver<VideoFrame>,
        fps: u32,
    ) {
        let frame = Arc::new(Mutex::new(Vec::new()));
        let frame_ref = Arc::clone(&frame);

        let upload_frame = Arc::new(AtomicBool::new(false));
        let upload_frame_ref = Arc::clone(&upload_frame);

        // Shared width/height that the reader task will update from the first frame
        let width = Arc::new(std::sync::atomic::AtomicI32::new(0));
        let height = Arc::new(std::sync::atomic::AtomicI32::new(0));
        let width_ref = Arc::clone(&width);
        let height_ref = Arc::clone(&height);

        // Aggiorna lo stato interno
        {
            let mut inner = self.0.borrow_mut();
            inner.width = 0;
            inner.height = 0;
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

        // We need the Internal's width/height to be updated from the reader task.
        // Since Internal is behind RefCell (not Send), we use the atomic width/height
        // and update Internal's copies in the widget's draw/layout calls.
        // However, the simpler approach: store w/h atomics that layout reads.
        let inner_width = Arc::clone(&width);
        let inner_height = Arc::clone(&height);
        {
            let mut inner = self.0.borrow_mut();
            inner.dyn_width = Some(inner_width);
            inner.dyn_height = Some(inner_height);
        }

        // Task di lettura dal canale
        tokio::spawn(async move {
            while let Some(vf) = rx.recv().await {
                let cur_w = width_ref.load(Ordering::SeqCst);
                let cur_h = height_ref.load(Ordering::SeqCst);
                let new_w = vf.width as i32;
                let new_h = vf.height as i32;

                match frame_ref.lock() {
                    Ok(mut buf) => {
                        // Resize buffer if dimensions changed
                        if cur_w != new_w || cur_h != new_h {
                            let new_size = (new_w * new_h * 4) as usize;
                            buf.resize(new_size, 0);
                            width_ref.store(new_w, Ordering::SeqCst);
                            height_ref.store(new_h, Ordering::SeqCst);
                        }
                        let len = buf.len().min(vf.data.len());
                        buf[..len].copy_from_slice(&vf.data[..len]);
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
        if let (Some(w), Some(h)) = (&inner.dyn_width, &inner.dyn_height) {
            let dw = w.load(Ordering::SeqCst);
            let dh = h.load(Ordering::SeqCst);
            if dw > 0 && dh > 0 {
                return (dw, dh);
            }
        }
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
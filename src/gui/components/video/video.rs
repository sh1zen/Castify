use crate::decoder::VideoFrame;
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::mpsc;

/// Shared frame buffer for video frame sharing between reader and GUI.
/// Uses a simple mutex-protected buffer with a "dirty" flag.
#[derive(Debug)]
pub struct FrameBuffer {
    /// The frame data (YUV420p format)
    data: Vec<u8>,
    /// Frame dimensions
    width: i32,
    height: i32,
    /// Whether new data is available
    has_data: bool,
}

impl FrameBuffer {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            width: 0,
            height: 0,
            has_data: false,
        }
    }

    /// Write frame data to the buffer
    pub fn write(&mut self, data: &[u8], width: i32, height: i32) {
        let expected_size = (width * height * 3 / 2) as usize;
        if self.data.len() != expected_size {
            self.data.resize(expected_size, 0);
        }
        let len = self.data.len().min(data.len());
        self.data[..len].copy_from_slice(&data[..len]);
        self.width = width;
        self.height = height;
        self.has_data = true;
    }

    /// Read frame data from the buffer. Returns None if no data is available.
    pub fn read(&mut self) -> Option<(&[u8], i32, i32)> {
        if self.has_data && !self.data.is_empty() {
            Some((&self.data, self.width, self.height))
        } else {
            None
        }
    }
}

/// Internal state for the video renderer.
pub struct Internal {
    pub id: u64,

    pub width: i32,
    pub height: i32,
    pub framerate: f64,

    pub frame: Arc<Mutex<FrameBuffer>>,
    pub has_new_frame: Arc<AtomicBool>,
    pub is_eos_flag: Arc<AtomicBool>,
    pub paused: bool,
    pub next_redraw: Instant,
    is_eos: bool,

    /// Dynamic width/height updated by the reader task (for dynamic resolution)
    pub dyn_width: Option<Arc<AtomicI32>>,
    pub dyn_height: Option<Arc<AtomicI32>>,
}

/// Video component: riceve frame H.264 (o raw RGBA) da un canale Tokio
/// e li espone come buffer per il rendering sulla GUI.
pub struct Video(pub RefCell<Internal>);

impl Video {
    pub fn new() -> Self {
        static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);

        Video(RefCell::new(Internal {
            id,
            width: 0,
            height: 0,
            framerate: 0.0,
            frame: Arc::new(Mutex::new(FrameBuffer::new())),
            has_new_frame: Arc::new(AtomicBool::new(false)),
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
    pub fn set_stream(&mut self, mut rx: mpsc::Receiver<VideoFrame>, fps: u32) {
        let frame = Arc::new(Mutex::new(FrameBuffer::new()));
        let frame_ref = Arc::clone(&frame);

        let has_new_frame = Arc::new(AtomicBool::new(false));
        let has_new_frame_ref = Arc::clone(&has_new_frame);

        // Shared width/height that the reader task will update from the first frame
        let width = Arc::new(AtomicI32::new(0));
        let height = Arc::new(AtomicI32::new(0));
        let width_ref = Arc::clone(&width);
        let height_ref = Arc::clone(&height);

        // Aggiorna lo stato interno
        {
            let mut inner = self.0.borrow_mut();
            inner.width = 0;
            inner.height = 0;
            inner.framerate = fps as f64;
            inner.frame = frame;
            inner.has_new_frame = has_new_frame;
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

        // Task di lettura dal canale - optimized for real-time playback
        tokio::spawn(async move {
            let mut frame_count = 0u64;
            let mut skipped_count = 0u64;
            let mut last_stats = Instant::now();

            log::info!("Video reader task started, waiting for frames...");

            loop {
                // Use try_recv to check for pending frames without blocking
                match rx.try_recv() {
                    Ok(vf) => {
                        frame_count += 1;

                        // Check if there are more frames waiting - if so, skip to newest
                        // This prevents accumulating latency when rendering is slow
                        let mut latest_vf = vf;
                        let mut frames_skipped = 0u64;
                        while let Ok(newer_vf) = rx.try_recv() {
                            latest_vf = newer_vf;
                            frames_skipped += 1;
                        }

                        if frames_skipped > 0 {
                            skipped_count += frames_skipped;
                            if frames_skipped > 5 {
                                log::debug!(
                                    "Video reader: skipped {} old frames to catch up",
                                    frames_skipped
                                );
                            }
                        }

                        // Log stats every 5 seconds
                        if last_stats.elapsed().as_secs() >= 5 {
                            if skipped_count > 0 {
                                log::info!(
                                    "Video reader: {} frames processed, {} skipped ({:.1}%)",
                                    frame_count,
                                    skipped_count,
                                    (skipped_count as f64 / (frame_count + skipped_count) as f64)
                                        * 100.0
                                );
                            } else {
                                log::info!("Video reader: {} frames processed", frame_count);
                            }
                            last_stats = Instant::now();
                        }

                        let new_w = latest_vf.width as i32;
                        let new_h = latest_vf.height as i32;

                        // Write to frame buffer - use try_lock to avoid blocking
                        match frame_ref.try_lock() {
                            Ok(mut buffer) => {
                                buffer.write(&latest_vf.data, new_w, new_h);
                                log::debug!(
                                    "Video reader: wrote frame {}x{}, {} bytes",
                                    new_w,
                                    new_h,
                                    latest_vf.data.len()
                                );
                            }
                            Err(_) => {
                                log::debug!("Video reader: buffer locked, skipping frame");
                            }
                        }

                        // Signal that a new frame is available
                        has_new_frame_ref.store(true, Ordering::Release);
                        width_ref.store(new_w, Ordering::Release);
                        height_ref.store(new_h, Ordering::Release);
                    }
                    Err(mpsc::error::TryRecvError::Empty) => {
                        // No frame available - wait a short time
                        tokio::time::sleep(std::time::Duration::from_micros(500)).await;
                    }
                    Err(mpsc::error::TryRecvError::Disconnected) => {
                        // Channel closed
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

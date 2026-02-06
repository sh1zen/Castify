use std::sync::atomic::{AtomicU64, Ordering};

/// Lightweight pipeline profiling that accumulates per-stage timing
/// and logs a summary periodically.
pub struct PipelineStats {
    pub capture_us: AtomicU64,
    pub convert_us: AtomicU64,
    pub encode_us: AtomicU64,
    pub send_us: AtomicU64,
    pub frames_encoded: AtomicU64,
    pub frames_skipped: AtomicU64,
    pub frames_dropped: AtomicU64,
    pub current_fps: AtomicU64,
    encoder_name: String,
}

impl PipelineStats {
    pub fn new(encoder_name: String) -> Self {
        Self {
            capture_us: AtomicU64::new(0),
            convert_us: AtomicU64::new(0),
            encode_us: AtomicU64::new(0),
            send_us: AtomicU64::new(0),
            frames_encoded: AtomicU64::new(0),
            frames_skipped: AtomicU64::new(0),
            frames_dropped: AtomicU64::new(0),
            current_fps: AtomicU64::new(60),
            encoder_name,
        }
    }

    pub fn log_summary(&self) {
        let n = self.frames_encoded.load(Ordering::Relaxed).max(1);
        let skipped = self.frames_skipped.load(Ordering::Relaxed);
        let dropped = self.frames_dropped.load(Ordering::Relaxed);
        let fps = self.current_fps.load(Ordering::Relaxed);

        log::info!(
            "Pipeline [{}]: fps={} capture={:.1}ms encode={:.1}ms send={:.1}ms | encoded={} skipped={} dropped={}",
            self.encoder_name,
            fps,
            self.capture_us.load(Ordering::Relaxed) as f64 / n as f64 / 1000.0,
            self.encode_us.load(Ordering::Relaxed) as f64 / n as f64 / 1000.0,
            self.send_us.load(Ordering::Relaxed) as f64 / n as f64 / 1000.0,
            n,
            skipped,
            dropped,
        );

        // Reset counters for next interval
        self.capture_us.store(0, Ordering::Relaxed);
        self.convert_us.store(0, Ordering::Relaxed);
        self.encode_us.store(0, Ordering::Relaxed);
        self.send_us.store(0, Ordering::Relaxed);
        self.frames_encoded.store(0, Ordering::Relaxed);
        self.frames_skipped.store(0, Ordering::Relaxed);
        self.frames_dropped.store(0, Ordering::Relaxed);
    }
}

use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct Sos {
    closing: Arc<AtomicBool>,

}

impl Sos {
    pub fn new() -> Self {
        Self {
            closing: Arc::new(AtomicBool::new(false))
        }
    }

    pub fn terminate(&mut self) {
        self.closing.store(true, Ordering::Relaxed);
    }

    pub fn is_closing(&self) -> bool {
        self.closing.load(Ordering::Relaxed)
    }
}

static INSTANCE: Lazy<Arc<Mutex<Sos>>> = Lazy::new(|| Arc::new(Mutex::new(Sos::new())));

pub fn get_instance() -> Arc<Mutex<Sos>> {
    INSTANCE.clone()
}
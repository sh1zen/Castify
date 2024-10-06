use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Debug, Default)]
pub struct SignalOfStop {
    closing: Arc<AtomicBool>,
}

impl SignalOfStop {
    pub fn new() -> SignalOfStop {
        SignalOfStop{
            closing: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&mut self) {
        self.closing.store(true, Ordering::Relaxed);
    }

    pub fn cancelled(&self) -> bool {
        self.closing.load(Ordering::Relaxed)
    }
}
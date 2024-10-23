use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub struct Status {
    initial: usize,
    status: Arc<AtomicUsize>,
}

impl Status {
    pub fn new(initial: usize) -> Status {
        Status {
            initial,
            status: Arc::new(AtomicUsize::new(initial)),
        }
    }

    pub fn get(&self) -> usize {
        self.status.load(Ordering::SeqCst)
    }

    pub fn set(&self, value: usize) {
        self.status.store(value, Ordering::SeqCst)
    }

    pub fn next(&self) {
        self.status.fetch_add(1, Ordering::SeqCst);
    }

    pub fn prev(&self) {
        self.status.fetch_sub(1, Ordering::SeqCst);
    }

    pub fn reset(&self) {
        self.status.store(self.initial, Ordering::SeqCst);
    }
}

impl Clone for Status {
    fn clone(&self) -> Status {
        Status {
            initial: self.initial,
            status: Arc::clone(&self.status),
        }
    }
}
use std::sync::{Arc, Mutex, Condvar};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug)]
pub struct SignalOfStop {
    // Shared state between clones
    shared: Arc<SharedState>,
}

#[derive(Debug)]
struct SharedState {
    closing: AtomicBool,
    mutex: Mutex<()>,
    condvar: Condvar,
}

impl SignalOfStop {
    pub fn new() -> SignalOfStop {
        SignalOfStop {
            shared: Arc::new(SharedState {
                closing: AtomicBool::new(false),
                mutex: Mutex::new(()),
                condvar: Condvar::new(),
            }),
        }
    }

    pub fn cancel(&self) {
        // Set the 'closing' flag to true
        self.shared.closing.store(true, Ordering::Relaxed);

        // Notify all threads waiting on the condition variable
        let _guard = self.shared.mutex.lock().unwrap(); // Lock briefly to synchronize with waiting threads
        self.shared.condvar.notify_all();
    }

    pub fn cancelled(&self) -> bool {
        self.shared.closing.load(Ordering::Relaxed)
    }

    pub fn wait_cancellation(&self) {
        // Only lock the mutex while checking and waiting on the condition variable
        let mut guard = self.shared.mutex.lock().unwrap();

        while !self.cancelled() {
            guard = self.shared.condvar.wait(guard).unwrap(); // Wait releases the lock, then reacquires it when notified
        }
    }
}

// Implementing the Clone trait
impl Clone for SignalOfStop {
    fn clone(&self) -> SignalOfStop {
        SignalOfStop {
            shared: Arc::clone(&self.shared),
        }
    }
}

use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use tokio::sync::Notify;

#[derive(Debug, Default)]
pub struct SignalOfStop {
    internal: Arc<SharedState>,
}

#[derive(Debug, Default)]
struct SharedState {
    closing: AtomicBool,
    notify: Notify,
    mutex: Mutex<()>,
    condvar: Condvar,
}

impl SignalOfStop {
    pub fn new() -> SignalOfStop {
        SignalOfStop {
            internal: Arc::new(SharedState {
                closing: AtomicBool::new(false),
                notify: Notify::new(),
                mutex: Mutex::new(()),
                condvar: Condvar::new(),
            }),
        }
    }

    pub fn cancel(&self) {
        self.internal.closing.store(true, Ordering::Relaxed);

        self.internal.notify.notify_waiters();

        // thread safety - hold lock while notifying
        if self.internal.mutex.lock().is_ok() {
            // Notify all waiting threads that they should wake up and check the condition
            self.internal.condvar.notify_all();
        }
    }

    pub fn restore(&self) {
        if self.internal.mutex.lock().is_ok() {
            self.internal.closing.store(false, Ordering::Relaxed);
        }
    }

    pub fn cancelled(&self) -> bool {
        self.internal.closing.load(Ordering::Relaxed)
    }

    pub async fn wait(&self) -> bool {
        // Fast path: If already cancelled, return immediately.
        if self.cancelled() {
            return true;
        }

        // Otherwise, await notification of cancellation.
        self.internal.notify.notified().await;

        // After being notified, check if we were cancelled.
        self.cancelled()
    }

    pub fn wait_cancellation(&self) {
        // Only lock the mutex while checking and waiting on the condition variable
        let mut guard = self.internal.mutex.lock().unwrap();

        while !self.cancelled() {
            guard = self.internal.condvar.wait(guard).unwrap();
        }
    }

    pub fn spawn<F>(&self, fut: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let clone = self.clone();
        tokio::spawn(async move {
            let _ = clone.select(fut).await;
        });
    }

    pub async fn select<F, T>(&self, fut: F) -> Result<T, ()>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let clone = self.clone();
        tokio::select! {
            res = fut => {
                Ok(res)
            },
            _ = clone.wait() => {
                Err(())
            }
        }
    }
}

impl Clone for SignalOfStop {
    fn clone(&self) -> SignalOfStop {
        SignalOfStop {
            internal: Arc::clone(&self.internal),
        }
    }
}

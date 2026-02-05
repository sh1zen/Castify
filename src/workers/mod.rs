//! Background worker tasks
//!
//! This module contains long-running background tasks that handle
//! screen capture, streaming, receiving, and system integration.

pub mod caster;
pub mod key_listener;
pub mod receiver;
pub mod save_stream;
pub mod tray_icon;

/// Trait for workers that need graceful shutdown.
pub trait WorkerClose {
    /// Close and clean up worker resources.
    fn close(&mut self);
}

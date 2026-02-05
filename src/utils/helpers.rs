//! General helper utilities
//!
//! This module contains miscellaneous utility functions used across the codebase.

use iced::Point;
use tokio::sync::mpsc;

/// Opens a URL in the system's default browser.
///
/// This is a fire-and-forget operation that spawns a browser process
/// without blocking the GUI.
pub fn open_link(web_page: &String) {
    let url = web_page;
    // Intentionally fire-and-forget: browser opening doesn't need to be waited on
    // and we don't want to block the GUI while the browser launches
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer").arg(url).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}

/// Normalizes two points to ensure start coordinates are less than end coordinates.
///
/// Given two points representing a selection rectangle, this function returns
/// them ordered such that the first point has the minimum x and y coordinates.
pub fn evaluate_points(point_a: Point, point_b: Point) -> (Point, Point) {
    let (mut start, mut end) = (point_a, point_b);
    if point_a.x > point_b.x {
        (start.x, end.x) = (point_b.x, point_a.x)
    };
    if point_a.y > point_b.y {
        (start.y, end.y) = (point_b.y, point_a.y)
    };

    (start, end)
}

/// Converts a Result to an Option, discarding any error.
pub fn result_to_option<T, E>(result: Result<T, E>) -> Option<T> {
    result.ok()
}

// ── Channel Helpers ─────────────────────────────────────────────────────────

/// Result of a try_send operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendResult {
    /// Message was sent successfully.
    Sent,
    /// Channel is full, message was dropped.
    Full,
    /// Channel is closed, no more messages can be sent.
    Closed,
}

impl SendResult {
    /// Returns true if the message was sent.
    #[inline]
    pub fn is_sent(self) -> bool {
        matches!(self, SendResult::Sent)
    }

    /// Returns true if the channel is full.
    #[inline]
    pub fn is_full(self) -> bool {
        matches!(self, SendResult::Full)
    }

    /// Returns true if the channel is closed.
    #[inline]
    pub fn is_closed(self) -> bool {
        matches!(self, SendResult::Closed)
    }
}

impl From<mpsc::error::TrySendError<()>> for SendResult {
    #[inline]
    fn from(err: mpsc::error::TrySendError<()>) -> Self {
        match err {
            mpsc::error::TrySendError::Full(()) => SendResult::Full,
            mpsc::error::TrySendError::Closed(()) => SendResult::Closed,
        }
    }
}

/// Try to send a message to an mpsc channel, returning the result.
///
/// This is a convenience wrapper that converts the error types to a simple enum,
/// making it easier to handle the common cases without matching on the full error.
///
/// # Example
/// ```
/// let (tx, mut rx) = tokio::sync::mpsc::channel(1);
///
/// // Successful send
/// assert!(try_send(&tx, 42).is_sent());
///
/// // Full channel
/// try_send(&tx, 1);
/// assert!(try_send(&tx, 2).is_full()); // Channel has capacity 1
///
/// // Closed channel
/// drop(rx);
/// assert!(try_send(&tx, 3).is_closed());
/// ```
#[inline]
pub fn try_send<T>(tx: &mpsc::Sender<T>, value: T) -> SendResult {
    match tx.try_send(value) {
        Ok(()) => SendResult::Sent,
        Err(mpsc::error::TrySendError::Full(_)) => SendResult::Full,
        Err(mpsc::error::TrySendError::Closed(_)) => SendResult::Closed,
    }
}

/// Try to send a message, logging a warning if the channel is closed.
///
/// This is useful for fire-and-forget sends where you want to know if the
/// receiver was dropped but don't need to handle the full case.
#[inline]
pub fn try_send_log<T>(tx: &mpsc::Sender<T>, value: T, context: &str) -> SendResult {
    let result = try_send(tx, value);
    if result.is_closed() {
        log::warn!("{}: channel closed", context);
    }
    result
}

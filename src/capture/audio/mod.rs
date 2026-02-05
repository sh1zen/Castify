//! Audio capture module
//!
//! Provides audio capture functionality using platform-specific APIs:
//! - Windows: WASAPI loopback for system audio capture
//! - Other platforms: cpal for microphone capture

#[cfg(target_os = "windows")]
mod loopback;

#[cfg(target_os = "windows")]
pub use loopback::WasapiLoopbackCapture as AudioCapture;

#[cfg(not(target_os = "windows"))]
mod capture;

#[cfg(not(target_os = "windows"))]
pub use capture::AudioCapture;

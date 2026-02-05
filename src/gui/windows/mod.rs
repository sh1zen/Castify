//! Window definitions and management
//!
//! This module contains the main window, area selector, annotation window,
//! and the window management infrastructure.

pub mod annotation;
pub mod area_selector;
pub mod main;
mod manager;

pub use manager::{GuiWindow, WindowMessage, WindowType, Windows};

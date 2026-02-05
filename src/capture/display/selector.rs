//! Display selection trait
//!
//! This module provides the `DisplaySelector` trait for selecting
//! which display to capture.

use anyhow::Result;

/// Trait for selecting a display for screen capture.
///
/// Implementations provide functionality to enumerate available
/// displays and select one for capture.
pub trait DisplaySelector {
    type Display: ToString + Eq + Send;

    /// Returns a list of available displays.
    fn available_displays(&mut self) -> Result<Vec<Self::Display>>;

    /// Selects a display for capture.
    fn select_display(&mut self, display: &Self::Display) -> Result<()>;

    /// Returns the currently selected display, if any.
    fn selected_display(&self) -> Result<Option<Self::Display>>;
}

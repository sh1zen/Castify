//! GUI module for the Castify application
//!
//! This module contains all GUI-related components including windows,
//! widgets, pages, styling, and the application runner.

mod app;
pub mod common;
mod components;
mod macros;
mod pages;
mod popup;
mod runner;
mod style;
mod widget;
mod windows;

pub use runner::run;

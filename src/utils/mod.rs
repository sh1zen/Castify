//! Utility modules and helpers
//!
//! This module provides various utility functions and data structures
//! used throughout the application.

pub mod bimap;
pub mod flags;
mod helpers;
pub mod ipc;
pub mod monitors;
pub mod net;
pub mod path;
pub mod perf;
pub mod sos;
pub mod status;
pub mod string;

pub use helpers::{
    SendResult, evaluate_points, open_link, result_to_option, try_send, try_send_log,
};

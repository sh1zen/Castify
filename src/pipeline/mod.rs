//! Pipeline abstraction layer for Castify
//!
//! This module provides a unified architecture for media processing pipelines,
//! separating concerns between:
//! - Control/Coordination: State machines and lifecycle management
//! - Data Transport: Channels and backpressure handling
//! - Media Processing: Capture, encode, decode, display stages
//! - Persistence: Recording to disk
//!
//! # Architecture
//!
//! The pipeline is organized into stages that communicate via channels:
//! - Each stage runs in its own async task
//! - Stages implement the `PipelineStage` trait
//! - Coordinators chain stages together and manage lifecycle
//! - MediaClock provides timestamp correlation for A/V sync
//! - Health monitoring tracks metrics and enables recovery

pub mod clock;
pub mod health;
pub mod receiver;
pub mod sender;
pub mod stage;
pub mod state;
pub mod types;

pub use clock::MediaClock;
pub use health::{HealthMonitor, PipelineHealth};
pub use stage::{PipelineCoordinator, PipelineStage};
pub use state::PipelineState;
pub use types::{MediaFrame, MediaKind, Timestamp};

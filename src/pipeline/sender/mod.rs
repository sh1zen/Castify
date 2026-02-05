//! Sender pipeline components
//!
//! This module contains the sender-side pipeline stages:
//! - CaptureStage: Screen/audio capture
//! - EncodeStage: H.264/Opus encoding
//! - TransmitStage: WebRTC transmission
//!
//! The sender pipeline flow:
//! ```text
//! Capture → Encode → Transmit → Network
//! ```

pub mod capture_stage;
pub mod coordinator;
pub mod encode_stage;
pub mod transmit_stage;

pub use capture_stage::CaptureStage;
pub use coordinator::SenderCoordinator;
pub use encode_stage::EncodeStage;
pub use transmit_stage::TransmitStage;

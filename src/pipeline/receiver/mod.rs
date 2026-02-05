//! Receiver pipeline components
//!
//! This module contains the receiver-side pipeline stages:
//! - ReceiveStage: RTP packet reception
//! - ReorderStage: Packet reordering + jitter buffer
//! - DecodeStage: H.264/Opus decoding
//! - SyncStage: Audio-video synchronization
//!
//! The receiver pipeline flow:
//! ```text
//! Network → Receive → Reorder → Decode → Sync → Display/Audio Output
//! ```

pub mod coordinator;
pub mod decode_stage;
pub mod receive_stage;
pub mod reorder_stage;
pub mod sync_stage;

pub use coordinator::ReceiverCoordinator;
pub use decode_stage::{DecodeStage, TimedVideoFrame};
pub use receive_stage::ReceiveStage;
pub use reorder_stage::{JitterBuffer, ReorderConfig, ReorderStage, RtpPacket};
pub use sync_stage::{AudioPlaybackTracker, SyncConfig, SyncStage};

//! Video playback components for the receiver UI

mod pipeline;
#[allow(clippy::module_inception)]
mod video;
mod video_player;

pub use video::Video;
pub use video_player::VideoPlayer;

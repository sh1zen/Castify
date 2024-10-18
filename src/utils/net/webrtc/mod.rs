mod werbrtc_server;
mod webrtc_client;
mod webrtc_common;

use std::future::Future;
pub use webrtc_client::WebRTCClient;
pub use werbrtc_server::WebRTCServer;

pub trait ManualSdp: Send + Sync {
    fn get_sdp(&self) -> String;

    fn set_remote_sdp(&mut self, sdp: String) -> bool;
}
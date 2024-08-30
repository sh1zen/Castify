use crate::gui::types::messages::Message;
use crate::utils::net::find_caster;
use std::net::SocketAddr;
use webrtc::rtp::packet::Packet;

pub async fn receiver(mut socket_addr: Option<SocketAddr>, tx: tokio::sync::mpsc::Sender<Packet>) -> Message {
    if socket_addr.is_none() {
        socket_addr = find_caster();
    }

    if let Some(socket_addr) = socket_addr {
        println!("Connecting to caster at {:?}", socket_addr);

        let addr: &str = &*format!("ws://{}", &(socket_addr.to_string()));

        let tt = crate::utils::net::WebRTCClient::new(addr).await;
        tt.receive_video(tx).await;
    } else {
        return Message::ConnectionError;
    }

    Message::Ignore
}
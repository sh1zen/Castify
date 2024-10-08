use crate::utils::net::common::find_caster;
use std::net::SocketAddr;
use webrtc::rtp::packet::Packet;
use crate::utils::sos::SignalOfStop;

pub async fn receiver(mut socket_addr: Option<SocketAddr>, tx: tokio::sync::mpsc::Sender<Packet>, sos: SignalOfStop) -> bool {
    if socket_addr.is_none() {
        socket_addr = find_caster();
    }

    let mut status = false;

    if let Some(socket_addr) = socket_addr {
        println!("Connecting to caster at {:?}", socket_addr);

        let addr: &str = &*format!("ws://{}", &(socket_addr.to_string()));

        if let Ok(tt) = crate::utils::net::webrtc::WebRTCClient::new(addr, sos).await {
            tt.receive_video(tx).await;
            status = true;
        }
    }
    status
}
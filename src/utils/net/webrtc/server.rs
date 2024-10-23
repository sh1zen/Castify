use crate::assets::CAST_SERVICE_PORT;
use crate::utils::net::webrtc::caster::WebRTCCaster;
use crate::utils::net::webrtc::manual::{SDPICEExchange, SDPICEExchangeWRTC};
use crate::utils::net::webrtc::peer::WRTCPeer;
use crate::utils::sos::SignalOfStop;
use async_trait::async_trait;
use async_tungstenite::tokio::accept_async;
use std::sync::Arc;
use tokio::net::TcpListener;

pub struct WebRTCServer {
    sos: SignalOfStop,
    caster: Arc<WebRTCCaster>,
}

impl WebRTCServer {
    pub fn new() -> Arc<WebRTCServer> {
        let sos = SignalOfStop::new();

        let server = WebRTCServer {
            sos: sos.clone(),
            caster: Arc::new(WebRTCCaster::new()),
        };

        Arc::new(server)
    }

    pub fn run(self: Arc<Self>) {
        let self_clone = Arc::clone(&self);

        self.sos.spawn(async move {
            if let Ok(listener) = TcpListener::bind(format!("0.0.0.0:{}", CAST_SERVICE_PORT).to_string()).await {
                println!("Server listener on: {:?}", listener);

                while let Ok((stream, _)) = listener.accept().await {
                    println!("Incoming connection: {:?}", stream);
                    let self_clone2 = Arc::clone(&self_clone);
                    // launch peer related operations
                    self_clone.sos.spawn(async move {
                        if let Ok(ws_stream) = accept_async(stream).await {
                            if let Ok(peer) = WRTCPeer::new().await {
                                self_clone2.caster.push(Arc::clone(&peer)).await;
                                if let Err(e) = Arc::clone(&peer).negotiate(ws_stream, true).await {
                                    peer.disconnect().await;
                                    eprintln!("Error handling signaling: {}", e);
                                }
                            }
                        }
                    });
                }
            }
        });
    }

    pub fn get_handler(&self) -> Arc<WebRTCCaster> {
        Arc::clone(&self.caster)
    }

    pub fn close(&self) {
        self.caster.close();
        self.sos.cancel();
    }
}


#[async_trait]
impl SDPICEExchangeWRTC for WebRTCServer {
    async fn get_sdp(&self) -> String {
        let peer = self.get_handler().get_manual_connection().await;

        let offer = peer.create_offer(true).await.unwrap_or_default();

        SDPICEExchange::new_with_spd(offer).pack().unwrap_or_default()
    }

    async fn set_remote_sdp(&self, remote_sdp: String) -> bool {
        let Ok(exchanger) = SDPICEExchange::unpack(remote_sdp) else {
            return false;
        };

        let peer = self.get_handler().get_manual_connection().await;

        for ice in exchanger.get_ice_candidates() {
            let _ = peer.get_connection().add_ice_candidate(ice).await;
        }

        let mut res = peer.set_remote_sdp(
            exchanger.get_sdp()
        ).await.is_ok();

        if res {
            res = self.get_handler().finalize_manual().await;
        }

        res
    }
}
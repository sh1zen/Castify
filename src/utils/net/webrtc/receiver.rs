use crate::utils::net::webrtc::manual::{SDPICEExchange, SDPICEExchangeWRTC};
use crate::utils::net::webrtc::peer::WRTCPeer;
use crate::utils::sos::SignalOfStop;
use async_trait::async_trait;
use async_tungstenite::tokio::{connect_async, ConnectStream};
use async_tungstenite::tungstenite::handshake::client::Response;
use async_tungstenite::tungstenite::Error;
use async_tungstenite::WebSocketStream;
use castbox::Arw;
use std::sync::Arc;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::Mutex;
use webrtc::rtp::packet::Packet;
use webrtc::rtp_transceiver::rtp_receiver::RTCRtpReceiver;
use webrtc::rtp_transceiver::RTCRtpTransceiver;
use webrtc::track::track_remote::TrackRemote;

pub struct WebRTCReceiver {
    sos: SignalOfStop,
    peer: Arw<Option<Arc<WRTCPeer>>>,
    manual_handler: Arw<Option<SDPICEExchange>>,
}

impl WebRTCReceiver {
    pub fn new() -> WebRTCReceiver {
        WebRTCReceiver {
            sos: SignalOfStop::new(),
            peer: Arw::new(None),
            manual_handler: Arw::new(None),
        }
    }

    async fn get_lazy_peer(&self) -> Arc<WRTCPeer> {
        if self.peer.as_ref().is_none() {
            self.peer.as_mut().replace(WRTCPeer::new().await.unwrap());
        }
        self.peer.as_ref().as_ref().unwrap().clone()
    }

    pub async fn connect(&self, ws_server_url: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut conn: Result<(WebSocketStream<ConnectStream>, Response), Error> = Err(Error::ConnectionClosed.into());

        while conn.is_err() {
            if self.sos.cancelled() {
                return Err(Error::ConnectionClosed.into());
            }
            let ws_c = String::from(ws_server_url);
            conn = self.sos.select(async move { connect_async(&*ws_c).await }).await.unwrap_or(Err(Error::ConnectionClosed));
        }

        // Connect to the signaling server
        let (ws_stream, _) = conn.unwrap();

        let peer = self.get_lazy_peer().await;

        self.sos.spawn(async move {
            let _ = peer.negotiate(ws_stream, false).await;
        });

        Ok(())
    }

    pub async fn receive_video(&self, tx: tokio::sync::mpsc::Sender<Packet>) {
        let tx = Arc::new(Mutex::new(tx));
        let sos = self.sos.clone();
        // Set up the event handler for incoming tracks
        self.get_lazy_peer().await.get_connection().on_track(Box::new(move |track: Arc<TrackRemote>, _receiver: Arc<RTCRtpReceiver>, _transceiver: Arc<RTCRtpTransceiver>| {
            // Send a PLI on an interval so that the publisher is pushing a keyframe every rtcpPLIInterval
            //let media_ssrc = track.ssrc();
            //let codec = track.codec();
            let sos = sos.clone();

            Box::pin({
                let tx = Arc::clone(&tx);
                async move {
                    sos.spawn(async move {
                        while let Ok((packet, _)) = track.read_rtp().await {
                            match tx.lock().await.send(packet).await {
                                Err(SendError(e)) => {
                                    println!("Error channel packet {}", e);
                                    break;
                                }
                                _ => {}
                            }
                        }
                    });
                }
            })
        }));
    }

    pub async fn is_connected(&self) -> bool {
        self.get_lazy_peer().await.is_online()
    }

    pub async fn close(&self) {
        self.sos.cancel();
        self.get_lazy_peer().await.disconnect().await;
    }
}

#[async_trait]
impl SDPICEExchangeWRTC for WebRTCReceiver {
    async fn get_sdp(&self) -> String {
        if self.manual_handler.as_ref().is_none() {
            return String::from("Wrong manual SDP negotiation!");
        }

        self.manual_handler.as_ref().as_ref().unwrap().pack().unwrap()
    }

    async fn set_remote_sdp(&self, remote_sdp: String) -> bool {
        let Ok(exchanger_offer) = SDPICEExchange::unpack(remote_sdp) else {
            return false;
        };

        let peer = self.get_lazy_peer().await;

        self.manual_handler.as_mut().replace(SDPICEExchange::new());

        let exchanger_clone = Arw::clone(&self.manual_handler);
        peer.get_connection().on_ice_candidate(Box::new(move |candidate| {
            let exchanger_clone = Arw::clone(&exchanger_clone);
            Box::pin(async move {
                if let Some(candidate) = candidate {
                    exchanger_clone.as_mut().as_mut().unwrap().add_ice_candidate(candidate);
                }
            })
        }));

        let res = peer.create_answer(
            exchanger_offer.get_sdp(),
            true,
        ).await.is_ok();

        if res {
            self.manual_handler.as_mut().as_mut().unwrap().set_sdp(peer.get_connection().local_description().await.unwrap());
        }

        res
    }
}
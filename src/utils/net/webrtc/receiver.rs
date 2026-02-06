use crate::utils::net::webrtc::manual::{SDPICEExchange, SDPICEExchangeWRTC};
use crate::utils::net::webrtc::peer::WRTCPeer;
use crate::utils::sos::SignalOfStop;
use async_trait::async_trait;
use async_tungstenite::tokio::{connect_async, ConnectStream};
use async_tungstenite::tungstenite::handshake::client::Response;
use async_tungstenite::tungstenite::Error;
use async_tungstenite::WebSocketStream;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use castbox::Arw;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
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
            // Receiver doesn't encode, so force_idr is unused
            let dummy_idr = Arc::new(AtomicBool::new(false));
            self.peer.as_mut().replace(WRTCPeer::new(dummy_idr).await.unwrap());
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

        let (ws_stream, _) = conn.unwrap();
        let peer = self.get_lazy_peer().await;

        self.sos.spawn(async move {
            let _ = peer.negotiate(ws_stream, false).await;
        });

        Ok(())
    }

    pub async fn receive_video(&self, video_tx: Sender<(Vec<u8>, bool)>, audio_tx: Sender<Vec<u8>>) {
        let video_tx = Arc::new(Mutex::new(video_tx));
        let audio_tx = Arc::new(Mutex::new(audio_tx));
        let sos = self.sos.clone();

        self.get_lazy_peer().await.get_connection().on_track(Box::new(move |track: Arc<TrackRemote>, _receiver: Arc<RTCRtpReceiver>, _transceiver: Arc<RTCRtpTransceiver>| {
            let sos = sos.clone();
            let mime_type = track.codec().capability.mime_type.clone();

            if mime_type.to_lowercase().contains("audio") {
                // Audio track
                Box::pin({
                    let audio_tx = Arc::clone(&audio_tx);
                    async move {
                        sos.spawn(async move {
                            while let Ok((packet, _)) = track.read_rtp().await {
                                let payload = packet.payload.to_vec();
                                if payload.is_empty() {
                                    continue;
                                }
                                if let Err(e) = audio_tx.lock().await.send(payload).await {
                                    log::error!("Audio channel closed: {}", e);
                                    break;
                                }
                            }
                        });
                    }
                })
            } else {
                // Video track
                Box::pin({
                    let video_tx = Arc::clone(&video_tx);
                    async move {
                        sos.spawn(async move {
                            while let Ok((packet, _)) = track.read_rtp().await {
                                let payload = packet.payload.to_vec();
                                if payload.is_empty() {
                                    continue;
                                }

                                let marker = packet.header.marker;
                                if let Err(e) = video_tx.lock().await.send((payload, marker)).await {
                                    log::error!("Video frame channel closed: {}", e);
                                    break;
                                }
                            }
                        });
                    }
                })
            }
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
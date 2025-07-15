use crate::utils::net::webrtc::common::{create_peer_connection, create_video_track, create_webrtc_api, SignalMessage};
use crate::utils::sos::SignalOfStop;
use async_tungstenite::tokio::ConnectStream;
use async_tungstenite::tungstenite::{Message, Utf8Bytes};
use async_tungstenite::WebSocketStream;
use futures_util::{StreamExt};
use iced::futures::executor::block_on;
use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::sdp_type::RTCSdpType;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtp_transceiver::rtp_codec::RTPCodecType;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;
use webrtc::track::track_local::TrackLocal;

static WRTC_PEER_UUID: Lazy<AtomicU32> = Lazy::new(|| AtomicU32::new(0));

pub struct WRTCPeer {
    connection: Arc<RTCPeerConnection>,
    pub video_track: Arc<TrackLocalStaticSample>,
    online: AtomicBool,
    id: u32,
    sos: SignalOfStop,
}

impl WRTCPeer {
    pub async fn new() -> Result<Arc<WRTCPeer>, Box<dyn std::error::Error + Send + Sync>> {
        let sos = SignalOfStop::new();
        let video_track = create_video_track();

        let connection = create_peer_connection(&create_webrtc_api()).await.map_err(|e| format!("WebRTCServer Error: {:?}", e))?;

        connection.add_transceiver_from_kind(RTPCodecType::Video, None).await.map_err(|e| format!("WebRTCServer Error: {:?}", e))?;

        if let Ok(rtp_sender) = connection.add_track(Arc::clone(&video_track) as Arc<dyn TrackLocal + Send + Sync>).await {
            // Read incoming RTCP packets
            sos.spawn(async move {
                let mut rtcp_buf = vec![0u8; 1500];
                while let Ok((_x, _)) = rtp_sender.read(&mut rtcp_buf).await {
                    //println!("info:::: {:?}", x);
                }
            });
        }

        let conn_clone = Arc::clone(&connection);

        let peer = Arc::new(
            WRTCPeer {
                connection,
                video_track,
                online: AtomicBool::new(true),
                id: WRTC_PEER_UUID.fetch_add(1, Ordering::Relaxed),
                sos,
            }
        );

        // This will notify you when the peer has connected/disconnected
        let peer_clone = Arc::clone(&peer);
        conn_clone.on_peer_connection_state_change(Box::new(move |connection_state: RTCPeerConnectionState| {
            println!("Peer Connection State has changed: {connection_state}");
            match connection_state {
                RTCPeerConnectionState::Disconnected | RTCPeerConnectionState::Failed | RTCPeerConnectionState::Closed => {
                    Arc::clone(&peer_clone).lazy_disconnect();
                }
                _ => {}
            }
            Box::pin(async {})
        }));

        println!("New peer connection {:?}", conn_clone);

        Ok(peer)
    }

    pub async fn disconnect(&self) {
        println!("Peer {} has disconnected", self.id);
        self.online.store(false, Ordering::Relaxed);
        self.sos.cancelled();
        let _ = self.connection.close().await;

        for transceiver in self.connection.get_transceivers().await.iter() {
            let _ = transceiver.stop().await;
        }
    }

    pub fn lazy_disconnect(self: Arc<Self>) {
        let self_clone = Arc::clone(&self);
        tokio::spawn(async move {
            self_clone.disconnect().await
        });
    }

    pub fn get_connection(&self) -> Arc<RTCPeerConnection> {
        Arc::clone(&self.connection)
    }

    pub fn is_online(&self) -> bool {
        // todo make online only when online
        self.online.load(Ordering::Relaxed)
    }

    pub async fn wait_ice(&self) {
        // Block until ICE Gathering is complete, disabling trickle ICE
        // we do this because we only can exchange one signaling message
        // in a production application you should exchange ICE Candidates via OnICECandidate
        self.connection.gathering_complete_promise().await.recv().await;
    }

    pub async fn create_offer(&self, wait: bool) -> Result<RTCSessionDescription, Box<dyn std::error::Error + Send + Sync>> {
        let offer = self.connection.create_offer(None).await?;
        self.connection.set_local_description(offer.clone()).await?;
        if wait {
            self.wait_ice().await;
        }
        Ok(offer)
    }

    pub async fn create_answer(&self, offer: RTCSessionDescription, wait: bool) -> Result<RTCSessionDescription, Box<dyn std::error::Error + Send + Sync>> {
        self.set_remote_sdp(offer).await?;
        let answer = self.connection.create_answer(None).await?;
        self.connection.set_local_description(answer.clone()).await?;
        if wait {
            self.wait_ice().await;
        }
        Ok(answer)
    }

    pub async fn set_remote_sdp(&self, sdp: RTCSessionDescription) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.connection.set_remote_description(sdp).await?;
        Ok(())
    }

    pub async fn negotiate(self: Arc<Self>, ws_stream: WebSocketStream<ConnectStream>, init_offer: bool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (ws_sender, mut ws_receiver) = ws_stream.split();
        let ws_sender = Arc::new(Mutex::new(ws_sender));

        if init_offer {
            let offer = self.create_offer(true).await?;
            // Start handling signaling
            // Create and send the initial offer to the server
            let offer_message = SignalMessage {
                sdp: Some(offer),
                candidate: None,
            };
            ws_sender.lock().await.send(Message::Text(Utf8Bytes::from(serde_json::to_string(&offer_message)?))).await?;
        } else {
            let peer_clone = Arc::clone(&self);
            let ws_stream_clone = Arc::clone(&ws_sender);
            self.connection.on_ice_candidate(Box::new(move |candidate| {
                if let Some(candidate) = candidate {
                    Box::pin({
                        let peer_clone = peer_clone.clone();
                        let ws_sender_clone = ws_stream_clone.clone();
                        async move {
                            let candidate_str = serde_json::to_string(&SignalMessage {
                                sdp: peer_clone.get_connection().local_description().await,
                                candidate: Some(candidate.to_json().unwrap_or_default()),
                            }).unwrap_or_default();

                            if ws_sender_clone.lock().await.send(Message::Text(Utf8Bytes::from(candidate_str))).await.is_err() {
                                eprintln!("Failed to send ICE candidate to client");
                                peer_clone.disconnect().await;
                            }
                        }
                    })
                } else {
                    Box::pin(async {})
                }
            }));
        }

        // Handle incoming signaling messages
        while let Some(Ok(msg)) = ws_receiver.next().await {
            if let Ok(signal) = serde_json::from_str::<SignalMessage>(&msg.to_string()) {
                if let Some(sdp) = signal.sdp {
                    match sdp.sdp_type {
                        RTCSdpType::Offer => {
                            let Ok(answer) = self.create_answer(sdp, true).await else {
                                continue;
                            };
                            ws_sender.lock().await.send(
                                Message::Text(
                                    Utf8Bytes::from(serde_json::to_string(&SignalMessage {
                                        sdp: Some(answer),
                                        candidate: None,
                                    }).unwrap_or_default())
                                )
                            ).await?;
                        }
                        RTCSdpType::Answer | RTCSdpType::Pranswer => {
                            let _ = self.connection.set_remote_description(sdp).await;
                        }
                        _ => {}
                    }
                }
                if let Some(candidate_sdp) = signal.candidate {
                    let _ = self.connection.add_ice_candidate(candidate_sdp).await;
                }
            }
        }

        Ok(())
    }
}

impl Drop for WRTCPeer {
    fn drop(&mut self) {
        block_on(async move {
            self.disconnect().await;
        })
    }
}
use crate::utils::net::webrtc::webrtc_common::{create_peer_connection, create_video_track, create_webrtc_api, SignalMessage};
use crate::utils::net::webrtc::ManualSdp;
use crate::utils::sos::SignalOfStop;
use async_tungstenite::tokio::{connect_async, ConnectStream};
use async_tungstenite::tungstenite::{Error, Message};
use futures_util::{SinkExt, StreamExt, TryFutureExt};
use std::sync::Arc;
use async_tungstenite::tungstenite::handshake::client::Response;
use async_tungstenite::WebSocketStream;
use serde::de::Unexpected::Str;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::Mutex;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtp::packet::Packet;
use webrtc::rtp_transceiver::rtp_codec::RTPCodecType;
use webrtc::rtp_transceiver::rtp_receiver::RTCRtpReceiver;
use webrtc::rtp_transceiver::RTCRtpTransceiver;
use webrtc::track::track_remote::TrackRemote;

pub struct WebRTCClient {
    sos: SignalOfStop,
    offer: Option<RTCSessionDescription>,
    connection: Option<Arc<RTCPeerConnection>>,
}

impl WebRTCClient {
    pub fn new() -> WebRTCClient {
        WebRTCClient {
            sos: SignalOfStop::new(),
            offer: None,
            connection: None,
        }
    }

    pub async fn connect(&mut self, ws_server_url: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
        // allow to be shared over treads
        let ws_stream = Arc::new(Mutex::new(ws_stream));

        let connection = self.create_connection().await?;

        // Set up ICE candidate handling
        let ws_stream_clone = Arc::clone(&ws_stream);
        connection.on_ice_candidate(Box::new(move |candidate| {
            Box::pin({
                let ws_stream_clone = ws_stream_clone.clone();
                async move {
                    if let Some(candidate) = candidate {
                        let candidate_json = serde_json::to_string(&SignalMessage {
                            sdp: None, // maybe resend offer
                            candidate: Some(candidate.to_json().unwrap()),
                        }).unwrap_or_default();

                        if ws_stream_clone.lock().await.send(Message::Text(candidate_json)).await.is_err() {
                            eprintln!("Failed to send ICE candidate to server");
                        }
                    }
                }
            })
        }));

        // Start handling signaling
        // Create and send the initial offer to the server
        let offer_message = SignalMessage {
            sdp: connection.local_description().await,
            candidate: None,
        };

        ws_stream.lock().await.send(Message::Text(serde_json::to_string(&offer_message).unwrap_or_default())).await.map_err(|e| format!("WebRTCClient Error: {:?}", e))?;

        // Handle incoming signaling messages
        let connection_clone = Arc::clone(&connection);
        self.sos.spawn(async move {
            while let Some(Ok(msg)) = ws_stream.lock().await.next().await {
                if let Ok(signal) = serde_json::from_str::<SignalMessage>(&msg.to_string()) {
                    if let Some(sdp) = signal.sdp {
                        connection_clone.set_remote_description(sdp).await.unwrap_or_default();
                    }
                    if let Some(candidate_sdp) = signal.candidate {
                        connection_clone.add_ice_candidate(candidate_sdp).await.unwrap_or_default();
                    }
                }
            }
        });

        self.connection = Some(connection);

        Ok(())
    }

    async fn create_connection(&self) -> Result<Arc<RTCPeerConnection>, Box<dyn std::error::Error + Send + Sync>> {
        let connection = create_peer_connection(&create_webrtc_api()).await.map_err(|e| format!("WebRTCClient Error: {:?}", e))?;

        connection.add_transceiver_from_kind(RTPCodecType::Video, None).await.map_err(|e| format!("WebRTCClient Error: {:?}", e))?;

        if let Ok(rtp_sender) = connection.add_track(create_video_track()).await {
            self.sos.spawn(async move {
                let mut rtcp_buf = vec![0u8; 1500];
                while let Ok((x, _)) = rtp_sender.read(&mut rtcp_buf).await {
                    println!("info:::: {:?}", x);
                }
            });
        }

        // Set the handler for Peer connection state
        // This will notify you when the peer has connected/disconnected
        connection.on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
            println!("Peer Connection State has changed: {s}");
            Box::pin(async {})
        }));

        let offer = connection.create_offer(None).await.map_err(|e| format!("WebRTCClient Error: {:?}", e))?;

        connection.set_local_description(offer.clone()).await.map_err(|e| format!("WebRTCClient Error: {:?}", e))?;

        Ok(connection)
    }

    pub async fn receive_video(&self, tx: tokio::sync::mpsc::Sender<Packet>) {
        if let Some(connection) = self.connection.as_ref() {
            let tx = Arc::new(Mutex::new(tx));
            let sos = self.sos.clone();
            // Set up the event handler for incoming tracks
            connection.on_track(Box::new(move |track: Arc<TrackRemote>, _receiver: Arc<RTCRtpReceiver>, _transceiver: Arc<RTCRtpTransceiver>| {
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
    }

    pub fn is_connected(&self) -> bool {
        self.connection.is_some()
    }

    pub async fn close(&mut self) {
        self.sos.cancel();
        if let Some(connection) = self.connection.take() {
            let senders = connection.get_senders();
            for sender in senders.await.iter() {
                sender.stop().await.unwrap();
            }
            connection.close().await.unwrap_or_default();
        }
    }

    pub(crate) async fn get_sdp(&mut self) -> String {
        //self.sos.cancelled();
        // Return a boxed future to match the return type in both branches
        if let Some(rtc_sd) = self.offer.clone() {
            // Synchronous value wrapped in an async block to match the return type
            rtc_sd.sdp.to_string()
        } else {
            // Asynchronous branch
            if let Ok(connection) = self.create_connection().await {
                let sdp = if let Some(sdp) = connection.local_description().await {
                    sdp.sdp
                } else {
                    String::new()
                };
                self.connection = Some(connection);
                sdp
            } else {
                String::new()
            }
        }
    }

    fn set_remote_sdp(&mut self, sdp: String) -> bool {
        let rtc_sd = RTCSessionDescription::answer(sdp);
        false
    }
}

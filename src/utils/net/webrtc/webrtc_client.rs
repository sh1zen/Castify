use crate::utils::net::webrtc::webrtc_common::{create_peer_connection, create_video_track, create_webrtc_api, SignalMessage};
use crate::utils::sos::SignalOfStop;
use crate::workers;
use async_tungstenite::tokio::connect_async;
use async_tungstenite::tungstenite::{Error, Message};
use async_tungstenite::WebSocketStream;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::Mutex;
use webrtc::ice_transport::ice_connection_state::RTCIceConnectionState;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtp::packet::Packet;
use webrtc::rtp_transceiver::rtp_codec::RTPCodecType;
use webrtc::rtp_transceiver::rtp_receiver::RTCRtpReceiver;
use webrtc::rtp_transceiver::RTCRtpTransceiver;
use webrtc::track::track_remote::TrackRemote;

#[derive(Clone)]
pub struct WebRTCClient {
    connection: Arc<RTCPeerConnection>,
    ws_stream: Arc<Mutex<WebSocketStream<async_tungstenite::tokio::ConnectStream>>>,
    local_sos: SignalOfStop,
}

impl WebRTCClient {
    pub async fn new(signaling_server_url: &str, sos: SignalOfStop) -> Result<Arc<WebRTCClient>, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = Err(Error::ConnectionClosed.into());

        while conn.is_err() {
            if sos.cancelled() {
                return Err(Error::ConnectionClosed.into());
            }
            conn = connect_async(signaling_server_url).await;
        }

        // Connect to the signaling server
        let (ws_stream, _) = conn.map_err(|e| format!("WebRTCClient Error: {:?}", e))?;

        // Create the WebRTC API
        let api = create_webrtc_api();

        let client = Arc::new(WebRTCClient {
            connection: create_peer_connection(&api).await.map_err(|e| format!("WebRTCClient Error: {:?}", e))?,
            ws_stream: Arc::new(Mutex::new(ws_stream)),
            local_sos: sos,
        });

        client.connection.add_transceiver_from_kind(RTPCodecType::Video, None).await.map_err(|e| format!("WebRTCClient Error: {:?}", e))?;

        let rtp_sender = client.connection.add_track(create_video_track()).await.map_err(|e| format!("WebRTCClient Error: {:?}", e))?;

        client.local_sos.spawn(async move {
            let mut rtcp_buf = vec![0u8; 1500];
            while let Ok((x, _)) = rtp_sender.read(&mut rtcp_buf).await {
                println!("info:::: {:?}", x);
            }
        });

        // Set the handler for Peer connection state
        // This will notify you when the peer has connected/disconnected
        client.connection.on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
            println!("Peer Connection State has changed: {s}");
            Box::pin(async {})
        }));

        // Set up ICE candidate handling
        let ws_stream_clone = Arc::clone(&client.ws_stream);
        let peer_conn_clone = Arc::clone(&client.connection);
        client.connection.on_ice_candidate(Box::new(move |candidate| {
            Box::pin({
                let ws_stream_clone = ws_stream_clone.clone();
                let peer_conn_clone = peer_conn_clone.clone();
                async move {
                    if let Some(candidate) = candidate {
                        let candidate_json = serde_json::to_string(&SignalMessage {
                            sdp: peer_conn_clone.local_description().await,
                            candidate: Some(candidate.to_json().unwrap()),
                        }).unwrap();

                        if ws_stream_clone.lock().await.send(Message::Text(candidate_json)).await.is_err() {
                            eprintln!("Failed to send ICE candidate to server");
                        }
                    }
                }
            })
        }));

        // Start handling signaling
        let client_clone = Arc::clone(&client);
        client.local_sos.spawn(async move {
            if let Err(e) = client_clone.client_handle_signaling().await {
                eprintln!("Error handling signaling: {}", e);
            }
        });

        Ok(client)
    }

    async fn client_handle_signaling(self: Arc<Self>) -> Result<(), Box<dyn std::error::Error>> {
        let ws_stream = Arc::clone(&self.ws_stream);
        let peer_connection = Arc::clone(&self.connection);
        let mut ws_stream_lock = ws_stream.lock().await;

        // Create and send the initial offer to the server
        let offer = peer_connection.create_offer(None).await?;
        peer_connection.set_local_description(offer.clone()).await?;
        let offer_message = SignalMessage {
            sdp: Some(offer),
            candidate: None,
        };
        ws_stream_lock.send(Message::Text(serde_json::to_string(&offer_message)?)).await?;

        // Handle incoming signaling messages
        while let Some(Ok(msg)) = ws_stream_lock.next().await {
            if let Ok(signal) = serde_json::from_str::<SignalMessage>(&msg.to_string()) {
                if let Some(sdp) = signal.sdp {
                    peer_connection.set_remote_description(sdp).await?;
                }
                if let Some(candidate_sdp) = signal.candidate {
                    peer_connection.add_ice_candidate(candidate_sdp).await?;
                }
            }
        }

        Ok(())
    }

    pub async fn receive_video(&self, tx: tokio::sync::mpsc::Sender<Packet>) {
        let tx = Arc::new(Mutex::new(tx));
        let connection = Arc::clone(&self.connection);

        let sos = self.local_sos.clone();

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
                            if workers::sos::get_instance().lock().unwrap().cancelled() {
                                break;
                            }
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

    pub async fn disconnect(&self) {
        if self.connection.ice_connection_state() == RTCIceConnectionState::Connected {
            let senders = self.connection.get_senders();
            for sender in senders.await.iter() {
                sender.stop().await.unwrap();
            }
            self.connection.close().await.unwrap();
        }
    }
}

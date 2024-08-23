use crate::utils::net::webrtc_common::{create_peer_connection, create_webrtc_api};
use async_tungstenite::tokio::connect_async;
use async_tungstenite::WebSocketStream;
use futures_util::{SinkExt, StreamExt};
use gstreamer::{Buffer, BufferRef};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use webrtc::api::API;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::interceptor::Attributes;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtp::packet::Packet;
use webrtc::rtp_transceiver::rtp_receiver::RTCRtpReceiver;
use webrtc::rtp_transceiver::RTCRtpTransceiver;
use webrtc::track::track_remote::TrackRemote;

#[derive(Serialize, Deserialize, Debug)]
struct SignalMessage {
    sdp: Option<RTCSessionDescription>,
    candidate: Option<String>,
}

pub struct WebRTCClient {
    api: Arc<API>,
    peer_connection: Arc<RTCPeerConnection>,
    ws_stream: Arc<Mutex<WebSocketStream<async_tungstenite::tokio::ConnectStream>>>,
}

impl WebRTCClient {
    pub async fn new(signaling_server_url: &str) -> Arc<WebRTCClient> {
        // Create the WebRTC API
        let api = create_webrtc_api();

        // Connect to the signaling server
        let (ws_stream, _) = connect_async(signaling_server_url).await.unwrap();
        let ws_stream = Arc::new(Mutex::new(ws_stream));

        // Create the PeerConnection
        let peer_connection = create_peer_connection(&api).await;

        let client = Arc::new(WebRTCClient {
            api,
            peer_connection,
            ws_stream,
        });

        // Set up ICE candidate handling
        let client_clone = Arc::clone(&client);
        client.peer_connection.on_ice_candidate(Box::new(move |candidate| {
            let client = Arc::clone(&client_clone);
            Box::pin(async move {
                if let Some(candidate) = candidate {
                    let candidate_json = serde_json::json!({
                        "sdp": None::<RTCSessionDescription>,
                        "candidate": Some(candidate.to_json().unwrap()),
                    });
                    let mut ws_stream = client.ws_stream.lock().await;
                    if let Err(e) = ws_stream.send(candidate_json.to_string().into()).await {
                        eprintln!("Failed to send ICE candidate: {}", e);
                    }
                }
            })
        }));

        // Start handling signaling
        let client_clone = Arc::clone(&client);
        tokio::spawn(async move {
            if let Err(e) = client_clone.client_handle_signaling().await {
                eprintln!("Error handling signaling: {}", e);
            }
        });

        client
    }

    async fn client_handle_signaling(self: Arc<Self>) -> Result<(), Box<dyn std::error::Error>> {
        let ws_stream = Arc::clone(&self.ws_stream);
        let peer_connection = Arc::clone(&self.peer_connection);
        let mut ws_stream_lock = ws_stream.lock().await;

        println!("quii1");

        // Send the initial offer to the server
        let offer = peer_connection.create_offer(None).await?;
        peer_connection.set_local_description(offer.clone()).await?;

        println!("quii2");

        let local_desc = peer_connection.local_description().await.unwrap();
        let offer_message = serde_json::json!({
            "sdp": local_desc,
            "candidate": None::<String>,
        });
        ws_stream_lock.send(offer_message.to_string().into()).await?;

        println!("quii3");

        // Handle incoming signaling messages
        while let Some(Ok(msg)) = ws_stream_lock.next().await {
            if let Ok(signal) = serde_json::from_str::<SignalMessage>(&msg.to_string()) {
                if let Some(sdp) = signal.sdp {
                    peer_connection.set_remote_description(sdp).await?;
                }

                if let Some(candidate_sdp) = signal.candidate {
                    let candidate_init = RTCIceCandidateInit {
                        candidate: candidate_sdp,
                        ..Default::default()
                    };
                    peer_connection.add_ice_candidate(candidate_init).await?;
                }
            }
        }
        Ok(())
    }

    pub async fn receive_video(self: Arc<Self>, mut tx: tokio::sync::mpsc::Sender<gstreamer::Buffer>) {
        // Set up the event handler for incoming tracks
        self.clone().peer_connection
            .on_track(Box::new(move |track: Arc<TrackRemote>, _receiver: Arc<RTCRtpReceiver>, _transceiver: Arc<RTCRtpTransceiver>| {
                println!("Receiving video from {:?}", self.peer_connection.get_stats_id());
                Box::pin({
                    let value = tx.clone();
                    async move {
                        let tx_arc_clone = value.clone();
                        while let Ok(sample) = track.read_rtp().await {
                            // Convert the webrtc::media::Sample to a gstreamer::Buffer
                            if let Some(gst_buffer) = Self::sample_to_gst_buffer(sample).await {
                                if let Err(e) = tx_arc_clone.clone().try_send(gst_buffer) {
                                    eprintln!("Error sending frame: {}", e);
                                }
                            }
                        }
                    }
                })
            }));
    }

    async fn sample_to_gst_buffer(sample: (Packet, Attributes)) -> Option<gstreamer::Buffer> {
        let packet = sample.0;
        let attr = sample.1;

        // Extract the payload from the RTP packet
        let payload = &packet.payload;

        // Allocate a new GStreamer buffer with the size of the payload
        let mut buffer = Buffer::with_size(payload.len()).ok()?;
        {
            let mut buffer_ref: &mut BufferRef = buffer.get_mut()?;

            // Set the GStreamer buffer's timestamp based on the RTP packet's timestamp
            buffer_ref.set_pts(
                gstreamer::ClockTime::from_nseconds(
                    packet.header.timestamp as u64 * 1_000_000_000 / 90_000, // Converting RTP timestamp to nanoseconds
                )
            );

            // Map the buffer writable and copy the payload data into the buffer
            let mut map = buffer_ref.map_writable().ok()?;
            map.copy_from_slice(payload);
        }

        println!("{:?}", buffer);

        Some(buffer)
    }
}

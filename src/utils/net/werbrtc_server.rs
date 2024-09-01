use std::ops::Add;
use crate::gui::resource::CAST_SERVICE_PORT;
use crate::utils::net::webrtc_common::{create_peer_connection, create_video_track, create_webrtc_api, SignalMessage};
use async_tungstenite::tokio::{accept_async, TokioAdapter};
use async_tungstenite::tungstenite::Message;
use async_tungstenite::WebSocketStream;
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, Notify};
use tokio::time::sleep;
use webrtc::api::API;
use webrtc::ice_transport::ice_connection_state::RTCIceConnectionState;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::sdp_type::RTCSdpType;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtp_transceiver::rtp_codec::RTPCodecType;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;
use webrtc::track::track_local::TrackLocal;

struct WRTCPeer {
    connection: Arc<RTCPeerConnection>,
    video_track: Arc<TrackLocalStaticSample>,
    notify: Arc<Notify>,
}

#[derive(Clone)]
pub struct WebRTCServer {
    api: Arc<API>,
    peers: Arc<Mutex<Vec<Arc<WRTCPeer>>>>,
}

impl WebRTCServer {
    pub fn new() -> Arc<WebRTCServer> {
        let api = create_webrtc_api();

        let server = Arc::new(WebRTCServer {
            api,
            peers: Arc::new(Mutex::new(Vec::new())),
        });

        let server_clone = Arc::clone(&server);
        tokio::spawn(async move {
            server_clone.run_signaling_server().await;
        });

        server
    }

    pub async fn run_signaling_server(&self) {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", CAST_SERVICE_PORT).to_string()).await.unwrap();

        println!("Listener: {:?}", listener);

        let peers = Arc::clone(&self.peers);
        let api = Arc::clone(&self.api);

        loop {
            let api_clone = Arc::clone(&api);
            let peers_clone = Arc::clone(&peers);

            if let Ok((stream, _)) = listener.accept().await {
                println!("Incoming connection: {:?}", stream);

                let ws_stream = accept_async(stream).await.unwrap();

                tokio::spawn(async move {
                    let peer = Arc::new(WRTCPeer {
                        connection: create_peer_connection(&api_clone).await,
                        video_track: create_video_track(),
                        notify: Arc::new(Notify::new()),
                    });

                    peer.connection.add_transceiver_from_kind(RTPCodecType::Video, None).await.unwrap();

                    let rtp_sender = peer.connection.add_track(Arc::clone(&peer.video_track) as Arc<dyn TrackLocal + Send + Sync>).await.unwrap();

                    // Read incoming RTCP packets
                    // Before these packets are returned they are processed by interceptors. For things
                    // like NACK this needs to be called.
                    tokio::spawn(async move {
                        let mut rtcp_buf = vec![0u8; 1500];
                        while let Ok((x, _)) = rtp_sender.read(&mut rtcp_buf).await {
                            println!("info:::: {:?}", x);
                        }
                        Result::<(), ()>::Ok(())
                    });

                    println!("peer conn {:?}", peer.connection);

                    // add new peer
                    peers_clone.lock().await.push(Arc::clone(&peer));

                    if let Err(e) = WebRTCServer::remote_handle_signaling(peer, ws_stream).await {
                        eprintln!("Error handling signaling: {}", e);
                    }
                });
            }
        }
    }

    async fn remote_handle_signaling(peer: Arc<WRTCPeer>, ws_stream: WebSocketStream<TokioAdapter<TcpStream>>) -> Result<(), Box<dyn std::error::Error>> {
        let (ws_sender, mut ws_receiver) = ws_stream.split();
        let ws_sender = Arc::new(Mutex::new(ws_sender));

        // Set the handler for ICE connection state
        // This will notify you when the peer has connected/disconnected
        let peer_clone = Arc::clone(&peer);
        peer.connection.on_ice_connection_state_change(Box::new(move |connection_state: RTCIceConnectionState| {
            if connection_state == RTCIceConnectionState::Connected {
                peer_clone.notify.notify_waiters();
            } else if connection_state == RTCIceConnectionState::Disconnected {
                // disconnections already handled by send_frame
            }
            Box::pin(async {})
        }));

        // Set the handler for Peer connection state
        // This will notify you when the peer has connected/disconnected
        peer.connection.on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
            println!("Peer Connection State has changed: {s}");
            Box::pin(async {})
        }));

        let ws_sender_clone = Arc::clone(&ws_sender);
        let peer_conn_clone = Arc::clone(&peer.connection);
        peer.connection.on_ice_candidate(Box::new(move |candidate| {
            println!("on_ice_candidate {:?}", candidate);
            Box::pin({
                let peer_conn_clone = peer_conn_clone.clone();
                let ws_sender_clone = ws_sender_clone.clone();
                async move {
                    if let Some(candidate) = candidate {
                        let candidate_str = serde_json::to_string(&SignalMessage {
                            sdp: peer_conn_clone.local_description().await,
                            candidate: Some(candidate.to_json().unwrap()),
                        }).unwrap();

                        if ws_sender_clone.lock().await.send(Message::Text(candidate_str)).await.is_err() {
                            eprintln!("Failed to send ICE candidate to client");
                        }
                    }
                }
            })
        }));

        while let Some(Ok(msg)) = ws_receiver.next().await {
            if let Message::Text(text) = msg {
                let signal: SignalMessage = serde_json::from_str(&text).unwrap();

                if let Some(sdp) = signal.sdp {
                    if sdp.sdp_type == RTCSdpType::Offer {
                        peer.connection.set_remote_description(sdp).await?;

                        // Create and send the answer
                        let answer = peer.connection.create_answer(None).await?;

                        // Create channel that is blocked until ICE Gathering is complete
                        let mut gather_complete = peer.connection.gathering_complete_promise().await;

                        peer.connection.set_local_description(answer.clone()).await?;

                        // Block until ICE Gathering is complete, disabling trickle ICE
                        // we do this because we only can exchange one signaling message
                        // in a production application you should exchange ICE Candidates via OnICECandidate
                        let _ = gather_complete.recv().await;

                        let answer_message = SignalMessage {
                            sdp: Some(answer),
                            candidate: None,
                        };
                        ws_sender.lock().await.send(Message::Text(serde_json::to_string(&answer_message)?)).await?;
                    } else if sdp.sdp_type == RTCSdpType::Answer {
                        peer.connection.set_remote_description(sdp).await?;
                    }
                }

                if let Some(candidate_sdp) = signal.candidate {
                    println!("received candidate {:?}", candidate_sdp);
                    peer.connection.add_ice_candidate(candidate_sdp).await.unwrap();
                }
            }
        }
        Ok(())
    }

    pub async fn send_video_frames(&self, mut receiver: tokio::sync::mpsc::Receiver<gstreamer::Buffer>) -> Result<(), Box<dyn std::error::Error>> {
        let mut frame_i = 0;
        while let Some(buffer) = receiver.recv().await {
            if self.peers.lock().await.len() == 0 {
                sleep(Duration::from_millis(100)).await;
                continue;
            }

            let duration = Duration::from(buffer.duration().unwrap());
            let timestamp = SystemTime::now().add(Duration::from_millis(frame_i));
            frame_i += 1;

            let map = buffer.map_readable().unwrap();
            let slice = map.as_slice();

            let sample = webrtc::media::Sample {
                data: Bytes::copy_from_slice(slice),
                duration,
                timestamp,
                ..Default::default()
            };

            let mut i = 0;
            let mut peers = self.peers.lock().await;
            for peer in peers.clone().iter() {
                //println!("sending sample {:?} to peer {:?} track {:?}", sample.timestamp, peer.connection, peer.video_track);
                if peer.video_track.write_sample(&sample).await.is_err() {
                    peers.remove(i);
                } else {
                    i += 1;
                }
            }
        }
        Ok(())
    }
}
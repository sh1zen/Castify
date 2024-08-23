use crate::gui::resource::CAST_SERVICE_PORT;
use crate::utils::net::webrtc_common::{create_peer_connection, create_webrtc_api};
use async_tungstenite::tokio::{accept_async, TokioAdapter};
use async_tungstenite::WebSocketStream;
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use webrtc::api::API;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;

#[derive(Serialize, Deserialize, Debug)]
struct SignalMessage {
    sdp: Option<RTCSessionDescription>,
    candidate: Option<String>,
}

#[derive(Clone)]
pub struct WebRTCServer {
    api: Arc<API>,
    peers: Arc<Mutex<Vec<Arc<RTCPeerConnection>>>>,
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

        loop {
            if let Ok((stream, _)) = listener.accept().await {
                println!("Incoming connection: {:?}", stream);

                let ws_stream = accept_async(stream).await.unwrap();

                let api = Arc::clone(&self.api);
                let peers = Arc::clone(&self.peers);

                tokio::spawn(async move {
                    let peer_connection = create_peer_connection(&api).await;

                    println!("peer conn {:?}", peer_connection);

                    // add new peer
                    peers.lock().await.push(Arc::clone(&peer_connection));

                    if let Err(e) = WebRTCServer::remote_handle_signaling(peer_connection, ws_stream).await {
                        eprintln!("Error handling signaling: {}", e);
                    }

                    //self.remove_peer(peer_connection).await;
                });
            }
        }
    }

    async fn remote_handle_signaling(
        peer_connection: Arc<RTCPeerConnection>,
        ws_stream: WebSocketStream<TokioAdapter<TcpStream>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        while let Some(Ok(msg)) = ws_receiver.next().await {
            println!("msg: {:?}", msg);
            if let Ok(signal) = serde_json::from_str::<SignalMessage>(&msg.to_string()) {
                if let Some(sdp) = signal.sdp {
                    if sdp.sdp_type == webrtc::peer_connection::sdp::sdp_type::RTCSdpType::Offer {
                        println!("sdp: {:?}", sdp);

                        if let Err(err) = peer_connection.set_remote_description(sdp.clone()).await {
                            eprintln!("Failed to set remote description: {}", err);
                            continue;
                        }

                        let offer = peer_connection.create_offer(None).await.unwrap();
                        peer_connection.set_local_description(offer.clone()).await.unwrap();

                        // Wait until ICE gathering is complete
                        let mut gather_complete = peer_connection.gathering_complete_promise().await;
                        gather_complete.recv().await;

                        let local_desc = peer_connection.local_description().await.unwrap();

                        let answer_message = serde_json::json!({
                            "sdp": local_desc,
                            "candidate": None::<String>,
                        });
                        ws_sender.send(answer_message.to_string().into()).await?;
                    } else {
                        peer_connection.set_remote_description(sdp).await.unwrap();
                    }
                }

                if let Some(candidate_sdp) = signal.candidate {
                    println!("{:?}", candidate_sdp);

                    let candidate_init = RTCIceCandidateInit {
                        candidate: candidate_sdp,
                        ..Default::default()
                    };

                    peer_connection.add_ice_candidate(candidate_init).await.unwrap();
                }
            }
        }
        Ok(())
    }

    pub async fn send_video_frames(&self, mut receiver: tokio::sync::mpsc::Receiver<gstreamer::Buffer>) -> Result<(), Box<dyn std::error::Error>> {

        while let Some(buffer) = receiver.recv().await {
            let duration = Duration::from(buffer.duration().unwrap());
            let timestamp = SystemTime::UNIX_EPOCH + Duration::from_nanos(buffer.pts().unwrap().nseconds());

            let map = buffer.map_readable().unwrap();
            let slice = map.as_slice();

            let sample = webrtc::media::Sample {
                data: Bytes::copy_from_slice(slice),
                duration,
                timestamp,
                ..Default::default()
            };
/*
            let peers = self.peers.lock().await;
            for peer in peers.iter() {
                let video_track_pc = Arc::clone(&video_track);
                peer.add_track(video_track_pc.clone()).await.unwrap();

                for sender in peer.get_senders().await {
                    if let Some(track) = sender.track().await {
                        let video_track = track.downcast_ref::<TrackLocalStaticSample>().unwrap();
                        video_track.write_sample(&sample).await?;
                    }
                }
            }*/
        }

        Ok(())
    }

    async fn add_peer(self: &Arc<Self>, peer: Arc<RTCPeerConnection>) {
        let mut peers = self.peers.lock().await;
        peers.push(peer);
    }

    async fn remove_peer(self: &Arc<Self>, peer: Arc<RTCPeerConnection>) {
        let mut peers = self.peers.lock().await;
        if let Some(index) = peers.iter().position(|p| Arc::ptr_eq(p, &peer)) {
            peers.remove(index);
        }
    }
}
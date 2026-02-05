use crate::utils::net::webrtc::manual::{SDPICEExchange, SDPICEExchangeWRTC};
use crate::utils::net::webrtc::peer::WRTCPeer;
use crate::utils::sos::SignalOfStop;
use crate::utils::{SendResult, try_send};
use async_trait::async_trait;
use async_tungstenite::WebSocketStream;
use async_tungstenite::tokio::{ConnectStream, connect_async};
use async_tungstenite::tungstenite::Error;
use async_tungstenite::tungstenite::handshake::client::Response;
use castbox::Arw;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tokio::sync::mpsc::Sender;
use webrtc::rtp_transceiver::RTCRtpTransceiver;
use webrtc::rtp_transceiver::rtp_receiver::RTCRtpReceiver;
use webrtc::track::track_remote::TrackRemote;

/// Type alias for video RTP packet channel: (payload, marker, sequence_number, timestamp)
type VideoPacketSender = Sender<(Vec<u8>, bool, u16, u32)>;
/// Type alias for audio RTP packet channel: (payload, timestamp)
type AudioPacketSender = Sender<(Vec<u8>, u32)>;

pub struct WebRTCReceiver {
    sos: SignalOfStop,
    peer: Arw<Option<Arc<WRTCPeer>>>,
    manual_handler: Arw<Option<SDPICEExchange>>,
    /// Pre-registered video channel (set before connection)
    video_tx: Arw<Option<VideoPacketSender>>,
    /// Pre-registered audio channel (set before connection)
    audio_tx: Arw<Option<AudioPacketSender>>,
}

impl Default for WebRTCReceiver {
    fn default() -> Self {
        Self::new()
    }
}

impl WebRTCReceiver {
    pub fn new() -> WebRTCReceiver {
        WebRTCReceiver {
            sos: SignalOfStop::new(),
            peer: Arw::new(None),
            manual_handler: Arw::new(None),
            video_tx: Arw::new(None),
            audio_tx: Arw::new(None),
        }
    }

    async fn get_lazy_peer(&self) -> Arc<WRTCPeer> {
        if self.peer.as_ref().is_none() {
            // Receiver doesn't encode, so force_idr is unused
            let dummy_idr = Arc::new(AtomicBool::new(false));
            let peer = WRTCPeer::new(dummy_idr).await.unwrap();

            // Set up on_track handler BEFORE any connection is made
            // This ensures we're ready to receive tracks as soon as they're negotiated
            let video_tx = Arw::clone(&self.video_tx);
            let audio_tx = Arw::clone(&self.audio_tx);
            let sos = self.sos.clone();

            peer.get_connection().on_track(Box::new(move |track: Arc<TrackRemote>, _receiver: Arc<RTCRtpReceiver>, _transceiver: Arc<RTCRtpTransceiver>| {
                let sos = sos.clone();
                let mime_type = track.codec().capability.mime_type.clone();
                
                // Check if we have pre-registered channels
                let video_tx_opt = video_tx.as_ref().clone();
                let audio_tx_opt = audio_tx.as_ref().clone();

                if mime_type.to_lowercase().contains("audio") {
                    // Audio track
                    Box::pin({
                        async move {
                            if let Some(audio_tx) = audio_tx_opt {
                                sos.spawn(async move {
                                    while let Ok((packet, _)) = track.read_rtp().await {
                                        let payload = packet.payload.to_vec();
                                        if payload.is_empty() {
                                            continue;
                                        }
                                        let timestamp = packet.header.timestamp;
                                        // Use try_send to avoid blocking the WebRTC track reader
                                        if try_send(&audio_tx, (payload, timestamp)).is_closed() {
                                            log::error!("Audio channel closed");
                                            break;
                                        }
                                    }
                                });
                            } else {
                                log::warn!("Audio track received but no audio channel registered");
                            }
                        }
                    })
                } else {
                    // Video track
                    Box::pin({
                        async move {
                            if let Some(video_tx) = video_tx_opt {
                                sos.spawn(async move {
                                    log::info!("=== WEBRTC RECEIVER: Video track handler STARTED ===");
                                    let mut packet_count = 0u64;
                                    let mut last_log = std::time::Instant::now();

                                    while let Ok((packet, _)) = track.read_rtp().await {
                                        let payload = packet.payload.to_vec();
                                        if payload.is_empty() {
                                            continue;
                                        }

                                        packet_count += 1;
                                        if packet_count == 1 {
                                            log::info!("WEBRTC RECEIVER: First RTP packet received!");
                                        }

                                        // Log heartbeat
                                        if last_log.elapsed().as_secs() >= 10 {
                                            log::info!("WEBRTC RECEIVER: {} RTP packets read from track", packet_count);
                                            last_log = std::time::Instant::now();
                                        }

                                        let marker = packet.header.marker;
                                        let seq_num = packet.header.sequence_number;
                                        let timestamp = packet.header.timestamp;
                                        // Use try_send to avoid blocking the WebRTC track reader
                                        match try_send(&video_tx, (payload, marker, seq_num, timestamp)) {
                                            SendResult::Sent => {}
                                            SendResult::Full => {
                                                // Channel full, drop packet - better than blocking
                                                log::warn!("WEBRTC RECEIVER: Video channel full, dropping RTP packet");
                                            }
                                            SendResult::Closed => {
                                                log::error!("WEBRTC RECEIVER: Video channel closed");
                                                break;
                                            }
                                        }
                                    }

                                    log::error!("=== WEBRTC RECEIVER: Video track read loop EXITED after {} packets ===", packet_count);
                                });
                            } else {
                                log::warn!("Video track received but no video channel registered");
                            }
                        }
                    })
                }
            }));

            self.peer.as_mut().replace(peer);
        }
        self.peer.as_ref().as_ref().unwrap().clone()
    }

    pub async fn connect(
        &self,
        ws_server_url: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut conn: Result<(WebSocketStream<ConnectStream>, Response), Error> =
            Err(Error::ConnectionClosed);

        while conn.is_err() {
            if self.sos.cancelled() {
                return Err(Error::ConnectionClosed.into());
            }
            let ws_c = String::from(ws_server_url);
            conn = self
                .sos
                .select(async move { connect_async(&*ws_c).await })
                .await
                .unwrap_or(Err(Error::ConnectionClosed));
        }

        let (ws_stream, _) = conn.unwrap();
        let peer = self.get_lazy_peer().await;

        self.sos.spawn(async move {
            let _ = peer.negotiate(ws_stream, false).await;
        });

        Ok(())
    }

    /// Register video and audio channels BEFORE connecting.
    /// This must be called before connect() to ensure the on_track handler
    /// is set up when the SDP negotiation happens.
    pub async fn receive_video(&self, video_tx: VideoPacketSender, audio_tx: AudioPacketSender) {
        // Store channels so they're available when on_track fires
        *self.video_tx.as_mut() = Some(video_tx);
        *self.audio_tx.as_mut() = Some(audio_tx);

        // Ensure peer is created with the on_track handler set up
        // This is crucial - the handler must be registered BEFORE connection
        self.get_lazy_peer().await;

        log::info!("WebRTCReceiver: Video and audio channels registered, on_track handler ready");
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

        self.manual_handler
            .as_ref()
            .as_ref()
            .unwrap()
            .pack()
            .unwrap()
    }

    async fn set_remote_sdp(&self, remote_sdp: String) -> bool {
        let Ok(exchanger_offer) = SDPICEExchange::unpack(remote_sdp) else {
            return false;
        };

        let peer = self.get_lazy_peer().await;

        self.manual_handler.as_mut().replace(SDPICEExchange::new());

        let exchanger_clone = Arw::clone(&self.manual_handler);
        peer.get_connection()
            .on_ice_candidate(Box::new(move |candidate| {
                let exchanger_clone = Arw::clone(&exchanger_clone);
                Box::pin(async move {
                    if let Some(candidate) = candidate {
                        exchanger_clone
                            .as_mut()
                            .as_mut()
                            .unwrap()
                            .add_ice_candidate(candidate);
                    }
                })
            }));

        let res = peer
            .create_answer(exchanger_offer.get_sdp(), true)
            .await
            .is_ok();

        if res {
            self.manual_handler
                .as_mut()
                .as_mut()
                .unwrap()
                .set_sdp(peer.get_connection().local_description().await.unwrap());
        }

        res
    }
}

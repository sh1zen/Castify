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
use rtc::rtp_transceiver::rtp_sender::RtpCodecKind;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tokio::sync::mpsc::Sender;
use webrtc::media_stream::track_remote::{TrackRemote, TrackRemoteEvent};

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
            let dummy_idr = Arc::new(AtomicBool::new(false));
            let peer = WRTCPeer::new(dummy_idr).await.unwrap();

            let video_tx = Arw::clone(&self.video_tx);
            let audio_tx = Arw::clone(&self.audio_tx);
            let sos = self.sos.clone();
            let mut track_rx = peer.subscribe_tracks();

            self.sos.spawn(async move {
                while let Ok(track) = track_rx.recv().await {
                    let video_tx_opt = video_tx.as_ref().clone();
                    let audio_tx_opt = audio_tx.as_ref().clone();
                    let sos = sos.clone();

                    match track.kind().await {
                        RtpCodecKind::Audio => {
                            if let Some(audio_tx) = audio_tx_opt {
                                spawn_audio_track_reader(sos, track, audio_tx);
                            } else {
                                log::warn!(
                                    "Audio track received but no audio channel registered"
                                );
                            }
                        }
                        RtpCodecKind::Video => {
                            if let Some(video_tx) = video_tx_opt {
                                spawn_video_track_reader(sos, track, video_tx);
                            } else {
                                log::warn!(
                                    "Video track received but no video channel registered"
                                );
                            }
                        }
                        _ => {
                            log::warn!("Ignoring unsupported remote track kind");
                        }
                    }
                }
            });

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

    pub async fn receive_video(&self, video_tx: VideoPacketSender, audio_tx: AudioPacketSender) {
        *self.video_tx.as_mut() = Some(video_tx);
        *self.audio_tx.as_mut() = Some(audio_tx);
        self.get_lazy_peer().await;
        log::info!("WebRTCReceiver: channel registration complete");
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

        let res = peer
            .create_answer(exchanger_offer.get_sdp(), true)
            .await
            .is_ok();

        if res
            && let Some(local_sdp) = peer.get_connection().local_description().await
        {
            self.manual_handler
                .as_mut()
                .as_mut()
                .unwrap()
                .set_sdp(local_sdp);
        }

        res
    }
}

fn spawn_audio_track_reader(
    sos: SignalOfStop,
    track: Arc<dyn TrackRemote>,
    audio_tx: AudioPacketSender,
) {
    sos.spawn(async move {
        while let Some(event) = track.poll().await {
            if let TrackRemoteEvent::OnRtpPacket(packet) = event {
                let payload = packet.payload.to_vec();
                if payload.is_empty() {
                    continue;
                }
                let timestamp = packet.header.timestamp;
                if try_send(&audio_tx, (payload, timestamp)).is_closed() {
                    log::error!("Audio channel closed");
                    break;
                }
            }
        }
    });
}

fn spawn_video_track_reader(
    sos: SignalOfStop,
    track: Arc<dyn TrackRemote>,
    video_tx: VideoPacketSender,
) {
    sos.spawn(async move {
        while let Some(event) = track.poll().await {
            if let TrackRemoteEvent::OnRtpPacket(packet) = event {
                let payload = packet.payload.to_vec();
                if payload.is_empty() {
                    continue;
                }

                let marker = packet.header.marker;
                let seq_num = packet.header.sequence_number;
                let timestamp = packet.header.timestamp;

                match try_send(&video_tx, (payload, marker, seq_num, timestamp)) {
                    SendResult::Sent => {}
                    SendResult::Full => {
                        log::warn!("WEBRTC RECEIVER: video channel full, dropping RTP packet");
                    }
                    SendResult::Closed => {
                        log::error!("WEBRTC RECEIVER: video channel closed");
                        break;
                    }
                }
            }
        }
    });
}

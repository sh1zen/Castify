use crate::utils::net::webrtc::common::{
    SignalMessage, create_audio_track, create_peer_connection, create_video_track,
};
use crate::utils::sos::SignalOfStop;
use async_tungstenite::WebSocketStream;
use async_tungstenite::tokio::ConnectStream;
use async_tungstenite::tungstenite::{Message, Utf8Bytes};
use futures_util::StreamExt;
use iced::futures::executor::block_on;
use once_cell::sync::Lazy;
use rtc::media::Sample;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use tokio::sync::{Notify, broadcast};
use webrtc::media_stream::Track;
use webrtc::media_stream::track_local::TrackLocal;
use webrtc::media_stream::track_local::static_sample::TrackLocalStaticSample;
use webrtc::media_stream::track_remote::TrackRemote;
use webrtc::peer_connection::{
    PeerConnection, PeerConnectionEventHandler, RTCIceGatheringState, RTCPeerConnectionState,
    RTCSdpType, RTCSessionDescription,
};

static WRTC_PEER_UUID: Lazy<AtomicU32> = Lazy::new(|| AtomicU32::new(0));

#[derive(Clone)]
struct WRTCPeerHandler {
    online: Arc<AtomicBool>,
    ice_complete: Arc<AtomicBool>,
    ice_notify: Arc<Notify>,
    track_tx: broadcast::Sender<Arc<dyn TrackRemote>>,
}

#[async_trait::async_trait]
impl PeerConnectionEventHandler for WRTCPeerHandler {
    async fn on_ice_gathering_state_change(&self, state: RTCIceGatheringState) {
        let complete = state == RTCIceGatheringState::Complete;
        self.ice_complete.store(complete, Ordering::Relaxed);
        if complete {
            self.ice_notify.notify_waiters();
        }
    }

    async fn on_connection_state_change(&self, state: RTCPeerConnectionState) {
        log::warn!("Peer connection state changed: {:?}", state);
        match state {
            RTCPeerConnectionState::Connected => {
                self.online.store(true, Ordering::Relaxed);
            }
            RTCPeerConnectionState::Disconnected
            | RTCPeerConnectionState::Failed
            | RTCPeerConnectionState::Closed => {
                self.online.store(false, Ordering::Relaxed);
            }
            _ => {}
        }
    }

    async fn on_track(&self, track: Arc<dyn TrackRemote>) {
        let _ = self.track_tx.send(track);
    }
}

pub struct WRTCPeer {
    connection: Arc<dyn PeerConnection>,
    video_track: Arc<TrackLocalStaticSample>,
    audio_track: Arc<TrackLocalStaticSample>,
    video_ssrc: u32,
    audio_ssrc: u32,
    online: Arc<AtomicBool>,
    ice_complete: Arc<AtomicBool>,
    ice_notify: Arc<Notify>,
    track_tx: broadcast::Sender<Arc<dyn TrackRemote>>,
    id: u32,
    sos: SignalOfStop,
}

impl WRTCPeer {
    pub async fn new(
        _force_idr: Arc<AtomicBool>,
    ) -> Result<Arc<WRTCPeer>, Box<dyn std::error::Error + Send + Sync>> {
        let sos = SignalOfStop::new();
        let online = Arc::new(AtomicBool::new(true));
        let ice_complete = Arc::new(AtomicBool::new(false));
        let ice_notify = Arc::new(Notify::new());
        let (track_tx, _) = broadcast::channel(8);

        let handler = Arc::new(WRTCPeerHandler {
            online: Arc::clone(&online),
            ice_complete: Arc::clone(&ice_complete),
            ice_notify: Arc::clone(&ice_notify),
            track_tx: track_tx.clone(),
        });

        let connection = create_peer_connection(handler).await?;
        let video_track = create_video_track()?;
        let audio_track = create_audio_track()?;

        connection
            .add_track(Arc::clone(&video_track) as Arc<dyn TrackLocal>)
            .await
            .map_err(|e| format!("WebRTC video track error: {e}"))?;
        connection
            .add_track(Arc::clone(&audio_track) as Arc<dyn TrackLocal>)
            .await
            .map_err(|e| format!("WebRTC audio track error: {e}"))?;

        let video_ssrc = *video_track
            .ssrcs()
            .await
            .first()
            .ok_or_else(|| std::io::Error::other("video track missing SSRC"))?;
        let audio_ssrc = *audio_track
            .ssrcs()
            .await
            .first()
            .ok_or_else(|| std::io::Error::other("audio track missing SSRC"))?;

        Ok(Arc::new(WRTCPeer {
            connection,
            video_track,
            audio_track,
            video_ssrc,
            audio_ssrc,
            online,
            ice_complete,
            ice_notify,
            track_tx,
            id: WRTC_PEER_UUID.fetch_add(1, Ordering::Relaxed),
            sos,
        }))
    }

    pub async fn disconnect(&self) {
        log::info!("Peer {} has disconnected", self.id);
        self.online.store(false, Ordering::Relaxed);
        self.sos.cancelled();
        let _ = self.connection.close().await;
    }

    pub fn lazy_disconnect(self: Arc<Self>) {
        let self_clone = Arc::clone(&self);
        tokio::spawn(async move { self_clone.disconnect().await });
    }

    pub fn get_connection(&self) -> Arc<dyn PeerConnection> {
        Arc::clone(&self.connection)
    }

    pub fn subscribe_tracks(&self) -> broadcast::Receiver<Arc<dyn TrackRemote>> {
        self.track_tx.subscribe()
    }

    pub fn is_online(&self) -> bool {
        self.online.load(Ordering::Relaxed)
    }

    pub async fn wait_ice(&self) {
        if self.ice_complete.load(Ordering::Relaxed) {
            return;
        }

        loop {
            self.ice_notify.notified().await;
            if self.ice_complete.load(Ordering::Relaxed) {
                return;
            }
        }
    }

    pub async fn create_offer(
        &self,
        wait: bool,
    ) -> Result<RTCSessionDescription, Box<dyn std::error::Error + Send + Sync>> {
        self.ice_complete.store(false, Ordering::Relaxed);
        let offer = self.connection.create_offer(None).await?;
        self.connection.set_local_description(offer.clone()).await?;
        if wait {
            self.wait_ice().await;
            if let Some(local_desc) = self.connection.local_description().await {
                return Ok(local_desc);
            }
        }
        Ok(offer)
    }

    pub async fn create_answer(
        &self,
        offer: RTCSessionDescription,
        wait: bool,
    ) -> Result<RTCSessionDescription, Box<dyn std::error::Error + Send + Sync>> {
        self.set_remote_sdp(offer).await?;
        self.ice_complete.store(false, Ordering::Relaxed);
        let answer = self.connection.create_answer(None).await?;
        self.connection.set_local_description(answer.clone()).await?;
        if wait {
            self.wait_ice().await;
            if let Some(local_desc) = self.connection.local_description().await {
                return Ok(local_desc);
            }
        }
        Ok(answer)
    }

    pub async fn set_remote_sdp(
        &self,
        sdp: RTCSessionDescription,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.connection.set_remote_description(sdp).await?;
        Ok(())
    }

    pub async fn send_video_sample(
        &self,
        sample: &Sample,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.video_track
            .sample_writer(self.video_ssrc)
            .write_sample(sample)
            .await?;
        Ok(())
    }

    pub async fn send_audio_sample(
        &self,
        sample: &Sample,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.audio_track
            .sample_writer(self.audio_ssrc)
            .write_sample(sample)
            .await?;
        Ok(())
    }

    pub async fn negotiate(
        self: Arc<Self>,
        ws_stream: WebSocketStream<ConnectStream>,
        init_offer: bool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        if init_offer {
            let offer = self.create_offer(true).await?;
            let offer_message = SignalMessage {
                sdp: Some(offer),
                candidate: None,
            };
            ws_sender
                .send(Message::Text(Utf8Bytes::from(serde_json::to_string(
                    &offer_message,
                )?)))
                .await?;
        }

        while let Some(Ok(msg)) = ws_receiver.next().await {
            if let Ok(signal) = serde_json::from_str::<SignalMessage>(&msg.to_string()) {
                if let Some(sdp) = signal.sdp {
                    match sdp.sdp_type {
                        RTCSdpType::Offer => {
                            let Ok(answer) = self.create_answer(sdp, true).await else {
                                continue;
                            };
                            ws_sender
                                .send(Message::Text(Utf8Bytes::from(
                                    serde_json::to_string(&SignalMessage {
                                        sdp: Some(answer),
                                        candidate: None,
                                    })?,
                                )))
                                .await?;
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

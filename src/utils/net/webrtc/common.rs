use rtc::interceptor::Registry;
use rtc::media_stream::MediaStreamTrack;
use rtc::peer_connection::configuration::interceptor_registry::register_default_interceptors;
use rtc::peer_connection::configuration::media_engine::{
    MIME_TYPE_H264, MIME_TYPE_OPUS, MediaEngine,
};
use rtc::rtp_transceiver::rtp_sender::{
    RTCRtpCodec, RTCRtpCodingParameters, RTCRtpEncodingParameters, RtpCodecKind,
};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use local_ip_address::local_ip;
use webrtc::media_stream::track_local::static_sample::TrackLocalStaticSample;
use webrtc::peer_connection::{
    PeerConnection, PeerConnectionBuilder, PeerConnectionEventHandler, RTCConfigurationBuilder,
    RTCIceCandidateInit, RTCIceServer, RTCSessionDescription,
};
use webrtc::runtime::default_runtime;

static NEXT_SSRC: AtomicU32 = AtomicU32::new(1);

#[derive(Serialize, Deserialize, Debug)]
pub struct SignalMessage {
    pub sdp: Option<RTCSessionDescription>,
    pub candidate: Option<RTCIceCandidateInit>,
}

pub async fn create_peer_connection(
    handler: Arc<dyn PeerConnectionEventHandler>,
) -> Result<Arc<dyn PeerConnection>, Box<dyn std::error::Error + Send + Sync>> {
    let mut media_engine = MediaEngine::default();
    media_engine.register_default_codecs()?;

    let registry = register_default_interceptors(Registry::new(), &mut media_engine)?;

    let config = RTCConfigurationBuilder::new()
        .with_ice_servers(vec![RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_string()],
            ..Default::default()
        }])
        .build();

    let runtime =
        default_runtime().ok_or_else(|| std::io::Error::other("no async runtime found"))?;

    let bind_addr = resolve_bind_addr();

    let peer_connection = PeerConnectionBuilder::new()
        .with_configuration(config)
        .with_media_engine(media_engine)
        .with_interceptor_registry(registry)
        .with_handler(handler)
        .with_runtime(runtime)
        .with_udp_addrs(vec![bind_addr])
        .build()
        .await?;

    Ok(Arc::new(peer_connection))
}

fn resolve_bind_addr() -> String {
    match local_ip() {
        Ok(IpAddr::V4(ip)) if !ip.is_unspecified() => format!("{ip}:0"),
        Ok(IpAddr::V6(ip)) if !ip.is_unspecified() => format!("[{ip}]:0"),
        _ => "127.0.0.1:0".to_string(),
    }
}

pub fn create_video_track() -> Result<Arc<TrackLocalStaticSample>, Box<dyn std::error::Error + Send + Sync>> {
    create_track(
        RtpCodecKind::Video,
        MIME_TYPE_H264,
        90000,
        0,
        "castify-video",
        "castify-video-track",
        "castify-video-label",
    )
}

pub fn create_audio_track() -> Result<Arc<TrackLocalStaticSample>, Box<dyn std::error::Error + Send + Sync>> {
    create_track(
        RtpCodecKind::Audio,
        MIME_TYPE_OPUS,
        48000,
        2,
        "castify-audio",
        "castify-audio-track",
        "castify-audio-label",
    )
}

fn create_track(
    kind: RtpCodecKind,
    mime_type: &str,
    clock_rate: u32,
    channels: u16,
    stream_id: &str,
    track_id: &str,
    label: &str,
) -> Result<Arc<TrackLocalStaticSample>, Box<dyn std::error::Error + Send + Sync>> {
    let ssrc = NEXT_SSRC.fetch_add(1, Ordering::Relaxed);
    let track = MediaStreamTrack::new(
        stream_id.to_string(),
        track_id.to_string(),
        label.to_string(),
        kind,
        vec![RTCRtpEncodingParameters {
            rtp_coding_parameters: RTCRtpCodingParameters {
                ssrc: Some(ssrc),
                ..Default::default()
            },
            codec: RTCRtpCodec {
                mime_type: mime_type.to_string(),
                clock_rate,
                channels,
                ..Default::default()
            },
            ..Default::default()
        }],
    );

    Ok(Arc::new(TrackLocalStaticSample::new(track)?))
}

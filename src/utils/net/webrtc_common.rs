use std::sync::Arc;
use serde::{Deserialize, Serialize};
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::{APIBuilder, API};
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct SignalMessage {
    pub sdp: Option<RTCSessionDescription>,
    pub candidate: Option<RTCIceCandidateInit>,
}

pub(crate) fn create_webrtc_api() -> Arc<API> {
    let mut media_engine = MediaEngine::default();
    media_engine.register_default_codecs().unwrap();

    let mut registry = Registry::new();
    registry = register_default_interceptors(registry, &mut media_engine).unwrap();

    let api = APIBuilder::new()
        .with_media_engine(media_engine)
        .with_interceptor_registry(registry)
        .build();

    Arc::new(api)
}

pub(crate) async fn create_peer_connection(api: &Arc<API>) -> Arc<RTCPeerConnection> {
    let config = RTCConfiguration {
        ice_servers: vec![
            webrtc::ice_transport::ice_server::RTCIceServer {
                urls: vec![
                    //"stun:stun.l.google.com:19302".to_string(),
                    //"stun:stun.services.mozilla.com:3478".to_string()
                ],
                ..Default::default()
            },
        ],
        ..Default::default()
    };

    let peer_connection = api.new_peer_connection(config).await.unwrap();

    Arc::new(peer_connection)
}

pub(crate) fn create_video_track() -> Arc<TrackLocalStaticSample> {
    Arc::new(TrackLocalStaticSample::new(
        RTCRtpCodecCapability {
            mime_type: "video/H264".to_string(), // video/x-h264
            ..Default::default()
        },
        "video".to_string(),
        "webrtc-ts".to_string(),
    ))
}

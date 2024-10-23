use crate::utils::string;
use async_trait::async_trait;
use base64::engine::general_purpose::PAD;
use base64::engine::GeneralPurpose;
use base64::{alphabet, Engine};
use serde::{Deserialize, Serialize};
use std::error::Error;
use webrtc::ice_transport::ice_candidate::{RTCIceCandidate, RTCIceCandidateInit};
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

#[async_trait]
pub trait SDPICEExchangeWRTC: Send + Sync {
    async fn get_sdp(&self) -> String;

    async fn set_remote_sdp(&self, sdp: String) -> bool;
}

#[derive(Serialize, Deserialize)]
pub struct SDPICEExchange {
    ice_candidates: Vec<RTCIceCandidateInit>,
    sdp: RTCSessionDescription,
}

#[allow(dead_code)]
impl SDPICEExchange {
    pub fn new() -> SDPICEExchange {
        SDPICEExchange {
            ice_candidates: Vec::new(),
            sdp: RTCSessionDescription::default(),
        }
    }

    pub fn new_with_spd(sdp: RTCSessionDescription) -> SDPICEExchange {
        SDPICEExchange {
            ice_candidates: Vec::new(),
            sdp,
        }
    }

    pub fn set_sdp(&mut self, sdp: RTCSessionDescription) {
        self.sdp = sdp;
    }

    pub fn get_sdp(&self) -> RTCSessionDescription {
        self.sdp.clone()
    }

    pub fn add_ice_candidate(&mut self, ice_candidate: RTCIceCandidate) {
        if let Ok(candidate) = ice_candidate.to_json() {
            self.ice_candidates.push(candidate);
        }
    }

    pub fn get_ice_candidates(&self) -> Vec<RTCIceCandidateInit> {
        self.ice_candidates.clone()
    }

    pub fn pack(&self) -> Result<String, Box<dyn Error + Sync + Send>> {
        let str = serde_json::to_string(&self)?;
        let str = string::compress_string(&*str)?;
        let str = GeneralPurpose::new(&alphabet::STANDARD, PAD).encode(str);
        Ok(str)
    }

    pub fn unpack(packed: String) -> Result<SDPICEExchange, Box<dyn Error + Sync + Send>> {
        let str = GeneralPurpose::new(&alphabet::STANDARD, PAD).decode(packed.trim())?;
        let str = string::decompress_string(&*str)?;
        let exchanger = serde_json::from_str::<SDPICEExchange>(&str)?;
        Ok(exchanger)
    }
}
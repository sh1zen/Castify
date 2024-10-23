use crate::utils::net::webrtc::peer::WRTCPeer;
use crate::utils::sos::SignalOfStop;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;

pub struct WebRTCCaster {
    sos: SignalOfStop,
    peers: Arc<Mutex<Vec<Arc<WRTCPeer>>>>,
    manual: Arc<Mutex<Option<Arc<WRTCPeer>>>>,
}

impl WebRTCCaster {
    pub fn new() -> Self {
        WebRTCCaster {
            sos: SignalOfStop::new(),
            peers: Arc::new(Mutex::new(Vec::new())),
            manual: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn push(&self, peer: Arc<WRTCPeer>) {
        self.peers.lock().await.push(peer);
    }

    pub async fn get_manual_connection(&self) -> Arc<WRTCPeer> {
        if self.manual.lock().await.is_none() {
            self.manual.lock().await.replace(WRTCPeer::new().await.unwrap());
        }
        Arc::clone(self.manual.lock().await.as_ref().unwrap())
    }

    pub async fn finalize_manual(&self) -> bool {
        if let Some(manual) = self.manual.lock().await.take() {
            self.push(manual).await;
            true
        } else {
            false
        }
    }

    pub fn send_video_frames(&self, mut receiver: tokio::sync::mpsc::Receiver<gstreamer::Buffer>) {
        let peers = Arc::clone(&self.peers);

        self.sos.spawn(async move {
            while let Some(buffer) = receiver.recv().await {
                if peers.lock().await.len() == 0 {
                    sleep(Duration::from_millis(100)).await;
                    continue;
                }

                let Ok(map) = buffer.map_readable() else {
                    continue;
                };

                let sample = Arc::new(
                    webrtc::media::Sample {
                        data: map.as_slice().to_vec().into(),
                        duration: Duration::from(buffer.duration().unwrap_or_default()),
                        ..Default::default()
                    }
                );

                peers.lock().await.retain(|peer| {
                    if peer.is_online() {
                        let peer = Arc::clone(&peer);
                        let sample_clone = Arc::clone(&sample);
                        tokio::spawn(async move {
                            let _ = peer.video_track.write_sample(&sample_clone).await;
                        });
                        true
                    } else {
                        false
                    }
                });
            }
        });
    }

    pub fn close(&self) {
        self.sos.cancel()
    }
}
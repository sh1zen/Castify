use crate::utils::net::webrtc::peer::WRTCPeer;
use crate::utils::sos::SignalOfStop;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};

pub struct WebRTCCaster {
    sos: SignalOfStop,
    peers: Arc<RwLock<Vec<Arc<WRTCPeer>>>>,
    manual: Arc<Mutex<Option<Arc<WRTCPeer>>>>,
    /// Incremented on peer add/remove so the send loop can detect changes
    /// and rebuild its cached snapshot only when needed.
    peers_version: Arc<AtomicU64>,
    /// Shared force_idr flag for manual peer creation
    force_idr: std::sync::Mutex<Arc<AtomicBool>>,
}

impl WebRTCCaster {
    pub fn new() -> Self {
        WebRTCCaster {
            sos: SignalOfStop::new(),
            peers: Arc::new(RwLock::new(Vec::new())),
            manual: Arc::new(Mutex::new(None)),
            peers_version: Arc::new(AtomicU64::new(0)),
            force_idr: std::sync::Mutex::new(Arc::new(AtomicBool::new(false))),
        }
    }

    pub fn set_force_idr(&self, flag: Arc<AtomicBool>) {
        *self.force_idr.lock().unwrap() = flag;
    }

    pub async fn push(&self, peer: Arc<WRTCPeer>) {
        self.peers.write().await.push(peer);
        self.peers_version.fetch_add(1, Ordering::Relaxed);
    }

    pub async fn get_manual_connection(&self) -> Arc<WRTCPeer> {
        if self.manual.lock().await.is_none() {
            let force_idr = self.force_idr.lock().unwrap().clone();
            self.manual.lock().await.replace(WRTCPeer::new(force_idr).await.unwrap());
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

    pub fn send_video_frames(&self, mut receiver: tokio::sync::mpsc::Receiver<Vec<u8>>) {
        let peers = Arc::clone(&self.peers);
        let peers_version = Arc::clone(&self.peers_version);

        self.sos.spawn(async move {
            // Cached peer snapshot — rebuilt only when peers_version changes
            let mut cached_peers: Vec<Arc<WRTCPeer>> = Vec::new();
            let mut last_version: u64 = u64::MAX; // force initial rebuild
            let mut last_frame_time = std::time::Instant::now();

            while let Some(data) = receiver.recv().await {
                let now = std::time::Instant::now();
                let frame_duration = now.duration_since(last_frame_time);
                last_frame_time = now;

                let sample = Arc::new(webrtc::media::Sample {
                    data: data.into(),
                    duration: frame_duration,
                    ..Default::default()
                });

                // Only rebuild snapshot when peers changed
                let current_version = peers_version.load(Ordering::Relaxed);
                if current_version != last_version {
                    cached_peers = {
                        let guard = peers.read().await;
                        guard.iter().filter(|p| p.is_online()).cloned().collect()
                    };
                    last_version = current_version;
                }

                // Send to all cached online peers
                for peer in &cached_peers {
                    let _ = peer.video_track.write_sample(&sample).await;
                }

                // Periodically clean up disconnected peers
                let has_offline = cached_peers.iter().any(|p| !p.is_online());
                if has_offline {
                    peers.write().await.retain(|p| p.is_online());
                    peers_version.fetch_add(1, Ordering::Relaxed);
                    // Force snapshot rebuild on next iteration
                    last_version = u64::MAX;
                }
            }
        });
    }

    pub fn send_audio_frames(&self, mut receiver: tokio::sync::mpsc::Receiver<Vec<u8>>) {
        let peers = Arc::clone(&self.peers);
        let peers_version = Arc::clone(&self.peers_version);

        self.sos.spawn(async move {
            let mut cached_peers: Vec<Arc<WRTCPeer>> = Vec::new();
            let mut last_version: u64 = u64::MAX;

            while let Some(data) = receiver.recv().await {
                let sample = Arc::new(webrtc::media::Sample {
                    data: data.into(),
                    duration: Duration::from_millis(10), // 10ms Opus frames
                    ..Default::default()
                });

                let current_version = peers_version.load(Ordering::Relaxed);
                if current_version != last_version {
                    cached_peers = {
                        let guard = peers.read().await;
                        guard.iter().filter(|p| p.is_online()).cloned().collect()
                    };
                    last_version = current_version;
                }

                for peer in &cached_peers {
                    let _ = peer.audio_track.write_sample(&sample).await;
                }
            }
        });
    }

    pub fn close(&self) {
        self.sos.cancel()
    }
}

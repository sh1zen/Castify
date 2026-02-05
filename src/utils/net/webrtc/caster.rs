use crate::utils::net::webrtc::peer::WRTCPeer;
use crate::utils::sos::SignalOfStop;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamProfile {
    High,
    Balanced,
    Low,
    Emergency,
}

impl StreamProfile {
    fn frame_skip(self) -> u64 {
        match self {
            StreamProfile::High => 0,
            StreamProfile::Balanced => 1,
            StreamProfile::Low => 2,
            StreamProfile::Emergency => 4,
        }
    }
}

struct AdaptiveVideoController {
    profile: StreamProfile,
    last_eval: Instant,
    stable_intervals: u32,
    avg_send_ms: f64,
    interval_sent: u64,
    interval_failures: u64,
    frame_counter: u64,
}

impl AdaptiveVideoController {
    fn new() -> Self {
        Self {
            profile: StreamProfile::High,
            last_eval: Instant::now(),
            stable_intervals: 0,
            avg_send_ms: 0.0,
            interval_sent: 0,
            interval_failures: 0,
            frame_counter: 0,
        }
    }

    fn should_send(&mut self, is_keyframe: bool) -> bool {
        self.frame_counter = self.frame_counter.wrapping_add(1);
        if is_keyframe {
            return true;
        }
        let skip = self.profile.frame_skip();
        skip == 0 || self.frame_counter.is_multiple_of(skip + 1)
    }

    fn observe(&mut self, send_ms: f64, failures: u64, peers: usize) {
        const ALPHA: f64 = 0.2;
        self.avg_send_ms = if self.avg_send_ms == 0.0 {
            send_ms
        } else {
            self.avg_send_ms * (1.0 - ALPHA) + send_ms * ALPHA
        };
        self.interval_sent += peers as u64;
        self.interval_failures += failures;
    }

    fn tick(&mut self) -> Option<StreamProfile> {
        if self.last_eval.elapsed() < Duration::from_secs(1) {
            return None;
        }
        self.last_eval = Instant::now();

        let failure_ratio = if self.interval_sent == 0 {
            0.0
        } else {
            self.interval_failures as f64 / self.interval_sent as f64
        };

        let congested = self.avg_send_ms > 28.0 || failure_ratio > 0.08;
        let severe = self.avg_send_ms > 55.0 || failure_ratio > 0.25;
        let previous = self.profile;

        if severe {
            self.profile = match self.profile {
                StreamProfile::High => StreamProfile::Low,
                StreamProfile::Balanced => StreamProfile::Low,
                StreamProfile::Low => StreamProfile::Emergency,
                StreamProfile::Emergency => StreamProfile::Emergency,
            };
            self.stable_intervals = 0;
        } else if congested {
            self.profile = match self.profile {
                StreamProfile::High => StreamProfile::Balanced,
                StreamProfile::Balanced => StreamProfile::Low,
                StreamProfile::Low => StreamProfile::Low,
                StreamProfile::Emergency => StreamProfile::Low,
            };
            self.stable_intervals = 0;
        } else {
            self.stable_intervals = self.stable_intervals.saturating_add(1);
            if self.stable_intervals >= 4 {
                self.profile = match self.profile {
                    StreamProfile::Emergency => StreamProfile::Low,
                    StreamProfile::Low => StreamProfile::Balanced,
                    StreamProfile::Balanced => StreamProfile::High,
                    StreamProfile::High => StreamProfile::High,
                };
                self.stable_intervals = 0;
            }
        }

        self.interval_sent = 0;
        self.interval_failures = 0;
        (previous != self.profile).then_some(self.profile)
    }
}

fn contains_idr(data: &[u8]) -> bool {
    let start_code = [0u8, 0, 0, 1];
    let mut i = 0usize;
    while i + 4 < data.len() {
        if data[i..i + 4] == start_code {
            if i + 4 < data.len() && (data[i + 4] & 0x1F) == 5 {
                return true;
            }
            i += 4;
        } else {
            i += 1;
        }
    }
    false
}

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
            self.manual
                .lock()
                .await
                .replace(WRTCPeer::new(force_idr).await.unwrap());
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

    pub fn send_video_frames(
        &self,
        mut receiver: tokio::sync::mpsc::Receiver<crate::capture::capturer::EncodedFrame>,
    ) {
        let peers = Arc::clone(&self.peers);
        let peers_version = Arc::clone(&self.peers_version);

        self.sos.spawn(async move {
            log::info!("=== WEBRTC SENDER: Video send loop STARTED ===");

            let mut cached_peers: Vec<Arc<WRTCPeer>> = Vec::new();
            let mut last_version: u64 = u64::MAX;
            let mut last_frame_time = Instant::now();
            let mut total_frames_sent = 0u64;
            let mut last_stats_log = Instant::now();
            let mut adaptive = AdaptiveVideoController::new();

            while let Some(frame) = receiver.recv().await {
                if total_frames_sent == 0 {
                    log::info!(
                        "WEBRTC SENDER: First frame received! Size: {} bytes",
                        frame.data.len()
                    );
                }

                let is_keyframe = contains_idr(&frame.data);
                if !adaptive.should_send(is_keyframe) {
                    continue;
                }

                let now = Instant::now();
                let frame_duration = now.duration_since(last_frame_time);
                last_frame_time = now;

                let sample = Arc::new(webrtc::media::Sample {
                    data: frame.data.into(),
                    duration: frame_duration,
                    ..Default::default()
                });

                let current_version = peers_version.load(Ordering::Relaxed);
                if current_version != last_version {
                    cached_peers = {
                        let guard = peers.read().await;
                        guard.iter().filter(|p| p.is_online()).cloned().collect()
                    };
                    last_version = current_version;
                    log::info!(
                        "Video sender: updated peer list, {} active peers",
                        cached_peers.len()
                    );
                }

                let send_started = Instant::now();
                let mut send_failures = 0u64;
                for peer in &cached_peers {
                    let send = tokio::time::timeout(
                        Duration::from_millis(80),
                        peer.video_track.write_sample(&sample),
                    )
                    .await;

                    match send {
                        Ok(Ok(())) => {}
                        Ok(Err(e)) => {
                            log::warn!("Failed to send video sample to peer: {}", e);
                            send_failures += 1;
                        }
                        Err(_) => {
                            log::warn!("Timed out sending video sample to peer");
                            send_failures += 1;
                        }
                    }
                }

                let send_ms = send_started.elapsed().as_secs_f64() * 1000.0;
                adaptive.observe(send_ms, send_failures, cached_peers.len());
                if let Some(profile) = adaptive.tick() {
                    log::warn!(
                        "Adaptive video profile switched to {:?} (avg_send={:.1}ms)",
                        profile,
                        adaptive.avg_send_ms
                    );
                }

                if !cached_peers.is_empty() && send_failures == cached_peers.len() as u64 {
                    log::error!(
                        "Failed to send video to ALL {} peers - connection may be stalled",
                        cached_peers.len()
                    );
                }

                total_frames_sent += 1;

                if last_stats_log.elapsed().as_secs() >= 30 {
                    log::info!(
                        "Video sender stats: {} frames sent, {} active peers, avg_send={:.1}ms",
                        total_frames_sent,
                        cached_peers.len(),
                        adaptive.avg_send_ms
                    );
                    last_stats_log = Instant::now();
                }

                let has_offline = cached_peers.iter().any(|p| !p.is_online());
                if has_offline {
                    peers.write().await.retain(|p| p.is_online());
                    peers_version.fetch_add(1, Ordering::Relaxed);
                    last_version = u64::MAX;
                }
            }

            log::error!(
                "=== WEBRTC SENDER: Video send loop EXITED after {} frames ===",
                total_frames_sent
            );
        });
    }

    pub fn send_audio_frames(&self, mut receiver: tokio::sync::mpsc::Receiver<Vec<u8>>) {
        let peers = Arc::clone(&self.peers);
        let peers_version = Arc::clone(&self.peers_version);

        self.sos.spawn(async move {
            let mut cached_peers: Vec<Arc<WRTCPeer>> = Vec::new();
            let mut last_version: u64 = u64::MAX;
            let mut total_frames_sent = 0u64;
            let mut last_stats_log = Instant::now();

            while let Some(data) = receiver.recv().await {
                let sample = Arc::new(webrtc::media::Sample {
                    data: data.into(),
                    duration: Duration::from_millis(10),
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

                let mut send_failures = 0;
                for peer in &cached_peers {
                    if let Err(e) = peer.audio_track.write_sample(&sample).await {
                        log::warn!("Failed to send audio sample to peer: {}", e);
                        send_failures += 1;
                    }
                }

                if send_failures > 0 && send_failures == cached_peers.len() {
                    log::error!(
                        "Failed to send audio to ALL {} peers - connection may be stalled",
                        cached_peers.len()
                    );
                }

                total_frames_sent += 1;
                if last_stats_log.elapsed().as_secs() >= 60 {
                    log::info!(
                        "Audio sender stats: {} frames sent, {} active peers",
                        total_frames_sent,
                        cached_peers.len()
                    );
                    last_stats_log = Instant::now();
                }
            }
        });
    }

    pub fn close(&self) {
        self.sos.cancel()
    }
}

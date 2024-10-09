use crate::assets::{FRAME_RATE, USE_WEBRTC};
use crate::utils::result_to_option;
use crate::utils::sos::SignalOfStop;
use crate::workers::save_stream::SaveStream;
use crate::workers::WorkerClose;
use gstreamer::Pipeline;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct Receiver {
    is_streaming: Arc<Mutex<bool>>,
    save_stream: Option<SaveStream>,
    caster_addr: Option<SocketAddr>,
    save_rx: Option<Arc<Mutex<tokio::sync::mpsc::Receiver<gstreamer::Buffer>>>>,
    local_sos: SignalOfStop,
}

impl WorkerClose for Receiver {
    fn close(&mut self) {
        self.local_sos.cancel();
        self.save_stop();
    }
}

impl Receiver {
    pub fn new(sos: SignalOfStop) -> Self {
        Receiver {
            is_streaming: Arc::new(Mutex::new(false)),
            save_stream: None,
            caster_addr: None,
            save_rx: None,
            local_sos: sos,
        }
    }

    pub fn set_caster_addr(&mut self, addr: SocketAddr) {
        self.caster_addr = Some(addr);
    }

    pub fn launch(&mut self) -> Option<Pipeline> {
        let caster_addr = self.caster_addr;

        let (save_tx, save_rx) = tokio::sync::mpsc::channel(FRAME_RATE as usize);

        self.save_rx = Some(Arc::new(Mutex::new(save_rx)));

        let sos = self.local_sos.clone();

        let pipeline: Option<Pipeline> = if USE_WEBRTC {
            let (tx, rx) = tokio::sync::mpsc::channel(1);
            let is_streaming = self.is_streaming.clone();
            tokio::spawn(async move {
                *is_streaming.lock().await = crate::utils::net::webrtc::receiver(caster_addr, tx, sos).await;
            });
            result_to_option(crate::utils::gist::create_rtp_view_pipeline(rx, save_tx))
        } else {
            let (tx, rx) = tokio::sync::mpsc::channel(1);
            tokio::spawn(async move {
                crate::utils::net::xgp::receiver(caster_addr, tx).await;
            });
            result_to_option(crate::utils::gist::create_view_pipeline(rx, save_tx))
        };

        pipeline
    }

    pub fn is_saving(&self) -> bool {
        if let Some(save_stream) = &self.save_stream {
            save_stream.is_saving()
        } else {
            false
        }
    }

    pub fn is_streaming(&self) -> bool {
        *self.is_streaming.blocking_lock()
    }

    pub fn save_stream(&mut self) {
        if let Some(saver_channel) = &self.save_rx {
            let mut stream_saver = SaveStream::new(Arc::clone(saver_channel));
            stream_saver.start();
            self.save_stream = Some(stream_saver);
        }
    }

    pub fn save_stop(&mut self) {
        if let Some(mut save_stream) = self.save_stream.take() {
            save_stream.stop();
        }
    }
}
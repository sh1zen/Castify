use crate::assets::{FRAME_RATE, USE_WEBRTC};
use crate::utils::handle_result;
use crate::workers::save_stream::SaveStream;
use crate::workers::WorkerClose;
use gstreamer::Pipeline;
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Debug)]
pub struct Receiver {
    save_stream: Option<SaveStream>,
    caster_addr: Option<SocketAddr>,
    save_rx: Option<Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<gstreamer::Buffer>>>>,
}

impl WorkerClose for Receiver {
    fn close(&mut self) {
        self.save_stop();
    }
}

impl Receiver {
    pub fn new() -> Self {
        Receiver {
            save_stream: None,
            caster_addr: None,
            save_rx: None,
        }
    }

    pub fn set_caster_addr(&mut self, addr: SocketAddr) {
        self.caster_addr = Some(addr);
    }

    pub fn launch(&mut self) -> Option<Pipeline> {
        let caster_addr = self.caster_addr;

        let (save_tx, save_rx) = tokio::sync::mpsc::channel(FRAME_RATE as usize);

        self.save_rx = Some(Arc::new(tokio::sync::Mutex::new(save_rx)));

        let pipeline: Option<Pipeline> = if USE_WEBRTC {
            let (tx, rx) = tokio::sync::mpsc::channel(1);
            tokio::spawn(async move {
                crate::utils::net::webrtc::receiver(caster_addr, tx).await;
            });
            handle_result(crate::utils::gist::create_rtp_view_pipeline(rx, save_tx))
        } else {
            let (tx, rx) = tokio::sync::mpsc::channel(1);
            tokio::spawn(async move {
                crate::utils::net::xgp::receiver(caster_addr, tx).await;
            });
            handle_result(crate::utils::gist::create_view_pipeline(rx, save_tx))
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
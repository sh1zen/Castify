use crate::assets::FRAME_RATE;
use crate::utils::net::common::find_caster;
use crate::utils::net::webrtc::WebRTCReceiver;
use crate::utils::result_to_option;
use crate::utils::sos::SignalOfStop;
use crate::workers::save_stream::SaveStream;
use crate::workers::WorkerClose;
use gstreamer::Pipeline;
use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Receiver {
    is_streaming: Arc<AtomicBool>,
    save_stream: Option<SaveStream>,
    caster_addr: Option<SocketAddr>,
    save_rx: Option<Arc<Mutex<tokio::sync::mpsc::Receiver<gstreamer::Buffer>>>>,
    local_sos: SignalOfStop,
    handler: Arc<WebRTCReceiver>,
}

impl Receiver {
    pub fn new(sos: SignalOfStop) -> Self {
        Receiver {
            is_streaming: Arc::new(AtomicBool::new(false)),
            save_stream: None,
            caster_addr: None,
            save_rx: None,
            local_sos: sos.clone(),
            handler: Arc::new(WebRTCReceiver::new()),
        }
    }

    pub fn set_caster_addr(&mut self, addr: SocketAddr) {
        self.caster_addr = Some(addr);
    }

    pub fn launch(&mut self, auto: bool) -> Option<Pipeline> {
        let (save_tx, save_rx) = tokio::sync::mpsc::channel(FRAME_RATE as usize);

        self.save_rx = Some(Arc::new(Mutex::new(save_rx)));

        let pipeline: Option<Pipeline> = {
            let (tx, rx) = tokio::sync::mpsc::channel(1);

            let is_streaming = Arc::clone(&self.is_streaming);
            let mut caster_addr = self.caster_addr;
            let handler = Arc::clone(&self.handler);

            tokio::spawn(async move {
                if auto {
                    if caster_addr.is_none() {
                        caster_addr = find_caster();
                    }

                    if let Some(socket_addr) = caster_addr {
                        let addr: &str = &format!("ws://{}", &(socket_addr.to_string()));

                        println!("Connecting to caster at {:?}", addr);

                        if handler.connect(addr).await.is_err() {
                            // todo handle error
                        }
                    }
                }

                if handler.is_connected().await {
                    is_streaming.store(true, std::sync::atomic::Ordering::Relaxed);
                    handler.receive_video(tx).await;
                }
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
        self.is_streaming.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn save_stream(&mut self, path: String) {
        if let Some(saver_channel) = &self.save_rx {
            let mut stream_saver = SaveStream::new(Arc::clone(saver_channel));
            stream_saver.start(path);
            self.save_stream = Some(stream_saver);
        }
    }

    pub fn save_stop(&mut self) {
        if let Some(mut save_stream) = self.save_stream.take() {
            save_stream.stop();
        }
    }

    pub fn get_connection_handler(&self) -> Arc<WebRTCReceiver> {
        Arc::clone(&self.handler)
    }
}


impl WorkerClose for Receiver {
    fn close(&mut self) {
        self.save_stop();
        let handler = Arc::clone(&self.handler);
        tokio::spawn(async move {
            handler.close().await;
        });
        self.local_sos.cancel();
    }
}
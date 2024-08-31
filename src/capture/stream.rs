use crate::capture::Capture;
use crate::gui::resource::SAMPLING_RATE;
use crate::workers;
use image::RgbaImage;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{interval, Interval};

#[derive(Debug, Clone)]
pub struct Streamer {
    capture: Arc<Mutex<Capture>>,
    tx: Arc<Mutex<mpsc::Sender<RgbaImage>>>,
    interval: Arc<Mutex<Interval>>,
}

unsafe impl Send for Streamer {}

impl Streamer {
    pub async fn stream(capture: Arc<Mutex<Capture>>, tx: mpsc::Sender<RgbaImage>) {
        let interval = Arc::new(Mutex::new(interval(Duration::from_secs_f32(1.0 / SAMPLING_RATE as f32))));

        let tx = Arc::new(Mutex::new(tx));

        let st = Streamer {
            capture,
            tx,
            interval,
        };

        for i in 0..3 {
            let mut tr = st.clone();
            tokio::spawn(async move {
                tr.threaded_stream(i).await;
            });
        }
    }

    async fn threaded_stream(&mut self, id: u32) {
        loop {
            self.interval.lock().await.tick().await;

            if !workers::caster::get_instance().lock().unwrap().streaming {
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue;
            }

            let monitor = workers::caster::get_instance().lock().unwrap().current_monitor();
            let blank = workers::caster::get_instance().lock().unwrap().is_blank_screen();

            let x_monitor = {
                let capture = self.capture.lock().await;
                capture.get_x_monitor(monitor).cloned()  // Use `cloned()` to extend lifetime
            };

            if let Some(x_monitor) = x_monitor {
                let frame = Capture::get_frame(&x_monitor, blank);
                if let Some(frame) = frame {
                    match self.tx.lock().await.try_send(frame.clone()) {
                        Err(TrySendError::Closed(_)) => {
                            eprintln!("Receiver channel dropped");
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

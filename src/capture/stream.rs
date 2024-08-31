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
    interval: Arc<Mutex<Interval>>,
    tx: Arc<Mutex<mpsc::Sender<RgbaImage>>>,
}


impl Streamer {
    pub async fn stream(capture: Arc<Mutex<Capture>>, tx: mpsc::Sender<RgbaImage>)
    {
        let tx = Arc::new(Mutex::new(tx));
        let interval = Arc::new(Mutex::new(interval(Duration::from_secs_f32(1.0 / SAMPLING_RATE as f32))));

        let mut st = Streamer {
            capture,
            interval,
            tx,
        };

        st.threaded_stream().await;
    }

    async fn threaded_stream(&mut self) {

        let interval = Arc::clone(&self.interval);
        let capture = Arc::clone(&self.capture);
        let tx = Arc::clone(&self.tx);

        let handle = tokio::spawn(async move {
            loop {
                interval.lock().await.tick().await;

                if !workers::caster::get_instance().lock().unwrap().streaming {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    continue;
                }

                let monitor = workers::caster::get_instance().lock().unwrap().current_monitor();
                let blank = workers::caster::get_instance().lock().unwrap().is_blank_screen();

                let frame = capture.lock().await.get_frame(
                    monitor,
                    blank,
                );

                if let Some(frame) = frame {
                    match tx.lock().await.try_send(frame.clone()) {
                        Err(TrySendError::Closed(_)) => {
                            eprintln!("Receiver channel dropped");
                            break;
                        }
                        _ => {}
                    }
                }
            }
        });

        handle.await.unwrap();
    }
}
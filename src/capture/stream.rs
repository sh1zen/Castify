use crate::capture::Capture;
use crate::gui::resource::SAMPLING_RATE;
use crate::workers;
use image::RgbaImage;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::{mpsc, Mutex};
use tokio::time::interval;

pub struct Streamer {}


impl Streamer {
    pub async fn stream(capture: Arc<Mutex<Capture>>, tx: mpsc::Sender<RgbaImage>)
    {
        let interval = interval(Duration::from_secs_f32(1.0 / SAMPLING_RATE as f32));
        tokio::pin!(interval);

        loop {
            interval.as_mut().tick().await;

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
                match tx.try_send(frame.clone()) {
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
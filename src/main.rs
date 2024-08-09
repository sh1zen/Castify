mod movie_capture;

use std::time::{Duration, Instant};
use std::thread;
use std::sync::{mpsc, Arc, Mutex};
use xcap::Monitor;
use image::imageops::FilterType;
use eframe::{egui, epi};

struct AppState {
    duration: Duration,
    interval: Duration,
    screenshot_dir: std::path::PathBuf,
    is_recording: Arc<Mutex<bool>>,
    sender: Option<mpsc::Sender<()>>,
}

impl AppState {
    fn new(duration: Duration, interval: Duration) -> Self {
        Self {
            duration,
            interval,
            screenshot_dir: std::path::PathBuf::from("target"),
            is_recording: Arc::new(Mutex::new(false)),
            sender: None,
        }
    }

    fn start_recording(&mut self) {
        if *self.is_recording.lock().unwrap() {
            return; // Già in fase di registrazione
        }

        *self.is_recording.lock().unwrap() = true;
        std::fs::create_dir_all(&self.screenshot_dir).expect("Failed to create target directory");

        let (sender, receiver) = mpsc::channel();
        self.sender = Some(sender);

        let is_recording = Arc::clone(&self.is_recording);
        let screenshot_dir = self.screenshot_dir.clone();
        let interval = self.interval;
        let duration = self.duration;

        thread::spawn(move || {
            let start = Instant::now();
            let mut frame_number = 0;

            while *is_recording.lock().unwrap() && start.elapsed() < duration {
                let capture_start = Instant::now();
                let monitors = Monitor::all().unwrap();

                for monitor in &monitors {
                    let image = monitor.capture_image().unwrap();
                    let resized_image = image::imageops::resize(&image, image.width() / 4, image.height() / 4, FilterType::Nearest);
                    let file_path = screenshot_dir.join(format!("monitor-{:05}.png", frame_number));
                    resized_image.save(&file_path).unwrap();
                    println!("Saved screenshot: {:?}", file_path);
                }

                frame_number += 1;
                let capture_duration = capture_start.elapsed();

                if interval > capture_duration {
                    thread::sleep(interval - capture_duration);
                }

                if receiver.try_recv().is_ok() {
                    break; // Ricevuto segnale di stop
                }
            }

            let total_duration = start.elapsed();
            println!("Total duration: {:?}", total_duration);
            let num_images = std::fs::read_dir(&screenshot_dir).unwrap().count();
            println!("Number of images in target directory: {}", num_images);
            if num_images < 300 {
                println!("Not enough images to create a video. Expected at least 300, but found {}.", num_images);
            } else {
                movie_capture::create_video_from_screenshots(1920, 1080, num_images as u32, interval.as_millis() as u32);
            }

            *is_recording.lock().unwrap() = false;
        });
    }

    fn stop_recording(&mut self) {
        if let Some(sender) = &self.sender {
            let _ = sender.send(()); // Invia segnale di stop
        }
    }
}
impl epi::App for AppState {
    fn name(&self) -> &str {
        "Screen Recorder"
    }

    fn update(&mut self, ctx: &egui::CtxRef, _frame: &epi::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Start Recording").clicked() {
                self.start_recording();
            }

            if ui.button("Stop Recording").clicked() {
                self.stop_recording();
            }

            if *self.is_recording.lock().unwrap() {
                ui.label("Recording...");
            } else {
                ui.label("Not recording.");
            }
        });
    }
}

fn main() {
    let duration = Duration::from_secs(30);
    let interval = Duration::from_millis(100); // Intervallo di cattura
    let app_state = AppState::new(duration, interval);

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        Box::new(app_state),
        native_options,
    );
}

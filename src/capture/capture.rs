use crate::gui::resource::{FRAME_HEIGHT, FRAME_WITH};
use crate::workers;
use chrono::{DateTime, Local};
use image::{GenericImage, GenericImageView, Rgba, RgbaImage};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;
use tokio::time::interval;
use xcap::image::imageops::FilterType;
use xcap::{image, Monitor};

#[derive(Debug, Clone)]
pub struct XMonitor {
    x: i32,
    y: i32,
    height: u32,
    width: u32,
    pub monitor: Monitor,
}

#[derive(Debug, Clone)]
pub struct Capture {
    monitors: HashMap<u32, XMonitor>,
    framerate: f32,
    pub main: u32,
    pub region: Option<(i32, i32, u32, u32)>,
}

impl Capture {
    pub fn new() -> Capture {
        let monitors: HashMap<u32, XMonitor> = Self::get_monitors();
        let main = Self::get_main();

        Capture {
            monitors,
            framerate: 24.0,
            main,
            region: None,
        }
    }

    pub fn resize(&mut self, id: u32, x: i32, y: i32, width: u32, height: u32) {
        if !self.monitors.contains_key(&id) {
            return;
        }

        self.monitors.get_mut(&id).unwrap().x = x;
        self.monitors.get_mut(&id).unwrap().y = y;
        self.monitors.get_mut(&id).unwrap().width = width;
        self.monitors.get_mut(&id).unwrap().height = height;

        Monitor::from_point(x, y).unwrap();
    }

    pub fn set_region(&mut self, x: i32, y: i32, width: u32, height: u32) {
        self.region = Some((x, y, width, height));
    }

    pub fn clear_region(&mut self) {
        self.region = None;
    }

    pub fn screen(&self, id: u32) {
        let now: DateTime<Local> = Local::now();

        if id > 0 {
            if !self.monitors.contains_key(&id) {
                println!("Out of bound monitor {}", id);
                return;
            }
            let monitor = &self.monitors.get(&id).unwrap().monitor;
            self.frame(monitor)
                .save(format!(
                    "target/monitor-{}-{}.png",
                    normalized(monitor.name()),
                    now.timestamp().to_string()
                ))
                .unwrap();
        } else {
            for (_, monitor) in self.monitors.iter() {
                self.frame(&monitor.monitor).save(format!(
                    "target/monitor-{}-{}.png",
                    normalized(monitor.monitor.name()),
                    now.timestamp().to_string()
                )).unwrap();
            }
        }
    }

    pub fn get_frame(&self, id: u32, blank: bool) -> Option<RgbaImage>
    {
        if self.monitors.contains_key(&id) {
            let monitor = &self.monitors.get(&id)?.monitor;
            let mut frame;

            if blank {
                frame = RgbaImage::new(monitor.width(), monitor.height());
                for pixel in frame.pixels_mut() {
                    *pixel = Rgba([255, 255, 255, 255]);
                }

                println!("Blank Frame {}", Local::now().timestamp_millis());
            } else {

                if let Some((x, y, width, height)) = self.region {
                    let mut full_image = monitor.capture_image().unwrap();
                    let sub_image = image::imageops::crop(&mut full_image, x as u32, y as u32, width as u32, height as u32);
                    frame = sub_image.to_image();
                } else {
                    frame = self.frame(monitor);
                }

                println!("Captured Frame {}", Local::now().timestamp_millis());

                frame = resize_and_pad(
                    &frame,
                    FRAME_WITH as u32,
                    FRAME_HEIGHT as u32,
                    FilterType::Lanczos3,
                );
            }

            Some(frame)
        } else {
            None
        }
    }

    pub async fn stream(&mut self, id: u32, tx: mpsc::Sender<RgbaImage>) {
        self.stream_internal(id, tx, None).await;
    }

    pub async fn stream_area(&mut self, id: u32, area: (i32, i32, u32, u32), tx: mpsc::Sender<RgbaImage>) {
        self.stream_internal(id, tx, Some(area)).await;
    }

    async fn stream_internal(&mut self, id: u32, tx: mpsc::Sender<RgbaImage>, area: Option<(i32, i32, u32, u32)>) {
        let interval = interval(Duration::from_secs_f32(1.0 / self.framerate));
        tokio::pin!(interval);

        loop {
            interval.as_mut().tick().await;

            if !workers::caster::get_instance().lock().unwrap().streaming {
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue;
            }

            if let Some((x, y, width, height)) = area {
                self.set_region(x, y, width, height);
            } else {
                self.clear_region();
            }

            let frame = self.get_frame(
                if id != 0 { id } else {
                    workers::caster::get_instance().lock().unwrap().monitor.clone()
                },
                workers::caster::get_instance().lock().unwrap().is_blank_screen(),
            );

            if !frame.is_none() {
                match tx.try_send(frame.unwrap().clone()) {
                    Err(TrySendError::Closed(_)) => {
                        eprintln!("Receiver channel dropped");
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn set_framerate(&mut self, framerate: f32) {
        self.framerate = framerate;
    }

    fn frame(&self, monitor: &Monitor) -> RgbaImage {
        monitor.capture_image().unwrap()
    }

    pub fn has_monitor(&self, id: u32) -> bool {
        self.monitors.contains_key(&id)
    }

    pub fn get_monitors() -> HashMap<u32, XMonitor> {
        let mut monitors = HashMap::new();

        for monitor in Monitor::all().unwrap() {
            monitors.insert(monitor.id(), XMonitor {
                x: 0,
                y: 0,
                height: monitor.height(),
                width: monitor.width(),
                monitor,
            });
        }
        monitors
    }

    pub fn get_main() -> u32 {
        let mut main = 0;
        for monitor in Monitor::all().unwrap() {
            if monitor.is_primary() {
                main = monitor.id();
                break;
            }
        }
        main
    }
}


fn normalized(filename: &str) -> String {
    filename
        .replace("|", "")
        .replace("\\", "")
        .replace(":", "")
        .replace("/", "")
}

/// Resize an image to the specified width and height while maintaining aspect ratio,
/// and pad with black borders if necessary.
fn resize_and_pad(image: &RgbaImage, new_width: u32, new_height: u32, filter: FilterType) -> RgbaImage {
    // Calculate the aspect ratio of the original image
    let (orig_width, orig_height) = image.dimensions();
    let aspect_ratio = orig_width as f32 / orig_height as f32;

    // Calculate the new dimensions that fit within the desired size while maintaining aspect ratio
    let (resize_width, resize_height) = if new_width as f32 / new_height as f32 > aspect_ratio {
        // Fit by height
        let height = new_height;
        let width = (new_height as f32 * aspect_ratio).round() as u32;
        (width, height)
    } else {
        // Fit by width
        let width = new_width;
        let height = (new_width as f32 / aspect_ratio).round() as u32;
        (width, height)
    };

    if resize_width == orig_width && resize_height == orig_height {
        return image.to_owned();
    }

    // Resize the image to the calculated dimensions
    let resized_image = image::imageops::resize(
        image,
        resize_width,
        resize_height,
        filter,
    );

    // Create a new image with the specified dimensions and black background
    let mut padded_image = RgbaImage::new(new_width, new_height);
    let black = Rgba([0, 0, 0, 255]);

    // Fill the new image with black
    for pixel in padded_image.pixels_mut() {
        *pixel = black;
    }

    // Calculate the position to place the resized image to center it
    let x_offset = (new_width - resize_width) / 2;
    let y_offset = (new_height - resize_height) / 2;

    // Overlay the resized image onto the black background
    padded_image.copy_from(&resized_image, x_offset, y_offset).unwrap();

    padded_image
}
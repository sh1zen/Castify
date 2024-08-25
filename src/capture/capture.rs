use crate::gui::resource::{FRAME_HEIGHT, FRAME_WITH};
use crate::workers;
use chrono::{DateTime, Local};
use image::{GenericImage, Rgba, RgbaImage};
use iced::Rectangle;
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
    capture_regions: HashMap<u32, Option<Rectangle>>, // Add for saving the capture region
}

impl Capture {
    pub fn new() -> Capture {
        let monitors: HashMap<u32, XMonitor> = Self::get_monitors();
        let main = Self::get_main();

        Capture {
            monitors,
            framerate: 24.0,
            main,
            capture_regions: HashMap::new(), // Setting the map of regions
        }
    }

    //Add a method for setting the capture region
    pub fn set_capture_region(&mut self, id: u32, region: Option<Rectangle>) {
        self.capture_regions.insert(id, region);
    }

    pub fn get_frame(&self, id: u32, blank: bool) -> Option<RgbaImage> {
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
                frame = self.frame(monitor);

                // its already setting (the region)?
                if let Some(region) = self.capture_regions.get(&id).unwrap_or(&None) {
                    if let Some(region) = region {
                        frame = Self::crop_to_region(&frame, region);
                    }
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

    // Method for cutting the region
    fn crop_to_region(image: &RgbaImage, region: &Rectangle) -> RgbaImage {
        image::imageops::crop(
            &mut image.clone(),
            region.x as u32,
            region.y as u32,
            region.width as u32,
            region.height as u32,
        )
            .to_image()
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

    pub async fn stream(&self, id: u32, tx: mpsc::Sender<RgbaImage>) {
        let interval = interval(Duration::from_secs_f32(1.0 / self.framerate));

        tokio::pin!(interval);

        loop {
            interval.as_mut().tick().await;

            if !workers::caster::get_instance().lock().unwrap().streaming {
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue;
            }

            let frame = self.get_frame(
                if id != 0 { id } else {
                    workers::caster::get_instance().lock().unwrap().monitor.clone()
                },
                workers::caster::get_instance().lock().unwrap().is_blank_screen(),
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

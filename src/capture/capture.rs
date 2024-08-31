use crate::gui::resource::{FRAME_HEIGHT, FRAME_WITH};
use chrono::{DateTime, Local};
use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
use std::collections::HashMap;
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

unsafe impl Send for XMonitor {}

#[derive(Debug, Clone)]
pub struct Capture {
    monitors: HashMap<u32, XMonitor>,
    pub main: u32,
}

impl Capture {
    pub fn new() -> Capture {
        let monitors: HashMap<u32, XMonitor> = Self::get_monitors();
        let main = Self::get_main();

        Capture {
            monitors,
            main,
        }
    }

    pub fn resize(&mut self, id: u32, mut x: i32, mut y: i32, mut width: u32, mut height: u32) {
        if !self.monitors.contains_key(&id) {
            return;
        }

        if x == 0 {
            x = self.monitors.get_mut(&id).unwrap().monitor.x();
        }
        if y == 0 {
            y = self.monitors.get_mut(&id).unwrap().monitor.y();
        }
        if width == 0 {
            width = self.monitors.get_mut(&id).unwrap().monitor.width();
        }
        if height == 0 {
            height = self.monitors.get_mut(&id).unwrap().monitor.height();
        }

        self.monitors.get_mut(&id).unwrap().x = x;
        self.monitors.get_mut(&id).unwrap().y = y;
        self.monitors.get_mut(&id).unwrap().width = width;
        self.monitors.get_mut(&id).unwrap().height = height;

        //println!("Monitor area resized {:?}", self.monitors.get_mut(&id).unwrap());
    }

    pub fn screen(&self, id: u32) {
        let now: DateTime<Local> = Local::now();

        if id > 0 {
            if !self.monitors.contains_key(&id) {
                println!("Out of bound monitor {}", id);
                return;
            }
            let monitor = &self.monitors.get(&id).unwrap().monitor;
            Self::frame(monitor)
                .save(format!(
                    "target/monitor-{}-{}.png",
                    normalized(monitor.name()),
                    now.timestamp().to_string()
                ))
                .unwrap();
        } else {
            for (_, monitor) in self.monitors.iter() {
                Self::frame(&monitor.monitor).save(format!(
                    "target/monitor-{}-{}.png",
                    normalized(monitor.monitor.name()),
                    now.timestamp().to_string()
                )).unwrap();
            }
        }
    }

    pub fn get_x_monitor(&self, id: u32) -> Option<&XMonitor> {
        if self.monitors.contains_key(&id) {
            self.monitors.get(&id)
        } else {
            None
        }
    }

    pub fn get_frame(x_monitor: &XMonitor, blank: bool) -> Option<RgbaImage> {
        let monitor = &x_monitor.monitor;
        let mut frame;
        if blank {
            frame = RgbaImage::new(x_monitor.width - x_monitor.x as u32, x_monitor.height - x_monitor.y as u32);
            for pixel in frame.pixels_mut() {
                *pixel = Rgba([255, 255, 255, 255]);
            }
            // println!("Blank Frame {}", Local::now().timestamp_millis());
        } else {
            frame = Self::frame(monitor);
            frame = Self::crop(frame, x_monitor.x as u32, x_monitor.y as u32, x_monitor.width, x_monitor.height);

            // println!("Captured Frame {}", Local::now().timestamp_millis());

            frame = resize_and_pad(
                &frame,
                FRAME_WITH as u32,
                FRAME_HEIGHT as u32,
                FilterType::Nearest,
            );
        }
        Some(frame)
    }

    fn crop(frame: RgbaImage, x: u32, y: u32, w: u32, h: u32) -> RgbaImage {
        DynamicImage::ImageRgba8(frame).view(x, y, w, h).to_image()
    }

    fn frame(monitor: &Monitor) -> RgbaImage {
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

    let (resize_width, resize_height) = if new_width as f32 / new_height as f32 > aspect_ratio {
        let height = new_height;
        let width = (new_height as f32 * aspect_ratio).round() as u32;
        (width, height)
    } else {
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

    // Create a new image with the specified dimensions and black background using imageproc
    let mut padded_image = RgbaImage::new(new_width, new_height);
    //imageproc::draw_filled_rect_mut(&mut padded_image, Rect::at(0, 0).of_size(new_width, new_height), Rgba([0, 0, 0, 255]));

    // Calculate the position to place the resized image to center it
    let x_offset = (new_width - resize_width) / 2;
    let y_offset = (new_height - resize_height) / 2;

    // Overlay the resized image onto the black background
    image::imageops::overlay(&mut padded_image, &resized_image, x_offset as i64, y_offset as i64);

    padded_image
}
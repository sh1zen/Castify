use crate::gui::resource::{FRAME_HEIGHT, FRAME_WITH};
use chrono::{DateTime, Local};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;
use xcap::image::imageops::FilterType;
use xcap::image::{DynamicImage, GenericImageView, RgbaImage};
use xcap::{image, Monitor};

struct MonitorArea {
    x: i32,
    y: i32,
    height: u32,
    width: u32,
}

pub struct Capture {
    monitors: HashMap<u32, Monitor>,
    area: HashMap<u32, MonitorArea>,
    framerate: f32,
    pub(crate) main: u32,
}

impl Capture {
    pub fn new() -> Capture {
        let mut monitors_area: HashMap<u32, MonitorArea> = HashMap::new();
        let mut monitors: HashMap<u32, Monitor> = HashMap::new();
        let mut main = 0;

        for monitor in Monitor::all().unwrap() {
            if main == 0 {
                main = monitor.id();
            }

            monitors.insert(monitor.id().clone(), monitor.clone());

            monitors_area.insert(monitor.id().clone(), MonitorArea {
                x: 0,
                y: 0,
                height: monitor.height().clone(),
                width: monitor.width().clone(),
            });
        }

        Capture {
            monitors,
            area: monitors_area,
            framerate: 18.0,
            main,
        }
    }

    pub fn resize(&mut self, id: u32, x: i32, y: i32, width: u32, height: u32) {
        if !self.area.contains_key(&id) {
            return;
        }

        *self.area.get_mut(&id).unwrap() = MonitorArea {
            x,
            y,
            height,
            width,
        };

        Monitor::from_point(x, y).unwrap();
    }

    pub fn screen(&self, id: u32) {
        let now: DateTime<Local> = Local::now();

        if id > 0 {
            if !self.monitors.contains_key(&id) {
                println!("Out of bound monitor {}", id);
                return;
            }
            let monitor = self.monitors.get(&id).unwrap();
            self.frame(monitor)
                .save(format!(
                    "target/monitor-{}-{}.png",
                    normalized(monitor.name()),
                    now.timestamp().to_string()
                ))
                .unwrap();
        } else {
            for (id, monitor) in self.monitors.iter() {
                self.frame(&monitor).save(format!(
                    "target/monitor-{}-{}.png",
                    normalized(monitor.name()),
                    now.timestamp().to_string()
                )).unwrap();
            }
        }
    }

    pub fn get_frame(&self, id: u32) -> Option<RgbaImage>
    {
        if self.monitors.contains_key(&id) {
            let monitor = self.monitors.get(&id)?;

            let mut frame = self.frame(monitor);

            // todo use resize and pad
            frame = image::imageops::resize(
                &frame,
                FRAME_WITH as u32,
                FRAME_HEIGHT as u32,
                FilterType::Nearest,
            );

            println!("Captured Frame");

            Some(frame)
        } else {
            None
        }
    }

    pub async fn stream(&self, id: u32, tx: mpsc::Sender<RgbaImage>) {
        let interval = interval(Duration::from_secs_f32(1.0 / self.framerate));

        tokio::pin!(interval);

        if !self.monitors.contains_key(&id) {
            // todo add error
        }

        loop {
            interval.as_mut().tick().await;

            let frame = self.get_frame(id);

            if !frame.is_none() {
                if tx.send(frame.unwrap().clone()).await.is_err() {
                    eprintln!("Receiver dropped");
                    break;
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
}

fn normalized(filename: &str) -> String {
    filename
        .replace("|", "")
        .replace("\\", "")
        .replace(":", "")
        .replace("/", "")
}


fn resize_and_pad(img_base: &DynamicImage, target_width: u32, target_height: u32, filter: FilterType) {
    /*
        let aspect_ratio = img_base.width() as f32 / img_base.height() as f32;

        let new_width;
        let new_height;

        if aspect_ratio > (target_width as f32 / target_height as f32) {
            new_width = target_width;
            new_height = (target_width as f32 / aspect_ratio) as u32;
        } else {
            new_width = (target_height as f32 * aspect_ratio) as u32;
            new_height = target_height;
        }

        // add gaussian blur filter to reduce noise
        let img = img_base.blur(2.0);

        // Ridimensiona l'immagine
        let resized_img = img.resize(new_width, new_height, filter);

        // Crea una nuova immagine con sfondo nero
        let mut padded_img = ImageBuffer::new(target_width, target_height);
        padded_img.fill(&image::Rgba([0, 0, 0, 255]));

        // Calcola le coordinate per centrare l'immagine ridimensionata
        let x_offset = (target_width - new_width) / 2;
        let y_offset = (target_height - new_height) / 2;

        // Copia l'immagine ridimensionata nella nuova immagine
        for y in 0..new_height {
            for x in 0..new_width {
                let pixel = resized_img.get_pixel(x, y);
                padded_img.put_pixel(x + x_offset, y + y_offset, &pixel);
            }
        }

        DynamicImage::ImageRgb8(padded_img)
     */
}
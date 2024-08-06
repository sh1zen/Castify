mod movie_capture;

use std::time::{Duration, Instant};
use std::thread::sleep;
use xcap::Monitor;
use std::path::Path;
use image::GenericImageView;

fn main() {
    // Specifica la durata della registrazione e l'intervallo di cattura degli screenshot
    let duration = Duration::from_secs(30); // Durata totale della registrazione (es. 10 secondi)
    let interval = Duration::from_millis(500); // Intervallo tra gli screenshot (es. 500 ms)

    let start = Instant::now();
    let monitors = Monitor::all().unwrap();
    let mut frame_number = 0;

    let screenshot_dir = Path::new("target");
    std::fs::create_dir_all(&screenshot_dir).expect("Failed to create target directory");

    while start.elapsed() < duration {
        for monitor in &monitors {
            let image = monitor.capture_image().unwrap();
            let file_path = screenshot_dir.join(format!("monitor-{:05}.png", frame_number));
            image.save(&file_path).unwrap();
            println!("Saved screenshot: {:?}", file_path);
        }
        frame_number += 1;
        sleep(interval);
    }

    println!("Total duration: {:?}", start.elapsed());

    // Verifica che i file siano presenti nella directory target
    for entry in std::fs::read_dir(&screenshot_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            println!("File in target directory: {:?}", path);
        }
    }

    // Passa le dimensioni dell'immagine al modulo movie_capture
    let sample_image_path = screenshot_dir.join("monitor-00000.png");
    let (width, height) = image::open(&sample_image_path).unwrap().dimensions();

    // Verifica che ci siano almeno 2 immagini nella directory target
    let num_images = std::fs::read_dir(&screenshot_dir).unwrap().count();
    println!("Number of images in target directory: {}", num_images);

    if num_images < 2 {
        eprintln!("Not enough images to create a video");
        return;
    }

    movie_capture::create_video_from_screenshots(width, height);
}

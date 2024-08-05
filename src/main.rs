mod movie_capture;

use std::time::{Duration, Instant};
use std::thread::sleep;
use xcap::Monitor;
use std::path::Path;

fn main() {
    // Specifica la durata della registrazione e l'intervallo di cattura degli screenshot
    let duration = Duration::from_secs(10); // Durata totale della registrazione (es. 10 secondi)
    let interval = Duration::from_millis(500); // Intervallo tra gli screenshot (es. 1000 ms)

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

    movie_capture::create_video_from_screenshots();
}


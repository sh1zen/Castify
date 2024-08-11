use chrono::{DateTime, Local};
use xcap::Monitor;

pub struct Capture {
    x: i32,
    y: i32,
    height: i32,
    width: i32,
}

impl Capture {
    pub fn new() -> Capture {
        Capture {
            x: 0,
            y: 0,
            height: 0,
            width: 0,
        }
    }

    pub fn resize(&mut self, x: i32, y: i32, width: i32, height: i32) {
        self.x = x;
        self.y = y;
        self.width = width;
        self.height = height;
        Monitor::from_point(self.x, self.y);
    }
}
impl Capture {
    pub fn screen(&self, monitor_n: usize) {
        let monitors = Monitor::all().unwrap();

        if monitor_n > 0 {
            let monitor = &monitors[monitor_n];
            self.frame(monitor)
        } else {
            for monitor in monitors {
                self.frame(&monitor)
            }
        }
    }

    fn frame(&self, monitor: &Monitor) {
        let now: DateTime<Local> = Local::now();

        println!("{:?} {:?}", monitor.width(), monitor.height());
        let image = monitor.capture_image().unwrap();

        image
            .save(format!(
                "target/monitor-{}-{}.png",
                normalized(monitor.name()),
                now.timestamp().to_string()
            ))
            .unwrap();
    }
}

fn normalized(filename: &str) -> String {
    filename
        .replace("|", "")
        .replace("\\", "")
        .replace(":", "")
        .replace("/", "")
}

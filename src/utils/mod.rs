use iced_core::Point;

pub mod gist;
pub mod net;
pub mod tray_icon;
pub mod key_listener;

pub fn get_string_after(s: String, c: char) -> String {
    let index = s.find(c);
    if index.is_none(){
        return s;
    }
    s.clone().split_off(index.unwrap() + 1)
}
pub fn open_link(web_page: &String) {
    let url = web_page;
    #[cfg(target_os = "windows")]
    std::process::Command::new("explorer")
        .arg(url)
        .spawn()
        .unwrap();
    #[cfg(target_os = "macos")]
    std::process::Command::new("open").arg(url).spawn().unwrap();
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    std::process::Command::new("xdg-open")
        .arg(url)
        .spawn()
        .unwrap();
}

pub fn format_seconds(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;

    format!("{}:{:02}:{:02}", hours, minutes, seconds)
}

pub fn evaluate_points(point_a: Point, point_b: Point) -> (Point, Point) {
    let (mut start, mut end) = (point_a, point_b);
    if point_a.x > point_b.x {
        (start.x, end.x) = (point_b.x, point_a.x)
    };
    if point_a.y > point_b.y {
        (start.y, end.y) = (point_b.y, point_a.y)
    };

    (start, end)
}
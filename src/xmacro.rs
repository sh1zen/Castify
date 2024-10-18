#[macro_export]
macro_rules! rgb8 {
    ($r:expr, $g:expr, $b:expr) => {
        iced::Color::from_rgb($r as f32 / 255.0, $g as f32 / 255.0, $b as f32 / 255.0)
    };
}

#[macro_export]
macro_rules! rgba8 {
    ($r:expr, $g:expr, $b:expr, $a:expr) => {
        iced::Color::from_rgba($r as f32 / 255.0, $g as f32 / 255.0, $b as f32 / 255.0, $a as f32)
    };
}
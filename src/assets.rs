use iced::font::{Family, Stretch, Style, Weight};
use iced::Font;

pub const FRAME_RATE: i32 = 30;
pub const FRAME_WITH: i32 = 1920;
pub const FRAME_HEIGHT: i32 = 1080;

// connections
pub const MAX_PACKAGES_FAIL: u8 = 5;
pub const CAST_SERVICE_PORT: u16 = 31413;

#[cfg(target_os = "windows")]
pub const TARGET_OS: &str = "windows";
#[cfg(target_os = "macos")]
pub const TARGET_OS: &str = "macos";
#[cfg(target_os = "linux")]
pub const TARGET_OS: &str = "linux";

pub const ICON_BYTES: &[u8] = include_bytes!("../resources/icons/96x96.png");

pub const FONT_AWESOME_BYTES: &[u8] = include_bytes!("../resources/Font Awesome 6 Free-Solid-900.otf");
pub const FONT_AWESOME: Font = Font {
    family: Family::Name("Font Awesome 6 Free"),
    weight: Weight::Black,
    stretch: Stretch::Normal,
    style: Style::Normal,
};

// font text base
pub const FONT_BASE_BYTES: &[u8] = include_bytes!("../resources/Raleway-VariableFont.ttf");

pub const FONT_FAMILY_BASE: Font = Font {
    family: Family::Name("Raleway"),
    weight: Weight::Normal,
    stretch: Stretch::Normal,
    style: Style::Normal,
};

pub const FONT_FAMILY_BOLD: Font = Font {
    family: Family::Name("Raleway"),
    weight: Weight::Bold,
    stretch: Stretch::Normal,
    style: Style::Normal,
};


// border theme
pub const BORDER_WIDTH: f32 = 0.0;
pub const BORDER_RADIUS: f32 = 8.0;
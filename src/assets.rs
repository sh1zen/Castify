use iced_core::Font;
use iced_core::font::{Family, Stretch, Style, Weight};

pub const USE_WEBRTC: bool = true;

// conv
pub const SAMPLING_RATE: i32 = 30;

pub const FRAME_RATE: i32 = 30;
pub const FRAME_WITH: i32 = 1920;
pub const FRAME_HEIGHT: i32 = 1080;

// connections
pub const MAX_PACKAGES_FAIL: u8 = 5;
pub const CAST_SERVICE_PORT: u16 = 31413;

// app name
pub const APP_VERSION: &str = "1.0.0";
pub const APP_NAME: &str = "Castify";
pub const APP_NAME_ID: &str = "castify";

#[cfg(target_os = "windows")]
pub const TARGET_OS: &str = "windows";
#[cfg(target_os = "macos")]
pub const TARGET_OS: &str = "macos";
#[cfg(target_os = "linux")]
pub const TARGET_OS: &str = "linux";

#[cfg(target_os = "windows")]
pub const ICON_BYTES: &[u8] = include_bytes!("../resources/icons/96x96.png");


//font to display icons
pub const ICON_FONT_FAMILY_NAME: &str = "icons4screencaster";
pub const ICONS_BYTES: &[u8] = include_bytes!("../resources/icons4screencaster.ttf");
pub const ICONS: Font = Font {
    family: Family::Name(ICON_FONT_FAMILY_NAME),
    weight: Weight::Normal,
    stretch: Stretch::Normal,
    style: Style::Normal,
};

// font text base
pub const FONT_BASE_DATA: &[u8] = include_bytes!("../resources/Raleway-VariableFont.ttf");

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

// font style
pub const FONT_SIZE_BODY: f32 = 14.0;
pub const FONT_SIZE_FOOTER: f32 = 11.0;

// border theme
pub const BORDER_WIDTH: f32 = 0.0;
pub const BORDER_RADIUS: f32 = 8.0;
pub const BORDER_ALPHA: f32 = 0.0;
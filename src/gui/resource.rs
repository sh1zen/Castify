use iced::font::{Family, Stretch, Style, Weight};
use iced::Font;

// conv
pub const FRAME_RATE: i32 = 3;
pub const FRAME_WITH: i32 = 1920;
pub const FRAME_HEIGHT: i32 = 1080;

// connections
pub const MAX_PACKAGES_FAIL: u8 = 5;
pub const CAST_SERVICE_PORT: u16 = 31413;

// app name
pub const APP_VERSION: &str = "1.0.0";
pub const APP_NAME: &str = "Screen Caster";
pub const APP_NAME_ID: &str = "screen_caster";


//font to display icons
pub const ICON_FONT_FAMILY_NAME: &str = "icons4screencaster";
pub const ICONS_BYTES: &[u8] = include_bytes!("../../resources/icons4screencaster.ttf");
pub const ICONS: Font = Font {
    family: Family::Name(ICON_FONT_FAMILY_NAME),
    weight: Weight::Normal,
    stretch: Stretch::Normal,
    style: Style::Normal,
};

// font text base
pub const TEXT_FONT_FAMILY_NAME: &str = "Raleway";
pub const RALEWAY_FONT_BYTES: &[u8] =
    include_bytes!("../../resources/Raleway-Bold.ttf");

pub const RALEWAY_FONT: Font = Font {
    family: Family::Name(TEXT_FONT_FAMILY_NAME),
    weight: Weight::Normal,
    stretch: Stretch::Normal,
    style: Style::Normal,
};

// font style
pub const FONT_SIZE_BODY: f32 = 14.0;
pub const FONT_SIZE_FOOTER: f32 = 11.0;

// border styles
pub const BORDER_WIDTH: f32 = 0.0;
pub const COMPONENT_BORDER_RADIUS: f32 = 8.0;
pub const BORDER_ALPHA: f32 = 0.0;

// button
pub const BUTTON_ALPHA: f32 = 0.7;
pub const P_BUTTON_ALPHA: f32 = 1.0;
pub const H_BUTTON_ALPHA: f32 = 0.9;

// utils
pub fn get(file: String) -> String {
    format!("resources/{}", file)
}

pub(crate) fn open_link(web_page: &str) {
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

use crate::assets::FONT_AWESOME;
use crate::gui::style::theme::csx::StyleType;
use iced::widget::Text;

pub enum Icon {
    Home,
    Info,
    Warning,
    Error,
    Connection,
    Pause,
    Stop,
    Browser,
    Cast,
    Video,
    Image,
    Folder,
    Menu,
    Download,
    Settings,
    Save,
    Screen,
    Area,
    Banned,
    LightDarkMode,
    Auto,
    Connect,
    Pencil,
    Eraser,
    Square,
    Circle,
    PenRuler,
    Close,
    Droplet,
    DropletSlash,
    Minus,
    CircleHalf,
    User,
    Version,
    Copyright,
}

impl Icon {
    pub fn codepoint(&self) -> char {
        match self {
            Icon::Auto => '\u{e2ca}',
            Icon::Connect => '\u{f0c1}',
            Icon::Home => '\u{f015}',
            Icon::Info => '\u{f05a}',
            Icon::Error | Icon::Warning => '\u{f071}',
            Icon::Connection => '\u{f519}',
            Icon::Pause => '\u{f04c}',
            Icon::Stop => '\u{f04d}',
            Icon::Browser => '\u{f14e}',
            Icon::Cast => '\u{e595}',
            Icon::Video => '\u{f03d}',
            Icon::Image => '\u{f03e}',
            Icon::Folder => '\u{f07b}',
            Icon::Menu => '\u{f0c9}',
            Icon::Download => '\u{f019}',
            Icon::Settings => '\u{f1de}',
            Icon::Save => '\u{f0c7}',
            Icon::Screen => '\u{f26c}',
            Icon::Area => '\u{f065}',
            Icon::Banned => '\u{f05e}',
            Icon::LightDarkMode => '\u{f042}',
            Icon::Pencil => '\u{f303}',
            Icon::Eraser => '\u{f12d}',
            Icon::Square => '\u{f0c8}',
            Icon::Circle => '\u{f111}',
            Icon::PenRuler => '\u{f5ae}',
            Icon::Minus => '\u{f068}',
            Icon::Close => '\u{f00d}',
            Icon::Droplet => '\u{f043}',
            Icon::DropletSlash => '\u{f5c7}',
            Icon::CircleHalf => '\u{f042}',
            Icon::User => '\u{f007}',
            Icon::Version => '\u{f386}',
            Icon::Copyright => '\u{f1f9}'
        }
    }

    pub fn to_text<'a>(&self) -> Text<'a, StyleType> {
        Text::new(self.codepoint().to_string()).font(FONT_AWESOME)
    }
}
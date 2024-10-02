use crate::assets::ICONS;
use iced::widget::Text;
use crate::gui::style::styles::csx::StyleType;

#[allow(dead_code)]
pub enum Icon {
    Browser,
    Search,
    Like,
    Info,
    Warning,
    Error,
    Invalid,
    Connection,
    Pause,
    Stop,
    GitHub,
    Share,
    Cast,
    Bin,
    Mute,
    Speak,
    Banned,
    Group,
    Video,
    Image,
    Folder,
    Burger,
    Speed,
    Plus,
    Download,
    Settings,
    Save,
    Screen,
    Area,
}

impl Icon {
    pub fn codepoint(&self) -> char {
        match self {
            Icon::Browser => '1',
            Icon::Search => '2',
            Icon::Like => '3',
            Icon::Info => '5',
            Icon::Warning => '8',
            Icon::Error => '9',
            Icon::Invalid => 'a',
            Icon::Connection => 'Z',
            Icon::Pause => 'd',
            Icon::Stop => 'f',
            Icon::GitHub => 'g',
            Icon::Share => 'h',
            Icon::Cast => 'i',
            Icon::Bin => 'l',
            Icon::Mute => 'm',
            Icon::Speak => 'n',
            Icon::Banned => 'r',
            Icon::Group => 's',
            Icon::Video => 't',
            Icon::Image => 'u',
            Icon::Folder => 'v',
            Icon::Burger => 'J',
            Icon::Speed => 'P',
            Icon::Plus => 'T',
            Icon::Download => 'U',
            Icon::Settings => 'V',
            Icon::Save => 'Y',
            Icon::Screen => 'u',
            Icon::Area => 'E',
        }
    }

    pub fn to_text<'a>(&self) -> Text<'a, StyleType> {
        Text::new(self.codepoint().to_string()).font(ICONS)
    }
}
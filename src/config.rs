use crate::gui::components::hotkeys::KeyTypes;
use iced_core::keyboard::key::Named;
use iced_core::keyboard::{Key, Modifiers};
use iced_core::Size;
use crate::workers::caster::Caster;

#[derive(Clone, Debug)]
pub enum Mode {
    Caster(Caster),
    Client
}

#[derive(Clone, Debug)]
pub struct Config {
    pub(crate) hotkey_map: HotkeyMap,
    pub(crate) window_size: Size,
    pub dark_mode: bool,
    pub e_time: u64,
    pub mode: Option<Mode>
}

impl Default for HotkeyMap {
    fn default() -> Self {
        HotkeyMap {
            pause: (Modifiers::CTRL, Key::Named(Named::F10)),
            record: (Modifiers::CTRL, Key::Named(Named::F11)),
            end_session: (Modifiers::CTRL, Key::Character("w".parse().unwrap())),
            blank_screen: (Modifiers::CTRL, Key::Named(Named::F2)),
            updating: KeyTypes::None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HotkeyMap {
    pub pause: (Modifiers, Key),
    pub record: (Modifiers, Key),
    pub end_session: (Modifiers, Key),
    pub blank_screen: (Modifiers, Key),
    pub updating: KeyTypes,
}


impl Default for Config {
    fn default() -> Self {
        Config {
            hotkey_map: Default::default(),
            window_size: Size { width: 660f32, height: 440f32 },
            dark_mode: false,
            e_time: 0,
            mode: None,
        }
    }
}

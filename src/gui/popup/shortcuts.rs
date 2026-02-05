use crate::config::Config;
use crate::gui::common::hotkeys::KeyTypes;
use crate::gui::components::awmodal::GuiInterface;
use crate::gui::components::button::{IconButton, Key4Board};
use crate::gui::widget::{Column, Element, Row, Text};
use crate::gui::windows::main::MainWindowEvent;
use iced::keyboard::{Key, Modifiers};

pub struct ShortcutModal {
    key: KeyTypes,
}

impl ShortcutModal {
    pub fn new() -> Self {
        ShortcutModal {
            key: KeyTypes::None,
        }
    }

    pub fn set_key(mut self, key: KeyTypes) -> Self {
        self.key = key;
        self
    }
}

impl GuiInterface for ShortcutModal {
    type Message = MainWindowEvent;

    fn title(&self) -> String {
        format!("Updating hotkey for: {:?}", self.key)
    }

    fn view<'a, 'b>(&'a self, config: &Config) -> Element<'b, Self::Message>
    where
        'b: 'a,
        Self::Message: Clone + 'b,
    {
        let default = (Modifiers::empty(), Key::Unidentified);
        let c_key = match self.key {
            KeyTypes::Pause => &config.shortcuts.pause,
            KeyTypes::Record => &config.shortcuts.record,
            KeyTypes::Close => &config.shortcuts.end_session,
            KeyTypes::BlankScreen => &config.shortcuts.blank_screen,
            _ => &default,
        };

        Column::new()
            .spacing(12)
            .push(
                Row::new()
                    .push(Key4Board::from_command(&c_key.0).build())
                    .push(Key4Board::from_key(&c_key.1).build())
                    .spacing(5),
            )
            .push(Text::new("Press any desired key.").height(20).size(12))
            .push(
                IconButton::new()
                    .label("Ok")
                    .build()
                    .on_press(MainWindowEvent::ClosePopup(None)),
            )
            .into()
    }
}

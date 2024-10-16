use crate::assets::FONT_FAMILY_BOLD;
use crate::gui::style::button::ButtonType;
use crate::gui::widget::{Button, Container, Text};
use iced::keyboard::{Key, Modifiers};
use iced_core::{alignment, Padding};
use std::default::Default;

#[derive(Default)]
pub struct Key4Board {
    label: String,
    size: usize,
}

impl Key4Board {
    pub fn new(label: String, size: usize) -> Self {
        Self {
            label,
            size,
        }
    }

    pub fn get_label(&self) -> String {
        self.label.clone()
    }

    pub fn from_command(key: Modifiers) -> Key4Board {
        let label = if key == Modifiers::empty() {
            String::new()
        } else {
            format!("{:?}", key)
                .trim_start_matches("Modifiers(")
                .trim_end_matches(')')
                .to_string()
        };
        Key4Board::new(label, 3)
    }

    pub fn from_key(key: Key) -> Key4Board {
        let label = format!("{:?}", key)
            .trim_start_matches("Character(\"")
            .trim_start_matches("Named(")
            .trim_end_matches("\")")
            .trim_end_matches(')')
            .to_uppercase()
            .replace("SHIFT", "")
            .replace("CONTROL", "")
            .replace("ALT", "")
            .to_string();
        Key4Board::new(label, 2)
    }

    pub fn build<'a, Message: 'a>(self) -> Button<'a, Message>
    {
        Button::new(
            Container::new(Text::new(self.label.clone()).font(FONT_FAMILY_BOLD))
                .align_x(alignment::Horizontal::Center)
                .align_y(alignment::Vertical::Center)
        )
            .padding(Padding { top: 2.0, right: 8.0, bottom: 2.0, left: 8.0 })
            .height(40)
            .width(
                match self.size {
                    0 => 40,
                    1 => 60,
                    2 => 80,
                    3 => 100,
                    _ => 120
                }
            )
            .class(ButtonType::KeyBoard)
    }
}

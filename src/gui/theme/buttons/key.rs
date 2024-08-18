use std::default::Default;

use iced::advanced::widget::Text;
use iced::keyboard::{Key, Modifiers};
use iced::widget::{button, Button, Container};

use crate::gui::theme::styles::buttons::ButtonType;
use crate::gui::theme::styles::csx::StyleType;


#[derive(Default)]
pub struct Key4Board {
    label: String,
    style: ButtonType,
    size: usize,
}

impl Key4Board {
    pub fn new(label: String, size: usize) -> Self {
        Self {
            label,
            style: ButtonType::KeyBoard,
            size,
        }
    }

    pub fn from_command(key: Modifiers) -> Key4Board {
        let label = format!("{:?}", key)
            .trim_start_matches("Modifiers(")
            .trim_end_matches(')')
            .to_string();
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

    pub fn style(mut self, style: ButtonType) -> Self {
        self.style = style;
        self
    }

    pub fn build<'a, Message: 'a>(self) -> Button<'a, Message, StyleType>
    {
        button(
            Container::new(Text::new(self.label.clone()))
                .center_y()
                .center_x()
        )
            .padding([2, 8, 2, 8])
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
            .style(self.style)
    }
}

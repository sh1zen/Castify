use crate::gui::style::button::ButtonType;
use crate::gui::common::icons::Icon;
use crate::gui::widget::{Button, Container, Row, Space, Text};
use iced_core::alignment::{Horizontal, Vertical};
use iced_core::{Length, Padding};


pub struct FilledButton {
    label: String,
    icon: Option<Icon>,
    button_type: ButtonType,
}

impl FilledButton {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.into(),
            icon: None,
            button_type: ButtonType::Standard,
        }
    }

    pub fn style(mut self, style: ButtonType) -> Self {
        self.button_type = style;
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn build<'a, Message: 'a>(self) -> Button<'a, Message>
    {
        let content = if let Some(icon) = self.icon {
            Container::new(
                Row::new()
                    .spacing(2)
                    .push(icon.to_text())
                    .push(Space::with_width(Length::Fill))
                    .push(Text::new(self.label.clone()).size(15))
                    .align_y(iced::Alignment::Center)
            )
        } else {
            Container::new(Text::new(self.label.clone()))
        };

        Button::new(
            content
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center)
        )
            .padding(Padding {
                top: 0.0,
                right: 22.0,
                bottom: 0.0,
                left: 22.0,
            })
            .height(40)
            .width(120)
            .class(self.button_type)
    }
}

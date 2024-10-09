use crate::gui::style::button::ButtonType;
use crate::gui::common::icons::Icon;
use crate::gui::widget::{Button, Container, IcedParentExt, Row, Space, Text};
use iced_core::alignment::{Horizontal, Vertical};
use iced_core::{Length, Padding};
use crate::assets::FONT_FAMILY_BOLD;

pub struct IconButton {
    label: Option<String>,
    icon: Option<Icon>,
    button_type: ButtonType,
    dim: Dimensions
}

pub enum Dimensions {
    Small,
    Medium,
    Large,
}

impl IconButton {
    pub fn new(label: Option<String>) -> Self {
        Self {
            label,
            icon: None,
            button_type: ButtonType::Standard,
            dim: Dimensions::Medium,
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
                    .push_if(self.label.is_some(), || Space::with_width(Length::Fill))
                    .push_if(self.label.is_some(), || Text::new(self.label.unwrap().clone()).size(15).font(FONT_FAMILY_BOLD))
                    .align_y(iced::Alignment::Center)
            )
        } else {
            Container::new(Text::new(self.label.unwrap_or("".parse().unwrap()).clone()).font(FONT_FAMILY_BOLD))
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
            .width(match self.dim {
                Dimensions::Small => 80,
                Dimensions::Medium => 130,
                Dimensions::Large => 160,
            })
            .class(self.button_type)
    }

    pub fn dim(mut self, dim: Dimensions) -> Self {
        self.dim = dim;
        self
    }
}

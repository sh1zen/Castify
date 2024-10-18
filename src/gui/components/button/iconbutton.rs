use crate::assets::FONT_FAMILY_BOLD;
use crate::gui::common::icons::Icon;
use crate::gui::style::button::ButtonType;
use crate::gui::style::text::TextType;
use crate::gui::widget::{Button, Container, IcedParentExt, Row, Space, Text};
use iced_core::alignment::{Horizontal, Vertical};
use iced_core::{Color, Length, Padding};

pub struct IconButton {
    label: Option<String>,
    icon: Option<Icon>,
    button_type: ButtonType,
    dim: Dimensions,
    size: f32,
    color: Option<Color>,
}

pub enum Dimensions {
    Small,
    Medium,
    Large,
    Auto,
}

impl IconButton {
    pub fn new() -> Self {
        Self {
            label: None,
            icon: None,
            button_type: ButtonType::Standard,
            dim: Dimensions::Medium,
            size: 15.0,
            color: None,
        }
    }

    pub fn label(mut self, label: String) -> Self {
        self.label = Some(label);
        self
    }

    pub fn label_if(mut self, condition: bool, label: String) -> Self {
        if condition {
            self.label = Some(label);
        }
        self
    }

    pub fn label_if_else(mut self, condition: bool, if_label: String, else_label: String) -> Self {
        if condition {
            self.label = Some(if_label);
        } else {
            self.label = Some(else_label);
        }
        self
    }

    pub fn style(mut self, style: ButtonType) -> Self {
        self.button_type = style;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn icon_if_else(mut self, condition: bool, if_icon: Icon, else_label: Icon) -> Self {
        if condition {
            self.icon = Some(if_icon);
        } else {
            self.icon = Some(else_label);
        }
        self
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn build<'a, Message: 'a>(self) -> Button<'a, Message>
    {
        let content = if let Some(icon) = self.icon {
            Container::new(
                Row::new()
                    .spacing(5)
                    .push(icon.to_text().size(self.size).class(TextType::maybe_colored(self.color)))
                    .push_if(self.label.is_some(), || Space::with_width(Length::Fill))
                    .push_if(self.label.is_some(), || Text::new(self.label.unwrap()).size(self.size).font(FONT_FAMILY_BOLD).class(TextType::maybe_colored(self.color)))
                    .align_y(iced::Alignment::Center)
            )
        } else {
            Container::new(Text::new(self.label.unwrap_or("".to_string())).font(FONT_FAMILY_BOLD).class(TextType::maybe_colored(self.color)))
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
                Dimensions::Small => Length::Fixed(80.0),
                Dimensions::Medium => Length::Fixed(130.0),
                Dimensions::Large => Length::Fixed(160.0),
                Dimensions::Auto => Length::Shrink,
            })
            .class(self.button_type)
    }

    pub fn dim(mut self, dim: Dimensions) -> Self {
        self.dim = dim;
        self
    }
}

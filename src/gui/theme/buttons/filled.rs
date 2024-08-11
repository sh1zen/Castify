use std::default::Default;

use iced::advanced::widget::Text;
use iced::widget::{button, Button, Container, horizontal_space, Row};

use crate::gui::theme::styles::buttons::ButtonType;
use crate::gui::theme::styles::csx::StyleType;
use crate::gui::types::icons::Icon;

#[derive(Default)]
pub struct FilledButton {
    label: String,
    icon: Option<Icon>,
    style: ButtonType,
}

impl FilledButton {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.into(),
            icon: None,
            style: ButtonType::Standard,
        }
    }

    pub fn style(mut self, style: ButtonType) -> Self {
        self.style = style;
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn build<'a, Message: 'a>(self) -> Button<'a, Message, StyleType>
    {
        let mut content;
        if let Some(icon) = self.icon {
            content = Container::new(
                Row::new()
                    .push(icon.to_text())
                    .push(horizontal_space())
                    .push(Text::new(self.label.clone()).size(16))
                    .align_items(iced::Alignment::Center)
            );
        } else {
            content = Container::new(Text::new(self.label.clone()));
        }
        button(
            content.center_y().center_x()
        )
            .padding([0, 24, 0, 24])
            .height(40)
            .width(120)
            .style(self.style)
    }
}

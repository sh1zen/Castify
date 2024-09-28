use crate::gui::resource::{BORDER_RADIUS, BORDER_WIDTH};
use crate::gui::style::styles::csx::StyleType;
use iced::widget::pick_list::{Catalog, Status, Style};
use iced_core::{Background, Border, Color};

#[derive(Clone, Copy, Debug, Default)]
pub enum PicklistType {
    #[default]
    Standard,
}

impl Catalog for StyleType {
    type Class<'a> = PicklistType;

    fn default<'a>() -> <Self as Catalog>::Class<'a> {
        PicklistType::Standard
    }

    fn style(&self, _class: &<Self as Catalog>::Class<'_>, status: Status) -> Style {
        let colors = self.get_palette();
        let buttons_color = colors.generate_element_color();
        let active = Style {
            text_color: colors.text_body,
            placeholder_color: colors.text_body,
            handle_color: colors.text_body,
            background: Background::Color(Color { a: 0.7, ..buttons_color }),
            border: Border {
                radius: BORDER_RADIUS.into(),
                width: 0.0,
                color: colors.primary_darker,
            },
        };
        match status {
            Status::Active => active,
            Status::Hovered => Style {
                background: Background::Color(Color { a: 0.9, ..buttons_color }),
                ..active
            },
            Status::Opened => Style {
                background: Background::Color(Color { a: 1.0, ..buttons_color }),
                border: Border {
                    width: BORDER_WIDTH,
                    radius: BORDER_RADIUS.into(),
                    color: colors.secondary,
                },
                ..active
            },
        }
    }
}
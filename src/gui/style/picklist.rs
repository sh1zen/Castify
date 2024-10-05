use crate::assets::{BORDER_RADIUS, BORDER_WIDTH};
use crate::gui::style::theme::csx::StyleType;
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
        let palette = self.get_palette();
        let active = Style {
            text_color: palette.text,
            placeholder_color: palette.disabled(palette.text),
            handle_color: palette.primary_darker,
            background: Background::Color(palette.primary),
            border: Border {
                width: BORDER_WIDTH,
                radius: BORDER_RADIUS.into(),
                color: palette.primary_darker,
            },
        };
        match status {
            Status::Active => active,
            Status::Hovered => Style {
                background: Background::Color(Color { a: 0.9, ..palette.primary }),
                ..active
            },
            Status::Opened => Style {
                background: Background::Color(Color { a: 1.0, ..palette.primary }),
                ..active
            },
        }
    }
}
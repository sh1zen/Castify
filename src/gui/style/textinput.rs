use crate::assets::{BORDER_RADIUS, BORDER_WIDTH};
use crate::gui::style::theme::csx::StyleType;
use iced::widget::text_input::{Catalog, Status, Style};
use iced_core::{Background, Border, Color};

#[derive(Clone, Copy, Debug, Default)]
#[allow(dead_code)]
pub enum TextInputType {
    #[default]
    Standard,
    Error,
}

impl Catalog for StyleType {
    type Class<'a> = TextInputType;

    fn default<'a>() -> Self::Class<'a> {
        TextInputType::Standard
    }

    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style {
        let palette = self.get_palette();
        let base = Style {
            background: Background::Color(palette.primary),
            border: Border {
                radius: BORDER_RADIUS.into(),
                width: BORDER_WIDTH,
                color: match class {
                    TextInputType::Standard => palette.primary,
                    TextInputType::Error => palette.danger,
                },
            },
            icon: palette.text,
            placeholder: palette.disabled(palette.text),
            value: palette.text,
            selection: Color {
                a: 0.4,
                ..palette.primary_darker
            },
        };

        let active = Style {
            background: Background::Color(palette.active(palette.primary)),
            value: palette.text,
            ..base
        };

        match status {
            Status::Active => base,
            Status::Hovered => active,
            Status::Focused => active,
            Status::Disabled => Style {
                background: Background::Color(palette.disabled(palette.primary)),
                value: palette.disabled(palette.text),
                ..base
            },
        }
    }
}
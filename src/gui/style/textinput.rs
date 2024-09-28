use crate::gui::resource::{BORDER_RADIUS, BORDER_WIDTH, BUTTON_ALPHA};
use crate::gui::style::styles::csx::StyleType;
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
        let buttons_color = palette.generate_element_color();
        let is_nightly = palette.is_nightly;

        let base = Style {
            background: Background::Color(match class {
                _ => Color {
                    a: BUTTON_ALPHA,
                    ..buttons_color
                },
            }),
            border: Border {
                radius: BORDER_RADIUS.into(),
                width: BORDER_WIDTH,
                color: match class {
                    TextInputType::Standard => buttons_color,
                    TextInputType::Error => Color::new(0.8, 0.15, 0.15, 1.0),
                },
            },
            icon: Color {
                a: if palette.is_nightly { 0.2 } else { 0.7 },
                ..palette.text_body
            },
            placeholder: Color {
                a: if is_nightly { 0.2 } else { 0.7 },
                ..palette.text_body
            },
            value: palette.text_body,
            selection: Color {
                a: if is_nightly { 0.05 } else { 0.4 },
                ..palette.text_body
            },
        };

        let active = Style {
            background: Background::Color(match class {
                _ => Color {
                    a: 1.0,
                    ..buttons_color
                },
            }),
            value: Color {
                a: 1.0,
                ..palette.text_body
            },
            ..base
        };

        match status {
            Status::Active => base,
            Status::Hovered => active,
            Status::Focused => active,
            Status::Disabled => Style {
                background: Background::Color(match class {
                    _ => Color {
                        a: 0.3,
                        ..buttons_color
                    },
                }),
                value: Color {
                    a: 0.6,
                    ..palette.text_body
                },
                ..base
            },
        }
    }
}
use crate::assets::{BORDER_RADIUS, BORDER_WIDTH};
use crate::gui::style::styles::csx::StyleType;
use iced::widget::button::{Catalog, Status, Style};
use iced_core::border::Radius;
use iced_core::{Background, Border, Color, Shadow, Vector};

#[derive(Clone, Copy, Debug, Default)]
#[allow(dead_code)]
pub enum ButtonType {
    #[default]
    Standard,
    Danger,
    Transparent,
    KeyBoard,
    Disabled,
    Rounded,
}

impl Catalog for StyleType {
    type Class<'a> = ButtonType;

    fn default<'a>() -> Self::Class<'a> {
        ButtonType::Standard
    }

    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style {
        let palette = self.get_palette();

        let active = Style {
            background: Some(match class {
                ButtonType::Danger => Background::Color(palette.danger),
                ButtonType::Transparent => Background::Color(Color::TRANSPARENT),
                ButtonType::Disabled => Background::Color(palette.disabled(palette.primary)),
                ButtonType::Rounded => Background::Color(palette.action),
                _ => Background::Color(palette.primary),
            }),
            border: Border {
                radius: match class {
                    ButtonType::Rounded => Radius {
                        top_left: 80.0,
                        top_right: 80.0,
                        bottom_right: 80.0,
                        bottom_left: 80.0,
                    },
                    _ => BORDER_RADIUS.into(),
                },
                width: match class {
                    ButtonType::KeyBoard => 2.0,
                    _ => BORDER_WIDTH,
                },
                color: match class {
                    ButtonType::Transparent => Color::TRANSPARENT,
                    ButtonType::Danger => palette.danger,
                    ButtonType::Standard => palette.primary,
                    ButtonType::KeyBoard => Color {
                        r: 145.0 / 255.0,
                        g: 145.0 / 255.0,
                        b: 245.0 / 255.0,
                        a: 1.0,
                    },
                    ButtonType::Disabled => palette.disabled(palette.primary),
                    _ => palette.secondary,
                },
            },
            text_color: match class {
                ButtonType::KeyBoard => Color::BLACK,
                ButtonType::Transparent => Color { a: 0.8, ..palette.text },
                ButtonType::Disabled => palette.disabled(palette.text_inv),
                _ => palette.text,
            },
            shadow: match class {
                ButtonType::Transparent => Shadow::default(),
                _ => Shadow {
                    color: Color::BLACK,
                    offset: Vector::ZERO,
                    blur_radius: 4.0,
                },
            },
        };

        match status {
            Status::Active => active,
            Status::Hovered => Style {
                background: Some(match class {
                    ButtonType::Danger => Background::Color(palette.active(palette.danger)),
                    ButtonType::Transparent => Background::Color(Color::TRANSPARENT),
                    ButtonType::Disabled => Background::Color(palette.disabled(palette.primary)),
                    ButtonType::Rounded => Background::Color(palette.active(palette.action)),
                    _ => Background::Color(palette.active(palette.primary)),
                }),
                shadow: match class {
                    ButtonType::Transparent => Shadow::default(),
                    _ => Shadow {
                        color: Color::BLACK,
                        offset: Vector::ZERO,
                        blur_radius: 5.0,
                    },
                },
                ..active
            },
            Status::Pressed => Style {
                background: Some(match class {
                    ButtonType::Danger => Background::Color(palette.active(palette.danger)),
                    ButtonType::Transparent => Background::Color(Color::TRANSPARENT),
                    ButtonType::Disabled => Background::Color(palette.disabled(palette.primary)),
                    ButtonType::Rounded => Background::Color(palette.active(palette.action)),
                    _ => Background::Color(palette.active(palette.primary)),
                }),
                shadow: match class {
                    ButtonType::Transparent => Shadow::default(),
                    _ => Shadow {
                        color: Color::BLACK,
                        offset: Vector::ZERO,
                        blur_radius: 2.0,
                    },
                },
                ..active
            },
            Status::Disabled => Style {
                background: Some(match class {
                    ButtonType::Danger => Background::Color(palette.disabled(palette.danger)),
                    ButtonType::Transparent => Background::Color(Color::TRANSPARENT),
                    _ => Background::Color(palette.disabled(palette.primary)),
                }),
                border: Border {
                    color: palette.disabled(active.border.color),
                    ..active.border
                },
                text_color: palette.disabled(active.text_color),
                shadow: match class {
                    ButtonType::KeyBoard => Shadow {
                        color: Color::BLACK,
                        offset: Vector::ZERO,
                        blur_radius: 4.0,
                    },
                    _ => Shadow::default(),
                },
                ..active
            },
        }
    }
}

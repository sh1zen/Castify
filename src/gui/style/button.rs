use crate::assets::{BORDER_RADIUS, BORDER_WIDTH};
use crate::gui::style::theme::csx::StyleType;
use iced::border::Radius;
use iced::widget::button::{Catalog, Status, Style};
use iced::{Background, Border, Color, Shadow, Vector};

#[derive(Clone, Copy, Debug, Default)]

pub enum ButtonType {
    #[default]
    Standard,
    Danger,
    Transparent,
    KeyBoard,
    Disabled,
    Rounded,
}

impl ButtonType {
    fn active_background(self, palette: &crate::gui::style::theme::palette::Palette) -> Color {
        match self {
            ButtonType::Danger => palette.danger,
            ButtonType::Transparent => Color::TRANSPARENT,
            ButtonType::Disabled => palette.disabled(palette.primary),
            ButtonType::Rounded => palette.action,
            _ => palette.primary,
        }
    }

    fn hovered_background(self, palette: &crate::gui::style::theme::palette::Palette) -> Color {
        match self {
            ButtonType::Transparent => Color {
                a: 0.14,
                ..palette.secondary
            },
            ButtonType::Disabled => palette.disabled(palette.primary),
            _ => palette.active(self.active_background(palette)),
        }
    }

    fn pressed_background(self, palette: &crate::gui::style::theme::palette::Palette) -> Color {
        match self {
            ButtonType::Transparent => Color {
                a: 0.24,
                ..palette.secondary
            },
            ButtonType::Disabled => palette.disabled(palette.primary),
            _ => palette.active(self.hovered_background(palette)),
        }
    }

    fn disabled_background(self, palette: &crate::gui::style::theme::palette::Palette) -> Color {
        match self {
            ButtonType::Danger => palette.disabled(palette.danger),
            ButtonType::Transparent => Color::TRANSPARENT,
            _ => palette.disabled(palette.primary),
        }
    }

    fn active_border(self, palette: &crate::gui::style::theme::palette::Palette) -> Color {
        match self {
            ButtonType::Transparent => Color::TRANSPARENT,
            ButtonType::Danger => palette.active(palette.danger),
            ButtonType::Standard => palette.primary_darker,
            ButtonType::KeyBoard => Color {
                r: 145.0 / 255.0,
                g: 145.0 / 255.0,
                b: 245.0 / 255.0,
                a: 1.0,
            },
            ButtonType::Disabled => palette.disabled(palette.primary),
            ButtonType::Rounded => palette.active(palette.action),
        }
    }

    fn hovered_border(self, palette: &crate::gui::style::theme::palette::Palette) -> Color {
        match self {
            ButtonType::Transparent => Color {
                a: 0.30,
                ..palette.secondary
            },
            ButtonType::Disabled => palette.disabled(palette.primary),
            _ => palette.active(self.active_border(palette)),
        }
    }

    fn pressed_border(self, palette: &crate::gui::style::theme::palette::Palette) -> Color {
        match self {
            ButtonType::Transparent => Color {
                a: 0.40,
                ..palette.secondary
            },
            ButtonType::Disabled => palette.disabled(palette.primary),
            _ => palette.active(self.hovered_border(palette)),
        }
    }

    fn text_color(self, palette: &crate::gui::style::theme::palette::Palette) -> Color {
        match self {
            ButtonType::KeyBoard => Color::BLACK,
            ButtonType::Transparent => Color {
                a: 0.9,
                ..palette.text
            },
            ButtonType::Danger | ButtonType::Rounded => palette.text_inv,
            ButtonType::Disabled => palette.disabled(palette.text_inv),
            _ => palette.text,
        }
    }
}

impl Catalog for StyleType {
    type Class<'a> = ButtonType;

    fn default<'a>() -> Self::Class<'a> {
        ButtonType::Standard
    }

    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style {
        let palette = self.get_palette();
        let button = *class;

        let active = Style {
            background: Some(Background::Color(button.active_background(&palette))),
            border: Border {
                radius: match button {
                    ButtonType::Rounded => Radius {
                        top_left: 80.0,
                        top_right: 80.0,
                        bottom_right: 80.0,
                        bottom_left: 80.0,
                    },
                    _ => BORDER_RADIUS.into(),
                },
                width: match button {
                    ButtonType::KeyBoard => 2.0,
                    _ => BORDER_WIDTH,
                },
                color: button.active_border(&palette),
            },
            text_color: button.text_color(&palette),
            shadow: match button {
                ButtonType::Transparent => Shadow::default(),
                _ => Shadow {
                    color: Color::BLACK,
                    offset: Vector::ZERO,
                    blur_radius: 4.0,
                },
            },
            snap: false,
        };

        match status {
            Status::Active => active,
            Status::Hovered => Style {
                background: Some(Background::Color(button.hovered_background(&palette))),
                border: Border {
                    color: button.hovered_border(&palette),
                    ..active.border
                },
                shadow: match button {
                    ButtonType::Transparent => Shadow::default(),
                    _ => Shadow {
                        color: Color::BLACK,
                        offset: Vector::new(0.0, 1.0),
                        blur_radius: 6.0,
                    },
                },
                ..active
            },
            Status::Pressed => Style {
                background: Some(Background::Color(button.pressed_background(&palette))),
                border: Border {
                    color: button.pressed_border(&palette),
                    ..active.border
                },
                shadow: match button {
                    ButtonType::Transparent => Shadow::default(),
                    _ => Shadow {
                        color: Color::BLACK,
                        offset: Vector::new(0.0, 0.5),
                        blur_radius: 1.0,
                    },
                },
                ..active
            },
            Status::Disabled => Style {
                background: Some(Background::Color(button.disabled_background(&palette))),
                border: Border {
                    color: palette.disabled(active.border.color),
                    ..active.border
                },
                text_color: palette.disabled(active.text_color),
                shadow: match button {
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

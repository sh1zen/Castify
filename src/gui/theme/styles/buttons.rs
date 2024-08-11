use iced::widget::button;
use iced::widget::button::Appearance;
use iced::{Background, Border, Color, Shadow, Vector};

use crate::gui::resource::{BORDER_BUTTON_RADIUS, BORDER_WIDTH, BUTTON_ALPHA, H_BUTTON_ALPHA};
use crate::gui::theme::color::mix;
use crate::gui::theme::styles::csx::StyleType;

#[derive(Clone, Copy, Default)]
pub enum ButtonType {
    #[default]
    Standard,
    Tab,
    Starred,
    Alert,
    Transparent,
}

impl button::StyleSheet for StyleType {
    type Style = ButtonType;

    fn active(&self, style: &Self::Style) -> Appearance {
        let colors = self.get_palette();
        let buttons_color = colors.generate_buttons_color();
        Appearance {
            background: Some(match style {
                ButtonType::Tab => Background::Color(Color {
                    a: BUTTON_ALPHA,
                    ..mix(colors.primary, buttons_color)
                }),
                ButtonType::Alert => Background::Color(Color {
                    a: BUTTON_ALPHA,
                    ..colors.alert
                }),
                ButtonType::Starred => Background::Color(Color {
                    a: BUTTON_ALPHA,
                    ..colors.starred
                }),
                ButtonType::Transparent => Background::Color(Color::TRANSPARENT),
                _ => Background::Color(Color {
                    a: BUTTON_ALPHA,
                    ..buttons_color
                })
            }),
            border: Border {
                radius: match style {
                    ButtonType::Tab => {
                        [0.0, 0.0, 30.0, 30.0].into()
                    }
                    _ => BORDER_BUTTON_RADIUS.into(),
                },
                width: match style {
                    ButtonType::Transparent | ButtonType::Tab | ButtonType::Standard => 0.0,
                    _ => BORDER_WIDTH,
                },
                color: match style {
                    ButtonType::Alert => colors.alert,
                    ButtonType::Standard => Color {
                        a: 0.7,
                        ..buttons_color
                    },
                    _ => colors.secondary,
                },
            },
            shadow_offset: match style {
                ButtonType::Tab => Vector::new(0.0, 2.0),
                _ => Vector::default(),
            },
            text_color: match style {
                ButtonType::Starred => Color::BLACK,
                ButtonType::Transparent => mix(colors.text_headers, colors.secondary),
                _ => colors.text_body,
            },
            shadow: match style {
                ButtonType::Tab => Shadow {
                    color: Color::BLACK,
                    offset: Vector::new(0.0, 2.0),
                    blur_radius: 4.0,
                },
                _ => Shadow::default(),
            },
        }
    }

    fn hovered(&self, style: &Self::Style) -> Appearance {
        let colors = self.get_palette();
        let buttons_color = colors.generate_buttons_color();
        Appearance {
            shadow_offset: match style {
                ButtonType::Tab => Vector::new(0.0, 3.0),
                _ => Vector::new(0.0, 0.0),
            },
            shadow: match style {
                ButtonType::Transparent => Shadow::default(),
                _ => Shadow {
                    color: Color::BLACK,
                    offset: match style {
                        ButtonType::Tab => Vector::new(0.0, 3.0),
                        _ => Vector::new(0.0, 0.0),
                    },
                    blur_radius: 5.0,
                },
            },
            background: Some(match style {
                ButtonType::Tab => Background::Color(Color {
                    a: H_BUTTON_ALPHA,
                    ..mix(colors.primary, buttons_color)
                }),
                ButtonType::Alert => Background::Color(Color {
                    a: H_BUTTON_ALPHA,
                    ..colors.alert
                }),
                ButtonType::Starred => Background::Color(Color {
                    a: H_BUTTON_ALPHA,
                    ..colors.starred
                }),
                ButtonType::Transparent => Background::Color(Color::TRANSPARENT),
                _ => Background::Color(Color {
                    a: H_BUTTON_ALPHA,
                    ..buttons_color
                })
            }),
            ..self.active(style)
        }
    }

    fn pressed(&self, style: &Self::Style) -> Appearance {
        Appearance {
            shadow_offset: match style {
                ButtonType::Tab => Vector::new(0.0, 3.0),
                _ => Vector::new(0.0, 0.0),
            },
            shadow: match style {
                ButtonType::Transparent => Shadow::default(),
                _ => Shadow {
                    color: Color::BLACK,
                    offset: match style {
                        ButtonType::Tab => Vector::new(0.0, 3.0),
                        _ => Vector::new(0.0, 0.0),
                    },
                    blur_radius: 2.0,
                },
            },
            ..self.active(style)
        }
    }

    fn disabled(&self, style: &Self::Style) -> Appearance {
        let colors = self.get_palette();
        let buttons_color = colors.generate_buttons_color();
        match style {
            ButtonType::Standard => Appearance {
                shadow_offset: Vector::default(),
                background: Some(Background::Color(Color {
                    a: 0.6,
                    ..buttons_color
                })),
                border: Border {
                    radius: BORDER_BUTTON_RADIUS.into(),
                    width: BORDER_WIDTH,
                    color: Color {
                        a: 0.6,
                        ..colors.secondary
                    },
                },
                text_color: Color {
                    a: 1.0,
                    ..colors.text_body
                },
                shadow: Shadow::default(),
            },
            _ => button::StyleSheet::active(self, style),
        }
    }
}


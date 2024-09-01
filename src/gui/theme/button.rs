use iced::widget::button;
use iced::widget::button::Appearance;
use iced::{Background, Border, Color, Shadow, Vector};

use crate::gui::resource::{BORDER_WIDTH, BUTTON_ALPHA, BORDER_RADIUS, H_BUTTON_ALPHA};
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
    KeyBoard,
    Disabled,
    Round
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
                    ..colors.highlight
                }),
                ButtonType::Transparent => Background::Color(Color::TRANSPARENT),
                ButtonType::Disabled => Background::Color(Color::from_rgb(0.6, 0.6, 0.6)), // Colore grigio per il pulsante disabilitato
                _ => Background::Color(Color {
                    a: BUTTON_ALPHA,
                    ..buttons_color
                })
            }),
            border: Border {
                radius: match style {
                    ButtonType::Round => {
                        [80.0, 80.0, 80.0, 80.0].into()
                    },
                    ButtonType::Tab => {
                        [0.0, 0.0, 30.0, 30.0].into()
                    }
                    _ => BORDER_RADIUS.into(),
                },
                width: match style {
                    ButtonType::Transparent | ButtonType::Tab | ButtonType::Standard => 0.0,
                    ButtonType::KeyBoard | ButtonType::Round => 2.0,
                    _ => BORDER_WIDTH,
                },
                color: match style {
                    ButtonType::Alert => colors.alert,
                    ButtonType::Standard => Color {
                        a: 0.7,
                        ..buttons_color
                    },
                    ButtonType::KeyBoard => Color {
                        r: 145.0 / 255.0,
                        g: 145.0 / 255.0,
                        b: 245.0 / 255.0,
                        a: 1.0,
                    },
                    ButtonType::Disabled => Color::from_rgb(0.5, 0.5, 0.5), // Colore grigio per il bordo disabilitato
                    _ => colors.secondary,
                },
            },
            shadow_offset: match style {
                ButtonType::Tab => Vector::new(0.0, 2.0),
                _ => Vector::default(),
            },
            text_color: match style {
                ButtonType::Starred | ButtonType::KeyBoard => Color::BLACK,
                ButtonType::Transparent => mix(colors.text_headers, colors.secondary),
                ButtonType::Disabled => Color::from_rgb(0.7, 0.7, 0.7), // Colore grigio per il testo disabilitato
                _ => colors.text_body,
            },
            shadow: match style {
                ButtonType::Tab => Shadow {
                    color: Color::BLACK,
                    offset: Vector::new(0.0, 2.0),
                    blur_radius: 4.0,
                },
                ButtonType::KeyBoard => Shadow {
                    color: Color::BLACK,
                    offset: Vector::new(0.0, 0.0),
                    blur_radius: 5.0,
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
                    ..colors.highlight
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
        let active = self.active(style);
        Appearance {
            shadow_offset: Vector::default(),
            background: Some(match style {
                ButtonType::Tab => Background::Color(Color {
                    a: 0.2,
                    ..mix(colors.primary, buttons_color)
                }),
                ButtonType::Alert => Background::Color(Color {
                    a: 0.2,
                    ..colors.alert
                }),
                ButtonType::Starred => Background::Color(Color {
                    a: 0.2,
                    ..colors.highlight
                }),
                ButtonType::Transparent => Background::Color(Color::TRANSPARENT),
                _ => Background::Color(Color {
                    a: 0.2,
                    ..buttons_color
                })
            }),
            border: Border {
                radius: active.border.radius,
                width: active.border.width,
                color: Color {
                    a: 0.3,
                    ..active.border.color
                },
            },
            text_color: Color {
                a: 0.4,
                ..active.text_color
            },
            shadow: match style {
                ButtonType::KeyBoard => Shadow {
                    color: Color::BLACK,
                    offset: Vector::new(0.0, 0.0),
                    blur_radius: 4.0,
                },
                _ => Shadow::default(),
            },
        }
    }
}

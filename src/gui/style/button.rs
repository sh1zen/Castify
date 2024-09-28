use crate::gui::resource::{BORDER_RADIUS, BORDER_WIDTH, BUTTON_ALPHA, H_BUTTON_ALPHA};
use crate::gui::style::color::mix;
use crate::gui::style::styles::csx::StyleType;
use iced::widget::button::{Catalog, Status, Style};
use iced_core::{Background, Border, Color, Shadow, Vector};
use iced_core::border::Radius;

#[derive(Clone, Copy, Debug, Default)]
#[allow(dead_code)]
pub enum ButtonType {
    #[default]
    Standard,
    Tab,
    Starred,
    Alert,
    Transparent,
    KeyBoard,
    Disabled,
    Round,
}

impl Catalog for StyleType {
    type Class<'a> = ButtonType;

    fn default<'a>() -> Self::Class<'a> {
        ButtonType::Standard
    }

    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style {
        let colors = self.get_palette();
        let buttons_color = colors.generate_element_color();

        let active = Style {
            background: Some(match class {
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
                radius: match class {
                    ButtonType::Round => Radius {
                            top_left: 80.0,
                            top_right: 80.0,
                            bottom_right: 80.0,
                            bottom_left: 80.0,
                        },
                    ButtonType::Tab =>  Radius {
                        top_left: 0.0,
                        top_right: 0.0,
                        bottom_right: 30.0,
                        bottom_left: 30.0,
                    },
                    _ => BORDER_RADIUS.into(),
                },
                width: match class {
                    ButtonType::Transparent | ButtonType::Tab | ButtonType::Standard => 0.0,
                    ButtonType::KeyBoard | ButtonType::Round => 2.0,
                    _ => BORDER_WIDTH,
                },
                color: match class {
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
            text_color: match class {
                ButtonType::Starred | ButtonType::KeyBoard => Color::BLACK,
                ButtonType::Transparent => mix(colors.text_headers, colors.secondary),
                ButtonType::Disabled => Color::from_rgb(0.7, 0.7, 0.7),
                _ => colors.text_body,
            },
            shadow: match class {
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
        };

        match status {
            Status::Active => active,
            Status::Hovered => Style {
                background: Some(match class {
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
                shadow: match class {
                    ButtonType::Transparent => Shadow::default(),
                    _ => Shadow {
                        color: Color::BLACK,
                        offset: match class {
                            ButtonType::Tab => Vector::new(0.0, 3.0),
                            _ => Vector::new(0.0, 0.0),
                        },
                        blur_radius: 5.0,
                    },
                },
                ..active
            },
            Status::Pressed => Style {
                shadow: match class {
                    ButtonType::Transparent => Shadow::default(),
                    _ => Shadow {
                        color: Color::BLACK,
                        offset: match class {
                            ButtonType::Tab => Vector::new(0.0, 3.0),
                            _ => Vector::new(0.0, 0.0),
                        },
                        blur_radius: 2.0,
                    },
                },
                ..active
            },
            Status::Disabled => Style {
                background: Some(match class {
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
                shadow: match class {
                    ButtonType::KeyBoard => Shadow {
                        color: Color::BLACK,
                        offset: Vector::new(0.0, 0.0),
                        blur_radius: 4.0,
                    },
                    _ => Shadow::default(),
                },
                ..active
            },
        }
    }
}

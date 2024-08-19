use crate::gui::resource::{BORDER_WIDTH, BUTTON_ALPHA, BORDER_RADIUS};
use crate::gui::theme::styles::csx::StyleType;
use iced::widget::text_input::Appearance;
use iced::{Background, Border, Color};

#[derive(Clone, Copy, Default)]
pub enum TextInputType {
    #[default]
    Standard,
    Error,
}

impl iced::widget::text_input::StyleSheet for StyleType {
    type Style = TextInputType;

    fn active(&self, style: &Self::Style) -> Appearance {
        let palette = self.get_palette();
        let buttons_color = palette.generate_buttons_color();
        Appearance {
            background: Background::Color(match style {
                _ => Color {
                    a: BUTTON_ALPHA,
                    ..buttons_color
                },
            }),
            border: Border {
                radius: BORDER_RADIUS.into(),
                width: BORDER_WIDTH,
                color: match style {
                    TextInputType::Standard => buttons_color,
                    TextInputType::Error => Color::new(0.8, 0.15, 0.15, 1.0),
                },
            },
            icon_color: Color {
                a: if palette.is_nightly { 0.2 } else { 0.7 },
                ..palette.text_body
            },
        }
    }

    fn focused(&self, style: &Self::Style) -> Appearance {
        let palette = self.get_palette();
        let buttons_color = palette.generate_buttons_color();
        let active = self.active(style);
        Appearance {
            background: Background::Color(match style {
                _ => Color {
                    a: 0.9,
                    ..buttons_color
                },
            }),
            ..active
        }
    }

    fn placeholder_color(&self, _: &Self::Style) -> Color {
        let palette = self.get_palette();
        let is_nightly = palette.is_nightly;
        Color {
            a: if is_nightly { 0.2 } else { 0.7 },
            ..palette.text_body
        }
    }

    fn value_color(&self, _: &Self::Style) -> Color {
        self.get_palette().text_body
    }

    fn disabled_color(&self, _style: &Self::Style) -> Color {
        let palette = self.get_palette();
        let is_nightly = palette.is_nightly;
        Color {
            a: if is_nightly { 0.2 } else { 0.7 },
            ..palette.text_body
        }
    }

    fn selection_color(&self, _: &Self::Style) -> Color {
        let palette = self.get_palette();
        let is_nightly = palette.is_nightly;
        Color {
            a: if is_nightly { 0.05 } else { 0.4 },
            ..palette.text_body
        }
    }

    fn hovered(&self, style: &Self::Style) -> Appearance {
        let palette = self.get_palette();
        let buttons_color = palette.generate_buttons_color();
        let active = self.active(style);
        Appearance {
            background: Background::Color(buttons_color),
           ..active
        }
    }

    fn disabled(&self, style: &Self::Style) -> Appearance {
        let palette = self.get_palette();
        let buttons_color = palette.generate_buttons_color();
        let active = self.active(style);
        Appearance {
            background: Background::Color(match style {
                _ => Color {
                    a: 0.3,
                    ..buttons_color
                },
            }),
            ..active
        }
    }
}
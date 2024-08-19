use iced::{application, Color};
use iced::application::Appearance;
use plotters::prelude::FontStyle;

use crate::gui::resource::RALEWAY_FONT;
use crate::gui::theme::styles::palette::Palette;

/// Used to specify the kind of style of the application
#[derive(Clone, Copy, Debug, Hash, PartialEq)]
pub enum StyleType {
    Venus,
}

impl Default for StyleType {
    fn default() -> Self {
        Self::Venus
    }
}

impl application::StyleSheet for StyleType {
    type Style = ();

    fn appearance(&self, (): &Self::Style) -> Appearance {
        let colors = self.get_palette();
        Appearance {
            background_color: colors.primary,
            text_color: colors.text_body
        }
    }
}

impl StyleType {
    pub fn get_palette(self) -> Palette {
        match self {
            StyleType::Venus => Palette {
                primary: Color {
                    r: 34.0 / 255.0,
                    g: 52.0 / 255.0,
                    b: 74.0 / 255.0,
                    a: 1.0,
                },
                primary_darker: Color {
                    r: 24.0 / 255.0,
                    g: 42.0 / 255.0,
                    b: 64.0 / 255.0,
                    a: 1.0,
                },
                secondary: Color {
                    r: 159.0 / 255.0,
                    g: 106.0 / 255.0,
                    b: 65.0 / 255.0,
                    a: 1.0,
                },
                highlight: Color {
                    r: 245.0 / 255.0,
                    g: 245.0 / 255.0,
                    b: 245.0 / 255.0,
                    a: 1.0,
                },
                alert: Color {
                    r: 245.0 / 255.0,
                    g: 120.0 / 255.0,
                    b: 120.0 / 255.0,
                    a: 1.0,
                },
                text_headers: Color::BLACK,
                text_body: Color::WHITE,
                font: RALEWAY_FONT,
                is_nightly: true,
            }
        }
    }

    pub fn get_font_weight(self) -> FontStyle {
        if self.get_palette().font.eq(&RALEWAY_FONT) {
            FontStyle::Bold
        } else {
            FontStyle::Normal
        }
    }
}
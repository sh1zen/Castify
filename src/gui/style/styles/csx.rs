use crate::gui::resource::RALEWAY_FONT;
use crate::gui::style::styles::palette::Palette;
use crate::rgba8;
use iced::application::{Appearance, DefaultStyle};
use iced_core::Color;
use plotters::prelude::FontStyle;

/// Used to specify the kind of style of the application
#[derive(Clone, Copy, Debug, Hash, PartialEq)]
pub enum StyleType {
    Venus,
    SemiTransparent,
}

impl Default for StyleType {
    fn default() -> Self {
        Self::Venus
    }
}

impl DefaultStyle for StyleType {
    fn default_style(&self) -> Appearance {
        let colors = self.get_palette();
        Appearance {
            background_color: colors.primary,
            text_color: colors.text_body,
        }
    }
}

impl StyleType {
    pub fn get_palette(self) -> Palette {
        match self {
            StyleType::Venus => Palette {
                primary: rgba8!(34.0, 52.0, 74.0, 1.0),
                primary_darker: rgba8!(24.0, 42.0, 64.0, 1.0),
                secondary: rgba8!(159.0, 106.0, 65.0, 1.0),
                highlight: rgba8!(245.0, 245.0, 245.0, 1.0),
                alert: rgba8!(245.0, 120.0, 120.0, 1.0),
                text_headers: Color::BLACK,
                text_body: Color::WHITE,
                font: RALEWAY_FONT,
                is_nightly: true,
            },
            StyleType::SemiTransparent => Palette {
                primary: rgba8!(60.0, 60.0, 60.0, 0.4),
                primary_darker: rgba8!(60.0, 60.0, 60.0, 0.6),
                secondary: rgba8!(159.0, 106.0, 65.0, 0.6),
                highlight: rgba8!(245.0, 245.0, 245.0, 0.6),
                alert: rgba8!(245.0, 120.0, 120.0, 0.6),
                text_headers: Color::BLACK,
                text_body: Color::WHITE,
                font: RALEWAY_FONT,
                is_nightly: false,
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
use crate::assets::{FONT_FAMILY_BASE, FONT_FAMILY_BOLD};
use crate::gui::style::styles::palette::Palette;
use crate::rgba8;
use iced::application::{Appearance, DefaultStyle};
use iced_core::Color;

/// Used to specify the kind of style of the application
#[derive(Clone, Copy, Debug, Hash, PartialEq)]
pub enum StyleType {
    DarkVenus,
    SemiTransparent,
    LightVenus,
}

impl Default for StyleType {
    fn default() -> Self {
        Self::DarkVenus
    }
}

impl DefaultStyle for StyleType {
    fn default_style(&self) -> Appearance {
        let colors = self.get_palette();
        Appearance {
            background_color: colors.primary,
            text_color: colors.text,
        }
    }
}

impl StyleType {
    pub fn get_palette(self) -> Palette {
        match self {
            StyleType::DarkVenus => Palette {
                background: rgba8!(34.0, 52.0, 74.0, 1.0),
                primary: rgba8!(34.0, 52.0, 74.0, 1.0),
                primary_darker: rgba8!(24.0, 42.0, 64.0, 1.0),
                secondary: rgba8!(159.0, 106.0, 65.0, 1.0),
                action: rgba8!(125.0, 125.0, 225.0, 1.0),
                danger: rgba8!(225.0, 100.0, 100.0, 1.0),
                text: Color::WHITE,
                text_inv: Color::BLACK,
                font: FONT_FAMILY_BASE,
                is_nightly: true,
                is_transparent: false,
            },
            StyleType::LightVenus => Palette {
                background: rgba8!(220.0, 220.0, 220.0, 1.0),
                primary: rgba8!(210.0, 210.0, 210.0, 1.0),
                primary_darker: rgba8!(180.0, 180.0, 180.0, 1.0),
                secondary: rgba8!(160.0, 160.0, 160.0, 1.0),
                action: rgba8!(220.0, 140.0, 80.0, 1.0),
                danger: rgba8!(225.0, 80.0, 80.0, 1.0),
                text: Color::BLACK,
                text_inv: Color::WHITE,
                font: FONT_FAMILY_BASE,
                is_nightly: false,
                is_transparent: false,
            },
            StyleType::SemiTransparent => Palette {
                background: rgba8!(40.0, 40.0, 40.0, 0.3),
                primary: rgba8!(210.0, 210.0, 210.0, 1.0),
                primary_darker: rgba8!(180.0, 180.0, 180.0, 1.0),
                secondary: rgba8!(160.0, 160.0, 160.0, 1.0),
                action: rgba8!(220.0, 140.0, 80.0, 1.0),
                danger: rgba8!(225.0, 80.0, 80.0, 1.0),
                text: Color::BLACK,
                text_inv: Color::WHITE,
                font: FONT_FAMILY_BOLD,
                is_nightly: false,
                is_transparent: true,
            }
        }
    }
}
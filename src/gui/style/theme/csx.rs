use crate::gui::style::theme::palette::Palette;
use crate::rgba8;
use iced::theme::{Base, Mode, Style};
use iced::Color;
use iced_anim::Animate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum StyleType {
    DarkVenus,
    SemiTransparent,
    #[default]
    LightVenus,
    Transparent,
    #[serde(skip)]
    Custom(Palette),
}

impl StyleType {
    pub fn toggle(&self) -> Self {
        match self {
            StyleType::LightVenus => StyleType::DarkVenus,
            _ => StyleType::LightVenus,
        }
    }
    pub fn get_palette(&self) -> Palette {
        match self {
            StyleType::DarkVenus => Palette {
                background: rgba8!(34.0, 52.0, 74.0, 1.0),
                primary: rgba8!(34.0, 52.0, 74.0, 1.0),
                primary_darker: rgba8!(24.0, 42.0, 64.0, 1.0),
                secondary: rgba8!(159.0, 106.0, 65.0, 1.0),
                action: rgba8!(220.0, 120.0, 20.0, 1.0),
                danger: rgba8!(225.0, 100.0, 100.0, 1.0),
                text: Color::WHITE,
                text_inv: Color::BLACK,
            },
            StyleType::LightVenus => Palette {
                background: rgba8!(220.0, 220.0, 220.0, 1.0),
                primary: rgba8!(210.0, 210.0, 210.0, 1.0),
                primary_darker: rgba8!(180.0, 180.0, 180.0, 1.0),
                secondary: rgba8!(160.0, 160.0, 160.0, 1.0),
                action: rgba8!(220.0, 120.0, 20.0, 1.0),
                danger: rgba8!(225.0, 80.0, 80.0, 1.0),
                text: Color::BLACK,
                text_inv: Color::WHITE,
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
            },
            StyleType::Transparent => Palette {
                background: rgba8!(0.0, 0.0, 0.0, 0.0),
                primary: rgba8!(210.0, 210.0, 210.0, 1.0),
                primary_darker: rgba8!(180.0, 180.0, 180.0, 1.0),
                secondary: rgba8!(160.0, 160.0, 160.0, 1.0),
                action: rgba8!(220.0, 120.0, 20.0, 1.0),
                danger: rgba8!(225.0, 80.0, 80.0, 1.0),
                text: Color::BLACK,
                text_inv: Color::WHITE,
            },
            StyleType::Custom(palette) => *palette,
        }
    }
}

impl Base for StyleType {
    fn default(preference: Mode) -> Self {
        match preference {
            Mode::Dark => StyleType::DarkVenus,
            _ => StyleType::LightVenus,
        }
    }
    fn mode(&self) -> Mode { Mode::None }
    fn base(&self) -> Style {
        let colors = self.get_palette();
        Style {
            background_color: colors.background,
            text_color: colors.text,
        }
    }
    fn palette(&self) -> Option<iced::theme::Palette> { None }
    fn name(&self) -> &str {
        match self {
            StyleType::DarkVenus => "DarkVenus",
            StyleType::LightVenus => "LightVenus",
            StyleType::SemiTransparent => "SemiTransparent",
            StyleType::Transparent => "Transparent",
            StyleType::Custom(_) => "Custom",
        }
    }
}

impl Animate for StyleType {
    fn components() -> usize {
        Palette::components()
    }

    fn update(&mut self, components: &mut impl Iterator<Item=f32>) {
        let mut palette = self.get_palette();
        palette.update(components);
        *self = StyleType::Custom(palette);
    }

    fn distance_to(&self, end: &Self) -> Vec<f32> {
        self.get_palette().distance_to(&end.get_palette())
    }

    fn lerp(&mut self, start: &Self, end: &Self, progress: f32) {
        let start = start.get_palette();
        let end = end.get_palette();
        let mut palette = start;
        palette.lerp(&start, &end, progress);
        *self = StyleType::Custom(palette);
    }
}
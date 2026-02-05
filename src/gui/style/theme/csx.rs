use crate::gui::style::theme::palette::Palette;
use crate::rgba8;
use iced::theme::{Base, Mode, Style};
use iced::Color;
use iced_anim::Animate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum StyleType {
    DarkVenus,
    Darcula,
    SemiTransparent,
    #[default]
    LightVenus,
    SmokedLightBlue,
    Transparent,
    #[serde(skip)]
    Custom(Palette),
}

impl StyleType {
    pub fn toggle(&self) -> Self {
        match self {
            StyleType::LightVenus => StyleType::SmokedLightBlue,
            StyleType::SmokedLightBlue => StyleType::DarkVenus,
            StyleType::DarkVenus => StyleType::Darcula,
            StyleType::Darcula => StyleType::LightVenus,
            _ => StyleType::LightVenus,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            StyleType::DarkVenus => "Dark Venus",
            StyleType::Darcula => "Darcula",
            StyleType::LightVenus => "Light Venus",
            StyleType::SmokedLightBlue => "Smoked Light Blue",
            StyleType::SemiTransparent => "Semi Transparent",
            StyleType::Transparent => "Transparent",
            StyleType::Custom(_) => "Custom",
        }
    }

    pub fn get_palette(&self) -> Palette {
        let light = Palette {
            background: rgba8!(236.0, 239.0, 243.0, 1.0),
            primary: rgba8!(224.0, 228.0, 234.0, 1.0),
            primary_darker: rgba8!(197.0, 205.0, 214.0, 1.0),
            secondary: rgba8!(130.0, 146.0, 166.0, 1.0),
            action: rgba8!(225.0, 130.0, 45.0, 1.0),
            danger: rgba8!(214.0, 86.0, 86.0, 1.0),
            text: Color::BLACK,
            text_inv: Color::WHITE,
        };

        let smoked_light_blue = Palette {
            background: rgba8!(223.0, 235.0, 245.0, 1.0),
            primary: rgba8!(204.0, 221.0, 236.0, 1.0),
            primary_darker: rgba8!(170.0, 194.0, 215.0, 1.0),
            secondary: rgba8!(112.0, 150.0, 182.0, 1.0),
            action: rgba8!(242.0, 156.0, 74.0, 1.0),
            danger: rgba8!(216.0, 88.0, 88.0, 1.0),
            text: rgba8!(22.0, 35.0, 48.0, 1.0),
            text_inv: Color::WHITE,
        };

        let darcula = Palette {
            background: rgba8!(43.0, 43.0, 43.0, 1.0),
            primary: rgba8!(60.0, 63.0, 65.0, 1.0),
            primary_darker: rgba8!(49.0, 51.0, 53.0, 1.0),
            secondary: rgba8!(104.0, 151.0, 187.0, 1.0),
            action: rgba8!(204.0, 120.0, 50.0, 1.0),
            danger: rgba8!(204.0, 102.0, 102.0, 1.0),
            text: rgba8!(169.0, 183.0, 198.0, 1.0),
            text_inv: Color::BLACK,
        };

        match self {
            StyleType::DarkVenus => Palette {
                background: rgba8!(28.0, 42.0, 60.0, 1.0),
                primary: rgba8!(33.0, 50.0, 70.0, 1.0),
                primary_darker: rgba8!(22.0, 34.0, 50.0, 1.0),
                secondary: rgba8!(164.0, 124.0, 84.0, 1.0),
                action: rgba8!(230.0, 145.0, 55.0, 1.0),
                danger: rgba8!(226.0, 106.0, 106.0, 1.0),
                text: Color::WHITE,
                text_inv: Color::BLACK,
            },
            StyleType::LightVenus => light,
            StyleType::SmokedLightBlue => smoked_light_blue,
            StyleType::Darcula => darcula,
            StyleType::SemiTransparent => Palette {
                background: rgba8!(18.0, 32.0, 46.0, 0.34),
                action: rgba8!(242.0, 156.0, 74.0, 1.0),
                ..smoked_light_blue
            },
            StyleType::Transparent => Palette {
                background: rgba8!(0.0, 0.0, 0.0, 0.0),
                ..smoked_light_blue
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
    fn mode(&self) -> Mode {
        Mode::None
    }
    fn base(&self) -> Style {
        let colors = self.get_palette();
        Style {
            background_color: colors.background,
            text_color: colors.text,
        }
    }
    fn palette(&self) -> Option<iced::theme::Palette> {
        None
    }
    fn name(&self) -> &str {
        match self {
            StyleType::DarkVenus => "DarkVenus",
            StyleType::Darcula => "Darcula",
            StyleType::LightVenus => "LightVenus",
            StyleType::SmokedLightBlue => "SmokedLightBlue",
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

    fn update(&mut self, components: &mut impl Iterator<Item = f32>) {
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

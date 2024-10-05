use crate::gui::style::theme::color::color_hash;
use crate::gui::style::theme::csx::StyleType;
use iced_anim::Animate;
use iced_core::Color;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Copy, PartialEq, Animate)]
pub struct Palette {
    /// main app color
    pub background: Color,
    /// Main color of the GUI (background)
    pub primary: Color,
    /// as primary but darker for elements
    pub primary_darker: Color,
    /// Secondary color of the GUI
    pub secondary: Color,
    /// Color of alert
    pub danger: Color,
    /// The action content color
    pub action: Color,
    /// Base text color
    pub text: Color,
    /// Inverted text color (light in dark mode, v.v.)
    pub text_inv: Color,
}

impl Palette {
    pub fn is_nightly(&self) -> bool {
        // Calculate the relative luminance using a simple approximation
        let luminance = 0.2126 * self.background.r + 0.7152 * self.background.g + 0.0722 * self.background.b;

        // If luminance is less than the threshold, the color is considered dark
        luminance < 0.5
    }
    pub fn active(&self, color: Color) -> Color {
        if self.is_nightly() {
            Color {
                r: f32::min(color.r + 0.15, 1.0),
                g: f32::min(color.g + 0.15, 1.0),
                b: f32::min(color.b + 0.15, 1.0),
                a: 1.0,
            }
        } else {
            Color {
                r: f32::max(color.r - 0.15, 0.0),
                g: f32::max(color.g - 0.15, 0.0),
                b: f32::max(color.b - 0.15, 0.0),
                a: 1.0,
            }
        }
    }
    pub fn disabled(&self, color: Color) -> Color {
        if self.is_nightly() {
            Color {
                r: f32::min(color.r - 0.1, 1.0),
                g: f32::min(color.g - 0.1, 1.0),
                b: f32::min(color.b - 0.1, 1.0),
                a: 0.6,
            }
        } else {
            Color {
                r: f32::max(color.r - 0.2, 0.0),
                g: f32::max(color.g - 0.2, 0.0),
                b: f32::max(color.b - 0.2, 0.0),
                a: 0.6,
            }
        }
    }
}

impl Default for Palette {
    fn default() -> Self {
        StyleType::get_palette(&StyleType::DarkVenus)
    }
}


impl Hash for Palette {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let Palette {
            background, primary, primary_darker, secondary, danger, action, text, text_inv
        } = self;

        color_hash(*background, state);
        color_hash(*primary, state);
        color_hash(*primary_darker, state);
        color_hash(*secondary, state);
        color_hash(*danger, state);
        color_hash(*action, state);
        color_hash(*text, state);
        color_hash(*text_inv, state);
    }
}
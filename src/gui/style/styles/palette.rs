use crate::gui::style::styles::csx::StyleType;
use iced_core::{Color, Font};
use std::hash::Hash;

#[derive(Debug, Clone, Copy, PartialEq)]
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
    /// Font used
    pub font: Font,
    /// If is nightly
    pub is_nightly: bool,
    /// If is nightly
    pub is_transparent: bool,
}

impl Palette {
    pub fn active(&self, color: Color) -> Color {
        if self.is_nightly {
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
        if self.is_nightly {
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
        StyleType::get_palette(StyleType::DarkVenus)
    }
}
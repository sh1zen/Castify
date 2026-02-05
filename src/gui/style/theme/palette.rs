use crate::gui::style::theme::color::color_hash;
use crate::gui::style::theme::csx::StyleType;
use iced::Color;
use iced_anim::Animate;
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
    fn adjust(&self, color: Color, amount: f32, alpha: f32) -> Color {
        let channel = |value: f32| {
            if self.is_dark() {
                f32::min(value + amount, 1.0)
            } else {
                f32::max(value - amount, 0.0)
            }
        };
        Color {
            r: channel(color.r),
            g: channel(color.g),
            b: channel(color.b),
            a: alpha,
        }
    }

    fn emphasized_text(&self, weight: f32, neutral: f32) -> Color {
        let anchor = if self.is_dark() {
            Color::WHITE
        } else {
            Color::BLACK
        };
        Color {
            r: neutral * (1.0 - weight) + anchor.r * weight,
            g: neutral * (1.0 - weight) + anchor.g * weight,
            b: neutral * (1.0 - weight) + anchor.b * weight,
            a: 1.0,
        }
    }

    pub fn is_dark(&self) -> bool {
        let luminance =
            0.2126 * self.background.r + 0.7152 * self.background.g + 0.0722 * self.background.b;
        luminance < 0.5
    }

    pub fn is_nightly(&self) -> bool {
        self.is_dark()
    }

    pub fn active(&self, color: Color) -> Color {
        self.adjust(color, 0.15, 1.0)
    }

    pub fn disabled(&self, color: Color) -> Color {
        self.adjust(color, if self.is_dark() { 0.1 } else { 0.2 }, 0.6)
    }

    pub fn title_text(&self) -> Color {
        let (weight, neutral) = if self.is_dark() {
            (0.4, 1.0)
        } else {
            (0.6, 0.7)
        };
        self.emphasized_text(weight, neutral)
    }

    pub fn subtitle_text(&self) -> Color {
        let (weight, neutral) = if self.is_dark() {
            (0.4, 0.8)
        } else {
            (0.6, 0.5)
        };
        self.emphasized_text(weight, neutral)
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
            background,
            primary,
            primary_darker,
            secondary,
            danger,
            action,
            text,
            text_inv,
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

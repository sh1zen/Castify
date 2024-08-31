//! Change the appearance of a AreaSelector.

use iced::Theme;
use iced_core::{Background, Border, Color, Pixels, Shadow, Vector};

/// The appearance of a AreaSelector.
#[derive(Debug, Clone, Copy)]
pub struct Appearance {
    /// The amount of offset to apply to the shadow of the AreaSelector.
    pub shadow_offset: Vector,
    /// The [`Background`] of the AreaSelector.
    pub background: Option<Background>,
    /// The [`Border`] of the AreaSelector.
    pub border: Border,
    /// The [`Shadow`] of the AreaSelector.
    pub shadow: Shadow,
}

impl Appearance {
    pub fn with_border(
        self,
        color: impl Into<Color>,
        width: impl Into<Pixels>,
    ) -> Self {
        Self {
            border: Border {
                color: color.into(),
                width: width.into().0,
                ..Border::default()
            },
            ..self
        }
    }

    pub fn with_background(self, background: impl Into<Background>) -> Self {
        Self {
            background: Some(background.into()),
            ..self
        }
    }
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            shadow_offset: Vector::default(),
            background: Some(Background::Color(
                Color {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: 0.6,
                }
            )),
            border: Border::default(),
            shadow: Shadow::default(),
        }
    }
}

/// A set of rules that dictate the style of AreaSelector.
pub trait StyleSheet {
    /// The supported style of the [`StyleSheet`].
    type Style: Default;

    fn appearance(&self, style: &Self::Style) -> Appearance;
}
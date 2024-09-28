use crate::gui::components::screenAreaStyle::{Appearance, StyleSheet};
use iced::Theme;
use iced_core::{Border, Shadow};

/// The style of a AreaSelector.
#[derive(Default)]
pub enum AreaSelector {
    /// No style.
    #[default]
    Translucent,
    /// A simple box.
    Box,
}

impl StyleSheet for Theme {
    type Style = AreaSelector;

    fn appearance(&self, style: &Self::Style) -> Appearance {
        match style {
            AreaSelector::Translucent => Appearance::default(),
            AreaSelector::Box => {
                let palette = self.extended_palette();

                Appearance {
                    shadow_offset: Default::default(),
                    background: Some(palette.background.weak.color.into()),
                    border: Border::default().rounded(10),
                    shadow: Shadow::default(),
                }
            }
        }
    }
}

impl<T: Fn(&Theme) -> Appearance> StyleSheet for T {
    type Style = Theme;

    fn appearance(&self, style: &Self::Style) -> Appearance {
        self(style)
    }
}
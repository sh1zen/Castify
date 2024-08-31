use crate::gui::theme::styles::csx::StyleType;
use iced::{Background, Border, Color, Shadow};
use crate::gui::components::raw::screenAreaStyle::Appearance;

#[derive(Clone, Copy, Default)]
pub enum AreaSelType {
    #[default]
    Standard,
}

impl crate::gui::components::raw::screenAreaStyle::StyleSheet for StyleType {
    type Style = AreaSelType;

    fn appearance(&self, style: &Self::Style) -> Appearance {
        let colors = self.get_palette();
        Appearance {
            shadow_offset: Default::default(),
            background: Some(Background::Color(Color::TRANSPARENT)),
            border: Border {
                radius: 8.0.into(),
                width: 0.0,
                color: Color {
                    a: 0.8,
                    ..colors.primary
                },
            },
            shadow: Shadow::default(),
        }
    }
}
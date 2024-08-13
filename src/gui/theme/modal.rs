use iced::Color;
use iced_aw::style::modal::Appearance;
use crate::gui::theme::styles::csx::StyleType;

impl iced_aw::widgets::modal::StyleSheet for StyleType {
    type Style = ();

    fn active(&self, style: &Self::Style) -> Appearance {
        Appearance {
            background: Color {
                r: 45.0 / 255.0,
                g: 45.0 / 255.0,
                b: 45.0 / 255.0,
                a: 0.6,
            }.into(),
        }
    }
}
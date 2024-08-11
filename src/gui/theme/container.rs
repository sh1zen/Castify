use iced::widget::container::Appearance;
use iced::{Background, Border, Color, Shadow};
use crate::gui::theme::styles::csx::StyleType;

#[derive(Clone, Copy, Default)]
pub enum ContainerType {
    #[default]
    Standard,
    BorderedRound,
    Tooltip,
    Badge,
    Modal,
    Highlighted,
}

impl iced::widget::container::StyleSheet for StyleType {
    type Style = ContainerType;

    fn appearance(&self, style: &Self::Style) -> Appearance {
        let colors = self.get_palette();
        Appearance {
            text_color: Some(match style {
                ContainerType::Highlighted => colors.text_headers,
                _ => colors.text_body,
            }),
            background: Some(match style {
                ContainerType::Highlighted => {
                    Background::Color(colors.secondary)
                }
                ContainerType::Tooltip => Background::Color(colors.primary),
                ContainerType::BorderedRound => Background::Color(Color {
                    a: 0.2,
                    ..colors.primary
                }),
                ContainerType::Badge => Background::Color(Color {
                    a: 0.8,
                    ..colors.secondary
                }),
                ContainerType::Modal => {
                    Background::Color(colors.primary)
                }
                ContainerType::Standard => {
                    Background::Color(Color::TRANSPARENT)
                }
            }),
            border: Border {
                radius: match style {
                    ContainerType::BorderedRound => 15.0.into(),
                    ContainerType::Modal => {
                        [0.0, 0.0, 15.0, 15.0].into()
                    }
                    ContainerType::Tooltip => 7.0.into(),
                    ContainerType::Badge | ContainerType::Highlighted => 100.0.into(),
                    _ => 0.0.into(),
                },
                width: match style {
                    ContainerType::Standard
                    | ContainerType::Modal
                    | ContainerType::Highlighted => 0.0,
                    ContainerType::Tooltip => 1.0,
                    ContainerType::BorderedRound => 4.0,
                    _ => 2.0,
                },
                color: match style {
                    _ => Color {
                        a: 0.8,
                        ..colors.primary
                    },
                },
            },
            shadow: Shadow::default(),
        }
    }
}
use crate::gui::theme::styles::csx::StyleType;
use iced::widget::container::Appearance;
use iced::{Background, Border, Color, Shadow};

#[derive(Clone, Copy, Default)]
pub enum ContainerType {
    #[default]
    Standard,
    Tooltip,
    Badge,
    Modal,
    Video,
}

impl iced::widget::container::StyleSheet for StyleType {
    type Style = ContainerType;

    fn appearance(&self, style: &Self::Style) -> Appearance {
        let colors = self.get_palette();
        Appearance {
            text_color: Some(colors.text_body),
            background: Some(match style {
                ContainerType::Tooltip => Background::Color(colors.primary),
                ContainerType::Badge => Background::Color(Color {
                    a: 0.8,
                    ..colors.secondary
                }),
                ContainerType::Modal => {
                    Background::Color(colors.primary)
                }
                _ =>  {
                    Background::Color(Color::TRANSPARENT)
                }
            }),
            border: Border {
                radius: match style {
                    ContainerType::Video => 0.0.into(),
                    ContainerType::Tooltip => 7.0.into(),
                    ContainerType::Badge => 100.0.into(),
                    _ => 8.0.into(),
                },
                width: match style {
                    ContainerType::Standard
                    | ContainerType::Modal => 0.0,
                    ContainerType::Tooltip => 1.0,
                    _ => 0.0,
                },
                color: match style {
                    _ => Color {
                        a: 0.8,
                        ..colors.primary
                    },
                },
            },
            shadow: Shadow::default()
        }
    }
}
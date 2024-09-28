use crate::gui::style::styles::csx::StyleType;
use iced::widget::container::{Catalog, Style};
use iced_core::{Background, Border, Color, Shadow, Vector};

#[derive(Clone, Copy, Debug, Default)]
#[allow(dead_code)]
pub enum ContainerType {
    #[default]
    Standard,
    Tooltip,
    Badge,
    Modal,
    Video,
}

impl Catalog for StyleType {
    type Class<'a> = ContainerType;

    fn default<'a>() -> Self::Class<'a> {
        ContainerType::Standard
    }

    fn style(&self, class: &Self::Class<'_>) -> Style {
        let colors = self.get_palette();
        Style {
            background: Some(match class {
                ContainerType::Tooltip => Background::Color(colors.primary),
                ContainerType::Badge => Background::Color(Color {
                    a: 0.8,
                    ..colors.secondary
                }),
                ContainerType::Modal => {
                    Background::Color(colors.primary)
                }
                _ => {
                    Background::Color(Color::TRANSPARENT)
                }
            }),
            border: Border {
                radius: match class {
                    ContainerType::Video => 0.0.into(),
                    ContainerType::Tooltip => 7.0.into(),
                    ContainerType::Badge => 100.0.into(),
                    _ => 8.0.into(),
                },
                width: match class {
                    ContainerType::Standard
                    | ContainerType::Modal => 0.0,
                    ContainerType::Tooltip => 1.0,
                    _ => 0.0,
                },
                color: match class {
                    _ => Color {
                        a: 0.8,
                        ..colors.primary
                    },
                },
            },
            text_color: Some(colors.text_body),
            shadow: Shadow {
                color: Color::TRANSPARENT,
                offset: Vector::ZERO,
                blur_radius: 0.0,
            },
        }
    }
}
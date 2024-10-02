use crate::gui::style::styles::csx::StyleType;
use iced::widget::container::{Catalog, Style};
use iced_core::{Background, Border, Color, Shadow, Vector};

#[derive(Clone, Copy, Debug, Default)]
#[allow(dead_code)]
pub enum ContainerType {
    #[default]
    Transparent,
    Standard,
    Modal,
    Video,
    Footer,
    DarkFilter,
}

impl Catalog for StyleType {
    type Class<'a> = ContainerType;

    fn default<'a>() -> Self::Class<'a> {
        ContainerType::Transparent
    }

    fn style(&self, class: &Self::Class<'_>) -> Style {
        let palette = self.get_palette();
        Style {
            background: Some(match class {
                ContainerType::Video => Background::Color(Color::BLACK),
                ContainerType::Standard | ContainerType::Footer => Background::Color(palette.primary_darker),
                ContainerType::Modal => {
                    Background::Color(palette.primary_darker)
                }
                ContainerType::DarkFilter => {
                    Background::Color(Color { a: 0.8, ..Color::BLACK })
                }
                _ => {
                    Background::Color(Color::TRANSPARENT)
                }
            }),
            border: Border {
                radius: match class {
                    ContainerType::Video => 3.0.into(),
                    ContainerType::Modal => 8.0.into(),
                    ContainerType::Standard => 6.0.into(),
                    _ => 0.0.into(),
                },
                width: match class {
                    ContainerType::Modal => 1.0,
                    _ => 0.0,
                },
                color: match class {
                    ContainerType::Modal => Color {
                        a: 0.6,
                        ..Color::BLACK
                    },
                    _ => Color::TRANSPARENT,
                },
            },
            text_color: Some(palette.text),
            shadow: match class {
                ContainerType::Modal => Shadow {
                    color: Color::BLACK,
                    offset: Vector::ZERO,
                    blur_radius: 3.0,
                },
                _ => Shadow::default(),
            },
        }
    }
}
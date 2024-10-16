use crate::assets::BORDER_RADIUS;
use crate::gui::style::theme::csx::StyleType;
use iced::widget::slider::{Catalog, Handle, HandleShape, Rail, Status, Style};
use iced::{Background, Border, Color};

#[derive(Clone, Copy, Debug, Default)]
pub enum ScrollbarType {
    #[default]
    Standard,
}

impl Catalog for StyleType {
    type Class<'a> = ScrollbarType;

    fn default<'a>() -> Self::Class<'a> {
        ScrollbarType::Standard
    }

    fn style(&self, _class: &Self::Class<'_>, status: Status) -> Style {
        let palette = self.get_palette();

        let color = match status {
            Status::Active => palette.primary,
            Status::Hovered => palette.active(palette.primary),
            Status::Dragged => palette.active(palette.primary),
        };

        let rail = Rail {
            backgrounds: (
                Background::Color(Color {
                    ..palette.primary
                }), Background::Color(Color {
                    ..palette.primary_darker
                })),
            width: 0.0,
            border: Border {
                radius: BORDER_RADIUS.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },

        };

        Style {
            rail,
            handle: Handle {
                shape:  HandleShape::Circle { radius: 7.0 },
                background:  Background::Color(color),
                border_color: Color::TRANSPARENT,
                border_width: 0.0,
            },
        }
    }
}
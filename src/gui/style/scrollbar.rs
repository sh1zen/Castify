use crate::assets::BORDER_RADIUS;
use crate::gui::style::theme::color::mix;
use crate::gui::style::theme::csx::StyleType;
use iced::widget::scrollable::Scroller;
use iced::widget::scrollable::{Catalog, Rail, Status, Style};
use iced::{Background, Border, Color};
use iced::widget::container;

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

        let rail = Rail {
            background: Some(Background::Color(Color {
                a: 0.2,
                ..palette.primary_darker
            })),
            border: Border {
                radius: BORDER_RADIUS.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
            scroller: Scroller {
                color: mix(palette.secondary, palette.primary_darker),
                border: Border {
                    radius: BORDER_RADIUS.into(),
                    width: 0.0,
                    color: Color::TRANSPARENT,
                },
            },
        };

        let base = Style {
            container: container::Style{
                text_color: Some(palette.text),
                background: Some(Background::Color(palette.disabled(palette.background))),
                border: Default::default(),
                shadow: Default::default(),
            },
            vertical_rail: rail,
            horizontal_rail: rail,
            gap: None,
        };

        let operative = Style {
            vertical_rail: Rail {
                scroller: Scroller {
                    color: palette.secondary,
                    border: Border {
                        radius: BORDER_RADIUS.into(),
                        width: 0.0,
                        color: Color::TRANSPARENT,
                    },
                },
                ..rail
            },
            horizontal_rail: Rail {
                scroller: Scroller {
                    color: palette.secondary,
                    border: Border {
                        radius: BORDER_RADIUS.into(),
                        width: 0.0,
                        color: Color::TRANSPARENT,
                    },
                },
                ..rail
            },
            ..base
        };

        match status {
            Status::Active => base,
            Status::Hovered { .. } => operative,
            Status::Dragged { .. } => operative,
        }
    }
}
use crate::gui::resource::BORDER_RADIUS;
use crate::gui::style::color::mix;
use crate::gui::style::styles::csx::StyleType;
use iced::widget::scrollable::Scroller;
use iced::widget::scrollable::{Catalog, Rail, Status, Style};
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
        let colors = self.get_palette();
        let buttons_color = colors.generate_element_color();

        let rail = Rail {
            background: Some(Background::Color(Color {
                a: 0.2,
                ..buttons_color
            })),
            border: Border {
                radius: BORDER_RADIUS.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
            scroller: Scroller {
                color: mix(colors.secondary, buttons_color),
                border: Border {
                    radius: BORDER_RADIUS.into(),
                    width: 0.0,
                    color: Color::TRANSPARENT,
                },
            },
        };

        let base = Style {
            container: Default::default(),
            vertical_rail: rail,
            horizontal_rail: rail,
            gap: None,
        };

        let operative = Style {
            vertical_rail: Rail {
                scroller: Scroller {
                    color: colors.secondary,
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
                    color: colors.secondary,
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
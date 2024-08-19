use crate::gui::resource::{BORDER_RADIUS};
use crate::gui::theme::color::mix;
use iced::widget::container;
use iced::widget::scrollable::{Appearance, Properties};
use iced::widget::scrollable::{Scrollbar, Scroller};
use iced::{Background, Border, Color};
use crate::gui::theme::styles::csx::StyleType;

#[derive(Clone, Copy, Default)]
pub enum ScrollbarType {
    #[default]
    Standard,
}

impl ScrollbarType {
    pub fn properties() -> Properties {
        Properties::new().width(5).scroller_width(5).margin(3)
    }
}

impl iced::widget::scrollable::StyleSheet for StyleType {
    type Style = ScrollbarType;

    fn active(&self, _: &Self::Style) -> Appearance {

        let colors = self.get_palette();
        let buttons_color = colors.generate_buttons_color();
        Appearance {
            container: container::Appearance::default(),
            scrollbar: Scrollbar {
                background: Some(Background::Color(Color::TRANSPARENT)),
                scroller: Scroller {
                    color: Color {
                        a: 0.1,
                        ..buttons_color
                    },
                    border: Border {
                        radius: BORDER_RADIUS.into(),
                        width: 0.0,
                        color: Color::TRANSPARENT,
                    },
                },
                border: Border {
                    radius: BORDER_RADIUS.into(),
                    width: 0.0,
                    color: Color::TRANSPARENT,
                },
            },
            gap: None,
        }
    }

    fn hovered(&self, _: &Self::Style, is_mouse_over_scrollbar: bool) -> Appearance {
        let colors = self.get_palette();
        let buttons_color = colors.generate_buttons_color();
        Appearance {
            container: container::Appearance::default(),
            scrollbar: Scrollbar {
                background: Some(Background::Color(Color {
                    a: 0.2,
                    ..buttons_color
                })),
                scroller: Scroller {
                    color: if is_mouse_over_scrollbar {
                        colors.secondary
                    } else {
                        mix(colors.secondary, buttons_color)
                    },
                    border: Border {
                        radius: BORDER_RADIUS.into(),
                        width: 0.0,
                        color: Color::TRANSPARENT,
                    },
                },
                border: Border {
                    radius: BORDER_RADIUS.into(),
                    width: 0.0,
                    color: Color::TRANSPARENT,
                },
            },
            gap: None,
        }
    }
}
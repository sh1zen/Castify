use crate::gui::resource::{BORDER_RADIUS, BORDER_WIDTH};
use crate::gui::theme::color::mix;
use crate::gui::theme::styles::csx::StyleType;
use iced::widget::pick_list;
use iced::widget::pick_list::StyleSheet;
use iced::{Background, Border, Color};

#[derive(Clone, Copy, Default)]
pub enum PicklistType {
    #[default]
    Standard,
}


impl iced::overlay::menu::StyleSheet for StyleType {
    type Style = PicklistType;

    fn appearance(&self, _: &Self::Style) -> iced::overlay::menu::Appearance {
        let colors = self.get_palette();
        let buttons_color = colors.generate_buttons_color();
        iced::overlay::menu::Appearance {
            text_color: colors.text_body,
            background: Background::Color(buttons_color),
            border: Border {
                width: BORDER_WIDTH,
                radius: BORDER_RADIUS.into(),
                color: colors.secondary,
            },
            selected_text_color: colors.text_body,
            selected_background: Background::Color(mix(buttons_color, colors.primary)),
        }
    }
}

impl StyleSheet for StyleType {
    type Style = PicklistType;

    fn active(&self, _: &Self::Style) -> pick_list::Appearance {
        let colors = self.get_palette();
        let buttons_color = colors.generate_buttons_color();
        pick_list::Appearance {
            text_color: colors.text_body,
            placeholder_color: colors.text_body,
            handle_color: colors.text_body,
            background: Background::Color(Color { a: 0.7, ..buttons_color }),
            border: Border {
                radius: BORDER_RADIUS.into(),
                width: 0.0,
                color: colors.primary_darker,
            },
        }
    }

    fn hovered(&self, style: &Self::Style) -> pick_list::Appearance {
        let colors = self.get_palette();
        let buttons_color = colors.generate_buttons_color();
        pick_list::Appearance {
            background: Background::Color(Color { a: 0.9, ..buttons_color }),
            ..self.active(style)
        }
    }
}
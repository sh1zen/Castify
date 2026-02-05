use crate::assets::{BORDER_RADIUS, BORDER_WIDTH};
use crate::gui::style::theme::csx::StyleType;
use iced::overlay::menu::{Catalog, Style};
use iced::{Background, Border};

#[derive(Clone, Copy, Debug, Default)]
pub enum MenuType {
    #[default]
    Standard,
}
impl Catalog for StyleType {
    type Class<'a> = MenuType;

    fn default<'a>() -> <Self as Catalog>::Class<'a> {
        MenuType::Standard
    }

    fn style(&self, _class: &<Self as Catalog>::Class<'_>) -> Style {
        let palette = self.get_palette();
        Style {
            text_color: palette.text,
            background: Background::Color(palette.primary),
            border: Border {
                width: BORDER_WIDTH,
                radius: BORDER_RADIUS.into(),
                color: palette.primary_darker,
            },
            selected_text_color: palette.text,
            selected_background: Background::Color(palette.secondary),
            shadow: Default::default(),
        }
    }
}

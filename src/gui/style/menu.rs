use crate::gui::resource::{BORDER_RADIUS, BORDER_WIDTH};
use crate::gui::style::color::mix;
use crate::gui::style::styles::csx::StyleType;
use iced::overlay::menu::{Catalog, Style};
use iced_core::{Background, Border};

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
        let colors = self.get_palette();
        let buttons_color = colors.generate_element_color();
        Style {
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
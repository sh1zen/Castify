use crate::gui::style::theme::color::mix;
use crate::gui::style::theme::csx::StyleType;
use iced::Color;

#[derive(Clone, Copy, Debug, Default)]

pub enum TextType {
    #[default]
    Standard,
    Title,
    Subtitle,
    Danger,
    White,
    Colored(Color),
}

impl TextType {
    pub fn maybe_colored(color: Option<Color>) -> TextType {
        if let Some(color) = color {
            TextType::Colored(color)
        } else {
            TextType::Standard
        }
    }
}

impl iced::widget::text::Catalog for StyleType {
    type Class<'a> = TextType;

    fn default<'a>() -> Self::Class<'a> {
        TextType::Standard
    }

    fn style(&self, class: &Self::Class<'_>) -> iced::widget::text::Style {
        let palette = self.get_palette();
        iced::widget::text::Style {
            color: Some(match class {
                TextType::Standard => palette.text,
                TextType::Title => palette.title_text(),
                TextType::Subtitle => palette.subtitle_text(),
                TextType::Danger => mix(palette.danger, palette.text),
                TextType::White => Color::WHITE,
                TextType::Colored(color) => *color,
            }),
        }
    }
}

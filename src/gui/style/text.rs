use crate::gui::common::messages::AppEvent;
use crate::gui::style::color::mix;
use crate::gui::style::styles::csx::StyleType;
use crate::gui::widget::{Column, Text};
use iced_core::Color;
use iced_core::Font;

#[derive(Clone, Copy, Debug, Default)]
#[allow(dead_code)]
pub enum TextType {
    #[default]
    Standard,
    Title,
    Subtitle,
    Danger,
    White,
}

impl TextType {
    pub fn highlighted_subtitle_with_desc(
        subtitle: &str,
        desc: &str,
        font: Font,
    ) -> Column<'static, AppEvent> {
        Column::new()
            .push(
                Text::new(format!("{subtitle}:"))
                    .class(TextType::Subtitle)
                    .font(font),
            )
            .push(Text::new(format!("   {desc}")).font(font))
    }
}

impl iced_core::widget::text::Catalog for StyleType {
    type Class<'a> = TextType;

    fn default<'a>() -> Self::Class<'a> {
        TextType::Standard
    }

    fn style(&self, class: &Self::Class<'_>) -> iced_core::widget::text::Style {
        let palette = self.get_palette();
        iced_core::widget::text::Style {
            color: Some(match class {
                TextType::Standard => palette.text,
                TextType::Title => {
                    let color = if palette.is_nightly { Color::WHITE } else { Color::BLACK };
                    let (p1, c) = if palette.is_nightly { (0.4, 1.0) } else { (0.6, 0.7) };
                    Color {
                        r: c * (1.0 - p1) + color.r * p1,
                        g: c * (1.0 - p1) + color.g * p1,
                        b: c * (1.0 - p1) + color.b * p1,
                        a: 1.0,
                    }
                }
                TextType::Subtitle => {
                    let color = if palette.is_nightly { Color::WHITE } else { Color::BLACK };
                    let (p1, c) = if palette.is_nightly { (0.4, 0.8) } else { (0.6, 0.5) };
                    Color {
                        r: c * (1.0 - p1) + color.r * p1,
                        g: c * (1.0 - p1) + color.g * p1,
                        b: c * (1.0 - p1) + color.b * p1,
                        a: 1.0,
                    }
                }
                TextType::Danger => mix(palette.danger, palette.text),
                TextType::White => Color::WHITE,
            })
        }
    }
}


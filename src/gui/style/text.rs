use crate::gui::style::styles::csx::StyleType;
use crate::gui::common::messages::AppEvent;
use crate::gui::widget::{Column, Text};
use iced_core::Color;
use iced_core::Font;

#[derive(Clone, Copy, Debug, Default)]
#[allow(dead_code)]
pub enum TextType {
    #[default]
    Standard,
    Secondary,
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
        iced_core::widget::text::Style {
            color: match class {
                TextType::Standard => None,
                _ => Some(highlight(&self, class))
            }
        }
    }
}

pub fn highlight(style: &StyleType, element: &TextType) -> Color {
    let colors = style.get_palette();
    let secondary = colors.secondary;
    let is_nightly = style.get_palette().is_nightly;
    match element {
        TextType::Title => {
            let (p1, c) = if is_nightly { (0.6, 1.0) } else { (0.9, 0.7) };
            Color {
                r: c * (1.0 - p1) + secondary.r * p1,
                g: c * (1.0 - p1) + secondary.g * p1,
                b: c * (1.0 - p1) + secondary.b * p1,
                a: 1.0,
            }
        }
        TextType::Subtitle => {
            let (p1, c) = if is_nightly { (0.4, 1.0) } else { (0.6, 0.7) };
            Color {
                r: c * (1.0 - p1) + secondary.r * p1,
                g: c * (1.0 - p1) + secondary.g * p1,
                b: c * (1.0 - p1) + secondary.b * p1,
                a: 1.0,
            }
        }
        TextType::Secondary => colors.secondary,
        TextType::Danger => Color::from_rgb(0.8, 0.15, 0.15),
        TextType::Standard => colors.text_body,
        TextType::White => colors.highlight,
    }
}
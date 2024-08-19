use crate::gui::theme::styles::csx::StyleType;
use crate::gui::types::messages::Message;
use iced::widget::text::Appearance;
use iced::widget::{Column, Text};
use iced::{Color, Font};

#[derive(Clone, Copy, Default, PartialEq)]
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
    ) -> Column<'static, Message, StyleType> {
        Column::new()
            .push(
                Text::new(format!("{subtitle}:"))
                    .style(TextType::Subtitle)
                    .font(font),
            )
            .push(Text::new(format!("   {desc}")).font(font))
    }
}

impl iced::widget::text::StyleSheet for StyleType {
    type Style = TextType;

    fn appearance(&self, style: Self::Style) -> Appearance {
        Appearance {
            color: if style == TextType::Standard {
                None
            } else {
                Some(highlight(*self, style))
            },
        }
    }
}

pub fn highlight(style: StyleType, element: TextType) -> Color {
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
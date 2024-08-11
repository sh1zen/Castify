use std::hash::{Hash, Hasher};
use iced::{Color, Font};
use crate::gui::theme::color::color_hash;
use crate::gui::theme::styles::csx::StyleType;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    /// Main color of the GUI (background, hovered buttons, active tab)
    pub primary: Color,
    /// Secondary color of the GUI (header, footer, buttons' borders)
    pub secondary: Color,
    /// Color of alert
    pub alert: Color,
    /// Color of favorites' star symbol
    pub starred: Color,
    /// Color of header and footer text
    pub text_headers: Color,
    /// Color of body and buttons text
    pub text_body: Color,
    /// the font used
    pub font: Font,
    /// if is nightly
    pub is_nightly: bool,
}

impl Palette {
    pub fn generate_buttons_color(mut self) -> Color {
        let primary = self.primary;
        self.is_nightly = primary.r + primary.g + primary.b <= 1.5;
        if self.is_nightly {
            Color {
                r: f32::min(primary.r + 0.15, 1.0),
                g: f32::min(primary.g + 0.15, 1.0),
                b: f32::min(primary.b + 0.15, 1.0),
                a: 1.0,
            }
        } else {
            Color {
                r: f32::max(primary.r - 0.15, 0.0),
                g: f32::max(primary.g - 0.15, 0.0),
                b: f32::max(primary.b - 0.15, 0.0),
                a: 1.0,
            }
        }
    }
}

impl Default for Palette {
    fn default() -> Self {
        StyleType::get_palette(StyleType::Venus)
    }
}


impl Hash for Palette {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let Palette {
            primary,
            secondary,
            alert,
            starred,
            text_headers,
            text_body,
            font,
            is_nightly
        } = self;

        color_hash(*primary, state);
        color_hash(*secondary, state);
        color_hash(*alert, state);
        color_hash(*starred, state);
        color_hash(*text_headers, state);
        color_hash(*text_body, state);
        font.hash(state);
        is_nightly.hash(state);
    }
}
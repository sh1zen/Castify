use iced::Color;
use std::hash::{Hash, Hasher};

pub fn mix(bg: Color, fg: Color) -> Color {
    let a = 1. - (1. - fg.a) * (1. - bg.a);
    Color {
        r: fg.r * fg.a / a + bg.r * bg.a * (1. - fg.a) / a,
        g: fg.g * fg.a / a + bg.g * bg.a * (1. - fg.a) / a,
        b: fg.b * fg.a / a + bg.b * bg.a * (1. - fg.a) / a,
        a,
    }
}

pub fn color_hash<H: Hasher>(color: Color, state: &mut H) {
    // Hash isn't implemented for floats, so lets hash the color as RGBA instead.
    let color = color.into_rgba8();
    color.hash(state);
}

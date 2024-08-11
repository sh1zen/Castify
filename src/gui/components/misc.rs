use iced::advanced::widget::Text;
use iced::{Alignment, Font};
use iced::widget::Row;
use crate::gui::resource::RALEWAY_FONT;
use crate::gui::theme::styles::csx::StyleType;
use crate::gui::theme::text::TextType;
use crate::gui::types::icons::Icon;
use crate::gui::types::messages::Message;

pub fn row_open_link_tooltip(text: &'static str) -> Row<'static, Message, StyleType> {
    Row::new()
        .align_items(Alignment::Center)
        .spacing(8)
        .push(Text::new(text).font(Font::from(RALEWAY_FONT)).size(10))
        .push(Icon::Browser.to_text().size(20))
}
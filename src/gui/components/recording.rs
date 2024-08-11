use iced::{Alignment, Length};
use iced::widget::{Container, Row};

use crate::gui::theme::buttons::FilledButton;
use crate::gui::theme::styles::csx::StyleType;
use crate::gui::types::appbase::App;
use crate::gui::types::icons::Icon;
use crate::gui::types::messages::Message as appMessage;

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Rec,
    Pause,
}

pub fn recording_page(app: &App) -> Container<appMessage, StyleType> {
    let content = Row::new()
        .align_items(iced::Alignment::Center).spacing(10)
        .push(FilledButton::new("Rec").icon(Icon::Video).build().on_press(
            appMessage::Ignore
        ))
        .push(FilledButton::new("Pause").icon(Icon::Pause).build().on_press(
            appMessage::Ignore
        ))
        .height(400)
        .align_items(Alignment::Center);

    Container::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x().center_y()
}
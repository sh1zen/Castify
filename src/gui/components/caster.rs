use crate::gui::theme::buttons::FilledButton;
use crate::gui::theme::styles::csx::StyleType;
use crate::gui::types::appbase::App;
use crate::gui::types::icons::Icon;
use crate::gui::types::messages::Message as appMessage;
use crate::workers;
use iced::widget::{Container, Row};
use iced::{Alignment, Length};

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Rec,
    Pause,
}

pub fn caster_page(_: &App) -> Container<appMessage, StyleType> {
    let action = if workers::caster::get_instance().lock().unwrap().streaming {
        FilledButton::new("Pause").icon(Icon::Pause).build().on_press(
            appMessage::Caster(Message::Pause)
        )
    } else {
        FilledButton::new("Rec").icon(Icon::Video).build().on_press(
            appMessage::Caster(Message::Rec)
        )
    };

    let content = Row::new()
        .align_items(iced::Alignment::Center).spacing(10)
        .push(action)
        .height(400)
        .align_items(Alignment::Center);

    Container::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x().center_y()
}
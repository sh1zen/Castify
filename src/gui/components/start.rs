use crate::gui::theme::buttons::FilledButton;
use crate::gui::theme::styles::buttons::ButtonType;
use crate::gui::theme::styles::csx::StyleType;
use crate::gui::types::appbase::App;
use crate::gui::types::icons::Icon;
use crate::gui::types::messages::Message as appMessage;
use iced::widget::{Container, Row};
use iced::{Alignment, Length};

#[derive(Debug, Clone)]
pub enum Message {
    ButtonCaster,
    ButtonReceiver,
}

pub fn initial_page(_: &App) -> Container<appMessage, StyleType> {
    let content = Row::new()
        .align_items(Alignment::Center).spacing(10)
        .push(FilledButton::new("Caster")
            .icon(Icon::Cast)
            .style(ButtonType::Standard)
            .build()
            .on_press(appMessage::Mode(Message::ButtonCaster)
            ))
        .push(
            FilledButton::new("Receiver")
                .icon(Icon::Connection)
                .style(ButtonType::Standard)
                .build()
                .on_press(appMessage::Mode(Message::ButtonReceiver)
                ))
        .push(
            FilledButton::new("Hotkeys")
                .icon(Icon::Settings)
                .style(ButtonType::Standard)
                .build()
                .on_press(appMessage::HotkeysPage)
        )
        .height(400)
        .align_items(Alignment::Center);

    Container::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x().center_y()
}

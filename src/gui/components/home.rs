use crate::gui::style::button::ButtonType;
use crate::gui::style::buttons::FilledButton;
use crate::gui::common::icons::Icon;
use crate::gui::common::messages::AppEvent as appMessage;
use crate::gui::widget::{Container, Row, Text};
use iced::{Alignment, Length};
use iced_core::alignment;

#[derive(Debug, Clone)]
pub enum Message {
    ButtonCaster,
    ButtonReceiver,
}
pub fn initial_page<'a>(os_supported: bool) -> Container<'a, appMessage> {
    let content = if !os_supported {
        Row::new().push(Text::new("Platform not supported!"))
    } else {
        Row::new()
            .align_y(Alignment::Center)
            .spacing(10)
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
            .align_y(Alignment::Center)
    };

    Container::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
}

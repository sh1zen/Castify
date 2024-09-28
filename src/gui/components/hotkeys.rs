use crate::gui::style::buttons::FilledButton;
use crate::gui::common::icons::Icon;
use crate::gui::common::messages::AppEvent as appMessage;
use crate::gui::widget::{Column, Container, Row};
use iced::{Alignment, Length};
use iced_core::alignment;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum KeyTypes {
    Pause,
    Record,
    Close,
    BlankScreen,
    None,
}

pub fn hotkeys<'a>() -> Container<'a, appMessage> {
    let actions = Column::new()
        .push(
            Row::new()
                .align_y(Alignment::Center).spacing(10)
                .push(
                    FilledButton::new("Pause")
                        .icon(Icon::Pause)
                        .build()
                        .on_press(appMessage::HotkeysTypePage(KeyTypes::Pause))
                )
                .push(
                    FilledButton::new("Record")
                        .icon(Icon::Stop)
                        .build()
                        .on_press(appMessage::HotkeysTypePage(KeyTypes::Record))
                ))
        .push(
            Row::new()
                .align_y(Alignment::Center).spacing(10)
                .push(
                    FilledButton::new("Terminate")
                        .icon(Icon::Stop)
                        .build()
                        .on_press(appMessage::HotkeysTypePage(KeyTypes::Close))
                )
                .push(
                    FilledButton::new("Blank Screen")
                        .icon(Icon::Banned)
                        .build()
                        .on_press(appMessage::HotkeysTypePage(KeyTypes::BlankScreen))
                )
        )
        .width(Length::Fill)
        .align_x(Alignment::Center)
        .spacing(10);

    let content = Column::new()
        .push(actions)
        .push(
            FilledButton::new("Home")
                .icon(Icon::Browser)
                .build()
                .on_press(appMessage::Home)
        )
        .width(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .spacing(40);

    Container::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
}
use crate::gui::theme::buttons::FilledButton;
use crate::gui::theme::styles::csx::StyleType;
use crate::gui::appbase::App;
use crate::gui::types::icons::Icon;
use crate::gui::types::messages::Message as appMessage;
use iced::widget::{Column, Container, Row};
use iced::{Alignment, Length};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum KeyTypes {
    Pause,
    Record,
    Close,
    BlankScreen,
    None,
}

pub fn hotkeys(_: &App) -> Container<appMessage, StyleType> {
    let actions = Column::new()
        .push(
            Row::new()
                .align_items(Alignment::Center).spacing(10)
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
                .align_items(Alignment::Center).spacing(10)
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
        .align_items(Alignment::Center)
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
        .align_items(Alignment::Center)
        .spacing(40);

    Container::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x().center_y()
}
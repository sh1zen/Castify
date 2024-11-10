use crate::assets::FONT_FAMILY_BOLD;
use crate::gui::common::icons::Icon;
use crate::gui::components::button::IconButton;
use crate::gui::style::container::ContainerType;
use crate::gui::widget::{horizontal_space, vertical_space, Container, Element, Row, Text};
use crate::gui::windows::main::MainWindowEvent;
use iced::{Alignment, Length};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum KeyTypes {
    Pause,
    Record,
    Close,
    BlankScreen,
    None,
}

pub fn hotkeys<'a>() -> Element<'a, MainWindowEvent> {
    let header = Container::new(
        crate::row![
            horizontal_space().width(Length::Fill),
            Text::new("Customize Shortcuts").font(FONT_FAMILY_BOLD).size(18),
            horizontal_space().width(Length::Fill),
        ].align_y(Alignment::Center)
    ).center(Length::Fill).height(80).class(ContainerType::Standard);

    let hotkeys = Container::new(
        crate::column![
            Row::new()
                .align_y(Alignment::Center).spacing(15)
                .push(
                    IconButton::new()
                        .label("Pause")
                        .icon(Icon::Pause)
                        .build().width(160)
                        .on_press(MainWindowEvent::HotkeysTypePage(KeyTypes::Pause))
                )
                .push(
                    IconButton::new()
                        .label("Record")
                        .icon(Icon::Video)
                        .build().width(160)
                        .on_press(MainWindowEvent::HotkeysTypePage(KeyTypes::Record))
                ),

            Row::new()
                .align_y(Alignment::Center).spacing(15)
                .push(
                    IconButton::new()
                        .label("Terminate")
                        .icon(Icon::Stop)
                        .build().width(160)
                        .on_press(MainWindowEvent::HotkeysTypePage(KeyTypes::Close))
                )
                .push(
                    IconButton::new()
                        .label("Blank Screen")
                        .icon(Icon::Banned)
                        .build().width(160)
                        .on_press(MainWindowEvent::HotkeysTypePage(KeyTypes::BlankScreen))
                )
        ]
            .width(Length::Fill)
            .align_x(Alignment::Center)
            .spacing(15)
    ).center(Length::Fill).height(160).class(ContainerType::Standard);

    let actions = Container::new(
        crate::row![
            horizontal_space().width(Length::Fill),
            IconButton::new()
                .label("Home")
                .icon(Icon::Home)
                .build()
                .on_press(MainWindowEvent::Home),
            horizontal_space().width(Length::Fill),
        ].align_y(Alignment::Center)
    ).center(Length::Fill).height(80).class(ContainerType::Standard);

    let content = crate::column![header, hotkeys, vertical_space(), actions].spacing(10).padding(15);

    Container::new(content).center(Length::Fill).into()
}
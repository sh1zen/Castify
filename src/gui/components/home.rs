use crate::assets::{APP_NAME, FONT_FAMILY_BOLD};
use crate::config::Config;
use crate::gui::common::icons::Icon;
use crate::gui::components::buttons::{IconButton, Key4Board};
use crate::gui::style::button::ButtonType;
use crate::gui::style::container::ContainerType;
use crate::gui::widget::{horizontal_space, vertical_space, Container, Element, Row, Text};
use crate::gui::windows::main::{MainWindow, MainWindowEvent};
use iced::{Alignment, Length};
use iced_core::alignment;
use iced_core::keyboard::{Key, Modifiers};

#[derive(Debug, Clone)]
pub enum Message {
    ButtonCaster,
    ButtonReceiver,
}
pub fn initial_page<'a>(main_window: &MainWindow, config: &Config) -> Element<'a, MainWindowEvent> {
    let header = crate::row![
            Text::new(APP_NAME).size(40).font(FONT_FAMILY_BOLD),
            horizontal_space(),
            IconButton::new(Some(String::from("Exit"))).style(ButtonType::Danger).build()
                .on_press(MainWindowEvent::ExitApp)
                .height(40)
                .width(80),
        ]
        .align_y(alignment::Vertical::Center);

    let body = crate::column![
            Container::new(
                crate::row![
                    horizontal_space(),
                    IconButton::new(Some(String::from("Hotkeys")))
                        .icon(Icon::Settings)
                        .style(ButtonType::Standard)
                        .build()
                        .on_press(MainWindowEvent::HotkeysPage),
                    horizontal_space().width(10),
                    IconButton::new(Some(String::from("Receiver")))
                        .icon(Icon::Connection)
                        .style(ButtonType::Standard)
                        .build()
                        .on_press(MainWindowEvent::Mode(Message::ButtonReceiver)),
                    horizontal_space().width(10),
                    IconButton::new(Some(String::from("Caster")))
                        .icon(Icon::Cast)
                        .style(ButtonType::Standard)
                        .build()
                        .on_press(MainWindowEvent::Mode(Message::ButtonCaster)),
                ]
                .align_y(alignment::Vertical::Center)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(10)
            )
            .class(ContainerType::Standard)
            .height(80),
            Container::new(
                crate::row![
                    Text::new("App Theme").align_x(alignment::Horizontal::Left).size(20).font(FONT_FAMILY_BOLD),
                    horizontal_space().width(Length::Fill),
                    IconButton::new(Some(String::from(if main_window.theme.target().get_palette().is_nightly() {"Dark"} else {"Light"}))).build().on_press(
                    MainWindowEvent::ThemeUpdate(main_window.theme.target().toggle().into())
                ),
                ]
                .align_y(Alignment::Center)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(10)
            )
            .class(ContainerType::Standard)
            .height(80)
        ]
        .spacing(10);

    let footer = Container::new(
        crate::row![
             crate::column![
                Text::new("Shortcuts:").size(16).font(FONT_FAMILY_BOLD),
                vertical_space().height(15),
                shortcuts(config.hotkey_map.record.clone(), "Record"),
                shortcuts(config.hotkey_map.pause.clone(), "Pause"),
                shortcuts(config.hotkey_map.blank_screen.clone(), "Blank Screen"),
                shortcuts(config.hotkey_map.end_session.clone(), "End Session"),
            ]
            .width(Length::Fill)
            .padding(10),
            vertical_space().width(Length::Fill),
            crate::column![
                Text::new("Info:").size(16).font(FONT_FAMILY_BOLD),
                vertical_space().height(15),
                footer_row(String::from("Local IP"), {
                    match config.local_ip {
                        Some(ip) => ip.to_string(),
                        _ => String::new(),
                    }
                }),
                footer_row(String::from("Public IP"), {
                    match *config.public_ip.lock().unwrap() {
                        Some(ip) => ip.to_string(),
                        _ => String::from("-------"),
                    }
                })
            ]
            .width(Length::Fill)
            .padding(10),
        ]
    ).height(140)
        .class(ContainerType::Standard);

    let content = crate::column![header, body, footer].spacing(10).padding(15);

    Container::new(content)
        .center(Length::Fill)
        .align_y(alignment::Vertical::Top)
        .into()
}

fn shortcuts<Message: 'static>(key_bind: (Modifiers, Key), str: &'static str) -> Row<'static, Message> {
    let s = format!("{}  {}", Key4Board::from_command(key_bind.0).get_label(), Key4Board::from_key(key_bind.1).get_label());
    footer_row(
        str.to_string(),
        s,
    )
}

fn footer_row<Message: 'static>(str1: String, str2: String) -> Row<'static, Message> {
    crate::row![
        Text::new(str1.to_string())
        .size(14).font(FONT_FAMILY_BOLD),
        horizontal_space().width(Length::Fill),
        Text::new(str2).size(14).font(FONT_FAMILY_BOLD),
    ]
}
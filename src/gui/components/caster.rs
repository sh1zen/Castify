use crate::assets::FONT_FAMILY_BOLD;
use crate::config::Config;
use crate::gui::common::icons::Icon;
use crate::gui::style::button::ButtonType;
use crate::gui::components::buttons::IconButton;
use crate::gui::style::container::ContainerType;
use crate::gui::widget::{horizontal_space, Column, Container, PickList, Row};
use crate::utils::{format_seconds, get_string_after};
use crate::windows::main::MainWindowEvent;
use crate::{row, workers};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{vertical_space, Text};
use iced::Length;
use crate::gui::common::datastructure::ScreenRect;

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Rec,
    Pause,
}

pub fn caster_page<'a>(config: &Config) -> Container<'a, MainWindowEvent> {
    let is_streaming = workers::caster::get_instance().lock().unwrap().streaming;

    let mut content = Column::new().spacing(10).padding(15);

    content = if is_streaming {
        content
            .push(
                Container::new(
                    row![Text::new(format_seconds(config.e_time).to_string()).font(FONT_FAMILY_BOLD)]
                ).width(Length::Fill).height(Length::Fill)
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center).height(80).class(ContainerType::Standard)
            )
    } else {
        content
            .push(
                Container::new(
                    row![monitors_list(is_streaming)]
                ).center(Length::Fill).height(80).class(ContainerType::Standard)
            )
            .push(
                Container::new(
                    row![
                        IconButton::new("Full Screen")
                        .icon(Icon::Screen)
                        .build()
                        .on_press(
                            MainWindowEvent::AreaSelected(ScreenRect::default())
                        ),
                        horizontal_space().width(10),
                        IconButton::new("Select Area")
                            .icon(Icon::Area)
                            .build()
                            .on_press(MainWindowEvent::AreaSelection)
                    ]
                ).center(Length::Fill).height(80).class(ContainerType::Standard)
            )
            .push(
                Container::new(
                    row![
                        IconButton::new("Home")
                        .icon(Icon::Browser)
                        .build()
                        .on_press(MainWindowEvent::Home)
                    ]
                ).center(Length::Fill)
                    .height(80).class(ContainerType::Standard)
            )
    }.push(vertical_space())
        .push(
            Container::new(
                if is_streaming {
                    IconButton::new("")
                        .icon(Icon::Pause)
                        .style(ButtonType::Rounded)
                        .build().width(80).height(80)
                        .on_press(MainWindowEvent::Caster(Message::Pause))
                } else {
                    IconButton::new("")
                        .icon(Icon::Video)
                        .style(ButtonType::Rounded)
                        .build().width(80).height(80)
                        .on_press(MainWindowEvent::Caster(Message::Rec))
                }
            )
                .width(Length::Fill).height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center).height(Length::Shrink).class(ContainerType::Transparent)
        );

    Container::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Top)
}

fn monitors_list(is_streaming: bool) -> Container<'static, MainWindowEvent> {
    if !is_streaming {
        monitors_picklist()
    } else {
        let empty_content = Row::new();
        Container::new(empty_content)
    }
}

fn monitor_name(id: u32) -> String {
    format!("Monitor #{}", id)
}

fn monitors_picklist() -> Container<'static, MainWindowEvent> {
    let mut monitors = Vec::new();

    for monitor_id in workers::caster::get_instance().lock().unwrap().get_monitors() {
        monitors.push(monitor_name(monitor_id));
    }

    if monitors.len() == 0 {
        return Container::new(iced::widget::Space::new(0, 0));
    }

    let selected = monitor_name(workers::caster::get_instance().lock().unwrap().current_monitor());
    let content = Column::new()
        .push(
            PickList::new(
                monitors,
                Some(selected),
                |val| {
                    workers::caster::get_instance().lock().unwrap().change_monitor(get_string_after(val.clone(), '#').trim().parse::<u32>().unwrap());
                    MainWindowEvent::Ignore
                },
            )
                .padding([11, 8])
        );

    Container::new(content)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
}
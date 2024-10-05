use crate::assets::FONT_FAMILY_BOLD;
use crate::config::Config;
use crate::gui::common::icons::Icon;
use crate::gui::components::buttons::IconButton;
use crate::gui::style::button::ButtonType;
use crate::gui::style::container::ContainerType;
use crate::gui::widget::{horizontal_space, vertical_space, Column, Container, Element, PickList, Text};
use crate::row;
use crate::utils::{format_seconds, get_string_after};
use crate::windows::main::MainWindowEvent;
use iced::alignment::{Horizontal, Vertical};
use iced::{Length};

pub fn caster_page<'a>(config: &Config) -> Element<'a, MainWindowEvent> {
    let mut is_streaming = false;

    let mut content = Column::new().spacing(10).padding(15);

    if let Some(crate::config::Mode::Caster(caster)) = &config.mode {
        if caster.is_streaming() {
            is_streaming = true;
            content = content
                .push(
                    Container::new(
                        row![Text::new(format_seconds(caster.streaming_time).to_string()).font(FONT_FAMILY_BOLD)]
                    ).width(Length::Fill).height(Length::Fill)
                        .align_x(Horizontal::Center)
                        .align_y(Vertical::Center).height(80).class(ContainerType::Standard)
                )
                .push(
                    Container::new(
                        row![
                            IconButton::new(Some(String::from("Annotations"))).icon(Icon::Image).build().width(180).on_press(MainWindowEvent::ShowAnnotationWindow)
                        ]
                    ).width(Length::Fill).height(Length::Fill)
                        .align_x(Horizontal::Center)
                        .align_y(Vertical::Center).height(80).class(ContainerType::Standard)
                );
        }
    }

    if !is_streaming {
        content = content
            .push(
                Container::new(row![monitors_picklist(config)])
                    .center(Length::Fill).height(80).class(ContainerType::Standard)
            )
            .push(
                Container::new(
                    row![
                        IconButton::new(Some(String::from("Full Screen")))
                        .icon(Icon::Screen)
                        .build()
                        .on_press(MainWindowEvent::AreaSelectedFullScreen),
                        horizontal_space().width(10),
                        IconButton::new(Some(String::from("Select Area")))
                            .icon(Icon::Area)
                            .build()
                            .on_press(MainWindowEvent::AreaSelection)
                    ]
                ).center(Length::Fill).height(80).class(ContainerType::Standard)
            )
            .push(
                Container::new(
                    row![
                        IconButton::new(Some(String::from("Home")))
                        .icon(Icon::Browser)
                        .build()
                        .on_press(MainWindowEvent::Home)
                    ]
                ).center(Length::Fill)
                    .height(80).class(ContainerType::Standard)
            )
    }

    content = content
        .push(vertical_space())
        .push(
            Container::new(
                if is_streaming {
                    IconButton::new(None)
                        .icon(Icon::Pause)
                        .style(ButtonType::Rounded)
                        .build().width(80).height(80)
                        .on_press(MainWindowEvent::CasterToggleStreaming)
                } else {
                    IconButton::new(None)
                        .icon(Icon::Video)
                        .style(ButtonType::Rounded)
                        .build().width(80).height(80)
                        .on_press(MainWindowEvent::CasterToggleStreaming)
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
        .into()
}

fn monitor_name(id: u32) -> String {
    format!("Monitor #{}", id)
}

fn monitors_picklist(config: &Config) -> Container<'static, MainWindowEvent> {
    let mut content = Column::new();

    if let Some(crate::config::Mode::Caster(caster)) = &config.mode {
        let mut monitors = Vec::new();

        for monitor_id in caster.get_monitors() {
            monitors.push(monitor_name(monitor_id));
        }

        if monitors.len() == 0 {
            return Container::new(iced::widget::Space::new(0, 0));
        }

        let selected = monitor_name(caster.current_monitor());
        content = content
            .push(
                PickList::new(
                    monitors,
                    Some(selected),
                    |val| {
                        MainWindowEvent::CasterMonitor(get_string_after(val.clone(), '#').trim().parse::<u32>().unwrap())
                    },
                ).padding([11, 8])
            )
    }

    Container::new(content)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
}
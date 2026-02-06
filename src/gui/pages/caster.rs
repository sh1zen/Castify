use crate::assets::FONT_FAMILY_BOLD;
use crate::config::Config;
use crate::gui::common::icons::Icon;
use crate::gui::components::button::{Dimensions, IconButton};
use crate::gui::style::button::ButtonType;
use crate::gui::style::container::ContainerType;
use crate::gui::widget::{horizontal_space, vertical_space, Column, Container, Element, PickList, Text};
use crate::gui::windows::main::MainWindowEvent;
use crate::row;
use crate::utils::format_seconds;
use iced::alignment::{Horizontal, Vertical};
use iced::Length;

pub fn caster_page<'a>(config: &Config) -> Element<'a, MainWindowEvent> {
    let Some(crate::config::Mode::Caster(caster)) = &config.mode else {
        unreachable!("Mode must be Caster here")
    };

    let mut is_streaming = false;

    let mut content = Column::new().spacing(10).padding(15);

    content = if caster.is_streaming() {
        is_streaming = true;
        content
            .push(
                Container::new(
                    row![
                        Icon::Clock.to_text(),
                        horizontal_space().width(7),
                        Text::new(format_seconds(caster.streaming_time).to_string()).font(FONT_FAMILY_BOLD)
                    ]
                ).width(Length::Fill).height(Length::Fill)
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center).height(80).class(ContainerType::Standard)
            )
            .push(
                Container::new(
                    row![
                            IconButton::new().label("Annotations").icon(Icon::Image).build().width(180).on_press(MainWindowEvent::ShowAnnotationWindow),
                            IconButton::new().label("Manual SDP").icon(Icon::Sync).build().width(180).on_press(MainWindowEvent::ShowSDP),
                            IconButton::new()
                                .label(if caster.is_audio_muted() { "Unmute" } else { "Mute" })
                                .icon(if caster.is_audio_muted() { Icon::VolumeMute } else { Icon::VolumeHigh })
                                .build().width(140)
                                .on_press(MainWindowEvent::ToggleAudioMute)
                    ].spacing(5)
                ).width(Length::Fill).height(Length::Fill)
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center).height(80).class(ContainerType::Standard)
            )
    } else {
        content
            .push(
                Container::new(row![displays_picklist(config)])
                    .center(Length::Fill).height(80).class(ContainerType::Standard)
            )
            .push(
                Container::new(
                    row![
                        IconButton::new()
                            .label("Full Screen")
                            .icon(Icon::Screen)
                            .dim(Dimensions::Large)
                            .build()
                            .on_press(MainWindowEvent::AreaSelectedFullScreen),
                        horizontal_space().width(10),
                        IconButton::new()
                            .label("Select Area")
                            .icon(Icon::Area)
                            .dim(Dimensions::Large)
                            .build()
                            .on_press(MainWindowEvent::AreaSelection)
                    ]
                ).center(Length::Fill).height(80).class(ContainerType::Standard)
            )
            .push(
                Container::new(
                    row![
                        IconButton::new().label("Home")
                            .icon(Icon::Home)
                            .build()
                            .on_press(MainWindowEvent::Home)
                    ]
                ).center(Length::Fill)
                    .height(80).class(ContainerType::Standard)
            )
    };

    content = content
        .push(vertical_space())
        .push(
            Container::new(
                if is_streaming {
                    IconButton::new()
                        .icon(Icon::Pause)
                        .style(ButtonType::Rounded)
                        .build().width(80).height(80)
                        .on_press(MainWindowEvent::CasterToggleStreaming)
                } else {
                    IconButton::new()
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

fn display_name(idx: usize) -> String {
    format!("Display #{}", idx + 1)
}

fn displays_picklist(config: &Config) -> Container<'static, MainWindowEvent> {
    let Some(crate::config::Mode::Caster(caster)) = &config.mode else {
        unreachable!("Mode must be Caster here")
    };

    let displays = caster.get_displays();

    if displays.is_empty() {
        return Container::new(iced::widget::Space::new());
    }

    let options: Vec<String> = (0..displays.len())
        .map(display_name)
        .collect();

    // Per ora selezioniamo il primo come default
    let selected = display_name(0);

    let content = Column::new().push(
        PickList::new(
            options,
            Some(selected),
            |val| {
                // Estrai l'indice dal nome "Display #N"
                let idx = val
                    .trim_start_matches("Display #")
                    .parse::<usize>()
                    .unwrap_or(1)
                    - 1;
                MainWindowEvent::CasterChangeDisplay(idx)
            },
        ).padding([11, 8])
    );

    Container::new(content)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
}
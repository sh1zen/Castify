use std::sync::MutexGuard;
use crate::capture::Capture;
use crate::gui::theme::buttons::FilledButton;
use crate::gui::theme::button::ButtonType;
use crate::gui::theme::styles::csx::StyleType;
use crate::gui::types::appbase::App;
use crate::gui::types::icons::Icon;
use crate::gui::types::messages::Message as appMessage;
use crate::utils::get_string_after;
use crate::workers;
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{Column, Container, PickList, Row};
use iced::{Alignment, Length};
use crate::workers::caster::Caster;

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Rec,
    Pause,
    FullScreenSelected,
    AreaSelected,
}

pub fn caster_page(_: &App) -> Container<appMessage, StyleType> {
    let caster_instance = workers::caster::get_instance();
    let mut caster = caster_instance.lock().unwrap();

    let action = if caster.streaming {
        FilledButton::new("Pause")
            .icon(Icon::Pause)
            .build()
            .on_press(appMessage::Caster(Message::Pause))
    } else {
        FilledButton::new("Rec")
            .icon(Icon::Video)
            .build()
            .on_press(appMessage::Caster(Message::Rec))
    };


    let mut fullscreen_button = FilledButton::new("Full Screen")
        .icon(Icon::Screen)
        .build()
        .on_press(appMessage::Caster(Message::FullScreenSelected));

    let mut select_area_button = FilledButton::new("Select Area")
        .icon(Icon::Area)
        .build()
        .on_press(appMessage::Caster(Message::AreaSelected));

    if caster.streaming {
        fullscreen_button = fullscreen_button.style(ButtonType::Disabled);
        select_area_button = select_area_button.style(ButtonType::Disabled);
    }


    let mut content = Row::new()
        .align_items(Alignment::Center)
        .spacing(10)
        .height(400)
        .push(action)
        .push(fullscreen_button)
        .push(select_area_button)
        .push(test(caster));

    Container::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
}

fn test(caster: MutexGuard<Caster>) -> Container<'static, appMessage, StyleType> {
if !caster.streaming {
    monitors_picklist(caster)
}
    else {
        let empty_content = Row::new();
        return Container::new(empty_content);
    }
}

fn monitor_name(id: u32) -> String {
    format!("Monitor #{}", id)
}

fn monitors_picklist(mut caster: MutexGuard<Caster>) -> Container<'static, appMessage, StyleType> {
    let mut monitors = Vec::new();

    for monitor in Capture::get_monitors() {
        monitors.push(monitor_name(monitor.1.monitor.id()));
    }

    if monitors.len() == 0 {
        return Container::new(iced::widget::Space::new(0, 0));
    }

    let selected = monitor_name(caster.monitor);
    caster.monitor = get_string_after(selected.clone(), '#').trim().parse::<u32>().unwrap();
    let content = Column::new()
        .push(
            PickList::new(
                monitors,
                Some(selected),
                |selected_value| {

                    appMessage::Ignore
                },
            )
                .padding([11, 8])
        );

    Container::new(content)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
}
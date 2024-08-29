use crate::capture::Capture;
use crate::gui::appbase::App;
use crate::gui::theme::buttons::FilledButton;
use crate::gui::theme::styles::csx::StyleType;
use crate::gui::types::icons::Icon;
use crate::gui::types::messages::Message as appMessage;
use crate::utils::get_string_after;
use crate::workers;
use crate::workers::caster::Caster;
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{Column, Container, PickList, Row};
use iced::{Alignment, Length};
use std::sync::MutexGuard;

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Rec,
    Pause,
    FullScreenSelected,
    AreaSelected((i32, i32, u32, u32)),
}

pub fn caster_page(_: &App) -> Container<appMessage, StyleType> {
    let mut content = Row::new()
        .align_items(Alignment::Center)
        .spacing(10)
        .height(400);

    let is_streaming = workers::caster::get_instance().try_lock().unwrap().streaming;

    if is_streaming {
        content = content.push(
            FilledButton::new("Pause")
                .icon(Icon::Pause)
                .build()
                .on_press(appMessage::Caster(Message::Pause))
        );
    } else {
        content = content.push(
            FilledButton::new("Rec")
                .icon(Icon::Video)
                .build()
                .on_press(appMessage::Caster(Message::Rec))
        )
            .push(FilledButton::new("Full Screen")
                .icon(Icon::Screen)
                .build()
                .on_press(appMessage::Caster(Message::FullScreenSelected)))
            .push(FilledButton::new("Select Area")
                .icon(Icon::Area)
                .build()
                .on_press(appMessage::Caster(Message::AreaSelected((100, 100, 800, 800)))));
    };

    content = content.push(monitors_list(is_streaming));

    Container::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
}

fn monitors_list(is_streaming: bool) -> Container<'static, appMessage, StyleType> {
    if !is_streaming {
        monitors_picklist(workers::caster::get_instance().lock().unwrap())
    } else {
        let empty_content = Row::new();
        Container::new(empty_content)
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

    let selected = monitor_name(caster.current_monitor());
    caster.change_monitor(get_string_after(selected.clone(), '#').trim().parse::<u32>().unwrap());
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
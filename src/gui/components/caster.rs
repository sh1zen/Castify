use crate::capture::Capture;
use crate::gui::appbase::App;
use crate::gui::components::screen_overlay::AreaSelectionMessage;
use crate::gui::theme::buttons::FilledButton;
use crate::gui::theme::styles::csx::StyleType;
use crate::gui::types::icons::Icon;
use crate::gui::types::messages::Message as appMessage;
use crate::utils::get_string_after;
use crate::workers;
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{Column, Container, PickList, Row};
use iced::{Alignment, Length};

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Rec,
    Pause,
    FullScreenSelected,
    AreaSelected((i32, i32, u32, u32)),
}

pub fn caster_page(_: &App) -> Container<appMessage, StyleType> {
    let mut action_row = Row::new()
        .align_items(Alignment::Center)
        .spacing(15);

    let is_streaming = workers::caster::get_instance().try_lock().unwrap().streaming;

    let actions = if is_streaming {
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

    action_row = action_row.push(actions).push(monitors_list(is_streaming));

    let mut screen_row = Row::new().spacing(15);

    if !is_streaming {
        screen_row = screen_row.push(FilledButton::new("Full Screen")
            .icon(Icon::Screen)
            .build()
            .on_press(appMessage::Caster(Message::FullScreenSelected)))
            .push(FilledButton::new("Select Area")
                .icon(Icon::Area)
                .build()
                .on_press(appMessage::AreaSelection(AreaSelectionMessage::StartSelection { x: 0.0, y: 0.0 })));
    }

    Container::new(
        Column::new().align_items(Alignment::Center)
            .spacing(20)
            .align_items(Alignment::Center)
            .push(action_row)
            .push(screen_row)
    )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
}

fn monitors_list(is_streaming: bool) -> Container<'static, appMessage, StyleType> {
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

fn monitors_picklist() -> Container<'static, appMessage, StyleType> {
    let mut monitors = Vec::new();

    for monitor in Capture::get_monitors() {
        monitors.push(monitor_name(monitor.1.monitor.id()));
    }

    if monitors.len() == 0 {
        return Container::new(iced::widget::Space::new(0, 0));
    }

    let selected = monitor_name(workers::caster::get_instance().lock().unwrap().current_monitor());
    workers::caster::get_instance().lock().unwrap().change_monitor(get_string_after(selected.clone(), '#').trim().parse::<u32>().unwrap());
    let content = Column::new()
        .push(
            PickList::new(
                monitors,
                Some(selected),
                |_| {
                    appMessage::Ignore
                },
            )
                .padding([11, 8])
        );

    Container::new(content)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
}
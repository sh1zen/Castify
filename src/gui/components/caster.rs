use crate::gui::appbase::App;
use crate::gui::components::raw::screenArea::ScreenRect;
use crate::gui::theme::button::ButtonType;
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

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Rec,
    Pause,
}

pub fn caster_page(_: &App) -> Container<appMessage, StyleType> {
    let mut action_row = Row::new()
        .align_items(Alignment::Center)
        .spacing(20);

    let is_streaming = workers::caster::get_instance().lock().unwrap().streaming;

    let actions = if is_streaming {
        FilledButton::new("Pause")
            .icon(Icon::Pause)
            .style(ButtonType::Round)
            .build()
            .on_press(appMessage::Caster(Message::Pause))
    } else {
        FilledButton::new("Rec")
            .icon(Icon::Video)
            .style(ButtonType::Round)
            .build()
            .on_press(appMessage::Caster(Message::Rec))
    };

    action_row = action_row.push(actions);

    let mut screen_row = Row::new().spacing(15);

    if !is_streaming {
        screen_row = screen_row
            .push(monitors_list(is_streaming))
            .push(FilledButton::new("Full Screen")
                .icon(Icon::Screen)
                .build()
                .on_press(
                    appMessage::AreaSelected(
                        ScreenRect {
                            x: 0.0,
                            y: 0.0,
                            width: 0.0,
                            height: 0.0,
                        }
                    )
                ))
            .push(
                FilledButton::new("Select Area")
                    .icon(Icon::Area)
                    .build()
                    .on_press(appMessage::AreaSelection))
            .push(
                FilledButton::new("Home")
                    .icon(Icon::Browser)
                    .build()
                    .on_press(appMessage::Home)
            );
    }

    Container::new(
        Column::new().align_items(Alignment::Center)
            .align_items(Alignment::Center)
            .spacing(40)
            .push(screen_row)
            .push(action_row)
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
                    appMessage::Ignore
                },
            )
                .padding([11, 8])
        );

    Container::new(content)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
}
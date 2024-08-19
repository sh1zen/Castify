use crate::capture::Capture;
use crate::gui::theme::buttons::FilledButton;
use crate::gui::theme::styles::csx::StyleType;
use crate::gui::types::appbase::App;
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
}

pub fn caster_page(_: &App) -> Container<appMessage, StyleType> {
    let action = if workers::caster::get_instance().lock().unwrap().streaming {
        FilledButton::new("Pause").icon(Icon::Pause).build().on_press(
            appMessage::Caster(Message::Pause)
        )
    } else {
        FilledButton::new("Rec").icon(Icon::Video).build().on_press(
            appMessage::Caster(Message::Rec)
        )
    };

    let mut content = Row::new()
        .align_items(Alignment::Center).spacing(10)
        .height(400)
        .push(action);

    if !workers::caster::get_instance().lock().unwrap().streaming {
        content = content.push(monitors_picklist());
    }

    Container::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x().center_y()
}

fn monitor_name(id: u32) -> String {
    format!("Monitor #{}", id)
}

fn monitors_picklist() -> Container<'static, appMessage, StyleType> {
    let mut monitors = Vec::new();

    for monitor in Capture::new().get_monitors() {
        monitors.push(monitor_name(monitor.1));
    }

    let selected = monitor_name(workers::caster::get_instance().lock().unwrap().monitor);

    let content = Column::new()
        .push(
            PickList::new(
                monitors,
                Some(selected),
                |selected_value| {
                    workers::caster::get_instance().lock().unwrap().monitor = get_string_after(selected_value, '#').trim().parse::<u32>().unwrap();
                    appMessage::Ignore
                },
            )
                .padding([11, 8])
        );

    Container::new(content)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
}
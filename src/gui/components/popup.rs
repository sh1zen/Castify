use crate::gui::components::hotkeys::KeyTypes;
use crate::gui::theme::buttons::{FilledButton, Key4Board};
use crate::gui::theme::container::ContainerType;
use crate::gui::theme::styles::csx::StyleType;
use crate::gui::appbase::App;
use crate::gui::types::messages::Message as appMessage;
use iced::advanced::widget::Text;
use iced::keyboard::Key;
use iced::widget::{Column, Container, Row, Space, TextInput};
use iced_aw::widgets::Modal;
use iced_wgpu::core::keyboard::Modifiers;
use std::hash::Hash;
use iced_core::Alignment;
use crate::gui::types::icons::Icon;

#[derive(Debug, Clone)]
pub struct Interaction {
    pub text: String,
    pub p_type: PopupType,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PopupType {
    IP,
    HotkeyUpdate,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PopupMsg {
    String(String),
    HotKey(KeyTypes),
}

pub fn show_popup<'a>(app: &'a App, body: Container<'a, appMessage, StyleType>) -> Container<'a, appMessage, StyleType> {
    if app.show_popup.is_none() {
        return Container::new(Space::new(0, 0));
    }

    let popup_type = app.show_popup.clone().unwrap();

    let content = match popup_type {
        PopupType::IP => {
            ip_popup(app)
        }
        PopupType::HotkeyUpdate => {
            hotkey_update(app)
        }
    };

    Container::new(
        Modal::new(
            body,
            Some(
                content
            ),
        )
    )
}


fn hotkey_update(app: &App) -> Container<appMessage, StyleType> {
    let updating_key = match get_popup_data(app, PopupType::HotkeyUpdate) {
        PopupMsg::HotKey(key) => {
            key
        }
        _ => { KeyTypes::None }
    };

    let c_key = match updating_key {
        KeyTypes::Pause => { app.hotkey_map.pause.clone() }
        KeyTypes::Record => { app.hotkey_map.record.clone() }
        KeyTypes::Close => { app.hotkey_map.end_session.clone() }
        KeyTypes::BlankScreen => { app.hotkey_map.blank_screen.clone() }
        _ => { (Modifiers::empty(), Key::Unidentified) }
    };

    let ok_button = FilledButton::new("Ok").build().on_press(appMessage::ClosePopup);

    let content = Column::new()
        .spacing(10)
        .padding(20)
        .push(
            Text::new(
                format!("Updating hotkey for: {:?}",
                        updating_key
                )
            ).size(20))
        .push(
            Row::new()
                .push(Key4Board::from_command(c_key.0).build())
                .push(Key4Board::from_key(c_key.1).build())
                .spacing(5)
        )
        .push(Text::new("Press any desired key.").height(20).size(12))
        .push(
            ok_button
        );

    Container::new(content.width(500).height(300)).style(ContainerType::Modal)
}

fn ip_popup(app: &App) -> Container<appMessage, StyleType> {
    let mut entered_ip = match get_popup_data(app, PopupType::IP) {
        PopupMsg::String(str) => {
            str
        }
        _ => { "".parse().unwrap() }
    };

    // remove any invalid char
    entered_ip = entered_ip.chars().filter(|c| ".0123456789:".contains(*c)).collect();

    let input = TextInput::new("192.168.1.1", &entered_ip)
        .on_input(move |new_value| {
            appMessage::PopupMessage(Interaction { text: new_value, p_type: PopupType::IP })
        })
        .padding([8, 12])
        .id(iced::widget::text_input::Id::new("ip_text_input"));

    let mut button = FilledButton::new("Connect").build();

    if !entered_ip.is_empty() {
        button = button.on_press(appMessage::ConnectToCaster(entered_ip.clone()));
    }

    let content = Column::new()
        .spacing(10)
        .padding(20)
        .push(Text::new("Enter Receiver IP Address:").size(20))
        .push(input)
        .push(
            Row::new().spacing(12)
                .push(button)
                .push(FilledButton::new("Auto").build().on_press(appMessage::ConnectToCaster("auto".parse().unwrap())))
                .push(
                    FilledButton::new("Home")
                        .icon(Icon::Browser)
                        .build()
                        .on_press(appMessage::Home)
                )
        );

    Container::new(content.width(500).height(300)).style(ContainerType::Modal)
}

fn get_popup_data(app: &App, popup_type: PopupType) -> PopupMsg {
    if app.popup_msg.contains_key(&popup_type) {
        app.popup_msg.get(&popup_type).unwrap().clone()
    } else {
        PopupMsg::String("".parse().unwrap())
    }
}
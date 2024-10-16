use crate::config::Config;
use crate::gui::common::icons::Icon;
use crate::gui::pages::hotkeys::KeyTypes;
use crate::gui::components::button::{Dimensions, IconButton, Key4Board};
use crate::gui::style::container::ContainerType;
use crate::gui::widget::{Column, Container, IcedParentExt, Row, Space, Stack, Text, TextInput};
use crate::gui::windows::main::MainWindowEvent;
use iced::keyboard::Key;
use iced_core::Length;
use iced_wgpu::core::keyboard::Modifiers;
use std::collections::HashMap;
use std::hash::Hash;

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

pub struct Popup {
    show_popup: Option<PopupType>,
    popups: HashMap<PopupType, PopupMsg>,
}

impl Popup {
    pub fn new() -> Self {
        Popup {
            show_popup: None,
            popups: Default::default(),
        }
    }

    pub fn is_visible(&self) -> bool {
        self.show_popup.is_some()
    }

    pub fn current(&self) -> Option<PopupType> {
        self.show_popup.clone()
    }

    pub fn hide(&mut self) {
        self.show_popup = None;
    }

    pub fn show(&mut self, p0: PopupType) {
        self.show_popup = Some(p0);
    }

    pub fn has(&self, p0: &PopupType) -> bool {
        self.popups.contains_key(&p0)
    }

    pub fn get_mut(&mut self, p0: &PopupType) -> Option<&mut PopupMsg> {
        self.popups.get_mut(p0)
    }

    pub fn get(&self, p0: &PopupType) -> Option<&PopupMsg> {
        self.popups.get(p0)
    }

    pub(crate) fn insert(&mut self, p0: PopupType, p1: PopupMsg) -> bool {
        self.popups.insert(p0, p1).is_some()
    }
}

pub fn show_popup<'a>(popup: &Popup, config: &Config, body: Container<'a, MainWindowEvent>) -> Container<'a, MainWindowEvent> {
    if !popup.is_visible() {
        return Container::new(Space::new(0, 0));
    }

    let popup_type = popup.current().unwrap();

    let pp = match popup_type {
        PopupType::IP => {
            ip_popup(popup)
        }
        PopupType::HotkeyUpdate => {
            hotkey_update(popup, config)
        }
    };

    let content = Container::new(pp)
        .class(ContainerType::Modal)
        .center_x(Length::Fixed(480.0))
        .center_y(Length::Fixed(260.0));

    let centered_content = Container::new(content).center(Length::Fill);

    let darkened_background = Container::new(Space::new(0, 0))
        .width(Length::Fill)
        .height(Length::Fill)
        .class(ContainerType::DarkFilter);

    Container::new(
        Stack::new()
            .push(body)
            .push(darkened_background)
            .push(centered_content),
    )
}

fn hotkey_update<'a>(popup: &Popup, config: &Config) -> Container<'a, MainWindowEvent> {
    let updating_key = match get_popup_data(popup, PopupType::HotkeyUpdate) {
        PopupMsg::HotKey(key) => {
            key
        }
        _ => { KeyTypes::None }
    };

    let c_key = match updating_key {
        KeyTypes::Pause => { config.hotkey_map.pause.clone() }
        KeyTypes::Record => { config.hotkey_map.record.clone() }
        KeyTypes::Close => { config.hotkey_map.end_session.clone() }
        KeyTypes::BlankScreen => { config.hotkey_map.blank_screen.clone() }
        _ => { (Modifiers::empty(), Key::Unidentified) }
    };

    let ok_button = IconButton::new().label(String::from("Ok")).build().on_press(MainWindowEvent::ClosePopup);

    let content = popup_base(None)
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

    Container::new(content.width(500).height(300)).class(ContainerType::Modal)
}

fn ip_popup<'a>(popup: &Popup) -> Container<'a, MainWindowEvent> {
    let mut entered_ip = match get_popup_data(popup, PopupType::IP) {
        PopupMsg::String(str) => {
            str
        }
        _ => { "".parse().unwrap() }
    };

    // remove any invalid char
    entered_ip = entered_ip.chars().filter(|c| ".0123456789:".contains(*c)).collect();

    let input = TextInput::new("192.168.1.1", &entered_ip)
        .on_input(move |new_value| {
            MainWindowEvent::PopupMessage(Interaction { text: new_value, p_type: PopupType::IP })
        })
        .padding([8, 12])
        .id(iced::widget::text_input::Id::new("ip_text_input"));

    let mut button = IconButton::new().label(String::from("Connect")).icon(Icon::Connect).dim(Dimensions::Large).build();

    if !entered_ip.is_empty() {
        button = button.on_press(MainWindowEvent::ConnectToCaster(entered_ip.clone()));
    }

    let content = popup_base(Some("Enter Receiver IP Address:"))
        .push(input)
        .push(
            Row::new().spacing(12)
                .push(button)
                .push(IconButton::new().label(String::from("Auto")).icon(Icon::Auto).build().on_press(MainWindowEvent::ConnectToCaster("auto".parse().unwrap())))
                .push(
                    IconButton::new().label(String::from("Home"))
                        .icon(Icon::Home)
                        .build()
                        .on_press(MainWindowEvent::Home)
                )
        );

    Container::new(content.width(500).height(300)).class(ContainerType::Modal)
}

fn popup_base<Message>(title: Option<&str>) -> Column<Message> {
    Column::new()
        .spacing(10)
        .padding(20)
        .push_if(title.is_some(), || Text::new(title.unwrap()).size(20))
}

fn get_popup_data(popup: &Popup, popup_type: PopupType) -> PopupMsg {
    if popup.has(&popup_type) {
        popup.get(&popup_type).unwrap().clone()
    } else {
        PopupMsg::String("".parse().unwrap())
    }
}
use crate::config::Config;
use crate::gui::common::hotkeys::KeyTypes;
use crate::gui::common::icons::Icon;
use crate::gui::components::button::{Dimensions, IconButton, Key4Board};
use crate::gui::style::container::ContainerType;
use crate::gui::widget::{vertical_space, Column, Container, Element, IcedButtonExt, IcedParentExt, Row, Space, Stack, Text, TextInput};
use crate::gui::windows::main::MainWindowEvent;
use iced::keyboard::Key;
use iced::Length;
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
    ShowSDP,
    SetSDP,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PopupContent {
    String(String),
    HotKey(KeyTypes),
}

pub struct Popup {
    show_popup: Option<PopupType>,
    popups: HashMap<PopupType, PopupContent>,
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

    pub fn get_mut(&mut self, p0: &PopupType) -> Option<&mut PopupContent> {
        self.popups.get_mut(p0)
    }

    pub fn get(&self, p0: &PopupType) -> Option<&PopupContent> {
        self.popups.get(p0)
    }

    pub(crate) fn insert(&mut self, p0: PopupType, p1: PopupContent) -> bool {
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
        PopupType::ShowSDP => {
            show_text(popup)
        }
        PopupType::SetSDP => {
            show_text(popup)
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

fn hotkey_update<'a>(popup: &Popup, config: &Config) -> Element<'a, MainWindowEvent> {
    let updating_key = match get_popup_data(popup, PopupType::HotkeyUpdate) {
        PopupContent::HotKey(key) => {
            key
        }
        _ => { KeyTypes::None }
    };

    let c_key = match updating_key {
        KeyTypes::Pause => { config.shortcuts.pause.clone() }
        KeyTypes::Record => { config.shortcuts.record.clone() }
        KeyTypes::Close => { config.shortcuts.end_session.clone() }
        KeyTypes::BlankScreen => { config.shortcuts.blank_screen.clone() }
        _ => { (Modifiers::empty(), Key::Unidentified) }
    };

    let ok_button = IconButton::new().label(String::from("Ok")).build().on_press(MainWindowEvent::ClosePopup);

    AwPopup::new()
        .content(
            Text::new(
                format!("Updating hotkey for: {:?}",
                        updating_key
                )
            ).size(20))
        .content(
            Row::new()
                .push(Key4Board::from_command(c_key.0).build())
                .push(Key4Board::from_key(c_key.1).build())
                .spacing(5)
        )
        .content(Text::new("Press any desired key.").height(20).size(12))
        .content(
            ok_button
        ).build()
}

fn show_text<'a>(popup: &Popup) -> Element<'a, MainWindowEvent> {
    let text = if let Some(PopupContent::String(text)) = &popup.popups.get(&PopupType::ShowSDP) {
        text
    } else { "" };

    let input = TextInput::new("Share to partner", text)
        .on_input(|_| MainWindowEvent::Ignore)
        .padding([8, 12]);

    AwPopup::new().title("Copy and share this to the other party.")
        .content(input)
        .build()
}

fn ip_popup<'a>(popup: &Popup) -> Element<'a, MainWindowEvent> {
    let mut entered_ip = match get_popup_data(popup, PopupType::IP) {
        PopupContent::String(str) => {
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

    let button =
        IconButton::new().label(String::from("Connect")).icon(Icon::Connect).dim(Dimensions::Large)
            .build()
            .on_press_if(!entered_ip.is_empty(), || MainWindowEvent::ConnectToCaster(entered_ip.clone()));

    AwPopup::new()
        .title("Enter Receiver IP Address:")
        .content(input)
        .content(
            Row::new().spacing(12)
                .push(button)
                .push(IconButton::new().label(String::from("Auto")).icon(Icon::Auto).build().on_press(MainWindowEvent::ConnectToCaster("auto".parse().unwrap())))
                .push(
                    IconButton::new().label(String::from("Home"))
                        .icon(Icon::Home)
                        .build()
                        .on_press(MainWindowEvent::Home)
                )
        )
        .build()
}

fn get_popup_data(popup: &Popup, popup_type: PopupType) -> PopupContent {
    if popup.has(&popup_type) {
        popup.get(&popup_type).unwrap().clone()
    } else {
        PopupContent::String("".parse().unwrap())
    }
}

pub struct AwPopup<'a, Message> {
    title: Option<&'a str>,
    content: Vec<Element<'a, Message>>,
    on_close: Option<Message>,
    width: f32,
    height: f32,
}

impl<'a, Message: 'a + Clone> AwPopup<'a, Message> {
    pub fn new() -> Self {
        AwPopup {
            title: None,
            content: Vec::new(),
            on_close: None,
            width: 500.0,
            height: 300.0,
        }
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    pub fn content(mut self, content: impl Into<Element<'a, Message>>) -> Self {
        self.content.push(content.into());
        self
    }

    pub fn on_close(mut self, message: Message) -> Self {
        self.on_close = Some(message);
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn build(&mut self) -> Element<'a, Message> {
        let items = std::mem::take(&mut self.content);
        let mut content = Column::new().spacing(12);

        for element in items.into_iter() {
            content = content.push(element);
        }

        let items = Column::new()
            .spacing(10)
            .padding(20)
            .push_if(self.title.is_some(), || Text::new(self.title.unwrap()).size(20))
            .push(vertical_space().height(5))
            .push(content)
            .push(vertical_space().height(5))
            .push_if(
                self.on_close.is_some(),
                || IconButton::new().icon(Icon::Close).label("Close".to_string()).build().on_press(self.on_close.clone().unwrap()),
            ).width(self.width).height(self.height);

        Container::new(items).class(ContainerType::Modal).into()
    }
}
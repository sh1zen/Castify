use crate::gui::components::hotkeys::KeyTypes;
use crate::gui::components::popup::{PopupMsg, PopupType};
use crate::gui::types::messages::Message;
use crate::gui::video::Video;
use iced::keyboard::key::Named;
use iced::keyboard::{Event, Key, Modifiers};
use iced::mouse::Event::ButtonPressed;
use iced::window::Id;
use iced::Event::{Keyboard, Window};
use iced::{window, Subscription};
use std::collections::HashMap;

pub enum Page {
    Home,
    Caster,
    Client,
    Hotkeys,
}

#[derive(Clone)]
pub struct HotkeyMap {
    pub pause: (Modifiers, Key),
    pub record: (Modifiers, Key),
    pub end_session: (Modifiers, Key),
    pub blank_screen: (Modifiers, Key),
    pub updating: KeyTypes,
}

pub struct App {
    /// whether this app needs act as caster or receiver
    pub(crate) is_caster: bool,
    pub(crate) page: Page,
    pub(crate) show_popup: Option<PopupType>,
    pub(crate) popup_msg: HashMap<PopupType, PopupMsg>,
    pub(crate) video: Video,
    pub(crate) hotkey_map: HotkeyMap,
}

impl App {
    pub fn new() -> App {
        App {
            is_caster: false,
            page: Page::Home,
            show_popup: None,
            popup_msg: HashMap::new(),
            video: Video::new(),
            hotkey_map: HotkeyMap {
                pause: (Modifiers::CTRL, Key::Named(Named::F10)),
                record: (Modifiers::CTRL, Key::Named(Named::F11)),
                end_session: (Modifiers::SHIFT, Key::Named(Named::Escape)),
                blank_screen: (Modifiers::CTRL, Key::Named(Named::F2)),
                updating: KeyTypes::None,
            },
        }
    }

    pub(crate) fn keyboard_subscription(&self) -> Subscription<Message> {
        //const NO_MODIFIER: Modifiers = Modifiers::empty();

        // used to update HotKeys
        if self.hotkey_map.updating != KeyTypes::None {
            iced::event::listen_with(|event, _| match event {
                Keyboard(Event::KeyPressed { key, modifiers, .. }) => {
                    Some(Message::HotkeysUpdate((modifiers, key)))
                }
                _ => None,
            })
        } else {
            iced::event::listen_with(|event, _| match event {
                Keyboard(Event::KeyPressed { key, modifiers, .. }) => {
                    Some(Message::KeyPressed((modifiers, key)))
                }
                _ => None,
            })
        }
    }

    pub(crate) fn mouse_subscription(&self) -> Subscription<Message> {
        iced::event::listen_with(|event, _| match event {
            iced::event::Event::Mouse(ButtonPressed(_)) => Some(Message::Drag),
            _ => None,
        })
    }

    pub(crate) fn window_subscription(&self) -> Subscription<Message> {
        iced::event::listen_with(|event, _| match event {
            Window(Id::MAIN, window::Event::Focused) => Some(Message::WindowFocused),
            Window(Id::MAIN, window::Event::Moved { x, y }) => Some(Message::WindowMoved(x, y)),
            Window(Id::MAIN, window::Event::Resized { width, height }) => {
                Some(Message::WindowResized(width, height))
            }
            Window(Id::MAIN, window::Event::CloseRequested) => Some(Message::CloseRequested),
            _ => None,
        })
    }
}
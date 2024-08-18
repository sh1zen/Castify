use crate::capture::Capture;
use crate::gui::components::caster::CasterOptions;
use crate::gui::components::hotkeys::KeyTypes;
use crate::gui::components::popup::{PopupMsg, PopupType};
use crate::gui::resource::FRAME_RATE;
use crate::gui::types::messages::Message;
use crate::gui::video::Video;
use iced::keyboard::key::Named;
use iced::keyboard::{Event, Key, Modifiers};
use iced::mouse::Event::ButtonPressed;
use iced::window::Id;
use iced::Event::{Keyboard, Window};
use iced::{window, Subscription};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use chrono::Local;

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
    caster: bool,
    pub(crate) page: Page,
    pub(crate) show_popup: Option<PopupType>,
    pub(crate) popup_msg: HashMap<PopupType, PopupMsg>,
    pub(crate) video: Video,
    pub(crate) caster_opt: Arc<Mutex<CasterOptions>>,
    pub(crate) hotkey_map: HotkeyMap,
}

impl App {
    pub fn new() -> App {
        App {
            caster: false,
            page: Page::Home,
            show_popup: None,
            popup_msg: HashMap::new(),
            video: Video::new().unwrap(),
            caster_opt: Arc::new(Mutex::new(CasterOptions::new())),
            hotkey_map: HotkeyMap {
                pause: (Modifiers::CTRL, Key::Named(Named::F10)),
                record: (Modifiers::CTRL, Key::Named(Named::F11)),
                end_session: (Modifiers::SHIFT, Key::Named(Named::Escape)),
                blank_screen: (Modifiers::CTRL, Key::Named(Named::F2)),
                updating: KeyTypes::None,
            },
        }
    }

    pub fn vdffd(&mut self) {
        self.caster_opt.lock().unwrap().streaming = true;

        //let mut rx = self.caster_opt.clone().lock().unwrap().get_rx();
        //let mut tx = self.caster_opt.clone().lock().unwrap().get_tx();

        let (tx, rx) = tokio::sync::mpsc::channel(1);
        // generate frames
        tokio::spawn(async move {
            let mut capture = Capture::new();
            capture.set_framerate(FRAME_RATE as f32);
            capture.stream(capture.main.clone(), tx, true).await;
        });
        // send frames over the local network
        tokio::spawn(async move {
            crate::utils::net::caster(rx).await;
        });
    }

    pub(crate) fn keyboard_subscription(&self) -> Subscription<Message> {
        const NO_MODIFIER: Modifiers = Modifiers::empty();

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
                Keyboard(Event::KeyPressed { key, modifiers, .. }) =>  {
                    Some(Message::KeyPressed((modifiers, key)))
                },
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
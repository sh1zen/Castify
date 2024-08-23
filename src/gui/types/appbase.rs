use crate::gui::components::hotkeys::KeyTypes;
use crate::gui::components::popup::{PopupMsg, PopupType};
use crate::gui::types::messages::Message;
use crate::gui::video::Video;
use crate::workers;
use iced::keyboard::key::Named;
use iced::keyboard::{Event, Key, Modifiers};
use iced::mouse::Event::ButtonPressed;
use iced::window::Id;
use iced::Event::{Keyboard, Window};
use iced::{window, Subscription};
use std::collections::HashMap;
use std::net::SocketAddr;
use gstreamer::ClockTime;
use gstreamer::prelude::ElementExt;
use crate::gui::resource::USE_WEBRTC;
use crate::utils::gist::create_ss_save_pipeline;

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
    pub(crate) os_supported: bool,
    pub(crate) is_caster: bool,
    pub(crate) page: Page,
    pub(crate) show_popup: Option<PopupType>,
    pub(crate) popup_msg: HashMap<PopupType, PopupMsg>,
    pub(crate) video: Video,
    pub(crate) hotkey_map: HotkeyMap,
}

impl App {
    pub fn new(supported:bool) -> App {
        App {
            os_supported: supported,
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

    pub(crate) fn launch_receiver(&mut self, socket_addr: Option<SocketAddr>) {

        let (tx, rx) = tokio::sync::mpsc::channel(1);

        if USE_WEBRTC {

            tokio::spawn(async move {
                let tt = crate::utils::net::WebRTCClient::new("ws://localhost:31413").await;
                tt.receive_video(tx).await;
            });

            let pipeline = create_ss_save_pipeline(rx).unwrap();
            pipeline.set_state(gstreamer::State::Playing).unwrap();
            let _ = pipeline.state(ClockTime::from_seconds(1));
            //self.video.set_pipeline(pipeline);
            // self.show_popup = None;
            //self.page = Page::Client;
        } else {
            let (tx, rx) = tokio::sync::mpsc::channel(1);
            tokio::spawn(async move {
                crate::utils::net::net::receiver(socket_addr, tx).await;
            });
            let pipeline = crate::utils::gist::create_view_pipeline(rx).unwrap();
            self.video.set_pipeline(pipeline);
            self.show_popup = None;
            self.page = Page::Client;
        }
    }

    pub(crate) fn launch_save_stream(&mut self) {
        workers::save_stream::get_instance().lock().unwrap().start();
    }
}
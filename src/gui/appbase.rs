use crate::gui::components::hotkeys::KeyTypes;
use crate::gui::components::popup::{PopupMsg, PopupType};
use crate::gui::resource::{FRAME_HEIGHT, FRAME_RATE, FRAME_WITH, USE_WEBRTC};
use crate::gui::types::messages::Message;
use crate::gui::video::Video;
use crate::workers;
use gstreamer::Pipeline;
use iced::keyboard::key::Named;
use iced::keyboard::{Event, Key, Modifiers};
use iced::window::Id;
use iced::Event::{Keyboard, Window};
use iced::{window, Subscription};
use iced_core::Size;
use std::collections::HashMap;
use std::net::SocketAddr;

#[derive(PartialEq, Eq)]
pub enum Page {
    Home,
    Caster,
    Client,
    Hotkeys,
    AreaSelection,
}

#[derive(Clone)]
pub struct HotkeyMap {
    pub pause: (Modifiers, Key),
    pub record: (Modifiers, Key),
    pub end_session: (Modifiers, Key),
    pub blank_screen: (Modifiers, Key),
    pub updating: KeyTypes,
}

impl Default for HotkeyMap {
    fn default() -> Self {
        HotkeyMap {
            pause: (Modifiers::CTRL, Key::Named(Named::F10)),
            record: (Modifiers::CTRL, Key::Named(Named::F11)),
            end_session: (Modifiers::CTRL, Key::Character("w".parse().unwrap())),
            blank_screen: (Modifiers::CTRL, Key::Named(Named::F2)),
            updating: KeyTypes::None,
        }
    }
}

pub struct App {
    pub(crate) os_supported: bool,
    pub(crate) page: Page,
    pub(crate) show_popup: Option<PopupType>,
    pub(crate) popup_msg: HashMap<PopupType, PopupMsg>,
    pub(crate) video: Video,
    pub(crate) hotkey_map: HotkeyMap,
    pub(crate) current_size: Size<f32>,
}

impl App {
    pub fn new(os_supported: bool) -> App {
        App {
            os_supported,
            page: Page::Home,
            show_popup: None,
            popup_msg: HashMap::new(),
            video: Video::new(),
            hotkey_map: Default::default(),
            current_size: Size { width: 400f32, height: 300f32 },
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
            //iced::event::Event::Mouse(ButtonPressed(_)) => Some(Message::Drag),
            _ => None,
        })
    }

    pub(crate) fn window_subscription(&self) -> Subscription<Message> {
        iced::event::listen_with(|event, _| match event {
            Window(Id::MAIN, window::Event::Resized { width, height }) => {
                Some(Message::WindowResized(width, height))
            }
            Window(Id::MAIN, window::Event::CloseRequested) => Some(Message::CloseRequested),
            _ => None,
        })
    }

    pub(crate) fn launch_receiver(&mut self, socket_addr: Option<SocketAddr>) {
        let pipeline: Pipeline = if USE_WEBRTC {
            let (tx, rx) = tokio::sync::mpsc::channel(1);
            tokio::spawn(async move {
                crate::utils::net::rtp::receiver(socket_addr, tx).await;
            });
            crate::utils::gist::create_rtp_view_pipeline(rx).unwrap()
        } else {
            let (tx, rx) = tokio::sync::mpsc::channel(1);
            tokio::spawn(async move {
                crate::utils::net::xgp::receiver(socket_addr, tx).await;
            });
            crate::utils::gist::create_view_pipeline(rx).unwrap()
        };

        self.video.set_pipeline(pipeline, FRAME_WITH, FRAME_HEIGHT, gstreamer::Fraction::new(FRAME_RATE, 1));
        self.show_popup = None;
        self.page = Page::Client;
    }

    pub(crate) fn launch_save_stream(&mut self) {
        workers::save_stream::get_instance().lock().unwrap().start();
    }
}



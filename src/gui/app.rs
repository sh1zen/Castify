use crate::gui::components::caster::caster_page;
use crate::gui::components::client::client_page;
use crate::gui::components::footer::footer;
use crate::gui::components::home::initial_page;
use crate::gui::components::hotkeys::{hotkeys, KeyTypes};
use crate::gui::components::popup::{show_popup, PopupMsg, PopupType};
use crate::gui::components::screen_overlay::screen_area_layer;
use crate::gui::components::{caster, home};
use crate::gui::resource::{open_link, CAST_SERVICE_PORT, FRAME_HEIGHT, FRAME_RATE, FRAME_WITH, USE_WEBRTC};
use crate::gui::style::styles::csx::StyleType;
use crate::gui::common::flags::Flags;
use crate::gui::common::messages::AppEvent;
use crate::gui::video::Video;
use crate::gui::widget::{Column, Container, IcedRenderer};
use crate::gui::widget::{Element, IcedParentExt};
use crate::workers;
use gstreamer::Pipeline;
use iced::application::{Appearance, DefaultStyle};
use iced::keyboard::key::Named;
use iced::keyboard::{Event, Key, Modifiers};
use iced::Event::{Keyboard, Window};
use iced::{window, Subscription};
use iced_core::window::Mode;
use iced_core::Size;
use iced_runtime::Task;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::process::exit;
use std::str::FromStr;
use std::time::Duration;
use tokio::time::sleep;

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
    transparent: bool,
}

impl App {
    pub fn new(flags: Flags) -> (Self, Task<AppEvent>) {
        (
            Self {
                os_supported: flags.os_supported,
                page: Page::Home,
                show_popup: None,
                popup_msg: HashMap::new(),
                video: Video::new(),
                hotkey_map: Default::default(),
                current_size: Size { width: 400f32, height: 300f32 },
                transparent: false,
            },
            Task::none(),
        )
    }

    pub(crate) fn update(&mut self, message: AppEvent) -> Task<AppEvent> {
        match message {
            AppEvent::Home => {
                self.hotkey_map.updating = KeyTypes::None;
                workers::caster::get_instance().lock().unwrap().close();
                self.show_popup = None;
                self.page = Page::Home;
                Task::none()
            }
            AppEvent::Mode(mode) => {
                match mode {
                    home::Message::ButtonCaster => {
                        self.page = Page::Caster
                    }
                    home::Message::ButtonReceiver => {
                        self.show_popup = Some(PopupType::IP);
                    }
                }
                Task::none()
            }
            AppEvent::Caster(mode) => {
                match mode {
                    caster::Message::Rec => {
                        workers::caster::get_instance().lock().unwrap().cast();
                    }
                    caster::Message::Pause => {
                        workers::caster::get_instance().lock().unwrap().pause();
                    }
                }
                Task::none()
            }
            AppEvent::BlankScreen => {
                workers::caster::get_instance().lock().unwrap().toggle_blank_screen();
                Task::none()
            }
            AppEvent::WindowResized(width, height) => {
                self.current_size = Size { width: width as f32, height: height as f32 };
                Task::none()
            }
            AppEvent::AreaSelection => {
                self.transparent = true;
                self.page = Page::AreaSelection;
                window::get_oldest().and_then(move |window| {
                    let commands = vec![
                        window::change_mode::<AppEvent>(window, Mode::Fullscreen),
                        window::toggle_decorations::<AppEvent>(window),
                        window::change_level::<AppEvent>(window, window::Level::AlwaysOnTop),
                    ];
                    Task::batch(commands)
                })
            }
            AppEvent::AreaSelected(rect) => {
                // set the new area selected
                self.transparent = false;
                workers::caster::get_instance().lock().unwrap().resize_rec_area(rect.x as i32, rect.y as i32, rect.width as u32, rect.height as u32);
                if self.page == Page::AreaSelection {
                    let task = window::get_oldest().and_then(move |window| {
                        let commands = vec![
                            window::change_mode::<AppEvent>(window, Mode::Windowed),
                            window::toggle_decorations::<AppEvent>(window),
                            window::change_level::<AppEvent>(window, window::Level::Normal),
                        ];
                        Task::batch(commands)
                    });
                    self.page = Page::Caster;
                    task
                } else {
                    Task::none()
                }
            }
            AppEvent::ConnectToCaster(mut caster_ip) => {
                if caster_ip == "auto" {
                    self.launch_receiver(None)
                } else if !caster_ip.contains(":") {
                    caster_ip = format!("{}:{}", caster_ip, CAST_SERVICE_PORT);
                    match SocketAddr::from_str(&*caster_ip) {
                        Ok(caster_socket_addr) => {
                            self.launch_receiver(Some(caster_socket_addr))
                        }
                        Err(e) => {
                            println!("{}", e);
                            *self.popup_msg.get_mut(&PopupType::IP).unwrap() = PopupMsg::String("".parse().unwrap())
                        }
                    }
                }
                Task::none()
            }
            AppEvent::SaveCapture => {
                self.launch_save_stream();
                Task::none()
            }
            AppEvent::SaveCaptureStop => {
                workers::save_stream::get_instance().lock().unwrap().stop();
                Task::none()
            }
            AppEvent::OpenWebPage(web_page) => {
                open_link(&web_page);
                Task::none()
            }
            AppEvent::HotkeysPage => {
                self.page = Page::Hotkeys;
                Task::none()
            }
            AppEvent::HotkeysTypePage(key) => {
                self.hotkey_map.updating = key;
                self.popup_msg.insert(
                    PopupType::HotkeyUpdate,
                    PopupMsg::HotKey(key),
                );
                self.show_popup = Some(PopupType::HotkeyUpdate);
                Task::none()
            }
            AppEvent::KeyPressed(item) => {
                if item == self.hotkey_map.pause {
                    let _ = self.update(AppEvent::Caster(caster::Message::Pause));
                } else if item == self.hotkey_map.record {
                    let _ = self.update(AppEvent::Caster(caster::Message::Rec));
                } else if item == self.hotkey_map.blank_screen {
                    let _ = self.update(AppEvent::BlankScreen);
                } else if item == self.hotkey_map.end_session {
                    let _ = self.update(AppEvent::CloseRequested);
                }
                Task::none()
            }
            AppEvent::HotkeysUpdate((modifier, key)) => {
                match self.hotkey_map.updating {
                    KeyTypes::Pause => {
                        self.hotkey_map.pause = (modifier, key)
                    }
                    KeyTypes::Record => {
                        self.hotkey_map.record = (modifier, key)
                    }
                    KeyTypes::BlankScreen => {
                        self.hotkey_map.blank_screen = (modifier, key)
                    }
                    KeyTypes::Close => {
                        self.hotkey_map.end_session = (modifier, key)
                    }
                    _ => {}
                }
                Task::none()
            }
            AppEvent::PopupMessage(msg) => {
                if self.popup_msg.contains_key(&msg.p_type) {
                    *self.popup_msg.get_mut(&msg.p_type).unwrap() = PopupMsg::String(msg.text)
                } else {
                    self.popup_msg.insert(msg.p_type, PopupMsg::String(msg.text));
                }
                Task::none()
            }
            AppEvent::ClosePopup => {
                self.show_popup = None;
                Task::none()
            }
            AppEvent::CloseRequested => {
                tokio::spawn(async {
                    workers::sos::get_instance().lock().unwrap().terminate();
                    sleep(Duration::from_millis(250)).await;
                    exit(0)
                });
                Task::none()
            }
            AppEvent::Ignore => {
                Task::none()
            }
            _ => {
                println!("Command not yet implemented!");
                Task::none()
            }
        }
    }

    pub(crate) fn view(&self) -> Element<AppEvent, StyleType, IcedRenderer> {
        let body = match self.page {
            Page::Home => {
                initial_page(self.os_supported)
            }
            Page::AreaSelection => {
                screen_area_layer()
            }
            Page::Caster => {
                caster_page()
            }
            Page::Client => {
                client_page(&self.video)
            }
            Page::Hotkeys => {
                hotkeys()
            }
        };

        let mut content = Column::new()
            .padding(4)
            .push(body)
            .push_if(self.page != Page::AreaSelection, footer);

        if self.show_popup.is_some() {
            content = Column::new().push(show_popup(self, Container::new(content)));
        }

        content.into()
    }

    pub fn style(&self, theme: &StyleType) -> Appearance {
        theme.default_style()
    }

    pub fn theme(&self) -> StyleType {
        if self.transparent {
            StyleType::SemiTransparent
        } else {
            StyleType::Venus
        }
    }

    pub(crate) fn subscription(&self) -> Subscription<AppEvent> {
        Subscription::batch([
            self.keyboard_subscription(),
            //self.mouse_subscription(),
            self.window_subscription()
        ])
    }

    pub(crate) fn keyboard_subscription(&self) -> Subscription<AppEvent> {
        //const NO_MODIFIER: Modifiers = Modifiers::empty();

        // used to update HotKeys
        if self.hotkey_map.updating != KeyTypes::None {
            iced::event::listen_with(|event, _, _| match event {
                Keyboard(Event::KeyPressed { key, modifiers, .. }) => {
                    Some(AppEvent::HotkeysUpdate((modifiers, key)))
                }
                _ => None,
            })
        } else {
            iced::event::listen_with(|event, _status, _id| match event {
                Keyboard(Event::KeyPressed { key, modifiers, .. }) => {
                    Some(AppEvent::KeyPressed((modifiers, key)))
                }
                _ => None,
            })
        }
    }

    pub(crate) fn window_subscription(&self) -> Subscription<AppEvent> {
        iced::event::listen_with(|event, _status, _id| match event {
            Window(window::Event::Resized(size)) => {
                Some(AppEvent::WindowResized(size.width as u32, size.height as u32))
            }
            Window(window::Event::CloseRequested) => Some(AppEvent::CloseRequested),
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
use crate::gui::components::caster::caster_page;
use crate::gui::components::client::client_page;
use crate::gui::components::footer::footer;
use crate::gui::components::hotkeys::{hotkeys, KeyTypes};
use crate::gui::components::popup::{show_popup, PopupMsg, PopupType};
use crate::gui::components::start::initial_page;
use crate::gui::components::{caster, start};
use crate::gui::resource::{open_link, CAST_SERVICE_PORT};
use crate::gui::theme::styles::csx::StyleType;
use crate::gui::types::appbase::{App, Page};
use crate::gui::types::messages::Message;
use iced::widget::{Column, Container};
use iced::{executor, Application, Command, Element, Subscription};
use std::net::SocketAddr;
use std::process::exit;
use std::str::FromStr;

impl Application for App {
    type Executor = executor::Default;
    type Message = Message;

    type Theme = StyleType;
    type Flags = App;

    fn new(flags: App) -> (App, Command<Message>) {
        (flags, Command::none())
    }

    fn title(&self) -> String {
        String::from("Screen Caster")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        fn launch_receiver(app: &mut App, socket_addr: Option<SocketAddr>) {
            let (tx, rx) = tokio::sync::mpsc::channel(1);
            tokio::spawn(async move {
                crate::utils::net::receiver(socket_addr, tx).await;
            });
            let pipeline = crate::utils::gist::create_pipeline(rx).unwrap();
            app.video.set_pipeline(pipeline);
            app.show_popup = None;
            app.page = Page::Client
        }

        match message {
            Message::Home => {
                self.hotkey_map.updating = KeyTypes::None;
                self.page = Page::Home
            }
            Message::Mode(mode) => {
                match mode {
                    start::Message::ButtonCaster => {
                        self.page = Page::Caster
                    }
                    start::Message::ButtonReceiver => {
                        self.show_popup = Some(PopupType::IP);
                    }
                }
            }
            Message::Caster(mode) => {
                match mode {
                    caster::Message::Rec => {
                        self.vdffd();
                    }
                    caster::Message::Pause => {
                        self.caster_opt.lock().unwrap().streaming = false;
                    }
                }
            }
            Message::ConnectToCaster(mut caster_ip) => {
                if caster_ip == "auto" {
                    launch_receiver(self, None)
                } else if !caster_ip.contains(":") {
                    caster_ip = format!("{}:{}", caster_ip, CAST_SERVICE_PORT);
                    match SocketAddr::from_str(&*caster_ip) {
                        Ok(caster_socket_addr) => {
                            launch_receiver(self, Some(caster_socket_addr))
                        }
                        Err(e) => {
                            println!("{}", e);
                            *self.popup_msg.get_mut(&PopupType::IP).unwrap() = PopupMsg::String("".parse().unwrap())
                        }
                    }
                }
            }
            Message::OpenWebPage(web_page) => open_link(&web_page),
            Message::HotkeysPage => {
                self.page = Page::Hotkeys
            }
            Message::HotkeysTypePage(key) => {
                self.hotkey_map.updating = key;
                self.popup_msg.insert(
                    PopupType::HotkeyUpdate,
                    PopupMsg::HotKey(key),
                );
                self.show_popup = Some(PopupType::HotkeyUpdate)
            }
            Message::KeyPressed((modifier, key)) => {
                if modifier == self.hotkey_map.pause.0 && key == self.hotkey_map.pause.1 {
                    self.update(Message::Pause);
                } else if modifier == self.hotkey_map.record.0 && key == self.hotkey_map.record.1 {
                   self.update(Message::Record);
                } else if modifier == self.hotkey_map.blank_screen.0 && key == self.hotkey_map.blank_screen.1 {
                    self.update(Message::BlankScreen);
                } else if modifier == self.hotkey_map.end_session.0 && key == self.hotkey_map.end_session.1 {
                    self.update(Message::CloseRequested);
                }
            }
            Message::HotkeysUpdate((modifier, key)) => {
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
            }
            Message::PopupMessage(msg) => {
                if self.popup_msg.contains_key(&msg.p_type) {
                    *self.popup_msg.get_mut(&msg.p_type).unwrap() = PopupMsg::String(msg.text)
                } else {
                    self.popup_msg.insert(msg.p_type, PopupMsg::String(msg.text));
                }
            }
            Message::ClosePopup => {
                self.show_popup = None
            }
            Message::CloseRequested => {
                exit(0)
            }
            _ => {
                println!("Command not yet implemented!");
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<Message, StyleType> {
        let body = match self.page {
            Page::Home => {
                initial_page(self)
            }
            Page::Caster => {
                caster_page(self)
            }
            Page::Client => {
                client_page(self)
            }
            Page::Hotkeys => {
                hotkeys(self)
            }
        };

        let footer = footer();

        let mut content = Column::new().padding(0).push(body).push(footer);

        if !self.show_popup.is_none() {
            content = Column::new().push(show_popup(self, Container::new(content)));
        }

        content.into()
    }

    fn theme(&self) -> Self::Theme {
        StyleType::Venus
    }

    fn subscription(&self) -> Subscription<Message> {
        /*
        /// handle mouse trascina schermo parte registra
        fn subscription(&self) -> Subscription<Message> {
        event::listen_with(|event, status| {
            if status == iced::event::Status::Captured {
                match event {
                    Mouse(ButtonPressed(Left)) => Some(Message::StartPan),
                    Mouse(ButtonReleased(Left)) => Some(Message::EndPan),
                    Mouse(CursorMoved {
                        position: Point { x, y },
                    }) => Some(Message::CursorMoved(x, y)),
                    Mouse(WheelScrolled {
                        delta: ScrollDelta::Lines { x: _, y },
                    }) => Some(Message::Scroll(y)),
                    _ => None,
                }
            } else {
                match event {
                    Window(_, window::Event::Resized { width, height }) => {
                        Some(Message::Resized(width, height))
                    }
                    _ => None,
                }
            }
        })
    }
         */

        Subscription::batch([
            self.keyboard_subscription(),
            self.mouse_subscription(),
            self.window_subscription()
        ])
    }
}


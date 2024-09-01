use crate::gui::appbase::{App, Page};
use crate::gui::components::caster::caster_page;
use crate::gui::components::caster::Message as CasterMessage;
use crate::gui::components::client::client_page;
use crate::gui::components::footer::footer;
use crate::gui::components::home::initial_page;
use crate::gui::components::hotkeys::{hotkeys, KeyTypes};
use crate::gui::components::popup::{show_popup, PopupMsg, PopupType};
use crate::gui::components::screen_overlay::screen_area_layer;
use crate::gui::components::{caster, home};
use crate::gui::resource::{open_link, CAST_SERVICE_PORT};
use crate::gui::theme::styles::csx::StyleType;
use crate::gui::types::messages::Message;
use crate::workers;
use iced::widget::{Column, Container};
use iced::{executor, Application, Command, Element, Subscription};
use iced_core::window::Mode;
use iced_core::Size;
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
        match message {
            Message::Home => {
                self.hotkey_map.updating = KeyTypes::None;
                self.page = Page::Home
            }
            Message::Mode(mode) => {
                match mode {
                    home::Message::ButtonCaster => {
                        self.is_caster = true;
                        self.page = Page::Caster
                    }
                    home::Message::ButtonReceiver => {
                        self.show_popup = Some(PopupType::IP);
                    }
                }
            }
            Message::Caster(mode) => {
                match mode {
                    caster::Message::Rec => {
                        workers::caster::get_instance().lock().unwrap().cast_screen();
                    }
                    caster::Message::Pause => {
                        workers::caster::get_instance().lock().unwrap().pause();
                    }
                    caster::Message::FullScreenSelected => {
                        // set to full screen
                        // if needed use .change_monitor(id) to set full screen for another monitor
                        workers::caster::get_instance().lock().unwrap().full_screen();
                    }
                    caster::Message::AreaSelected((x, y, w, h)) => {
                        // set here new screen area
                        // if needed use .change_monitor(id) to set resize_rec_area for another monitor
                        workers::caster::get_instance().lock().unwrap().resize_rec_area(x, y, w, h);
                    }
                }
            }
            Message::BlankScreen => {
                workers::caster::get_instance().lock().unwrap().toggle_blank_screen();
            }
            Message::WindowResized(width, height) => {
                self.current_size = Size { width: width as f32, height: height as f32 }
            }
            Message::AreaSelection => {
                let commands = vec![
                    iced_runtime::window::change_mode::<Message>(iced_core::window::Id::MAIN, Mode::Fullscreen),
                    iced_runtime::window::toggle_decorations::<Message>(iced_core::window::Id::MAIN),
                    iced_runtime::window::change_level::<Message>(iced_core::window::Id::MAIN, iced_core::window::Level::AlwaysOnTop),
                ];
                self.page = Page::AreaSelection;
                return Command::batch(commands);
            }
            Message::AreaSelected(rect) => {
                let commands = vec![
                    iced_runtime::window::change_mode::<Message>(iced_core::window::Id::MAIN, Mode::Windowed),
                    iced_runtime::window::toggle_decorations::<Message>(iced_core::window::Id::MAIN),
                    iced_runtime::window::change_level::<Message>(iced_core::window::Id::MAIN, iced_core::window::Level::Normal),
                ];
                workers::caster::get_instance().lock().unwrap().resize_rec_area(rect.x as i32, rect.y as i32, rect.width as u32, rect.height as u32);
                self.page = Page::Caster;
                return Command::batch(commands);
            }
            Message::ConnectToCaster(mut caster_ip) => {
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
            }
            Message::SaveCapture => {
                self.launch_save_stream();
            }
            Message::SaveCaptureStop => {
                workers::save_stream::get_instance().lock().unwrap().stop();
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
            Message::KeyPressed(item) => {
                if item == self.hotkey_map.pause {
                    let _ = self.update(Message::Caster(CasterMessage::Pause));
                } else if item == self.hotkey_map.record {
                    let _ = self.update(Message::Caster(CasterMessage::Rec));
                } else if item == self.hotkey_map.blank_screen {
                    let _ = self.update(Message::BlankScreen);
                } else if item == self.hotkey_map.end_session {
                    let _ = self.update(Message::CloseRequested);
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
            Page::AreaSelection => {
                screen_area_layer(self)
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
        Subscription::batch([
            self.keyboard_subscription(),
            //self.mouse_subscription(),
            self.window_subscription()
        ])
    }
}


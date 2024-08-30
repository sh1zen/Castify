use crate::gui::components::caster::caster_page;
use crate::gui::components::caster::Message as CasterMessage;
use crate::gui::components::client::client_page;
use crate::gui::components::footer::footer;
use crate::gui::components::hotkeys::{hotkeys, KeyTypes};
use crate::gui::components::popup::{show_popup, PopupMsg, PopupType};
use crate::gui::components::home::initial_page;
use crate::gui::components::{caster, home};
use crate::gui::resource::{open_link, CAST_SERVICE_PORT};
use crate::gui::theme::styles::csx::StyleType;
use crate::gui::types::appbase::{App, Page};
use crate::gui::types::messages::{AreaSelectionMessage, Message};
use crate::workers;
use iced::widget::{Column, Container};
use iced::{executor, Application, Command, Element, Subscription, Point};
use std::net::SocketAddr;
use std::process::exit;
use std::str::FromStr;
use crate::gui::types::appbase::CaptureMode;
use crate::gui::components::area_selector::AreaSelector;


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
                        let capture_mode = workers::caster::get_instance().lock().unwrap().capture_mode;
                        workers::caster::get_instance().lock().unwrap().cast_screen(capture_mode);
                    }
                    caster::Message::Pause => {
                        workers::caster::get_instance().lock().unwrap().pause();
                    }

                    caster::Message::FullScreenSelected => {
                        workers::caster::get_instance().lock().unwrap().capture_mode = CaptureMode::FullScreen;
                    }
                    caster::Message::AreaSelected => {
                        workers::caster::get_instance().lock().unwrap().capture_mode = CaptureMode::Area;
                    }

                }
            }
            Message::AreaSelection(msg) => {
                match msg {
                    AreaSelectionMessage::StartSelection { x, y } => {
                        // Inizializza la selezione
                        self.start_selection = Some(Point::new(x, y));
                        self.end_selection = None;
                        println!("Start selection at: ({}, {})", x, y);
                    }
                    AreaSelectionMessage::UpdateSelection { x, y } => {
                        // Aggiorna la selezione
                        if let Some(start) = self.start_selection {
                            self.end_selection = Some(Point::new(x, y));
                            println!("Selection updated to: ({}, {})", x, y);
                        }
                    }
                    AreaSelectionMessage::EndSelection => {
                        // Concludi la selezione e calcola i dati dell'area
                        if let (Some(start), Some(end)) = (self.start_selection, self.end_selection) {
                            let area_x = start.x.min(end.x);
                            let area_y = start.y.min(end.y);
                            let area_width = (start.x - end.x).abs();
                            let area_height = (start.y - end.y).abs();
                            println!("Selection area: x = {}, y = {}, width = {}, height = {}", area_x, area_y, area_width, area_height);

                            // Passa queste informazioni al worker
                            let caster_instance = workers::caster::get_instance();
                            let mut caster = caster_instance.lock().unwrap();
                            caster.set_area_selection(area_x, area_y, area_width, area_height);

                        }
                        self.start_selection = None;
                        self.end_selection = None;
                    }
                }
            }
            Message::BlankScreen => {
                workers::caster::get_instance().lock().unwrap().toggle_blank_screen();
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
        let body: Element<Message, StyleType> = match self.page {
            Page::Home => initial_page(self).into(),
            Page::Caster => {
                if self.selecting_area {
                    AreaSelector::new().view().map(Message::AreaSelection).into()
                } else {
                    caster_page(self).into()
                }
            }
            Page::Client => client_page(self).into(),
            Page::Hotkeys => hotkeys(self).into(),
        };

        let footer = footer().into();

        let mut content = Column::new().padding(0).push(body).push(footer);

        if !self.show_popup.is_none() {
            content = Column::new().push(show_popup(self, Container::new(content)).into());
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


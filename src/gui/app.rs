use std::process::exit;
use std::time::Duration;

use iced::widget::{Column, Container};
use iced::{executor, Application, Command, Element, Executor, Sandbox, Subscription};

use crate::gui::components::client::client_page;
use crate::gui::components::footer::footer;
use crate::gui::components::popup::{show_popup, PopupType};
use crate::gui::components::recording::recording_page;
use crate::gui::components::start::initial_page;
use crate::gui::components::start;
use crate::gui::theme::styles::csx::StyleType;
use crate::gui::types::appbase::{App, Page};
use crate::gui::types::messages::Message;

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
            Message::OpenWebPage(web_page) => Self::open_web(&web_page),
            Message::PopupMessage(msg) => {
                if self.popup_msg.contains_key(&msg.p_type) {
                    *self.popup_msg.get_mut(&msg.p_type).unwrap() = msg.text
                } else {
                    self.popup_msg.insert(msg.p_type, msg.text);
                }
            }
            Message::Mode(mode) => {
                match mode {
                    start::Message::ButtonCaster => {
                        let (tx, rx) = tokio::sync::mpsc::channel(10);

                        // qui genero le immagini e le invio tramite il canale tx
                        tokio::spawn(async move {
                            let mut uuid = 0;
                            loop {
                                tx.send(format!("Hello from sender!, {}", uuid)).await.unwrap();
                                uuid += 1;
                                tokio::time::sleep(Duration::from_secs(2)).await;
                            }
                        });

                        tokio::spawn(async move {
                            crate::utils::net::caster(Some(rx)).await;
                        });

                        self.page = Page::Recording
                    }
                    start::Message::ButtonReceiver => {
                        self.show_popup = Some(PopupType::IP);

                        tokio::spawn(async move {
                            crate::utils::net::receiver(None).await
                        });

                        self.page = Page::Client
                    }
                }
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
            Page::Recording => {
                recording_page(self)
            }
            Page::Client => {
                client_page(self)
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
    /*
     fn subscription(&self) -> Subscription<Self::Message> {
        iced::subscription::channel("updates", 10, |mut s| async move {
            let (sender, mut receiver) = channel(10);
            s.send(Message::UpdateChannel(sender)).await.unwrap();
            loop {
                let _ = receiver.recv().await;
                s.send(Message::Ignore).await.unwrap();
            }
        })
    }
     */
}


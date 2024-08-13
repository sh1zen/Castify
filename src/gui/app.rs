use std::process::exit;
use std::time::Duration;

use iced::widget::Column;
use iced::{executor, Application, Command, Element, Executor, Sandbox, Subscription};

use crate::gui::components::client::client_page;
use crate::gui::components::footer::footer;
use crate::gui::components::recording::recording_page;
use crate::gui::components::start;
use crate::gui::components::start::initial_page;
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
            Message::Mode(mode) => {
                match mode {
                    start::Message::ButtonCaster => {
                        println!("Caster pressed");
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
                        println!("Receiver pressed");
                        let handle = tokio::runtime::Handle::current();
                        let job_handler = handle.spawn(async move {
                            crate::utils::net::receiver().await
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

        let content = Column::new().padding(0).push(body).push(footer);

        content.into()
    }

    fn theme(&self) -> Self::Theme {
        StyleType::Venus
    }

    fn subscription(&self) -> Subscription<Message> {
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


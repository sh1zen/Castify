use crate::capture::Capture;
use crate::gui::components::client::client_page;
use crate::gui::components::footer::footer;
use crate::gui::components::popup::{show_popup, PopupType};
use crate::gui::components::recording::recording_page;
use crate::gui::components::start;
use crate::gui::components::start::initial_page;
use crate::gui::resource::{CAST_SERVICE_PORT, FRAME_RATE};
use crate::gui::theme::styles::csx::StyleType;
use crate::gui::types::appbase::{App, Page};
use crate::gui::types::messages::Message;
use gstreamer::prelude::ElementExt;
use gstreamer_video::gst;
use iced::widget::{Column, Container};
use iced::{executor, Application, Command, Element, Executor, Sandbox, Subscription};
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
            let (tx, mut rx) = tokio::sync::mpsc::channel(FRAME_RATE as usize);
            tokio::spawn(async move {
                crate::utils::net::receiver(socket_addr, tx).await;
            });
            let pipeline = crate::utils::gist::create_pipeline(rx).unwrap();
            app.video.set_pipeline(pipeline);
            /*
            tokio::spawn(async move {

                pipeline.set_state(gst::State::Playing).expect("GStream:: Failed set gst::State::Playing");
                let bus = pipeline.bus().unwrap();
                for msg in bus.iter_timed(gst::ClockTime::NONE) {
                    match msg.view() {
                        gst::MessageView::Eos(..) => {
                            println!("Reached End of Stream");
                            break;
                        }
                        gst::MessageView::Error(err) => {
                            println!(
                                "Error from {:?}: {} ({:?})",
                                err.src().map(|s| s.path_string()),
                                err.error(),
                                err.debug()
                            );
                            break;
                        }
                        gst::MessageView::Warning(warning) => {
                            println!(
                                "Warning from {:?}: {} ({:?})",
                                warning.src().map(|s| s.path_string()),
                                warning.error(),
                                warning.debug()
                            );
                        }
                        _ => () //e => println!("{:?}", e),
                    }
                }
                pipeline.set_state(gst::State::Null).unwrap();
            });
            */
            app.show_popup = None;
            app.page = Page::Client
        }

        match message {
            Message::Mode(mode) => {
                match mode {
                    start::Message::ButtonCaster => {
                        let (tx, rx) = tokio::sync::mpsc::channel(30);
                        // generate frames
                        tokio::spawn(async move {
                            let mut capture = Capture::new();
                            capture.set_framerate(FRAME_RATE as f32);
                            capture.stream(capture.main.clone(), tx).await;
                        });
                        // send frames over the local network
                        tokio::spawn(async move {
                            crate::utils::net::caster(rx).await;
                        });
                        self.page = Page::Recording
                    }
                    start::Message::ButtonReceiver => {
                        self.show_popup = Some(PopupType::IP);
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
                        Err(E) => {
                            println!("{}", E);
                            *self.popup_msg.get_mut(&PopupType::IP).unwrap() = "".parse().unwrap()
                        }
                    }
                }
            }
            Message::OpenWebPage(web_page) => Self::open_web(&web_page),
            Message::PopupMessage(msg) => {
                if self.popup_msg.contains_key(&msg.p_type) {
                    *self.popup_msg.get_mut(&msg.p_type).unwrap() = msg.text
                } else {
                    self.popup_msg.insert(msg.p_type, msg.text);
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
                client_page(self, self.video.0.borrow_mut())
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


use std::thread::Thread;

use iced::{Subscription, window};
use iced::Event::{Keyboard, Window};
use iced::keyboard::{Event, Key, Modifiers};
use iced::keyboard::key::Named;
use iced::mouse::Event::ButtonPressed;
use iced::window::Id;

use crate::gui::types::messages::Message;

pub enum Page {
    Home,
    Recording,
    Client,
}

pub struct App {
    caster: bool,
    pub(crate) page: Page,
    threads: Vec<Thread>,
}

impl App {
    pub fn new() -> App {
        App {
            caster: false,
            page: Page::Home,
            threads: vec![],
        }
    }

    pub(crate) fn keyboard_subscription(&self) -> Subscription<Message> {
        const NO_MODIFIER: Modifiers = Modifiers::empty();

        iced::event::listen_with(|event, _| match event {
            Keyboard(Event::KeyPressed { key, modifiers, .. }) => match modifiers {
                Modifiers::COMMAND => match key.as_ref() {
                    Key::Character("q") => Some(Message::CloseRequested),
                    Key::Character("t") => Some(Message::CtrlTPressed),
                    Key::Named(Named::Backspace) => Some(Message::ResetButtonPressed),
                    Key::Character("d") => Some(Message::CtrlDPressed),
                    Key::Named(Named::ArrowLeft) => Some(Message::ArrowPressed(false)),
                    Key::Named(Named::ArrowRight) => Some(Message::ArrowPressed(true)),
                    _ => None,
                },
                Modifiers::SHIFT => match key {
                    //Key::Named(Named::Tab) => Some(Message::SwitchPage(false)),
                    _ => { println!("ciao"); None },
                },
                NO_MODIFIER => match key {
                    Key::Named(Named::Enter) => Some(Message::ReturnKeyPressed),
                    Key::Named(Named::Escape) => Some(Message::EscKeyPressed),
                    //Key::Named(Named::Tab) => Some(Message::SwitchPage(true)),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        })
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

    pub(crate) fn open_web(web_page: &String) {
        let url = web_page;
        #[cfg(target_os = "windows")]
        std::process::Command::new("explorer")
            .arg(url)
            .spawn()
            .unwrap();
        #[cfg(target_os = "macos")]
        std::process::Command::new("open").arg(url).spawn().unwrap();
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .unwrap();
    }
}
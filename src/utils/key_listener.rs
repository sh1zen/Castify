use crate::gui::common::messages::AppEvent;
use iced::keyboard::Key as icedKey;
use iced::{futures::{SinkExt, Stream}, stream};
use iced_core::keyboard::key::Named;
use rdev::{listen, EventType, Key as RdevKey};
#[cfg(target_os = "linux")]
use rdev::start_grab_listen;
#[cfg(not(target_os = "linux"))]
use rdev::grab;
use tokio::sync::mpsc::channel;

pub fn global_key_listener() -> impl Stream<Item=AppEvent> {
    let (sender, mut receiver) = channel(10);

    std::thread::spawn(move || {
        #[cfg(target_os = "linux")]
        start_grab_listen(move |event| {
            sender.blocking_send(event.clone()).ok();
            Some(event)
        }).unwrap_or_default();
        #[cfg(not(target_os = "linux"))]
        grab(move |event| {
            sender.blocking_send(event.clone()).ok();
            Some(event)
        }).unwrap_or_default();
    });

    /*
    std::thread::spawn(move || {
        listen(move |event| {
            sender.blocking_send(event.clone()).unwrap_or_default();
        }).unwrap_or_default();
    });
     */

    stream::channel(10, move |mut output| async move {
        let mut handler = KeyState::new();

        loop {
            if let Some(event) = receiver.recv().await {
                if let Some((modifier, key)) = handler.mapping(event) {
                    output.send(AppEvent::KeyPressed(modifier, key)).await.unwrap_or_default();
                }
            }
        }
    })
}

struct KeyState {
    alt: bool,
    control: bool,
    shift: bool,
    logo: bool,
}

impl KeyState {
    pub fn new() -> Self {
        KeyState {
            alt: false,
            control: false,
            shift: false,
            logo: false,
        }
    }

    pub fn mapping(&mut self, event: rdev::Event) -> Option<(iced::keyboard::Modifiers, icedKey)> {
        match event.event_type {
            EventType::KeyPress(key) => match key {
                RdevKey::Alt => {
                    self.alt = true;
                    Some((self.to_modifiers(), icedKey::Unidentified))
                }
                RdevKey::ShiftLeft | RdevKey::ShiftRight => {
                    self.shift = true;
                    Some((self.to_modifiers(), icedKey::Unidentified))
                }
                RdevKey::ControlLeft | RdevKey::ControlRight => {
                    self.control = true;
                    Some((self.to_modifiers(), icedKey::Unidentified))
                }
                RdevKey::MetaLeft | RdevKey::MetaRight => {
                    self.logo = true;
                    Some((self.to_modifiers(), icedKey::Unidentified))
                }
                _ => {
                    Some((self.to_modifiers(), self.to_iced(key)))
                }
            },
            EventType::KeyRelease(key) => match key {
                RdevKey::Alt => {
                    self.alt = false;
                    None
                }
                RdevKey::ShiftLeft | RdevKey::ShiftRight => {
                    self.shift = false;
                    None
                }
                RdevKey::ControlLeft | RdevKey::ControlRight => {
                    self.control = false;
                    None
                }
                RdevKey::MetaLeft | RdevKey::MetaRight => {
                    self.logo = false;
                    None
                }
                _ => None,
            },
            _ => None,
        }
    }

    fn to_iced(&self, rdev_key: RdevKey) -> icedKey {
        match rdev_key {
            // mapping chars
            RdevKey::KeyA => icedKey::Character("a".into()),
            RdevKey::KeyB => icedKey::Character("b".into()),
            RdevKey::KeyC => icedKey::Character("c".into()),
            RdevKey::KeyD => icedKey::Character("d".into()),
            RdevKey::KeyE => icedKey::Character("e".into()),
            RdevKey::KeyF => icedKey::Character("f".into()),
            RdevKey::KeyG => icedKey::Character("g".into()),
            RdevKey::KeyH => icedKey::Character("h".into()),
            RdevKey::KeyI => icedKey::Character("i".into()),
            RdevKey::KeyJ => icedKey::Character("j".into()),
            RdevKey::KeyK => icedKey::Character("k".into()),
            RdevKey::KeyL => icedKey::Character("l".into()),
            RdevKey::KeyM => icedKey::Character("m".into()),
            RdevKey::KeyN => icedKey::Character("n".into()),
            RdevKey::KeyO => icedKey::Character("o".into()),
            RdevKey::KeyP => icedKey::Character("p".into()),
            RdevKey::KeyQ => icedKey::Character("q".into()),
            RdevKey::KeyR => icedKey::Character("r".into()),
            RdevKey::KeyS => icedKey::Character("s".into()),
            RdevKey::KeyT => icedKey::Character("t".into()),
            RdevKey::KeyU => icedKey::Character("u".into()),
            RdevKey::KeyV => icedKey::Character("v".into()),
            RdevKey::KeyW => icedKey::Character("w".into()),
            RdevKey::KeyX => icedKey::Character("x".into()),
            RdevKey::KeyY => icedKey::Character("y".into()),
            RdevKey::KeyZ => icedKey::Character("z".into()),

            // mapping NumKeys
            RdevKey::Num1 => icedKey::Character("1".into()),
            RdevKey::Num2 => icedKey::Character("2".into()),
            RdevKey::Num3 => icedKey::Character("3".into()),
            RdevKey::Num4 => icedKey::Character("4".into()),
            RdevKey::Num5 => icedKey::Character("5".into()),
            RdevKey::Num6 => icedKey::Character("6".into()),
            RdevKey::Num7 => icedKey::Character("7".into()),
            RdevKey::Num8 => icedKey::Character("8".into()),
            RdevKey::Num9 => icedKey::Character("9".into()),
            RdevKey::Num0 => icedKey::Character("0".into()),

            // mapping F1..F12 keys
            RdevKey::F1 => icedKey::Named(Named::F1),
            RdevKey::F2 => icedKey::Named(Named::F2),
            RdevKey::F3 => icedKey::Named(Named::F3),
            RdevKey::F4 => icedKey::Named(Named::F4),
            RdevKey::F5 => icedKey::Named(Named::F5),
            RdevKey::F6 => icedKey::Named(Named::F6),
            RdevKey::F7 => icedKey::Named(Named::F7),
            RdevKey::F8 => icedKey::Named(Named::F8),
            RdevKey::F9 => icedKey::Named(Named::F9),
            RdevKey::F10 => icedKey::Named(Named::F10),
            RdevKey::F11 => icedKey::Named(Named::F11),
            RdevKey::F12 => icedKey::Named(Named::F12),

            // mapping some special
            RdevKey::Pause => icedKey::Named(Named::Pause),
            RdevKey::Return => icedKey::Named(Named::Enter),
            RdevKey::Escape => icedKey::Named(Named::Escape),

            _ => icedKey::Unidentified,
        }
    }

    fn to_modifiers(&self) -> iced::keyboard::Modifiers {
        let mut modifiers = iced::keyboard::Modifiers::empty();
        if self.alt {
            modifiers |= iced::keyboard::Modifiers::ALT;
        }
        if self.control {
            modifiers |= iced::keyboard::Modifiers::CTRL;
        }
        if self.shift {
            modifiers |= iced::keyboard::Modifiers::SHIFT;
        }
        modifiers
    }
}
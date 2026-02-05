use crate::gui::common::messages::AppEvent;
use iced::keyboard::Key as icedKey;
use iced::keyboard::key::Named;
use iced::keyboard::{Key, Modifiers};
use iced::{
    futures::{SinkExt, Stream},
    stream,
};
use rdev::{EventType, Key as RdevKey, listen};
use tokio::sync::mpsc::channel;

pub fn global_key_listener() -> impl Stream<Item = AppEvent> {
    let (sender, mut receiver) = channel(20);

    std::thread::spawn(move || {
        let _ = listen(move |event| {
            let _ = sender.blocking_send(event.clone());
        });
    });

    stream::channel(
        10,
        move |mut output: iced::futures::channel::mpsc::Sender<AppEvent>| async move {
            let mut handler = KeyState::new();

            loop {
                if let Some(event) = receiver.recv().await
                    && let Some((modifier, key)) = handler.mapping(event)
                {
                    let _ = output.send(AppEvent::KeyEvent(modifier, key)).await;
                }
            }
        },
    )
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

    pub fn mapping(&mut self, event: rdev::Event) -> Option<(Modifiers, icedKey)> {
        match event.event_type {
            EventType::KeyPress(key) => {
                self.set_modifier(key, true);
                None
            }
            EventType::KeyRelease(key) => match key {
                RdevKey::Alt
                | RdevKey::ShiftLeft
                | RdevKey::ShiftRight
                | RdevKey::ControlLeft
                | RdevKey::ControlRight
                | RdevKey::MetaLeft
                | RdevKey::MetaRight => {
                    self.set_modifier(key, false);
                    None
                }
                _ => Some((self.to_modifiers(), self.to_iced(key))),
            },
            _ => None,
        }
    }

    fn set_modifier(&mut self, key: RdevKey, is_pressed: bool) {
        match key {
            RdevKey::Alt => self.alt = is_pressed,
            RdevKey::ShiftLeft | RdevKey::ShiftRight => self.shift = is_pressed,
            RdevKey::ControlLeft | RdevKey::ControlRight => self.control = is_pressed,
            RdevKey::MetaLeft | RdevKey::MetaRight => self.logo = is_pressed,
            _ => {}
        }
    }

    fn to_iced(&self, rdev_key: RdevKey) -> icedKey {
        if let Some(c) = Self::alpha_key(rdev_key) {
            return icedKey::Character(c.into());
        }
        if let Some(c) = Self::digit_key(rdev_key) {
            return icedKey::Character(c.into());
        }
        match rdev_key {
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

    fn alpha_key(key: RdevKey) -> Option<&'static str> {
        match key {
            RdevKey::KeyA => Some("a"),
            RdevKey::KeyB => Some("b"),
            RdevKey::KeyC => Some("c"),
            RdevKey::KeyD => Some("d"),
            RdevKey::KeyE => Some("e"),
            RdevKey::KeyF => Some("f"),
            RdevKey::KeyG => Some("g"),
            RdevKey::KeyH => Some("h"),
            RdevKey::KeyI => Some("i"),
            RdevKey::KeyJ => Some("j"),
            RdevKey::KeyK => Some("k"),
            RdevKey::KeyL => Some("l"),
            RdevKey::KeyM => Some("m"),
            RdevKey::KeyN => Some("n"),
            RdevKey::KeyO => Some("o"),
            RdevKey::KeyP => Some("p"),
            RdevKey::KeyQ => Some("q"),
            RdevKey::KeyR => Some("r"),
            RdevKey::KeyS => Some("s"),
            RdevKey::KeyT => Some("t"),
            RdevKey::KeyU => Some("u"),
            RdevKey::KeyV => Some("v"),
            RdevKey::KeyW => Some("w"),
            RdevKey::KeyX => Some("x"),
            RdevKey::KeyY => Some("y"),
            RdevKey::KeyZ => Some("z"),
            _ => None,
        }
    }

    fn digit_key(key: RdevKey) -> Option<&'static str> {
        match key {
            RdevKey::Num0 => Some("0"),
            RdevKey::Num1 => Some("1"),
            RdevKey::Num2 => Some("2"),
            RdevKey::Num3 => Some("3"),
            RdevKey::Num4 => Some("4"),
            RdevKey::Num5 => Some("5"),
            RdevKey::Num6 => Some("6"),
            RdevKey::Num7 => Some("7"),
            RdevKey::Num8 => Some("8"),
            RdevKey::Num9 => Some("9"),
            _ => None,
        }
    }

    fn to_modifiers(&self) -> Modifiers {
        let mut modifiers = Modifiers::empty();
        if self.alt {
            modifiers |= Modifiers::ALT;
        }
        if self.control {
            modifiers |= Modifiers::CTRL;
        }
        if self.shift {
            modifiers |= Modifiers::SHIFT;
        }
        modifiers
    }
}

pub fn valid_iced_key(key: Key) -> bool {
    matches!(
        key,
        Key::Character(_)
            | Key::Named(Named::F1 | Named::F2 | Named::F3)
            | Key::Named(Named::F4 | Named::F5 | Named::F6)
            | Key::Named(Named::F7 | Named::F8 | Named::F9)
            | Key::Named(Named::F10 | Named::F11 | Named::F12)
            | Key::Named(Named::Pause | Named::Enter | Named::Escape)
    )
}

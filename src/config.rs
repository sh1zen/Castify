use crate::gui::components::hotkeys::KeyTypes;
use crate::utils::sos::SignalOfStop;
use crate::workers::caster::Caster;
use crate::workers::receiver::Receiver;
use crate::workers::WorkerClose;
use iced_core::keyboard::key::Named;
use iced_core::keyboard::{Key, Modifiers};
use iced_core::Size;
use local_ip_address::local_ip;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};

pub enum Mode {
    Caster(Caster),
    Receiver(Receiver),
}

impl Mode {
    pub fn close(&mut self) {
        match self {
            Mode::Caster(closable) => closable.close(),
            Mode::Receiver(closable) => closable.close(),
        }
    }
}

#[derive(Clone, PartialEq)]
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

pub struct Config {
    pub hotkey_map: HotkeyMap,
    pub window_size: Size,
    pub e_time: u64,
    pub mode: Option<Mode>,
    pub public_ip: Arc<Mutex<Option<Ipv4Addr>>>,
    pub local_ip: Option<Ipv4Addr>,
    pub sos: SignalOfStop,
}

impl Config {
    pub fn new() -> Self {
        let conf = Config {
            hotkey_map: Default::default(),
            window_size: Size { width: 680f32, height: 460f32 },
            e_time: 0,
            mode: None,
            public_ip: Arc::new(Mutex::new(None)),
            local_ip: local_ip().ok().and_then(|ip|
                if let IpAddr::V4(ip) = ip {
                    Some(ip)
                } else {
                    None
                }),
            sos: SignalOfStop::new(),
        };

        let public_ip = Arc::clone(&conf.public_ip);
        tokio::spawn(async move {
            if let Some(ip) = public_ip::addr_v4().await {
                public_ip.lock().unwrap().replace(ip);
            }
        });

        conf
    }

    pub fn reset_mode(&mut self) {
        if self.mode.is_some() {
            let mut mode = self.mode.take().unwrap();
            mode.close();
        }
    }
}

/// Returns a version as specified in Cargo.toml
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}


pub fn app_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}
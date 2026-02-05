use crate::gui::common::hotkeys::KeyTypes;
use crate::utils::flags::Flags;
use crate::utils::path::default_saving_path;
use crate::utils::sos::SignalOfStop;
use crate::utils::string::capitalize_first_letter;
use crate::workers::WorkerClose;
use crate::workers::caster::Caster;
use crate::workers::receiver::Receiver;
use castbox::Arw;
use chrono::Local;
use iced::Size;
use iced::keyboard::key::Named;
use iced::keyboard::{Key, Modifiers};
use local_ip_address::local_ip;
use native_dialog::DialogBuilder;
use std::net::{IpAddr, Ipv4Addr};
use std::ops::DerefMut;

pub enum Mode {
    Caster(Caster),
    Receiver(Receiver),
}

impl Mode {
    pub fn close(&mut self) {
        match self {
            Mode::Caster(caster) => caster.close(),
            Mode::Receiver(receiver) => receiver.close(),
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
    pub shortcuts: HotkeyMap,
    pub window_size: Size,
    pub e_time: u64,
    pub mode: Option<Mode>,
    pub public_ip: Arw<Option<Ipv4Addr>>,
    pub local_ip: Option<Ipv4Addr>,
    pub sos: SignalOfStop,
    pub multi_instance: bool,
    pub fps: u32,
}

impl Config {
    pub fn new(flags: Flags) -> Self {
        let conf = Config {
            shortcuts: Default::default(),
            window_size: Size {
                width: 680f32,
                height: 460f32,
            },
            e_time: 0,
            mode: None,
            public_ip: Arw::new(None),
            local_ip: local_ip().ok().and_then(|ip| {
                if let IpAddr::V4(ip) = ip {
                    Some(ip)
                } else {
                    None
                }
            }),
            sos: SignalOfStop::new(),
            multi_instance: flags.multi_instance,
            fps: 30,
        };

        let public_ip = Arw::clone(&conf.public_ip);
        tokio::spawn(async move {
            if let Some(ip) = public_ip::addr_v4().await {
                public_ip.as_mut().deref_mut().replace(ip);
            }
        });

        conf
    }

    pub fn reset_mode(&mut self) {
        if let Some(mut mode) = self.mode.take() {
            mode.close();
        }
    }
}

pub fn saving_path() -> String {
    let default_path = default_saving_path();

    let save_p = DialogBuilder::file()
        .set_location(&default_path)
        .set_filename(&*Local::now().format("%Y-%m-%d_%H-%M-%S").to_string())
        .set_title("Save")
        .add_filter("Video", ["mp4", "mkv", "mov"])
        .save_single_file()
        .show()
        .unwrap();

    if let Some(path) = save_p {
        path.into_os_string().into_string().unwrap()
    } else {
        default_path
    }
}

pub fn app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn app_name() -> String {
    capitalize_first_letter(env!("CARGO_PKG_NAME"))
}

pub fn app_id() -> String {
    String::from(env!("CARGO_PKG_NAME"))
        .chars()
        .filter(|c| !c.is_whitespace() && !c.is_numeric())
        .collect::<String>()
        .to_lowercase()
}

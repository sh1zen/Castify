use crate::assets::{APP_NAME, CAST_SERVICE_PORT, FRAME_HEIGHT, FRAME_RATE, FRAME_WITH, USE_WEBRTC};
use crate::config::{Config, Mode};
use crate::gui::common::datastructure::ScreenRect;
use crate::gui::common::messages::AppEvent;
use crate::gui::components::caster::caster_page;
use crate::gui::components::client::client_page;
use crate::gui::components::footer::footer;
use crate::gui::components::home::initial_page;
use crate::gui::components::hotkeys::{hotkeys, KeyTypes};
use crate::gui::components::popup::{show_popup, Popup, PopupMsg, PopupType};
use crate::gui::components::{caster, home, popup};
use crate::gui::style::styles::csx::StyleType;
use crate::gui::video::Video;
use crate::gui::widget::{Column, Container, Element, IcedRenderer};
use crate::windows::GuiWindow;
use crate::workers;
use crate::workers::caster::Caster;
use gstreamer::Pipeline;
use iced::{window::Id, Task};
use std::hash::Hash;
use std::net::SocketAddr;
use std::str::FromStr;

#[derive(PartialEq, Eq)]
pub enum Page {
    Home,
    Caster,
    Client,
    Hotkeys,
}

pub struct MainWindow {
    page: Page,
    popup: Popup,
    pub(crate) video: Video,
}

#[derive(Debug, Clone)]
pub enum MainWindowEvent {
    Home,
    /// the app mode caster / receiver
    Mode(home::Message),
    /// caster play pause
    CasterToggleStreaming,
    /// Set Caster monitor
    CasterMonitor(u32),
    /// A collector of all popups messages
    PopupMessage(popup::Interaction),
    /// close any popup
    ClosePopup,
    /// Connect to caster, passing caster ip as String
    ConnectToCaster(String),
    /// Save the capture
    SaveCapture,
    /// stop saving capture
    SaveCaptureStop,
    /// Ignore
    Ignore,
    /// Setup hotkeys
    HotkeysPage,
    /// handle hot keys request update
    HotkeysTypePage(KeyTypes),
    /// Request for area selection page
    AreaSelection,
    /// Messages for handling area selection, set to 0 to restore default screen size
    AreaSelectedFullScreen,
    /// Quit the app
    ExitApp,
    /// Open the supplied web page
    OpenWebPage(String),
    /// Toggle Dark Mode
    DarkModeToggle,
}

impl GuiWindow for MainWindow {
    type Message = MainWindowEvent;

    fn new() -> Self {
        Self {
            // make as unique initializer using same as workers
            page: Page::Home,
            popup: Popup::new(),
            video: Video::new(),
        }
    }

    fn title(&self) -> String {
        APP_NAME.into()
    }

    fn update(&mut self, _id: Id, message: MainWindowEvent, config: &mut Config) -> Task<AppEvent> {
        match message {
            MainWindowEvent::Home => {
                config.hotkey_map.updating = KeyTypes::None;
                config.reset_mode();
                println!("{:?}", config.mode);
                self.popup.hide();
                self.page = Page::Home;
                Task::none()
            }
            MainWindowEvent::Mode(mode) => {
                match mode {
                    home::Message::ButtonCaster => {
                        config.mode = Some(Mode::Caster(Caster::new()));
                        self.page = Page::Caster
                    }
                    home::Message::ButtonReceiver => {
                        config.mode = Some(Mode::Client);
                        self.popup.show(PopupType::IP);
                    }
                }
                Task::none()
            }
            MainWindowEvent::CasterToggleStreaming => {
                Task::done(AppEvent::CasterToggleStreaming)
            }
            MainWindowEvent::CasterMonitor(mon) => {
                if let Some(Mode::Caster(caster)) = &mut config.mode {
                    caster.change_monitor(mon);
                }
                Task::none()
            }
            MainWindowEvent::PopupMessage(msg) => {
                if self.popup.has(&msg.p_type) {
                    *self.popup.get_mut(&msg.p_type).unwrap() = PopupMsg::String(msg.text)
                } else {
                    self.popup.insert(msg.p_type, PopupMsg::String(msg.text));
                }
                Task::none()
            }
            MainWindowEvent::ClosePopup => {
                self.popup.hide();
                Task::none()
            }
            MainWindowEvent::HotkeysPage => {
                self.page = Page::Hotkeys;
                Task::none()
            }
            MainWindowEvent::HotkeysTypePage(key) => {
                config.hotkey_map.updating = key;
                self.popup.insert(
                    PopupType::HotkeyUpdate,
                    PopupMsg::HotKey(key),
                );
                self.popup.show(PopupType::HotkeyUpdate);
                Task::none()
            }
            MainWindowEvent::ConnectToCaster(mut caster_ip) => {
                self.popup.hide();
                self.page = Page::Client;
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
                            *self.popup.get_mut(&PopupType::IP).unwrap() = PopupMsg::String("".parse().unwrap())
                        }
                    }
                }
                Task::none()
            }
            MainWindowEvent::SaveCapture => {
                self.launch_save_stream();
                Task::none()
            }
            MainWindowEvent::SaveCaptureStop => {
                workers::save_stream::get_instance().lock().unwrap().stop();
                Task::none()
            }
            MainWindowEvent::OpenWebPage(s) => { Task::done(AppEvent::OpenWebPage(s)) }
            MainWindowEvent::AreaSelection => { Task::done(AppEvent::AreaSelection) }
            MainWindowEvent::AreaSelectedFullScreen => { Task::done(AppEvent::AreaSelected(ScreenRect::default())) }
            MainWindowEvent::ExitApp => { Task::done(AppEvent::ExitApp) }
            MainWindowEvent::DarkModeToggle => {
                config.dark_mode = !config.dark_mode;
                Task::none()
            }
            MainWindowEvent::Ignore => {
                Task::none()
            }
        }
    }

    fn view(&self, config: &Config) -> Element<MainWindowEvent, StyleType, IcedRenderer> {
        let body = match self.page {
            Page::Home => {
                initial_page(config)
            }
            Page::Caster => {
                caster_page(config)
            }
            Page::Client => {
                client_page(&self.video)
            }
            Page::Hotkeys => {
                hotkeys()
            }
        };

        let mut content = Column::new()
            .push(body)
            .push(footer());

        if self.popup.is_visible() {
            content = Column::new().push(show_popup(&self.popup, config, Container::new(content)));
        }

        content.into()
    }
}

impl MainWindow {
    fn launch_receiver(&mut self, socket_addr: Option<SocketAddr>) {
        let pipeline: Pipeline = if USE_WEBRTC {
            let (tx, rx) = tokio::sync::mpsc::channel(1);
            tokio::spawn(async move {
                crate::utils::net::rtp::receiver(socket_addr, tx).await;
            });
            crate::utils::gist::create_rtp_view_pipeline(rx).unwrap()
        } else {
            let (tx, rx) = tokio::sync::mpsc::channel(1);
            tokio::spawn(async move {
                crate::utils::net::xgp::receiver(socket_addr, tx).await;
            });
            crate::utils::gist::create_view_pipeline(rx).unwrap()
        };

        self.video.set_pipeline(pipeline, FRAME_WITH, FRAME_HEIGHT, gstreamer::Fraction::new(FRAME_RATE, 1));
    }

    fn launch_save_stream(&mut self) {
        workers::save_stream::get_instance().lock().unwrap().start();
    }
}
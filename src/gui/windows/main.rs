use crate::assets::{APP_NAME, CAST_SERVICE_PORT, FRAME_HEIGHT, FRAME_RATE, FRAME_WITH};
use crate::config::{Config, Mode};
use crate::gui::common::datastructure::ScreenRect;
use crate::gui::common::messages::AppEvent;
use crate::gui::components::caster::caster_page;
use crate::gui::components::footer::footer;
use crate::gui::components::home::initial_page;
use crate::gui::components::hotkeys::{hotkeys, KeyTypes};
use crate::gui::components::popup::{show_popup, Popup, PopupMsg, PopupType};
use crate::gui::components::receiver::client_page;
use crate::gui::components::{home, popup};
use crate::gui::style::theme::csx::StyleType;
use crate::gui::video::Video;
use crate::gui::widget::{Column, Container, Element};
use crate::gui::windows::GuiWindow;
use crate::workers::caster::Caster;
use crate::workers::receiver::Receiver;
use iced::{window::Id, Task};
use iced_anim::{Animation, Spring, SpringEvent};
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
    pub theme: Spring<StyleType>,
    page: Page,
    popup: Popup,
    video: Video,
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
    /// Handle animated style change
    ThemeUpdate(SpringEvent<StyleType>),
    /// handle the launch of annotation window
    ShowAnnotationWindow,
}

impl MainWindow {
    pub fn new() -> Self {
        Self {
            // make as unique initializer using same as workers
            theme: Spring::new(StyleType::default()),
            page: Page::Home,
            popup: Popup::new(),
            video: Video::new(),
        }
    }
}

impl GuiWindow for MainWindow {
    type Message = MainWindowEvent;

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
                        config.mode = Some(Mode::Caster(Caster::new(config.sos.clone())));
                        self.page = Page::Caster
                    }
                    home::Message::ButtonReceiver => {
                        config.mode = Some(Mode::Client(Receiver::new(config.sos.clone())));
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
                let Some(Mode::Client(client)) = &mut config.mode else {
                    return Task::none();
                };
                if caster_ip != "auto" {
                    if !caster_ip.contains(":") {
                        caster_ip = format!("{}:{}", caster_ip, CAST_SERVICE_PORT);
                    }

                    match SocketAddr::from_str(&caster_ip) {
                        Ok(caster_socket_addr) => {
                            client.set_caster_addr(caster_socket_addr);
                        }
                        Err(e) => {
                            println!("{}", e);
                            *self.popup.get_mut(&PopupType::IP).unwrap() = PopupMsg::String("".parse().unwrap());
                            return Task::none();
                        }
                    }
                }

                if let Some(pipeline) = client.launch() {
                    self.video.set_pipeline(pipeline, FRAME_WITH, FRAME_HEIGHT, gstreamer::Fraction::new(FRAME_RATE, 1));
                }

                Task::none()
            }
            MainWindowEvent::SaveCapture => {
                let Some(Mode::Client(client)) = &mut config.mode else {
                    return Task::none();
                };
                client.save_stream();
                Task::none()
            }
            MainWindowEvent::SaveCaptureStop => {
                let Some(Mode::Client(client)) = &mut config.mode else {
                    return Task::none();
                };
                client.save_stop();
                Task::none()
            }
            MainWindowEvent::ShowAnnotationWindow => { Task::done(AppEvent::OpenAnnotationWindow) }
            MainWindowEvent::OpenWebPage(s) => { Task::done(AppEvent::OpenWebPage(s)) }
            MainWindowEvent::AreaSelection => { Task::done(AppEvent::OpenAreaSelectionWindow) }
            MainWindowEvent::AreaSelectedFullScreen => { Task::done(AppEvent::AreaSelected(ScreenRect::default())) }
            MainWindowEvent::ExitApp => { Task::done(AppEvent::ExitApp) }
            MainWindowEvent::ThemeUpdate(event) => self.theme.update(event).into(),
        }
    }

    fn view(&self, config: &Config) -> Element<MainWindowEvent> {
        let body = match self.page {
            Page::Home => {
                initial_page(&self, config)
            }
            Page::Caster => {
                caster_page(config)
            }
            Page::Client => {
                client_page(&self.video, config)
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

        Animation::new(&self.theme, content)
            .on_update(MainWindowEvent::ThemeUpdate)
            .into()
    }

    fn theme(&self) -> StyleType {
        self.theme.value().clone()
    }
}
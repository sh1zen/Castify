use crate::assets::{CAST_SERVICE_PORT, FRAME_RATE};
use crate::config::{app_name, saving_path, Config, Mode};
use crate::gui::common::datastructure::ScreenRect;
use crate::gui::common::hotkeys::{hotkeys, KeyTypes};
use crate::gui::common::messages::AppEvent;
use crate::gui::components::awmodal::{AwModalManager, GuiComponent};
use crate::gui::components::video::Video;
use crate::gui::pages::caster::caster_page;
use crate::gui::pages::footer::footer;
use crate::gui::pages::home;
use crate::gui::pages::home::initial_page;
use crate::gui::pages::info::info_page;
use crate::gui::pages::popup::PopupType;
use crate::gui::pages::receiver::client_page;
use crate::gui::popup::ip::IPModal;
use crate::gui::popup::shortcuts::ShortcutModal;
use crate::gui::popup::wrtc::WrtcModal;
use crate::gui::style::container::ContainerType;
use crate::gui::style::theme::csx::StyleType;
use crate::gui::widget::{Column, Container, Element, Space, Stack};
use crate::gui::windows::GuiWindow;
use crate::utils::net::webrtc::SDPICEExchangeWRTC;
use crate::workers::caster::Caster;
use crate::workers::receiver::Receiver;
use arboard::Clipboard;
use castbox::AnyRef;
use iced::{window::Id, Length, Task};
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Page {
    Home,
    Caster,
    Client,
    Hotkeys,
    Info,
}

#[derive(Debug, Clone)]
pub enum MainWindowEvent {
    Home,
    Mode(home::Message),
    CasterToggleStreaming,
    CasterChangeDisplay(usize),
    PopupMessage(AnyRef),
    ClosePopup(Option<Page>),
    ConnectToCaster(String),
    SaveCapture,
    SaveCaptureStop,
    HotkeysPage,
    HotkeysTypePage(KeyTypes),
    AreaSelection,
    AreaSelectedFullScreen,
    ExitApp,
    OpenWebPage(String),
    ThemeUpdate(StyleType),
    ShowAnnotationWindow,
    OpenInfo,
    Ignore,
    ShowSDP,
    CopyToClipboard(String),
    ToggleAudioMute,
}

pub struct MainWindow {
    pub theme: StyleType,
    page: Page,
    prev_page: Page,
    popup: AwModalManager<PopupType>,
    video: Video,
}

impl MainWindow {
    pub fn new() -> Self {
        Self {
            theme: StyleType::default(),
            page: Page::Home,
            popup: AwModalManager::new(),
            video: Video::new(),
            prev_page: Page::Home,
        }
    }

    pub fn change_page(&mut self, page: Page) {
        self.prev_page = self.page;
        self.page = page;
    }

    /// Collega il canale video dal Receiver al componente Video per il rendering.
    fn attach_video_stream(&mut self, receiver: &mut Receiver) {
        if let Some(rx) = receiver.launch(true) {
            self.video.set_stream(rx, FRAME_RATE);
        }
    }

    fn attach_video_stream_manual(&mut self, receiver: &mut Receiver) {
        if let Some(rx) = receiver.launch(false) {
            self.video.set_stream(rx, FRAME_RATE);
        }
    }

    fn popup_update(&mut self, value: AnyRef, config: &mut Config) {
        if let Some(popup) = self.popup.get_mut_ref() {
            popup.as_mut_gui().update(value, config);
        }
    }

    fn receiver_mut(config: &mut Config) -> Option<&mut Receiver> {
        match &mut config.mode {
            Some(Mode::Receiver(receiver)) => Some(receiver),
            _ => None,
        }
    }

    fn caster_mut(config: &mut Config) -> Option<&mut Caster> {
        match &mut config.mode {
            Some(Mode::Caster(caster)) => Some(caster),
            _ => None,
        }
    }

    fn parse_caster_addr(caster_ip: &str) -> Result<SocketAddr, ()> {
        let caster_ip = if caster_ip.contains(':') {
            caster_ip.to_owned()
        } else {
            format!("{}:{}", caster_ip, CAST_SERVICE_PORT)
        };
        SocketAddr::from_str(&caster_ip).map_err(|_| ())
    }

    fn active_sdp_provider(
        &mut self,
        config: &mut Config,
    ) -> Option<(bool, Arc<dyn SDPICEExchangeWRTC>)> {
        match &mut config.mode {
            Some(Mode::Caster(caster)) => Some((
                true,
                caster.get_connection_handler() as Arc<dyn SDPICEExchangeWRTC>,
            )),
            Some(Mode::Receiver(receiver)) => {
                self.attach_video_stream_manual(receiver);
                self.page = Page::Client;
                Some((
                    false,
                    receiver.get_connection_handler() as Arc<dyn SDPICEExchangeWRTC>,
                ))
            }
            _ => None,
        }
    }
}

impl GuiWindow for MainWindow {
    type Message = MainWindowEvent;

    fn title(&self) -> String {
        app_name()
    }

    fn update(&mut self, _id: Id, message: MainWindowEvent, config: &mut Config) -> Task<AppEvent> {
        match message {
            MainWindowEvent::Home => {
                config.shortcuts.updating = KeyTypes::None;
                config.reset_mode();
                self.popup.hide();
                self.change_page(Page::Home);
                Task::none()
            }
            MainWindowEvent::Mode(mode) => {
                match mode {
                    home::Message::ButtonCaster => {
                        config.mode =
                            Some(Mode::Caster(Caster::new(config.fps, config.sos.clone())));
                        self.change_page(Page::Caster);
                    }
                    home::Message::ButtonReceiver => {
                        config.mode = Some(Mode::Receiver(Receiver::new(config.sos.clone())));
                        self.popup.set(PopupType::IP(IPModal::new()));
                        self.popup.show();
                    }
                }
                Task::none()
            }
            MainWindowEvent::ShowSDP => {
                if let Some((is_caster, sdp)) = self.active_sdp_provider(config) {
                    self.popup
                        .set(PopupType::ManualWRTC(WrtcModal::new(is_caster)));
                    self.popup.show();

                    Task::future(async move {
                        let remote_sdp = sdp.get_sdp().await;
                        if !remote_sdp.starts_with("Wrong") {
                            sdp.set_remote_sdp(remote_sdp).await;
                        }

                        sleep(Duration::from_millis(1500)).await;
                        AppEvent::Ignore
                    })
                } else {
                    Task::none()
                }
            }
            MainWindowEvent::CasterToggleStreaming => Task::done(AppEvent::CasterToggleStreaming),
            MainWindowEvent::CasterChangeDisplay(idx) => {
                if let Some(caster) = Self::caster_mut(config) {
                    let displays = caster.get_displays();
                    if let Some(display) = displays.into_iter().nth(idx) {
                        caster.change_display(display);
                    }
                }
                Task::none()
            }
            MainWindowEvent::PopupMessage(value) => {
                self.popup_update(value, config);
                Task::none()
            }
            MainWindowEvent::ClosePopup(page) => {
                self.popup.hide();
                if let Some(p) = page {
                    self.page = p;
                }
                Task::none()
            }
            MainWindowEvent::HotkeysPage => {
                self.change_page(Page::Hotkeys);
                Task::none()
            }
            MainWindowEvent::HotkeysTypePage(key) => {
                config.shortcuts.updating = key;
                self.popup
                    .set(PopupType::HotkeyUpdate(ShortcutModal::new().set_key(key)));
                self.popup.show();
                Task::none()
            }
            MainWindowEvent::ConnectToCaster(caster_ip) => {
                self.popup.hide();
                self.change_page(Page::Client);

                if caster_ip != "auto" {
                    match Self::parse_caster_addr(&caster_ip) {
                        Ok(caster_socket_addr) => {
                            let Some(client) = Self::receiver_mut(config) else {
                                return Task::none();
                            };
                            client.set_caster_addr(caster_socket_addr)
                        }
                        Err(_) => {
                            self.popup_update(AnyRef::new(String::new()), config);
                            return Task::none();
                        }
                    }
                }

                let Some(client) = Self::receiver_mut(config) else {
                    return Task::none();
                };
                self.attach_video_stream(client);
                Task::none()
            }
            MainWindowEvent::SaveCapture => {
                let Some(client) = Self::receiver_mut(config) else {
                    return Task::none();
                };
                let saving_path = saving_path();
                client.save_stream(saving_path);
                Task::none()
            }
            MainWindowEvent::SaveCaptureStop => {
                let Some(client) = Self::receiver_mut(config) else {
                    return Task::none();
                };
                client.save_stop();
                Task::none()
            }
            MainWindowEvent::OpenInfo => {
                if self.page == Page::Info {
                    self.page = self.prev_page;
                } else {
                    self.change_page(Page::Info);
                }
                Task::none()
            }
            MainWindowEvent::ShowAnnotationWindow => Task::done(AppEvent::OpenAnnotationWindow),
            MainWindowEvent::OpenWebPage(s) => Task::done(AppEvent::OpenWebPage(s)),
            MainWindowEvent::AreaSelection => Task::done(AppEvent::OpenAreaSelectionWindow),
            MainWindowEvent::AreaSelectedFullScreen => {
                Task::done(AppEvent::AreaSelected(ScreenRect::default()))
            }
            MainWindowEvent::ExitApp => Task::done(AppEvent::ExitApp),
            MainWindowEvent::ThemeUpdate(theme) => {
                self.theme = theme;
                Task::none()
            }
            MainWindowEvent::Ignore => Task::none(),
            MainWindowEvent::CopyToClipboard(text) => {
                if let Ok(mut clipboard) = Clipboard::new() {
                    let _ = clipboard.set_text(text);
                }
                Task::none()
            }
            MainWindowEvent::ToggleAudioMute => Task::done(AppEvent::ToggleAudioMute),
        }
    }

    fn view(&self, config: &Config) -> Element<'_, MainWindowEvent> {
        let body = match self.page {
            Page::Home => initial_page(self, config),
            Page::Caster => caster_page(config),
            Page::Client => client_page(&self.video, config),
            Page::Hotkeys => hotkeys(),
            Page::Info => info_page(),
        };

        let mut content = Column::new().push(body).push(footer());

        if self.popup.is_visible() {
            let darkened_background = Container::new(Space::new())
                .width(Length::Fill)
                .height(Length::Fill)
                .class(ContainerType::DarkFilter);

            content = Column::new().push(Container::new(
                Stack::new()
                    .push(darkened_background)
                    .push(self.popup.render(config)),
            ));
        }

        content.into()
    }

    fn theme(&self) -> StyleType {
        self.theme.clone()
    }
}

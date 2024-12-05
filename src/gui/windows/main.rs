use crate::assets::{CAST_SERVICE_PORT, FRAME_HEIGHT, FRAME_RATE, FRAME_WITH};
use crate::config::{app_name, saving_path, Config, Mode};
use crate::gui::common::anybox::AnyBox;
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
use iced::{window::Id, Length, Task};
use iced_anim::{Animated, Animation};
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
    /// the app mode caster / receiver
    Mode(home::Message),
    /// caster play pause
    CasterToggleStreaming,
    /// Set Caster monitor
    CasterMonitor(u32),
    /// handle popup messages
    PopupMessage(AnyBox),
    /// close any popup
    ClosePopup(Option<Page>),
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
    ThemeUpdate(iced_anim::event::Event<StyleType>),
    /// handle the launch of annotation window
    ShowAnnotationWindow,
    /// program info
    OpenInfo,
    /// Ignore the event
    Ignore,
    /// Show WebRTC SDP
    ShowSDP,
    /// Copy Text To Clipboard
    CopyToClipboard(String),
}

pub struct MainWindow {
    pub theme: Animated<StyleType>,
    page: Page,
    prev_page: Page,
    popup: AwModalManager<PopupType>,
    video: Video,
}

impl MainWindow {
    pub fn new() -> Self {
        Self {
            theme: Animated::new(StyleType::default(), iced_anim::animated::Mode::Spring(iced_anim::spring::Motion::SMOOTH)),
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
                        config.mode = Some(Mode::Caster(Caster::new(config.sos.clone())));
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
                let mut is_caster = false;
                if let Some(sdp_provider) = match &mut config.mode {
                    Some(Mode::Caster(caster)) => {
                        is_caster = true;
                        Some(caster.get_connection_handler() as Arc<dyn SDPICEExchangeWRTC>)
                    }
                    Some(Mode::Receiver(receiver)) => {
                        if let Some(pipeline) = receiver.launch(false) {
                            self.video.set_pipeline(pipeline, FRAME_WITH, FRAME_HEIGHT, gstreamer::Fraction::new(FRAME_RATE, 1));
                        }
                        self.page = Page::Client;
                        Some(receiver.get_connection_handler() as Arc<dyn SDPICEExchangeWRTC>)
                    }
                    _ => None,
                } {
                    self.popup.set(PopupType::ManualWRTC(WrtcModal::new(is_caster)));
                    self.popup.show();

                    let mut popup = self.popup.clone();

                    Task::future(async move {
                        let binding = popup.get_mut_ref().unwrap();
                        let wrtc_popup = binding.as_mut_any().downcast_mut::<WrtcModal>().unwrap();

                        wrtc_popup.set_sdp_provider(sdp_provider).await;
                        wrtc_popup.handle_sdp_negotiation(true).await;
                        wrtc_popup.handle_sdp_negotiation(false).await;

                        sleep(Duration::from_millis(1500)).await;

                        popup.remove();
                        AppEvent::Ignore
                    })
                } else {
                    Task::none()
                }
            }
            MainWindowEvent::CasterToggleStreaming => { Task::done(AppEvent::CasterToggleStreaming) }
            MainWindowEvent::CasterMonitor(mon) => {
                if let Some(Mode::Caster(caster)) = &mut config.mode {
                    caster.change_monitor(mon);
                }
                Task::none()
            }
            MainWindowEvent::PopupMessage(value) => {
                self.popup.get_mut_ref().unwrap().as_mut_gui().update(value, config);
                Task::none()
            }
            MainWindowEvent::ClosePopup(page) => {
                self.popup.hide();
                if page.is_some() {
                    self.page = page.unwrap();
                }
                Task::none()
            }
            MainWindowEvent::HotkeysPage => {
                self.change_page(Page::Hotkeys);
                Task::none()
            }
            MainWindowEvent::HotkeysTypePage(key) => {
                config.shortcuts.updating = key;
                self.popup.set(PopupType::HotkeyUpdate(ShortcutModal::new().set_key(key)));
                self.popup.show();
                Task::none()
            }
            MainWindowEvent::ConnectToCaster(mut caster_ip) => {
                self.popup.hide();
                self.change_page(Page::Client);
                let Some(Mode::Receiver(client)) = &mut config.mode else {
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
                        Err(_) => {
                            self.popup.get_mut_ref().unwrap().as_mut_gui().update(AnyBox::new(String::from("")), config);
                            return Task::none();
                        }
                    }
                }

                if let Some(pipeline) = client.launch(true) {
                    self.video.set_pipeline(pipeline, FRAME_WITH, FRAME_HEIGHT, gstreamer::Fraction::new(FRAME_RATE, 1));
                }

                Task::none()
            }
            MainWindowEvent::SaveCapture => {
                let Some(Mode::Receiver(client)) = &mut config.mode else {
                    return Task::none();
                };
                let saving_path = saving_path();
                client.save_stream(saving_path);
                Task::none()
            }
            MainWindowEvent::SaveCaptureStop => {
                let Some(Mode::Receiver(client)) = &mut config.mode else {
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
            MainWindowEvent::ShowAnnotationWindow => { Task::done(AppEvent::OpenAnnotationWindow) }
            MainWindowEvent::OpenWebPage(s) => { Task::done(AppEvent::OpenWebPage(s)) }
            MainWindowEvent::AreaSelection => { Task::done(AppEvent::OpenAreaSelectionWindow) }
            MainWindowEvent::AreaSelectedFullScreen => { Task::done(AppEvent::AreaSelected(ScreenRect::default())) }
            MainWindowEvent::ExitApp => { Task::done(AppEvent::ExitApp) }
            MainWindowEvent::ThemeUpdate(event) => self.theme.update(event).into(),
            MainWindowEvent::Ignore => Task::none(),
            MainWindowEvent::CopyToClipboard(text) => {
                if let Ok(mut clipboard) = Clipboard::new() {
                    let _ = clipboard.set_text(text);
                }
                Task::none()
            }
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
            Page::Info => {
                info_page()
            }
        };

        let mut content = Column::new()
            .push(body)
            .push(footer());

        if self.popup.is_visible() {
            let darkened_background = Container::new(Space::new(0, 0))
                .width(Length::Fill)
                .height(Length::Fill)
                .class(ContainerType::DarkFilter);

            content = Column::new()
                .push(
                    Container::new(
                        Stack::new()
                            //.push(content)
                            .push(darkened_background)
                            .push(self.popup.render(config))
                    )
                );
        }

        Animation::new(&self.theme, content)
            .on_update(MainWindowEvent::ThemeUpdate)
            .into()
    }

    fn theme(&self) -> StyleType {
        self.theme.value().clone()
    }
}
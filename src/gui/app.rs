use crate::assets::ICON_BYTES;
use crate::config::Config;
use crate::gui::common::messages::AppEvent;
use crate::gui::components::hotkeys::KeyTypes;
use crate::gui::style::theme::csx::StyleType;
use crate::gui::widget::Element;
use crate::gui::widget::IcedRenderer;
use crate::utils::key_listener::global_key_listener;
use crate::utils::open_link;
use crate::utils::tray_icon::{tray_icon_listener, tray_menu_listener};
use crate::windows::area_selector::ASWindow;
use crate::windows::main::MainWindow;
use crate::windows::{GuiWindow, WindowManager};
use crate::workers;
use iced::application::Appearance;
use iced::widget::horizontal_space;
use iced::Event::Window;
use iced::{window, Subscription};
use iced_core::keyboard::{Event, Key};
use iced_core::window::settings::PlatformSpecific;
use iced_core::window::{Id, Mode, Position};
use iced_core::Event::Keyboard;
use iced_core::Size;
use iced_runtime::window::{change_mode, gain_focus};
use iced_runtime::Task;
use std::collections::BTreeMap;
use std::time::Duration;

pub struct App {
    pub config: Config,
    dark_mode: bool,
    windows: BTreeMap<Id, WindowManager>,
    main_window: Option<Id>,
}

impl App {
    pub fn new() -> (Self, Task<AppEvent>) {
        (
            Self {
                config: Config::default(),
                dark_mode: false,
                windows: Default::default(),
                main_window: None,
            },
            Task::done(AppEvent::ShowMainWindow),
        )
    }

    pub(crate) fn update(&mut self, message: AppEvent) -> Task<AppEvent> {
        match message {
            AppEvent::ShowMainWindow => {
                if self.main_window.is_none() {
                    let (id, open_task) = window::open(window::Settings {
                        size: self.config.window_size,
                        position: Position::Centered,
                        min_size: Some(Size::new(400f32, 300f32)),
                        max_size: None,
                        visible: true,
                        resizable: true,
                        decorations: true,
                        transparent: false,
                        icon: Some(
                            window::icon::from_file_data(
                                ICON_BYTES,
                                None,
                            ).unwrap(),
                        ),
                        #[cfg(target_os = "macos")]
                        platform_specific: PlatformSpecific {
                            title_hidden: true,
                            titlebar_transparent: true,
                            fullsize_content_view: true,
                        },
                        #[cfg(target_os = "linux")]
                        platform_specific: PlatformSpecific {
                            application_id: String::from(APP_NAME_ID),
                            override_redirect: true,
                        },
                        exit_on_close_request: false,
                        ..Default::default()
                    });
                    self.main_window = Some(id);
                    self.windows.insert(id, WindowManager::Main(MainWindow::new()));
                    open_task.discard().chain(gain_focus(id))
                } else {
                    gain_focus(self.main_window.unwrap())
                }
            }
            AppEvent::AreaSelection => {
                if self.windows.len() <= 1 {
                    let (id, open_task) = window::open(window::Settings {
                        transparent: true,
                        decorations: false,
                        resizable: false,
                        #[cfg(target_os = "windows")]
                        platform_specific: PlatformSpecific {
                            drag_and_drop: false,
                            skip_taskbar: true,
                            undecorated_shadow: false,
                        },
                        ..Default::default()
                    });
                    self.windows.insert(id, WindowManager::AreaSelector(ASWindow::new()));
                    open_task
                        .discard()
                        .chain(gain_focus(id))
                        .chain(change_mode(id, Mode::Fullscreen))
                } else {
                    Task::none()
                }
            }
            AppEvent::AreaSelected(rect) => {
                // set the new area selected
                if let Some(crate::config::Mode::Caster(caster)) = &mut self.config.mode {
                    caster.resize_rec_area(rect);
                }
                Task::none()
            }
            AppEvent::CloseWindow(id) => {
                if self.main_window == Some(id) {
                    self.main_window = None;
                }
                self.windows.remove(&id);
                window::close(id)
            }
            AppEvent::WindowEvent(id, message) => {
                match self.windows.get_mut(&id) {
                    Some(window_handler) => window_handler.update(id, message, &mut self.config),
                    None => Task::none(),
                }
            }
            AppEvent::TimeTick => {
                if let Some(crate::config::Mode::Caster(caster)) = &mut self.config.mode {
                    if caster.is_streaming() {
                        caster.streaming_time += 1;
                    }
                }
                self.config.e_time = self.config.e_time + 1;
                Task::none()
            }
            AppEvent::WindowResized(id, width, height) => {
                if Some(id) == self.main_window {
                    self.config.window_size = Size { width: width as f32, height: height as f32 };
                }
                Task::none()
            }
            AppEvent::OpenWebPage(web_page) => {
                open_link(&web_page);
                Task::none()
            }
            AppEvent::BlankScreen => {
                if let Some(crate::config::Mode::Caster(caster)) = &mut self.config.mode {
                    caster.toggle_blank_screen();
                }
                Task::none()
            }
            AppEvent::CasterToggleStreaming => {
                if let Some(crate::config::Mode::Caster(caster)) = &mut self.config.mode {
                    caster.toggle_streaming();
                }
                Task::none()
            }
            AppEvent::KeyPressed(modifier, key) => {
                if key == Key::Unidentified {
                    return Task::none();
                }

                let item = (modifier, key);

                if self.config.hotkey_map.updating != KeyTypes::None {
                    match self.config.hotkey_map.updating {
                        KeyTypes::Pause => {
                            self.config.hotkey_map.pause = item
                        }
                        KeyTypes::Record => {
                            self.config.hotkey_map.record = item
                        }
                        KeyTypes::BlankScreen => {
                            self.config.hotkey_map.blank_screen = item
                        }
                        KeyTypes::Close => {
                            self.config.hotkey_map.end_session = item
                        }
                        _ => {}
                    }
                    Task::none()
                } else {
                    if item == self.config.hotkey_map.pause || item == self.config.hotkey_map.record {
                        Task::done(AppEvent::CasterToggleStreaming)
                    } else if item == self.config.hotkey_map.blank_screen {
                        Task::done(AppEvent::BlankScreen)
                    } else if item == self.config.hotkey_map.end_session {
                        Task::done(AppEvent::ExitApp)
                    } else { Task::none() }
                }
            }
            AppEvent::ExitApp => {
                workers::sos::get_instance().lock().unwrap().terminate();
                iced::exit()
            }
            AppEvent::Ignore => {
                Task::none()
            }
            _ => {
                println!("Command not yet implemented!");
                Task::none()
            }
        }
    }

    pub(crate) fn view(&self, id: Id) -> Element<AppEvent, StyleType, IcedRenderer> {
        match self.windows.get(&id) {
            Some(window_handler) => window_handler.view(&self.config).map(move |message| AppEvent::WindowEvent(id, message)),
            None => horizontal_space().into(),
        }
    }

    pub fn title(&self, id: Id) -> String {
        match self.windows.get(&id) {
            Some(window_handler) => window_handler.title(),
            None => String::new(),
        }
    }

    pub fn theme(&self, id: Id) -> StyleType {
        match self.windows.get(&id) {
            Some(window_handler) => window_handler.theme(),
            None => StyleType::default(),
        }
    }

    pub fn style(&self, theme: &StyleType) -> Appearance {
        Appearance {
            background_color: theme.get_palette().background,
            text_color: theme.get_palette().text,
        }
    }

    pub fn subscription(&self) -> Subscription<AppEvent> {
        let tray_menu_listener = Subscription::run(tray_menu_listener);
        let tray_icon_listener = Subscription::run(tray_icon_listener);

        let time_listener = iced::time::every(Duration::from_secs(1)).map(|_| AppEvent::TimeTick);

        let global_key_listener = Subscription::run(global_key_listener);

        Subscription::batch([
            time_listener,
            tray_menu_listener,
            tray_icon_listener,
            global_key_listener,
            self.keyboard_subscription(),
            self.window_subscription()
        ])
    }

    fn keyboard_subscription(&self) -> Subscription<AppEvent> {
        iced::event::listen_with(|event, _status, _id| match event {
            Keyboard(Event::KeyPressed { key, modifiers, .. }) => {
                Some(AppEvent::KeyPressed(modifiers, key))
            }
            _ => None,
        })
    }

    fn window_subscription(&self) -> Subscription<AppEvent> {
        iced::event::listen_with(|event, _status, id| match event {
            Window(window::Event::CloseRequested) => {
                Some(AppEvent::CloseWindow(id))
            }
            Window(window::Event::Resized(size)) => {
                Some(AppEvent::WindowResized(id, size.width as u32, size.height as u32))
            }
            _ => None,
        })
    }
}
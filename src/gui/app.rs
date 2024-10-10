use crate::assets::ICON_BYTES;
use crate::config::Config;
use crate::gui::common::messages::AppEvent;
use crate::gui::components::hotkeys::KeyTypes;
use crate::gui::style::theme::csx::StyleType;
use crate::gui::widget::Element;
use crate::gui::windows::{GuiWindow, WindowType, Windows};
use crate::utils::key_listener::global_key_listener;
use crate::utils::open_link;
use crate::utils::tray_icon::{tray_icon, tray_icon_listener, tray_menu_listener};
use iced::application::Appearance;
use iced::widget::horizontal_space;
use iced::Event::Window;
use iced::{window, Subscription};
use iced_core::keyboard::{Event, Key};
use iced_core::window::settings::PlatformSpecific;
use iced_core::window::{Id, Mode, Position};
use iced_core::Event::Keyboard;
use iced_core::Size;
use iced_runtime::Task;
use std::process::exit;
use std::time::Duration;
use tray_icon::TrayIcon;

pub struct App {
    pub config: Config,
    dark_mode: bool,
    windows: Windows,
    tray_icon: TrayIcon,
}

impl App {
    pub fn new() -> (Self, Task<AppEvent>) {
        let tray_icon = tray_icon();
        (
            Self {
                config: Config::new(),
                dark_mode: false,
                windows: Windows::new(),
                tray_icon,
            },
            Task::done(AppEvent::OpenMainWindow),
        )
    }

    pub fn update(&mut self, message: AppEvent) -> Task<AppEvent> {
        match message {
            AppEvent::OpenMainWindow => {
                let main_window = self.windows.get_id(WindowType::Main);
                if main_window.is_none() {
                    let (id, open_task) = window::open(window::Settings {
                        size: self.config.window_size,
                        position: Position::Centered,
                        min_size: Some(Size { width: 680f32, height: 460f32 }),
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
                    self.windows.insert(id, WindowType::Main);
                    open_task.discard().chain(window::gain_focus(id))
                } else {
                    window::gain_focus(main_window.unwrap())
                }
            }
            AppEvent::OpenAreaSelectionWindow => {
                if !self.windows.contains(WindowType::AreaSelector) {
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
                    self.windows.insert(id, WindowType::AreaSelector);
                    open_task
                        .discard()
                        .chain(window::gain_focus(id))
                        .chain(window::change_mode(id, Mode::Fullscreen))
                } else {
                    Task::none()
                }
            }
            AppEvent::OpenAnnotationWindow => {
                if !self.windows.contains(WindowType::Annotation) {
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
                    self.windows.insert(id, WindowType::Annotation);
                    open_task
                        .discard()
                        .chain(window::gain_focus(id))
                        .chain(window::change_mode(id, Mode::Fullscreen))
                    //.chain(window::enable_mouse_passthrough(id))
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
                self.windows.remove(id, self.windows.of_type(id, WindowType::Main));
                window::close(id)
            }
            AppEvent::WindowEvent(id, message) => {
                match self.windows.get_manager_mut(id) {
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
                if self.windows.of_type(id, WindowType::Main) {
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
                for (id, _) in self.windows.iter() {
                    let _: Task<AppEvent> = window::close(*id);
                }
                self.config.reset_mode();
                self.config.sos.cancel();
                exit(0)
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

    pub fn view(&self, id: Id) -> Element<AppEvent> {
        match self.windows.get_manager(id) {
            Some(window_handler) => window_handler.view(&self.config).map(move |message| AppEvent::WindowEvent(id, message)),
            None => horizontal_space().into(),
        }
    }

    pub fn title(&self, id: Id) -> String {
        match self.windows.get_manager(id) {
            Some(window_handler) => window_handler.title(),
            None => String::new(),
        }
    }

    pub fn theme(&self, id: Id) -> StyleType {
        match self.windows.get_manager(id) {
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
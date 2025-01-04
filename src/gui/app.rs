#[cfg(target_os = "linux")]
use crate::app_id;
use crate::assets::ICON_BYTES;
use crate::config::Config;
use crate::gui::common::hotkeys::KeyTypes;
use crate::gui::common::messages::AppEvent;
use crate::gui::style::theme::csx::StyleType;
use crate::gui::widget::Element;
use crate::gui::windows::{WindowType, Windows};
use crate::utils::flags::Flags;
use crate::utils::ipc::ipc;
use crate::utils::open_link;
use crate::workers::key_listener::{global_key_listener, valid_iced_key};
use crate::workers::tray_icon::{tray_icon, tray_icon_listener, tray_menu_listener};
use iced::keyboard::{Event, Key, Modifiers};
use iced::widget::horizontal_space;
use iced::{
    application::Appearance,
    window,
    window::{
        settings::PlatformSpecific,
        Id, Mode, Position,
    },
    Event::{Keyboard, Window}, Point, Size, Subscription, Task,
};
use std::process::exit;
use std::time::Duration;
use tray_icon::TrayIcon;

#[allow(dead_code)]
pub struct App {
    pub config: Config,
    windows: Windows,
    tray_icon: TrayIcon,
}

impl App {
    pub fn new(flags: Flags) -> (Self, Task<AppEvent>) {
        let tray_icon = tray_icon();
        (
            Self {
                config: Config::new(flags),
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
                        transparent: true,
                        icon: Some(
                            window::icon::from_file_data(
                                ICON_BYTES,
                                None,
                            ).unwrap(),
                        ),
                        #[cfg(target_os = "macos")]
                        platform_specific: PlatformSpecific {
                            title_hidden: false,
                            titlebar_transparent: false,
                            fullsize_content_view: true,
                        },
                        #[cfg(target_os = "linux")]
                        platform_specific: PlatformSpecific {
                            application_id: String::from(app_id()),
                            override_redirect: false,
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
                let Some(crate::config::Mode::Caster(caster)) = &mut self.config.mode else {
                    unreachable!("Mode must be Caster here")
                };
                if !self.windows.contains(WindowType::AreaSelector) {
                    let monitor = caster.get_monitor();

                    let (id, open_task) = window::open(window::Settings {
                        size: monitor.map_or(Size { width: 1920.0, height: 1080.0 }, |mon| Size {
                            width: mon.width as f32,
                            height: mon.height as f32,
                        }),
                        position: Position::Specific(monitor.map_or(Point::default(), |mon| Point {
                            x: mon.x as f32,
                            y: mon.y as f32,
                        })),
                        transparent: true,
                        decorations: false,
                        resizable: false,
                        #[cfg(target_os = "windows")]
                        platform_specific: PlatformSpecific {
                            drag_and_drop: false,
                            skip_taskbar: true,
                            undecorated_shadow: false,
                        },
                        #[cfg(target_os = "linux")]
                        platform_specific: PlatformSpecific {
                            application_id: String::from(app_id()),
                            override_redirect: true,
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
                let Some(crate::config::Mode::Caster(caster)) = &mut self.config.mode else {
                    unreachable!("Mode must be Caster here")
                };

                if !self.windows.contains(WindowType::Annotation) {
                    let monitor = caster.get_monitor();

                    let (id, open_task) = window::open(window::Settings {
                        size: monitor.map_or(Size { width: 1920.0, height: 1080.0 }, |mon| Size {
                            width: mon.width as f32,
                            height: mon.height as f32,
                        }),
                        position: Position::Specific(monitor.map_or(Point::default(), |mon| Point {
                            x: mon.x as f32,
                            y: mon.y as f32,
                        })),
                        transparent: true,
                        decorations: false,
                        resizable: false,
                        #[cfg(target_os = "windows")]
                        platform_specific: PlatformSpecific {
                            drag_and_drop: false,
                            skip_taskbar: true,
                            undecorated_shadow: false,
                        },
                        #[cfg(target_os = "linux")]
                        platform_specific: PlatformSpecific {
                            application_id: String::from(app_id()),
                            override_redirect: true,
                        },
                        ..Default::default()
                    });
                    self.windows.insert(id, WindowType::Annotation);
                    open_task
                        .discard()
                        .chain(window::gain_focus(id))
                        .chain(window::change_mode(id, Mode::Fullscreen))
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
            AppEvent::KeyEvent(modifier, key) => {
                if key == Key::Unidentified {
                    return Task::none();
                }
                //println!("KeyEvent {:?} {:?}", modifier, key);
                let item = (modifier, key);

                if self.config.shortcuts.updating != KeyTypes::None {
                    match self.config.shortcuts.updating {
                        KeyTypes::Pause => {
                            self.config.shortcuts.pause = item
                        }
                        KeyTypes::Record => {
                            self.config.shortcuts.record = item
                        }
                        KeyTypes::BlankScreen => {
                            self.config.shortcuts.blank_screen = item
                        }
                        KeyTypes::Close => {
                            self.config.shortcuts.end_session = item
                        }
                        _ => {}
                    }
                    Task::none()
                } else {
                    if item == self.config.shortcuts.pause || item == self.config.shortcuts.record {
                        Task::done(AppEvent::CasterToggleStreaming)
                    } else if item == self.config.shortcuts.blank_screen {
                        Task::done(AppEvent::BlankScreen)
                    } else if item == self.config.shortcuts.end_session {
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
                //println!("Command not yet implemented!");
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
        let mut batch = Vec::new();

        batch.push(Subscription::run(tray_menu_listener));
        batch.push(Subscription::run(tray_icon_listener));
        batch.push(iced::time::every(Duration::from_secs(1)).map(|_| AppEvent::TimeTick));
        if self.config.fps.is_some() {
            batch.push(iced::time::every(Duration::from_millis(self.config.fps.unwrap())).map(|_| AppEvent::TimeTickFPS));
        }
        batch.push(Subscription::run(ipc));
        batch.push(self.keyboard_subscription());
        batch.push(self.window_subscription());

        if !self.config.multi_instance {
            batch.push(Subscription::run(global_key_listener));
        }

        Subscription::batch(batch)
    }

    fn keyboard_subscription(&self) -> Subscription<AppEvent> {
        iced::event::listen_with(|event, _status, _id| match event {
            Keyboard(Event::KeyReleased { key, modifiers, .. }) => {
                if modifiers == Modifiers::empty() && !valid_iced_key(key.clone()) {
                    None
                } else {
                    Some(AppEvent::KeyEvent(modifiers, key))
                }
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


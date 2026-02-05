#[cfg(target_os = "linux")]
use crate::app_id;
use crate::assets::ICON_BYTES;
use crate::config::Config;
use crate::gui::common::hotkeys::KeyTypes;
use crate::gui::common::messages::AppEvent;
use crate::gui::style::theme::csx::StyleType;
use crate::gui::widget::Element;
use crate::gui::widget::horizontal_space;
use crate::gui::windows::{WindowType, Windows};
use crate::utils::flags::Flags;
use crate::utils::ipc::ipc;
use crate::utils::open_link;
use crate::workers::key_listener::{global_key_listener, valid_iced_key};
use crate::workers::tray_icon::{tray_icon, tray_icon_listener, tray_menu_listener};
use iced::keyboard::{Event, Key, Modifiers};
use iced::{
    Event::{Keyboard, Window},
    Point, Size, Subscription, Task,
    theme::Style,
    window,
    window::{Id, Mode, Position, settings::PlatformSpecific},
};
use std::process::exit;
use std::time::Duration;
use tray_icon::TrayIcon;

/// Apply DWM transparency to a window by its raw HWND (Windows only).
/// Extends the glass frame into the entire client area for per-pixel alpha compositing.
/// Requires wgpu to use PostMultiplied/PreMultiplied alpha mode (iced_wgpu 0.14 selects this automatically).
/// If transparency still doesn't work, try setting WGPU_BACKEND=vulkan.
#[cfg(target_os = "windows")]
fn apply_dwm_transparency(raw_id: u64) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Dwm::DwmExtendFrameIntoClientArea;
    use windows::Win32::UI::Controls::MARGINS;

    let hwnd = HWND(raw_id as *mut std::ffi::c_void);
    let margins = MARGINS {
        cxLeftWidth: -1,
        cxRightWidth: -1,
        cyTopHeight: -1,
        cyBottomHeight: -1,
    };
    unsafe {
        match DwmExtendFrameIntoClientArea(hwnd, &margins) {
            Ok(()) => log::debug!("DWM transparency applied to window {:?}", hwnd),
            Err(e) => log::warn!("Failed to apply DWM transparency: {}", e),
        }
    }
}

pub struct App {
    pub config: Config,
    windows: Windows,
    #[allow(dead_code)] // Keep tray_icon alive for system tray functionality
    tray_icon: Option<TrayIcon>,
}

impl App {
    pub fn new(flags: Flags) -> (Self, Task<AppEvent>) {
        let tray_icon = tray_icon().ok();
        (
            Self {
                config: Config::new(flags),
                windows: Windows::new(),
                tray_icon,
            },
            Task::done(AppEvent::OpenMainWindow),
        )
    }

    /// Apply DWM transparency to a window after it's been created (Windows only).
    #[cfg(target_os = "windows")]
    fn apply_transparency(id: Id) -> Task<AppEvent> {
        window::raw_id::<AppEvent>(id).map(|raw_id| {
            apply_dwm_transparency(raw_id);
            AppEvent::Ignore
        })
    }

    /// Helper: returns (width, height, x, y) of the selected display in physical pixels.
    fn caster_display_size(caster: &crate::workers::caster::Caster) -> (f32, f32, f32, f32) {
        if let Some(display) = caster.get_selected_display() {
            #[cfg(target_os = "windows")]
            {
                display.rect()
            }
            #[cfg(not(target_os = "windows"))]
            {
                let _ = display;
                (1920.0, 1080.0, 0.0, 0.0)
            }
        } else {
            (1920.0, 1080.0, 0.0, 0.0)
        }
    }

    /// Helper: returns the DPI scale factor for the selected display.
    fn caster_dpi_scale(caster: &crate::workers::caster::Caster) -> f32 {
        #[cfg(target_os = "windows")]
        {
            caster
                .get_selected_display()
                .map(|d| d.dpi_scale() as f32)
                .unwrap_or(1.0)
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = caster;
            1.0
        }
    }

    pub fn update(&mut self, message: AppEvent) -> Task<AppEvent> {
        match message {
            AppEvent::OpenMainWindow => {
                let main_window = self.windows.get_id(WindowType::Main);
                if let Some(id) = main_window {
                    window::gain_focus(id)
                } else {
                    let (id, open_task) = window::open(window::Settings {
                        size: self.config.window_size,
                        position: Position::Centered,
                        min_size: Some(Size {
                            width: 680f32,
                            height: 460f32,
                        }),
                        max_size: None,
                        visible: true,
                        resizable: true,
                        decorations: true,
                        transparent: true,
                        icon: Some(window::icon::from_file_data(ICON_BYTES, None).unwrap()),
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
                }
            }
            AppEvent::OpenAreaSelectionWindow => {
                let Some(crate::config::Mode::Caster(caster)) = &mut self.config.mode else {
                    unreachable!("Mode must be Caster here")
                };
                if !self.windows.contains(WindowType::AreaSelector) {
                    let (w, h, x, y) = Self::caster_display_size(caster);
                    let dpi = Self::caster_dpi_scale(caster);

                    let (id, open_task) = window::open(window::Settings {
                        size: Size {
                            width: (w / dpi) - 1.0,
                            height: (h / dpi) - 1.0,
                        },
                        position: Position::Specific(Point {
                            x: x / dpi,
                            y: y / dpi,
                        }),
                        transparent: true,
                        decorations: false,
                        resizable: false,
                        #[cfg(target_os = "windows")]
                        platform_specific: PlatformSpecific {
                            drag_and_drop: false,
                            skip_taskbar: true,
                            undecorated_shadow: false,
                            corner_preference: Default::default(),
                        },
                        #[cfg(target_os = "macos")]
                        platform_specific: PlatformSpecific {
                            title_hidden: true,
                            titlebar_transparent: true,
                            fullsize_content_view: true,
                        },
                        #[cfg(target_os = "linux")]
                        platform_specific: PlatformSpecific {
                            application_id: String::from(app_id()),
                            override_redirect: true,
                        },
                        ..Default::default()
                    });
                    self.windows.insert(id, WindowType::AreaSelector);
                    if cfg!(target_os = "macos") || cfg!(target_os = "windows") {
                        let mut task = open_task.discard().chain(window::gain_focus(id));
                        #[cfg(target_os = "windows")]
                        {
                            task = task.chain(Self::apply_transparency(id));
                        }
                        task
                    } else {
                        open_task
                            .discard()
                            .chain(window::gain_focus(id))
                            .chain(window::set_mode(id, Mode::Fullscreen))
                    }
                } else {
                    Task::none()
                }
            }
            AppEvent::OpenAnnotationWindow => {
                let Some(crate::config::Mode::Caster(caster)) = &mut self.config.mode else {
                    unreachable!("Mode must be Caster here")
                };

                if !self.windows.contains(WindowType::Annotation) {
                    let (w, h, x, y) = Self::caster_display_size(caster);
                    let dpi = Self::caster_dpi_scale(caster);

                    let (id, open_task) = window::open(window::Settings {
                        size: Size {
                            width: (w / dpi) - 1.0,
                            height: (h / dpi) - 1.0,
                        },
                        position: Position::Specific(Point {
                            x: x / dpi,
                            y: y / dpi,
                        }),
                        transparent: true,
                        decorations: false,
                        resizable: false,
                        #[cfg(target_os = "windows")]
                        platform_specific: PlatformSpecific {
                            drag_and_drop: false,
                            skip_taskbar: true,
                            undecorated_shadow: false,
                            corner_preference: Default::default(),
                        },
                        #[cfg(target_os = "macos")]
                        platform_specific: PlatformSpecific {
                            title_hidden: true,
                            titlebar_transparent: true,
                            fullsize_content_view: true,
                        },
                        #[cfg(target_os = "linux")]
                        platform_specific: PlatformSpecific {
                            application_id: String::from(app_id()),
                            override_redirect: true,
                        },
                        ..Default::default()
                    });
                    self.windows.insert(id, WindowType::Annotation);
                    if cfg!(target_os = "macos") || cfg!(target_os = "windows") {
                        let mut task = open_task.discard().chain(window::gain_focus(id));
                        #[cfg(target_os = "windows")]
                        {
                            task = task.chain(Self::apply_transparency(id));
                        }
                        task
                    } else {
                        open_task
                            .discard()
                            .chain(window::gain_focus(id))
                            .chain(window::set_mode(id, Mode::Fullscreen))
                    }
                } else {
                    Task::none()
                }
            }
            AppEvent::AreaSelected(rect) => {
                if let Some(crate::config::Mode::Caster(caster)) = &mut self.config.mode {
                    let dpi = Self::caster_dpi_scale(caster);
                    let scaled = crate::gui::common::datastructure::ScreenRect {
                        x: rect.x * dpi,
                        y: rect.y * dpi,
                        width: rect.width * dpi,
                        height: rect.height * dpi,
                    };
                    caster.resize_rec_area(scaled);
                }
                Task::none()
            }
            AppEvent::CloseWindow(id) => {
                self.windows
                    .remove(id, self.windows.of_type(id, WindowType::Main));
                window::close(id)
            }
            AppEvent::WindowEvent(id, message) => match self.windows.get_manager_mut(id) {
                Some(window_handler) => window_handler.update(id, message, &mut self.config),
                None => Task::none(),
            },
            AppEvent::TimeTick => {
                if let Some(crate::config::Mode::Caster(caster)) = &mut self.config.mode
                    && caster.is_streaming()
                {
                    caster.streaming_time += 1;
                }
                self.config.e_time += 1;
                Task::none()
            }
            AppEvent::WindowResized(id, width, height) => {
                if self.windows.of_type(id, WindowType::Main) {
                    self.config.window_size = Size {
                        width: width as f32,
                        height: height as f32,
                    };
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
            AppEvent::ToggleAudioMute => {
                match &mut self.config.mode {
                    Some(crate::config::Mode::Caster(caster)) => {
                        caster.toggle_audio_mute();
                    }
                    Some(crate::config::Mode::Receiver(receiver)) => {
                        receiver.toggle_audio_mute();
                    }
                    _ => {}
                }
                Task::none()
            }
            AppEvent::KeyEvent(modifier, key) => {
                if key == Key::Unidentified {
                    return Task::none();
                }
                let item = (modifier, key);

                if self.config.shortcuts.updating != KeyTypes::None {
                    match self.config.shortcuts.updating {
                        KeyTypes::Pause => self.config.shortcuts.pause = item,
                        KeyTypes::Record => self.config.shortcuts.record = item,
                        KeyTypes::BlankScreen => self.config.shortcuts.blank_screen = item,
                        KeyTypes::Close => self.config.shortcuts.end_session = item,
                        _ => {}
                    }
                    Task::none()
                } else if item == self.config.shortcuts.pause
                    || item == self.config.shortcuts.record
                {
                    Task::done(AppEvent::CasterToggleStreaming)
                } else if item == self.config.shortcuts.blank_screen {
                    Task::done(AppEvent::BlankScreen)
                } else if item == self.config.shortcuts.end_session {
                    Task::done(AppEvent::ExitApp)
                } else {
                    Task::none()
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
            AppEvent::Ignore => Task::none(),
            _ => Task::none(),
        }
    }

    pub fn view(&self, id: Id) -> Element<'_, AppEvent> {
        match self.windows.get_manager(id) {
            Some(window_handler) => window_handler
                .view(&self.config)
                .map(move |message| AppEvent::WindowEvent(id, message)),
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

    pub fn style(&self, theme: &StyleType) -> Style {
        Style {
            background_color: theme.get_palette().background,
            text_color: theme.get_palette().text,
        }
    }

    pub fn subscription(&self) -> Subscription<AppEvent> {
        let mut batch = vec![
            Subscription::run(tray_menu_listener),
            Subscription::run(tray_icon_listener),
            iced::time::every(Duration::from_secs(1)).map(|_| AppEvent::TimeTick),
            Subscription::run(ipc),
            self.keyboard_subscription(),
            self.window_subscription(),
        ];

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
            Window(window::Event::CloseRequested) => Some(AppEvent::CloseWindow(id)),
            Window(window::Event::Resized(size)) => Some(AppEvent::WindowResized(
                id,
                size.width as u32,
                size.height as u32,
            )),
            _ => None,
        })
    }
}

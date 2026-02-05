//! Window management types and traits
//!
//! This module provides the `WindowManager` enum for managing different window types,
//! the `Windows` collection for tracking open windows, and the `GuiWindow` trait
//! that all windows must implement.

use crate::config::Config;
use crate::gui::common::messages::AppEvent;
use crate::gui::style::theme::csx::StyleType;
use crate::gui::widget::Element;
use crate::gui::windows::annotation::{AnnotationWindow, AnnotationWindowEvent};
use crate::gui::windows::area_selector::{ASWindow, ASWindowEvent};
use crate::gui::windows::main::{MainWindow, MainWindowEvent};
use crate::utils::bimap::{BiMap, Either};
use iced::Task;
use iced::window::Id;
use std::collections::HashMap;
use std::collections::hash_map::Iter;
use std::hash::Hash;

/// Types of windows in the application.
#[derive(Clone, Hash, Eq, PartialEq, Copy)]
pub enum WindowType {
    Main,
    AreaSelector,
    Annotation,
}

/// Manager for different window types.
///
/// This enum wraps the specific window implementations and provides
/// a uniform interface for interacting with any window type.
pub enum WindowManager {
    Main(Box<MainWindow>),
    AreaSelector(ASWindow),
    Annotation(AnnotationWindow),
}

/// Messages that can be sent to windows.
#[derive(Clone, Debug)]
pub enum WindowMessage {
    Main(MainWindowEvent),
    AreaSelector(ASWindowEvent),
    Annotation(AnnotationWindowEvent),
}

/// Trait that all GUI windows must implement.
///
/// This trait defines the common interface for windows including
/// title, update logic, view rendering, and theming.
pub trait GuiWindow {
    type Message;
    fn title(&self) -> String;
    fn update(&mut self, id: Id, message: Self::Message, config: &mut Config) -> Task<AppEvent>;
    fn view(&self, config: &Config) -> Element<'_, Self::Message>;
    fn theme(&self) -> StyleType;
}

impl WindowManager {
    pub fn title(&self) -> String {
        match self {
            Self::Main(window) => window.title(),
            Self::AreaSelector(window) => window.title(),
            Self::Annotation(window) => window.title(),
        }
    }

    pub fn update(
        &mut self,
        id: Id,
        message: WindowMessage,
        config: &mut Config,
    ) -> Task<AppEvent> {
        match self {
            Self::Main(window) => {
                let WindowMessage::Main(message) = message else {
                    return Task::none();
                };
                window.update(id, message, config)
            }
            Self::AreaSelector(window) => {
                let WindowMessage::AreaSelector(message) = message else {
                    return Task::none();
                };
                window.update(id, message, config)
            }
            Self::Annotation(window) => {
                let WindowMessage::Annotation(message) = message else {
                    return Task::none();
                };
                window.update(id, message, config)
            }
        }
    }

    pub fn view(&self, config: &Config) -> Element<'_, WindowMessage> {
        match self {
            Self::Main(window) => window.view(config).map(WindowMessage::Main),
            Self::AreaSelector(window) => window.view(config).map(WindowMessage::AreaSelector),
            Self::Annotation(window) => window.view(config).map(WindowMessage::Annotation),
        }
    }

    pub fn theme(&self) -> StyleType {
        match self {
            Self::Main(window) => window.theme(),
            Self::AreaSelector(window) => window.theme(),
            Self::Annotation(window) => window.theme(),
        }
    }
}

impl From<Id> for Either<Id, WindowType> {
    fn from(value: Id) -> Self {
        Either::Left(value)
    }
}

impl From<WindowType> for Either<Id, WindowType> {
    fn from(value: WindowType) -> Self {
        Either::Right(value)
    }
}

/// Collection of open windows with bidirectional lookup.
///
/// This struct manages the set of open windows and provides
/// efficient lookup by both window ID and window type.
pub struct Windows {
    windows: HashMap<Id, WindowManager>,
    persistent: HashMap<WindowType, WindowManager>,
    w_type: BiMap<Id, WindowType>,
}

impl Windows {
    pub fn new() -> Self {
        Windows {
            windows: HashMap::new(),
            w_type: BiMap::new(),
            persistent: HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: Id, w_type: WindowType) {
        if let Some(window) = self.persistent.remove(&w_type) {
            self.windows.insert(id, window);
        } else {
            self.windows.insert(
                id,
                match w_type {
                    WindowType::Main => WindowManager::Main(Box::new(MainWindow::new())),
                    WindowType::AreaSelector => WindowManager::AreaSelector(ASWindow::new()),
                    WindowType::Annotation => WindowManager::Annotation(AnnotationWindow::new()),
                },
            );
        }

        self.w_type.insert(id, w_type);
    }

    pub fn get_id(&self, t: WindowType) -> Option<Id> {
        let w_id = self.w_type.get(t);
        if let Some(Either::Left(w_id)) = w_id {
            Some(*w_id)
        } else {
            None
        }
    }

    pub fn get_type(&self, id: Id) -> Option<WindowType> {
        let w_type = self.w_type.get(id);
        if let Some(Either::Right(w_type)) = w_type {
            Some(*w_type)
        } else {
            None
        }
    }

    pub fn of_type(&self, id: Id, w_type: WindowType) -> bool {
        self.get_type(id) == Some(w_type)
    }

    pub fn contains<T>(&self, t: T) -> bool
    where
        T: Into<Either<Id, WindowType>>,
    {
        self.w_type.contains(t)
    }

    pub fn get_manager_mut<T>(&mut self, t: T) -> Option<&mut WindowManager>
    where
        T: Into<Either<Id, WindowType>>,
    {
        let key: Either<Id, WindowType> = t.into();

        match key {
            Either::Left(w_id) => self.windows.get_mut(&w_id),
            Either::Right(w_type) => {
                if let Some(Either::Left(w_id)) = self.w_type.get(w_type) {
                    self.windows.get_mut(w_id)
                } else {
                    None
                }
            }
        }
    }

    pub fn get_manager<T>(&self, t: T) -> Option<&WindowManager>
    where
        T: Into<Either<Id, WindowType>>,
    {
        let key: Either<Id, WindowType> = t.into();

        match key {
            Either::Left(w_id) => self.windows.get(&w_id),
            Either::Right(w_type) => {
                if let Some(Either::Left(w_id)) = self.w_type.get(w_type) {
                    self.windows.get(w_id)
                } else {
                    None
                }
            }
        }
    }

    pub fn remove<T>(&mut self, t: T, persistent: bool)
    where
        T: Into<Either<Id, WindowType>>,
    {
        let key: Either<Id, WindowType> = t.into();

        let w_type = match key {
            Either::Left(w_id) => self.get_type(w_id),
            Either::Right(w_type) => Some(w_type),
        };

        if let Some(w_type) = w_type {
            let w_id = self.w_type.remove(w_type);
            let w_manager = self.windows.remove(&w_id.unwrap());

            if let Some(w_manager) = w_manager
                && persistent
            {
                self.persistent.insert(w_type, w_manager);
            }
        }
    }

    pub fn iter(&self) -> Iter<'_, Id, WindowType> {
        self.w_type.iter()
    }
}

impl Default for Windows {
    fn default() -> Self {
        Self::new()
    }
}

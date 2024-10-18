use crate::config::Config;
use crate::gui::common::messages::AppEvent;
use crate::gui::style::theme::csx::StyleType;
use crate::gui::widget::Element;
use crate::gui::windows::annotation::{AnnotationWindow, AnnotationWindowEvent};
use crate::gui::windows::area_selector::{ASWindow, ASWindowEvent};
use crate::gui::windows::main::{MainWindow, MainWindowEvent};
use crate::utils::bimap::{BiMap, Either};
use iced::window::Id;
use iced::Task;
use std::collections::hash_map::Iter;
use std::collections::HashMap;
use std::hash::Hash;

pub mod main;
pub mod area_selector;
pub mod annotation;

#[derive(Clone, Hash, Eq, PartialEq, Copy)]
pub enum WindowType {
    Main,
    AreaSelector,
    Annotation,
}

pub enum WindowManager {
    Main(MainWindow),
    AreaSelector(ASWindow),
    Annotation(AnnotationWindow),
}

#[derive(Clone, Debug)]
pub enum WindowMessage {
    Main(MainWindowEvent),
    AreaSelector(ASWindowEvent),
    Annotation(AnnotationWindowEvent),
}

pub trait GuiWindow {
    type Message;
    fn title(&self) -> String;
    fn update(&mut self, id: Id, message: Self::Message, config: &mut Config) -> Task<AppEvent>;
    fn view(&self, config: &Config) -> Element<Self::Message>;
    fn theme(&self) -> StyleType;
}

impl WindowManager {
    pub(crate) fn title(&self) -> String {
        match self {
            Self::Main(window) => window.title(),
            Self::AreaSelector(window) => window.title(),
            Self::Annotation(window) => window.title(),
        }
    }

    pub(crate) fn update(&mut self, id: Id, message: WindowMessage, config: &mut Config) -> Task<AppEvent> {
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

    pub(crate) fn view(&self, config: &Config) -> Element<WindowMessage> {
        match self {
            Self::Main(window) => window.view(config).map(move |message| WindowMessage::Main(message)),
            Self::AreaSelector(window) => window.view(config).map(move |message| WindowMessage::AreaSelector(message)),
            Self::Annotation(window) => window.view(config).map(move |message| WindowMessage::Annotation(message)),
        }
    }

    pub(crate) fn theme(&self) -> StyleType {
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

pub struct Windows {
    windows: HashMap<Id, WindowManager>,
    w_type: BiMap<Id, WindowType>,
    persistent: HashMap<WindowType, WindowManager>,
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
            self.windows.insert(id, match w_type {
                WindowType::Main => WindowManager::Main(MainWindow::new()),
                WindowType::AreaSelector => WindowManager::AreaSelector(ASWindow::new()),
                WindowType::Annotation => WindowManager::Annotation(AnnotationWindow::new()),
            });
        }

        self.w_type.insert(id, w_type);
    }

    pub fn get_id(&self, t: WindowType) -> Option<Id> {
        let w_id = self.w_type.get(t);
        if let Some(Either::Left(w_id)) = w_id {
            Some(w_id.clone())
        } else {
            None
        }
    }

    pub fn get_type(&self, id: Id) -> Option<WindowType> {
        let w_type = self.w_type.get(id);
        if let Some(Either::Right(w_type)) = w_type {
            Some(w_type.clone())
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
                    self.windows.get_mut(&w_id)
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
                    self.windows.get(&w_id)
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
            Either::Right(w_type) => Some(w_type)
        };

        if let Some(w_type) = w_type {
            let w_id = self.w_type.remove(w_type);
            let w_manager = self.windows.remove(&w_id.unwrap());

            if let Some(w_manager) = w_manager {
                if persistent {
                    self.persistent.insert(w_type, w_manager);
                }
            }
        }
    }

    pub(crate) fn iter(&self) -> Iter<'_, Id, WindowType> {
        self.w_type.iter()
    }
}
use crate::config::Config;
use crate::gui::common::messages::AppEvent;
use crate::gui::style::theme::csx::StyleType;
use crate::gui::widget::{Element, IcedRenderer};
use crate::windows::area_selector::{ASWindow, ASWindowEvent};
use crate::windows::main::{MainWindow, MainWindowEvent};
use iced_core::window::Id;
use iced_runtime::Task;

pub mod main;
pub mod area_selector;

pub enum WindowManager {
    Main(MainWindow),
    AreaSelector(ASWindow),
}

#[derive(Clone, Debug)]
pub enum WindowMessage {
    Main(MainWindowEvent),
    AreaSelector(ASWindowEvent),
}

pub trait GuiWindow {
    type Message;
    fn new() -> Self;
    fn title(&self) -> String;
    fn update(&mut self, id: Id, message: Self::Message, config: &mut Config) -> Task<AppEvent>;
    fn view(&self, config: &Config) -> Element<Self::Message, StyleType, IcedRenderer>;
    fn theme(&self) -> StyleType;
}

impl WindowManager {
    pub(crate) fn title(&self) -> String {
        match self {
            Self::Main(window) => window.title(),
            Self::AreaSelector(window) => window.title(),
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
        }
    }

    pub(crate) fn view(&self, config: &Config) -> Element<WindowMessage, StyleType, IcedRenderer> {
        match self {
            Self::Main(window) => window.view(config).map(move |message| WindowMessage::Main(message)),
            Self::AreaSelector(window) => window.view(config).map(move |message| WindowMessage::AreaSelector(message)),
        }
    }

    pub(crate) fn theme(&self) -> StyleType {
        match self {
            Self::Main(window) => window.theme(),
            Self::AreaSelector(window) => window.theme(),
        }
    }
}
use crate::gui::components::awmodal::{GuiComponent, GuiInterface};
use crate::gui::popup::ip::IPModal;
use crate::gui::popup::shortcuts::ShortcutModal;
use crate::gui::popup::wrtc::WrtcModal;
use crate::gui::windows::main::MainWindowEvent;
use std::any::Any;

pub enum PopupType {
    IP(IPModal),
    HotkeyUpdate(ShortcutModal),
    ManualWRTC(WrtcModal),
}

impl GuiComponent for PopupType {
    type Message = MainWindowEvent;

    fn as_gui<'a>(&'a self) -> &'a dyn GuiInterface<Message=Self::Message> {
        match self {
            PopupType::IP(modal) => modal,
            PopupType::HotkeyUpdate(modal) => modal,
            PopupType::ManualWRTC(modal) => modal,
        }
    }

    fn as_mut_gui(&mut self) -> &mut dyn GuiInterface<Message=Self::Message> {
        match self {
            PopupType::IP(modal) => modal,
            PopupType::HotkeyUpdate(modal) => modal,
            PopupType::ManualWRTC(modal) => modal,
        }
    }

    fn as_mut_any(&mut self) -> Box<&mut dyn Any>
    {
        match self {
            PopupType::IP(modal) => Box::from(modal as &mut dyn Any),
            PopupType::HotkeyUpdate(modal) => Box::from(&mut *modal as &mut dyn Any),
            PopupType::ManualWRTC(modal) => Box::from(&mut *modal as &mut dyn Any),
        }
    }
}
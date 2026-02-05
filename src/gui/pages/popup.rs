use crate::gui::components::awmodal::{GuiComponent, GuiInterface};
use crate::gui::popup::ip::IPModal;
use crate::gui::popup::shortcuts::ShortcutModal;
use crate::gui::popup::wrtc::WrtcModal;
use crate::gui::windows::main::MainWindowEvent;

pub enum PopupType {
    IP(IPModal),
    HotkeyUpdate(ShortcutModal),
    ManualWRTC(WrtcModal),
}

impl GuiComponent for PopupType {
    type Message = MainWindowEvent;

    fn as_gui(&self) -> &dyn GuiInterface<Message = Self::Message> {
        match self {
            PopupType::IP(modal) => modal,
            PopupType::HotkeyUpdate(modal) => modal,
            PopupType::ManualWRTC(modal) => modal,
        }
    }

    fn as_mut_gui(&mut self) -> &mut dyn GuiInterface<Message = Self::Message> {
        match self {
            PopupType::IP(modal) => modal,
            PopupType::HotkeyUpdate(modal) => modal,
            PopupType::ManualWRTC(modal) => modal,
        }
    }
}

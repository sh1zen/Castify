use crate::gui::components::screenArea::ScreenRect;
use crate::gui::components::{caster, home, hotkeys, popup};
use iced::keyboard::{Key, Modifiers};

#[derive(Debug, Clone)]
pub enum AppEvent {
    /// homepage
    Home,
    /// the app mode caster / receiver
    Mode(home::Message),
    /// caster play pause
    Caster(caster::Message),
    /// A collector of all popups messages
    PopupMessage(popup::Interaction),
    /// close any popup
    ClosePopup,
    /// Connect to caster, passing caster ip as String
    ConnectToCaster(String),
    /// Save the capture
    SaveCapture,
    /// stop saving capture
    SaveCaptureStop,
    /// Ignore
    Ignore,
    /// blank the recording
    BlankScreen,
    /// The app window size has been changed
    WindowResized(u32, u32),
    /// Quit the app
    CloseRequested,
    /// Open the supplied web page
    OpenWebPage(String),
    /// Connection Error
    ConnectionError,
    /// Setup hotkeys
    HotkeysPage,
    /// handle hot keys request update
    HotkeysTypePage(hotkeys::KeyTypes),
    /// update hot key
    HotkeysUpdate((Modifiers, Key)),
    /// hotkeys support
    KeyPressed((Modifiers, Key)),
    /// Request for area selection page
    AreaSelection,
    /// Messages for handling area selection, set to 0 to restore default screen size
    AreaSelected(ScreenRect),
}
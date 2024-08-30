use iced::keyboard::{Key, Modifiers};

use crate::gui::components::{caster, hotkeys, popup, home};

#[derive(Debug, Clone)]
/// Messages types that permit to react to application interactions/subscriptions
pub enum Message {
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
    /// Emit when the main window be focused
    WindowFocused,
    /// The app window position has been changed
    WindowMoved(i32, i32),
    /// The app window size has been changed
    WindowResized(u32, u32),
    /// Quit the app
    CloseRequested,
    /// Drag the window
    Drag,
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
    /// Messages for handling area selection
    AreaSelection(AreaSelectionMessage),
}
#[derive(Debug, Clone, Copy)]
pub enum AreaSelectionMessage {
    /// Start the area selection
    StartSelection { x: f32, y: f32 },
    /// Update the area selection
    UpdateSelection { x: f32, y: f32 },
    /// End the area selection
    EndSelection,
}

use crate::gui::common::datastructure::ScreenRect;
use crate::gui::windows::WindowMessage;
use iced::keyboard::{Key, Modifiers};
use iced_core::window::Id;

#[derive(Debug, Clone)]
pub enum AppEvent
{
    /// Open Main Window
    OpenMainWindow,
    /// Open Annotation Window
    OpenAnnotationWindow,
    /// Close an app window
    CloseWindow(Id),
    /// Specified Window Event Message
    WindowEvent(Id, WindowMessage),
    /// The app window size has been changed
    WindowResized(Id, u32, u32),
    /// Time tick update
    TimeTick,
    /// Ignore
    Ignore,
    /// Quit the app
    ExitApp,
    /// Open the supplied web page
    OpenWebPage(String),
    /// blank the recording
    BlankScreen,
    /// Connection Error
    ConnectionError,
    /// Hotkeys support
    KeyPressed(Modifiers, Key),
    /// Request for area selection page
    OpenAreaSelectionWindow,
    /// Messages for handling area selection, set to 0 to restore default screen size
    AreaSelected(ScreenRect),
    /// Handle Caster Rec/Pause actions
    CasterToggleStreaming,
    Terminate,
}
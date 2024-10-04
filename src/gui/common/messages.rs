use crate::gui::common::datastructure::ScreenRect;
use crate::windows::WindowMessage;
use iced::keyboard::{Key, Modifiers};
use iced_core::window::Id;

#[derive(Debug, Clone)]
pub enum AppEvent
{
    /// Open Main Window
    ShowMainWindow,
    /// Close an app window
    CloseWindow(Id),
    /// Specified Window Event Message
    WindowEvent(Id, WindowMessage),
    /// The app window size has been changed
    WindowResized(u32, u32),
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
    /// hotkeys support
    KeyPressed(Modifiers, Key),
    /// Request for area selection page
    AreaSelection,
    /// Messages for handling area selection, set to 0 to restore default screen size
    AreaSelected(ScreenRect),
    CasterToggleStreaming,
}
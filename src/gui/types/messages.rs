use iced::keyboard::Key;

use crate::gui::components::start;

#[derive(Debug, Clone)]
/// Messages types that permit to react to application interactions/subscriptions
pub enum Message {
    /// the app mode caster / receiver
    Mode(start::Message),
    ///
    KeyPressed(Key),
    /// Ignore
    Ignore,
    /// Start recording packets
    Start,
    /// Pause recording
    Pause,
    /// The enter (return) key has been pressed
    ReturnKeyPressed,
    /// The esc key has been pressed
    EscKeyPressed,
    /// The reset button has been pressed or the backspace key has been pressed while running
    ResetButtonPressed,
    /// Ctrl+D keys have been pressed
    CtrlDPressed,
    /// Left (false) or Right (true) arrow key has been pressed
    ArrowPressed(bool),
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
    /// Ctrl+T keys have been pressed
    CtrlTPressed,
    /// Open the supplied web page
    OpenWebPage(String),
}
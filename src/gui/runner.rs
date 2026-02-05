//! GUI application runner
//!
//! This module provides the main entry point for running the GUI application.

use crate::assets::{FONT_AWESOME_BYTES, FONT_BASE_BYTES, FONT_FAMILY_BASE};
use crate::config::app_id;
use crate::utils::flags::Flags;
use native_dialog::{DialogBuilder, MessageLevel};

use super::app::App;

/// Runs the GUI application with the given flags.
///
/// This function initializes the Iced daemon with all necessary settings
/// including fonts, themes, and subscriptions. If the GUI fails to initialize,
/// it displays an error dialog before exiting.
pub fn run(flags: Flags) {
    let app = iced::daemon(move || App::new(flags), App::update, App::view)
        .settings(iced::Settings {
            id: Some(app_id()),
            ..Default::default()
        })
        .title(App::title)
        .style(App::style)
        .theme(App::theme)
        .antialiasing(false)
        .font(FONT_AWESOME_BYTES)
        .font(FONT_BASE_BYTES)
        .default_font(FONT_FAMILY_BASE)
        .scale_factor(|_, _| 1.0)
        .subscription(App::subscription);

    if let Err(e) = app.run() {
        eprintln!("Failed to initialize GUI: {e:?}");

        if let Err(e) = DialogBuilder::message()
            .set_title("Gui error")
            .set_text(e.to_string().as_str())
            .set_level(MessageLevel::Warning)
            .alert()
            .show()
        {
            eprintln!("Failed to display error dialog: {e:?}");
        }
    }
}

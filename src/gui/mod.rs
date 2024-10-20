use crate::assets::{FONT_AWESOME_BYTES, FONT_BASE_BYTES, FONT_FAMILY_BASE};
use crate::config::app_id;
use crate::utils::flags::Flags;

mod app;
mod components;
mod style;
pub mod common;
mod widget;
mod windows;
mod pages;

use self::app::App;

// todo move into widget
#[macro_export]
macro_rules! column {
    () => (
        $crate::gui::widget::Column::new()
    );
    ($($x:expr),+ $(,)?) => (
        $crate::gui::widget::Column::with_children([$($crate::gui::widget::Element::from($x)),+])
    );
}

#[macro_export]
macro_rules! row {
    () => (
        $crate::gui::widget::Row::new()
    );
    ($($x:expr),+ $(,)?) => (
        $crate::gui::widget::Row::with_children([$($crate::gui::widget::Element::from($x)),+])
    );
}

pub fn run(flags: Flags) {
    let app = iced::daemon(App::title, App::update, App::view)
        .settings(iced::Settings {
            id: Some(app_id()),
            ..Default::default()
        })
        .style(App::style)
        .theme(App::theme)
        .antialiasing(true)
        .font(FONT_AWESOME_BYTES)
        .font(FONT_BASE_BYTES)
        .default_font(FONT_FAMILY_BASE)
        .scale_factor(|_, _| 1.0)
        .subscription(App::subscription);

    if let Err(e) = app.run_with(|| { App::new(flags) }) {
        eprintln!("Failed to initialize GUI: {e:?}");

        if let Err(e) = native_dialog::MessageDialog::new()
            .set_type(native_dialog::MessageType::Error)
            .set_text(e.to_string().as_str())
            .show_alert()
        {
            eprintln!("Failed to display error dialog: {e:?}");
        }
    }
}

use crate::assets::{FONT_BASE_DATA, FONT_FAMILY_BASE, ICONS_BYTES};
use crate::utils::tray_icon::tray_icon;
use std::borrow::Cow;

pub mod app;
pub mod components;
pub mod style;
pub mod common;
pub mod video;
pub mod widget;

use self::app::App;

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

pub fn run() {
    let _tray_icon = tray_icon();

    let app = iced::daemon(App::title, App::update, App::view)
        .style(App::style)
        .theme(App::theme)
        .antialiasing(true)
        .font(Cow::Borrowed(ICONS_BYTES))
        .font(Cow::Borrowed(FONT_BASE_DATA))
        .default_font(FONT_FAMILY_BASE)
        .subscription(App::subscription);

    if let Err(e) = app.run_with(App::new) {
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

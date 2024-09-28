use crate::gui::resource::{APP_NAME, ICONS_BYTES, ICON_BYTES, RALEWAY_FONT_BYTES, TEXT_FONT_FAMILY_NAME};
use crate::gui::common::flags::Flags;
use iced::window;
use iced_core::window::Position;
use iced_core::{Font, Size};
use std::borrow::Cow;

pub mod app;
pub mod components;
pub mod resource;
pub mod style;
pub mod common;
pub mod video;
pub mod widget;

use self::app::App;

pub fn run(flags: Flags) {
    let app = iced::application(APP_NAME, App::update, App::view)
        .window(window::Settings {
            size: Size::new(640f32, 380f32),
            position: Position::Centered,
            min_size: Some(Size::new(400f32, 300f32)),
            visible: true,
            resizable: true,
            decorations: true,
            transparent: true,
            exit_on_close_request: true,
            icon: Some(
                window::icon::from_file_data(
                    ICON_BYTES,
                    None,
                ).unwrap(),
            ),
            ..Default::default()
        })
        .style(App::style)
        .theme(App::theme)
        .antialiasing(true)
        .centered()
        .font(Cow::Borrowed(ICONS_BYTES))
        .font(Cow::Borrowed(RALEWAY_FONT_BYTES))
        .default_font(Font::with_name(TEXT_FONT_FAMILY_NAME))
        .subscription(App::subscription);

    if let Err(e) = app.run_with(move || App::new(flags)) {
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

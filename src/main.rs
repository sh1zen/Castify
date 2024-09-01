#![cfg_attr(
    not(debug_assertions),
    windows_subsystem = "windows"
)] // hide console window on Windows in release

use castgo::gui::appbase::App;
use castgo::gui::resource::{APP_NAME_ID, FONT_SIZE_BODY, ICONS_BYTES, ICON_BYTES, RALEWAY_FONT_BYTES, TEXT_FONT_FAMILY_NAME};
use castgo::workers;
use iced::{Application, Font, Pixels, Settings, Size};
use iced_core::window::Position;
use std::borrow::Cow;
use std::{panic, process};


#[tokio::main]
async fn main() {
    gstreamer::init().expect("gstreamer init error.");

    // kill the main thread as soon as a secondary thread panics
    let orig_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        workers::sos::get_instance().lock().unwrap().terminate();
        // invoke the default handler and exit the process
        orig_hook(panic_info);
        process::exit(120);
    }));

    // gracefully close the app when receiving SIGINT, SIGTERM, or SIGHUP
    ctrlc::set_handler(move || {
        workers::sos::get_instance().lock().unwrap().terminate();
        process::exit(130);
    }).expect("Error setting Ctrl-C handler");

    App::run(Settings {
        id: Some(String::from(APP_NAME_ID)),
        antialiasing: true,
        window: iced::window::Settings {
            size: Size::new(640f32, 380f32),
            position: Position::Centered,
            min_size: Some(Size::new(400f32, 300f32)),
            visible: true,
            resizable: true,
            decorations: true,
            transparent: true,
            exit_on_close_request: true,
            icon: Some(
                iced::window::icon::from_file_data(
                    ICON_BYTES,
                    None,
                ).unwrap(),
            ),
            ..Default::default()
        },
        flags: App::new(true),
        fonts: vec![
            Cow::Borrowed(RALEWAY_FONT_BYTES),
            Cow::Borrowed(ICONS_BYTES),
        ],
        default_font: Font::with_name(TEXT_FONT_FAMILY_NAME),
        default_text_size: Pixels(FONT_SIZE_BODY),
    }).unwrap();
}
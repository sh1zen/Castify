#![cfg_attr(
    not(debug_assertions),
    windows_subsystem = "windows"
)] // hide console window on Windows in release

use std::borrow::Cow;
use std::{panic, process};

use castgo::gui::resource::{APP_NAME_ID, FONT_SIZE_BODY, ICONS_BYTES, RALEWAY_FONT_BYTES, TEXT_FONT_FAMILY_NAME};
use castgo::gui::types::appbase::App;
use iced::{Application, Font, Pixels, Sandbox, Settings, Size};

#[derive(Debug)]
enum Mode {
    Caster,
    Receiver,
}

#[tokio::main]
async fn main() {

    // kill the main thread as soon as a secondary thread panics
    let orig_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // invoke the default handler and exit the process
        orig_hook(panic_info);
        process::exit(1);
    }));

    // gracefully close the app when receiving SIGINT, SIGTERM, or SIGHUP
    ctrlc::set_handler(move || {
        process::exit(130);
    }).expect("Error setting Ctrl-C handler");

    App::run(Settings {
        id: Some(String::from(APP_NAME_ID)),
        antialiasing: true,
        window: iced::window::Settings {
            size: Size::new(640f32, 373f32),
            min_size: Some(Size::new(400f32, 300f32)),
            visible: true,
            resizable: true,
            decorations: true,
            transparent: false,
            exit_on_close_request: false,
            icon: Some(
                iced::window::icon::from_file_data(
                    include_bytes!("../resources/icons/192x192.png"),
                    None,
                )
                    .unwrap(),
            ),
            ..Default::default()
        },
        flags: App::new(),
        fonts: vec![
            Cow::Borrowed(RALEWAY_FONT_BYTES),
            Cow::Borrowed(ICONS_BYTES),
        ],
        default_font: Font::with_name(TEXT_FONT_FAMILY_NAME),
        default_text_size: Pixels(FONT_SIZE_BODY),
    }).unwrap();

    /*
    let events = events::Events::init();

    // Start grabbing events; handle errors if any occur
    if let Err(error) = grab(move |e| events.handle(e)) {
        println!("Error: {error:?}");
    }
    */
}

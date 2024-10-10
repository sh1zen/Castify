#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{panic, process};

pub mod gui;
pub mod utils;
pub mod workers;
pub mod assets;
pub mod config;
pub mod xmacro;

fn main() {
    let os_supported = gstreamer::init().is_ok();

    // kill the main thread as soon as a secondary thread panics
    let orig_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // invoke the default handler and exit the process
        orig_hook(panic_info);
        process::exit(105);
    }));

    // gracefully close the app when receiving SIGINT, SIGTERM, or SIGHUP
    ctrlc::set_handler(move || {
        process::exit(130);
    }).expect("Error setting Ctrl-C handler");

    if !os_supported {
        if let Err(e) = native_dialog::MessageDialog::new()
            .set_type(native_dialog::MessageType::Error)
            .set_text("OS not yet supported.")
            .show_alert()
        {
            eprintln!("Failed to display error dialog: {e:?}");
        }
    }

    gui::run();
}
#![cfg_attr(
    not(debug_assertions),
    windows_subsystem = "windows"
)] // hide console window on Windows in release

mod gui;
mod utils;
mod workers;
mod xmacro;

use std::{panic, process};
use crate::gui::common::flags::Flags;

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

    let flags = Flags {
        os_supported: true,
    };

    gui::run(flags);
}
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::config::{app_id, app_name, app_version};
use crate::utils::flags::Flags;
use clap::{Arg, Command, ValueHint};
use interprocess::local_socket::traits::Stream;
use interprocess::local_socket::{GenericNamespaced, ToNsName};
use std::{panic, process};

pub mod gui;
pub mod utils;
pub mod workers;
pub mod assets;
pub mod config;
pub mod xmacro;

fn main() {
    let app_name = Box::leak(app_name().into_boxed_str());

    let matches = Command::new(&*app_name)
        .version(app_version())
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::new("multi-instance")
                .short('m')
                .long("multi-instance")
                .value_name("MULTI INSTANCE")
                .help("Allow multi application instance at once (yes/no).")
                .default_value("no")
                .value_hint(ValueHint::CommandString)
        )
        .get_matches();

    let multi_instances = match matches.get_one::<String>("multi-instance") {
        Some(v) => v.to_lowercase() == "yes",
        None => false,
    };

    if !multi_instances {
        let name = app_id().to_ns_name::<GenericNamespaced>().unwrap();
        if interprocess::local_socket::Stream::connect(name).is_ok() {
            return;
        };
    }

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

    gui::run(Flags {
        multi: multi_instances
    });
}
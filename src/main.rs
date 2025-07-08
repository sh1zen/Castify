#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::config::{app_id, app_name, app_version};
use crate::utils::flags::Flags;
use clap::{Arg, Command};
use interprocess::local_socket::traits::Stream;
use interprocess::local_socket::{GenericNamespaced, ToNsName};
use std::{panic, process};
use native_dialog::DialogBuilder;

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
                .long("multi")
                .value_name("MULTI INSTANCE")
                .help("Allow multiple application instance at once (yes/no).")
                .required(false)
                .default_missing_value("yes")
                .ignore_case(true)
                .num_args(0..=1)
                .default_value("no")
        )
        .get_matches();

    let multi_instances = match matches.get_one::<String>("multi-instance") {
        Some(val) => &val.to_lowercase() == "yes",
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
        if let Err(e) = DialogBuilder::message()
            .set_text("OS not yet supported.")
            .alert()
            .show()
        {
            eprintln!("Failed to display error dialog: {e:?}");
        }
    }

    gui::run(Flags {
        multi_instance: multi_instances
    });
}
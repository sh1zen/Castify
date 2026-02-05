use crate::config::app_name;
use std::env::var_os;
use std::fs::DirBuilder;
use std::path::Path;

fn home_path() -> Option<String> {
    #[cfg(not(target_os = "windows"))]
    let home = var_os("HOME").map(|home| home.to_string_lossy().to_string());

    #[cfg(target_os = "windows")]
    let home = var_os("HOMEDRIVE").and_then(|drive| {
        var_os("HOMEPATH")
            .map(|home| format!("{}{}", drive.to_string_lossy(), home.to_string_lossy()))
    });

    home
}

pub fn shorten_path(path: String) -> String {
    if let Some(home) = home_path() {
        let replaced_path = path.replace(&home, "~");

        if replaced_path.len() > 20 {
            format!("...{}", &replaced_path[replaced_path.len() - 17..])
        } else {
            replaced_path
        }
    } else {
        String::new()
    }
}

pub fn default_saving_path() -> String {
    let path = if let Some(home) = home_path() {
        let path = format!("{}/{}/", home, &app_name());
        DirBuilder::new()
            .recursive(true)
            .create(Path::new(&path))
            .expect("error creating directory.");
        path
    } else {
        String::from("./")
    };

    path.replace("/", std::path::MAIN_SEPARATOR_STR)
        .replace("\\", std::path::MAIN_SEPARATOR_STR)
}

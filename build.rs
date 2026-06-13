use std::path::PathBuf;

fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("resources/icons/icon.ico");
        res.compile().unwrap_or_default();

        let manifest_dir =
            PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR missing"));
        let ffmpeg_bin = manifest_dir.join("third_party/ffmpeg/bin");
        let ffmpeg_lib = manifest_dir.join("third_party/ffmpeg/lib");

        // Copy FFmpeg DLLs only for dynamic builds. Static builds must link
        // FFmpeg into the executable and should not ship runtime DLLs.
        if std::env::var("FFMPEG_STATIC").is_ok_and(|value| value != "0") {
            validate_static_ffmpeg_libs(&ffmpeg_lib);
            return;
        }

        let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
        // OUT_DIR è tipo target/debug/build/<crate>/out, risaliamo a target/debug/
        let target_dir = out_dir
            .ancestors()
            .nth(3)
            .expect("Cannot resolve target dir");

        if ffmpeg_bin.exists() {
            for entry in std::fs::read_dir(&ffmpeg_bin).expect("Cannot read ffmpeg bin dir") {
                let entry = entry.unwrap();
                let path = entry.path();
                if is_required_ffmpeg_dll(&path) {
                    let dest = target_dir.join(path.file_name().unwrap());
                    if !dest.exists() || file_modified(&path) > file_modified(&dest) {
                        std::fs::copy(&path, &dest).unwrap_or_else(|e| {
                            panic!("Failed to copy {:?} → {:?}: {}", path, dest, e)
                        });
                        println!("cargo:warning=Copied {} to output dir", path.display());
                    }
                }
            }
            // Indica a rustc dove trovare le librerie FFmpeg per il linking
            println!("cargo:rustc-link-search=native={}", ffmpeg_bin.display());
        } else {
            println!(
                "cargo:warning=FFmpeg DLLs not found at {}",
                ffmpeg_bin.display()
            );
        }

        // Riesegui build.rs se la cartella FFmpeg cambia
        println!("cargo:rerun-if-changed=third_party/ffmpeg/bin");
    }
}

#[cfg(target_os = "windows")]
fn file_modified(path: &PathBuf) -> std::time::SystemTime {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
}

#[cfg(target_os = "windows")]
fn is_required_ffmpeg_dll(path: &std::path::Path) -> bool {
    const REQUIRED_DLL_PREFIXES: &[&str] = &[
        "avcodec-",
        "avformat-",
        "avutil-",
        "swresample-",
        "swscale-",
    ];

    let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };

    let lower = file_name.to_ascii_lowercase();
    lower.ends_with(".dll")
        && REQUIRED_DLL_PREFIXES
            .iter()
            .any(|prefix| lower.starts_with(prefix))
}

#[cfg(target_os = "windows")]
fn validate_static_ffmpeg_libs(lib_dir: &std::path::Path) {
    const REQUIRED_LIBS: &[&str] = &[
        "avcodec.lib",
        "avformat.lib",
        "avutil.lib",
        "swresample.lib",
        "swscale.lib",
    ];

    let missing = REQUIRED_LIBS
        .iter()
        .filter(|lib| !lib_dir.join(lib).is_file())
        .copied()
        .collect::<Vec<_>>();

    if !missing.is_empty() {
        panic!(
            "FFMPEG_STATIC=1 requires static FFmpeg .lib files in {}. Missing: {}",
            lib_dir.display(),
            missing.join(", ")
        );
    }

    let import_defs = std::fs::read_dir(lib_dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("def"))
        })
        .collect::<Vec<_>>();

    if !import_defs.is_empty() {
        let names = import_defs
            .iter()
            .filter_map(|path| path.file_name().and_then(|name| name.to_str()))
            .collect::<Vec<_>>()
            .join(", ");

        panic!(
            "FFMPEG_STATIC=1 is enabled, but {} contains DLL import libraries ({names}). \
Use a static FFmpeg build: real static .lib files are much larger and must not depend on avcodec-*.dll, avformat-*.dll, avutil-*.dll, swresample-*.dll, or swscale-*.dll.",
            lib_dir.display()
        );
    }
}

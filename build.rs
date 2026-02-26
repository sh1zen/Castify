use std::path::PathBuf;

fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("resources/icons/icon.ico");
        res.compile().unwrap_or_default();

        // Copia le DLL di FFmpeg nella cartella di output
        let ffmpeg_bin = PathBuf::from("third_party/ffmpeg/bin");
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
    lower.ends_with(".dll") && REQUIRED_DLL_PREFIXES.iter().any(|prefix| lower.starts_with(prefix))
}

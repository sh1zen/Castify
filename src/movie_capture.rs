use gstreamer as gst;
use gstreamer::prelude::*;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub fn create_video_from_screenshots(width: u32, height: u32) {
    // Inizializza GStreamer
    gst::init().unwrap();

    // Definisci la directory degli screenshot e il nome del file di output
    let screenshot_dir = Path::new("target");
    let output_file = "output.mp4";

    // Stampa il percorso assoluto
    println!("Screenshot directory: {:?}", screenshot_dir.canonicalize().unwrap());

    // Crea un elemento pipeline
    let pipeline = gst::Pipeline::new(None);

    // Crea elementi utilizzando ElementFactory::make_with_name
    let filesrc = gst::ElementFactory::make_with_name("multifilesrc", Some("filesrc"))
        .expect("Failed to create element 'multifilesrc'");
    let pngdec = gst::ElementFactory::make_with_name("pngdec", Some("pngdec"))
        .expect("Failed to create element 'pngdec'");
    let videoconvert = gst::ElementFactory::make_with_name("videoconvert", Some("videoconvert"))
        .expect("Failed to create element 'videoconvert'");
    let videorate = gst::ElementFactory::make_with_name("videorate", Some("videorate"))
        .expect("Failed to create element 'videorate'");
    let queue = gst::ElementFactory::make_with_name("queue", Some("queue"))
        .expect("Failed to create element 'queue'");
    let capsfilter = gst::ElementFactory::make_with_name("capsfilter", Some("capsfilter"))
        .expect("Failed to create element 'capsfilter'");
    let identity = gst::ElementFactory::make_with_name("identity", Some("identity"))
        .expect("Failed to create element 'identity'");
    let x264enc = gst::ElementFactory::make_with_name("x264enc", Some("x264enc"))
        .expect("Failed to create element 'x264enc'");
    let mp4mux = gst::ElementFactory::make_with_name("mp4mux", Some("mp4mux"))
        .expect("Failed to create element 'mp4mux'");
    let filesink = gst::ElementFactory::make_with_name("filesink", Some("filesink"))
        .expect("Failed to create element 'filesink'");

    // Configura il filesrc per leggere i file dalla directory degli screenshot
    filesrc.set_property("location", &format!("{}/monitor-%05d.png", screenshot_dir.display()));
    filesrc.set_property("start-index", &0i32); // Cambiato a i32
    filesrc.set_property("stop-index", &-1i32); // Cambiato a i32

    // Configura i caps
    let caps = gst::Caps::builder("video/x-raw")
        .field("format", &"I420")
        .field("width", &(width as i32))
        .field("height", &(height as i32))
        .field("framerate", &gst::Fraction::new(2, 1)) // 2 frame al secondo
        .build();
    capsfilter.set_property("caps", &caps);

    // Imposta il percorso del file di output
    filesink.set_property("location", &output_file);

    // Aggiungi elementi alla pipeline
    pipeline
        .add_many(&[
            &filesrc,
            &pngdec,
            &videoconvert,
            &videorate,
            &queue,
            &capsfilter,
            &identity,
            &x264enc,
            &mp4mux,
            &filesink,
        ])
        .unwrap();

    // Collegamenti fissi
    gst::Element::link_many(&[
        &filesrc,
        &pngdec,
        &videoconvert,
        &videorate,
        &queue,
        &capsfilter,
        &identity,
        &x264enc,
        &mp4mux,
        &filesink,
    ])
        .expect("Failed to link elements");

    // Aggiungi callback di debug all'elemento identity
    let buffer_count = Arc::new(Mutex::new(0));
    let buffer_count_clone = Arc::clone(&buffer_count);

    identity.connect("handoff", false, move |values| {
        let buffer = values[1].get::<gst::Buffer>().expect("Failed to get buffer");
        println!("Buffer received with PTS: {:?}", buffer.pts());
        let mut count = buffer_count_clone.lock().unwrap();
        *count += 1;
        None
    });

    // Imposta la pipeline allo stato "Playing"
    pipeline.set_state(gst::State::Playing).expect("Failed to set pipeline to playing");

    // Ottieni il bus e attendi fino a quando la pipeline non emette un messaggio di fine o errore
    let bus = pipeline.bus().unwrap();
    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        match msg.view() {
            gst::MessageView::Eos(..) => {
                println!("Reached End of Stream");
                println!("Total buffers processed: {}", buffer_count.lock().unwrap());
                break;
            }
            gst::MessageView::Error(err) => {
                eprintln!(
                    "Error from {:?}: {} ({:?})",
                    err.src().map(|s| s.path_string()),
                    err.error(),
                    err.debug()
                );
                break;
            }
            gst::MessageView::Warning(warning) => {
                eprintln!(
                    "Warning from {:?}: {} ({:?})",
                    warning.src().map(|s| s.path_string()),
                    warning.error(),
                    warning.debug()
                );
            }
            _ => (),
        }
    }

    // Imposta la pipeline allo stato "Null"
    pipeline.set_state(gst::State::Null).unwrap();
}

use gstreamer as gst;
use gstreamer::prelude::*;
use std::path::Path;

pub fn create_video_from_screenshots() {
    // Inizializza GStreamer
    gst::init().unwrap();

    // Definisci la directory degli screenshot e il nome del file di output
    let screenshot_dir = Path::new("target");
    let output_file = "output.mp4";

    // Crea un elemento pipeline
    let pipeline = gst::Pipeline::new(None);

    // Crea elementi utilizzando ElementFactory::make_with_name
    let filesrc = gst::ElementFactory::make_with_name("multifilesrc", Some("filesrc"))
        .expect("Failed to create element 'multifilesrc'");
    let pngdec = gst::ElementFactory::make_with_name("pngdec", Some("pngdec"))
        .expect("Failed to create element 'pngdec'");
    let videoconvert = gst::ElementFactory::make_with_name("videoconvert", Some("videoconvert"))
        .expect("Failed to create element 'videoconvert'");
    let x264enc = gst::ElementFactory::make_with_name("x264enc", Some("x264enc"))
        .expect("Failed to create element 'x264enc'");
    let mp4mux = gst::ElementFactory::make_with_name("mp4mux", Some("mp4mux"))
        .expect("Failed to create element 'mp4mux'");
    let filesink = gst::ElementFactory::make_with_name("filesink", Some("filesink"))
        .expect("Failed to create element 'filesink'");

    // Configura il filesrc per leggere i file dalla directory degli screenshot
    filesrc.set_property("location", &format!("{}/monitor-%05d.png", screenshot_dir.display()));
    //filesrc.set_property("index", 0);

    //filesrc.set_property("stop-index", -1);  // Leggi tutti i file disponibili (DEBUG)


   // filesrc.set_property("caps", &gst::Caps::new_simple("image/png", &[]));

    // Crea e imposta i caps con il framerate
    let caps = gst::Caps::builder("image/png")
        .field("framerate", &gst::Fraction::new(24, 1))  // 2 frame al secondo
        .build();

    // Applica i caps al filesrc_capsfilter
    filesrc.set_property("caps", &caps);

    // Imposta il percorso del file di output
    filesink.set_property("location", &output_file);

    // Aggiungi elementi alla pipeline
    pipeline.add_many(&[&filesrc, &pngdec, &videoconvert, &x264enc, &mp4mux, &filesink]).unwrap();

    // Collegamenti fissi
    if !gst::Element::link_many(&[&filesrc, &pngdec, &videoconvert, &x264enc, &mp4mux, &filesink]).is_ok() {
        eprintln!("Failed to link elements");
        return;
    }

    // Imposta la pipeline allo stato "Playing"
    if pipeline.set_state(gst::State::Playing).is_err() {
        eprintln!("Failed to set pipeline to playing");
        return;
    }

    // Ottieni il bus e attendi fino a quando la pipeline non emette un messaggio di fine o errore
    let bus = pipeline.bus().unwrap();
    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        match msg.view() {
            gst::MessageView::Eos(..) => break,
            gst::MessageView::Error(err) => {
                eprintln!(
                    "Error from {:?}: {} ({:?})",
                    err.src().map(|s| s.path_string()),
                    err.error(),
                    err.debug()
                );
                break;
            }
            _ => (),
        }
    }

    // Imposta la pipeline allo stato "Null"
    pipeline.set_state(gst::State::Null).unwrap();
}

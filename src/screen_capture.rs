use std::error::Error;
use std::time::Instant;
use gstreamer as gst;
use gstreamer::prelude::*;
use tokio;

pub async fn capture_screenshots() -> Result<(), Box<dyn Error>> {
    gst::init()?;

    // Configura il nome del file video di output
    let filename = "output.mp4";

    // Crea la pipeline GStreamer per catturare il video
    let pipeline_description = format!(
        "videotestsrc ! videoconvert ! videoscale ! video/x-raw,width=640,height=480 ! x264enc ! mp4mux ! filesink location={}",
        filename
    );
    let pipeline = gst::parse_launch(&pipeline_description)?;

    // Avvia la pipeline
    match pipeline.set_state(gst::State::Playing) {
        Ok(_) => (),
        Err(e) => {
            eprintln!("Errore nel cambiare lo stato della pipeline a Playing: {:?}", e);
            return Err(Box::new(e));
        }
    }

    // Imposta un timer per la durata della registrazione (es. 10 secondi)
    let duration = std::time::Duration::from_secs(10);

    // Aspetta fino alla fine della registrazione
    tokio::time::sleep(duration).await;

    // Ferma la pipeline
    match pipeline.set_state(gst::State::Null) {
        Ok(_) => (),
        Err(e) => {
            eprintln!("Errore nel cambiare lo stato della pipeline a Null: {:?}", e);
            return Err(Box::new(e));
        }
    }

    // Stampare il tempo impiegato
    println!("Tempo impiegato: {:?}", Instant::now().elapsed());

    Ok(())
}


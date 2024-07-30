use rdev::{grab};
use std::time::Instant;
use rust_st::events;

#[tokio::main]
async fn main() -> std::io::Result<()>  {
    let start = Instant::now();

    // let (tx, mut rx) = mpsc::channel(1);

    let events = events::Events::init();

    // Start grabbing events; handle errors if any occur
    if let Err(error) = grab(move |e| events.handle(e)) {
        println!("Error: {error:?}");
    }

    println!("T elapsed: {:?}", start.elapsed());

    Ok(())
}
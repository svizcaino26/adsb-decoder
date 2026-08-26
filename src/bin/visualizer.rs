use std::fs::File;

use adsb_decoder::{
    message::Message,
    source::{FileSource, FrameSource, SourceError},
    tracker::AircraftTracker,
};
use macroquad::prelude::*;

#[macroquad::main("Visualizer")]
async fn main() {
    let mut source =
        FileSource::from_file(File::open("rawframes.txt").expect("Raw frames file not found"));

    let mut tracker = AircraftTracker::new();

    loop {
        clear_background(BLACK);

        let frame = source.next_frame();
        match frame {
            Ok(Some(frame)) => {
                let message = Message::try_from(&frame);

                match message {
                    Ok(msg) => match tracker.update(msg) {
                        Ok(_) => (),
                        Err(_) => (),
                    },
                    Err(_) => (),
                }
            }
            Ok(None) => (),
            Err(SourceError::Adsb(_)) => (),
            Err(SourceError::IO(_)) => (),
        }
        next_frame().await;
    }
}

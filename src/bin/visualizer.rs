use std::{fs::File, path::Path};

use adsb_decoder::{source::FileSource, tracker::AircraftTracker};
use macroquad::prelude::*;

#[macroquad::main("Visualizer")]
async fn main() {
    let source =
        FileSource::from_file(File::open("rawframes.txt").expect("Raw frames file not found"));

    let tracker = AircraftTracker::new();

    loop {
        clear_background(BLACK);
        next_frame().await;
    }
}

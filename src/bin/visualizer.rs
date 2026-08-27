use std::fs::File;

use adsb_decoder::{
    message::Message,
    source::{FileSource, FileSourceError, FrameSource},
    tracker::AircraftTracker,
};
use macroquad::prelude::*;

/// Interval for processing frames from source 1 every second.
const FRAME_INTERVAL: f32 = 1.0;
/// Interval for running `prune()` method on `AircraftTracker`.
const PRUNE_INTERVAL: f32 = 30.0;

#[macroquad::main("Visualizer")]
async fn main() {
    let mut frame_elapsed = 0.0;
    let mut prune_elapsed = 0.0;
    let mut delta_time;

    let mut source =
        FileSource::from_file(File::open("rawframes.txt").expect("Raw frames file not found"));

    let mut tracker = AircraftTracker::new();

    loop {
        clear_background(BLACK);
        delta_time = get_frame_time();
        frame_elapsed += delta_time;
        prune_elapsed += delta_time;

        if frame_elapsed >= FRAME_INTERVAL {
            frame_elapsed -= FRAME_INTERVAL;
            let frame = source.next_frame();

            match frame {
                Ok(Some(frame)) => eprintln!("{frame:?}"), //println!("{frame:?}"),
                Ok(None) => (),
                Err(FileSourceError::Io(e)) => eprint!("{e}"),
                Err(FileSourceError::Adsb(e)) => eprintln!("{e}"),
            }
        }

        if prune_elapsed >= PRUNE_INTERVAL {
            prune_elapsed -= PRUNE_INTERVAL;
            tracker.prune();
        }
        next_frame().await;
    }
}

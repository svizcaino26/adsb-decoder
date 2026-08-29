use std::{collections::HashMap, fs::File};

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

const AIRCRAFT_HEIGHT: f32 = 25.;
const AIRCRAFT_BASE: f32 = 22.;

#[derive(Default, Debug)]
struct Aircraft {
    pos: Vec2,
    rot: f32,
    vel: f32,
}

#[derive(Default, Debug)]
struct AircraftDisplay {
    aircraft: HashMap<IcaoAddress, Aircraft>,
}

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

        for (icao, aircraft) in tracker.iter() {
            if let (Some(velocity), Some(heading)) = (aircraft.velocity(), aircraft.heading()) {
                aircraft_display
                    .aircraft
                    .entry(*icao)
                    .and_modify(|v| {
                        v.rot = heading;
                        v.vel = velocity
                    })
                    .or_insert(Aircraft {
                        pos: Vec2::new(screen_width() / 2., screen_height() / 2.),
                        vel: velocity,
                        rot: heading,
                    });
            }
        }

        for (_, aircraft) in aircraft_display.aircraft.iter_mut() {
            let direction = Vec2::new(aircraft.rot.sin(), -aircraft.rot.cos());
            aircraft.pos += direction * aircraft.vel * SPEED_SCALE * get_frame_time();
            let v1 = rotate_point(vec2(0., -AIRCRAFT_HEIGHT / 2.), aircraft.rot) + aircraft.pos;
        next_frame().await;
    }
}

fn rotate_point(point: Vec2, rotation: f32) -> Vec2 {
    Vec2::new(
        point.x * rotation.cos() - point.y * rotation.sin(),
        point.x * rotation.sin() + point.y * rotation.cos(),
    )
}

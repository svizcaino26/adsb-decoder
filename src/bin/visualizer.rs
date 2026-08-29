use std::{collections::HashMap, fs::File};

use adsb_decoder::{
    frame::IcaoAddress,
    message::Message,
    source::{FileSource, FileSourceError, FrameSource},
    tracker::AircraftTracker,
};
use macroquad::prelude::*;

const SPEED_SCALE: f32 = 0.1;
/// Interval between processing frames from the source.
const FRAME_INTERVAL: f32 = 1.0;
/// Interval for running `prune()` method on `AircraftTracker`.
const PRUNE_INTERVAL: f32 = 30.0;

const AIRCRAFT_HEIGHT: f32 = 25.;
const AIRCRAFT_BASE: f32 = 22.;

/// Represents an aircraft for display in the visualizer.
#[derive(Default, Debug)]
struct Aircraft {
    pos: Vec2,
    rot: f32,
    vel: f32,
}

impl Aircraft {
    /// Creates an effect for aircraft going off-screen to
    /// wrap around the opposite side remaining visible.
    fn wrap_around(&mut self) {
        if self.pos.x > screen_width() {
            self.pos.x = 0.;
        } else if self.pos.x < 0. {
            self.pos.x = screen_width();
        }

        if self.pos.y > screen_height() {
            self.pos.y = 0.;
        } else if self.pos.y < 0. {
            self.pos.y = screen_height();
        }
    }
}

/// Keeps a map of aircraft to be rendered on screen.
#[derive(Default, Debug)]
struct AircraftDisplay {
    aircraft: HashMap<IcaoAddress, Aircraft>,
}

impl AircraftDisplay {
    /// Syncronizes the display map with the tracker
    /// removing expired aircraft from screen.
    fn prune(&mut self, tracker: &AircraftTracker) {
        self.aircraft.retain(|icao, _| tracker.get(*icao).is_some());
    }
}

#[macroquad::main("Visualizer")]
async fn main() {
    let mut aircraft_display = AircraftDisplay::default();
    let mut frame_elapsed = 0.0;
    let mut prune_elapsed = 0.0;
    let mut frame_dt;

    let mut source =
        FileSource::from_file(File::open("rawframes.txt").expect("Raw frames file not found"));

    let mut tracker = AircraftTracker::new();

    loop {
        clear_background(BLACK);
        frame_dt = get_frame_time();
        frame_elapsed += frame_dt;
        prune_elapsed += frame_dt;

        if frame_elapsed >= FRAME_INTERVAL {
            frame_elapsed -= FRAME_INTERVAL;
            let frame = source.next_frame();

            match frame {
                Ok(Some(frame)) => {
                    let message = Message::try_from(&frame);

                    match message {
                        Ok(msg) => {
                            if let Err(e) = tracker.update(msg) {
                                eprintln!("{e}");
                            }
                        }
                        Err(e) => eprintln!("{e}"),
                    }
                }
                Ok(None) => (),
                Err(FileSourceError::Io(e)) => eprint!("{e}"),
                Err(FileSourceError::Adsb(e)) => eprintln!("{e}"),
            }
        }

        if prune_elapsed >= PRUNE_INTERVAL {
            prune_elapsed -= PRUNE_INTERVAL;
            tracker.prune();
            aircraft_display.prune(&tracker);
        }

        for (icao, aircraft) in tracker.iter() {
            if let (Some(velocity), Some(heading)) = (aircraft.velocity(), aircraft.heading()) {
                aircraft_display
                    .aircraft
                    .entry(*icao)
                    .and_modify(|aircraft| {
                        aircraft.vel = velocity;
                        aircraft.rot = heading;
                    })
                    .or_insert_with(|| Aircraft {
                        pos: Vec2::new(screen_width() / 2., screen_height() / 2.),
                        vel: velocity,
                        rot: heading,
                    });
            }
        }

        for (_, aircraft) in &mut aircraft_display.aircraft {
            let direction = Vec2::new(aircraft.rot.sin(), -aircraft.rot.cos());
            aircraft.pos += direction * aircraft.vel * SPEED_SCALE * get_frame_time();
            aircraft.wrap_around();
            let v1 = rotate_point(vec2(0., -AIRCRAFT_HEIGHT / 2.), aircraft.rot) + aircraft.pos;

            let v2 = rotate_point(
                vec2(-AIRCRAFT_BASE / 2., AIRCRAFT_HEIGHT / 2.),
                aircraft.rot,
            ) + aircraft.pos;

            let v3 = rotate_point(vec2(AIRCRAFT_BASE / 2., AIRCRAFT_HEIGHT / 2.), aircraft.rot)
                + aircraft.pos;

            draw_triangle_lines(v1, v2, v3, 2., WHITE);
            draw_text(
                format!("{:.0} kts", aircraft.vel),
                AIRCRAFT_BASE.mul_add(2., aircraft.pos.x),
                AIRCRAFT_HEIGHT.mul_add(2., aircraft.pos.y),
                20.,
                WHITE,
            );
        }

        next_frame().await;
    }
}

fn rotate_point(point: Vec2, rotation: f32) -> Vec2 {
    Vec2::new(
        point.y.mul_add(-rotation.sin(), point.x * rotation.cos()),
        point.y.mul_add(rotation.cos(), point.x * rotation.sin()),
    )
}

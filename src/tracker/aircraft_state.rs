use std::time::{Duration, Instant};

use crate::message::{
    airborne_velocity::AirborneVelocity, aircraft_identification::AircraftIdentification, Message,
};

#[derive(Debug)]
pub struct AircraftState {
    identification: Option<AircraftIdentification>,
    velocity: Option<AirborneVelocity>,
    last_seen: Instant,
}

impl Default for AircraftState {
    fn default() -> Self {
        Self {
            identification: Option::default(),
            velocity: Option::default(),
            last_seen: Instant::now(),
        }
    }
}

impl AircraftState {
    pub fn time_since_last_seen(&self) -> Duration {
        self.last_seen.elapsed()
    }

    pub fn update(&mut self, msg: Message) {
        match msg {
            Message::AircraftIdentification(aircraft_identification) => {
                self.last_seen = Instant::now();
                self.identification = Some(aircraft_identification);
            }
            Message::AirborneVelocity(airborne_velocity) => {
                self.last_seen = Instant::now();
                self.velocity = Some(airborne_velocity);
            }
        }
    }
}

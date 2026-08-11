use std::time::{Duration, Instant};

use crate::message::{
    airborne_velocity::AirborneVelocity, aircraft_identification::AircraftIdentification,
};

#[derive(Debug)]
pub struct AircraftState {
    identification: Option<AircraftIdentification>,
    velocity: Option<AirborneVelocity>,
    last_seen: Instant,
}

impl AircraftState {
    pub fn time_since_last_seen(&self) -> Duration {
        self.last_seen.elapsed()
    }
}

use std::time::Instant;

use crate::message::{
    airborne_velocity::AirborneVelocity, aircraft_identification::AircraftIdentification,
};

#[derive(Debug)]
pub struct AircraftState {
    identification: Option<AircraftIdentification>,
    velocity: Option<AirborneVelocity>,
    last_seen: Instant,
}

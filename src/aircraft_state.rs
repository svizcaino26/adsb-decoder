use crate::message::{
    airborne_velocity::AirborneVelocity, aircraft_identification::AircraftIdentification,
};
use std::time::Instant;

pub struct AircraftState {
    identification: Option<AircraftIdentification>,
    velocity: Option<AirborneVelocity>,
    last_seen: Instant,
}

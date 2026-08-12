use std::time::{Duration, Instant};

use crate::message::{
    airborne_velocity::AirborneVelocity, aircraft_identification::AircraftIdentification, Message,
};

/// Represents the currently known state of a single aircraft.
///
/// The state is accumulated from the different ADS-B messages received from
/// the aircraft. Individual fields remain unavailable until the corresponding
/// message type has been received.
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
    /// Returns the amount of time elapsed since the aircraft was last observed.
    pub fn last_seen(&self) -> Instant {
        self.last_seen
    }

    /// Applies an ADS-B message to the aircraft's current state.
    ///
    /// Receiving a message also refreshes the timestamp used to determine
    /// whether the aircraft should be retained by the tracker.
    pub fn update(&mut self, msg: Message) {
        self.last_seen = Instant::now();
        match msg {
            Message::AircraftIdentification(aircraft_identification) => {
                self.identification = Some(aircraft_identification);
            }
            Message::AirborneVelocity(airborne_velocity) => {
                self.velocity = Some(airborne_velocity);
            }
        }
    }
}

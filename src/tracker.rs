use std::{collections::HashMap, time::Duration};

use crate::frame::IcaoAddress;
use crate::message::Message;

mod aircraft_state;

const RETAIN_THRESHOLD: Duration = Duration::from_mins(1);

use aircraft_state::AircraftState;

/// Maintains the current state of aircraft observed through ADS-B messages.
///
/// An [`AircraftTracker`] groups messages by their ICAO address and accumulates
/// the information reported by each aircraft over time.
///
/// Aircraft that have not been observed within [`RETAIN_THRESHOLD`] are
/// removed when [`AircraftTracker::prune`] is called.
pub struct AircraftTracker {
    aircraft: HashMap<IcaoAddress, AircraftState>,
}

impl AircraftTracker {
    /// Removes aircraft that have not been observed within the retention threshold.
    pub fn prune(&mut self) {
        self.aircraft
            .retain(|_, aircraft| aircraft.time_since_last_seen() <= RETAIN_THRESHOLD);
    }

    /// Updates the state of the aircraft associated with the given message.
    ///
    /// If the aircraft has not been seen before, a new state is created.
    /// Otherwise, the existing state is updated with the information contained
    /// in the message.
    pub fn update(&mut self, msg: Message) {
        let icao_address = match &msg {
            Message::AircraftIdentification(msg) => msg.icao,
            Message::AirborneVelocity(msg) => msg.icao,
        };

        self.aircraft.entry(icao_address).or_default().update(msg);
    }
}

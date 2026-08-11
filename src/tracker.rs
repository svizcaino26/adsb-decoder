use std::{collections::HashMap, time::Duration};

use crate::frame::IcaoAddress;
use crate::message::Message;

mod aircraft_state;

const RETAIN_THRESHOLD: Duration = Duration::from_mins(1);

use aircraft_state::AircraftState;

pub struct AircraftTracker {
    aircraft: HashMap<IcaoAddress, AircraftState>,
}

impl AircraftTracker {
    pub fn prune(&mut self) {
        self.aircraft
            .retain(|_, aircraft| aircraft.time_since_last_seen() <= RETAIN_THRESHOLD);
    }

    pub fn update(&mut self, msg: Message) {
        let icao_address = match &msg {
            Message::AircraftIdentification(msg) => msg.icao,
            Message::AirborneVelocity(msg) => msg.icao,
        };

        self.aircraft.entry(icao_address).or_default().update(msg);
    }
}

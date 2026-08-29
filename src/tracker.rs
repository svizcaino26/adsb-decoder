use std::time::Instant;
use std::{collections::HashMap, time::Duration};

use crate::error::AdsbError;
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
#[derive(Debug)]
pub struct AircraftTracker {
    aircraft: HashMap<IcaoAddress, AircraftState>,
}

impl AircraftTracker {
    #[must_use]
    pub fn new() -> Self {
        Self {
            aircraft: HashMap::new(),
        }
    }

    #[must_use]
    pub fn get(&self, icao: IcaoAddress) -> Option<&AircraftState> {
        self.aircraft.get(&icao)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&IcaoAddress, &AircraftState)> {
        self.aircraft.iter()
    }

    /// Removes aircraft that have not been observed within the retention threshold.
    /// Aircraft tracking expires when `last_seen + RETAIN_THRESHOLD` resolves to
    /// a time earlier than `now`.
    pub fn prune(&mut self) {
        let now = Instant::now();

        self.aircraft
            .retain(|_, aircraft| aircraft.last_seen() + RETAIN_THRESHOLD >= now);
    }

    /// Updates the state of the aircraft associated with the given message.
    ///
    /// If the aircraft has not been seen before, a new state is created.
    /// Otherwise, the existing state is updated with the information contained
    /// in the message.
    ///
    /// # Errors
    /// - If global position decoding from paired CPR messages fails.
    pub fn update(&mut self, msg: Message) -> Result<(), AdsbError> {
        let icao_address = match &msg {
            Message::AircraftIdentification(msg) => msg.icao,
            Message::AirborneVelocity(msg) => msg.icao,
            Message::AirbornePosition(aircraft_altitude, _) => aircraft_altitude.icao,
        };

        self.aircraft.entry(icao_address).or_default().update(msg)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::frame::RawFrame;

    use super::*;

    #[test]
    #[allow(clippy::unwrap_used)]
    fn update_creates_aircraft_state() {
        let frame = RawFrame::from_hex("8D4840D6202CC371C32CE0576098").unwrap();
        let message = Message::try_from(&frame).unwrap();

        let mut tracker = AircraftTracker::new();

        tracker.update(message).unwrap();

        assert_eq!(tracker.aircraft.len(), 1);
        assert!(tracker.aircraft.contains_key(&frame.icao()));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn update_accumulates_state_for_same_aircraft() {
        let even_frame = RawFrame::from_hex("8D40621D58C382D690C8AC2863A7").unwrap();
        let odd_frame = RawFrame::from_hex("8D40621D58C386435CC412692AD6").unwrap();

        let even = Message::try_from(&even_frame).unwrap();
        let odd = Message::try_from(&odd_frame).unwrap();

        let mut tracker = AircraftTracker::new();

        tracker.update(even).unwrap();
        tracker.update(odd).unwrap();

        assert_eq!(tracker.aircraft.len(), 1);

        let aircraft = tracker.aircraft.get(&even_frame.icao()).unwrap();

        assert!(aircraft.cpr_even.is_some());
        assert!(aircraft.cpr_odd.is_some());
        assert!(aircraft.airborne_position.is_some());
    }
}

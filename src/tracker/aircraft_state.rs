use std::time::{Duration, Instant};

use crate::{
    error::AdsbError,
    message::{
        airborne_position::{AircraftAltitude, Cpr, Even, Odd, Position},
        airborne_velocity::AirborneVelocity,
        aircraft_identification::AircraftIdentification,
        Message,
    },
};

const CPR_GLOBAL_DECODE_WINDOW: Duration = Duration::from_secs(10);

/// Represents the currently known state of a single aircraft.
///
/// The state is accumulated from the different ADS-B messages received from
/// the aircraft. Individual fields remain unavailable until the corresponding
/// message type has been received.
#[derive(Debug)]
pub struct AircraftState {
    identification: Option<AircraftIdentification>,
    velocity: Option<AirborneVelocity>,
    altitude: Option<AircraftAltitude>,
    cpr_even: Option<Even>,
    cpr_odd: Option<Odd>,
    airborne_position: Option<Position>,
    last_seen: Instant,
}

impl Default for AircraftState {
    fn default() -> Self {
        Self {
            identification: Option::default(),
            velocity: Option::default(),
            altitude: Option::default(),
            cpr_even: Option::default(),
            cpr_odd: Option::default(),
            airborne_position: Option::default(),
            last_seen: Instant::now(),
        }
    }
}

impl AircraftState {
    /// Returns the amount of time elapsed since the aircraft was last observed.
    pub const fn last_seen(&self) -> Instant {
        self.last_seen
    }

    /// Applies an ADS-B message to the aircraft's current state.
    ///
    /// Receiving a message also refreshes the timestamp used to determine
    /// whether the aircraft should be retained by the tracker.
    pub fn update(&mut self, msg: Message) -> Result<(), AdsbError> {
        match msg {
            Message::AircraftIdentification(aircraft_identification) => {
                self.identification = Some(aircraft_identification);
            }
            Message::AirborneVelocity(airborne_velocity) => {
                self.velocity = Some(airborne_velocity);
            }
            Message::AirbornePosition(aircraft_altitude, cpr) => {
                self.altitude = Some(aircraft_altitude);

                match cpr {
                    Cpr::Even(cpr_even) => self.cpr_even = Some(cpr_even),
                    Cpr::Odd(cpr_odd) => self.cpr_odd = Some(cpr_odd),
                }

                if let (Some(cpr_even), Some(cpr_odd)) = (&self.cpr_even, &self.cpr_odd) {
                    let time_delta = if cpr_even.time() >= cpr_odd.time() {
                        cpr_even.time().duration_since(cpr_odd.time())
                    } else {
                        cpr_odd.time().duration_since(cpr_even.time())
                    };

                    if time_delta <= CPR_GLOBAL_DECODE_WINDOW {
                        let position = Position::decode_global_position(cpr_even, cpr_odd)?;
                        self.airborne_position = Some(position);
                    }
                }
            }
        }

        self.last_seen = Instant::now();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{frame::RawFrame, message::Message};

    use super::*;

    #[test]
    #[allow(clippy::unwrap_used)]
    fn update_accumulates_aircraft_information() {
        let identification_frame = RawFrame::from_hex("8D4840D6202CC371C32CE0576098").unwrap();
        let velocity_frame = RawFrame::from_hex("8D485020994409940838175B284F").unwrap();

        let identification_msg = Message::try_from(&identification_frame).unwrap();
        let velocity_msg = Message::try_from(&velocity_frame).unwrap();

        let mut state = AircraftState::default();

        state.update(identification_msg).unwrap();
        state.update(velocity_msg).unwrap();

        assert!(state.identification.is_some());
        assert!(state.velocity.is_some());
    }
}

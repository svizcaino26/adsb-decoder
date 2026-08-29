use std::time::{Duration, Instant};

use crate::{
    error::AdsbError,
    message::{
        Message,
        airborne_position::{AircraftAltitude, Cpr, Even, Odd, Position},
        airborne_velocity::{AirborneVelocity, Velocity},
        aircraft_identification::AircraftIdentification,
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
    pub(crate) cpr_even: Option<Even>,
    pub(crate) cpr_odd: Option<Odd>,
    pub(crate) airborne_position: Option<Position>,
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

    pub fn position(&self) -> Option<&Position> {
        self.airborne_position.as_ref()
    }

    pub fn velocity(&self) -> Option<f32> {
        let airborne_velocity = self.velocity.as_ref()?;
        match &airborne_velocity.velocity {
            Velocity::GroundSpeed {
                east_west,
                north_south,
            } => {
                if let (Some(ew_speed), Some(ns_speed)) = (east_west.value(), north_south.value()) {
                    Some(f32::sqrt(
                        f32::from(ew_speed).powi(2) + f32::from(ns_speed).powi(2),
                    ))
                } else {
                    None
                }
            }
            Velocity::AirSpeed { airspeed, .. } => airspeed.value().map(f32::from),
        }
    }

    pub fn heading(&self) -> Option<f32> {
        let airborne_velocity = self.velocity.as_ref()?;
        match &airborne_velocity.velocity {
            Velocity::GroundSpeed {
                east_west,
                north_south,
            } => {
                if let (Some(ew_speed), Some(ns_speed)) = (east_west.value(), north_south.value()) {
                    let heading = f32::atan2(f32::from(ew_speed), f32::from(ns_speed)).to_degrees();

                    Some((heading + 360.0) % 360.0)
                } else {
                    None
                }
            }
            Velocity::AirSpeed { heading, .. } => heading.value().map(f32::from),
        }
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

    #[test]
    #[allow(clippy::unwrap_used, clippy::panic)]
    fn update_decodes_position_from_recent_cpr_pair() {
        let even_frame = RawFrame::from_hex("8D40621D58C382D690C8AC2863A7").unwrap();
        let odd_frame = RawFrame::from_hex("8D40621D58C386435CC412692AD6").unwrap();

        let odd = Message::try_from(&odd_frame).unwrap();
        let even = Message::try_from(&even_frame).unwrap();

        let mut state = AircraftState::default();

        state.update(even).unwrap();
        state.update(odd).unwrap();

        let position = state
            .airborne_position
            .expect("expected global decoded position");

        assert!(dbg!((dbg!(position.latitude()) - 52.257_202_148_437_5).abs()) < 0.000_000_1);
        assert!(dbg!((dbg!(position.longitude()) - 3.919_372_558_593_75).abs()) < 0.000_000_1);
    }
}

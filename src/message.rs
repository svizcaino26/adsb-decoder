use std::time::Instant;

use crate::{
    error::AdsbError,
    frame::RawFrame,
    message::airborne_position::{AircraftAltitude, Cpr},
};

pub mod airborne_position;
pub mod airborne_velocity;
pub mod aircraft_identification;

use airborne_velocity::AirborneVelocity;
use aircraft_identification::AircraftIdentification;

/// Represents a decoded ADS-B message.
///
/// Each variant contains the strongly typed representation of a
/// particular ADS-B message as defined by its Type Code (TC).
#[derive(Debug)]
pub enum Message {
    /// Aircraft Identification and Category (Type Codes 1–4).
    AircraftIdentification(AircraftIdentification),
    AirborneVelocity(AirborneVelocity),
    AirbornePosition(AircraftAltitude, Cpr),
}

impl TryFrom<&RawFrame> for Message {
    type Error = AdsbError;

    /// Decodes a [`RawFrame`] into the corresponding ADS-B message.
    ///
    /// The message type is determined from the frame's Type Code.
    ///
    /// # Errors
    ///
    /// Returns [`AdsbError::UnsupportedTypeCode`] if the Type Code
    /// has not yet been implemented.
    fn try_from(frame: &RawFrame) -> Result<Self, AdsbError> {
        match frame.type_code().value() {
            1..=4 => Ok(Self::AircraftIdentification(
                AircraftIdentification::try_from(frame)?,
            )),
            9..=18 | 20..=22 => {
                let time = Instant::now();

                Ok(Self::AirbornePosition(
                    AircraftAltitude::try_from(frame)?,
                    Cpr::try_from((frame, time))?,
                ))
            }
            19 => Ok(Self::AirborneVelocity(AirborneVelocity::try_from(frame)?)),
            tc => Err(AdsbError::UnsupportedTypeCode(tc)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        frame,
        message::{
            airborne_velocity::{
                AirSpeed, EastWestVelocity, GeometricAltitudeDelta, MagneticHeading,
                NorthSouthVelocity, Velocity, VerticalRate,
            },
            aircraft_identification::AircraftCategory,
        },
    };

    use super::*;
    use std::assert_matches;

    #[test]
    #[allow(clippy::unwrap_used, clippy::panic)]
    fn decode_aircraft_identification() {
        let frame = RawFrame::from_hex("8D4840D6202CC371C32CE0576098").unwrap();
        let message = Message::try_from(&frame).unwrap();

        let Message::AircraftIdentification(msg) = message else {
            panic!("expected Aircraft Identification ADS-B frame");
        };

        assert_eq!(msg.icao, frame::IcaoAddress::new(0x48_40D6));
        assert_eq!(msg.callsign, "KLM1023");
        assert_matches!(msg.category, AircraftCategory::NoCategoryInformation);
    }

    #[test]
    #[allow(clippy::unwrap_used, clippy::panic)]
    fn decode_ground_velocity() {
        let frame = RawFrame::from_hex("8D485020994409940838175B284F").unwrap();
        let message = Message::try_from(&frame).unwrap();

        let Message::AirborneVelocity(msg) = message else {
            panic!("expected Airborne Velocity ADS-B frame");
        };

        let Velocity::GroundSpeed {
            east_west,
            north_south,
        } = msg.velocity
        else {
            panic!("expected Ground Speed encoded message");
        };

        assert_eq!(
            (east_west, north_south),
            (EastWestVelocity::West(8), NorthSouthVelocity::South(159))
        );
        assert_eq!(msg.vertical_rate, VerticalRate::Descending(832));
        assert_eq!(msg.geo_minus_baro, GeometricAltitudeDelta::Above(550));
    }

    #[test]
    #[allow(clippy::unwrap_used, clippy::panic)]
    fn decode_air_velocity() {
        let frame = RawFrame::from_hex("8DA05F219B06B6AF189400CBC33F").unwrap();
        let message = Message::try_from(&frame).unwrap();

        let Message::AirborneVelocity(msg) = message else {
            panic!("expected Airborne Velocity ADS-B frame");
        };

        let Velocity::AirSpeed { heading, airspeed } = msg.velocity else {
            panic!("expected Air Speed encoded message");
        };

        assert_eq!(
            (heading, airspeed),
            (
                MagneticHeading::Available(243.984_375),
                AirSpeed::TrueAirSpeed(375)
            )
        );
        assert_eq!(msg.vertical_rate, VerticalRate::Descending(2304));
        assert_eq!(msg.geo_minus_baro, GeometricAltitudeDelta::Unavailable);
    }
}

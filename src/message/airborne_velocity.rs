//! Decodes ADS-B Airborne velocities messages (Type Code 19).
//!
//! Airbone velocity messages may contain the encoded aircraft
//! Ground Speed or Air Speed.
//!
//! Ground Speed is encoded as 2 components `east-west` velocity and
//! `north-south` velocity.
//!
//! Air Speed has an encoded speed and a heading value.
//!
//! The implementation follows the ADS-B specification described in:
//! - ICAO Annex 10, Volume IV
//! - Junzi Sun, *The 1090 Megahertz Riddle*
//!   <https://mode-s.org/1090mhz/content/ads-b/5-airborne-velocity.html>
use std::{fmt::Display, ops::RangeInclusive};

use crate::{
    error::AdsbError,
    frame::{IcaoAddress, RawFrame},
};

const FIELD_SUBTYPE: RangeInclusive<u8> = 38u8..=40u8;
const FIELD_INTENT_CHANGE: RangeInclusive<u8> = 41u8..=41u8;
const FIELD_IFR_CAPABILITY: RangeInclusive<u8> = 42u8..=42u8;
const EAST_WEST_VELOCITY_SIGN: RangeInclusive<u8> = 46u8..=46u8;
const FIELD_EAST_WEST_VELOCITY: RangeInclusive<u8> = 47u8..=56u8;
const NORTH_SOUTH_VELOCITY_SIGN: RangeInclusive<u8> = 57u8..=57u8;
const FIELD_NORTH_SOUTH_VELOCITY: RangeInclusive<u8> = 58u8..=67u8;
const MAGNETIC_HEADING_STATUS: RangeInclusive<u8> = 46u8..=46u8;
const DEGREES_PER_LSB: f64 = 360.0 / 1024.0;
const FIELD_MAGNETIC_HEADING: RangeInclusive<u8> = 47u8..=56u8;
const AIRSPEED_TYPE: RangeInclusive<u8> = 57u8..=57u8;
const FIELD_AIRSPEED: RangeInclusive<u8> = 58u8..=67u8;
const FIELD_VERTICAL_RATE_SIGN: RangeInclusive<u8> = 69u8..=69u8;
const FEET_PER_LSB: i16 = 64;
const FIELD_VERTICAL_RATE: RangeInclusive<u8> = 70u8..=78u8;
const GEO_ALTITUDE_DELTA_SIGN: RangeInclusive<u8> = 81u8..=81u8;
const FIELD_GEO_ALTITUDE_DELTA: RangeInclusive<u8> = 82..=88;
const GEO_ALTITUDE_DELTA_STEPS: i16 = 25;

/// Speed sub type `ST` is 3 bit encoded.
/// Determines wether the encoded speed is
/// Supersonic or Subsonic.
#[derive(Debug)]
pub enum SubType {
    GroundSubSonic,
    GroundSuperSonic,
    AirSubSonic,
    AirSuperSonic,
}

impl TryFrom<&RawFrame> for SubType {
    type Error = AdsbError;

    fn try_from(frame: &RawFrame) -> Result<Self, Self::Error> {
        let st: u8 = frame
            .bits(FIELD_SUBTYPE)?
            .try_into()
            .expect("3 bit encoded value fits in u8");

        match st {
            1 => Ok(Self::GroundSubSonic),
            2 => Ok(Self::GroundSuperSonic),
            3 => Ok(Self::AirSubSonic),
            4 => Ok(Self::AirSuperSonic),
            st => Err(AdsbError::InvalidVelocitySubType(st)),
        }
    }
}

impl SubType {
    /// The multiplier is a scaling factor used to
    /// calculate speed magnitudes.
    ///
    /// The spec defines this as `4x` the calculated
    /// magnitude.
    const fn multiplier(self) -> i16 {
        match self {
            Self::GroundSubSonic | Self::AirSubSonic => 1,
            Self::GroundSuperSonic | Self::AirSuperSonic => 4,
        }
    }
}

/// East-West component of an aircraft's ground velocity.
///
/// Positive values are represented as `East`bound.
/// Negative values are represented as `West`bound.
/// Unavailable variant means all10 bits of the field are zero.
#[derive(Debug, PartialEq, Eq)]
pub enum EastWestVelocity {
    East(i16),
    West(i16),
    Unavailable,
}

impl Display for EastWestVelocity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::East(value) => write!(f, "{value}"),
            Self::West(value) => write!(f, "-{value}"),
            Self::Unavailable => write!(f, "Unavailable"),
        }
    }
}

/// North-South component of an aircraft's ground velocity.
///
/// Positive values are represented as `North`bound.
/// Negative values are represented as `South`bound.
/// Unavailable variant means all 10 bits of the field are zero.
#[derive(Debug, PartialEq, Eq)]
pub enum NorthSouthVelocity {
    North(i16),
    South(i16),
    Unavailable,
}

impl Display for NorthSouthVelocity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::North(value) => write!(f, "{value}"),
            Self::South(value) => write!(f, "-{value}"),
            Self::Unavailable => write!(f, "Unavailable"),
        }
    }
}

/// Heading coponent of an aircraft's air velocity.
///
/// Variants are determined by bit 46 of the ADS-B frame.
///
/// When available a value between 0-360 degrees is encoded
/// with 0 meaning the aircraft is heading `North`.
#[derive(Debug, PartialEq)]
pub enum MagneticHeading {
    Available(f64),
    Unavailable,
}

impl Display for MagneticHeading {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Available(value) => write!(f, "{value:.2} degrees"),
            Self::Unavailable => write!(f, "Unavailable"),
        }
    }
}

/// ADS-B velocity messages with sub-type 3-4.
/// Broadcasted when the aircraft's ground speed cannot be obtained.
///
/// Unavailable means all 10 bits of the field are zero.
///
/// Variants `IndicatedAirSpeed` and `TrueAirSpeed` are determined
/// by bit 57 of the ADS-B frame.
///
/// More information at <https://pilotinstitute.com/airspeed-types>
#[derive(Debug, PartialEq, Eq)]
pub enum AirSpeed {
    IndicatedAirSpeed(i16),
    TrueAirSpeed(i16),
    Unavailable,
}

impl Display for AirSpeed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IndicatedAirSpeed(value) => write!(f, "{value} (IAS)"),
            Self::TrueAirSpeed(value) => write!(f, "{value} (TAS)"),
            Self::Unavailable => write!(f, "Unavailable"),
        }
    }
}

/// ADS-B velocity messages (Type Code 19) may encode Ground Speed,
/// or Air Speed when Ground Speed cannot be obtained.
///
/// Each variant is represented with different components.
#[derive(Debug)]
pub enum Velocity {
    GroundSpeed {
        east_west: EastWestVelocity,
        north_south: NorthSouthVelocity,
    },
    AirSpeed {
        heading: MagneticHeading,
        airspeed: AirSpeed,
    },
}

impl Velocity {
    const fn decode_speed_value(value: i16, multiplier: i16) -> i16 {
        multiplier * (value - 1)
    }

    const fn decode_magnetic_heading(value: f64) -> f64 {
        value * DEGREES_PER_LSB
    }
}

impl TryFrom<&RawFrame> for Velocity {
    type Error = AdsbError;

    fn try_from(frame: &RawFrame) -> Result<Self, Self::Error> {
        let st = SubType::try_from(frame)?;

        match st {
            SubType::GroundSubSonic | SubType::GroundSuperSonic => {
                let multiplier = st.multiplier();

                let east_west_vector: i16 = frame
                    .bits(FIELD_EAST_WEST_VELOCITY)?
                    .try_into()
                    .expect("10 bit encoded value fits in i16");

                let east_west_speed = if east_west_vector == 0 {
                    EastWestVelocity::Unavailable
                } else if frame.bits(EAST_WEST_VELOCITY_SIGN)? == 0 {
                    EastWestVelocity::East(Self::decode_speed_value(east_west_vector, multiplier))
                } else {
                    EastWestVelocity::West(Self::decode_speed_value(east_west_vector, multiplier))
                };

                let north_south_vector: i16 = frame
                    .bits(FIELD_NORTH_SOUTH_VELOCITY)?
                    .try_into()
                    .expect("10 bit encoded value fits in i16");

                let north_south_speed = if north_south_vector == 0 {
                    NorthSouthVelocity::Unavailable
                } else if frame.bits(NORTH_SOUTH_VELOCITY_SIGN)? == 0 {
                    NorthSouthVelocity::North(Self::decode_speed_value(
                        north_south_vector,
                        multiplier,
                    ))
                } else {
                    NorthSouthVelocity::South(Self::decode_speed_value(
                        north_south_vector,
                        multiplier,
                    ))
                };

                Ok(Self::GroundSpeed {
                    east_west: east_west_speed,
                    north_south: north_south_speed,
                })
            }
            SubType::AirSubSonic | SubType::AirSuperSonic => {
                let multiplier = st.multiplier();

                let heading = if frame.bits(MAGNETIC_HEADING_STATUS)? == 0 {
                    MagneticHeading::Unavailable
                } else {
                    // heading here is a 10 bit encoded value, will always fit in an f64 so the conversion is safe.
                    #[allow(clippy::as_conversions, clippy::cast_precision_loss)]
                    let heading = frame.bits(FIELD_MAGNETIC_HEADING)? as f64;
                    MagneticHeading::Available(Self::decode_magnetic_heading(heading))
                };

                let airspeed: i16 = frame
                    .bits(FIELD_AIRSPEED)?
                    .try_into()
                    .expect("10 bit encoded value");

                let airspeed = if airspeed == 0 {
                    AirSpeed::Unavailable
                } else if frame.bits(AIRSPEED_TYPE)? == 0 {
                    AirSpeed::IndicatedAirSpeed(Self::decode_speed_value(airspeed, multiplier))
                } else {
                    AirSpeed::TrueAirSpeed(Self::decode_speed_value(airspeed, multiplier))
                };

                Ok(Self::AirSpeed { heading, airspeed })
            }
        }
    }
}

/// Represents the ascending or descending speed of an
/// aircraft in feet/min.
///
/// Unavailable when all 8 bits of the field are zero.
#[derive(Debug, PartialEq, Eq)]
pub enum VerticalRate {
    Ascending(i16),
    Descending(i16),
    Unavailable,
}

impl TryFrom<&RawFrame> for VerticalRate {
    type Error = AdsbError;
    fn try_from(frame: &RawFrame) -> Result<Self, Self::Error> {
        let vertical_rate: i16 = frame
            .bits(FIELD_VERTICAL_RATE)?
            .try_into()
            .expect("8 bit encoded field fits in i16");

        if vertical_rate == 0 {
            Ok(Self::Unavailable)
        } else if frame.bits(FIELD_VERTICAL_RATE_SIGN)? == 0 {
            Ok(Self::Ascending(Self::decode_rate(vertical_rate)))
        } else {
            Ok(Self::Descending(Self::decode_rate(vertical_rate)))
        }
    }
}

impl VerticalRate {
    const fn decode_rate(vertical_rate: i16) -> i16 {
        FEET_PER_LSB * (vertical_rate - 1)
    }
}

impl Display for VerticalRate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ascending(value) => write!(f, "{value}"),
            Self::Descending(value) => write!(f, "-{value}"),
            Self::Unavailable => write!(f, "Unavailable"),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum GeometricAltitudeDelta {
    Above(i16),
    Below(i16),
    Unavailable,
}

impl TryFrom<&RawFrame> for GeometricAltitudeDelta {
    type Error = AdsbError;
    fn try_from(frame: &RawFrame) -> Result<Self, AdsbError> {
        let geo_altitude_delta: i16 = frame
            .bits(FIELD_GEO_ALTITUDE_DELTA)?
            .try_into()
            .expect("7 bit encoded field");

        if geo_altitude_delta == 0 {
            Ok(Self::Unavailable)
        } else if frame.bits(GEO_ALTITUDE_DELTA_SIGN)? == 0 {
            Ok(Self::Above(Self::decode_altitude_delta(geo_altitude_delta)))
        } else {
            Ok(Self::Below(Self::decode_altitude_delta(geo_altitude_delta)))
        }
    }
}

impl GeometricAltitudeDelta {
    const fn decode_altitude_delta(value: i16) -> i16 {
        (value - 1) * GEO_ALTITUDE_DELTA_STEPS
    }
}

impl Display for GeometricAltitudeDelta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Above(value) => write!(f, "{value}"),
            Self::Below(value) => write!(f, "-{value}"),
            Self::Unavailable => write!(f, "Unavailable"),
        }
    }
}

#[derive(Debug)]
pub struct AirborneVelocity {
    pub icao: IcaoAddress,
    pub intent_change: bool,
    pub ifr_capability: bool,
    pub velocity: Velocity,
    pub vertical_rate: VerticalRate,
    pub geo_minus_baro: GeometricAltitudeDelta,
}

impl TryFrom<&RawFrame> for AirborneVelocity {
    type Error = AdsbError;

    fn try_from(frame: &RawFrame) -> Result<Self, Self::Error> {
        let intent_change = frame.bits(FIELD_INTENT_CHANGE)? == 1;

        let ifr_capability = frame.bits(FIELD_IFR_CAPABILITY)? == 1;

        let velocity = Velocity::try_from(frame)?;

        let vertical_rate = VerticalRate::try_from(frame)?;

        let geo_minus_baro = GeometricAltitudeDelta::try_from(frame)?;
        Ok(Self {
            icao: frame.icao(),
            intent_change,
            ifr_capability,
            velocity,
            vertical_rate,
            geo_minus_baro,
        })
    }
}

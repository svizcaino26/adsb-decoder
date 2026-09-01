//! Decodes ADS-B Airborne Velocity messages (Type Code 19).
//!
//! Airborne Velocity messages encode either an aircraft's ground
//! velocity or airspeed, depending on the velocity subtype.
//!
//! Ground velocity is represented as independent east-west and
//! north-south velocity components.
//!
//! Airspeed messages contain a magnetic heading (when available)
//! together with either indicated or true airspeed.
//!
//! The implementation follows:
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
const DEGREES_PER_LSB: f32 = 360.0 / 1024.0;
const FIELD_MAGNETIC_HEADING: RangeInclusive<u8> = 47u8..=56u8;
const AIRSPEED_TYPE: RangeInclusive<u8> = 57u8..=57u8;
const FIELD_AIRSPEED: RangeInclusive<u8> = 58u8..=67u8;
const FIELD_VERTICAL_RATE_SIGN: RangeInclusive<u8> = 69u8..=69u8;
const FEET_PER_LSB: i16 = 64;
const FIELD_VERTICAL_RATE: RangeInclusive<u8> = 70u8..=78u8;
const GEO_ALTITUDE_DELTA_SIGN: RangeInclusive<u8> = 81u8..=81u8;
const FIELD_GEO_ALTITUDE_DELTA: RangeInclusive<u8> = 82..=88;
const GEO_ALTITUDE_DELTA_STEPS: i16 = 25;

/// Velocity subtype (`ST`) encoded in bits 38–40.
///
/// Determines whether the message contains ground speed or
/// airspeed information, and whether the encoded values use
/// subsonic or supersonic scaling.
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
        let st = frame.bits_as(FIELD_SUBTYPE)?;

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

/// East-west component of an aircraft's ground velocity.
///
/// `East` represents motion toward the east.
/// `West` represents motion toward the west.
///
/// `Unavailable` indicates that the velocity component is not
/// available (all encoded bits are zero).
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

impl EastWestVelocity {
    #[must_use]
    pub const fn value(&self) -> Option<i16> {
        match self {
            Self::East(value) | Self::West(value) => Some(*value),
            Self::Unavailable => None,
        }
    }
}

/// North-South component of an aircraft's ground velocity.
///
/// `North` represents motion toward the north.
/// `South` represents motion toward the south.
/// `Unavailable` indicates that the velocity component is not
/// available (all encoded bits are zero).
#[derive(Debug, PartialEq, Eq)]
pub enum NorthSouthVelocity {
    North(i16),
    South(i16),
    Unavailable,
}

impl NorthSouthVelocity {
    #[must_use]
    pub const fn value(&self) -> Option<i16> {
        match self {
            Self::North(value) | Self::South(value) => Some(*value),
            Self::Unavailable => None,
        }
    }
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

/// Aircraft magnetic heading.
///
/// `Available` contains the decoded magnetic heading in degrees.
/// `Unavailable` indicates that the transmitter did not provide
/// heading information.
#[derive(Debug, PartialEq)]
pub enum MagneticHeading {
    Available(f32),
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

impl MagneticHeading {
    #[must_use]
    pub fn value(&self) -> Option<f32> {
        match &self {
            Self::Available(value) => Some(f32::from(*value)),
            Self::Unavailable => None,
        }
    }
}

/// ADS-B velocity messages with sub-type 3-4.
/// Broadcasted when the aircraft's ground speed cannot be obtained.
///
/// Unavailable means all 10 bits of the field are zero.
///
/// Two possible variants are encoded `IndicatedAirSpeed` and `TrueAirSpeed`.
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

impl AirSpeed {
    #[must_use]
    pub const fn value(&self) -> Option<i16> {
        match self {
            Self::IndicatedAirSpeed(value) | Self::TrueAirSpeed(value) => Some(*value),
            Self::Unavailable => None,
        }
    }
}

/// ADS-B velocity messages (Type Code 19) may encode Ground Speed,
/// or Air Speed when Ground Speed cannot be obtained.
///
/// ADS-B Airborne Velocity messages encode either:
///
/// - ground speed, represented as east-west and north-south
///   velocity components, or
/// - airspeed, represented as magnetic heading and airspeed.
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

    const fn decode_magnetic_heading(value: f32) -> f32 {
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

                let east_west_vector = frame.bits_as(FIELD_EAST_WEST_VELOCITY)?;

                let east_west_speed = if east_west_vector == 0 {
                    EastWestVelocity::Unavailable
                } else if frame.bits(EAST_WEST_VELOCITY_SIGN)? == 0 {
                    EastWestVelocity::East(Self::decode_speed_value(east_west_vector, multiplier))
                } else {
                    EastWestVelocity::West(Self::decode_speed_value(east_west_vector, multiplier))
                };

                let north_south_vector = frame.bits_as(FIELD_NORTH_SOUTH_VELOCITY)?;

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
                    let heading = frame.bits(FIELD_MAGNETIC_HEADING)? as f32;
                    MagneticHeading::Available(Self::decode_magnetic_heading(heading))
                };

                let airspeed = frame.bits_as(FIELD_AIRSPEED)?;

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

/// Aircraft vertical rate in feet per minute.
///
/// `Ascending` and `Descending` contain the decoded climb or
/// descent rate.
///
/// `Unavailable` indicates that no vertical rate was transmitted.
#[derive(Debug, PartialEq, Eq)]
pub enum VerticalRate {
    Ascending(i16),
    Descending(i16),
    Unavailable,
}

impl TryFrom<&RawFrame> for VerticalRate {
    type Error = AdsbError;
    fn try_from(frame: &RawFrame) -> Result<Self, Self::Error> {
        let vertical_rate = frame.bits_as(FIELD_VERTICAL_RATE)?;

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

/// Difference between geometric (GNSS) altitude and barometric altitude.
///
/// `Above` indicates the geometric altitude is above the
/// barometric altitude.
///
/// `Below` indicates the geometric altitude is below the
/// barometric altitude.
///
/// `Unavailable` indicates that no altitude difference was
/// transmitted.
#[derive(Debug, PartialEq, Eq)]
pub enum GeometricAltitudeDelta {
    Above(i16),
    Below(i16),
    Unavailable,
}

impl TryFrom<&RawFrame> for GeometricAltitudeDelta {
    type Error = AdsbError;
    fn try_from(frame: &RawFrame) -> Result<Self, AdsbError> {
        let geo_altitude_delta = frame.bits_as(FIELD_GEO_ALTITUDE_DELTA)?;

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

/// Decoded ADS-B Airborne Velocity message (Type Code 19).
///
/// Contains the aircraft ICAO address together with the decoded
/// velocity information, vertical rate, and the difference
/// between geometric and barometric altitude.
#[derive(Debug)]
pub struct AirborneVelocity {
    /// Aircraft ICAO address.
    pub icao: IcaoAddress,

    /// Intent Change flag.
    pub intent_change: bool,

    /// IFR Capability flag.
    pub ifr_capability: bool,

    /// Aircraft velocity.
    pub velocity: Velocity,

    /// Aircraft climb/descent rate.
    pub vertical_rate: VerticalRate,

    /// Difference between geometric and barometric altitude.
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

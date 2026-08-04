use std::{fmt::Display, ops::RangeInclusive};

use crate::{
    error::AdsbError,
    frame::{IcaoAddress, RawFrame},
};

const FIELD_SUBTYPE: RangeInclusive<u8> = 38u8..=40u8;
const FIELD_EAST_WEST_VELOCITY: RangeInclusive<u8> = 47u8..=56u8;
const FIELD_NORTH_SOUTH_VELOCITY: RangeInclusive<u8> = 58u8..=67u8;
const FIELD_HDG: RangeInclusive<u8> = 47u8..=56u8;
const FIELD_AIRSPEED: RangeInclusive<u8> = 58u8..=67u8;
const FIELD_VERTICAL_RATE_SIGN: RangeInclusive<u8> = 69u8..=69u8;
const FIELD_VERTICAL_RATE: RangeInclusive<u8> = 70u8..=78u8;

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
            .expect("resulting value is 3 bit encoded");

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
    const fn multiplier(self) -> i16 {
        match self {
            Self::GroundSubSonic | Self::AirSubSonic => 1,
            Self::GroundSuperSonic | Self::AirSuperSonic => 4,
        }
    }
}

#[derive(Debug)]
pub enum Velocity {
    GroundSpeed {
        east_west: Option<i16>,
        north_south: Option<i16>,
    },
    AirSpeed {
        heading: Option<f64>,
        airspeed: Option<i16>,
        is_true_airspeed: bool,
    },
}

impl TryFrom<&RawFrame> for Velocity {
    type Error = AdsbError;

    fn try_from(frame: &RawFrame) -> Result<Self, Self::Error> {
        let st = SubType::try_from(frame)?;

        match st {
            SubType::GroundSubSonic | SubType::GroundSuperSonic => {
                let d_ew = if let Ok(0) = frame.bits(46..=46) {
                    1
                } else {
                    -1
                };
                let d_ns = if let Ok(0) = frame.bits(57..=57) {
                    1
                } else {
                    -1
                };
                let multiplier = st.multiplier();

                let v_ew: i16 = frame
                    .bits(FIELD_EAST_WEST_VELOCITY)?
                    .try_into()
                    .expect("10 bits encoded value");
                let v_x = if let 0 = v_ew {
                    None
                } else {
                    Some(d_ew * multiplier * (v_ew - 1))
                };

                let v_ns: i16 = frame
                    .bits(FIELD_NORTH_SOUTH_VELOCITY)?
                    .try_into()
                    .expect("10 bit encoded value");
                let v_y = if let 0 = v_ns {
                    None
                } else {
                    Some(d_ns * multiplier * (v_ns - 1))
                };

                Ok(Self::GroundSpeed {
                    east_west: v_x,
                    north_south: v_y,
                })
            }
            SubType::AirSubSonic | SubType::AirSuperSonic => {
                let is_true_airspeed = if let Ok(1) = frame.bits(57..=57) {
                    true
                } else {
                    false
                };
                let heading = if let Ok(0) = frame.bits(46..=46) {
                    None
                } else {
                    let hdg: i16 = frame
                        .bits(FIELD_HDG)?
                        .try_into()
                        .expect("10 bit encoded value");
                    let heading: f64 = hdg as f64 * 360.0 / 10244.0;
                    Some(heading)
                };

                let airspeed: i16 = frame
                    .bits(FIELD_AIRSPEED)?
                    .try_into()
                    .expect("10 bit encoded value");
                let airspeed = if let 0 = airspeed {
                    None
                } else {
                    let multiplier = st.multiplier();
                    let airspeed = multiplier * (airspeed - 1);
                    Some(airspeed)
                };

                Ok(Self::AirSpeed {
                    heading,
                    airspeed,
                    is_true_airspeed,
                })
            }
        }
    }
}

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
            .expect("8 bit encoded field");

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
        64 * (vertical_rate - 1)
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
pub enum GeometriAltitudeDelta {
    Above(i16),
    Below(i16),
    Unavailable,
}

impl TryFrom<&RawFrame> for GeometriAltitudeDelta {
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

impl GeometriAltitudeDelta {
    const fn decode_altitude_delta(value: i16) -> i16 {
        (value - 1) * GEO_ALTITUDE_DELTA_STEPS
    }
}

impl Display for GeometriAltitudeDelta {
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
    pub geo_minus_baro: GeometriAltitudeDelta,
}

impl TryFrom<&RawFrame> for AirborneVelocity {
    type Error = AdsbError;

    fn try_from(frame: &RawFrame) -> Result<Self, Self::Error> {
        let intent_change = frame.bits(41..=41)? == 1;

        let ifr_capability = frame.bits(42..=42)? == 1;

        let velocity = Velocity::try_from(frame)?;

        let vertical_rate = VerticalRate::try_from(frame)?;

        let geo_minus_baro = GeometriAltitudeDelta::try_from(frame)?;
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

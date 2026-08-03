use std::ops::RangeInclusive;

use crate::{
    error::AdsbError,
    frame::{IcaoAddress, RawFrame},
};

const FIELD_SUBTYPE: RangeInclusive<u8> = 38u8..=40u8;
const FIELD_EAST_WEST_VELOCITY: RangeInclusive<u8> = 47u8..=56u8;
const FIELD_NORTH_SOUTH_VELOCITY: RangeInclusive<u8> = 58u8..=67u8;
const FIELD_HDG: RangeInclusive<u8> = 47u8..=56u8;
const FIELD_AIRSPEED: RangeInclusive<u8> = 58u8..=67u8;

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
    fn multiplier(self) -> i16 {
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

#[derive(Debug)]
pub struct AirborneVelocity {
    pub icao: IcaoAddress,
    pub subtype: SubType,
    pub intent_change: bool,
    pub ifr_capability: bool,
    pub velocity: Velocity,
    pub vertical_rate: i16,
    pub geo_minus_baro: i16,
}

impl TryFrom<&RawFrame> for AirborneVelocity {
    type Error = AdsbError;

    fn try_from(frame: &RawFrame) -> Result<Self, Self::Error> {
        Ok(Self { icao: frame.icao() })
    }
}

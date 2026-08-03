use std::ops::RangeInclusive;

use crate::{
    error::AdsbError,
    frame::{IcaoAddress, RawFrame},
};

const FIELD_SUBTYPE: RangeInclusive<u8> = 38u8..=40u8;
// const EAST_WEST_VELOCITY_

#[derive(Debug)]
pub enum SubType {
    GroundSubSonic,
    GroundSuperSonic,
    AirSubSonic,
    AirSuperSonic,
    UnknownSubType,
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
        east_west: i16,
        north_south: i16,
    },
    AirSpeed {
        heading: f64,
        airspeed: u16,
        is_true_airspeed: bool,
    },
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

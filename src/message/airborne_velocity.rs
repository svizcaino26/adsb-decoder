use std::ops::RangeInclusive;

use crate::{
    error::AdsbError,
    frame::{IcaoAddress, RawFrame},
};

const FIELD_SUBTYPE: RangeInclusive<u8> = 38u8..=40u8;

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

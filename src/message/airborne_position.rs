use std::ops::RangeInclusive;

use crate::{
    error::AdsbError,
    frame::{IcaoAddress, RawFrame},
};

const BAROMETRIC_ALTITUDE_STEP: i32 = 25;
const BAROMETRIC_ALTITUDE_STEPS_ALT: i32 = 100;
const ALTITUDE_OFFSET_FT: i32 = 1000;
const FIELD_ENCODED_ALTITUDE: RangeInclusive<u8> = 41..=52;
const ALTITUDE_QBIT: RangeInclusive<u8> = 48..=48;

pub struct Feet(i32);

impl Feet {
    fn new(value: i32) -> Self {
        Self(value)
    }

    fn value(self) -> i32 {
        self.0
    }
}

pub struct Meters(i32);

pub enum Altitude {
    Barometric(Feet),
    Geometric(Meters),
    Unavailable,
}

impl Altitude {
    const fn decode_barometric_altitude(value: i32) -> Feet {
        Feet(value * BAROMETRIC_ALTITUDE_STEP - ALTITUDE_OFFSET_FT)
    }
}
}

pub struct AirbornePosition {
    icao_address: IcaoAddress,
    altitude: Altitude,
}

impl TryFrom<&RawFrame> for AirbornePosition {
    type Error = AdsbError;
    fn try_from(frame: &RawFrame) -> Result<Self, Self::Error> {
        let altitude = Altitude::try_from(frame)?;
        Ok(Self {
            icao_address: frame.icao(),
            altitude,
        })
    }
}

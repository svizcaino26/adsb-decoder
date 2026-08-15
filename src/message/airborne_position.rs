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

    const fn remove_qbit(value: i32) -> i32 {
        let right_bits = value & 0x0F;
        let left_bits = (value >> 5) & 0x7F;
        (left_bits << 4) | (right_bits)
    }
}

impl TryFrom<&RawFrame> for Altitude {
    type Error = AdsbError;

    fn try_from(frame: &RawFrame) -> Result<Self, Self::Error> {
        match frame.type_code().value() {
            9..=18 => {
                let encoded_altitude = frame.bits_as::<i32>(FIELD_ENCODED_ALTITUDE)?;
                if encoded_altitude == 0 {
                    Ok(Self::Unavailable)
                } else if frame.bits(ALTITUDE_QBIT)? == 1 {
                    let n = Self::remove_qbit(encoded_altitude);
                    Ok(Self::Barometric(Self::decode_barometric_altitude(n)))
                } else {
                }
            }
            20..=22 => Ok(Self::Geometric(777)),
            _ => Err(AdsbError::UnsupportedTypeCode(frame.type_code().value())),
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

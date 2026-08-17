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

    /// Reorders the 12 bit encoded altitude field into Gillham coded bits.
    ///
    /// Original order:
    /// `C1 A1 C2 A2 C4 A4 B1 D1 B2 D2 B4 D4`
    ///
    /// Reordered:
    /// `D1 D2 D4 A1 A2 A4 B1 B2 B4 C1 C2 C4`
    /// The Q bit `D1` is discarded.
    ///
    /// Two sets of values are returned.
    /// - `gc500` encodes the coarse altitude in 500 feet increments.
    /// - `gc100` encodes the fine altitude adjustment in 100 feet increments.
    ///
    /// Sources:
    /// - <https://aviation.stackexchange.com/questions/97210/adsb-df17-altitude-decoding>
    /// - <https://grokipedia.com/page/Gillham_code>
    const fn reorder_gillham_bits(bits: i32) -> (i32, i32) {
        let reordered = bits >> 7 & 0x01 // C4 LSB
            | bits >> 8 & 0x02  // C2
            | bits >> 9 & 0x04  // C1
            | bits << 2 & 0x08  // B4
            | bits << 1 & 0x10  // B2
            | bits & 0x20       // B1
            | bits & 0x40       // A4
            | bits >> 1 & 0x80  // A2
            | bits >> 2 & 0x100 // A1
            | bits << 9 & 0x200 // D4
            | bits << 8 & 0x400; // D2 MSB

        let (gc500, gc100) = ((reordered >> 3) & 0xFF, reordered & 0x07);

        (gc500, gc100)
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
                    let (gc500, gc100) = Self::reorder_gillham_bits(encoded_altitude);
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

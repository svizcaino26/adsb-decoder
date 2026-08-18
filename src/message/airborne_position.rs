use std::ops::RangeInclusive;

use crate::{
    error::AdsbError,
    frame::{IcaoAddress, RawFrame},
};

const BAROMETRIC_ALTITUDE_STEP: i32 = 25;
const COARSE_ALTITUDE_STEP: i32 = 500;
const ALTITUDE_OFFSET_FT: i32 = 1000;
const FIELD_ENCODED_ALTITUDE: RangeInclusive<u8> = 41..=52;
const ALTITUDE_QBIT: RangeInclusive<u8> = 48..=48;
const FIELD_CPR_FORMAT: RangeInclusive<u8> = 54..=54;
const FIELD_ENCODED_LATITUDE: RangeInclusive<u8> = 55..=71;
const FIELD_ENCODED_LONGITUDE: RangeInclusive<u8> = 72..=88;
const CPR_SCALE: f64 = 131_072.0;

#[derive(Debug)]
pub struct Feet(i32);

impl Feet {
    const fn new(value: i32) -> Self {
        Self(value)
    }

    const fn value(self) -> i32 {
        self.0
    }
}

#[derive(Debug)]
pub struct Meters(i32);

#[derive(Debug)]
pub enum Altitude {
    Barometric(Feet),
    Geometric(Meters),
    Unavailable,
}

impl Altitude {
    const fn decode_barometric_altitude(value: i32) -> Feet {
        Feet::new(value * BAROMETRIC_ALTITUDE_STEP - ALTITUDE_OFFSET_FT)
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

    const fn gray_to_binary(mut value: i32) -> i32 {
        value ^= value >> 1;
        value ^= value >> 2;
        value ^= value >> 4;

        value
    }

    const fn decode_coarse_altitude(b500: i32) -> i32 {
        (b500 - 2) * COARSE_ALTITUDE_STEP
    }

    const fn decode_fine_adjustment(b500: i32, gc100: i32) -> Result<i32, AdsbError> {
        let parity = b500 & 1;

        match (parity, gc100) {
            (0, 0b001) => Ok(100),
            (0, 0b011) => Ok(300),
            (0, 0b101) => Ok(500),
            (1, 0b000) => Ok(600),
            (1, 0b010) => Ok(0),
            (1, 0b100) => Ok(200),
            (1, 0b110) => Ok(400),
            _ => Err(AdsbError::InvalidGillhamCode),
        }
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
                    let b500 = Self::gray_to_binary(gc500);
                    let coarse_altitude = Self::decode_coarse_altitude(b500);
                    let fine_adjustment = Self::decode_fine_adjustment(b500, gc100)?;
                    Ok(Self::Barometric(Feet::new(
                        coarse_altitude + fine_adjustment,
                    )))
                }
            }
            20..=22 => {
                let encoded_altitude = frame.bits_as::<i32>(FIELD_ENCODED_ALTITUDE)?;
                if encoded_altitude == 0 {
                    Ok(Self::Unavailable)
                } else {
                    Ok(Self::Geometric(Meters(encoded_altitude)))
                }
            }
            _ => Err(AdsbError::UnsupportedTypeCode(frame.type_code().value())),
        }
    }
}

#[derive(Debug)]
pub struct Odd {
    lat_cpr: u32,
    lon_cpr: u32,
}

#[derive(Debug)]
pub struct Even {
    lat_cpr: u32,
    lon_cpr: u32,
}

#[derive(Debug)]
pub enum Cpr {
    Even(Even),
    Odd(Odd),
}

impl TryFrom<&RawFrame> for Cpr {
    type Error = AdsbError;

    fn try_from(frame: &RawFrame) -> Result<Self, Self::Error> {
        let lat_cpr = frame.bits_as::<u32>(FIELD_ENCODED_LATITUDE)?;
        let lon_cpr = frame.bits_as::<u32>(FIELD_ENCODED_LONGITUDE)?;
        match frame.bits_as::<u8>(FIELD_CPR_FORMAT)? {
            0 => Ok(Self::Even(Even { lat_cpr, lon_cpr })),
            1 => Ok(Self::Odd(Odd { lat_cpr, lon_cpr })),
            format => Err(AdsbError::InvalidCprFormat(format)),
        }
    }
}

#[derive(Debug)]
pub struct AirbornePosition {
    latitude: f64,
    longitude: f64,
}

impl AirbornePosition {
    const fn latitude_zone_index() {}
    const fn decode_global_position(odd: &Odd, even: &Even) -> Self {
        Self {
            latitude: 1.0,
            longitude: 1.0,
        }
    }
}

#[derive(Debug)]
pub struct AircraftAltitude {
    pub icao: IcaoAddress,
    altitude: Altitude,
}

impl TryFrom<&RawFrame> for AircraftAltitude {
    type Error = AdsbError;
    fn try_from(frame: &RawFrame) -> Result<Self, Self::Error> {
        let altitude = Altitude::try_from(frame)?;
        Ok(Self {
            icao: frame.icao(),
            altitude,
        })
    }
}

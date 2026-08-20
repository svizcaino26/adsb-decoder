use std::{f64::consts::PI, ops::RangeInclusive, time::Instant};

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
const NZ: f64 = 15.0;
const EVEN_LAT_ZONE_SIZE: f64 = 360.0 / 4.0 * NZ;
const ODD_LAT_ZONE_SIZE: f64 = 360.0 / (4.0 * NZ - 1.0);

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
    time: Instant,
}

#[derive(Debug)]
pub struct Even {
    lat_cpr: u32,
    lon_cpr: u32,
    time: Instant,
}

#[derive(Debug)]
pub enum Cpr {
    Even(Even),
    Odd(Odd),
}

impl TryFrom<(&RawFrame, Instant)> for Cpr {
    type Error = AdsbError;

    fn try_from((frame, time): (&RawFrame, Instant)) -> Result<Self, Self::Error> {
        let lat_cpr = frame.bits_as::<u32>(FIELD_ENCODED_LATITUDE)?;
        let lon_cpr = frame.bits_as::<u32>(FIELD_ENCODED_LONGITUDE)?;
        match frame.bits_as::<u8>(FIELD_CPR_FORMAT)? {
            0 => Ok(Self::Even(Even {
                lat_cpr,
                lon_cpr,
                time,
            })),
            1 => Ok(Self::Odd(Odd {
                lat_cpr,
                lon_cpr,
                time,
            })),
            format => Err(AdsbError::InvalidCprFormat(format)),
        }
    }
}

#[derive(Debug)]
pub struct Position {
    latitude: f64,
    longitude: f64,
}

impl Position {
    #[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
    fn latitude_zone_index(even: &Even, odd: &Odd) -> i32 {
        let lat_cpr_even = f64::from(even.lat_cpr) / CPR_SCALE;
        let lat_cpr_odd = f64::from(odd.lat_cpr) / CPR_SCALE;

        // f64::floor(59.0 * lat_cpr_even - 60.0 * lat_cpr_odd + 0.5) as i32
        f64::floor(60.0f64.mul_add(-lat_cpr_odd, 59.0 * lat_cpr_even) + 0.5) as i32
    }

    #[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
    fn longitude_zone_index(even: &Even, odd: &Odd, lat_record: f64) -> i32 {
        let lon_cpr_even = f64::from(even.lon_cpr) / CPR_SCALE;
        let lon_cpr_odd = f64::from(odd.lon_cpr) / CPR_SCALE;

        let nl_lat = f64::from(Self::longitude_zone_number(lat_record));

        f64::floor(lon_cpr_odd.mul_add(-nl_lat, lon_cpr_even * (nl_lat - 1.0)) + 0.5) as i32
    }

    /// Normalizes the latitude value in the range `[-90, +90]`
    fn normalize_latitude(lat: f64) -> f64 {
        if lat >= 270.0 {
            lat - 360.0
        } else {
            lat
        }
    }

    fn decode_latitude(even: &Even, odd: &Odd, j_index: i32) -> (f64, f64) {
        let lat_cpr_even = f64::from(even.lat_cpr) / CPR_SCALE;
        let lat_cpr_odd = f64::from(odd.lat_cpr) / CPR_SCALE;

        let lat_even = EVEN_LAT_ZONE_SIZE * (f64::from(j_index) % 60.0 + lat_cpr_even);
        let lat_odd = ODD_LAT_ZONE_SIZE * (f64::from(j_index) % 59.0 + lat_cpr_odd);

        (
            Self::normalize_latitude(lat_even),
            Self::normalize_latitude(lat_odd),
        )
    }

    #[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
    fn longitude_zone_number(lat: f64) -> i32 {
        match lat {
            0.0 => 59,
            87.0 | -87.0 => 2,
            lat if !(-87.0..=87.0).contains(&lat) => 1,
            lat => {
                let numerator = 1.0 - f64::cos(PI / (2.0 * NZ));
                let denominator = f64::cos(PI / 180.0 * lat).powi(2);

                f64::floor(2.0 * PI / f64::acos(1.0 - numerator / denominator)) as i32
            }
        }
    }

    fn is_same_latitude(lat_even: f64, lat_odd: f64) -> Result<(), AdsbError> {
        let nl_lat_even = Self::longitude_zone_number(lat_even);
        let nl_lat_odd = Self::longitude_zone_number(lat_odd);

        if nl_lat_even != nl_lat_odd {
            return Err(AdsbError::MismatchedLatitude);
        }

        Ok(())
    }

    fn select_latitude(lat_even: f64, lat_odd: f64, time_even: Instant, time_odd: Instant) -> f64 {
        if time_even >= time_odd {
            lat_even
        } else {
            lat_odd
        }
    }

    fn decode_longitude(even: &Even, odd: &Odd, m_index: i32, nl_lat: i32) -> (f64, f64) {
        let n_even = max(nl_lat, 1);
        let n_odd = max(nl_lat - 1, 1);

        let (dlon_even, dlon_odd) = (360.0 / f64::from(n_even), 360.0 / f64::from(n_odd));

        let lon_cpr_even = f64::from(even.lon_cpr) / CPR_SCALE;
        let lon_cpr_odd = f64::from(odd.lon_cpr) / CPR_SCALE;

        let lon_even = dlon_even * (f64::from(m_index % n_even) + lon_cpr_even);
        let lon_odd = dlon_odd * (f64::from(m_index % n_odd) + lon_cpr_odd);

        (
            Self::normalize_longitude(lon_even),
            Self::normalize_longitude(lon_odd),
        )
    }

    fn normalize_longitude(lon: f64) -> f64 {
        if lon >= 180.0 {
            lon - 360.0
        } else {
            lon
        }
    }

    fn decode_global_position(even: &Even, odd: &Odd) -> Result<Self, AdsbError> {
        let j_index = Self::latitude_zone_index(even, odd);
        let (lat_even, lat_odd) = Self::decode_latitude(even, odd, j_index);

        Self::is_same_latitude(lat_even, lat_odd)?;

        let latitude = Self::select_latitude(lat_even, lat_odd, even.time, odd.time);
        let nl_lat = Self::longitude_zone_number(latitude);
        let m_index = Self::longitude_zone_index(even, odd, nl_lat);
        let (lon_even, lon_odd) = Self::decode_longitude(even, odd, m_index, nl_lat);
        Ok(Self {
            latitude,
            longitude: 1.0,
        })
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

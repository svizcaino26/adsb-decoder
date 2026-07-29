//! Decodes ADS-B Aircraft Identification messages (Type Codes 1–4).
//!
//! Aircraft Identification messages contain the aircraft callsign (e.g.
//! `KLM1023`) together with an aircraft category.
//!
//! The implementation follows the ADS-B specification described in:
//! - ICAO Annex 10, Volume IV
//! - Junzi Sun, *The 1090 Megahertz Riddle*
//!   <https://mode-s.org/1090mhz/content/ads-b/2-identification.html>
use std::ops::RangeInclusive;

use crate::{
    error::AdsbError,
    frame::{IcaoAddress, RawFrame, TypeCode},
};

const CALLSIGN_BITS: u8 = 6;
const CHAR_MAP_LENGTH: usize = 1usize << CALLSIGN_BITS;
/// Six-bit character lookup table defined by the ADS-B specification.
///
/// Each six-bit symbol indexes directly into this table to recover one
/// callsign character.
const CHAR_MAP: &[u8; CHAR_MAP_LENGTH] =
    b"#ABCDEFGHIJKLMNOPQRSTUVWXYZ##### ###############0123456789######";
#[allow(clippy::as_conversions)]
const CALLSIGN_MASK: u64 = (CHAR_MAP_LENGTH as u64) - 1;
const FIELD_CALL_SIGN: RangeInclusive<u8> = 41u8..=88u8;
const FIELD_CATEGORY: RangeInclusive<u8> = 38u8..=40u8;

#[derive(Debug, PartialEq)]
/// Decoded ADS-B Aircraft Identification message.
pub struct AircraftIdentification {
    pub icao: IcaoAddress,
    pub callsign: String,
    pub category: AircraftCategory,
}

impl TryFrom<&RawFrame> for AircraftIdentification {
    type Error = AdsbError;

    fn try_from(frame: &RawFrame) -> Result<Self, Self::Error> {
        let type_code = frame.type_code();
        let category: u8 = frame
            .bits(FIELD_CATEGORY)?
            .try_into()
            .expect("resulting value is 3 bits");

        Ok(Self {
            icao: frame.icao(),
            callsign: decode_callsign(frame),
            category: AircraftCategory::try_from(AircraftCategoryCode {
                type_code,
                category,
            })?,
        })
    }
}

#[derive(Debug, PartialEq)]
/// Aircraft category decoded from the Type Code and category bits.
pub enum AircraftCategory {
    Reserved(AircraftCategoryCode),
    UnknownCategory(AircraftCategoryCode),
    NoCategoryInformation,

    // TC = 2
    SurfaceEmergencyVehicle,
    SurfaceServiceVehicle,
    GroundObstruction,

    // TC = 3
    Glider,
    LighterThanAir,
    Parachutist,
    Ultralight,
    UnmannedAerialVehicle,
    SpaceVehicle,

    // TC = 4
    Light,
    Medium1,
    Medium2,
    HighVortex,
    Heavy,
    HighPerformance,
    Rotorcraft,
}

#[derive(Debug, PartialEq)]
/// Raw `(type_code, category)` pair used to decode an `AircraftCategory`.
struct AircraftCategoryCode {
    type_code: TypeCode,
    category: u8,
}

impl TryFrom<AircraftCategoryCode> for AircraftCategory {
    type Error = AdsbError;

    fn try_from(code: AircraftCategoryCode) -> Result<Self, Self::Error> {
        match (code.type_code.value(), code.category) {
            (1, 1..=7) | (2, 4..=7) | (3, 5) => Ok(Self::Reserved(code)),
            (_, 0) => Ok(Self::NoCategoryInformation),
            (2, 1) => Ok(Self::SurfaceEmergencyVehicle),
            (2, 2) => Ok(Self::SurfaceServiceVehicle),
            (2, 3) => Ok(Self::GroundObstruction),
            (3, 1) => Ok(Self::Glider),
            (3, 2) => Ok(Self::LighterThanAir),
            (3, 3) => Ok(Self::Parachutist),
            (3, 4) => Ok(Self::Ultralight),
            (3, 6) => Ok(Self::UnmannedAerialVehicle),
            (3, 7) => Ok(Self::SpaceVehicle),
            (4, 1) => Ok(Self::Light),
            (4, 2) => Ok(Self::Medium1),
            (4, 3) => Ok(Self::Medium2),
            (4, 4) => Ok(Self::HighVortex),
            (4, 5) => Ok(Self::Heavy),
            (4, 6) => Ok(Self::HighPerformance),
            (4, 7) => Ok(Self::Rotorcraft),
            _ => Ok(Self::UnknownCategory(code)),
        }
    }
}

#[allow(clippy::as_conversions, clippy::indexing_slicing)]
/// Decodes the eight-character aircraft callsign from an ADS-B frame.
///
/// The callsign is encoded as eight six-bit symbols, each indexing into
/// the ADS-B character lookup table. Trailing spaces are removed.
fn decode_callsign(frame: &RawFrame) -> String {
    let mut callsign = String::with_capacity(8);
    let bits: u64 = frame
        .bits(FIELD_CALL_SIGN)
        .expect("Callsign range is always valid")
        .try_into()
        .expect("48 bit field fits in u64");

    for shift in (0..48).step_by(6).rev() {
        let index = ((bits >> shift) & CALLSIGN_MASK) as usize;
        callsign.push(char::from(CHAR_MAP[index]));
    }

    // this clears trailing spaces in place without
    // having to trim and then convert back to a String
    while callsign.ends_with(' ') {
        callsign.pop();
    }

    callsign
}

use std::ops::RangeInclusive;

use crate::{error::AdsbError, frame::RawFrame};

const CHAR_MAP: &[u8; 64] = b"#ABCDEFGHIJKLMNOPQRSTUVWXYZ##### ###############0123456789######";
const FIELD_CALL_SIGN: RangeInclusive<u8> = 38u8..=88u8;

pub(crate) struct AircraftIdentification {
    pub icao: u32,
    pub callsign: String,
    pub category: u8,
}
#[allow(clippy::expect_used, clippy::as_conversions, clippy::indexing_slicing)]
fn decode_callsign(frame: &RawFrame) -> String {
    let mut callsign = String::with_capacity(8);
    let mut bits: u64 = frame
        .bits(FIELD_CALL_SIGN)
        .expect("Callsign range is always valid")
        .try_into()
        .expect("48 bit field fits in u64");

    for shift in (0..48).step_by(6).rev() {
        let index = ((bits >> shift) & 0x3F) as usize;
        callsign.push(char::from(CHAR_MAP[index]));
    }

    callsign
}

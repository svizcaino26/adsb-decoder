use std::{num::ParseIntError, ops::RangeInclusive};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AdsbError {
    #[error("invalid hex string: {0}")]
    InvalidHex(#[from] ParseIntError),

    #[error("invalid frame length: expected 28 digits, got {0}")]
    InvalidLength(usize),

    #[error("not an ADS-B message: downlink format was {0}, expected 17")]
    NotAdsb(u8),

    #[error("Invalid bit range: {0:?}")]
    InvalidBitRange(RangeInclusive<u8>),

    #[error("Unsupported type code: {0}")]
    UnsupportedTypeCode(u8),

    #[error("Invalid velocity subtype: {0}")]
    InvalidVelocitySubType(u8),

    #[error("Failed to convert extracted bits to target type")]
    InvalidBitConversion,

    #[error("Invalid Gillham encoding")]
    InvalidGillhamCode,

    #[error("Invalid CPR format {0}")]
    InvalidCprFormat(u8),

    #[error("Mismatched latitudes")]
    MismatchedLatitude,
}

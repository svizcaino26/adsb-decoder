use std::num::ParseIntError;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AdsbError {
    #[error("invalid hex string: {0}")]
    InvalidHex(#[from] ParseIntError),

    #[error("invalid frame length: expected 28 digits, got {0}")]
    InvalidLength(usize),

    #[error("not an ADS-B message: downlink format was {0}, expected 17")]
    NotAdsb(u8),

    #[error("Invalid bit range start={start}, len={len}")]
    InvalidBitRange { start: u8, len: u8 },
}

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AdsbError {
    #[error("invalid hex string: {0}")]
    InvalidHex(#[from] hex::FromHexError),

    #[error("invalid frame length: expected 14 bytes, got {0}")]
    InvalidLength(usize),

    #[error("not an ADS-B message: downlink format was {0}, expected 17")]
    NotAdsb(u8),
}

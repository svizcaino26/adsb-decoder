use std::ops::RangeInclusive;

use crate::error::AdsbError;

const ADSB_FRAME_LENGTH: usize = 28;
const ADSB_FRAME_BITS: u8 = 112;

/// Below are common field constants to provide ergonomic field access.
/// Downlink Format (DF)
const FIELD_DOWNLINK_FORMAT: RangeInclusive<u8> = 1u8..=5u8;

/// Transponder Capability (CA)
const FIELD_CAPABILITY: RangeInclusive<u8> = 6u8..=8u8;

/// ICAO Aircraft Address
const FIELD_ICAO_ADDRESS: RangeInclusive<u8> = 9..=32;

/// TypeCode(TC)
const FIELD_TYPE_CODE: RangeInclusive<u8> = 33..=37;

/// Message (ME)
const FIELD_MESSAGE: RangeInclusive<u8> = 33..=88;

/// ADS-B frames are 112 bits.
/// Bit 1 is the most significant bit of the frame, following the ICAO specification.
/// Internally the frame is stored in the lower 112 bits of a `u128`.
/// # References
/// - ICAO Annex 10, Volume IV
/// - Junzi Sun, *The 1090 Megahertz Riddle* <https://mode-s.org/1090mhz/index.html>
#[derive(Debug)]
pub struct RawFrame {
    bits: u128,
}

impl RawFrame {
    /// Parses a 112-bit ADS-B frame from a hexadecimal string
    ///
    /// The input must contain exactly 28 hexadecimal characters.
    /// Optional dump1090 delimiters (`*` and `;`) are ignored.
    /// # Error
    /// - If the string is not 28 digits long.
    /// - If the string contains invalid hexadecimal characters.
    /// - If the first 5 most significant bits are not equal to 17 (Not an ADS-B message).
    pub fn from_hex(hex_str: &str) -> Result<Self, AdsbError> {
        let hex_str = hex_str.trim().trim_start_matches('*').trim_end_matches(';');

        let len = hex_str.len();
        if len != ADSB_FRAME_LENGTH {
            return Err(AdsbError::InvalidLength(len));
        }

        let bits = u128::from_str_radix(hex_str, 16)?;

        #[allow(clippy::expect_used)]
        let df = u8::try_from((bits >> 107) & 0x1F).expect("DF is always <= 31 after masking");
        if df != 17 {
            return Err(AdsbError::NotAdsb(df));
        }

        Ok(Self { bits })
    }

    /// Extracts the field defined by the given inclusive bit range.
    ///
    /// Bit numbering follows the ICAO ADS-B specification:
    /// - Bit 1 is the most significant bit of the 112-bit frame.
    /// - Bit 112 is the least significant bit.
    /// - The range must be between 1 and 112 (included)
    ///
    /// # Examples
    ///
    /// ```text
    /// bits(1..=5)   -> Downlink Format (DF)
    /// bits(6..=8)   -> Capability (CA)
    /// bits(9..=32)  -> ICAO address
    /// bits(33..=37)  -> Type Code
    /// ```
    pub const fn bits(&self, range: RangeInclusive<u8>) -> Result<u128, AdsbError> {
        let start = *range.start();
        let end = *range.end();

        if start == 0 || start > end || end > ADSB_FRAME_BITS {
            return Err(AdsbError::InvalidBitRange(range));
        }

        let len = end - start + 1;
        let shift = ADSB_FRAME_BITS - end;
        let mask = (1u128 << len) - 1;
        Ok((self.bits >> shift) & mask)
    }

    pub fn icao(&self) -> u32 {
        self.bits(FIELD_ICAO_ADDRESS)
            .expect("ICAO range is always valid")
            .try_into()
            .expect("24 bits always fit into u32")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::assert_matches;

    #[test]
    fn test_valid_frame_parsed() {
        let frame = RawFrame::from_hex("8D4840D6202CC371C32CE0576098");
        assert!(frame.is_ok());
    }

    #[test]
    fn test_non_adsb_df_rejected() {
        let result = RawFrame::from_hex("284840D6202CC371C32CE0576022");
        assert_matches!(result, Err(AdsbError::NotAdsb(_)));
    }

    #[test]
    fn test_invalid_hex_rejected() {
        let result = RawFrame::from_hex("ZZZZZZZZZZZZZZZZZZZZZZZZZZZZ");
        assert_matches!(result, Err(AdsbError::InvalidHex(_)));
    }

    #[test]
    fn test_wrong_length_rejected() {
        let result = RawFrame::from_hex("8D4840D6");
        assert_matches!(result, Err(AdsbError::InvalidLength(_)));
    }

    #[test]
    #[allow(clippy::unwrap_used, clippy::panic)]
    fn test_extract_bits() {
        let Ok(frame) = RawFrame::from_hex("8D4840D6202CC371C32CE0576098") else {
            panic!("failed to parse frame");
        };

        assert_eq!(frame.bits(FIELD_DOWNLINK_FORMAT).unwrap(), 17); // DF
        assert_eq!(frame.bits(FIELD_CAPABILITY).unwrap(), 5); // CA
        assert_eq!(frame.bits(FIELD_ICAO_ADDRESS).unwrap(), 0x0048_40D6); // ICAO
        assert_eq!(frame.bits(FIELD_TYPE_CODE).unwrap(), 4); // Type Code
        assert_eq!(frame.bits(FIELD_MESSAGE).unwrap(), 0x0020_2CC3_71C3_2CE0); // Payload
    }

    #[test]
    #[allow(clippy::unwrap_used, clippy::panic)]
    fn test_invalid_bit_range() {
        let frame = RawFrame::from_hex("8D4840D6202CC371C32CE0576098").unwrap();

        assert_matches!(frame.bits(0..=5), Err(AdsbError::InvalidBitRange { .. }));

        assert_matches!(frame.bits(1..=113), Err(AdsbError::InvalidBitRange { .. }));
    }
}

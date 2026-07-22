use crate::error::AdsbError;

const ADSB_FRAME_LENGTH: usize = 28;

/// ADS-B frames are 112 bits.
/// Bit 1 is the most significant bit of the frame, following the ICAO specification.
/// Internally the frame is stored in the lower 112 bits of a `u128`.
#[derive(Debug)]
pub struct RawFrame {
    bits: u128,
}

impl RawFrame {
    pub fn from_hex(hex_str: &str) -> Result<Self, AdsbError> {
        let hex_str = hex_str.trim().trim_start_matches('*').trim_end_matches(';');

        let len = hex_str.len();
        if len != ADSB_FRAME_LENGTH {
            return Err(AdsbError::InvalidLength(len));
        }

        let bits = u128::from_str_radix(hex_str, 16)?;

        let df = (bits >> 107) & 0x1F;
        if df != 17 {
            return Err(AdsbError::NotAdsb(df));
        }

        Ok(Self { bits })
    }

    pub const fn capability(&self) -> u8 {
        // CA is the transponder capabilities corresponds to bits 6-8
        self.bytes[0] & 0x07
    }

    pub fn icao(&self) -> u32 {
        (u32::from(self.bytes[1]) << 16)
            | (u32::from(self.bytes[2]) << 8)
            | u32::from(self.bytes[3])
    }

    pub const fn type_code(&self) -> u8 {
        (self.bytes[4] >> 3) & 0x1F
    }

    pub fn payload(&self) -> &[u8] {
        &self.bytes[4..11]
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
    fn test_transponder_capability_parsed() {
        let Ok(frame) = RawFrame::from_hex("8D4840D6202CC371C32CE0576098") else {
            return;
        };
        // 5 means a Level 2+ transponder,
        // with ability to set CA to 7,
        // airborne
        assert_eq!(frame.capability(), 5);
    }

    #[test]
    fn test_parse_icao_address() {
        let Ok(frame) = RawFrame::from_hex("8D4840D6202CC371C32CE0576098") else {
            return;
        };
        assert_eq!(frame.icao(), 0x0048_40D6);
    }

    #[test]
    fn test_valid_type_code() {
        let Ok(frame) = RawFrame::from_hex("8D4840D6202CC371C32CE0576098") else {
            return;
        };
        assert_eq!(frame.type_code(), 4);
    }

    #[test]
    fn test_extract_payload() {
        let Ok(frame) = RawFrame::from_hex("8D4840D6202CC371C32CE0576098") else {
            return;
        };
        assert_matches!(frame.payload(), &[0x20, 0x2C, 0xC3, 0x71, 0xC3, 0x2C, 0xE0]);
    }
}

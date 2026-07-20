use crate::error::AdsbError;

const ADSB_FRAME_BYTES: usize = 14;

pub struct RawFrame {
    pub bytes: [u8; ADSB_FRAME_BYTES],
}

impl RawFrame {
    pub fn from_hex(hex_str: &str) -> Result<Self, AdsbError> {
        let hex_str = hex_str.trim().trim_start_matches('*').trim_end_matches(';');

        let bytes_vec = hex::decode(hex_str)?;

        let len = bytes_vec.len();
        if len != ADSB_FRAME_BYTES {
            return Err(AdsbError::InvalidLength(len));
        }

        // avoiding slice indexing
        let df = bytes_vec.first().map_or(0, |n| n >> 3);
        if df != 17 {
            return Err(AdsbError::NotAdsb(df));
        }

        let mut bytes = [0u8; ADSB_FRAME_BYTES];
        bytes.copy_from_slice(&bytes_vec);

        Ok(Self { bytes })
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
        assert!(matches!(result, Err(AdsbError::NotAdsb(_))));
    }

    #[test]
    fn test_invalid_hex_rejected() {
        let result = RawFrame::from_hex("ZZZZZZZZZZZZZZZZZZZZZZZZZZZZ");
        assert!(matches!(result, Err(AdsbError::InvalidHex(_))));
    }

    #[test]
    fn test_wrong_length_rejected() {
        let result = RawFrame::from_hex("8D4840D6");
        assert!(matches!(result, Err(AdsbError::InvalidLength(_))));
    }

    #[test]
    fn test_transponder_capability_parsed() {
        let Ok(frame) = RawFrame::from_hex("8D4840D6202CC371C32CE0576098") else {
            return;
        };
        // 5 means a Level 2+ transponder,
        // with ability to set CA to 7,
        // airborne
        assert!(matches!(frame.capability(), 5));
    }

    #[test]
    fn test_parse_icao_address() {
        let Ok(frame) = RawFrame::from_hex("8D4840D6202CC371C32CE0576098") else {
            return;
        };
        assert_matches!(frame.icao(), 0x0048_40D6);
    }

    #[test]
    fn test_valid_type_code() {
        let Ok(frame) = RawFrame::from_hex("8D4840D6202CC371C32CE0576098") else {
            return;
        };
        assert_matches!(frame.type_code(), 4);
    }
}

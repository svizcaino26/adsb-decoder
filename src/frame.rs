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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}

use crate::{error::AdsbError, frame::RawFrame};

mod aircraft_identification;

use aircraft_identification::{AircraftCategory, AircraftIdentification};

enum Message {
    AircraftIdentification(AircraftIdentification),
}

impl TryFrom<&RawFrame> for Message {
    type Error = AdsbError;

    fn try_from(frame: &RawFrame) -> Result<Self, AdsbError> {
        match frame.type_code() {
            1..=4 => Ok(Self::AircraftIdentification(
                AircraftIdentification::try_from(frame)?,
            )),
            tc => Err(AdsbError::UnsupportedTypeCode(tc)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::assert_matches;

    #[test]
    #[allow(clippy::unwrap_used)]
    fn decode_callsign() {
        let frame = RawFrame::from_hex("8D4840D6202CC371C32CE0576098").unwrap();
        let message = Message::try_from(&frame).unwrap();

        match message {
            Message::AircraftIdentification(msg) => {
                assert_eq!(msg.icao, 0x48_40D6);
                assert_eq!(msg.callsign, "KLM1023");
                assert_matches!(msg.category, AircraftCategory::NoCategoryInformation);
            }
        }
    }
}

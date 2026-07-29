use crate::{error::AdsbError, frame::RawFrame};

mod aircraft_identification;

use aircraft_identification::{AircraftCategory, AircraftIdentification};

#[derive(Debug)]
/// Represents a decoded ADS-B message.
///
/// Each variant contains the strongly typed representation of a
/// particular ADS-B message as defined by its Type Code (TC).
enum Message {
    /// Aircraft Identification and Category (Type Codes 1–4).
    AircraftIdentification(AircraftIdentification),
}

impl TryFrom<&RawFrame> for Message {
    type Error = AdsbError;

    /// Decodes a [`RawFrame`] into the corresponding ADS-B message.
    ///
    /// The message type is determined from the frame's Type Code.
    ///
    /// # Errors
    ///
    /// Returns [`AdsbError::UnsupportedTypeCode`] if the Type Code
    /// has not yet been implemented.
    fn try_from(frame: &RawFrame) -> Result<Self, AdsbError> {
        match frame.type_code().value() {
            1..=4 => Ok(Self::AircraftIdentification(
                AircraftIdentification::try_from(frame)?,
            )),
            tc => Err(AdsbError::UnsupportedTypeCode(tc)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::frame;

    use super::*;
    use std::assert_matches;

    #[test]
    #[allow(clippy::unwrap_used)]
    fn decode_aircraft_identification() {
        let frame = RawFrame::from_hex("8D4840D6202CC371C32CE0576098").unwrap();
        let message = Message::try_from(&frame).unwrap();

        match message {
            Message::AircraftIdentification(msg) => {
                assert_eq!(msg.icao, frame::IcaoAddress::new(0x48_40D6));
                assert_eq!(msg.callsign, "KLM1023");
                assert_matches!(msg.category, AircraftCategory::NoCategoryInformation);
            }
        }
    }
}

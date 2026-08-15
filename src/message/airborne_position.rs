use crate::frame::IcaoAddress;

pub enum Altitude {
    Barometric(u16),
    Geometric(u16),
    Unavailable(u16),
}

pub struct AirbornePosition {
    icao_address: IcaoAddress,
    altitude: Altitude,
}

impl TryFrom<&RawFrame> for AirbornePosition {
    type Error = AdsbError;
    fn try_from(frame: &RawFrame) -> Result<Self, Self::Error> {
        let altitude = Altitude::try_from(frame)?;
        Ok(Self {
            icao_address: frame.icao(),
            altitude,
        })
    }
}

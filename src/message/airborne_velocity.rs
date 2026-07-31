use crate::{
    error::AdsbError,
    frame::{IcaoAddress, RawFrame},
};

#[derive(Debug)]
pub struct AirborneVelocity {
    pub icao: IcaoAddress,
}

impl TryFrom<&RawFrame> for AirborneVelocity {
    type Error = AdsbError;

    fn try_from(frame: &RawFrame) -> Result<Self, Self::Error> {
        Ok(Self { icao: frame.icao() })
    }
}

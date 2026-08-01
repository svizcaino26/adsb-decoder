use crate::{
    error::AdsbError,
    frame::{IcaoAddress, RawFrame},
};

#[derive(Debug)]
pub struct AirborneVelocity {
    pub icao: IcaoAddress,
    pub intent_change: bool,
    pub ifr_capability: bool,
    pub velocity: Velocity,
    pub vertical_rate: i16,
    pub geo_minus_baro: i16,
}

impl TryFrom<&RawFrame> for AirborneVelocity {
    type Error = AdsbError;

    fn try_from(frame: &RawFrame) -> Result<Self, Self::Error> {
        Ok(Self { icao: frame.icao() })
    }
}

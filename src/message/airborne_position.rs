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

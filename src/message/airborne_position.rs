use crate::frame::IcaoAddress;

pub struct Feet(i32);

impl Feet {
    fn new(value: i32) -> Self {
        Self(value)
    }

    fn value(self) -> i32 {
        self.0
    }
}

pub struct Meters(i32);

pub enum Altitude {
    Barometric(Feet),
    Geometric(Meters),
    Unavailable,
}

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

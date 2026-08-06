use std::collections::HashMap;

use crate::frame::IcaoAddress;

mod aircraft_state;

use aircraft_state::AircraftState;

pub struct AircraftTracker {
    aircraft: HashMap<IcaoAddress, AircraftState>,
}

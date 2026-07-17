const ADSB_FRAME_BYTES: usize = 14;

pub struct RawFrame {
    pub bytes: [u8; ADSB_FRAME_BYTES],
}

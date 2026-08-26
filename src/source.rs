pub trait FrameSource {
    type Error;

    fn next_frame(&mut self) -> Result<Option<RawFrame>, Self::Error>;
}

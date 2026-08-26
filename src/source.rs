use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use crate::{error::AdsbError, frame::RawFrame};
use thiserror::Error;

pub trait FrameSource {
    type Error;

    fn next_frame(&mut self) -> Result<Option<RawFrame>, Self::Error>;
}

pub struct FileSource {
    reader: BufReader<File>,
}

impl FileSource {
    pub fn from_file(file: File) -> Self {
        Self {
            reader: BufReader::new(file),
        }
    }
}

impl FrameSource for FileSource {
    type Error = SourceError;

    fn next_frame(&mut self) -> Result<Option<RawFrame>, Self::Error> {
        let mut line = String::new();

        let bytes_read = self.reader.read_line(&mut line)?;

        if bytes_read == 0 {
            return Ok(None);
        }

        Ok(Some(RawFrame::from_hex(&line)?))
    }
}

#[derive(Debug, Error)]
pub enum SourceError {
    #[error("Failed to read frame source")]
    IO(#[from] std::io::Error),
    #[error("Failed to parse ADS-B frame")]
    Adsb(#[from] AdsbError),
}

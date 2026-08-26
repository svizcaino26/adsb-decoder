use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use crate::{error::AdsbError, frame::RawFrame};
use thiserror::Error;

pub trait FrameSource {
    type Error;

    /// Produces a `RawFrame` from a data source.
    ///
    /// # Errors
    /// - If there are any problem reading from the source.
    /// - If `RawFrame` cannot be parsed from the soruce.
    fn next_frame(&mut self) -> Result<Option<RawFrame>, Self::Error>;
}

pub struct FileSource {
    reader: BufReader<File>,
}

impl FileSource {
    #[must_use]
    pub fn from_file(file: File) -> Self {
        Self {
            reader: BufReader::new(file),
        }
    }
}

impl FrameSource for FileSource {
    type Error = SourceError;

    /// Parses a `Rawframe` from lines on the open file
    ///
    /// # Errors
    /// - If ADS-B hex string is invalid.
    /// - If Downlink Format is not ADS-B (DF 17).
    /// - If there are any IO errors when reading the file.
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

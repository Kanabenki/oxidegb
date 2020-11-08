use std::{error::Error as StdError, fmt::Display};

#[derive(Debug)]
pub enum Error {
    InvalidBootRom,
    InvalidRomHeader(String),
    UnsupportedMapper,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidBootRom => write!(f, "BootRom is invalid"),
            Error::InvalidRomHeader(reason) => {
                write!(f, "Rom header cannot be parsed ({})", reason)
            }
            Error::UnsupportedMapper => write!(f, "Unsupported rom mapper"),
        }
    }
}

impl StdError for Error {}

use std::{error::Error as StdError, fmt::Display};

#[derive(Debug)]
pub enum Error {
    InvalidBootRom,
    InvalidRomHeader(&'static str),
    UnsupportedMapper(u8),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidBootRom => write!(f, "BootRom is invalid"),
            Self::InvalidRomHeader(reason) => {
                write!(f, "Rom header cannot be parsed ({reason})")
            }
            Self::UnsupportedMapper(id) => write!(f, "Unsupported rom mapper id {id}"),
        }
    }
}

impl StdError for Error {}

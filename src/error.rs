use std::{error::Error as StdError, fmt::Display};

#[derive(Debug)]
pub enum Error {
    MissingBootrom,
    InvalidBootRom,
    InvalidSave,
    InvalidRomHeader(&'static str),
    UnsupportedMapper(u8),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingBootrom => write!(f, "bootrom is enabled but no bootrom was provided"),
            Self::InvalidBootRom => write!(f, "bootrom size isn't 0x100 bytes"),
            Self::InvalidSave => write!(
                f,
                "save size does not match the size expected by the loaded rom"
            ),
            Self::InvalidRomHeader(reason) => {
                write!(f, "rom header cannot be parsed ({reason})")
            }
            Self::UnsupportedMapper(id) => write!(f, "unsupported rom mapper id {id}"),
        }
    }
}

impl StdError for Error {}

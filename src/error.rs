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
            Self::MissingBootrom => write!(f, "BootRom is enabled but not BootRom was provided"),
            Self::InvalidBootRom => write!(f, "BootRom is invalid"),
            Self::InvalidSave => write!(
                f,
                "Save size does not match the size expected by the loaded rom"
            ),
            Self::InvalidRomHeader(reason) => {
                write!(f, "Rom header cannot be parsed ({reason})")
            }
            Self::UnsupportedMapper(id) => write!(f, "Unsupported rom mapper id {id}"),
        }
    }
}

impl StdError for Error {}

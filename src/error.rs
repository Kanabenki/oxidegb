#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("bootrom is enabled but no bootrom was provided")]
    MissingBootrom,
    #[error("bootrom size isn't 0x100 bytes")]
    InvalidBootRom,
    #[error("save size does not match the size expected by the loaded rom")]
    InvalidSave,
    #[error("failed to parse saved rtc data")]
    InvalidRtcData,
    #[error("the current rom does not support save data")]
    SaveNotSupported,
    #[error("rom header cannot be parsed ({0})")]
    InvalidRomHeader(&'static str),
    #[error("unsupported rom mapper id {0}")]
    UnsupportedMapper(u8),
}

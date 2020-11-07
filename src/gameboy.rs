use crate::{cpu::Cpu, error::Error};

pub struct Gameboy {
    cpu: Cpu,
}

impl Gameboy {
    pub fn new() -> Result<Self, Error> {
        Ok(Self { cpu: Cpu::new()? })
    }
}

use crate::gameboy::mmu::MemoryOps;

pub struct Io {}

impl Io {
    pub fn new() -> Self {
        Self {}
    }

    pub fn read(&self, address: u16) -> u8 {
        todo!()
    }

    pub fn write(&mut self, address: u16, value: u8) {
        todo!()
    }
}

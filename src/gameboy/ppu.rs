use crate::gameboy::mmu::MemoryOps;

pub struct Ppu {}

impl Ppu {
    pub fn new() -> Self {
        Self {}
    }

    pub fn read_vram(&self, address: u16) -> u8 {
        0
    }

    pub fn write_vram(&self, address: u16, value: u8) {}

    pub fn read_oam(&self, address: u16) -> u8 {
        0
    }

    pub fn write_oam(&self, address: u16, value: u8) {}
}

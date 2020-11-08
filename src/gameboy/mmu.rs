use crate::{
    error::Error,
    gameboy::{cartridge::Cartridge, io::Io, ppu::Ppu},
};

mod map {
    pub const BOOTROM_START: u16 = 0x0000;
    pub const BOOTROM_END: u16 = 0x0100;
    pub const ROM_START: u16 = 0x0000;
    pub const ROM_END: u16 = 0x7FFF;
    pub const VRAM_START: u16 = 0x8000;
    pub const VRAM_END: u16 = 0x9FFF;
    pub const RAM_START: u16 = 0xA000;
    pub const RAM_END: u16 = 0xbFFF;
}

pub trait MemoryOps {
    fn read_byte(&mut self, address: u16) -> u8;

    fn write_byte(&mut self, address: u16, value: u8);

    fn read_dbyte(&mut self, address: u16) -> u16 {
        let low = self.read_byte(address);
        let high = self.read_byte(address + 1);
        u16::from_be_bytes([high, low])
    }

    fn write_dbyte(&mut self, address: u16, value: u16) {
        let [high, low] = value.to_be_bytes();
        self.write_byte(address, low);
        self.write_byte(address + 1, high);
    }
}

pub struct Mmu {
    work_ram: [u8; 8192],
    high_ram: [u8; 126],
    ppu: Ppu,
    io: Io,
    cartridge: Cartridge,
}

impl Mmu {
    pub fn new(rom: Vec<u8>, bootrom: Option<Vec<u8>>) -> Result<Self, Error> {
        Ok(Self {
            work_ram: [0; 8192],
            high_ram: [0; 126],
            ppu: Ppu::new(),
            io: Io::new(),
            cartridge: Cartridge::new(rom, bootrom)?,
        })
    }
}

impl MemoryOps for Mmu {
    fn read_byte(&mut self, address: u16) -> u8 {
        use map::*;
        match address {
            ROM_START..=ROM_END => self.cartridge.read_rom(address), // Cartridge ROM.
            VRAM_START..=VRAM_END => self.ppu.read_vram(address),    // vram
            RAM_START..=RAM_END => self.cartridge.read_ram(address), // card
            0xC000..=0xDFFF => self.work_ram[(address & 0xFFF) as usize], // Work RAM.
            0xE000..=0xFDFF => self.work_ram[(address & 0xFFF) as usize], // Echo work RAM.
            0xFE00..=0xFE9F => self.ppu.read_oam(address),           // ppu
            0xFEA0..=0xFEFF => 0,                                    // unusable ?
            0xFF00..=0xFF7F => self.io.read(address),                // IO registers.
            0xFF80..=0xFFFE => self.high_ram[(address & 0x7F) as usize], // High RAM.
            0xFFFF => 0,
        }
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        use map::*;
        match address {
            ROM_START..=ROM_END => self.cartridge.write_rom(address, value), // card
            VRAM_START..=VRAM_END => self.ppu.write_vram(address, value),    // vram
            RAM_START..=RAM_END => self.cartridge.write_ram(address, value), // card
            0xC000..=0xDFFF => self.work_ram[(address & 0xFFF) as usize] = value, // Work RAM.
            0xE000..=0xFDFF => self.work_ram[(address & 0xFFF) as usize] = value, // Echo work RAM.
            0xFE00..=0xFE9F => self.ppu.write_oam(address, value),           // ppu
            0xFEA0..=0xFEFF => (),                                           // unusable ?
            0xFF00..=0xFF7F => self.io.write(address, value),                // IO Registers.
            0xFF80..=0xFFFE => self.high_ram[(address & 0x7F) as usize] = value, // High RAM.
            0xFFFF => (),
        }
    }
}

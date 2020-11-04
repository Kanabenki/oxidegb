use crate::{io::Io, ppu::Ppu};

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
    work_ram: [u8; 4096],
    high_ram: [u8; 126],
    ppu: Ppu,
    io: Io,
}

impl MemoryOps for Mmu {
    fn read_byte(&mut self, address: u16) -> u8 {
        match address {
            0x0000..=0x7FFF => 0,                                         // card
            0x8000..=0x9FFF => 0,                                         // vram
            0xA000..=0xbFFF => 0,                                         // card
            0xC000..=0xDFFF => self.work_ram[(address & 0xFFF) as usize], // Work RAM.
            0xE000..=0xFDFF => self.work_ram[(address & 0xFFF) as usize], // Echo work RAM.
            0xFE00..=0xFE9F => self.ppu.read_byte(address),               // ppu
            0xFEA0..=0xFEFF => 0,                                         // unusable ?
            0xFF00..=0xFF7F => self.io.read_byte(address),                // IO registers.
            0xFF80..=0xFFFE => self.high_ram[(address & 0x7F) as usize],  // High RAM.
            0xFFFF => 0,
        }
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        match address {
            0x0000..=0x7FFF => (),                                                // card
            0x8000..=0x9FFF => (),                                                // vram
            0xA000..=0xbFFF => (),                                                // card
            0xC000..=0xDFFF => self.work_ram[(address & 0xFFF) as usize] = value, // Work RAM.
            0xE000..=0xFDFF => self.work_ram[(address & 0xFFF) as usize] = value, // Echo work RAM.
            0xFE00..=0xFE9F => (),                                                // ppu
            0xFEA0..=0xFEFF => (),                                                // unusable ?
            0xFF00..=0xFF7F => self.io.write_byte(address, value),                // IO Registers.
            0xFF80..=0xFFFE => self.high_ram[(address & 0x7F) as usize] = value,  // High RAM.
            0xFFFF => (),
        }
        todo!()
    }
}

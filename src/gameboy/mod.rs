mod buttons;
mod cartridge;
mod cpu;
mod interrupts;
mod io;
mod mmu;
mod ppu;

use self::cpu::Cpu;
use crate::error::Error;

pub struct Gameboy {
    cpu: Cpu,
}

impl Gameboy {
    const FREQUENCY: u32 = 4_194_304u32;

    pub fn new(rom: Vec<u8>, bootrom: Option<Vec<u8>>) -> Result<Self, Error> {
        Ok(Self {
            cpu: Cpu::new(rom, bootrom)?,
        })
    }

    pub fn tick(&mut self) {
        self.cpu.tick();
    }
}

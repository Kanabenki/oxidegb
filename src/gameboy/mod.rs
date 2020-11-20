mod cartridge;
mod cpu;
mod interrupts;
mod io;
mod mmu;
mod ppu;

use crate::error::Error;
use cpu::Cpu;

pub use io::Button;

pub struct Gameboy {
    cpu: Cpu,
}

impl Gameboy {
    const _FREQUENCY: u32 = 4_194_304u32;

    pub fn new(rom: Vec<u8>, bootrom: Option<Vec<u8>>) -> Result<Self, Error> {
        Ok(Self {
            cpu: Cpu::new(rom, bootrom)?,
        })
    }

    pub fn tick(&mut self) {
        self.cpu.next_instruction();
    }

    pub fn screen(&self) -> &[ppu::Color; 166 * 144] {
        self.cpu.mmu.ppu.screen()
    }

    pub fn set_button(&mut self, button: Button, set: bool) {
        self.cpu.mmu.io.buttons.set_button(button, set);
    }
}

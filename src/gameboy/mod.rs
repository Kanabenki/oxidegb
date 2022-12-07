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
    const TICKS_PER_FRAME: u32 = 70_224;

    pub fn new(rom: Vec<u8>, bootrom: Option<Vec<u8>>) -> Result<Self, Error> {
        Ok(Self {
            cpu: Cpu::new(rom, bootrom)?,
        })
    }

    pub fn run_frame(&mut self, _delta: u32) -> u32 {
        loop {
            let cycles_elapsed = self.cpu.next_instruction();
            if cycles_elapsed >= Self::TICKS_PER_FRAME {
                self.cpu.reset_cycles();
                return cycles_elapsed - Self::TICKS_PER_FRAME;
            }
        }
    }

    pub fn screen(&self) -> &[ppu::Color; 160 * 144] {
        self.cpu.mmu.ppu.screen()
    }

    pub fn rom_header(&self) -> &cartridge::Header {
        self.cpu.mmu.cartridge.header()
    }

    pub fn set_button(&mut self, button: Button, set: bool) {
        self.cpu.mmu.io.buttons.set_button(button, set);
    }
}

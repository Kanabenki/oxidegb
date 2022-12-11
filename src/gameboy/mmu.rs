use flagset::FlagSet;

use super::{cartridge::Cartridge, interrupts::Interrupt, io::Io, ppu::DmaRequest, ppu::Ppu};
use crate::error::Error;

mod map {
    pub const ROM_START: u16 = 0x0000;
    pub const ROM_END: u16 = 0x7FFF;
    pub const VRAM_START: u16 = 0x8000;
    pub const VRAM_END: u16 = 0x9FFF;
    pub const EXT_RAM_START: u16 = 0xA000;
    pub const EXT_RAM_END: u16 = 0xBFFF;
    pub const WRAM_START: u16 = 0xC000;
    pub const WRAM_END: u16 = 0xDFFF;
    pub const ECHO_WRAM_START: u16 = 0xE000;
    pub const ECHO_WRAM_END: u16 = 0xFDFF;
    pub const OAM_START: u16 = 0xFE00;
    pub const OAM_END: u16 = 0xFE9F;
    pub const UNUSED_START: u16 = 0xFEA0;
    pub const UNUSED_END: u16 = 0xFEFF;
    pub const IO_START: u16 = 0xFF00;
    pub const IO_END: u16 = 0xFF07;
    pub const UNUSED_2_START: u16 = 0xFF08;
    pub const UNUSED_2_END: u16 = 0xFF0E;
    pub const INTERRUPT_FLAGS: u16 = 0xFF0F;
    pub const SPU_REGISTERS_START: u16 = 0xFF10;
    pub const SPU_REGISTERS_END: u16 = 0xFF3F;
    pub const PPU_REGISTERS_START: u16 = 0xFF40;
    pub const PPU_REGISTERS_END: u16 = 0xFF4F;
    pub const DISABLE_BOOTROM: u16 = 0xFF50;
    pub const CGB_REGISTERS_START: u16 = 0xFF51;
    pub const CGB_REGISTERS_END: u16 = 0xFF7F;
    pub const HRAM_START: u16 = 0xFF80;
    pub const HRAM_END: u16 = 0xFFFE;
    pub const INTERRUPT_ENABLE: u16 = 0xFFFF;
}

#[derive(Debug)]
enum Dma {
    None,
    InProgress {
        stored_value: u8,
        offset: u8,
        base_address: u16,
    },
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

#[derive(Debug)]
pub struct Mmu {
    wram: [u8; 8192],
    hram: [u8; 127],
    pub(super) ppu: Ppu,
    pub(super) io: Io,
    pub(super) cartridge: Cartridge,
    dma: Dma,
    interrupt_flags: FlagSet<Interrupt>,
    interrupt_enable: FlagSet<Interrupt>,
    if_value: u8,
    ie_value: u8,
}

impl Mmu {
    pub fn new(rom: Vec<u8>, bootrom: Option<Vec<u8>>) -> Result<Self, Error> {
        Ok(Self {
            wram: [0; 8192],
            hram: [0; 127],
            ppu: Ppu::new(),
            io: Io::new(),
            cartridge: Cartridge::new(rom, bootrom)?,
            dma: Dma::None,
            interrupt_flags: FlagSet::default(),
            interrupt_enable: FlagSet::default(),
            if_value: 0,
            ie_value: 0,
        })
    }

    pub fn tick(&mut self) {
        let io_interrupts = self.io.tick();
        let (ppu_interrupts, dma) = self.ppu.tick();
        self.interrupt_flags |= io_interrupts | ppu_interrupts;

        if let DmaRequest::Start(high_byte) = dma {
            let base_address = (high_byte as u16) << 8;
            let stored_value = self.read_byte(base_address);
            self.dma = Dma::InProgress {
                stored_value,
                offset: 0,
                base_address,
            };
        } else if let Dma::InProgress {
            stored_value,
            offset,
            base_address,
        } = self.dma
        {
            self.write_byte(map::OAM_START + offset as u16, stored_value);
            let offset = offset + 1;
            let stored_value = self.read_byte(base_address + offset as u16);
            self.dma = if offset < Ppu::OAM_SIZE as u8 {
                Dma::InProgress {
                    stored_value,
                    offset,
                    base_address,
                }
            } else {
                Dma::None
            };
        }
    }

    pub fn interrupts(&self) -> FlagSet<Interrupt> {
        self.interrupt_flags & self.interrupt_enable
    }

    pub const fn interrupt_enable(&self) -> FlagSet<Interrupt> {
        self.interrupt_enable
    }

    pub const fn interrupt_flags(&self) -> FlagSet<Interrupt> {
        self.interrupt_flags
    }

    pub fn reset_interrupt(&mut self, interrupt: Interrupt) {
        self.interrupt_flags &= !interrupt;
    }
}

impl MemoryOps for Mmu {
    fn read_byte(&mut self, address: u16) -> u8 {
        use map::*;
        match address {
            ROM_START..=ROM_END => self.cartridge.read_rom(address - ROM_START),
            VRAM_START..=VRAM_END => self.ppu.read_vram(address - VRAM_START),
            EXT_RAM_START..=EXT_RAM_END => self.cartridge.read_ram(address - EXT_RAM_START),
            WRAM_START..=WRAM_END => self.wram[(address - WRAM_START) as usize],
            ECHO_WRAM_START..=ECHO_WRAM_END => self.wram[(address - ECHO_WRAM_START) as usize],
            OAM_START..=OAM_END => self.ppu.read_oam(address - OAM_START),
            UNUSED_START..=UNUSED_END => 0xFF,
            IO_START..=IO_END => self.io.read(address),
            UNUSED_2_START..=UNUSED_2_END => 0xFF,
            INTERRUPT_FLAGS => self.interrupt_flags.bits() | (self.if_value & 0b11100000),
            SPU_REGISTERS_START..=SPU_REGISTERS_END => 0xFF,
            PPU_REGISTERS_START..=PPU_REGISTERS_END => self.ppu.read_registers(address),
            DISABLE_BOOTROM => 0xFF,
            CGB_REGISTERS_START..=CGB_REGISTERS_END => 0xFF,
            HRAM_START..=HRAM_END => self.hram[(address - HRAM_START) as usize],
            INTERRUPT_ENABLE => self.interrupt_enable.bits() | (self.ie_value & 0b11100000),
        }
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        use map::*;
        match address {
            ROM_START..=ROM_END => self.cartridge.write_rom(address - ROM_START, value),
            VRAM_START..=VRAM_END => self.ppu.write_vram(address - VRAM_START, value),
            EXT_RAM_START..=EXT_RAM_END => self.cartridge.write_ram(address - EXT_RAM_START, value),
            WRAM_START..=WRAM_END => self.wram[(address - WRAM_START) as usize] = value,
            ECHO_WRAM_START..=ECHO_WRAM_END => {
                self.wram[(address - ECHO_WRAM_START) as usize] = value
            }
            OAM_START..=OAM_END => self.ppu.write_oam(address - OAM_START, value),
            UNUSED_START..=UNUSED_END => {}
            IO_START..=IO_END => self.io.write(address, value),
            UNUSED_2_START..=UNUSED_2_END => {}
            INTERRUPT_FLAGS => {
                self.interrupt_flags = FlagSet::new_truncated(value);
                self.if_value = value;
            }
            SPU_REGISTERS_START..=SPU_REGISTERS_END => {}
            PPU_REGISTERS_START..=PPU_REGISTERS_END => self.ppu.write_registers(address, value),
            DISABLE_BOOTROM => {
                if value != 0 {
                    self.cartridge.disable_bootrom();
                }
            }
            CGB_REGISTERS_START..=CGB_REGISTERS_END => {}
            HRAM_START..=HRAM_END => self.hram[(address - HRAM_START) as usize] = value,
            INTERRUPT_ENABLE => {
                self.interrupt_enable = FlagSet::new_truncated(value);
                self.ie_value = value;
            }
        }
    }
}

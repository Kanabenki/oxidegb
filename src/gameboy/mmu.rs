use flagset::FlagSet;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

use super::{
    apu::Apu, cartridge::Cartridge, interrupts::Interrupt, io::Io, ppu::DmaRequest, ppu::Ppu,
};
use crate::error::Error;

mod map {
    pub(crate) const ROM_START: u16 = 0x0000;
    pub(crate) const ROM_END: u16 = 0x7FFF;
    pub(crate) const VRAM_START: u16 = 0x8000;
    pub(crate) const VRAM_END: u16 = 0x9FFF;
    pub(crate) const EXT_RAM_START: u16 = 0xA000;
    pub(crate) const EXT_RAM_END: u16 = 0xBFFF;
    pub(crate) const WRAM_START: u16 = 0xC000;
    pub(crate) const WRAM_END: u16 = 0xDFFF;
    pub(crate) const ECHO_WRAM_START: u16 = 0xE000;
    pub(crate) const ECHO_WRAM_END: u16 = 0xFDFF;
    pub(crate) const OAM_START: u16 = 0xFE00;
    pub(crate) const OAM_END: u16 = 0xFE9F;
    pub(crate) const UNUSED_START: u16 = 0xFEA0;
    pub(crate) const UNUSED_END: u16 = 0xFEFF;
    pub(crate) const IO_START: u16 = 0xFF00;
    pub(crate) const IO_END: u16 = 0xFF07;
    pub(crate) const UNUSED_2_START: u16 = 0xFF08;
    pub(crate) const UNUSED_2_END: u16 = 0xFF0E;
    pub(crate) const INTERRUPT_FLAGS: u16 = 0xFF0F;
    pub(crate) const APU_REGISTERS_START: u16 = 0xFF10;
    pub(crate) const APU_REGISTERS_END: u16 = 0xFF3F;
    pub(crate) const PPU_REGISTERS_START: u16 = 0xFF40;
    pub(crate) const PPU_REGISTERS_END: u16 = 0xFF4F;
    pub(crate) const DISABLE_BOOTROM: u16 = 0xFF50;
    pub(crate) const CGB_REGISTERS_START: u16 = 0xFF51;
    pub(crate) const CGB_REGISTERS_END: u16 = 0xFF7F;
    pub(crate) const HRAM_START: u16 = 0xFF80;
    pub(crate) const HRAM_END: u16 = 0xFFFE;
    pub(crate) const INTERRUPT_ENABLE: u16 = 0xFFFF;
}

#[derive(Serialize, Deserialize, Debug)]
enum Dma {
    None,
    InProgress {
        stored_value: u8,
        offset: u8,
        base_address: u16,
    },
}

pub(crate) trait MemoryOps {
    fn read_byte(&mut self, address: u16) -> u8;

    fn write_byte(&mut self, address: u16, value: u8);

    fn read_dbyte(&mut self, address: u16) -> u16 {
        let low = self.read_byte(address);
        let high = self.read_byte(address + 1);
        u16::from_be_bytes([high, low])
    }

    fn write_dbyte(&mut self, address: u16, value: u16) {
        let [high, low] = value.to_be_bytes();
        self.write_byte(address + 1, high);
        self.write_byte(address, low);
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Mmu {
    #[serde(with = "BigArray")]
    wram: [u8; 8192],
    #[serde(with = "BigArray")]
    hram: [u8; 127],
    pub(crate) apu: Apu,
    pub(crate) ppu: Ppu,
    pub(crate) io: Io,
    pub(crate) cartridge: Cartridge,
    dma: Dma,
    pub(crate) interrupt_flags: FlagSet<Interrupt>,
    pub(crate) interrupt_enable: FlagSet<Interrupt>,
    ie_value: u8,
}

impl Mmu {
    pub(crate) fn new(
        rom: Vec<u8>,
        bootrom: Option<Vec<u8>>,
        save: Option<Vec<u8>>,
    ) -> Result<Self, Error> {
        let ppu = if bootrom.is_some() {
            Ppu::new()
        } else {
            Ppu::new_post_bootrom()
        };
        Ok(Self {
            wram: [0; 8192],
            hram: [0; 127],
            apu: Apu::new(),
            ppu,
            io: Io::new(),
            cartridge: Cartridge::new(rom, bootrom, save)?,
            dma: Dma::None,
            interrupt_flags: FlagSet::default(),
            interrupt_enable: FlagSet::default(),
            ie_value: 0,
        })
    }

    pub(crate) fn tick(&mut self) {
        self.cartridge.tick();
        let io_tick = self.io.tick();
        let (ppu_interrupts, dma_request) = self.ppu.tick();
        self.interrupt_flags |= io_tick.interrupts | ppu_interrupts;

        if io_tick.apu_inc_div {
            self.apu.inc_div();
        }
        self.apu.tick();

        if let DmaRequest::Start(high_byte) = dma_request {
            let base_address = (high_byte as u16) << 8;
            let stored_value = self.read_byte_no_conflict(base_address);
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
            self.write_byte_no_conflict(map::OAM_START + offset as u16, stored_value);
            let offset = offset + 1;
            let stored_value = self.read_byte_no_conflict(base_address + offset as u16);
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

    pub(crate) fn tick_stopped(&mut self) {
        self.interrupt_flags |= self.io.tick_stopped();
    }

    pub(crate) fn interrupts(&self) -> FlagSet<Interrupt> {
        self.interrupt_flags & self.interrupt_enable
    }

    pub(crate) fn next_interrupt(&self) -> Option<Interrupt> {
        // Needed as the flagset iteration order is undefined
        let interrupts = self.interrupts();
        [
            Interrupt::VBlank,
            Interrupt::LcdStat,
            Interrupt::Timer,
            Interrupt::Serial,
            Interrupt::Joypad,
        ]
        .iter()
        .copied()
        .find(|&f| interrupts.contains(f))
    }

    pub(crate) fn reset_interrupt(&mut self, interrupt: Interrupt) {
        self.interrupt_flags &= !interrupt;
    }

    fn read_byte_no_conflict(&mut self, address: u16) -> u8 {
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
            INTERRUPT_FLAGS => self.interrupt_flags.bits() | 0b1110_0000,
            APU_REGISTERS_START..=APU_REGISTERS_END => self.apu.read(address),
            PPU_REGISTERS_START..=PPU_REGISTERS_END => self.ppu.read_registers(address),
            DISABLE_BOOTROM => 0xFF,
            CGB_REGISTERS_START..=CGB_REGISTERS_END => 0xFF,
            HRAM_START..=HRAM_END => self.hram[(address - HRAM_START) as usize],
            INTERRUPT_ENABLE => self.interrupt_enable.bits() | (self.ie_value & 0b1110_0000),
        }
    }

    fn write_byte_no_conflict(&mut self, address: u16, value: u8) {
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
            INTERRUPT_FLAGS => self.interrupt_flags = FlagSet::new_truncated(value),
            APU_REGISTERS_START..=APU_REGISTERS_END => self.apu.write(address, value),
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

impl MemoryOps for Mmu {
    // FIXME: Emulating the OAM DMA bus conflict seems to break everything, disabled for now.
    fn read_byte(&mut self, address: u16) -> u8 {
        // TODO: Figure out actual conflict interaction between cpu and dma read.
        //if let Dma::InProgress { .. } = self.dma {
        //    if !(map::HRAM_START..=map::HRAM_END).contains(&address) {
        //        return 0xFF;
        //    }
        //}
        self.read_byte_no_conflict(address)
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        //// TODO: Figure out actual conflict interaction between cpu and dma write.
        //if let Dma::InProgress { .. } = self.dma {
        //    if !(map::HRAM_START..=map::HRAM_END).contains(&address) {
        //        return;
        //    }
        //}
        self.write_byte_no_conflict(address, value);
    }
}

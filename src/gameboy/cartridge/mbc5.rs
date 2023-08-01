use serde::{Deserialize, Serialize};

use super::{
    MapperOps, HIGH_BANK_END, HIGH_BANK_START, LOW_BANK_END, LOW_BANK_START, RAM_BANK_SIZE,
    ROM_BANK_SIZE,
};

// TODO: Rumble with controller.
#[derive(Serialize, Deserialize, Debug)]
pub struct Mbc5 {
    _has_rumble: bool,
    has_ram: bool,
    has_battery: bool,
    rom_bank: u16,
    ram_bank: u8,
    ram_bank_mask: u8,
    ram_enabled: bool,
}

impl Mbc5 {
    const RAM_ENABLE_START: u16 = 0x0000;
    const RAM_ENABLE_END: u16 = 0x1FFF;
    const ROM_BANK_LOW_START: u16 = 0x2000;
    const ROM_BANK_LOW_END: u16 = 0x2FFF;
    const ROM_BANK_HIGH_START: u16 = 0x3000;
    const ROM_BANK_HIGH_END: u16 = 0x3FFF;
    const RAM_BANK_START: u16 = 0x4000;
    const RAM_BANK_END: u16 = 0x5FFF;

    pub fn new(has_rumble: bool, has_ram: bool, has_battery: bool) -> Self {
        Self {
            _has_rumble: has_rumble,
            has_ram,
            has_battery,
            rom_bank: 1,
            ram_bank: 0,
            ram_bank_mask: if has_rumble { 0b0111 } else { 0b1111 },
            ram_enabled: false,
        }
    }
}

impl MapperOps for Mbc5 {
    fn read_rom(&mut self, rom: &[u8], address: u16) -> u8 {
        match address {
            LOW_BANK_START..=LOW_BANK_END => rom[address as usize],
            HIGH_BANK_START..=HIGH_BANK_END => {
                rom[address as usize + (self.rom_bank - 1) as usize * ROM_BANK_SIZE]
            }
            _ => panic!("Tried to read Mbc3 rom out of range"),
        }
    }

    fn write_rom(&mut self, _rom: &mut [u8], address: u16, value: u8) {
        match address {
            Self::RAM_ENABLE_START..=Self::RAM_ENABLE_END => {
                self.ram_enabled = value & 0x0F == 0b1010
            }
            Self::ROM_BANK_LOW_START..=Self::ROM_BANK_LOW_END => {
                self.rom_bank = (self.rom_bank & 0xFF00) | value as u16;
            }
            Self::ROM_BANK_HIGH_START..=Self::ROM_BANK_HIGH_END => {
                self.rom_bank = (self.rom_bank & 0x00FF) | ((value as u16 & 1) << 8);
            }
            Self::RAM_BANK_START..=Self::RAM_BANK_END => self.ram_bank = value & self.ram_bank_mask,
            _ => panic!("Tried to write Mbc3 rom out of range"),
        }
    }

    fn read_ram(&mut self, ram: &[u8], address: u16) -> u8 {
        if self.has_ram {
            ram[address as usize + self.ram_bank as usize * RAM_BANK_SIZE]
        } else {
            0xFF
        }
    }

    fn write_ram(&mut self, _ram: &mut [u8], _address: u16, _value: u8) {}

    fn can_save(&self) -> bool {
        self.has_battery
    }
}

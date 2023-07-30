use super::{
    MapperOps, HIGH_BANK_END, HIGH_BANK_START, LOW_BANK_END, LOW_BANK_START, ROM_BANK_SIZE,
};

#[derive(Debug)]
pub struct Mbc2 {
    has_battery: bool,
    ram_enabled: bool,
    _rom_bank_count: u16,
    rom_bank: u8,
}

impl Mbc2 {
    pub const RAM_SIZE: usize = 0x200;
    const RAM_ADDR_MASK: u16 = 0x1FF;

    pub const fn new(rom_bank_count: u16, has_battery: bool) -> Self {
        Self {
            has_battery,
            ram_enabled: false,
            _rom_bank_count: rom_bank_count,
            rom_bank: 1,
        }
    }

    const fn rom_address_high(&self, address: u16) -> usize {
        address as usize + (self.rom_bank - 1) as usize * ROM_BANK_SIZE
    }

    const fn ram_address(&self, address: u16) -> usize {
        (address & Self::RAM_ADDR_MASK) as usize
    }
}

impl MapperOps for Mbc2 {
    fn read_rom(&mut self, rom: &[u8], address: u16) -> u8 {
        match address {
            LOW_BANK_START..=LOW_BANK_END => rom[address as usize],
            HIGH_BANK_START..=HIGH_BANK_END => rom[self.rom_address_high(address)],
            _ => panic!("Tried to read Mbc2 rom out of range"),
        }
    }

    fn write_rom(&mut self, _rom: &mut [u8], address: u16, value: u8) {
        if let LOW_BANK_START..=LOW_BANK_END = address {
            if address & 0x100 == 0 {
                self.ram_enabled = (value & 0xF) == 0b1010;
            } else {
                self.rom_bank = u8::max(value & 0x0F, 1);
            }
        }
    }

    fn read_ram(&mut self, ram: &[u8], address: u16) -> u8 {
        if !self.ram_enabled {
            0xFF
        } else {
            ram[self.ram_address(address)] | 0xF0
        }
    }

    fn write_ram(&mut self, ram: &mut [u8], address: u16, value: u8) {
        if self.ram_enabled {
            ram[self.ram_address(address)] = value | 0xF0;
        }
    }

    fn can_save(&self) -> bool {
        self.has_battery
    }
}

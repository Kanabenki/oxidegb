use super::{MapperOps, RAM_BANK_SIZE, ROM_BANK_SIZE};

#[derive(Debug)]
enum BankMode {
    Rom,
    Ram,
}

#[derive(Debug)]
pub struct Mbc1 {
    has_ram: bool,
    _has_battery: bool,
    ram_enabled: bool,
    rom_bank_count: u16,
    rom_bank_mask: u8,
    rom_bank: u8,
    ram_bank: u8,
    bank_mode: BankMode,
}

impl Mbc1 {
    const LOW_BANK_START: u16 = 0x0000;
    const LOW_BANK_END: u16 = 0x3FFF;
    const HIGH_BANK_START: u16 = 0x4000;
    const HIGH_BANK_END: u16 = 0x7FFF;

    const WRITE_RAM_ENABLE_START: u16 = 0x0000;
    const WRITE_RAM_ENABLE_END: u16 = 0x1FFF;
    const WRITE_ROM_BANK_START: u16 = 0x2000;
    const WRITE_ROM_BANK_END: u16 = 0x3FFF;
    const WRITE_MODE_BANK_START: u16 = 0x4000;
    const WRITE_MODE_BANK_END: u16 = 0x5FFF;
    const WRITE_MODE_START: u16 = 0x6000;
    const WRITE_MODE_END: u16 = 0x7FFF;

    pub const fn new(rom_bank_count: u16, has_ram: bool, has_battery: bool) -> Self {
        Self {
            has_ram,
            _has_battery: has_battery,
            ram_enabled: false,
            rom_bank_count,
            rom_bank: 1,
            ram_bank: 0,
            bank_mode: BankMode::Rom,
            rom_bank_mask: rom_bank_count as u8 - 1,
        }
    }

    const fn rom_address_low(&self, address: u16) -> usize {
        match self.bank_mode {
            BankMode::Rom => address as usize,
            BankMode::Ram => {
                address as usize + (self.rom_bank & 0b1110_0000) as usize * ROM_BANK_SIZE
            }
        }
    }

    const fn rom_address_high(&self, address: u16) -> usize {
        address as usize - ROM_BANK_SIZE + self.rom_bank as usize * ROM_BANK_SIZE
    }

    fn ram_address(&self, address: u16) -> usize {
        match self.bank_mode {
            BankMode::Rom => address as usize,
            BankMode::Ram => address as usize + self.ram_bank as usize * RAM_BANK_SIZE,
        }
    }
}

impl MapperOps for Mbc1 {
    fn read_rom(&mut self, rom: &[u8], address: u16) -> u8 {
        match address {
            Self::LOW_BANK_START..=Self::LOW_BANK_END => rom[self.rom_address_low(address)],
            Self::HIGH_BANK_START..=Self::HIGH_BANK_END => rom[self.rom_address_high(address)],
            _ => panic!("Tried to read Mbc1 rom out of range"),
        }
    }

    fn write_rom(&mut self, _rom: &mut [u8], address: u16, value: u8) {
        match address {
            Self::WRITE_RAM_ENABLE_START..=Self::WRITE_RAM_ENABLE_END => {
                self.ram_enabled = (value & 0xF) == 0xA;
            }
            Self::WRITE_ROM_BANK_START..=Self::WRITE_ROM_BANK_END => {
                let mut lower_bits = value & 0b11111;
                if lower_bits as u16 >= self.rom_bank_count {
                    lower_bits &= self.rom_bank_mask;
                }
                self.rom_bank = (self.rom_bank & 0b1110_0000) | u8::max(lower_bits, 1);
            }
            Self::WRITE_MODE_BANK_START..=Self::WRITE_MODE_BANK_END => {
                let bits = value & 0b11;
                self.ram_bank = bits;
                self.rom_bank = (self.rom_bank & 0b11111) | (bits << 5);
            }
            Self::WRITE_MODE_START..=Self::WRITE_MODE_END => {
                self.bank_mode = match value & 0b1 {
                    0 => BankMode::Rom,
                    1 => BankMode::Ram,
                    _ => unreachable!(),
                }
            }
            _ => panic!(),
        }
    }

    fn read_ram(&mut self, ram: &[u8], address: u16) -> u8 {
        if !self.has_ram || !self.ram_enabled && (address as usize) < ram.len() {
            0xFF
        } else {
            ram[self.ram_address(address)]
        }
    }

    fn write_ram(&mut self, ram: &mut [u8], address: u16, value: u8) {
        if self.has_ram && self.ram_enabled && (address as usize) < ram.len() {
            ram[self.ram_address(address)] = value;
        }
    }
}

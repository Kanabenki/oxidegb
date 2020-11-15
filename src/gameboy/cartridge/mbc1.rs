use super::Mapper;

#[derive(Debug)]
enum BankMode {
    RomHighBits,
    Ram,
}

#[derive(Debug)]
pub struct Mbc1 {
    has_ram: bool,
    _has_battery: bool,
    ram_enabled: bool,
    rom_bank_count: u16,
    current_rom_bank: u8,
    bank_mode: BankMode,
    mode_register: u8,
}

impl Mbc1 {
    const LOW_BANK_START: u16 = 0x0000;
    const LOW_BANK_END: u16 = 0x3FFF;
    const HIGH_BANK_START: u16 = 0x4000;
    const HIGH_BANK_END: u16 = 0x7FFF;

    const ROM_BANK_SIZE: usize = 0x4000;
    const RAM_BANK_SIZE: usize = 0x2000;

    const WRITE_RAM_ENABLE_START: u16 = 0x0000;
    const WRITE_RAM_ENABLE_END: u16 = 0x1FFF;
    const WRITE_ROM_BANK_START: u16 = 0x2000;
    const WRITE_ROM_BANK_END: u16 = 0x3FFF;
    const WRITE_MODE_BANK_START: u16 = 0x4000;
    const WRITE_MODE_BANK_END: u16 = 0x5FFF;
    const WRITE_MODE_START: u16 = 0x6000;
    const WRITE_MODE_END: u16 = 0x7FFF;

    pub fn new(rom_bank_count: u16, has_ram: bool, has_battery: bool) -> Self {
        Self {
            has_ram,
            _has_battery: has_battery,
            ram_enabled: false,
            rom_bank_count,
            current_rom_bank: 1,
            bank_mode: BankMode::RomHighBits,
            mode_register: 0,
        }
    }

    fn rom_address_low(&self, address: u16) -> usize {
        match self.bank_mode {
            BankMode::RomHighBits => {
                address as usize + ((self.mode_register << 5) as usize * Self::ROM_BANK_SIZE)
            }
            BankMode::Ram => address as usize,
        }
    }

    fn rom_address_high(&self, address: u16) -> usize {
        match self.bank_mode {
            BankMode::RomHighBits => {
                address as usize - Self::ROM_BANK_SIZE
                    + ((self.mode_register << 5 | self.current_rom_bank) as usize
                        * Self::ROM_BANK_SIZE)
            }
            BankMode::Ram => {
                address as usize - Self::ROM_BANK_SIZE
                    + self.current_rom_bank as usize * Self::ROM_BANK_SIZE
            }
        }
    }

    fn ram_address(&self, address: u16) -> usize {
        match self.bank_mode {
            BankMode::RomHighBits => address as usize,
            BankMode::Ram => {
                address as usize + self.current_rom_bank as usize * Self::RAM_BANK_SIZE
            }
        }
    }
}

impl Mapper for Mbc1 {
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
                let lower_bits = (value & 0b11111) & (self.rom_bank_count - 1) as u8;
                self.current_rom_bank = (self.current_rom_bank & !0b11111)
                    | if lower_bits != 0 { lower_bits } else { 0b1 };
            }
            Self::WRITE_MODE_BANK_START..=Self::WRITE_MODE_BANK_END => {
                self.mode_register = value & 0b11
            }
            Self::WRITE_MODE_START..=Self::WRITE_MODE_END => {
                self.bank_mode = match value & 0b1 {
                    0 => BankMode::RomHighBits,
                    1 => BankMode::Ram,
                    _ => unreachable!(),
                }
            }
            _ => panic!(),
        }
    }

    fn read_ram(&mut self, ram: &[u8], address: u16) -> u8 {
        if !self.has_ram || !self.ram_enabled || address as usize >= ram.len() {
            0xFF
        } else {
            ram[self.ram_address(address)]
        }
    }

    fn write_ram(&mut self, ram: &mut [u8], address: u16, value: u8) {
        if self.has_ram && self.ram_enabled && address as usize <= ram.len() {
            ram[self.ram_address(address)] = value;
        }
    }
}

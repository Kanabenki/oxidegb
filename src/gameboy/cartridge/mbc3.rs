use super::{
    MapperOps, HIGH_BANK_END, HIGH_BANK_START, LOW_BANK_END, LOW_BANK_START, ROM_BANK_SIZE,
};

#[derive(Debug)]
pub struct Mbc3 {
    rom_bank: u8,
    ram_bank_rtc_select: u8,
    ram_rtc_enabled: bool,
}

impl Mbc3 {
    const RAM_RTC_ENABLE_START: u16 = 0x0000;
    const RAM_RTC_ENABLE_END: u16 = 0x1FFF;
    const ROM_BANK_SELECT_START: u16 = 0x2000;
    const ROM_BANK_SELECT_END: u16 = 0x3FFF;
    const RAM_RTC_SELECT_START: u16 = 0x4000;
    const RAM_RTC_SELECT_END: u16 = 0x5FFF;
    const RTC_LATCH_START: u16 = 0x6000;
    const RTC_LATCH_END: u16 = 0x7FFF;

    const _MAX_RAM_BANK_SELECT: usize = 4;

    pub fn new(_has_rtc: bool, _has_ram: bool, _has_battery: bool) -> Self {
        Self {
            rom_bank: 1,
            ram_bank_rtc_select: 0,
            ram_rtc_enabled: false,
        }
    }
}

impl MapperOps for Mbc3 {
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
            Self::RAM_RTC_ENABLE_START..=Self::RAM_RTC_ENABLE_END => {
                self.ram_rtc_enabled = value & 0x0F == 0x0A
            }
            Self::ROM_BANK_SELECT_START..=Self::ROM_BANK_SELECT_END => {
                self.rom_bank = u8::max(value & 0b01111111, 1);
            }
            Self::RAM_RTC_SELECT_START..=Self::RAM_RTC_SELECT_END => {
                self.ram_bank_rtc_select = value & 0x0F;
            }
            Self::RTC_LATCH_START..=Self::RTC_LATCH_END => todo!(),
            _ => panic!("Tried to write Mbc3 rom out of range"),
        }
    }

    fn read_ram(&mut self, _ram: &[u8], _address: u16) -> u8 {
        todo!()
    }

    fn write_ram(&mut self, _rom: &mut [u8], _address: u16, _value: u8) {
        todo!()
    }
}

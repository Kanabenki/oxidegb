use serde::{Deserialize, Serialize};

use crate::gameboy::Gameboy;

use super::{
    MapperOps, HIGH_BANK_END, HIGH_BANK_START, LOW_BANK_END, LOW_BANK_START, RAM_BANK_SIZE,
    ROM_BANK_SIZE,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct Mbc3 {
    has_rtc: bool,
    has_ram: bool,
    has_battery: bool,
    rom_bank: u8,
    ram_bank_rtc_select: u8,
    ram_rtc_enabled: bool,
    current_time: RtcRegisters,
    latched_time: Option<RtcRegisters>,
    rtc_halt: bool,
    rtc_carry: bool,
    cycles: usize,
}

// TODO: Fetch current time and save/restore once save are implemented
#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy)]
struct RtcRegisters {
    seconds: u8,
    minutes: u8,
    hours: u8,
    days: u16,
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

    const MAX_RAM_BANK_SELECT: u8 = 3;
    const RTC_SECONDS_REG: u8 = 0x08;
    const RTC_MINUTES_REG: u8 = 0x09;
    const RTC_HOURS_REG: u8 = 0x0A;
    const RTC_DAY_LOW_REG: u8 = 0x0B;
    const RTC_DAY_HIGH_REG: u8 = 0x0C;

    pub(crate) fn new(has_rtc: bool, has_ram: bool, has_battery: bool) -> Self {
        Self {
            has_rtc,
            has_ram,
            has_battery,
            rom_bank: 1,
            ram_bank_rtc_select: 0,
            ram_rtc_enabled: false,
            current_time: RtcRegisters::default(),
            latched_time: None,
            rtc_halt: false,
            rtc_carry: false,
            cycles: 0,
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
                self.rom_bank = u8::max(value & 0b0111_1111, 1);
            }
            Self::RAM_RTC_SELECT_START..=Self::RAM_RTC_SELECT_END => {
                self.ram_bank_rtc_select = value & 0x0F;
            }
            Self::RTC_LATCH_START..=Self::RTC_LATCH_END => {
                self.latched_time = (value & 1 != 0).then_some(self.current_time);
            }
            _ => panic!("Tried to write Mbc3 rom out of range"),
        }
    }

    fn read_ram(&mut self, ram: &[u8], address: u16) -> u8 {
        if !self.ram_rtc_enabled {
            return 0xFF;
        }

        let time = self.latched_time.unwrap_or(self.current_time);
        match self.ram_bank_rtc_select {
            0..=Self::MAX_RAM_BANK_SELECT if self.has_ram => {
                ram[address as usize + self.ram_bank_rtc_select as usize * RAM_BANK_SIZE]
            }
            Self::RTC_SECONDS_REG if self.has_rtc => time.seconds,
            Self::RTC_MINUTES_REG if self.has_rtc => time.minutes,
            Self::RTC_HOURS_REG if self.has_rtc => time.hours,
            Self::RTC_DAY_LOW_REG if self.has_rtc => time.days as u8,
            Self::RTC_DAY_HIGH_REG if self.has_rtc => {
                (time.days >> 8) as u8 & 0b1
                    | (self.rtc_halt as u8) << 6
                    | (self.rtc_carry as u8) << 7
            }
            _ => 0xFF,
        }
    }

    fn write_ram(&mut self, ram: &mut [u8], address: u16, value: u8) {
        if !self.ram_rtc_enabled {
            return;
        }

        match self.ram_bank_rtc_select {
            0..=Self::MAX_RAM_BANK_SELECT => {
                ram[address as usize + self.ram_bank_rtc_select as usize * RAM_BANK_SIZE] = value
            }
            Self::RTC_SECONDS_REG => self.current_time.seconds = value,
            Self::RTC_MINUTES_REG => self.current_time.minutes = value,
            Self::RTC_HOURS_REG => self.current_time.hours = value,
            Self::RTC_DAY_LOW_REG => self.current_time.days &= 0xFF00 | value as u16,
            Self::RTC_DAY_HIGH_REG => {
                self.current_time.days &= 0x00FF | ((value as u16 & 1) << 8);
                self.rtc_carry = value >> 7 != 0;
                self.rtc_halt = (value >> 6) & 1 != 0;
            }
            _ => {}
        }
    }

    fn tick(&mut self) {
        if !self.has_rtc || self.rtc_halt {
            self.cycles = 0;
        } else {
            self.cycles += 1;
            if self.cycles == Gameboy::CYCLES_PER_SECOND as usize / 4 {
                self.cycles = 0;
                self.current_time.seconds += 1;
                if self.current_time.seconds == 60 {
                    self.current_time.seconds = 0;
                    self.current_time.minutes += 1;
                    if self.current_time.minutes == 60 {
                        self.current_time.minutes = 0;
                        self.current_time.hours += 1;
                        if self.current_time.hours == 24 {
                            self.current_time.hours = 0;
                            self.current_time.days += 1;
                            if self.current_time.days == 0b1_1111_1111 {
                                self.current_time.days = 0;
                                self.rtc_carry = true;
                            }
                        }
                    }
                }
            }
        }
    }

    fn can_save(&self) -> bool {
        self.has_battery
    }
}

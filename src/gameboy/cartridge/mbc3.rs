use std::{
    io,
    time::{SystemTime, UNIX_EPOCH},
};

use cookie_factory as cf;
use serde::{Deserialize, Serialize};

use super::{
    MapperOps, HIGH_BANK_END, HIGH_BANK_START, LOW_BANK_END, LOW_BANK_START, RAM_BANK_SIZE,
    ROM_BANK_SIZE,
};
use crate::gameboy::Gameboy;

#[derive(Serialize, Deserialize, Debug)]
pub struct Mbc3 {
    has_rtc: bool,
    has_ram: bool,
    has_battery: bool,
    rom_bank: u8,
    ram_bank_rtc_select: u8,
    ram_rtc_enabled: bool,
    current_time: RtcRegisters,
    latched_time: RtcRegisters,
    latch_toggle: bool,
    cycles: usize,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy)]
struct RtcRegisters {
    seconds: u8,
    minutes: u8,
    hours: u8,
    days: u16,
    carry: bool,
    halt: bool,
}

impl RtcRegisters {
    fn parse(input: &[u8]) -> nom::IResult<&[u8], Self> {
        let (input, seconds) = nom::number::complete::le_u32(input)?;
        let (input, minutes) = nom::number::complete::le_u32(input)?;
        let (input, hours) = nom::number::complete::le_u32(input)?;
        let (input, days_low) = nom::number::complete::le_u32(input)?;
        let (input, days_high) = nom::number::complete::le_u32(input)?;
        let mut regs = Self {
            seconds: seconds as u8,
            minutes: minutes as u8,
            hours: hours as u8,
            days: days_low as u16,
            carry: false,
            halt: false,
        };
        regs.set_days_high_byte(days_high as u8);
        Ok((input, regs))
    }

    fn serialize<W>(&self) -> impl cf::SerializeFn<W>
    where
        W: io::Write,
    {
        cf::sequence::tuple((
            cf::bytes::le_u32(self.seconds as u32),
            cf::bytes::le_u32(self.minutes as u32),
            cf::bytes::le_u32(self.hours as u32),
            cf::bytes::le_u32((self.days & 0xFF) as u32),
            cf::bytes::le_u32((self.days >> 8) as u32),
        ))
    }

    fn days_high_byte(&self) -> u8 {
        (self.days >> 8) as u8 & 0b1 | (self.halt as u8) << 6 | (self.carry as u8) << 7
    }

    fn set_days_high_byte(&mut self, value: u8) {
        self.days = (self.days & 0x00FF) | ((value as u16 & 1) << 8);
        self.carry = value >> 7 != 0;
        self.halt = (value >> 6) & 1 != 0;
    }
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
            latched_time: RtcRegisters::default(),
            latch_toggle: false,
            cycles: 0,
        }
    }

    // See https://bgb.bircd.org/rtcsave.html for the format.
    pub(crate) fn set_rtc_data<'a>(&mut self, rtc_data: &'a [u8]) -> nom::IResult<&'a [u8], ()> {
        if !self.has_rtc {
            return Ok((rtc_data, ()));
        }

        let (input, current_time) = RtcRegisters::parse(rtc_data)?;
        let (input, latched_time) = RtcRegisters::parse(input)?;
        let (input, timestamp) = nom::number::complete::le_u64(input)?;
        let timestamp_now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.current_time = current_time;
        self.latched_time = latched_time;
        if timestamp_now > timestamp && !self.current_time.halt {
            let carry_add = |a: u8, b, prev_carry| {
                let (sum, carry) = a.overflowing_add(b);
                let (sum, carry2) = sum.overflowing_add(u8::from(prev_carry));
                (sum, carry | carry2)
            };

            let delta = timestamp_now - timestamp;
            let (seconds, carry) = ((delta % 60) as u8).overflowing_add(self.current_time.seconds);
            self.current_time.seconds = seconds;
            let (minutes, carry) = carry_add(
                ((delta % 3600) / 60) as u8,
                self.current_time.minutes,
                carry,
            );
            self.current_time.minutes = minutes;
            let (hours, carry) = carry_add(
                ((delta % 86400) / 3600) as u8,
                self.current_time.hours,
                carry,
            );
            self.current_time.hours = hours;
            self.current_time.days +=
                u16::min((delta / 86400) as u16, 0b1_1111_1111) + u16::from(carry);
            if self.current_time.days > 0b1_1111_1111 {
                self.current_time.days -= 0b1_1111_1111;
                self.current_time.carry = true;
            }
        }

        Ok((input, ()))
    }

    pub(crate) fn rtc_data(&self) -> Option<Vec<u8>> {
        if !self.has_rtc {
            return None;
        }

        let mut buf: Vec<u8> = vec![];
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        cf::gen(
            cf::sequence::tuple((
                self.current_time.serialize(),
                self.latched_time.serialize(),
                cf::bytes::le_u64(timestamp),
            )),
            &mut buf,
        )
        .unwrap();
        Some(buf)
    }

    pub(crate) fn has_rtc(&self) -> bool {
        self.has_rtc
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
                let set = (value & 1) != 0;
                if !self.latch_toggle && set {
                    self.latched_time = self.current_time;
                }
                self.latch_toggle = set;
            }
            _ => panic!("Tried to write Mbc3 rom out of range"),
        }
    }

    fn read_ram(&mut self, ram: &[u8], address: u16) -> u8 {
        if !self.ram_rtc_enabled {
            return 0xFF;
        }

        match self.ram_bank_rtc_select {
            0..=Self::MAX_RAM_BANK_SELECT if self.has_ram => {
                ram[address as usize + self.ram_bank_rtc_select as usize * RAM_BANK_SIZE]
            }
            Self::RTC_SECONDS_REG if self.has_rtc => self.latched_time.seconds,
            Self::RTC_MINUTES_REG if self.has_rtc => self.latched_time.minutes,
            Self::RTC_HOURS_REG if self.has_rtc => self.latched_time.hours,
            Self::RTC_DAY_LOW_REG if self.has_rtc => self.latched_time.days as u8,
            Self::RTC_DAY_HIGH_REG if self.has_rtc => self.latched_time.days_high_byte(),
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
            Self::RTC_SECONDS_REG => self.current_time.seconds = value & 0b11_1111,
            Self::RTC_MINUTES_REG => self.current_time.minutes = value & 0b11_1111,
            Self::RTC_HOURS_REG => self.current_time.hours = value & 0b1_1111,
            Self::RTC_DAY_LOW_REG => {
                self.current_time.days = (self.current_time.days & 0xFF00) | value as u16
            }
            Self::RTC_DAY_HIGH_REG => self.current_time.set_days_high_byte(value),
            _ => {}
        }
    }

    fn tick(&mut self) {
        if !self.has_rtc || self.current_time.halt {
            self.cycles = 0;
        } else {
            let time = &mut self.current_time;
            self.cycles += 4;
            if self.cycles == Gameboy::CYCLES_PER_SECOND as usize {
                self.cycles = 0;
                time.seconds += 1;
                if time.seconds == 60 {
                    time.seconds = 0;
                    time.minutes += 1;
                    if time.minutes == 60 {
                        time.minutes = 0;
                        time.hours += 1;
                        if time.hours == 24 {
                            time.hours = 0;
                            time.days += 1;
                            if time.days > 0b1_1111_1111 {
                                time.days = 0;
                                time.carry = true;
                            }
                        } else if time.hours == 0b10_0000 {
                            time.hours = 0;
                        }
                    } else if time.minutes == 0b100_0000 {
                        time.minutes = 0;
                    }
                } else if time.seconds == 0b100_0000 {
                    time.seconds = 0;
                }
            }
        }
    }

    fn has_battery(&self) -> bool {
        self.has_battery
    }
}

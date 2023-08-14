mod mbc1;
mod mbc2;
mod mbc3;
mod mbc5;
mod rom_only;

use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};

use self::{mbc1::Mbc1, mbc2::Mbc2, mbc3::Mbc3, mbc5::Mbc5, rom_only::RomOnly};
use crate::error::Error;

#[derive(Serialize, Deserialize, Debug)]
pub(crate) enum Destination {
    Japanese,
    NonJapanese,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Header {
    pub(crate) title: String,
    pub(crate) rom_size: u32,
    pub(crate) rom_bank_count: u32,
    pub(crate) ram_size: u32,
    pub(crate) ram_bank_count: u32,
    pub(crate) destination: Destination,
}

impl Header {
    fn parse(rom_bytes: &[u8]) -> Result<(Self, Mapper), Error> {
        if rom_bytes.len() < 0x014F {
            return Err(Error::InvalidRomHeader("Header is too short"));
        }

        let title = String::from_utf8_lossy(
            rom_bytes[0x0134..=0x143]
                .splitn(2, |byte| *byte == 0)
                .next()
                .unwrap(),
        )
        .to_string();

        let rom_bank_count = match rom_bytes[0x148] {
            rom_bank_byte @ 0x00..=0x08 => 2 << rom_bank_byte,
            _ => return Err(Error::InvalidRomHeader("Invalid rom bank count")),
        };

        let rom_size = 0x4000 * rom_bank_count;

        let (ram_bank_count, ram_size) = match rom_bytes[0x149] {
            0x00 => (0, 0),
            0x02 => (1, 0x2000),
            0x03 => (4, 0x2000 * 4),
            0x04 => (16, 0x2000 * 16),
            0x05 => (8, 0x2000 * 8),
            _ => return Err(Error::InvalidRomHeader("Invalid ram bank count")),
        };

        let mapper = match rom_bytes[0x147] {
            0x00 => Mapper::RomOnly(RomOnly),
            0x01 => Mapper::Mbc1(Mbc1::new(rom_bank_count as u16, false, false)),
            0x02 => Mapper::Mbc1(Mbc1::new(rom_bank_count as u16, true, false)),
            0x03 => Mapper::Mbc1(Mbc1::new(rom_bank_count as u16, true, true)),

            0x05 => Mapper::Mbc2(Mbc2::new(false)),
            0x06 => Mapper::Mbc2(Mbc2::new(true)),

            0x0F => Mapper::Mbc3(Mbc3::new(true, false, true)),
            0x10 => Mapper::Mbc3(Mbc3::new(true, true, true)),
            0x11 => Mapper::Mbc3(Mbc3::new(false, false, false)),
            0x12 => Mapper::Mbc3(Mbc3::new(false, true, false)),
            0x13 => Mapper::Mbc3(Mbc3::new(false, true, true)),

            0x19 => Mapper::Mbc5(Mbc5::new(false, false, false)),
            0x1A => Mapper::Mbc5(Mbc5::new(false, true, false)),
            0x1B => Mapper::Mbc5(Mbc5::new(false, true, true)),
            0x1C => Mapper::Mbc5(Mbc5::new(true, false, false)),
            0x1D => Mapper::Mbc5(Mbc5::new(true, true, false)),
            0x1E => Mapper::Mbc5(Mbc5::new(true, true, true)),

            id => return Err(Error::UnsupportedMapper(id)),
        };

        let destination = match rom_bytes[0x14A] {
            0x00 => Destination::Japanese,
            0x01 => Destination::NonJapanese,
            _ => return Err(Error::InvalidRomHeader("Invalid destination")),
        };

        Ok((
            Self {
                title,
                rom_size,
                rom_bank_count,
                ram_size,
                ram_bank_count,
                destination,
            },
            mapper,
        ))
    }
}

#[enum_dispatch(Mapper)]
pub(crate) trait MapperOps {
    fn read_rom(&mut self, rom: &[u8], address: u16) -> u8;
    fn write_rom(&mut self, rom: &mut [u8], address: u16, value: u8);
    fn read_ram(&mut self, ram: &[u8], address: u16) -> u8;
    fn write_ram(&mut self, rom: &mut [u8], address: u16, value: u8);
    fn tick(&mut self) {}
    fn has_battery(&self) -> bool {
        false
    }
}

#[derive(Debug, Default)]
pub struct SaveData<'a> {
    pub ram: Option<&'a [u8]>,
    pub rtc: Option<Vec<u8>>,
}

#[enum_dispatch]
#[derive(Serialize, Deserialize, Debug)]
pub enum Mapper {
    RomOnly(RomOnly),
    Mbc1(Mbc1),
    Mbc2(Mbc2),
    Mbc3(Mbc3),
    Mbc5(Mbc5),
}

const BOOTROM_END: u16 = 0x0100;

const LOW_BANK_START: u16 = 0x0000;
const LOW_BANK_END: u16 = 0x3FFF;
const HIGH_BANK_START: u16 = 0x4000;
const HIGH_BANK_END: u16 = 0x7FFF;

const ROM_BANK_SIZE: usize = 0x4000;
const RAM_BANK_SIZE: usize = 0x2000;

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Cartridge {
    pub(crate) header: Header,
    pub(crate) mapper: Mapper,
    #[serde(skip)]
    pub(crate) bootrom: Option<Vec<u8>>,
    pub(crate) bootrom_enabled: bool,
    #[serde(skip)]
    pub(crate) rom: Vec<u8>,
    pub(crate) ram: Vec<u8>,
}

impl Cartridge {
    pub(crate) fn new(
        rom: Vec<u8>,
        bootrom: Option<Vec<u8>>,
        save: Option<Vec<u8>>,
    ) -> Result<Self, Error> {
        let (header, mut mapper) = Header::parse(&rom)?;
        // Mbc2 has 512 half-bytes of internal RAM that are not reported in the header.
        let ram_size = if let Mapper::Mbc2(_) = mapper {
            Mbc2::RAM_SIZE
        } else {
            header.ram_size as usize
        };
        let ram = if let Some(mut save) = save {
            if !mapper.has_battery() {
                return Err(Error::SaveNotSupported);
            }
            if let Mapper::Mbc3(mapper) = &mut mapper {
                if save.len() == ram_size + 48
                    && mapper.has_rtc()
                    && mapper.set_rtc_data(&save[ram_size..]).is_ok()
                {
                    save.truncate(ram_size);
                } else {
                    return Err(Error::InvalidRtcData);
                }
            }
            if save.len() != ram_size {
                return Err(Error::InvalidSave);
            }

            save
        } else {
            vec![0; ram_size]
        };
        let bootrom_enabled = bootrom.is_some();
        if let Some(bootrom) = &bootrom {
            if bootrom.len() != 0x100 {
                return Err(Error::InvalidBootRom);
            }
        }

        Ok(Self {
            header,
            mapper,
            bootrom,
            bootrom_enabled,
            rom,
            ram,
        })
    }

    pub(crate) fn tick(&mut self) {
        self.mapper.tick();
    }

    pub(crate) fn disable_bootrom(&mut self) {
        self.bootrom_enabled = false;
    }

    pub(crate) fn read_rom(&mut self, address: u16) -> u8 {
        match &self.bootrom {
            Some(bootrom) if self.bootrom_enabled && address <= BOOTROM_END => {
                bootrom[address as usize]
            }
            _ => self.mapper.read_rom(&self.rom, address),
        }
    }

    pub(crate) fn write_rom(&mut self, address: u16, value: u8) {
        match &mut self.bootrom {
            Some(bootrom) if self.bootrom_enabled && address <= BOOTROM_END => {
                bootrom[address as usize] = value
            }
            _ => self.mapper.write_rom(&mut self.rom, address, value),
        }
    }

    pub(crate) fn read_ram(&mut self, address: u16) -> u8 {
        self.mapper.read_ram(&self.ram, address)
    }

    pub(crate) fn write_ram(&mut self, address: u16, value: u8) {
        self.mapper.write_ram(&mut self.ram, address, value);
    }

    pub(crate) fn save_data(&self) -> SaveData {
        if !self.mapper.has_battery() {
            return SaveData::default();
        }

        let ram = (!self.ram.is_empty()).then_some(&self.ram[..]);
        let rtc = if let Mapper::Mbc3(mapper) = &self.mapper {
            mapper.rtc_data()
        } else {
            None
        };

        SaveData { ram, rtc }
    }
}

mod mbc1;
mod mbc2;
mod rom_only;

use enum_dispatch::enum_dispatch;

use self::{mbc1::Mbc1, mbc2::Mbc2, rom_only::RomOnly};
use crate::error::Error;

#[derive(Debug)]
pub enum Destination {
    Japanese,
    NonJapanese,
}

#[derive(Debug)]
pub struct Header {
    pub title: String,
    pub rom_size: u32,
    pub rom_bank_count: u32,
    pub ram_size: u32,
    pub ram_bank_count: u32,
    pub destination: Destination,
}

impl Header {
    fn parse(rom_bytes: &[u8]) -> Result<(Self, Mapper), Error> {
        if rom_bytes.len() < 0x014F {
            return Err(Error::InvalidRomHeader("Header is too short"));
        }

        let title = String::from_utf8(
            rom_bytes[0x0134..=0x143]
                .splitn(2, |byte| *byte == 0)
                .next()
                .unwrap()
                .to_owned(),
        )
        .map_err(|_| Error::InvalidRomHeader("Could not parse title"))?;

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

            0x05 => Mapper::Mbc2(Mbc2::new(rom_bank_count as u16, false)),
            0x06 => Mapper::Mbc2(Mbc2::new(rom_bank_count as u16, true)),
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
trait MapperOps {
    fn read_rom(&mut self, rom: &[u8], address: u16) -> u8;
    fn write_rom(&mut self, rom: &mut [u8], address: u16, value: u8);
    fn read_ram(&mut self, ram: &[u8], address: u16) -> u8;
    fn write_ram(&mut self, rom: &mut [u8], address: u16, value: u8);
}

#[enum_dispatch]
#[derive(Debug)]
pub enum Mapper {
    RomOnly(RomOnly),
    Mbc1(Mbc1),
    Mbc2(Mbc2),
}

#[derive(Debug)]
pub struct Cartridge {
    header: Header,
    mapper: Mapper,
    bootrom: Option<Vec<u8>>,
    pub bootrom_enabled: bool,
    rom: Vec<u8>,
    ram: Vec<u8>,
}

impl Cartridge {
    pub const BOOTROM_END: u16 = 0x0100;

    pub fn new(rom: Vec<u8>, bootrom: Option<Vec<u8>>) -> Result<Self, Error> {
        let (header, mapper) = Header::parse(&rom)?;
        // Mbc2 has 512 half-bytes of internal RAM that are not reported in the header.
        let ram_size = if let Mapper::Mbc2(_) = mapper {
            Mbc2::RAM_SIZE
        } else {
            header.ram_size as usize
        };
        let ram = vec![0; ram_size];
        let bootrom_enabled = bootrom.is_some();
        if let Some(bootrom) = &bootrom {
            if bootrom.len() != 0x100 {
                // TODO Checksum ?
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

    pub const fn header(&self) -> &Header {
        &self.header
    }

    pub const fn mapper(&self) -> &Mapper {
        &self.mapper
    }

    pub fn disable_bootrom(&mut self) {
        self.bootrom_enabled = false;
    }

    pub fn read_rom(&mut self, address: u16) -> u8 {
        match &self.bootrom {
            Some(bootrom) if self.bootrom_enabled && address <= Self::BOOTROM_END => {
                bootrom[address as usize]
            }
            _ => self.mapper.read_rom(&self.rom, address),
        }
    }

    pub fn write_rom(&mut self, address: u16, value: u8) {
        match &mut self.bootrom {
            Some(bootrom) if self.bootrom_enabled && address <= Self::BOOTROM_END => {
                bootrom[address as usize] = value
            }
            _ => self.mapper.write_rom(&mut self.rom, address, value),
        }
    }

    pub fn read_ram(&mut self, address: u16) -> u8 {
        self.mapper.read_ram(&self.ram, address)
    }

    pub fn write_ram(&mut self, address: u16, value: u8) {
        self.mapper.write_ram(&mut self.ram, address, value);
    }
}

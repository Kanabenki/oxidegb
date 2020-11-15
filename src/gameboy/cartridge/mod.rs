mod mbc1;
mod rom_only;

use enum_dispatch::enum_dispatch;

use self::{mbc1::Mbc1, rom_only::RomOnly};
use crate::error::Error;

#[derive(Debug)]
enum Destination {
    Japanese,
    NonJapanese,
}

#[derive(Debug)]
struct Header {
    title: String,
    rom_size: u32,
    rom_bank_count: u32,
    ram_size: u32,
    ram_bank_count: u32,
    destination: Destination,
}

impl Header {
    fn parse(rom_bytes: &[u8]) -> Result<(Self, MapperKind), Error> {
        if rom_bytes.len() < 0x014F {
            return Err(Error::InvalidRomHeader("Header is too short".into()));
        }

        let title = String::from_utf8(
            rom_bytes[0x0134..=0x143]
                .splitn(2, |byte| *byte == 0)
                .next()
                .unwrap()
                .to_owned(),
        )
        .map_err(|_| Error::InvalidRomHeader("Could not parse title".into()))?;

        let rom_bank_count = match rom_bytes[0x148] {
            rom_bank_byte @ 0x00..=0x08 => 2 << rom_bank_byte,
            _ => return Err(Error::InvalidRomHeader("Invalid rom bank count".into())),
        };

        let rom_size = 0x4000 * rom_bank_count;

        let (ram_bank_count, ram_size) = match rom_bytes[0x149] {
            0x00 => (0, 0),
            0x01 => (1, 0x2000),
            0x02 => (1, 0x0800),
            0x03 => (4, 0x2000 * 4),
            0x04 => (16, 0x2000 * 16),
            0x05 => (8, 0x2000 * 8),
            _ => return Err(Error::InvalidRomHeader("Invalid ram bank count".into())),
        };

        let mapper = match rom_bytes[0x147] {
            0x00 => MapperKind::RomOnly(RomOnly),
            0x01 => MapperKind::Mbc1(Mbc1::new(rom_bank_count as u16, false, false)),
            _ => return Err(Error::UnsupportedMapper),
        };

        let destination = match rom_bytes[0x14A] {
            0x00 => Destination::Japanese,
            0x01 => Destination::NonJapanese,
            _ => return Err(Error::InvalidRomHeader("Invalid destination".into())),
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

#[enum_dispatch(MapperKind)]
trait Mapper {
    fn read_rom(&mut self, rom: &[u8], address: u16) -> u8;
    fn write_rom(&mut self, rom: &mut [u8], address: u16, value: u8);
    fn read_ram(&mut self, ram: &[u8], address: u16) -> u8;
    fn write_ram(&mut self, rom: &mut [u8], address: u16, value: u8);
}

#[enum_dispatch]
#[derive(Debug)]
enum MapperKind {
    RomOnly(RomOnly),
    Mbc1(Mbc1),
}

#[derive(Debug)]
pub struct Cartridge {
    header: Header,
    mapper: MapperKind,
    bootrom: Option<Vec<u8>>,
    pub bootrom_enabled: bool,
    rom: Vec<u8>,
    ram: Vec<u8>,
}

impl Cartridge {
    pub const BOOTROM_END: u16 = 0x0100;

    pub fn new(rom: Vec<u8>, bootrom: Option<Vec<u8>>) -> Result<Self, Error> {
        let (header, mapper) = Header::parse(&rom)?;
        let ram = vec![0; header.ram_size as usize];
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
        self.mapper.write_ram(&mut self.ram, address, value)
    }
}

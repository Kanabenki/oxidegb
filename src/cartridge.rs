use std::ffi::CStr;

use enum_dispatch::enum_dispatch;

use crate::error::Error;

enum Destination {
    Japanese,
    NonJapanese,
}

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
            return Err(Error::InvalidRomHeader);
        }
        let title = CStr::from_bytes_with_nul(&rom_bytes[0x0134..=0x143])
            .or(Err(Error::InvalidRomHeader))?
            .to_str()
            .or(Err(Error::InvalidRomHeader))?
            .to_owned();

        let mapper = match rom_bytes[0x147] {
            0x00 => MapperKind::RomOnly(RomOnly),
            _ => return Err(Error::UnsupportedMapper),
        };

        let rom_bank_count = match rom_bytes[0x148] {
            rom_bank_byte @ 0x00..=0x08 => 2 << rom_bank_byte,
            0x52 => 72,
            0x53 => 80,
            0x54 => 96,
            _ => return Err(Error::InvalidRomHeader),
        };

        let rom_size = 0x4000 * rom_bank_count;

        let (ram_bank_count, ram_size) = match rom_bytes[0x149] {
            0x00 => (0, 0),
            0x01 => (1, 0x0800),
            0x02 => (1, 0x2000),
            0x03 => (4, 0x2000 * 4),
            0x04 => (16, 0x2000 * 16),
            0x05 => (8, 0x2000 * 8),
            _ => return Err(Error::InvalidRomHeader),
        };

        let destination = match rom_bytes[0x14A] {
            0x00 => Destination::Japanese,
            0x01 => Destination::NonJapanese,
            _ => return Err(Error::InvalidRomHeader),
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

struct RomOnly;

impl Mapper for RomOnly {
    fn read_rom(&mut self, rom: &[u8], address: u16) -> u8 {
        rom[address as usize]
    }

    fn write_rom(&mut self, rom: &mut [u8], address: u16, value: u8) {}

    fn read_ram(&mut self, ram: &[u8], address: u16) -> u8 {
        todo!()
    }

    fn write_ram(&mut self, rom: &mut [u8], address: u16, value: u8) {
        todo!()
    }
}

#[enum_dispatch]
enum MapperKind {
    RomOnly(RomOnly),
}

pub struct Cartridge {
    header: Header,
    mapper: MapperKind,
    bootrom: Option<Vec<u8>>,
    pub bootrom_enabled: bool,
    rom: Vec<u8>,
    ram: Vec<u8>,
    has_battery: bool,
}

impl Cartridge {
    fn new(rom_bytes: Vec<u8>, boot_rom_bytes: Option<Vec<u8>>) -> Result<Self, Error> {
        todo!()
    }

    pub fn read_rom(&mut self, address: u16) -> u8 {
        match &self.bootrom {
            Some(bootrom) if self.bootrom_enabled && address <= 0x0100 => bootrom[address as usize],
            _ => self.mapper.read_rom(&self.rom, address),
        }
    }

    pub fn write_rom(&mut self, address: u16, value: u8) {
        match &mut self.bootrom {
            Some(bootrom) if self.bootrom_enabled && address <= 0x0100 => {
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

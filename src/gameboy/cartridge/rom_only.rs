use serde::{Deserialize, Serialize};

use super::MapperOps;

#[derive(Serialize, Deserialize, Debug)]
pub struct RomOnly;

impl MapperOps for RomOnly {
    fn read_rom(&mut self, rom: &[u8], address: u16) -> u8 {
        rom[address as usize]
    }

    fn write_rom(&mut self, _rom: &mut [u8], _address: u16, _value: u8) {}

    fn read_ram(&mut self, _ram: &[u8], _address: u16) -> u8 {
        0xFF
    }

    fn write_ram(&mut self, _rom: &mut [u8], _address: u16, _value: u8) {}
}

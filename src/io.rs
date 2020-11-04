use crate::mmu::MemoryOps;

pub struct Io {}

impl MemoryOps for Io {
    fn read_byte(&mut self, address: u16) -> u8 {
        todo!()
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        todo!()
    }
}

use crate::buttons::Buttons;

pub trait MemoryOps {
    fn read_byte(&mut self, address: u16) -> u8;

    fn write_byte(&mut self, value: u8, address: u16);

    fn read_dbyte(&mut self, address: u16) -> u16 {
        let lower = self.read_byte(address) as u16;
        let upper = (self.read_byte(address + 1) as u16) << 8;
        upper | lower
    }

    fn write_dbyte(&mut self, value: u16, address: u16) {
        self.write_byte(value as u8, address);
        self.write_byte((value >> 8) as u8, address + 1);
    }
}

pub struct Mmu {
    buttons: Buttons,
}

impl MemoryOps for Mmu {
    fn read_byte(&mut self, address: u16) -> u8 {
        todo!()
    }

    fn write_byte(&mut self, value: u8, address: u16) {
        todo!()
    }
}

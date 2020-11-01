use crate::buttons::Buttons;

pub trait MemoryOps {
    fn read_byte(&self, address: u16) -> u8;

    fn write_byte(&mut self, address: u16, value: u8);

    fn read_dbyte(&mut self, address: u16) -> u16 {
        let low = self.read_byte(address);
        let high = self.read_byte(address + 1);
        u16::from_be_bytes([high, low])
    }

    fn write_dbyte(&mut self, address: u16, value: u16) {
        let [high, low] = value.to_be_bytes();
        self.write_byte(address, low);
        self.write_byte(address + 1, high);
    }
}

pub struct Mmu {
    buttons: Buttons,
}

impl MemoryOps for Mmu {
    fn read_byte(&self, address: u16) -> u8 {
        todo!()
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        todo!()
    }
}

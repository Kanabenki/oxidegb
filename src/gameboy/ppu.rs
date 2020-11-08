#[derive(Debug, Copy, Clone)]
pub struct Color([u8; 4]);

pub struct Ppu {
    screen: [Color; 166 * 144],
    vram: [u8; 8192],
    oam: [u8; 0x9F],
}

impl Ppu {
    pub fn new() -> Self {
        Self {
            screen: [Color([0, 0, 0, 0]); 166 * 144],
            vram: [0; 8192],
            oam: [0; 0x9F],
        }
    }

    pub fn screen(&self) -> &[Color; 166 * 144] {
        &self.screen
    }

    pub fn read_vram(&self, address: u16) -> u8 {
        self.vram[address as usize]
    }

    pub fn write_vram(&mut self, address: u16, value: u8) {
        self.vram[address as usize] = value;
    }

    pub fn read_oam(&self, address: u16) -> u8 {
        self.oam[address as usize]
    }

    pub fn write_oam(&mut self, address: u16, value: u8) {
        self.oam[address as usize] = value;
    }
}

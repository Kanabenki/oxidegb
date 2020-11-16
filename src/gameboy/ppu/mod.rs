mod lcd_control;
mod lcd_status;
mod palette;

use self::{lcd_control::LcdControl, lcd_status::LcdStatus, palette::Palette};

#[derive(Debug, Copy, Clone)]
pub struct Color([u8; 4]);

pub struct Ppu {
    screen: [Color; 166 * 144],
    vram: [u8; 8192],
    oam: [u8; 0x9F],
    lcdc: LcdControl,
    stat: LcdStatus,
    bg_palette: Palette,
    obj_palette_0: Palette,
    obj_palette_1: Palette,
}

impl Ppu {
    const LCDC: u16 = 0xFF40;
    const STAT: u16 = 0xFF41;
    const BG_PALETTE: u16 = 0xFF47;
    const OBJ_PALETTE_0: u16 = 0xFF48;
    const OBJ_PALETTE_1: u16 = 0xFF49;

    pub fn new() -> Self {
        Self {
            screen: [Color([0, 0, 0, 0]); 166 * 144],
            vram: [0; 8192],
            oam: [0; 0x9F],
            lcdc: LcdControl::new(),
            stat: LcdStatus::new(),
            bg_palette: Palette::new(),
            obj_palette_0: Palette::new(),
            obj_palette_1: Palette::new(),
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

    pub fn read_registers(&self, address: u16) -> u8 {
        match address {
            Self::LCDC => self.lcdc.value(),
            Self::STAT => self.stat.value(),
            Self::BG_PALETTE => self.bg_palette.value(),
            Self::OBJ_PALETTE_0 => self.obj_palette_0.value(),
            Self::OBJ_PALETTE_1 => self.obj_palette_1.value(),
            _ => panic!("Tried to read ppu register at invalid address"),
        }
    }

    pub fn write_registers(&mut self, address: u16, value: u8) {
        match address {
            Self::LCDC => self.lcdc.set_value(value),
            Self::STAT => self.stat.set_value(value),
            Self::BG_PALETTE => self.bg_palette.set_value(value),
            Self::OBJ_PALETTE_0 => self.obj_palette_0.set_value(value),
            Self::OBJ_PALETTE_1 => self.obj_palette_1.set_value(value),
            _ => panic!("Tried to write ppu register at invalid address"),
        }
    }
}

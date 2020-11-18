mod lcd_control;
mod lcd_status;
mod palette;
mod pixel_transfer;
mod sprite;

use self::{
    lcd_control::LcdControl,
    lcd_status::{LcdStatus, Mode},
    palette::Palette,
    pixel_transfer::PixelFifo,
};

#[derive(Debug, Copy, Clone)]
pub struct Color([u8; 4]);

pub struct Ppu {
    screen: [Color; 166 * 144],
    vram: [u8; 8192],
    oam: [u8; Self::OAM_SIZE],
    bg_pixel_fifo: PixelFifo,
    obj_pixel_fifo: PixelFifo,
    visible_sprites: Vec<sprite::Attributes>,
    oam_index: usize,
    lcdc: LcdControl,
    stat: LcdStatus,
    bg_palette: Palette,
    obj_palette_0: Palette,
    obj_palette_1: Palette,
    scroll_y: u8,
    scroll_x: u8,
    x_pos: u8,
    line_y: u8,
    line_y_compare: u8,
    window_y: u8,
    window_x: u8,
    line_cycles_count: u8,
}

impl Ppu {
    const OAM_SIZE: usize = 0x9F;

    const LCD_SIZE_X: u8 = 166;
    const LCD_SIZE_Y: u8 = 144;

    const LAST_VISIBLE_LINE: u8 = Self::LCD_SIZE_Y - 1;

    const LCDC: u16 = 0xFF40;
    const STAT: u16 = 0xFF41;
    const SCROLL_Y: u16 = 0xFF42;
    const SCROLL_X: u16 = 0xFF43;
    const LINE_Y: u16 = 0xFF44;
    const LINE_Y_COMPARE: u16 = 0xFF45;
    const BG_PALETTE: u16 = 0xFF47;
    const OBJ_PALETTE_0: u16 = 0xFF48;
    const OBJ_PALETTE_1: u16 = 0xFF49;
    const WINDOW_Y: u16 = 0xFF4A;
    const WINDOW_X: u16 = 0xFF4B;

    const _WINDOW_X_OFFSET: u8 = 7;

    const CYCLES_PER_LINE: u8 = 114;
    const LINES_PER_FRAME: u8 = 154;

    const MAX_VISIBLE_SPRITES: usize = 10;

    pub fn new() -> Self {
        Self {
            screen: [Color([0, 0, 0, 0]); 166 * 144],
            vram: [0; 8192],
            oam: [0; Self::OAM_SIZE],
            bg_pixel_fifo: PixelFifo::new(),
            obj_pixel_fifo: PixelFifo::new(),
            visible_sprites: Vec::with_capacity(10),
            oam_index: 0,
            lcdc: LcdControl::new(),
            stat: LcdStatus::new(),
            bg_palette: Palette::new(),
            obj_palette_0: Palette::new(),
            obj_palette_1: Palette::new(),
            scroll_x: 0,
            scroll_y: 0,
            x_pos: 0,
            line_y: 0,
            line_y_compare: 0,
            window_y: 0,
            window_x: 0,
            line_cycles_count: 0,
        }
    }

    pub fn tick(&mut self) {
        match self.stat.mode {
            Mode::OamSearch => {
                let i = self.oam_index;
                let sprite = sprite::Attributes::parse([
                    self.oam[i],
                    self.oam[i + 1],
                    self.oam[i + 2],
                    self.oam[i + 3],
                ]);

                if self.visible_sprites.len() < Self::MAX_VISIBLE_SPRITES
                    && sprite.x != 0
                    && sprite.y <= self.line_y + 16
                    && self.line_y + 16 < sprite.y + self.lcdc.obj_size.height()
                {
                    self.visible_sprites.push(sprite);
                }

                self.oam_index += 4;

                if self.oam_index == Self::OAM_SIZE {
                    self.oam_index = 0;
                    self.visible_sprites.clear();
                    self.stat.mode = Mode::PixelTransfer;
                }
            }
            Mode::PixelTransfer => {
                if self.x_pos == 160 {
                    self.bg_pixel_fifo.clear();
                    self.obj_pixel_fifo.clear();
                    self.stat.mode = Mode::HBlank;
                    self.x_pos = 0;
                }
            }
            Mode::HBlank | Mode::VBlank => {}
        }

        self.line_cycles_count = (self.line_cycles_count + 1) % Self::CYCLES_PER_LINE;
        if self.line_cycles_count == 0 {
            self.line_y = (self.line_y + 1) % Self::LINES_PER_FRAME;
            match self.line_y {
                0..=Self::LAST_VISIBLE_LINE => self.stat.mode = Mode::OamSearch,
                Self::LCD_SIZE_Y => self.stat.mode = Mode::VBlank,
                _ => {}
            }
        }
    }

    pub fn screen(&self) -> &[Color; Self::LCD_SIZE_X as usize * Self::LCD_SIZE_Y as usize] {
        &self.screen
    }

    pub fn read_vram(&self, address: u16) -> u8 {
        match self.stat.mode {
            Mode::OamSearch | Mode::HBlank | Mode::VBlank => self.vram[address as usize],
            Mode::PixelTransfer => 0xFF,
        }
    }

    pub fn write_vram(&mut self, address: u16, value: u8) {
        match self.stat.mode {
            Mode::OamSearch | Mode::HBlank | Mode::VBlank => self.vram[address as usize] = value,
            Mode::PixelTransfer => {}
        }
    }

    pub fn read_oam(&self, address: u16) -> u8 {
        match self.stat.mode {
            Mode::HBlank | Mode::VBlank => self.oam[address as usize],
            Mode::OamSearch | Mode::PixelTransfer => 0xFF,
        }
    }

    pub fn write_oam(&mut self, address: u16, value: u8) {
        match self.stat.mode {
            Mode::HBlank | Mode::VBlank => self.oam[address as usize] = value,
            Mode::OamSearch | Mode::PixelTransfer => {}
        }
    }

    pub fn read_registers(&self, address: u16) -> u8 {
        match address {
            Self::LCDC => self.lcdc.value(),
            Self::STAT => self.stat.value(),
            Self::SCROLL_Y => self.scroll_y,
            Self::SCROLL_X => self.scroll_x,
            Self::LINE_Y => self.line_y,
            Self::LINE_Y_COMPARE => self.line_y_compare,
            Self::BG_PALETTE => self.bg_palette.value(),
            Self::OBJ_PALETTE_0 => self.obj_palette_0.value(),
            Self::OBJ_PALETTE_1 => self.obj_palette_1.value(),
            Self::WINDOW_Y => self.window_y,
            Self::WINDOW_X => self.window_x,
            _ => panic!("Tried to read ppu register at invalid address"),
        }
    }

    pub fn write_registers(&mut self, address: u16, value: u8) {
        match address {
            Self::LCDC => self.lcdc.set_value(value),
            Self::STAT => self.stat.set_value(value),
            Self::SCROLL_Y => self.scroll_y = value,
            Self::SCROLL_X => self.scroll_x = value,
            Self::LINE_Y => todo!(),
            Self::LINE_Y_COMPARE => self.line_y_compare = value,
            Self::BG_PALETTE => self.bg_palette.set_value(value),
            Self::OBJ_PALETTE_0 => self.obj_palette_0.set_value(value),
            Self::OBJ_PALETTE_1 => self.obj_palette_1.set_value(value),
            Self::WINDOW_Y => self.window_y = value,
            Self::WINDOW_X => self.window_x = value,
            _ => panic!("Tried to write ppu register at invalid address"),
        }
    }
}

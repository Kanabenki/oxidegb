mod lcd_control;
mod lcd_status;
mod obj;
mod palette;
mod pixel_transfer;

use std::convert::TryInto;

use flagset::FlagSet;

use self::{
    lcd_control::LcdControl,
    lcd_status::{LcdStatus, Mode},
    palette::Palette,
    pixel_transfer::{mix_pixels, BgPixel, Fetcher, ObjPixel, PixelFifo},
};
use super::interrupts::Interrupt;

#[derive(Debug)]
pub enum DmaRequest {
    None,
    Start(u8),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Color([u8; 4]);

impl From<palette::Color> for Color {
    fn from(color: palette::Color) -> Self {
        match color {
            palette::Color::White => Self([0xE0, 0xF8, 0xD0, 0xFF]),
            palette::Color::LightGray => Self([0x88, 0xC0, 0x70, 0xFF]),
            palette::Color::DarkGray => Self([0x34, 0x68, 0x56, 0xFF]),
            palette::Color::Black => Self([0x08, 0x18, 0x20, 0xFF]),
        }
    }
}

impl From<Color> for [u8; 4] {
    fn from(val: Color) -> Self {
        val.0
    }
}

impl From<[u8; 4]> for Color {
    fn from(val: [u8; 4]) -> Self {
        Self(val)
    }
}

#[derive(Debug, Default)]
pub struct Palettes {
    pub bg: Palette,
    pub obj_0: Palette,
    pub obj_1: Palette,
}

#[derive(Debug)]
pub struct Ppu {
    screen: [Color; Self::LCD_SIZE_X as usize * Self::LCD_SIZE_Y as usize],
    vram: [u8; Self::VRAM_SIZE],
    oam: [u8; Self::OAM_SIZE],
    bg_fifo: PixelFifo<BgPixel>,
    obj_fifo: PixelFifo<ObjPixel>,
    fetcher: Fetcher,
    visible_objs: Vec<obj::Attributes>,
    dma: DmaRequest,
    dma_address: u8,
    oam_index: usize,
    lcdc: LcdControl,
    stat: LcdStatus,
    palettes: Palettes,
    scroll_y: u8,
    scroll_x: u8,
    to_discard_x: u8,
    x_pos: u8,
    pub line_y: u8,
    line_y_compare: u8,
    line_y_window: u8,
    window_triggered: bool,
    window_y: u8,
    window_x: u8,
    line_cycles_count: u8,
}

impl Ppu {
    pub const OAM_SIZE: usize = 0xA0;
    const VRAM_SIZE: usize = 0x2000;

    const LCD_SIZE_X: u8 = 160;
    const LCD_SIZE_Y: u8 = 144;

    const LAST_VISIBLE_LINE: u8 = Self::LCD_SIZE_Y - 1;

    const LCDC: u16 = 0xFF40;
    const STAT: u16 = 0xFF41;
    const SCROLL_Y: u16 = 0xFF42;
    const SCROLL_X: u16 = 0xFF43;
    const LINE_Y: u16 = 0xFF44;
    const LINE_Y_COMPARE: u16 = 0xFF45;
    const OAM_DMA: u16 = 0xFF46;
    const BG_PALETTE: u16 = 0xFF47;
    const OBJ_PALETTE_0: u16 = 0xFF48;
    const OBJ_PALETTE_1: u16 = 0xFF49;
    const WINDOW_Y: u16 = 0xFF4A;
    const WINDOW_X: u16 = 0xFF4B;
    const UNUSED_START: u16 = 0xFF4C;
    const UNUSED_END: u16 = 0xFF4E;
    const CGB_VRAM_BANK_SELECT: u16 = 0xFF4F;

    const _WINDOW_X_OFFSET: u8 = 7;

    const CYCLES_PER_LINE: u8 = 114;
    const LINES_PER_FRAME: u8 = 154;

    const MAX_VISIBLE_SPRITES: usize = 10;

    pub fn new() -> Self {
        Self {
            screen: [palette::Color::Black.into();
                Self::LCD_SIZE_X as usize * Self::LCD_SIZE_Y as usize],
            vram: [0; Self::VRAM_SIZE],
            oam: [0; Self::OAM_SIZE],
            bg_fifo: PixelFifo::new(),
            obj_fifo: PixelFifo::new(),
            fetcher: Fetcher::new(),
            visible_objs: Vec::with_capacity(10),
            dma: DmaRequest::None,
            dma_address: 0,
            oam_index: 0,
            lcdc: LcdControl::new(),
            stat: LcdStatus::new(),
            palettes: Palettes::default(),
            scroll_x: 0,
            to_discard_x: 0,
            scroll_y: 0,
            x_pos: 0,
            line_y: 0,
            line_y_compare: 0,
            line_y_window: 0,
            window_triggered: false,
            window_y: 0,
            window_x: 0,
            line_cycles_count: 0,
        }
    }

    pub fn new_post_bootrom() -> Self {
        let mut ppu = Self::new();
        ppu.lcdc.lcd_enable = true;
        ppu
    }

    pub fn tick(&mut self) -> (FlagSet<Interrupt>, DmaRequest) {
        if !self.lcdc.lcd_enable {
            self.line_y = 0;
            self.stat.mode = Mode::VBlank;
            return (FlagSet::default(), DmaRequest::None);
        }

        let mut interrupts = FlagSet::default();

        match self.stat.mode {
            Mode::OamSearch => {
                self.tick_oam_search();
                self.tick_oam_search();

                if self.oam_index == Self::OAM_SIZE {
                    self.oam_index = 0;
                    self.to_discard_x = self.scroll_x & 0b111;
                    self.stat.mode = Mode::PixelTransfer;

                    if self.stat.oam_interrupt_enabled() {
                        // TODO: handle stat blocking
                        interrupts |= Interrupt::LcdStat;
                    }
                }
            }
            Mode::PixelTransfer => {
                for _ in 0..4 {
                    self.tick_pixel_transfer();

                    if self.x_pos == Self::LCD_SIZE_X + 8 {
                        self.bg_fifo.clear();
                        self.fetcher.start_line();
                        self.visible_objs.clear();
                        self.x_pos = 0;
                        if self.window_triggered {
                            self.line_y_window += 1;
                        }
                        self.window_triggered = false;

                        self.stat.mode = Mode::HBlank;

                        if self.stat.hblank_interrupt_enabled() {
                            interrupts |= Interrupt::LcdStat;
                        }

                        break;
                    }
                }
            }
            Mode::HBlank | Mode::VBlank => {}
        }

        self.line_cycles_count = (self.line_cycles_count + 1) % Self::CYCLES_PER_LINE;
        if self.line_cycles_count == 0 {
            self.line_y = (self.line_y + 1) % Self::LINES_PER_FRAME;

            self.stat.lyc_coincidence = self.line_y == self.line_y_compare;
            if self.stat.lyc_coincidence && self.stat.coincidence_interrupt_enabled() {
                interrupts |= Interrupt::LcdStat;
            }

            match self.line_y {
                0..=Self::LAST_VISIBLE_LINE => self.stat.mode = Mode::OamSearch,
                Self::LCD_SIZE_Y => {
                    self.line_y_window = 0;
                    self.stat.mode = Mode::VBlank;
                    interrupts |= Interrupt::VBlank;
                    if self.stat.vblank_interrupt_enabled() {
                        interrupts |= Interrupt::LcdStat;
                    }
                }
                _ => {}
            }
        }

        (
            interrupts,
            std::mem::replace(&mut self.dma, DmaRequest::None),
        )
    }

    fn tick_oam_search(&mut self) {
        let obj = obj::Attributes::parse(
            self.oam[self.oam_index..self.oam_index + 4]
                .try_into()
                .unwrap(),
        );

        if self.visible_objs.len() < Self::MAX_VISIBLE_SPRITES
            && obj.x != 0
            && obj.y <= self.line_y + 16
            && self.line_y + 16 < obj.y + self.lcdc.obj_size.height()
        {
            self.visible_objs.push(obj);
        }

        self.oam_index += 4;
    }

    fn tick_pixel_transfer(&mut self) {
        if !self.window_triggered
            && self.lcdc.window_enable
            && self.x_pos == self.window_x + 1
            && self.line_y >= self.window_y
        {
            self.window_triggered = true;
            self.bg_fifo.clear();
            self.fetcher.start_window();
        }

        let pending = self.fetcher.tick(
            &mut self.bg_fifo,
            &mut self.obj_fifo,
            &self.visible_objs,
            &self.lcdc,
            self.x_pos,
            self.line_y,
            self.scroll_x,
            self.scroll_y,
            &self.vram,
            self.window_triggered,
            self.line_y_window,
        );

        if pending {
            return;
        }

        if let Some(bg_pixel) = self.bg_fifo.pop() {
            if self.to_discard_x > 0 {
                self.to_discard_x -= 1;
            } else {
                let color = mix_pixels(bg_pixel, self.obj_fifo.pop(), &self.lcdc, &self.palettes);
                if self.x_pos >= 8 {
                    self.screen[(self.x_pos - 8) as usize
                        + (self.line_y as usize * Self::LCD_SIZE_X as usize)] = color.into();
                }
                self.x_pos += 1;
            }
        }
    }

    pub const fn screen(&self) -> &[Color; Self::LCD_SIZE_X as usize * Self::LCD_SIZE_Y as usize] {
        &self.screen
    }

    pub const fn read_vram(&self, address: u16) -> u8 {
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

    pub const fn read_oam(&self, address: u16) -> u8 {
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
            Self::OAM_DMA => self.dma_address,
            Self::BG_PALETTE => self.palettes.bg.value(),
            Self::OBJ_PALETTE_0 => self.palettes.obj_0.value(),
            Self::OBJ_PALETTE_1 => self.palettes.obj_1.value(),
            Self::WINDOW_Y => self.window_y,
            Self::WINDOW_X => self.window_x,
            Self::UNUSED_START..=Self::UNUSED_END | Self::CGB_VRAM_BANK_SELECT => 0xFF,
            _ => panic!("Tried to read ppu register at invalid address"),
        }
    }

    pub fn write_registers(&mut self, address: u16, value: u8) {
        match address {
            Self::LCDC => {
                let enabled = self.lcdc.lcd_enable;
                self.lcdc.set_value(value);
                if !enabled && self.lcdc.lcd_enable {
                    self.stat.mode = Mode::OamSearch;
                }
            }
            Self::STAT => self.stat.set_value(value),
            Self::SCROLL_Y => self.scroll_y = value,
            Self::SCROLL_X => self.scroll_x = value,
            Self::LINE_Y => {}
            Self::LINE_Y_COMPARE => self.line_y_compare = value,
            Self::OAM_DMA => {
                self.dma = DmaRequest::Start(value);
                self.dma_address = value;
            }
            Self::BG_PALETTE => self.palettes.bg.set_value(value),
            Self::OBJ_PALETTE_0 => self.palettes.obj_0.set_value(value),
            Self::OBJ_PALETTE_1 => self.palettes.obj_1.set_value(value),
            Self::WINDOW_Y => self.window_y = value,
            Self::WINDOW_X => self.window_x = value,
            Self::UNUSED_START..=Self::UNUSED_END | Self::CGB_VRAM_BANK_SELECT => {}
            invalid_address => panic!(
                "Tried to write ppu register at invalid address 0x{:X}",
                invalid_address
            ),
        }
    }
}

mod lcd_control;
mod lcd_status;
mod palette;

use self::{lcd_control::LcdControl, lcd_status::LcdStatus, palette::Palette};

#[derive(Debug, Copy, Clone)]
pub struct Color([u8; 4]);

struct PixelFifo {
    _fifo: [palette::Color; 16],
    _start: usize,
    _end: usize,
}

impl PixelFifo {
    const SIZE: usize = 16;

    fn new() -> Self {
        Self {
            _fifo: [palette::Color::White; Self::SIZE],
            _start: 0,
            _end: 0,
        }
    }

    fn _size(&self) -> usize {
        (self._end as isize - self._start as isize
            + (if self._start > self._end {
                Self::SIZE as isize
            } else {
                0
            })) as usize
    }

    fn _push_line(&mut self, colors: &[palette::Color; 8]) -> bool {
        if self._size() <= 8 {
            for (i, color) in colors.iter().cloned().enumerate() {
                self._fifo[self._start + (i % Self::SIZE)] = color;
            }
            self._end = (self._end + 1) % 16;
            true
        } else {
            false
        }
    }

    fn _pop(&mut self) -> Option<palette::Color> {
        if self._start != self._end {
            let color = Some(self._fifo[self._start]);
            self._start = (self._start + 1) % Self::SIZE;
            color
        } else {
            None
        }
    }
}

pub struct Ppu {
    screen: [Color; 166 * 144],
    vram: [u8; 8192],
    oam: [u8; 0x9F],
    _pixel_fifo: PixelFifo,
    lcdc: LcdControl,
    stat: LcdStatus,
    bg_palette: Palette,
    obj_palette_0: Palette,
    obj_palette_1: Palette,
    scroll_y: u8,
    scroll_x: u8,
    line_y: u8,
    line_y_compare: u8,
    window_y: u8,
    window_x: u8,
}

impl Ppu {
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

    pub fn new() -> Self {
        Self {
            screen: [Color([0, 0, 0, 0]); 166 * 144],
            vram: [0; 8192],
            oam: [0; 0x9F],
            _pixel_fifo: PixelFifo::new(),
            lcdc: LcdControl::new(),
            stat: LcdStatus::new(),
            bg_palette: Palette::new(),
            obj_palette_0: Palette::new(),
            obj_palette_1: Palette::new(),
            scroll_x: 0,
            scroll_y: 0,
            line_y: 0,
            line_y_compare: 0,
            window_y: 0,
            window_x: 0,
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

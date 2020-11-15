#[derive(Debug, Copy, Clone)]
pub struct Color([u8; 4]);

enum TileMapRange {
    Low,
    High,
}

enum TileDataAddressing {
    Unsigned,
    Signed,
}

enum SpriteSize {
    S8x8,
    S8x16,
}

struct LcdControl {
    lcd_enable: bool,
    window_tile_map: TileMapRange,
    window_enable: bool,
    bg_window_addressing: TileDataAddressing,
    bg_tile_map: TileMapRange,
    obj_size: SpriteSize,
    obj_enable: bool,
    bg_window_enable: bool,
}

impl LcdControl {
    fn new() -> Self {
        Self {
            lcd_enable: true,
            window_tile_map: TileMapRange::Low,
            window_enable: false,
            bg_window_addressing: TileDataAddressing::Unsigned,
            bg_tile_map: TileMapRange::Low,
            obj_size: SpriteSize::S8x8,
            obj_enable: true,
            bg_window_enable: true,
        }
    }

    fn set_value(&mut self, value: u8) {
        self.lcd_enable = value & (1 << 7) != 0;
        self.window_tile_map = if value & (1 << 6) == 0 {
            TileMapRange::Low
        } else {
            TileMapRange::High
        };
        self.window_enable = value & (1 << 5) != 0;
        self.bg_window_addressing = if value & (1 << 4) == 0 {
            TileDataAddressing::Unsigned
        } else {
            TileDataAddressing::Signed
        };
        self.bg_tile_map = if value & (1 << 3) != 0 {
            TileMapRange::Low
        } else {
            TileMapRange::High
        };
        self.obj_size = if value & (1 << 2) != 0 {
            SpriteSize::S8x8
        } else {
            SpriteSize::S8x16
        };
        self.obj_enable = value & (1 << 1) != 0;
        self.bg_window_enable = value & 1 != 0;
    }
}

pub struct Ppu {
    screen: [Color; 166 * 144],
    vram: [u8; 8192],
    oam: [u8; 0x9F],
    lcdc: LcdControl,
}

impl Ppu {
    pub fn new() -> Self {
        Self {
            screen: [Color([0, 0, 0, 0]); 166 * 144],
            vram: [0; 8192],
            oam: [0; 0x9F],
            lcdc: LcdControl::new(),
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

    pub fn read_registers(&self, _address: u16) -> u8 {
        todo!()
    }

    pub fn write_registers(&mut self, address: u16, value: u8) {
        match address {
            0xFF40 => self.lcdc.set_value(value),
            _ => panic!("Tried to write ppu register at invalid address"),
        }
    }
}

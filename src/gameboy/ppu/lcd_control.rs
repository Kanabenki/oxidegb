#[derive(Debug, Copy, Clone)]
pub enum TileMapRange {
    Low = 0,
    High = 1,
}

impl TileMapRange {
    pub fn base_address(&self) -> u16 {
        match self {
            TileMapRange::Low => 0x1800,
            TileMapRange::High => 0x1C00,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum TileDataAddressing {
    Unsigned = 0,
    Signed = 1,
}

impl TileDataAddressing {
    pub fn address_from_index(&self, index: u8, line: u16) -> u16 {
        match self {
            TileDataAddressing::Unsigned => (index as u16 * 16) + (line % 8) * 2,
            TileDataAddressing::Signed => {
                0x1000u16.wrapping_add((index as i8 as i16 * 16) as u16) + (line % 8) * 2
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum SpriteSize {
    S8x8 = 0,
    S8x16 = 1,
}

impl SpriteSize {
    pub fn height(&self) -> u8 {
        match self {
            SpriteSize::S8x8 => 8,
            SpriteSize::S8x16 => 16,
        }
    }
}

#[derive(Debug)]
pub struct LcdControl {
    pub lcd_enable: bool,
    pub window_tile_map: TileMapRange,
    pub window_enable: bool,
    pub bg_window_addressing: TileDataAddressing,
    pub bg_tile_map: TileMapRange,
    pub obj_size: SpriteSize,
    pub obj_enable: bool,
    pub bg_window_enable: bool,
}

impl LcdControl {
    pub fn new() -> Self {
        Self {
            lcd_enable: false,
            window_tile_map: TileMapRange::Low,
            window_enable: false,
            bg_window_addressing: TileDataAddressing::Unsigned,
            bg_tile_map: TileMapRange::Low,
            obj_size: SpriteSize::S8x8,
            obj_enable: true,
            bg_window_enable: true,
        }
    }

    pub fn value(&self) -> u8 {
        (self.lcd_enable as u8) << 7
            | (self.window_tile_map as u8) << 6
            | (self.window_enable as u8) << 5
            | (self.bg_window_addressing as u8) << 4
            | (self.bg_tile_map as u8) << 3
            | (self.obj_size as u8) << 2
            | (self.bg_window_enable as u8)
    }

    pub fn set_value(&mut self, value: u8) {
        self.lcd_enable = value & (1 << 7) != 0;
        self.window_tile_map = if value & (1 << 6) == 0 {
            TileMapRange::Low
        } else {
            TileMapRange::High
        };
        self.window_enable = value & (1 << 5) != 0;
        self.bg_window_addressing = if value & (1 << 4) == 0 {
            TileDataAddressing::Signed
        } else {
            TileDataAddressing::Unsigned
        };
        self.bg_tile_map = if value & (1 << 3) == 0 {
            TileMapRange::Low
        } else {
            TileMapRange::High
        };
        self.obj_size = if value & (1 << 2) == 0 {
            SpriteSize::S8x8
        } else {
            SpriteSize::S8x16
        };
        self.obj_enable = value & (1 << 1) != 0;
        self.bg_window_enable = value & 1 != 0;

        eprintln!("{:?}", &self);
    }
}

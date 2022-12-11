use super::{
    lcd_control::{TileDataAddressing, TileMapRange},
    palette::{self, Palette},
};

#[derive(Debug, Copy, Clone)]
enum FetcherAction {
    ReadTile,
    ReadDataH { tile_index: u8 },
    ReadDataL { data_address: u16, data_h: u8 },
    Wait { colors: [palette::Color; 8] },
}

#[derive(Debug)]
pub struct Fetcher {
    action: FetcherAction,
    waiting_cycle: bool,
    tile_map_index: u8,
}

impl Fetcher {
    const TILE_MAP_WIDTH: u16 = 32;
    const SPRITE_HEIGHT: u16 = 8;

    pub const fn new() -> Self {
        Self {
            action: FetcherAction::ReadTile,
            waiting_cycle: false,
            tile_map_index: 0,
        }
    }

    pub fn clear(&mut self) {
        *self = Self::new();
    }

    pub fn tick(
        &mut self,
        fifo: &mut PixelFifo,
        tile_map: TileMapRange,
        addressing: TileDataAddressing,
        palette: Palette,
        line_y: u8,
        scroll_y: u8,
        vram: &[u8],
    ) {
        let waiting = self.waiting_cycle;
        self.waiting_cycle = !self.waiting_cycle;
        if waiting {
            return;
        }

        let line = (line_y as u16 + scroll_y as u16) % (Self::TILE_MAP_WIDTH * Self::SPRITE_HEIGHT);

        match self.action {
            FetcherAction::ReadTile => {
                self.action = FetcherAction::ReadDataH {
                    tile_index: vram[(tile_map.base_address()
                        + line / 8 * 32
                        + (self.tile_map_index as u16))
                        as usize],
                };
                self.tile_map_index = (self.tile_map_index + 1) % Self::TILE_MAP_WIDTH as u8;
            }
            FetcherAction::ReadDataH { tile_index } => {
                let data_address = addressing.address_from_index(tile_index, line);
                self.action = FetcherAction::ReadDataL {
                    data_address,
                    data_h: vram[data_address as usize],
                };
            }
            FetcherAction::ReadDataL {
                data_address,
                data_h,
            } => {
                let data_l = vram[data_address as usize + 1];
                let colors = palette::Color::from_packed(data_h, data_l, palette);
                self.action = if fifo.push_line(&colors) {
                    FetcherAction::ReadTile
                } else {
                    FetcherAction::Wait { colors }
                };
            }
            FetcherAction::Wait { ref colors } => {
                if fifo.push_line(colors) {
                    self.action = FetcherAction::ReadTile;
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct PixelFifo {
    fifo: [palette::Color; Self::SIZE],
    start: usize,
    size: usize,
}

impl PixelFifo {
    const SIZE: usize = 16;
    const HALF_SIZE: usize = Self::SIZE / 2;

    pub const fn new() -> Self {
        Self {
            fifo: [palette::Color::White; Self::SIZE],
            start: 0,
            size: 0,
        }
    }

    fn push_line(&mut self, colors: &[palette::Color; 8]) -> bool {
        if self.size <= Self::HALF_SIZE {
            for (i, color) in colors.iter().cloned().enumerate() {
                self.fifo[(self.start + self.size + i) % Self::SIZE] = color;
            }
            self.size += 8;
            true
        } else {
            false
        }
    }

    pub fn pop(&mut self) -> Option<palette::Color> {
        if self.size > Self::HALF_SIZE {
            let color = Some(self.fifo[self.start]);
            self.start = (self.start + 1) % Self::SIZE;
            self.size -= 1;
            color
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.start = 0;
        self.size = 0;
    }
}

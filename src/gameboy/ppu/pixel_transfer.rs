use super::{
    lcd_control::{TileDataAddressing, TileMapRange},
    palette,
};

#[derive(Debug, Copy, Clone)]
enum FetcherAction {
    ReadTile,
    ReadData0 { tile_index: u8 },
    ReadData1 { data_address: u16, upper_data: u8 },
    Wait { colors: [palette::Color; 8] },
}

pub struct Fetcher {
    action: FetcherAction,
    waiting_cycle: bool,
    tile_map_index: u8,
}

impl Fetcher {
    pub fn new() -> Self {
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
        line: u16,
        vram: &[u8],
    ) {
        //println!("state {:?} waiting {} tile {}", self.action, self.waiting_cycle, self.tile_map_index);
        let waiting = self.waiting_cycle;
        self.waiting_cycle = !self.waiting_cycle;
        if waiting {
            return;
        }

        match self.action {
            FetcherAction::ReadTile => {
                self.action = FetcherAction::ReadData0 {
                    tile_index: vram[(tile_map.base_address()
                        + line / 8 * 32
                        + (self.tile_map_index as u16))
                        as usize],
                };
                self.tile_map_index += 1;
            }
            FetcherAction::ReadData0 { tile_index } => {
                let data_address = addressing.address_from_index(tile_index, line);
                self.action = FetcherAction::ReadData1 {
                    data_address,
                    upper_data: vram[data_address as usize],
                };
            }
            FetcherAction::ReadData1 {
                data_address,
                upper_data,
            } => {
                let data = (upper_data as u16) << 8 | vram[data_address as usize + 1] as u16;
                let colors = palette::Color::from_packed(data);
                self.action = if fifo.push_line(&colors) {
                    FetcherAction::ReadTile
                } else {
                    FetcherAction::Wait { colors }
                };
            }
            FetcherAction::Wait { ref colors } => {
                if fifo.push_line(&colors) {
                    self.action = FetcherAction::ReadTile;
                }
            }
        }
    }
}

pub struct PixelFifo {
    fifo: [palette::Color; Self::SIZE],
    start: usize,
    size: usize,
}

impl PixelFifo {
    const SIZE: usize = 16;
    const HALF_SIZE: usize = Self::SIZE / 2;

    pub fn new() -> Self {
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

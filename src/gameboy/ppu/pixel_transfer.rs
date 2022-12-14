use super::{
    lcd_control::LcdControl,
    obj::{self, Attributes, Priority},
    palette, Palettes,
};

#[derive(Debug, Copy, Clone)]
enum FetcherAction {
    ReadTile,
    ReadDataL { tile_index: u8 },
    ReadDataH { data_address: u16, data_l: u8 },
    Wait { indices: [u8; 8] },
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

    fn unpack_indices(data_l: u8, data_h: u8) -> [u8; 8] {
        let mut indices = [0; 8];
        for i in 0..8 {
            indices[7 - i] = (((data_h >> i) << 1) & 0b10) | ((data_l >> i) & 0b01);
        }

        indices
    }

    pub const fn new() -> Self {
        Self {
            action: FetcherAction::ReadTile,
            waiting_cycle: false,
            tile_map_index: 0,
        }
    }

    pub fn clear(&mut self) {
        *self = Self {
            waiting_cycle: self.waiting_cycle,
            ..Self::new()
        }
    }

    // TODO simplify arguments
    #[allow(clippy::too_many_arguments)]
    pub fn tick(
        &mut self,
        fifo: &mut PixelFifo<BgPixel>,
        _visible_sprites: &[Attributes],
        lcdc: &LcdControl,
        x_pos: u8,
        line_y: u8,
        _scroll_x: u8,
        scroll_y: u8,
        vram: &[u8],
        window_x: u8,
        window_y: u8,
    ) {
        let waiting = self.waiting_cycle;
        self.waiting_cycle = !self.waiting_cycle;
        if waiting {
            return;
        }

        let in_window = lcdc.window_enable && x_pos >= window_x && line_y >= window_y;
        let window_y_offset = if in_window {
            (-(window_y as i16)) as u16
        } else {
            0
        };

        let line = (line_y as u16 + scroll_y as u16 + window_y_offset)
            % (Self::TILE_MAP_WIDTH * Self::SPRITE_HEIGHT);

        match self.action {
            FetcherAction::ReadTile => {
                let tile_map = if in_window {
                    lcdc.window_tile_map
                } else {
                    lcdc.bg_tile_map
                };
                self.action = FetcherAction::ReadDataL {
                    tile_index: vram[(tile_map.base_address()
                        + line / 8 * 32
                        + (self.tile_map_index as u16))
                        as usize],
                };
                self.tile_map_index = (self.tile_map_index + 1) % Self::TILE_MAP_WIDTH as u8;
            }
            FetcherAction::ReadDataL { tile_index } => {
                let data_address = lcdc
                    .bg_window_addressing
                    .address_from_index(tile_index, line);
                self.action = FetcherAction::ReadDataH {
                    data_address,
                    data_l: vram[data_address as usize],
                };
            }
            FetcherAction::ReadDataH {
                data_address,
                data_l,
            } => {
                let data_h = vram[data_address as usize + 1];
                let indices = Self::unpack_indices(data_l, data_h);
                let pixels = indices.map(|index| BgPixel { index });
                self.action = if fifo.push_line(&pixels) {
                    FetcherAction::ReadTile
                } else {
                    FetcherAction::Wait { indices }
                };
            }
            FetcherAction::Wait { ref indices } => {
                let pixels = indices.map(|index| BgPixel { index });
                if fifo.push_line(&pixels) {
                    self.action = FetcherAction::ReadTile;
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BgPixel {
    pub index: u8,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ObjPixel {
    pub index: u8,
    pub palette: obj::Palette,
    pub priority: obj::Priority,
}

pub(crate) fn mix_pixels(
    bg_pixel: BgPixel,
    obj_pixel: ObjPixel,
    obj_enable: bool,
    palettes: &Palettes,
) -> palette::Color {
    if !obj_enable
        || obj_pixel.index == 0
        || (obj_pixel.priority == Priority::BehindNonZeroBg && bg_pixel.index > 0)
    {
        palettes.bg[bg_pixel.index]
    } else {
        match obj_pixel.palette {
            obj::Palette::ObjP0 => palettes.obj_0[obj_pixel.index],
            obj::Palette::ObjP1 => palettes.obj_0[obj_pixel.index],
        }
    }
}

#[derive(Debug)]
pub struct PixelFifo<T> {
    fifo: [T; PixelFifo::<BgPixel>::SIZE],
    start: usize,
    size: usize,
}

impl<T> PixelFifo<T> {
    const SIZE: usize = 16;
    const HALF_SIZE: usize = Self::SIZE / 2;
}

impl<T: Copy + Default> PixelFifo<T> {
    pub fn new() -> Self {
        Self {
            // TODO Find a way to get SIZE without referencing a concrete type
            fifo: [T::default(); PixelFifo::<BgPixel>::SIZE],
            start: 0,
            size: 0,
        }
    }

    fn push_line(&mut self, pixels: &[T; 8]) -> bool {
        if self.size <= Self::HALF_SIZE {
            for (i, color) in pixels.iter().cloned().enumerate() {
                self.fifo[(self.start + self.size + i) % Self::SIZE] = color;
            }
            self.size += 8;
            true
        } else {
            false
        }
    }

    pub fn can_pop(&self) -> bool {
        self.size > Self::HALF_SIZE
    }

    pub fn pop(&mut self) -> Option<T> {
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

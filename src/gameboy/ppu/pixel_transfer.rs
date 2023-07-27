use super::{
    lcd_control::{LcdControl, SpriteSize, TileDataAddressing},
    obj::{self, Attributes, Priority},
    palette, Palettes,
};

// TODO: simplify this mess
#[derive(Debug, Copy, Clone)]
enum Action {
    ObjReadAttr,
    ObjReadDataL {
        attr_index: usize,
    },
    ObjReadDataH {
        data_address: u16,
        data_l: u8,
        attr_index: usize,
    },
    BgReadStartTile,
    BgReadStartDataL,
    BgReadStartDataH,
    BgReadTile,
    BgReadDataL {
        tile_index: u8,
    },
    BgReadDataH {
        data_address: u16,
        data_l: u8,
    },
    BgWait {
        pixels: [BgPixel; 8],
    },
}

impl Action {
    fn pending_obj(&self) -> bool {
        matches!(
            self,
            Self::ObjReadAttr { .. } | Self::ObjReadDataL { .. } | Self::ObjReadDataH { .. }
        )
    }
}

#[derive(Debug)]
pub struct Fetcher {
    action: Action,
    waiting_cycle: bool,
    tile_map_index: u8,
    drawn_objs: [bool; 10],
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
            action: Action::BgReadStartTile,
            waiting_cycle: false,
            tile_map_index: 0,
            drawn_objs: [false; 10],
        }
    }

    pub fn start_line(&mut self) {
        *self = Self {
            waiting_cycle: self.waiting_cycle,
            action: Action::BgReadStartTile,
            tile_map_index: 0,
            drawn_objs: [false; 10],
        }
    }

    pub fn start_window(&mut self) {
        *self = Self {
            waiting_cycle: self.waiting_cycle,
            action: Action::BgReadTile,
            tile_map_index: 0,
            drawn_objs: self.drawn_objs,
        }
    }

    // TODO simplify arguments
    #[allow(clippy::too_many_arguments)]
    pub fn tick(
        &mut self,
        bg_fifo: &mut PixelFifo<BgPixel>,
        obj_fifo: &mut PixelFifo<ObjPixel>,
        visible_objs: &[Attributes],
        lcdc: &LcdControl,
        x_pos: u8,
        line_y: u8,
        scroll_x: u8,
        scroll_y: u8,
        vram: &[u8],
        window_triggered: bool,
        line_y_window: u8,
    ) -> bool {
        let next_obj = |visible_objs: &[Attributes], drawn_objs: &[bool]| {
            visible_objs
                .iter()
                .enumerate()
                .position(|(i, obj)| obj.x == x_pos && !drawn_objs[i])
        };

        let waiting = self.waiting_cycle;
        self.waiting_cycle = !self.waiting_cycle;
        if waiting {
            return next_obj(visible_objs, &self.drawn_objs).is_some();
        }

        let bg_w_line = if !window_triggered {
            (line_y as u16 + scroll_y as u16) % (Self::TILE_MAP_WIDTH * Self::SPRITE_HEIGHT)
        } else {
            line_y_window as u16
        };

        // TODO: check if sprite fetch aborts bg fetch or wait for it to finish
        if !self.action.pending_obj() && next_obj(visible_objs, &self.drawn_objs).is_some() {
            // There is a sprite to draw, abort current bg fetch
            self.action = Action::ObjReadAttr;
        }

        self.action = match self.action {
            Action::ObjReadAttr => {
                let attr_index = next_obj(visible_objs, &self.drawn_objs).unwrap();
                Action::ObjReadDataL { attr_index }
            }
            Action::ObjReadDataL { attr_index } => {
                let obj = &visible_objs[attr_index];
                let line = if obj.flip_y {
                    lcdc.obj_size.height() as u16 - 1 - (line_y + 16 - obj.y) as u16
                } else {
                    (line_y + 16 - obj.y) as u16
                };
                let index = match lcdc.obj_size {
                    SpriteSize::S8x8 => obj.tile_index,
                    SpriteSize::S8x16 => obj.tile_index & !0b1,
                };
                let data_address =
                    TileDataAddressing::Unsigned.address_from_index_obj(index, line, lcdc.obj_size);
                let mut data_l = vram[data_address as usize];
                if obj.flip_x {
                    data_l = data_l.reverse_bits();
                }

                Action::ObjReadDataH {
                    data_address,
                    data_l,
                    attr_index,
                }
            }
            Action::ObjReadDataH {
                data_address,
                data_l,
                attr_index,
            } => {
                let obj = &visible_objs[attr_index];
                let mut data_h = vram[data_address as usize + 1];
                if obj.flip_x {
                    data_h = data_h.reverse_bits();
                }
                let indices = Self::unpack_indices(data_l, data_h);
                let pixels = indices.map(|index| ObjPixel {
                    index,
                    palette: obj.palette,
                    priority: obj.priority,
                });
                obj_fifo.push_line(&pixels);
                self.drawn_objs[attr_index] = true;
                if next_obj(visible_objs, &self.drawn_objs).is_some() {
                    Action::ObjReadAttr
                } else if x_pos == 0 {
                    Action::BgReadStartTile
                } else {
                    Action::BgReadTile
                }
            }
            Action::BgReadStartTile => Action::BgReadStartDataL,
            Action::BgReadStartDataL => Action::BgReadStartDataH,
            Action::BgReadStartDataH => {
                let pixels = [BgPixel { index: 0 }; 8];
                if bg_fifo.push_line(&pixels) {
                    if next_obj(visible_objs, &self.drawn_objs).is_some() {
                        Action::ObjReadAttr
                    } else {
                        Action::BgReadTile
                    }
                } else {
                    Action::BgWait { pixels }
                }
            }
            Action::BgReadTile => {
                let (tile_map, scroll_offset) = if window_triggered {
                    (lcdc.window_tile_map, 0)
                } else {
                    (lcdc.bg_tile_map, scroll_x / 8)
                };
                let tile_map_index =
                    (self.tile_map_index as u16 + scroll_offset as u16) % Self::TILE_MAP_WIDTH;

                Action::BgReadDataL {
                    tile_index: vram[(tile_map.base_address()
                        + bg_w_line / 8 * Self::TILE_MAP_WIDTH
                        + tile_map_index) as usize],
                }
            }
            Action::BgReadDataL { tile_index } => {
                let data_address = lcdc
                    .bg_window_addressing
                    .address_from_index_bg(tile_index, bg_w_line);
                Action::BgReadDataH {
                    data_address,
                    data_l: vram[data_address as usize],
                }
            }
            Action::BgReadDataH {
                data_address,
                data_l,
            } => {
                let data_h = vram[data_address as usize + 1];
                let indices = Self::unpack_indices(data_l, data_h);
                let pixels = indices.map(|index| BgPixel { index });
                if bg_fifo.push_line(&pixels) {
                    self.tile_map_index = (self.tile_map_index + 1) % Self::TILE_MAP_WIDTH as u8;
                    Action::BgReadTile
                } else {
                    Action::BgWait { pixels }
                }
            }
            wait @ Action::BgWait { ref pixels } => {
                if bg_fifo.push_line(pixels) {
                    self.tile_map_index = (self.tile_map_index + 1) % Self::TILE_MAP_WIDTH as u8;
                    Action::BgReadTile
                } else {
                    wait
                }
            }
        };

        next_obj(visible_objs, &self.drawn_objs).is_some()
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

pub fn mix_pixels(
    bg_pixel: BgPixel,
    obj_pixel: Option<ObjPixel>,
    lcdc: &LcdControl,
    palettes: &Palettes,
) -> palette::Color {
    if let Some(obj_pixel) = obj_pixel {
        if lcdc.obj_enable
            && obj_pixel.index != 0
            && (obj_pixel.priority != Priority::BehindNonZeroBg || bg_pixel.index == 0)
        {
            return match obj_pixel.palette {
                obj::Palette::ObjP0 => palettes.obj_0[obj_pixel.index],
                obj::Palette::ObjP1 => palettes.obj_1[obj_pixel.index],
            };
        }
    }

    if lcdc.bg_window_enable {
        palettes.bg[bg_pixel.index]
    } else {
        palette::Color::White
    }
}

#[derive(Debug)]
pub struct PixelFifo<T> {
    fifo: [T; PixelFifo::<BgPixel>::SIZE],
    start: usize,
    size: usize,
}

impl<T> PixelFifo<T> {
    const SIZE: usize = 8;
}

impl PixelFifo<ObjPixel> {
    fn push_line(&mut self, pixels: &[ObjPixel; 8]) {
        // Fill available Fifo space with transparent pixels
        for i in self.size..Self::SIZE {
            self.fifo[(self.start + i) % Self::SIZE] = ObjPixel {
                index: 0,
                palette: obj::Palette::ObjP0,
                priority: Priority::BehindNonZeroBg,
            };
        }
        self.size = Self::SIZE;

        for (i, new_pixel) in pixels.iter().copied().enumerate() {
            let pixel = &mut self.fifo[(self.start + i) % Self::SIZE];
            if pixel.index == 0 && new_pixel.index > 0 {
                *pixel = new_pixel;
            }
        }
    }
}

impl PixelFifo<BgPixel> {
    fn push_line(&mut self, pixels: &[BgPixel; 8]) -> bool {
        if self.size == 0 {
            for (i, pixel) in pixels.iter().copied().enumerate() {
                self.fifo[(self.start + self.size + i) % Self::SIZE] = pixel;
            }
            self.size += 8;
            true
        } else {
            false
        }
    }
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

    pub fn pop(&mut self) -> Option<T> {
        if self.size > 0 {
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

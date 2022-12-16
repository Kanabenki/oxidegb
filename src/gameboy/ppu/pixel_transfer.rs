use super::{
    lcd_control::{LcdControl, TileDataAddressing},
    obj::{self, Attributes, Priority},
    palette, Palettes,
};

// TODO: simplify this mess
#[derive(Debug, Copy, Clone)]
enum Action {
    ObjReadAttr {
        prev_index: Option<usize>,
    },
    ObjReadDataL {
        attr_index: usize,
    },
    ObjReadDataH {
        data_address: u16,
        data_l: u8,
        attr_index: usize,
    },
    // TODO Check if this state is ever triggered
    ObjWait {
        pixels: [ObjPixel; 8],
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
            Action::ObjReadAttr { .. }
                | Action::ObjReadDataL { .. }
                | Action::ObjReadDataH { .. }
                | Action::ObjWait { .. }
        )
    }
}

#[derive(Debug)]
pub struct Fetcher {
    action: Action,
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
            action: Action::BgReadStartTile,
            waiting_cycle: false,
            tile_map_index: 0,
        }
    }

    pub fn start_line(&mut self) {
        *self = Self {
            waiting_cycle: self.waiting_cycle,
            action: Action::BgReadStartTile,
            tile_map_index: 0,
        }
    }

    pub fn start_window(&mut self) {
        *self = Self {
            waiting_cycle: self.waiting_cycle,
            action: Action::BgReadTile,
            tile_map_index: 0,
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
        window_x: u8,
        window_y: u8,
    ) -> bool {
        let check_for_obj = |last_checked: Option<usize>| {
            let mut obj_range = if let Some(idx) = last_checked {
                idx + 1..visible_objs.len()
            } else {
                0..visible_objs.len()
            };
            obj_range.find(|&i| visible_objs[i].x == x_pos)
        };

        let waiting = self.waiting_cycle;
        self.waiting_cycle = !self.waiting_cycle;
        if waiting {
            return self.action.pending_obj();
        }

        let in_window = lcdc.window_enable && x_pos >= window_x && line_y >= window_y;
        let window_y_offset = if in_window {
            (-(window_y as i16)) as u16
        } else {
            0
        };

        let bg_w_line = (line_y as u16 + scroll_y as u16 + window_y_offset)
            % (Self::TILE_MAP_WIDTH * Self::SPRITE_HEIGHT);

        match self.action {
            Action::ObjReadAttr { prev_index } => {
                let attr_index = check_for_obj(prev_index).unwrap();
                self.action = Action::ObjReadDataL { attr_index };
            }
            Action::ObjReadDataL { attr_index } => {
                let obj = &visible_objs[attr_index];
                let line = if obj.flip_y {
                    7 - (line_y + 16 - obj.y) as u16
                } else {
                    (line_y + 16 - obj.y) as u16
                };
                let data_address =
                    TileDataAddressing::Unsigned.address_from_index(obj.tile_index, line);
                let mut data_l = vram[data_address as usize];
                if obj.flip_x {
                    data_l = data_l.reverse_bits();
                }
                self.action = Action::ObjReadDataH {
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
                if obj_fifo.push_line(&pixels) {
                    self.action = if check_for_obj(Some(attr_index)).is_some() {
                        Action::ObjReadAttr {
                            prev_index: Some(attr_index),
                        }
                    } else {
                        Action::BgReadTile
                    }
                } else {
                    self.action = Action::ObjWait { attr_index, pixels };
                }
            }
            Action::ObjWait { attr_index, pixels } => {
                if obj_fifo.push_line(&pixels) {
                    self.action = if check_for_obj(Some(attr_index)).is_some() {
                        Action::ObjReadAttr {
                            prev_index: Some(attr_index),
                        }
                    } else {
                        Action::BgReadTile
                    };
                }
            }
            Action::BgReadStartTile => self.action = Action::BgReadStartDataL,
            Action::BgReadStartDataL => self.action = Action::BgReadStartDataH,
            Action::BgReadStartDataH => {
                let pixels = [BgPixel { index: 0 }; 8];
                self.action = if bg_fifo.push_line(&pixels) {
                    if check_for_obj(None).is_some() {
                        Action::ObjReadAttr { prev_index: None }
                    } else {
                        Action::BgReadTile
                    }
                } else {
                    Action::BgWait { pixels }
                };
            }
            Action::BgReadTile => {
                let (tile_map, scroll_offset) = if in_window {
                    (lcdc.window_tile_map, 0)
                } else {
                    (lcdc.bg_tile_map, scroll_x / 8)
                };
                let tile_map_index =
                    (self.tile_map_index as u16 + scroll_offset as u16) % Self::TILE_MAP_WIDTH;
                self.action = Action::BgReadDataL {
                    tile_index: vram[(tile_map.base_address()
                        + bg_w_line / 8 * Self::TILE_MAP_WIDTH
                        + tile_map_index) as usize],
                };
                self.tile_map_index = (self.tile_map_index + 1) % Self::TILE_MAP_WIDTH as u8;
            }
            Action::BgReadDataL { tile_index } => {
                let data_address = lcdc
                    .bg_window_addressing
                    .address_from_index(tile_index, bg_w_line);
                self.action = Action::BgReadDataH {
                    data_address,
                    data_l: vram[data_address as usize],
                };
            }
            Action::BgReadDataH {
                data_address,
                data_l,
            } => {
                let data_h = vram[data_address as usize + 1];
                let indices = Self::unpack_indices(data_l, data_h);
                let pixels = indices.map(|index| BgPixel { index });
                self.action = if bg_fifo.push_line(&pixels) {
                    if check_for_obj(None).is_some() {
                        Action::ObjReadAttr { prev_index: None }
                    } else {
                        Action::BgReadTile
                    }
                } else {
                    Action::BgWait { pixels }
                };
            }
            Action::BgWait { ref pixels } => {
                if bg_fifo.push_line(pixels) {
                    self.action = if check_for_obj(None).is_some() {
                        Action::ObjReadAttr { prev_index: None }
                    } else {
                        Action::BgReadTile
                    }
                }
            }
        }

        self.action.pending_obj()
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
    fn push_line(&mut self, pixels: &[ObjPixel; 8]) -> bool {
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
        true
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

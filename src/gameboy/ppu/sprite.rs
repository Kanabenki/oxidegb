#[derive(Debug, Clone, Copy)]
pub enum Palette {
    Obp0,
    Obp1,
}

impl Default for Palette {
    fn default() -> Self {
        Palette::Obp0
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Priority {
    BehindNonZeroBg,
    AboveBg,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::BehindNonZeroBg
    }
}

#[derive(Debug)]
pub struct Attributes {
    pub x: u8,
    pub y: u8,
    pub tile_index: u8,
    pub flip_x: bool,
    pub flip_y: bool,
    pub priority: Priority,
    pub palette: Palette,
}

impl Attributes {
    pub const fn parse(values: [u8; 4]) -> Self {
        let x = values[0];
        let y = values[1];
        let tile_index = values[2];

        let priority = if values[3] & (1 << 7) == 0 {
            Priority::AboveBg
        } else {
            Priority::BehindNonZeroBg
        };

        let flip_y = values[3] & (1 << 6) != 0;
        let flip_x = values[3] & (1 << 5) != 0;

        let palette = if values[3] & (1 << 4) == 0 {
            Palette::Obp0
        } else {
            Palette::Obp1
        };

        Self {
            x,
            y,
            tile_index,
            flip_x,
            flip_y,
            priority,
            palette,
        }
    }
}

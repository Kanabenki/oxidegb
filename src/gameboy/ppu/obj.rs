#[derive(Debug, Clone, Copy, Default)]
pub enum Palette {
    #[default]
    ObjP0,
    ObjP1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Priority {
    #[default]
    BehindNonZeroBg,
    AboveBg,
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
        let y = values[0];
        let x = values[1];
        let tile_index = values[2];

        let priority = if values[3] & (1 << 7) == 0 {
            Priority::AboveBg
        } else {
            Priority::BehindNonZeroBg
        };

        let flip_y = values[3] & (1 << 6) != 0;
        let flip_x = values[3] & (1 << 5) != 0;

        let palette = if values[3] & (1 << 4) == 0 {
            Palette::ObjP0
        } else {
            Palette::ObjP1
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

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default)]
pub(crate) enum Palette {
    #[default]
    ObjP0,
    ObjP1,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Priority {
    #[default]
    BehindNonZeroBg,
    AboveBg,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Attributes {
    pub(crate) x: u8,
    pub(crate) y: u8,
    pub(crate) tile_index: u8,
    pub(crate) flip_x: bool,
    pub(crate) flip_y: bool,
    pub(crate) priority: Priority,
    pub(crate) palette: Palette,
}

impl Attributes {
    pub(crate) const fn parse(values: [u8; 4]) -> Self {
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

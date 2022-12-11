use std::convert::{TryFrom, TryInto};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Color {
    White = 0,
    LightGray = 1,
    DarkGray = 2,
    Black = 3,
}

impl Default for Color {
    fn default() -> Self {
        Color::White
    }
}

impl Color {
    pub fn from_packed(data_l: u8, data_h: u8, palette: Palette) -> [Self; 8] {
        let mut colors = [Self::White; 8];
        for i in 0..8 {
            colors[7 - i] = match (((data_h >> i) << 1) & 0b10) | ((data_l >> i) & 0b01) {
                0 => palette.0,
                1 => palette.1,
                2 => palette.2,
                3 => palette.3,
                _ => unreachable!(),
            };
        }

        colors
    }
}

impl TryFrom<u8> for Color {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::White),
            1 => Ok(Self::LightGray),
            2 => Ok(Self::DarkGray),
            3 => Ok(Self::Black),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct Palette(Color, Color, Color, Color);

impl Palette {
    pub const fn new() -> Self {
        Self(Color::White, Color::White, Color::White, Color::White)
    }

    pub const fn value(&self) -> u8 {
        self.0 as u8 | (self.1 as u8) << 2 | (self.2 as u8) << 4 | (self.3 as u8) << 6
    }

    pub fn set_value(&mut self, value: u8) {
        *self = Self(
            (value & 0b11).try_into().unwrap(),
            ((value >> 2) & 0b11).try_into().unwrap(),
            ((value >> 4) & 0b11).try_into().unwrap(),
            ((value >> 6) & 0b11).try_into().unwrap(),
        )
    }
}

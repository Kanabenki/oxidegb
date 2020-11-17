use std::convert::{TryFrom, TryInto};

#[derive(Debug, Copy, Clone)]
pub enum Color {
    White = 0,
    LightGray = 1,
    DarkGray = 2,
    Black = 3,
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

pub struct Palette(Color, Color, Color, Color);

impl Palette {
    pub fn new() -> Self {
        Self(Color::White, Color::White, Color::White, Color::White)
    }

    pub fn value(&self) -> u8 {
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

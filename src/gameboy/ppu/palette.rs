use std::{
    convert::{TryFrom, TryInto},
    ops::Index,
};

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
pub struct Palette([Color; 4]);

impl Index<u8> for Palette {
    type Output = Color;

    fn index(&self, index: u8) -> &Self::Output {
        &self.0[index as usize]
    }
}

impl Palette {
    pub fn value(&self) -> u8 {
        self[0] as u8 | (self[1] as u8) << 2 | (self[2] as u8) << 4 | (self[3] as u8) << 6
    }

    pub fn set_value(&mut self, value: u8) {
        *self = Self([
            (value & 0b11).try_into().unwrap(),
            ((value >> 2) & 0b11).try_into().unwrap(),
            ((value >> 4) & 0b11).try_into().unwrap(),
            ((value >> 6) & 0b11).try_into().unwrap(),
        ])
    }
}

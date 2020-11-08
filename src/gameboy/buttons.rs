use std::collections::HashSet;

pub enum Button {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    Select,
    Start,
}

pub struct Buttons(HashSet<Button>);

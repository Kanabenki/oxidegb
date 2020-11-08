pub enum Button {
    Down,
    Up,
    Left,
    Right,
    Start,
    Select,
    A,
    B,
}

impl Button {
    fn bit(&self) -> u8 {
        match self {
            Button::Down | Button::Start => 0b0001,
            Button::Up | Button::Select => 0b0010,
            Button::Left | Button::A => 0b0100,
            Button::Right | Button::B => 0b1000,
        }
    }

    fn line(&self) -> InputLine {
        match self {
            Button::Down | Button::Up | Button::Left | Button::Right => InputLine::Directions,
            Button::Start | Button::Select | Button::A | Button::B => InputLine::Buttons,
        }
    }
}

enum InputLine {
    Directions,
    Buttons,
}

pub struct Buttons {
    directions: u8,
    buttons: u8,
    current_line: InputLine,
}

impl Buttons {
    fn new() -> Self {
        Self {
            directions: 0x0F,
            buttons: 0x0F,
            current_line: InputLine::Directions,
        }
    }

    fn read(&self) -> u8 {
        match self.current_line {
            InputLine::Directions => (self.directions & 0xF) & 0xE0,
            InputLine::Buttons => (self.buttons & 0xF) & 0xD0,
        }
    }

    fn write(&mut self, value: u8) {
        self.current_line = match (value >> 4) & 0b11 {
            0b01 => InputLine::Buttons,
            0b10 => InputLine::Directions,
            _ => {
                return;
            }
        }
    }

    pub fn set_button(&mut self, button: Button, set: bool) {
        let line = match button.line() {
            InputLine::Directions => &mut self.directions,
            InputLine::Buttons => &mut self.buttons,
        };

        if set {
            *line &= !button.bit() & 0xF;
        } else {
            *line |= button.bit();
        }
    }
}

mod map {
    pub const BUTTONS: u16 = 0xFF00;
}

pub struct Io {
    pub(super) buttons: Buttons,
}

impl Io {
    pub fn new() -> Self {
        Self {
            buttons: Buttons::new(),
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        use map::*;
        match address {
            BUTTONS => self.buttons.read(),
            _ => todo!(),
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        use map::*;
        match address {
            BUTTONS => self.buttons.write(value),
            _ => todo!(),
        }
    }
}

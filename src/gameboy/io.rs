use flagset::FlagSet;

use super::interrupts::Interrupt;

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
    Both,
}

pub struct Buttons {
    directions: u8,
    buttons: u8,
    current_line: InputLine,
    interrupt_raised: bool,
}

impl Buttons {
    fn new() -> Self {
        Self {
            directions: 0x0F,
            buttons: 0x0F,
            current_line: InputLine::Directions,
            interrupt_raised: false,
        }
    }

    fn read(&self) -> u8 {
        match self.current_line {
            InputLine::Directions => (self.directions & 0xF) & 0xE0,
            InputLine::Buttons => (self.buttons & 0xF) & 0xD0,
            InputLine::Both => (self.buttons | self.directions & 0xF) & 0xC,
        }
    }

    fn write(&mut self, value: u8) {
        self.current_line = match (value >> 4) & 0b11 {
            0b01 => InputLine::Buttons,
            0b10 => InputLine::Directions,
            0b11 => InputLine::Both,
            _ => unreachable!(),
        };

        if self.read() & 0xF != 0xF {
            self.interrupt_raised = true;
        }
    }

    pub fn set_button(&mut self, button: Button, set: bool) {
        if set && self.read() & 0x0F == 0x0F {
            self.interrupt_raised = true;
        }

        let line = match button.line() {
            InputLine::Directions => &mut self.directions,
            InputLine::Buttons => &mut self.buttons,
            InputLine::Both => unreachable!(),
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
    pub const INTERRUPT_FLAGS: u16 = 0xFF0F;
}

pub struct Io {
    pub(super) buttons: Buttons,
    pub interrupt_flags: FlagSet<Interrupt>,
}

impl Io {
    pub fn new() -> Self {
        Self {
            buttons: Buttons::new(),
            interrupt_flags: FlagSet::new_truncated(0),
        }
    }

    pub fn tick(&mut self) -> FlagSet<Interrupt> {
        let mut interrupts = FlagSet::new_truncated(0);
        if self.buttons.interrupt_raised {
            interrupts |= Interrupt::Joypad;
        }

        interrupts
    }

    pub fn read(&self, address: u16) -> u8 {
        use map::*;
        match address {
            BUTTONS => self.buttons.read(),
            INTERRUPT_FLAGS => self.interrupt_flags.bits() | 0b11100000,
            _ => todo!(),
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        use map::*;
        match address {
            BUTTONS => self.buttons.write(value),
            INTERRUPT_FLAGS => self.interrupt_flags = FlagSet::new_truncated(value),
            _ => todo!(),
        }
    }
}

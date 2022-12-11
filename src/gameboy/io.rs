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
    const fn bit(&self) -> u8 {
        match self {
            Button::Down | Button::Start => 0b0001,
            Button::Up | Button::Select => 0b0010,
            Button::Left | Button::A => 0b0100,
            Button::Right | Button::B => 0b1000,
        }
    }

    const fn line(&self) -> InputLine {
        match self {
            Button::Down | Button::Up | Button::Left | Button::Right => InputLine::Directions,
            Button::Start | Button::Select | Button::A | Button::B => InputLine::Buttons,
        }
    }
}

#[derive(Debug)]
enum InputLine {
    Directions,
    Buttons,
    Both,
}

#[derive(Debug)]
pub struct Buttons {
    directions: u8,
    buttons: u8,
    current_line: InputLine,
    interrupt_raised: bool,
}

impl Buttons {
    const fn new() -> Self {
        Self {
            directions: 0x0F,
            buttons: 0x0F,
            current_line: InputLine::Directions,
            interrupt_raised: false,
        }
    }

    const fn read(&self) -> u8 {
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

#[derive(Debug, Copy, Clone)]
enum InputClock {
    CpuDiv1024 = 0b00,
    CpuDiv16 = 0b01,
    CpuDiv64 = 0b10,
    CpuDiv256 = 0b11,
}

impl InputClock {
    const fn bit(&self) -> u16 {
        match self {
            InputClock::CpuDiv1024 => 1 << 10,
            InputClock::CpuDiv16 => 1 << 4,
            InputClock::CpuDiv64 => 1 << 6,
            InputClock::CpuDiv256 => 1 << 8,
        }
    }
}

#[derive(Debug)]
enum TimerState {
    Normal,
    Overflowed,
}

#[derive(Debug)]
struct Timer {
    divider: u16,
    counter: u8,
    modulo: u8,
    enabled: bool,
    input_clock: InputClock,
    state: TimerState,
}

impl Timer {
    const DIVIDER_ADDRESS: u16 = 0xFF04;
    const COUNTER_ADDRESS: u16 = 0xFF05;
    const MODULO_ADDRESS: u16 = 0xFF06;
    const CONTROL_ADDRESS: u16 = 0xFF07;

    const fn new() -> Self {
        Self {
            divider: 0,
            counter: 0,
            modulo: 0,
            enabled: false,
            input_clock: InputClock::CpuDiv1024,
            state: TimerState::Normal,
        }
    }

    fn tick(&mut self) -> bool {
        let new_divider = self.divider.wrapping_add(4);

        match self.state {
            TimerState::Normal => {
                if self.enabled && (new_divider ^ self.divider) & self.input_clock.bit() != 0 {
                    self.increase_counter();

                    return false;
                }
            }
            TimerState::Overflowed => {
                self.counter = self.modulo;
                self.state = TimerState::Normal;
                return true;
            }
        }

        false
    }

    fn increase_counter(&mut self) {
        let (counter, overflowed) = self.modulo.overflowing_add(1);
        self.counter = counter;

        if overflowed {
            self.state = TimerState::Overflowed;
        }
    }

    fn read(&self, address: u16) -> u8 {
        match address {
            Self::DIVIDER_ADDRESS => (self.divider >> 8) as u8,
            Self::COUNTER_ADDRESS => self.counter,
            Self::MODULO_ADDRESS => self.modulo,
            Self::CONTROL_ADDRESS => {
                0b1111_1000 | (u8::from(self.enabled) << 2) | self.input_clock as u8
            }
            _ => panic!("Tried to read timer register out of range"),
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            Self::DIVIDER_ADDRESS => {
                if self.divider & self.input_clock.bit() != 0 {
                    self.increase_counter()
                }
                self.divider = 0;
            }
            Self::COUNTER_ADDRESS => self.counter = value,
            Self::MODULO_ADDRESS => self.modulo = value,
            Self::CONTROL_ADDRESS => {
                let old_enabled = self.enabled;
                let old_div_bit = self.divider & self.input_clock.bit();

                self.enabled = value & 0b100 != 0;
                self.input_clock = match value & 0b11 {
                    0 => InputClock::CpuDiv1024,
                    1 => InputClock::CpuDiv16,
                    2 => InputClock::CpuDiv64,
                    3 => InputClock::CpuDiv256,
                    _ => unreachable!(),
                };

                let new_div_bit = self.divider & self.input_clock.bit();
                if (old_enabled && !self.enabled && old_div_bit != 0)
                    || (!old_enabled && self.enabled && old_div_bit == 0 && new_div_bit != 0)
                {
                    self.increase_counter();
                }
            }
            _ => panic!("Tried to write timer register out of range"),
        }
    }
}

mod map {
    pub const BUTTONS: u16 = 0xFF00;
    pub const SERIAL_TRANSFER: u16 = 0xFF01;
    pub const SERIAL_CONTROL: u16 = 0xFF02;
    pub const UNUSED: u16 = 0xFF03;
    pub const TIMER_START: u16 = 0xFF04;
    pub const TIMER_END: u16 = 0xFF07;
}

#[derive(Debug)]
pub struct Io {
    pub(super) buttons: Buttons,
    timer: Timer,
}

impl Io {
    pub const fn new() -> Self {
        Self {
            buttons: Buttons::new(),
            timer: Timer::new(),
        }
    }

    pub fn tick(&mut self) -> FlagSet<Interrupt> {
        let mut interrupts = FlagSet::new_truncated(0);
        if self.buttons.interrupt_raised {
            interrupts |= Interrupt::Joypad;
        }

        if self.timer.tick() {
            interrupts |= Interrupt::Timer;
        }

        interrupts
    }

    pub fn read(&self, address: u16) -> u8 {
        use map::*;
        match address {
            BUTTONS => self.buttons.read(),
            SERIAL_TRANSFER => 0xFF,
            SERIAL_CONTROL => 0xFF,
            UNUSED => 0xFF,
            TIMER_START..=TIMER_END => self.timer.read(address),
            invalid_address => panic!(
                "Tried to read at invalid io register address 0x{:X}",
                invalid_address
            ),
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        use map::*;
        match address {
            BUTTONS => self.buttons.write(value),
            SERIAL_TRANSFER => {}
            SERIAL_CONTROL => {}
            UNUSED => {}
            TIMER_START..=TIMER_END => self.timer.write(address, value),
            invalid_address => panic!(
                "Tried to write at invalid io register address 0x{:X}",
                invalid_address
            ),
        }
    }
}

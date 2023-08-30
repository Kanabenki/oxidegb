use flagset::FlagSet;
use serde::{Deserialize, Serialize};

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
            Self::Right | Self::A => 0b0001,
            Self::Left | Self::B => 0b0010,
            Self::Up | Self::Select => 0b0100,
            Self::Down | Self::Start => 0b1000,
        }
    }

    const fn line(&self) -> InputLine {
        match self {
            Self::Down | Self::Up | Self::Left | Self::Right => InputLine::Directions,
            Self::Start | Self::Select | Self::A | Self::B => InputLine::Buttons,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
enum InputLine {
    Directions = 0b0001_0000,
    Buttons = 0b0010_0000,
    Both = 0b0011_0000,
    None = 0b0000_0000,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Buttons {
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
            current_line: InputLine::None,
            interrupt_raised: false,
        }
    }

    const fn read(&self) -> u8 {
        let nibble_l = match self.current_line {
            InputLine::Directions => self.directions & 0xF,
            InputLine::Buttons => self.buttons & 0xF,
            InputLine::Both => 0xF,
            InputLine::None => self.buttons & self.directions & 0xF,
        };

        0b1100_0000 | self.current_line as u8 | nibble_l
    }

    fn write(&mut self, value: u8) {
        self.current_line = match (value >> 4) & 0b11 {
            0b01 => InputLine::Buttons,
            0b10 => InputLine::Directions,
            0b11 => InputLine::Both,
            0b00 => InputLine::None,
            _ => unreachable!(),
        };

        if self.read() & 0xF != 0xF {
            self.interrupt_raised = true;
        }
    }

    pub(crate) fn set_button(&mut self, button: Button, set: bool) {
        if set && self.read() & 0x0F == 0x0F {
            self.interrupt_raised = true;
        }

        let line = match button.line() {
            InputLine::Directions => &mut self.directions,
            InputLine::Buttons => &mut self.buttons,
            InputLine::Both | InputLine::None => unreachable!(),
        };

        if set {
            *line &= !button.bit() & 0xF;
        } else {
            *line |= button.bit();
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
enum InputClock {
    CpuDiv1024 = 0b00,
    CpuDiv16 = 0b01,
    CpuDiv64 = 0b10,
    CpuDiv256 = 0b11,
}

impl InputClock {
    const fn bit(&self) -> u16 {
        match self {
            Self::CpuDiv1024 => 1 << 10,
            Self::CpuDiv16 => 1 << 4,
            Self::CpuDiv64 => 1 << 6,
            Self::CpuDiv256 => 1 << 8,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
enum TimerState {
    Normal,
    Overflowed,
}

#[derive(Serialize, Deserialize, Debug)]
struct Timer {
    divider: u16,
    counter: u8,
    modulo: u8,
    enabled: bool,
    apu_inc_div: bool,
    input_clock: InputClock,
    state: TimerState,
}

pub(crate) struct TimerTick {
    pub(crate) timer_overflow: bool,
    pub(crate) apu_inc_div: bool,
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
            apu_inc_div: false,
            state: TimerState::Normal,
        }
    }

    fn tick(&mut self) -> TimerTick {
        let old_divider = self.divider;
        self.divider = self.divider.wrapping_add(4);

        // Bit 4 high to low
        let apu_inc_div = self.divider == 0b10_0000 || self.apu_inc_div;
        self.apu_inc_div = false;

        if !self.enabled {
            return TimerTick {
                apu_inc_div,
                timer_overflow: false,
            };
        }

        let timer_overflow = match self.state {
            TimerState::Normal => {
                if (old_divider ^ self.divider) & self.input_clock.bit() != 0 {
                    self.increase_counter();
                }
                false
            }
            TimerState::Overflowed => {
                self.counter = self.modulo;
                self.state = TimerState::Normal;
                true
            }
        };

        TimerTick {
            timer_overflow,
            apu_inc_div,
        }
    }

    fn increase_counter(&mut self) {
        let (counter, overflowed) = self.counter.overflowing_add(1);
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
                // TODO check read/write behaviour of unused bits.
                0b1111_1000 | (u8::from(self.enabled) << 2) | self.input_clock as u8
            }
            _ => panic!("Tried to read timer register out of range"),
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            Self::DIVIDER_ADDRESS => {
                if self.divider & self.input_clock.bit() != 0 {
                    self.increase_counter();
                }
                if self.divider & 0b1_0000 != 0 {
                    self.apu_inc_div = true;
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
    pub(crate) const BUTTONS: u16 = 0xFF00;
    pub(crate) const SERIAL_TRANSFER: u16 = 0xFF01;
    pub(crate) const SERIAL_CONTROL: u16 = 0xFF02;
    pub(crate) const UNUSED: u16 = 0xFF03;
    pub(crate) const TIMER_START: u16 = 0xFF04;
    pub(crate) const TIMER_END: u16 = 0xFF07;
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Io {
    pub(super) buttons: Buttons,
    timer: Timer,
}

pub(crate) struct IoTick {
    pub(crate) interrupts: FlagSet<Interrupt>,
    pub(crate) apu_inc_div: bool,
}

impl Io {
    pub(crate) const fn new() -> Self {
        Self {
            buttons: Buttons::new(),
            timer: Timer::new(),
        }
    }

    pub(crate) fn tick(&mut self) -> IoTick {
        let mut interrupts = FlagSet::new_truncated(0);
        if self.buttons.interrupt_raised {
            interrupts |= Interrupt::Joypad;
        }

        let TimerTick {
            timer_overflow,
            apu_inc_div,
        } = self.timer.tick();
        if timer_overflow {
            interrupts |= Interrupt::Timer;
        }

        IoTick {
            interrupts,
            apu_inc_div,
        }
    }

    pub(crate) fn tick_stopped(&mut self) -> FlagSet<Interrupt> {
        if self.buttons.interrupt_raised {
            Interrupt::Joypad.into()
        } else {
            FlagSet::new_truncated(0)
        }
    }

    pub(crate) fn read(&self, address: u16) -> u8 {
        use map::*;
        match address {
            BUTTONS => self.buttons.read(),
            SERIAL_TRANSFER => 0xFF,
            SERIAL_CONTROL => 0xFF,
            UNUSED => 0xFF,
            TIMER_START..=TIMER_END => self.timer.read(address),
            invalid_address => {
                panic!("Tried to read at invalid io register address 0x{invalid_address:X}")
            }
        }
    }

    pub(crate) fn write(&mut self, address: u16, value: u8) {
        use map::*;
        match address {
            BUTTONS => self.buttons.write(value),
            SERIAL_TRANSFER => {}
            SERIAL_CONTROL => {}
            UNUSED => {}
            TIMER_START..=TIMER_END => self.timer.write(address, value),
            invalid_address => {
                panic!("Tried to write at invalid io register address 0x{invalid_address:X}")
            }
        }
    }
}

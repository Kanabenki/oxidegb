use flagset::{flags, FlagSet};

#[derive(Debug, Copy, Clone)]
pub struct RegisterIndex(u8);

impl RegisterIndex {
    pub const fn from_opcode_first(opcode: u8) -> Self {
        Self((opcode >> 3) & 0b11)
    }

    pub const fn from_opcode_second(opcode: u8) -> Self {
        Self(opcode & 0b11)
    }

    pub const fn value(&self) -> u8 {
        self.0
    }
}

#[derive(Debug, Copy, Clone)]
pub struct DoubleRegisterIndex(u8);

impl DoubleRegisterIndex {
    pub const fn from_opcode(opcode: u8) -> Self {
        Self((opcode >> 4) & 0b11)
    }
}

flags! {
    enum Flag: u8 {
        Z = 0b10000000,
        N = 0b01000000,
        H = 0b00100000,
        C = 0b00010000
    }
}

pub enum FlagOp {
    Carry,
    Borrow,
}

#[derive(Debug, Copy, Clone, Default)]
pub struct Flags(FlagSet<Flag>);

impl Flags {
    fn is_set(&self, flag: Flag) -> bool {
        self.0.contains(flag)
    }

    fn set(&mut self, flag: Flag, set: bool) {
        if set {
            self.0 |= flag;
        } else {
            self.0 &= !flag;
        }
    }

    fn value(&self) -> u8 {
        self.0.bits()
    }

    fn set_value(&mut self, value: u8) {
        self.0 = FlagSet::new_truncated(value);
    }

    pub fn update_carry_u8(&mut self, lhs: u8, rhs: u8, with_carry: bool, flag_op: FlagOp) {
        let carry = u8::from(with_carry && self.carry());
        let set = match flag_op {
            FlagOp::Carry => lhs as u16 + rhs as u16 + carry as u16 > 0xff,
            FlagOp::Borrow => (lhs as u16) < (rhs as u16 + carry as u16),
        };
        self.set(Flag::H, set);
    }

    pub fn update_carry_u16(&mut self, lhs: u16, rhs: u16, flag_op: FlagOp) {
        let set = match flag_op {
            FlagOp::Carry => lhs as u32 + rhs as u32 > 0xffff,
            FlagOp::Borrow => lhs < rhs,
        };
        self.set(Flag::H, set);
    }

    pub fn update_half_carry_u8(&mut self, lhs: u8, rhs: u8, with_carry: bool, flag_op: FlagOp) {
        let carry = u8::from(with_carry && self.carry());
        let set = match flag_op {
            FlagOp::Carry => ((lhs & 0xf) + (rhs & 0xf)) + carry > 0xf,
            FlagOp::Borrow => (lhs & 0xf) < ((rhs & 0xf) + carry),
        };
        self.set(Flag::H, set);
    }

    pub fn update_half_carry_u16(&mut self, lhs: u16, rhs: u16, flag_op: FlagOp) {
        let set = match flag_op {
            FlagOp::Carry => ((lhs & 0xfff) + (rhs & 0xfff)) > 0xfff,
            FlagOp::Borrow => (lhs & 0xfff) < (rhs & 0xfff),
        };
        self.set(Flag::H, set);
    }

    pub fn update_zero(&mut self, value: u8) {
        self.set(Flag::Z, value == 0);
    }

    pub fn zero(&self) -> bool {
        self.is_set(Flag::Z)
    }

    pub fn set_negative(&mut self, set: bool) {
        self.set(Flag::N, set);
    }

    pub fn set_half_carry(&mut self, set: bool) {
        self.set(Flag::H, set);
    }

    pub fn carry(&self) -> bool {
        self.is_set(Flag::C)
    }

    pub fn set_carry(&mut self, set: bool) {
        self.set(Flag::C, set);
    }

    pub fn clear(&mut self) {
        self.0 = FlagSet::new_truncated(0);
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct Registers {
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub flags: Flags,
    pub a: u8,
    pub sp: u16,
    pub pc: u16,
    pub ime: bool,
}

impl Registers {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn new_post_bootrom() -> Self {
        let flags = Flags(Flag::C | Flag::H | Flag::Z);
        Self {
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xD8,
            h: 0x01,
            l: 0x4D,
            flags,
            a: 0x01,
            sp: 0xFFFE,
            pc: 0x100,
            ime: true,
        }
    }

    pub fn r(&self, index: RegisterIndex) -> u8 {
        match index.0 {
            0 => self.b,
            1 => self.c,
            2 => self.d,
            3 => self.e,
            4 => self.h,
            5 => self.l,
            6 => panic!("tried to get (hb) from registers"),
            7 => self.a,
            _ => panic!("tried to get register with invalid index"),
        }
    }

    pub fn set_r(&mut self, index: RegisterIndex, value: u8) {
        match index.0 {
            0 => self.b = value,
            1 => self.c = value,
            2 => self.d = value,
            3 => self.e = value,
            4 => self.h = value,
            5 => self.l = value,
            6 => panic!("tried to get (hb) from registers"),
            7 => self.a = value,
            _ => panic!("tried to get register with invalid index"),
        }
    }

    pub fn rr(&self, index: DoubleRegisterIndex) -> u16 {
        let bytes = match index.0 {
            0 => [self.b, self.c],
            1 => [self.d, self.e],
            2 => [self.h, self.l],
            3 => return self.sp,
            _ => panic!("tried tot get double register with invalid index"),
        };
        u16::from_be_bytes(bytes)
    }

    pub fn set_rr(&mut self, index: DoubleRegisterIndex, value: u16) {
        let [high, low] = value.to_be_bytes();
        match index.0 {
            0 => {
                self.b = high;
                self.c = low;
            }
            1 => {
                self.d = high;
                self.e = low;
            }
            2 => {
                self.h = high;
                self.l = low;
            }
            3 => self.sp = value,
            _ => panic!("tried to set double register with invalid index"),
        }
    }

    pub const fn hl(&self) -> u16 {
        u16::from_be_bytes([self.h, self.l])
    }

    pub fn set_hl(&mut self, value: u16) {
        let [high, low] = value.to_be_bytes();
        self.h = high;
        self.l = low;
    }

    pub fn af(&self) -> u16 {
        u16::from_be_bytes([self.a, self.flags.value()])
    }

    pub fn set_af(&mut self, value: u16) {
        let [high, low] = value.to_be_bytes();
        self.a = high;
        self.flags.set_value(low);
    }
}

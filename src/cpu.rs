use std::{num::Wrapping as Wr, primitive::u16};

use crate::mmu::{MemoryOps, Mmu};
use flagset::{flags, FlagSet};

#[derive(Debug, Copy, Clone)]
struct RegisterIndex(u8);

impl RegisterIndex {
    fn from_opcode_first(opcode: u8) -> Self {
        Self((opcode >> 3) & 0b11)
    }

    fn from_opcode_second(opcode: u8) -> Self {
        Self(opcode & 0b11)
    }
}

#[derive(Debug, Copy, Clone)]
struct DoubleRegisterIndex(u8);

impl DoubleRegisterIndex {
    fn from_opcode(opcode: u8) -> Self {
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

enum FlagOp {
    Carry,
    Borrow,
}

#[derive(Debug, Copy, Clone)]
struct Registers {
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    f: FlagSet<Flag>,
    a: u8,
    sp: u16,
    pc: u16,
}

impl Registers {
    fn r(&self, index: RegisterIndex) -> u8 {
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

    fn set_r(&mut self, index: RegisterIndex, value: u8) {
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

    fn rr(&self, index: DoubleRegisterIndex) -> u16 {
        let bytes = match index.0 {
            0 => [self.b, self.c],
            1 => [self.d, self.e],
            2 => [self.h, self.l],
            3 => return self.sp,
            _ => panic!("tried tot get double register with invalid index"),
        };
        u16::from_be_bytes(bytes)
    }

    fn set_rr(&mut self, index: DoubleRegisterIndex, value: u16) {
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

    fn hl(&self) -> u16 {
        u16::from_be_bytes([self.h, self.l])
    }

    fn set_hl(&mut self, value: u16) {
        let [high, low] = value.to_be_bytes();
        self.h = high;
        self.l = low;
    }

    fn af(&self) -> u16 {
        u16::from_be_bytes([self.a, self.f.bits()])
    }

    fn set_af(&mut self, value: u16) {
        let [high, low] = value.to_be_bytes();
        self.a = high;
        self.f = FlagSet::new_truncated(low);
    }

    fn flag(&self, flag: Flag) -> bool {
        self.f.contains(flag)
    }

    fn set_flag(&mut self, flag: Flag, set: bool) {
        if set {
            self.f |= flag;
        } else {
            self.f &= !flag;
        }
    }

    fn update_carry_flag_u8(&mut self, lhs: u8, rhs: u8, with_carry: bool, flag_op: FlagOp) {
        let carry = u8::from(with_carry && self.carry_flag());
        let set = match flag_op {
            FlagOp::Carry => lhs as u16 + rhs as u16 + carry as u16 > 0xff,
            FlagOp::Borrow => (lhs as u16) < (rhs as u16 + carry as u16),
        };
        self.set_flag(Flag::H, set);
    }

    fn update_carry_flag_u16(&mut self, lhs: u16, rhs: u16, flag_op: FlagOp) {
        let set = match flag_op {
            FlagOp::Carry => lhs as u32 + rhs as u32 > 0xffff,
            FlagOp::Borrow => lhs < rhs,
        };
        self.set_flag(Flag::H, set);
    }

    fn update_half_carry_flag_u8(&mut self, lhs: u8, rhs: u8, with_carry: bool, flag_op: FlagOp) {
        let carry = u8::from(with_carry && self.carry_flag());
        let set = match flag_op {
            FlagOp::Carry => ((lhs & 0xf) + (rhs & 0xf)) + carry > 0xf,
            FlagOp::Borrow => (lhs & 0xf) < ((rhs & 0xf) + carry),
        };
        self.set_flag(Flag::H, set);
    }

    fn update_half_carry_flag_u16(&mut self, lhs: u16, rhs: u16, flag_op: FlagOp) {
        let set = match flag_op {
            FlagOp::Carry => ((lhs & 0xfff) + (rhs & 0xfff)) > 0xfff,
            FlagOp::Borrow => (lhs & 0xfff) < (rhs & 0xfff),
        };
        self.set_flag(Flag::H, set);
    }

    fn update_zero_flag(&mut self, value: u8) {
        self.set_flag(Flag::Z, value == 0);
    }

    fn zero_flag(&self) -> bool {
        self.flag(Flag::Z)
    }

    fn set_negative_flag(&mut self, set: bool) {
        self.set_flag(Flag::N, set);
    }

    fn set_half_carry_flag(&mut self, set: bool) {
        self.set_flag(Flag::H, set);
    }

    fn carry_flag(&self) -> bool {
        self.flag(Flag::C)
    }

    fn set_carry_flag(&mut self, set: bool) {
        self.set_flag(Flag::C, set);
    }

    fn clear_flags(&mut self) {
        self.f = FlagSet::new_truncated(0);
    }
}

enum ExecutionState {
    Continue,
    Stop,
    IllegalInstruction,
}

pub struct Cpu {
    registers: Registers,
    mmu: Mmu,
    cycles_count: u32,
    execution_state: ExecutionState,
}

impl MemoryOps for Cpu {
    fn read_byte(&self, address: u16) -> u8 {
        self.mmu.read_byte(address)
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        self.mmu.write_byte(address, value);
    }
}

impl Cpu {
    #[rustfmt::skip]
    const OPCODE_TABLE: [fn(&mut Self, u8); 256] = [
        Self::nop,       Self::ld_rr_u16, Self::ld_mrr_a,    Self::inc_rr,  Self::inc_r,       Self::dec_r,   Self::ld_r_u8,  Self::rlca,    Self::ld_mu16_sp, Self::add_hl_rr, Self::ld_a_mrr,    Self::dec_rr,    Self::inc_r,       Self::dec_r,    Self::ld_r_u8, Self::rrca,
        Self::stop,      Self::ld_rr_u16, Self::ld_mrr_a,    Self::inc_rr,  Self::inc_r,       Self::dec_r,   Self::ld_r_u8,  Self::rla,     Self::jr_r8,      Self::add_hl_rr, Self::ld_a_mrr,    Self::dec_rr,    Self::inc_r,       Self::dec_r,    Self::ld_r_u8,  Self::rra,
        Self::jr_cc,     Self::ld_rr_u16, Self::ld_mhlinc_a, Self::inc_rr,  Self::inc_r,       Self::dec_r,   Self::ld_r_u8,  Self::daa,     Self::jr_cc,      Self::add_hl_rr, Self::ld_a_mhlinc, Self::dec_rr,    Self::inc_r,       Self::dec_r,    Self::ld_r_u8,  Self::cpl,
        Self::jr_cc,     Self::ld_rr_u16, Self::ld_mhldec_a, Self::inc_rr,  Self::inc_r,       Self::dec_r,   Self::ld_r_u8,  Self::scf,     Self::jr_cc,      Self::add_hl_rr, Self::ld_a_mhldec, Self::dec_rr,    Self::inc_r,       Self::dec_r,    Self::ld_r_u8,  Self::ccf,
        Self::ld_r_r,    Self::ld_r_r,    Self::ld_r_r,      Self::ld_r_r,  Self::ld_r_r,      Self::ld_r_r,  Self::ld_r_r,   Self::ld_r_r,  Self::ld_r_r,     Self::ld_r_r,    Self::ld_r_r,      Self::ld_r_r,    Self::ld_r_r,      Self::ld_r_r,   Self::ld_r_r,   Self::ld_r_r,
        Self::ld_r_r,    Self::ld_r_r,    Self::ld_r_r,      Self::ld_r_r,  Self::ld_r_r,      Self::ld_r_r,  Self::ld_r_r,   Self::ld_r_r,  Self::ld_r_r,     Self::ld_r_r,    Self::ld_r_r,      Self::ld_r_r,    Self::ld_r_r,      Self::ld_r_r,   Self::ld_r_r,   Self::ld_r_r,
        Self::ld_r_r,    Self::ld_r_r,    Self::ld_r_r,      Self::ld_r_r,  Self::ld_r_r,      Self::ld_r_r,  Self::ld_r_r,   Self::ld_r_r,  Self::ld_r_r,     Self::ld_r_r,    Self::ld_r_r,      Self::ld_r_r,    Self::ld_r_r,      Self::ld_r_r,   Self::ld_r_r,   Self::ld_r_r,
        Self::ld_r_r,    Self::ld_r_r,    Self::ld_r_r,      Self::ld_r_r,  Self::ld_r_r,      Self::ld_r_r,  Self::halt,     Self::ld_r_r,  Self::ld_r_r,     Self::ld_r_r,    Self::ld_r_r,      Self::ld_r_r,    Self::ld_r_r,      Self::ld_r_r,   Self::ld_r_r,   Self::ld_r_r,
        Self::add_a_r,   Self::add_a_r,   Self::add_a_r,     Self::add_a_r, Self::add_a_r,     Self::add_a_r, Self::add_a_r,  Self::add_a_r, Self::adc_a_r,    Self::adc_a_r,   Self::adc_a_r,     Self::adc_a_r,   Self::adc_a_r,     Self::adc_a_r,  Self::adc_a_r,  Self::adc_a_r,
        Self::sub_a_r,   Self::sub_a_r,   Self::sub_a_r,     Self::sub_a_r, Self::sub_a_r,     Self::sub_a_r, Self::sub_a_r,  Self::sub_a_r, Self::sbc_a_r,    Self::sbc_a_r,   Self::sbc_a_r,     Self::sbc_a_r,   Self::sbc_a_r,     Self::sbc_a_r,  Self::sbc_a_r,  Self::sbc_a_r,
        Self::and_a_r,   Self::and_a_r,   Self::and_a_r,     Self::and_a_r, Self::and_a_r,     Self::and_a_r, Self::and_a_r,  Self::and_a_r, Self::xor_a_r,    Self::xor_a_r,   Self::xor_a_r,     Self::xor_a_r,   Self::xor_a_r,     Self::xor_a_r,  Self::xor_a_r,  Self::xor_a_r,
        Self::or_a_r,    Self::or_a_r,    Self::or_a_r,      Self::or_a_r,  Self::or_a_r,      Self::or_a_r,  Self::or_a_r,   Self::or_a_r,  Self::cp_a_r,     Self::cp_a_r,    Self::cp_a_r,      Self::cp_a_r,    Self::cp_a_r,      Self::cp_a_r,   Self::cp_a_r,   Self::cp_a_r,
        Self::ret_cc,    Self::pop_rr,    Self::jp_cc_u16,   Self::jp_u16,  Self::call_cc_u16, Self::push_rr, Self::add_a_u8, Self::rst,     Self::ret_cc,     Self::ret,       Self::jp_cc_u16,   Self::prefix_cb, Self::call_cc_u16, Self::call_u16, Self::adc_a_u8, Self::rst,
        Self::ret_cc,    Self::pop_rr,    Self::jp_cc_u16,   Self::ill,     Self::call_cc_u16, Self::push_rr, Self::sub_a_u8, Self::rst,     Self::ret_cc,     Self::reti,      Self::jp_cc_u16,   Self::ill,       Self::call_cc_u16, Self::ill,      Self::sbc_a_u8, Self::rst,
        Self::ldh_mu8_a, Self::pop_rr,    Self::ldh_mc_a,    Self::ill,     Self::ill,         Self::push_rr, Self::and_a_u8, Self::rst,     Self::add_sp_r8,  Self::jp_mhl,    Self::ld_mu16_a,   Self::ill,       Self::ill,         Self::ill,      Self::xor_a_u8, Self::rst,
        Self::ldh_a_mu8, Self::pop_af,    Self::ldh_a_mc,    Self::di,      Self::ill,         Self::push_af, Self::or_a_u8,  Self::rst,     Self::ld_hl_spu8, Self::ld_sp_hl,  Self::ld_a_mu16,   Self::ei,        Self::ill,         Self::ill,      Self::cp_a_u8,  Self::rst
        ];

    pub fn new() -> Self {
        todo!()
    }

    fn tick(&mut self) {
        let opcode = self.fetch_byte_pc();
        Self::OPCODE_TABLE[opcode as usize](self, opcode);
    }

    fn fetch_byte_pc(&mut self) -> u8 {
        let value = self.read_byte(self.registers.pc);
        self.registers.pc += 1;
        value
    }

    fn fetch_dbyte_pc(&mut self) -> u16 {
        let lower = self.fetch_byte_pc();
        let upper = self.fetch_byte_pc();
        u16::from_be_bytes([upper, lower])
    }

    fn r(&self, index: RegisterIndex) -> u8 {
        if index.0 == 6 {
            self.read_byte(self.registers.hl())
        } else {
            self.r(index)
        }
    }

    fn set_r(&mut self, index: RegisterIndex, value: u8) {
        if index.0 == 6 {
            self.write_byte(self.registers.hl(), value);
        } else {
            self.set_r(index, value);
        }
    }

    fn test_cc(&self, opcode: u8) -> bool {
        match (opcode >> 3) & 0b11 {
            0 => !self.registers.zero_flag(),
            1 => self.registers.zero_flag(),
            2 => !self.registers.carry_flag(),
            3 => self.registers.carry_flag(),
            _ => unreachable!(),
        }
    }

    fn pop_stack(&mut self) -> u16 {
        let value = self.read_dbyte(self.registers.sp);
        self.registers.sp += 2;
        value
    }

    fn push_stack(&mut self, value: u16) {
        self.write_dbyte(self.registers.sp, value);
        self.registers.sp -= 2;
    }

    // Opcodes implementations.

    // Control operations.

    fn nop(&mut self, _opcode: u8) {}

    fn stop(&mut self, _opcode: u8) {
        self.execution_state = ExecutionState::Stop;
    }

    fn halt(&mut self, _opcode: u8) {
        todo!()
    }

    fn ill(&mut self, _opcode: u8) {
        self.execution_state = ExecutionState::IllegalInstruction;
    }

    // Load operations.

    fn ld_r_r(&mut self, opcode: u8) {
        let target_index = RegisterIndex::from_opcode_first(opcode);
        let value_index = RegisterIndex::from_opcode_second(opcode);
        self.set_r(target_index, self.r(value_index));
    }

    fn ld_r_u8(&mut self, opcode: u8) {
        let index = RegisterIndex::from_opcode_first(opcode);
        let value = self.fetch_byte_pc();
        self.set_r(index, value);
    }

    fn ld_rr_u16(&mut self, opcode: u8) {
        let value = self.fetch_dbyte_pc();
        let index = DoubleRegisterIndex::from_opcode(opcode);
        self.registers.set_rr(index, value);
    }

    fn ld_a_mrr(&mut self, opcode: u8) {
        let index = DoubleRegisterIndex::from_opcode(opcode);
        let address = self.registers.rr(index);
        let value = self.read_byte(address);
        self.registers.a = value;
    }

    fn ld_mrr_a(&mut self, opcode: u8) {
        let index = DoubleRegisterIndex::from_opcode(opcode);
        let address = self.registers.rr(index);
        self.write_byte(address, self.registers.a);
    }

    fn ld_mhlinc_a(&mut self, _opcode: u8) {
        let address = self.registers.hl();
        self.write_byte(address, self.registers.a);
        self.registers.set_hl((Wr(address) + Wr(1)).0);
    }

    fn ld_mhldec_a(&mut self, _opcode: u8) {
        let address = self.registers.hl();
        self.write_byte(address, self.registers.a);
        self.registers.set_hl((Wr(address) - Wr(1)).0);
    }

    fn ld_a_mhlinc(&mut self, _opcode: u8) {
        let address = self.registers.hl();
        self.registers.a = self.read_byte(address);
        self.registers.set_hl((Wr(address) + Wr(1)).0);
    }

    fn ld_a_mhldec(&mut self, _opcode: u8) {
        let address = self.registers.hl();
        self.registers.a = self.read_byte(address);
        self.registers.set_hl((Wr(address) - Wr(1)).0);
    }

    fn ld_a_mu16(&mut self, _opcode: u8) {
        let address = self.fetch_dbyte_pc();
        self.registers.a = self.read_byte(address);
    }

    fn ld_mu16_a(&mut self, _opcode: u8) {
        let address = self.fetch_dbyte_pc();
        self.write_byte(address, self.registers.a);
    }

    fn ld_mu16_sp(&mut self, _opcode: u8) {
        let address = self.fetch_dbyte_pc();
        self.write_dbyte(address, self.registers.sp);
    }

    fn ld_hl_spu8(&mut self, _opcode: u8) {
        let sp_value = self.registers.sp;
        let u8_value = self.fetch_byte_pc() as i8 as u16;
        self.registers.clear_flags();
        self.registers
            .update_carry_flag_u16(sp_value, u8_value, FlagOp::Carry);
        self.registers
            .update_half_carry_flag_u16(sp_value, u8_value, FlagOp::Carry);
        self.registers.set_hl((Wr(sp_value) + Wr(u8_value)).0);
    }

    fn ld_sp_hl(&mut self, _opcode: u8) {
        self.registers.sp = self.registers.hl();
    }

    fn ldh_mu8_a(&mut self, _opcode: u8) {
        let address = 0xFF00 | (self.fetch_byte_pc() as u16);
        self.write_byte(address, self.registers.a);
    }

    fn ldh_a_mu8(&mut self, _opcode: u8) {
        let address = 0xFF00 | (self.fetch_byte_pc() as u16);
        self.registers.a = self.read_byte(address);
    }

    fn ldh_mc_a(&mut self, _opcode: u8) {
        let address = 0xFF00 | (self.registers.c as u16);
        self.write_byte(address, self.registers.a);
    }

    fn ldh_a_mc(&mut self, _opcode: u8) {
        let address = 0xFF00 | (self.registers.c as u16);
        self.registers.a = self.read_byte(address);
    }

    // Increment / Decrement operations.

    fn inc_r(&mut self, opcode: u8) {
        let index = RegisterIndex::from_opcode_first(opcode);
        let prev_value = self.r(index);
        self.registers
            .update_half_carry_flag_u8(prev_value, 1, false, FlagOp::Carry);
        let value = Wr(prev_value) + Wr(1);
        self.registers.set_negative_flag(false);
        self.registers.update_zero_flag(value.0);
        self.set_r(index, value.0);
    }

    fn dec_r(&mut self, opcode: u8) {
        let index = RegisterIndex::from_opcode_first(opcode);
        let prev_value = self.r(index);
        self.registers
            .update_half_carry_flag_u8(prev_value, 1, false, FlagOp::Borrow);
        let value = Wr(prev_value) - Wr(1);
        self.registers.set_negative_flag(true);
        self.registers.update_zero_flag(value.0);
        self.set_r(index, value.0);
    }

    fn inc_rr(&mut self, opcode: u8) {
        let index = DoubleRegisterIndex::from_opcode(opcode);
        let value = Wr(self.registers.rr(index)) + Wr(1);
        self.registers.set_rr(index, value.0);
    }

    fn dec_rr(&mut self, opcode: u8) {
        let index = DoubleRegisterIndex::from_opcode(opcode);
        let value = Wr(self.registers.rr(index)) - Wr(1);
        self.registers.set_rr(index, value.0);
    }

    // Arithmetic operations.

    fn add_a_generic(&mut self, added: u8, with_carry: bool) {
        self.registers.set_negative_flag(false);
        self.registers.update_half_carry_flag_u8(
            self.registers.a,
            added,
            with_carry,
            FlagOp::Carry,
        );
        self.registers
            .update_carry_flag_u8(self.registers.a, added, with_carry, FlagOp::Carry);
        let carry_bit = u8::from(with_carry && self.registers.carry_flag());
        let value = (Wr(self.registers.a) + Wr(added) + Wr(carry_bit)).0;
        self.registers.update_zero_flag(value);
        self.registers.a = value;
    }

    fn add_a_r(&mut self, opcode: u8) {
        let index = RegisterIndex::from_opcode_second(opcode);
        let added = self.r(index);
        self.add_a_generic(added, false);
    }

    fn add_a_u8(&mut self, _opcode: u8) {
        let value = self.fetch_byte_pc();
        self.add_a_generic(value, false);
    }

    fn adc_a_r(&mut self, opcode: u8) {
        let index = RegisterIndex::from_opcode_second(opcode);
        let added = self.r(index);
        self.add_a_generic(added, true);
    }

    fn adc_a_u8(&mut self, _opcode: u8) {
        let value = self.fetch_byte_pc();
        self.add_a_generic(value, true);
    }

    fn sub_a_generic(&mut self, subbed: u8, with_carry: bool) {
        self.registers.set_negative_flag(true);
        self.registers.update_half_carry_flag_u8(
            self.registers.a,
            subbed,
            with_carry,
            FlagOp::Borrow,
        );
        self.registers
            .update_carry_flag_u8(self.registers.a, subbed, with_carry, FlagOp::Borrow);
        let carry_bit = u8::from(with_carry && self.registers.carry_flag());
        let value = (Wr(self.registers.a) - Wr(subbed) - Wr(carry_bit)).0;
        self.registers.update_zero_flag(value);
        self.registers.a = value;
    }

    fn sub_a_r(&mut self, opcode: u8) {
        let index = RegisterIndex::from_opcode_second(opcode);
        let subbed = self.r(index);
        self.sub_a_generic(subbed, false);
    }

    fn sub_a_u8(&mut self, _opcode: u8) {
        let value = self.fetch_byte_pc();
        self.sub_a_generic(value, false);
    }

    fn sbc_a_r(&mut self, opcode: u8) {
        let index = RegisterIndex::from_opcode_second(opcode);
        let subbed = self.r(index);
        self.sub_a_generic(subbed, true);
    }

    fn sbc_a_u8(&mut self, _opcode: u8) {
        let value = self.fetch_byte_pc();
        self.sub_a_generic(value, true);
    }

    fn add_hl_rr(&mut self, opcode: u8) {
        let index = DoubleRegisterIndex::from_opcode(opcode);
        let hl_value = self.registers.hl();
        let reg_value = self.registers.rr(index);
        self.registers.set_negative_flag(false);
        self.registers
            .update_half_carry_flag_u16(hl_value, reg_value, FlagOp::Carry);
        self.registers
            .update_carry_flag_u16(hl_value, reg_value, FlagOp::Carry);
        self.registers.set_hl((Wr(hl_value) + Wr(reg_value)).0);
    }

    fn add_sp_r8(&mut self, _opcode: u8) {
        self.registers.clear_flags();
        let sp_value = self.registers.sp;
        let r8_value = self.fetch_byte_pc() as i8 as u16;
        self.registers
            .update_carry_flag_u16(sp_value, r8_value, FlagOp::Carry);
        self.registers
            .update_half_carry_flag_u16(sp_value, r8_value, FlagOp::Carry);
        self.registers.sp = (Wr(sp_value) + Wr(r8_value)).0;
    }

    // Boolean operations.

    fn and_a_generic(&mut self, value: u8) {
        self.registers.a &= value;
        self.registers.clear_flags();
        self.registers.set_half_carry_flag(true);
        self.registers.update_zero_flag(self.registers.a);
    }

    fn and_a_r(&mut self, opcode: u8) {
        let index = RegisterIndex::from_opcode_second(opcode);
        self.and_a_generic(self.r(index));
    }

    fn and_a_u8(&mut self, _opcode: u8) {
        let value = self.fetch_byte_pc();
        self.and_a_generic(value);
    }

    fn or_a_generic(&mut self, value: u8) {
        self.registers.a |= value;
        self.registers.clear_flags();
        self.registers.update_zero_flag(self.registers.a);
    }

    fn or_a_r(&mut self, opcode: u8) {
        let index = RegisterIndex::from_opcode_second(opcode);
        self.or_a_generic(self.r(index));
    }

    fn or_a_u8(&mut self, _opcode: u8) {
        let value = self.fetch_byte_pc();
        self.or_a_generic(value);
    }

    fn xor_a_generic(&mut self, value: u8) {
        self.registers.a ^= value;
        self.registers.clear_flags();
        self.registers.update_zero_flag(self.registers.a);
    }

    fn xor_a_r(&mut self, opcode: u8) {
        let index = RegisterIndex::from_opcode_second(opcode);
        self.xor_a_generic(self.r(index));
    }

    fn xor_a_u8(&mut self, opcode: u8) {
        let value = self.fetch_byte_pc();
        self.xor_a_generic(value);
    }

    fn cp_a_generic(&mut self, subbed: u8) {
        self.registers.set_negative_flag(true);
        self.registers
            .update_half_carry_flag_u8(self.registers.a, subbed, false, FlagOp::Borrow);
        self.registers
            .update_carry_flag_u8(self.registers.a, subbed, false, FlagOp::Borrow);
        let value = (Wr(self.registers.a) - Wr(subbed)).0;
        self.registers.update_zero_flag(value);
    }

    fn cp_a_r(&mut self, opcode: u8) {
        let index = RegisterIndex::from_opcode_second(opcode);
        self.cp_a_generic(self.r(index));
    }

    fn cp_a_u8(&mut self, _opcode: u8) {
        let value = self.fetch_byte_pc();
        self.cp_a_generic(value);
    }

    // Shift operations.

    fn rlca(&mut self, _opcode: u8) {
        let carry_set = (self.registers.a >> 7) != 0;
        self.registers.clear_flags();
        self.registers.set_carry_flag(carry_set);
        self.registers.a = self.registers.a.rotate_left(1);
    }

    fn rrca(&mut self, _opcode: u8) {
        let carry_set = (self.registers.a & 0b1) != 0;
        self.registers.clear_flags();
        self.registers.set_carry_flag(carry_set);
        self.registers.a = self.registers.a.rotate_right(1);
    }

    fn rla(&mut self, _opcode: u8) {
        let carry_set = (self.registers.a >> 7) != 0;
        let carry_bit = u8::from(self.registers.carry_flag());
        self.registers.clear_flags();
        self.registers.set_carry_flag(carry_set);
        self.registers.a = (self.registers.a << 1) | carry_bit;
    }

    fn rra(&mut self, _opcode: u8) {
        let carry_set = (self.registers.a & 0b1) != 0;
        let carry_bit = u8::from(self.registers.carry_flag()) << 7;
        self.registers.clear_flags();
        self.registers.set_carry_flag(carry_set);
        self.registers.a = (self.registers.a >> 1) | carry_bit;
    }

    fn cpl(&mut self, _opcode: u8) {
        self.registers.a = !self.registers.a;
        self.registers.set_negative_flag(true);
        self.registers.set_half_carry_flag(true);
    }

    fn daa(&mut self, _opcode: u8) {
        todo!()
    }

    // Control flow operations.

    fn jp_u16(&mut self, _opcode: u8) {
        self.registers.pc = self.fetch_dbyte_pc();
    }

    fn jp_mhl(&mut self, _opcode: u8) {
        self.registers.pc = self.read_dbyte(self.registers.hl());
    }

    fn jp_cc_u16(&mut self, opcode: u8) {
        let address = self.fetch_dbyte_pc();
        if self.test_cc(opcode) {
            self.registers.pc = address;
        }
    }

    fn jr_r8(&mut self, _opcode: u8) {
        self.registers.pc = (Wr(self.registers.pc) + Wr(self.fetch_byte_pc() as i8 as u16)).0;
    }

    fn jr_cc(&mut self, opcode: u8) {
        let address = (Wr(self.registers.pc) + Wr(self.fetch_byte_pc() as i8 as u16)).0;
        if self.test_cc(opcode) {
            self.registers.pc = address;
        }
    }

    fn call_u16(&mut self, _opcode: u8) {
        self.push_stack(self.registers.pc);
        self.registers.pc = self.fetch_dbyte_pc();
    }

    fn call_cc_u16(&mut self, opcode: u8) {
        let address = self.fetch_dbyte_pc();
        if self.test_cc(opcode) {
            self.push_stack(self.registers.pc);
            self.registers.pc = address;
        }
    }

    fn ret(&mut self, _opcode: u8) {
        self.registers.pc = self.pop_stack();
    }

    fn reti(&mut self, _opcode: u8) {
        todo!()
    }

    fn ret_cc(&mut self, opcode: u8) {
        if self.test_cc(opcode) {
            self.ret(opcode);
        }
    }

    fn rst(&mut self, opcode: u8) {
        self.push_stack(self.registers.pc);
        self.registers.pc = ((opcode >> 3) & 0b111) as u16;
        self.registers.pc = self.fetch_dbyte_pc();
    }

    // Flags operations.

    fn scf(&mut self, _opcode: u8) {
        self.registers.set_half_carry_flag(false);
        self.registers.set_negative_flag(false);
        self.registers.set_carry_flag(true);
    }

    fn ccf(&mut self, _opcode: u8) {
        self.registers.set_half_carry_flag(false);
        self.registers.set_negative_flag(false);
        self.registers.set_carry_flag(!self.registers.carry_flag());
    }

    // Stack operations.

    fn push_rr(&mut self, opcode: u8) {
        let index = DoubleRegisterIndex::from_opcode(opcode);
        self.push_stack(self.registers.rr(index));
    }

    fn push_af(&mut self, _opcode: u8) {
        self.push_stack(self.registers.af());
    }

    fn pop_rr(&mut self, opcode: u8) {
        let index = DoubleRegisterIndex::from_opcode(opcode);
        let value = self.pop_stack();
        self.registers.set_rr(index, value);
    }

    fn pop_af(&mut self, opcode: u8) {
        let value = self.pop_stack();
        self.registers.set_af(value);
    }

    // Interrupt operations.

    fn ei(&mut self, _opcode: u8) {
        todo!()
    }

    fn di(&mut self, _opcode: u8) {
        todo!()
    }

    // CB prefixed operations.

    fn prefix_cb(&mut self, _opcode: u8) {
        let cb_opcode = self.fetch_byte_pc();
        match cb_opcode {
            0x00..=0x07 => self.rlc(cb_opcode),
            0x08..=0x0F => self.rrc(cb_opcode),
            0x10..=0x17 => self.rl(cb_opcode),
            0x18..=0x1F => self.rr(cb_opcode),
            0x20..=0x27 => self.sla(cb_opcode),
            0x28..=0x2F => self.sra(cb_opcode),
            0x30..=0x37 => self.swap(cb_opcode),
            0x38..=0x3F => self.srl(cb_opcode),
            0x40..=0x7F => self.bit(cb_opcode),
            0x80..=0xBF => self.res(cb_opcode),
            0xC0..=0xFF => self.set(cb_opcode)
        }
    }

    fn rlc(&mut self, opcode: u8) {
        let index = RegisterIndex::from_opcode_second(opcode);
        let value = self.r(index);
        self.registers.set_carry_flag(value >> 7 != 0);
        self.set_r(index, value.rotate_left(1));
    }

    fn rrc(&mut self, opcode: u8) {
        let index = RegisterIndex::from_opcode_second(opcode);
        let value = self.r(index);
        self.registers.set_carry_flag(value & 0b1 != 0);
        self.set_r(index, value.rotate_right(1));
    }

    fn rl(&mut self, opcode: u8) {
        let index = RegisterIndex::from_opcode_second(opcode);
        let carry_bit = u8::from(self.registers.carry_flag());
        let old_value = self.r(index);
        let value = (old_value << 1) | carry_bit;
        self.registers.clear_flags();
        self.registers.set_carry_flag(old_value >> 7 != 0);
        self.registers.update_zero_flag(value);
        self.set_r(index, value);
    }

    fn rr(&mut self, opcode: u8) {
        let index = RegisterIndex::from_opcode_second(opcode);
        let carry_bit = u8::from(self.registers.carry_flag()) << 7;
        let old_value = self.r(index);
        let value = (old_value >> 1) | carry_bit;
        self.registers.clear_flags();
        self.registers.set_carry_flag(old_value & 0b1 != 0);
        self.registers.update_zero_flag(value);
        self.set_r(index, value);
    }

    fn sla(&mut self, opcode: u8) {
        let index = RegisterIndex::from_opcode_second(opcode);
        let old_value = self.r(index);
        let value = old_value << 1;
        self.registers.clear_flags();
        self.registers.set_carry_flag(old_value >> 7 != 0);
        self.registers.update_zero_flag(value);
        self.set_r(index, value);
    }

    fn sra(&mut self, opcode: u8) {
        let index = RegisterIndex::from_opcode_second(opcode);
        let old_value = self.r(index);
        let value = (old_value >> 1) & (old_value & 0x80);
        self.registers.clear_flags();
        self.registers.set_carry_flag(old_value & 0b1 != 0);
        self.registers.update_zero_flag(value);
        self.set_r(index, value);
    }

    fn srl(&mut self, opcode: u8) {
        let index = RegisterIndex::from_opcode_second(opcode);
        let old_value = self.r(index);
        let value = old_value >> 1;
        self.registers.clear_flags();
        self.registers.set_carry_flag(old_value & 0b1 != 0);
        self.registers.update_zero_flag(value);
        self.set_r(index, value);
    }

    fn swap(&mut self, opcode: u8) {
        let index = RegisterIndex::from_opcode_second(opcode);
        let value = self.r(index).rotate_left(4);
        self.registers.clear_flags();
        self.registers.update_zero_flag(value);
        self.set_r(index, value);
    }

    fn bit(&mut self, opcode: u8) {
        let index = RegisterIndex::from_opcode_second(opcode);
        self.registers.set_negative_flag(false);
        self.registers.set_half_carry_flag(true);
        let bit_test =  self.r(index) & (1 << ((opcode >> 3) & 0b111));
        self.registers.update_zero_flag(bit_test);
    }

    fn res(&mut self, opcode: u8) {
        let index = RegisterIndex::from_opcode_second(opcode);
        let value = self.r(index) & !(1 << ((opcode >> 3) & 0b111));
        self.set_r(index, value);
    }

    fn set(&mut self, opcode: u8) {
        let index = RegisterIndex::from_opcode_second(opcode);
        let value = self.r(index) | (1 << ((opcode >> 3) & 0b111));
        self.set_r(index, value);
    }
}

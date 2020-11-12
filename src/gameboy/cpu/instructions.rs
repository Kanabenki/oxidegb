use std::num::Wrapping as Wr;

use super::{
    registers::{DoubleRegisterIndex, FlagOp, RegisterIndex},
    Cpu, ExecutionState,
};
use crate::gameboy::mmu::MemoryOps;

impl Cpu {
    #[rustfmt::skip]
    pub(super) const OPCODE_TABLE: [fn(&mut Self, u8); 256] = [
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
        let value = self.r(RegisterIndex::from_opcode_second(opcode));
        self.set_r(target_index, value);
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
        self.registers.flags.clear();
        self.registers
            .flags
            .update_carry_u16(sp_value, u8_value, FlagOp::Carry);
        self.registers
            .flags
            .update_half_carry_u16(sp_value, u8_value, FlagOp::Carry);
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
            .flags
            .update_half_carry_u8(prev_value, 1, false, FlagOp::Carry);
        let value = Wr(prev_value) + Wr(1);
        self.registers.flags.set_negative(false);
        self.registers.flags.update_zero(value.0);
        self.set_r(index, value.0);
    }

    fn dec_r(&mut self, opcode: u8) {
        let index = RegisterIndex::from_opcode_first(opcode);
        let prev_value = self.r(index);
        self.registers
            .flags
            .update_half_carry_u8(prev_value, 1, false, FlagOp::Borrow);
        let value = Wr(prev_value) - Wr(1);
        self.registers.flags.set_negative(true);
        self.registers.flags.update_zero(value.0);
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
        self.registers.flags.set_negative(false);
        self.registers.flags.update_half_carry_u8(
            self.registers.a,
            added,
            with_carry,
            FlagOp::Carry,
        );
        self.registers
            .flags
            .update_carry_u8(self.registers.a, added, with_carry, FlagOp::Carry);
        let carry_bit = u8::from(with_carry && self.registers.flags.carry());
        let value = (Wr(self.registers.a) + Wr(added) + Wr(carry_bit)).0;
        self.registers.flags.update_zero(value);
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
        self.registers.flags.set_negative(true);
        self.registers.flags.update_half_carry_u8(
            self.registers.a,
            subbed,
            with_carry,
            FlagOp::Borrow,
        );
        self.registers
            .flags
            .update_carry_u8(self.registers.a, subbed, with_carry, FlagOp::Borrow);
        let carry_bit = u8::from(with_carry && self.registers.flags.carry());
        let value = (Wr(self.registers.a) - Wr(subbed) - Wr(carry_bit)).0;
        self.registers.flags.update_zero(value);
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
        self.registers.flags.set_negative(false);
        self.registers
            .flags
            .update_half_carry_u16(hl_value, reg_value, FlagOp::Carry);
        self.registers
            .flags
            .update_carry_u16(hl_value, reg_value, FlagOp::Carry);
        self.registers.set_hl((Wr(hl_value) + Wr(reg_value)).0);
    }

    fn add_sp_r8(&mut self, _opcode: u8) {
        self.registers.flags.clear();
        let sp_value = self.registers.sp;
        let r8_value = self.fetch_byte_pc() as i8 as u16;
        self.registers
            .flags
            .update_carry_u16(sp_value, r8_value, FlagOp::Carry);
        self.registers
            .flags
            .update_half_carry_u16(sp_value, r8_value, FlagOp::Carry);
        self.registers.sp = (Wr(sp_value) + Wr(r8_value)).0;
    }

    // Boolean operations.

    fn and_a_generic(&mut self, value: u8) {
        self.registers.a &= value;
        self.registers.flags.clear();
        self.registers.flags.set_half_carry(true);
        self.registers.flags.update_zero(self.registers.a);
    }

    fn and_a_r(&mut self, opcode: u8) {
        let value = self.r(RegisterIndex::from_opcode_second(opcode));
        self.and_a_generic(value);
    }

    fn and_a_u8(&mut self, _opcode: u8) {
        let value = self.fetch_byte_pc();
        self.and_a_generic(value);
    }

    fn or_a_generic(&mut self, value: u8) {
        self.registers.a |= value;
        self.registers.flags.clear();
        self.registers.flags.update_zero(self.registers.a);
    }

    fn or_a_r(&mut self, opcode: u8) {
        let value = self.r(RegisterIndex::from_opcode_second(opcode));
        self.or_a_generic(value);
    }

    fn or_a_u8(&mut self, _opcode: u8) {
        let value = self.fetch_byte_pc();
        self.or_a_generic(value);
    }

    fn xor_a_generic(&mut self, value: u8) {
        self.registers.a ^= value;
        self.registers.flags.clear();
        self.registers.flags.update_zero(self.registers.a);
    }

    fn xor_a_r(&mut self, opcode: u8) {
        let value = self.r(RegisterIndex::from_opcode_second(opcode));
        self.xor_a_generic(value);
    }

    fn xor_a_u8(&mut self, _opcode: u8) {
        let value = self.fetch_byte_pc();
        self.xor_a_generic(value);
    }

    fn cp_a_generic(&mut self, subbed: u8) {
        self.registers.flags.set_negative(true);
        self.registers
            .flags
            .update_half_carry_u8(self.registers.a, subbed, false, FlagOp::Borrow);
        self.registers
            .flags
            .update_carry_u8(self.registers.a, subbed, false, FlagOp::Borrow);
        let value = (Wr(self.registers.a) - Wr(subbed)).0;
        self.registers.flags.update_zero(value);
    }

    fn cp_a_r(&mut self, opcode: u8) {
        let value = self.r(RegisterIndex::from_opcode_second(opcode));
        self.cp_a_generic(value);
    }

    fn cp_a_u8(&mut self, _opcode: u8) {
        let value = self.fetch_byte_pc();
        self.cp_a_generic(value);
    }

    // Shift operations.

    fn rlca(&mut self, _opcode: u8) {
        let carry_set = (self.registers.a >> 7) != 0;
        self.registers.flags.clear();
        self.registers.flags.set_carry(carry_set);
        self.registers.a = self.registers.a.rotate_left(1);
    }

    fn rrca(&mut self, _opcode: u8) {
        let carry_set = (self.registers.a & 0b1) != 0;
        self.registers.flags.clear();
        self.registers.flags.set_carry(carry_set);
        self.registers.a = self.registers.a.rotate_right(1);
    }

    fn rla(&mut self, _opcode: u8) {
        let carry_set = (self.registers.a >> 7) != 0;
        let carry_bit = u8::from(self.registers.flags.carry());
        self.registers.flags.clear();
        self.registers.flags.set_carry(carry_set);
        self.registers.a = (self.registers.a << 1) | carry_bit;
    }

    fn rra(&mut self, _opcode: u8) {
        let carry_set = (self.registers.a & 0b1) != 0;
        let carry_bit = u8::from(self.registers.flags.carry()) << 7;
        self.registers.flags.clear();
        self.registers.flags.set_carry(carry_set);
        self.registers.a = (self.registers.a >> 1) | carry_bit;
    }

    fn cpl(&mut self, _opcode: u8) {
        self.registers.a = !self.registers.a;
        self.registers.flags.set_negative(true);
        self.registers.flags.set_half_carry(true);
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
        self.registers.flags.set_half_carry(false);
        self.registers.flags.set_negative(false);
        self.registers.flags.set_carry(true);
    }

    fn ccf(&mut self, _opcode: u8) {
        self.registers.flags.set_half_carry(false);
        self.registers.flags.set_negative(false);
        self.registers
            .flags
            .set_carry(!self.registers.flags.carry());
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

    fn pop_af(&mut self, _opcode: u8) {
        let value = self.pop_stack();
        self.registers.set_af(value);
    }

    // Interrupt operations.

    fn ei(&mut self, _opcode: u8) {
        self.registers.ime = true;
    }

    fn di(&mut self, _opcode: u8) {
        self.registers.ime = false;
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
            0xC0..=0xFF => self.set(cb_opcode),
        }
    }

    fn rlc(&mut self, opcode: u8) {
        let index = RegisterIndex::from_opcode_second(opcode);
        let value = self.r(index);
        self.registers.flags.set_carry(value >> 7 != 0);
        self.set_r(index, value.rotate_left(1));
    }

    fn rrc(&mut self, opcode: u8) {
        let index = RegisterIndex::from_opcode_second(opcode);
        let value = self.r(index);
        self.registers.flags.set_carry(value & 0b1 != 0);
        self.set_r(index, value.rotate_right(1));
    }

    fn rl(&mut self, opcode: u8) {
        let index = RegisterIndex::from_opcode_second(opcode);
        let carry_bit = u8::from(self.registers.flags.carry());
        let old_value = self.r(index);
        let value = (old_value << 1) | carry_bit;
        self.registers.flags.clear();
        self.registers.flags.set_carry(old_value >> 7 != 0);
        self.registers.flags.update_zero(value);
        self.set_r(index, value);
    }

    fn rr(&mut self, opcode: u8) {
        let index = RegisterIndex::from_opcode_second(opcode);
        let carry_bit = u8::from(self.registers.flags.carry()) << 7;
        let old_value = self.r(index);
        let value = (old_value >> 1) | carry_bit;
        self.registers.flags.clear();
        self.registers.flags.set_carry(old_value & 0b1 != 0);
        self.registers.flags.update_zero(value);
        self.set_r(index, value);
    }

    fn sla(&mut self, opcode: u8) {
        let index = RegisterIndex::from_opcode_second(opcode);
        let old_value = self.r(index);
        let value = old_value << 1;
        self.registers.flags.clear();
        self.registers.flags.set_carry(old_value >> 7 != 0);
        self.registers.flags.update_zero(value);
        self.set_r(index, value);
    }

    fn sra(&mut self, opcode: u8) {
        let index = RegisterIndex::from_opcode_second(opcode);
        let old_value = self.r(index);
        let value = (old_value >> 1) & (old_value & 0x80);
        self.registers.flags.clear();
        self.registers.flags.set_carry(old_value & 0b1 != 0);
        self.registers.flags.update_zero(value);
        self.set_r(index, value);
    }

    fn srl(&mut self, opcode: u8) {
        let index = RegisterIndex::from_opcode_second(opcode);
        let old_value = self.r(index);
        let value = old_value >> 1;
        self.registers.flags.clear();
        self.registers.flags.set_carry(old_value & 0b1 != 0);
        self.registers.flags.update_zero(value);
        self.set_r(index, value);
    }

    fn swap(&mut self, opcode: u8) {
        let index = RegisterIndex::from_opcode_second(opcode);
        let value = self.r(index).rotate_left(4);
        self.registers.flags.clear();
        self.registers.flags.update_zero(value);
        self.set_r(index, value);
    }

    fn bit(&mut self, opcode: u8) {
        let index = RegisterIndex::from_opcode_second(opcode);
        self.registers.flags.set_negative(false);
        self.registers.flags.set_half_carry(true);
        let bit_test = self.r(index) & (1 << ((opcode >> 3) & 0b111));
        self.registers.flags.update_zero(bit_test);
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

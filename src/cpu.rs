use crate::mmu::{MemoryOps, Mmu};
use flagset::{flags, FlagSet};

flags! {
    enum Flags: u8 {
        Z = 0b10000000,
        N = 0b01000000,
        H = 0b00100000,
        C = 0b00010000
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Registers {
    pub a: u8,
    f: FlagSet<Flags>,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub sp: u16,
    pub pc: u16,
}

impl Registers {
    pub fn af(&self) -> u16 {
        (self.a as u16) << 8 | self.f.bits() as u16
    }

    pub fn set_af(&mut self, value: u16) {
        self.a = (value >> 8) as u8;
        self.f = FlagSet::new_truncated(value as u8);
    }

    pub fn bc(&self) -> u16 {
        (self.b as u16) << 8 | self.c as u16
    }

    pub fn set_bc(&mut self, value: u16) {
        self.b = (value >> 8) as u8;
        self.c = value as u8;
    }

    pub fn de(&self) -> u16 {
        (self.d as u16) << 8 | self.e as u16
    }

    pub fn set_de(&mut self, value: u16) {
        self.d = (value >> 8) as u8;
        self.e = value as u8;
    }

    pub fn hl(&self) -> u16 {
        (self.h as u16) << 8 | self.l as u16
    }

    pub fn set_hl(&mut self, value: u16) {
        self.h = (value >> 8) as u8;
        self.l = value as u8;
    }
}

/*

#[rustfmt::skip]
const OPCODE_TABLE: [fn(&mut Cpu, u8) -> (); 256] = [
    Cpu::nop,       Cpu::ld_rr_u16, Cpu::ld_mrr_a,  Cpu::inc_rr,   Cpu::inc_r,    Cpu::dec_r,    Cpu::ld_r_u8,   Cpu::rlc_r,    Cpu::ld_mu16_sp, Cpu::add_hl_rr, Cpu::ld_a_mrr,  Cpu::dec_rr,    Cpu::inc_r,   Cpu::dec_r,   Cpu::ld_r_u8,   Cpu::rrc_r,
    Cpu::stop,      Cpu::ld_rr_u16, Cpu::ld_mrr_a,  Cpu::inc_rr,   Cpu::inc_r,    Cpu::dec_r,    Cpu::ld_r_u8,   Cpu::rl_r,     Cpu::jr_i8,      Cpu::add_hl_rr, Cpu::ld_a_mrr,  Cpu::dec_rr,    Cpu::inc_r,   Cpu::dec_r,   Cpu::ld_r_u8,   Cpu::rr_r,
    Cpu::jr_cc_i8,  Cpu::ld_rr_u16, Cpu::ld_mhli_a, Cpu::inc_rr,   Cpu::inc_r,    Cpu::dec_r,    Cpu::ld_r_u8,   Cpu::daa,      Cpu::jr_cc_i8,   Cpu::add_hl_rr, Cpu::ld_a_mhli, Cpu::dec_rr,    Cpu::inc_r,   Cpu::dec_r,   Cpu::ld_r_u8,   Cpu::cpl,
    Cpu::jr_cc_i8,  Cpu::ld_rr_u16, Cpu::ld_mhld_a, Cpu::inc_rr,   Cpu::inc_r,    Cpu::dec_r,    Cpu::ld_r_u8,   Cpu::scf,      Cpu::jr_cc_i8,   Cpu::add_hl_rr, Cpu::ld_a_mhld, Cpu::dec_rr,    Cpu::inc_r,   Cpu::dec_r,   Cpu::ld_r_u8,   Cpu::ccf,
    Cpu::ld_r_r,    Cpu::ld_r_r,    Cpu::ld_r_r,    Cpu::ld_r_r,   Cpu::ld_r_r,   Cpu::ld_r_r,   Cpu::ld_r_r,    Cpu::ld_r_r,   Cpu::ld_r_r,     Cpu::ld_r_r,    Cpu::ld_r_r,    Cpu::ld_r_r,    Cpu::ld_r_r,  Cpu::ld_r_r,  Cpu::ld_r_r,    Cpu::ld_r_r,
    Cpu::ld_r_r,    Cpu::ld_r_r,    Cpu::ld_r_r,    Cpu::ld_r_r,   Cpu::ld_r_r,   Cpu::ld_r_r,   Cpu::ld_r_r,    Cpu::ld_r_r,   Cpu::ld_r_r,     Cpu::ld_r_r,    Cpu::ld_r_r,    Cpu::ld_r_r,    Cpu::ld_r_r,  Cpu::ld_r_r,  Cpu::ld_r_r,    Cpu::ld_r_r,
    Cpu::ld_r_r,    Cpu::ld_r_r,    Cpu::ld_r_r,    Cpu::ld_r_r,   Cpu::ld_r_r,   Cpu::ld_r_r,   Cpu::ld_r_r,    Cpu::ld_r_r,   Cpu::ld_r_r,     Cpu::ld_r_r,    Cpu::ld_r_r,    Cpu::ld_r_r,    Cpu::ld_r_r,  Cpu::ld_r_r,  Cpu::ld_r_r,    Cpu::ld_r_r,
    Cpu::ld_r_r,    Cpu::ld_r_r,    Cpu::ld_r_r,    Cpu::ld_r_r,   Cpu::ld_r_r,   Cpu::ld_r_r,   Cpu::halt,      Cpu::ld_r_r,   Cpu::ld_r_r,     Cpu::ld_r_r,    Cpu::ld_r_r,    Cpu::ld_r_r,    Cpu::ld_r_r,  Cpu::ld_r_r,  Cpu::ld_r_r,    Cpu::ld_r_r,
    Cpu::add_a_r,   Cpu::add_a_r,   Cpu::add_a_r,   Cpu::add_a_r,  Cpu::add_a_r,  Cpu::add_a_r,  Cpu::add_a_r,   Cpu::add_a_r,  Cpu::adc_a_r,    Cpu::adc_a_r,   Cpu::adc_a_r,   Cpu::adc_a_r,   Cpu::adc_a_r, Cpu::adc_a_r, Cpu::adc_a_r,   Cpu::adc_a_r,
    Cpu::sub_a_r,   Cpu::sub_a_r,   Cpu::sub_a_r,   Cpu::sub_a_r,  Cpu::sub_a_r,  Cpu::sub_a_r,  Cpu::sub_a_r,   Cpu::sub_a_r,  Cpu::sbc_a_r,    Cpu::sbc_a_r,  Cpu::sbc_a_r,    Cpu::sbc_a_r,   Cpu::sbc_a_r, Cpu::sbc_a_r, Cpu::sbc_a_r,   Cpu::sbc_a_r,
    Cpu::and_a_r,   Cpu::and_a_r,   Cpu::and_a_r,   Cpu::and_a_r,  Cpu::and_a_r,  Cpu::and_a_r,  Cpu::and_a_r,   Cpu::and_a_r,  Cpu::xor_a_r,    Cpu::xor_a_r,  Cpu::xor_a_r,    Cpu::xor_a_r,   Cpu::xor_a_r, Cpu::xor_a_r, Cpu::xor_a_r,   Cpu::xor_a_r,
    Cpu::or_a_r,    Cpu::or_a_r,    Cpu::or_a_r,    Cpu::or_a_r,   Cpu::or_a_r,   Cpu::or_a_r,   Cpu::or_a_r,    Cpu::or_a_r,   Cpu::cp_a_r,     Cpu::cp_a_r,   Cpu::cp_a_r,     Cpu::cp_a_r,    Cpu::cp_a_r,  Cpu::cp_a_r,  Cpu::cp_a_r,    Cpu::cp_a_r,
    Cpu::ret_cc,    Cpu::pop_rr,    Cpu::jp_cc_u16, Cpu::jp_u16,   Cpu::call_cc,  Cpu::push_rr,  Cpu::add_a_u8,  Cpu::rst,      Cpu::ret_cc,     Cpu::ret,      Cpu::jp_cc_u16,  Cpu::prefix_cb, Cpu::call_cc, Cpu::call,    Cpu::adc_a_u8,  Cpu::rst,
    Cpu::ret_cc,    Cpu::pop_rr,    Cpu::jp_cc_u16, Cpu::ill,      Cpu::call_cc,  Cpu::push_rr,  Cpu::sub_a_u8,  Cpu::rst,      Cpu::ret_cc,     Cpu::reti,     Cpu::jp_cc_u16,  Cpu::ill,       Cpu::call_cc, Cpu::ill,     Cpu::sbc_a_u8,  Cpu::rst,
    Cpu::ld_zpu8_a, Cpu::pop_rr,    Cpu::ld_zpc_a,  Cpu::ill,      Cpu::ill,      Cpu::push_rr,  Cpu::and_a_u8,  Cpu::rst,      Cpu::add_sp_i8,  Cpu::jp_hl,    Cpu::ld_mu16_a,  Cpu::ill,       Cpu::ill,     Cpu::ill,     Cpu::xor_a_u8,  Cpu::rst,
    Cpu::ld_a_zpu8, Cpu::pop_rr,    Cpu::ld_a_zpc,  Cpu::di,       Cpu::ill,      Cpu::push_rr,  Cpu::or_a_u8,   Cpu::rst,      Cpu::ld_hl_spi8, Cpu::ld_sp_hl, Cpu::ld_a_mu16,  Cpu::ei,        Cpu::ill,     Cpu::ill,     Cpu::cp_a_u8,   Cpu::rst,
];

#[rustfmt::skip]
const PREFIX_OPCODE_TABLE: [fn(&mut Cpu, u8) -> (); 256] = [
    Cpu::rlc_r,   Cpu::rlc_r,   Cpu::rlc_r,   Cpu::rlc_r,   Cpu::rlc_r,   Cpu::rlc_r,  Cpu::rlc_r,    Cpu::rlc_r,   Cpu::rrc_r,   Cpu::rrc_r,   Cpu::rrc_r,   Cpu::rrc_r,   Cpu::rrc_r,   Cpu::rrc_r,   Cpu::rrc_r,    Cpu::rrc_r,
    Cpu::rl_r,    Cpu::rl_r,    Cpu::rl_r,    Cpu::rl_r,    Cpu::rl_r,    Cpu::rl_r,   Cpu::rl_r,     Cpu::rl_r,    Cpu::rr_r,    Cpu::rr_r,    Cpu::rr_r,    Cpu::rr_r,    Cpu::rr_r,    Cpu::rr_r,    Cpu::rr_r,    Cpu::rr_r,
    Cpu::sla_r,   Cpu::sla_r,   Cpu::sla_r,   Cpu::sla_r,   Cpu::sla_r,   Cpu::sla_r,  Cpu::sla_r,    Cpu::sla_r,   Cpu::sra_r,   Cpu::sra_r,   Cpu::sra_r,   Cpu::sra_r,   Cpu::sra_r,   Cpu::sra_r,   Cpu::sra_r,   Cpu::sra_r,
    Cpu::swap_r,  Cpu::swap_r,  Cpu::swap_r,  Cpu::swap_r,  Cpu::swap_r,  Cpu::swap_r, Cpu::swap_r,   Cpu::swap_r,  Cpu::srl_r,   Cpu::srl_r,   Cpu::srl_r,   Cpu::srl_r,   Cpu::srl_r,   Cpu::srl_r,   Cpu::srl_r,   Cpu::srl_r,
    Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r,
    Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r,
    Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r,
    Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r, Cpu::bit_b_r,
    Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r,
    Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r,
    Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r,
    Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r, Cpu::res_b_r,
    Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r,
    Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r,
    Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r,
    Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r, Cpu::set_b_r,
];

*/

pub struct Cpu {
    registers: Registers,
    mmu: Mmu,
}

impl MemoryOps for Cpu {
    fn read_byte(&mut self, address: u16) -> u8 {
        self.mmu.read_byte(address)
    }

    fn write_byte(&mut self, value: u8, address: u16) {
        self.mmu.write_byte(value, address);
    }
}

impl Cpu {
    pub fn new() -> Self {
        todo!()
    }

    fn tick(&mut self) {}

    fn fetch_byte_pc(&mut self) -> u8 {
        let value = self.read_byte(self.registers.pc);
        self.registers.pc += 1;
        value
    }

    fn fetch_dbyte_pc(&mut self) -> u16 {
        let lower = self.fetch_byte_pc() as u16;
        let upper = (self.fetch_byte_pc() as u16) << 8;
        upper | lower

    }

    fn nop(&mut self) {}

    fn stop(&mut self) {}

    fn halt(&mut self) {}
}

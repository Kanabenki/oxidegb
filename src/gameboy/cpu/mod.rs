mod instructions;
mod registers;

use std::primitive::u16;

use crate::{
    error::Error,
    gameboy::mmu::{MemoryOps, Mmu},
};

use self::registers::{RegisterIndex, Registers};

enum ExecutionState {
    Continue,
    Stop,
    IllegalInstruction,
}

pub struct Cpu {
    pub registers: Registers,
    pub mmu: Mmu,
    cycles_count: u32,
    execution_state: ExecutionState,
}

impl MemoryOps for Cpu {
    fn read_byte(&mut self, address: u16) -> u8 {
        self.cycles_count += 4;
        self.mmu.read_byte(address)
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        self.cycles_count += 4;
        self.mmu.write_byte(address, value);
    }
}

impl Cpu {
    pub fn new(rom: Vec<u8>, bootrom: Option<Vec<u8>>) -> Result<Self, Error> {
        let registers = if bootrom.is_some() {
            Registers::new()
        } else {
            Registers::new_post_bootrom()
        };
        Ok(Self {
            registers,
            mmu: Mmu::new(rom, bootrom)?,
            cycles_count: 0,
            execution_state: ExecutionState::Continue,
        })
    }

    pub fn tick(&mut self) {
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

    fn r(&mut self, index: RegisterIndex) -> u8 {
        if index.value() == 6 {
            self.read_byte(self.registers.hl())
        } else {
            self.registers.r(index)
        }
    }

    fn set_r(&mut self, index: RegisterIndex, value: u8) {
        if index.value() == 6 {
            self.write_byte(self.registers.hl(), value);
        } else {
            self.registers.set_r(index, value);
        }
    }

    fn test_cc(&self, opcode: u8) -> bool {
        match (opcode >> 3) & 0b11 {
            0 => !self.registers.flags.zero(),
            1 => self.registers.flags.zero(),
            2 => !self.registers.flags.carry(),
            3 => self.registers.flags.carry(),
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
}

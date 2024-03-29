mod instructions;
mod registers;

use std::primitive::u16;

use serde::{Deserialize, Serialize};

use self::registers::{RegisterIndex, Registers};
use super::mmu::{MemoryOps, Mmu};
use crate::error::Error;

#[derive(Serialize, Deserialize, Debug)]
pub(crate) enum ExecutionState {
    Continue,
    Stop,
    IllegalInstruction,
    Halt,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Cpu {
    pub(crate) registers: Registers,
    enable_ime: bool,
    pub(crate) mmu: Mmu,
    pub(crate) cycles: u64,
    pub(crate) execution_state: ExecutionState,
}

impl MemoryOps for Cpu {
    fn read_byte(&mut self, address: u16) -> u8 {
        self.tick();
        self.mmu.read_byte(address)
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        self.tick();
        self.mmu.write_byte(address, value);
    }
}

impl Cpu {
    pub(crate) const MAX_TICKS_PER_INSTR: usize = 6 * 4;

    pub(crate) fn new(
        rom: Vec<u8>,
        bootrom: Option<Vec<u8>>,
        save: Option<Vec<u8>>,
    ) -> Result<Self, Error> {
        let registers = if bootrom.is_some() {
            Registers::new()
        } else {
            Registers::new_post_bootrom()
        };
        Ok(Self {
            registers,
            enable_ime: false,
            mmu: Mmu::new(rom, bootrom, save)?,
            cycles: 0,
            execution_state: ExecutionState::Continue,
        })
    }

    pub(crate) fn next_instruction(&mut self) -> u64 {
        // TODO Ensure proper behaviour for those.
        match self.execution_state {
            ExecutionState::Continue => {}
            ExecutionState::Halt => {
                if self.mmu.interrupts().is_empty() {
                    self.tick();
                    return self.cycles;
                } else {
                    self.execution_state = ExecutionState::Continue;
                }
            }
            ExecutionState::Stop => {
                if self.mmu.interrupts().is_empty() {
                    self.tick_stopped();
                    return self.cycles;
                } else {
                    self.execution_state = ExecutionState::Continue;
                }
            }
            ExecutionState::IllegalInstruction => {
                self.tick();
                return self.cycles;
            }
        }

        if self.registers.ime {
            if let Some(interrupt) = self.mmu.next_interrupt() {
                self.push_stack(self.registers.pc);
                self.registers.pc = interrupt.address();
                self.mmu.reset_interrupt(interrupt);
                self.registers.ime = false;

                return self.cycles;
            }
        }

        if self.enable_ime {
            self.enable_ime = false;
            self.registers.ime = true;
        }

        let opcode = self.fetch_byte_pc();

        Self::OPCODE_TABLE[opcode as usize](self, opcode);

        self.cycles
    }

    fn tick(&mut self) {
        self.cycles += 4;
        self.mmu.tick();
    }

    fn tick_stopped(&mut self) {
        self.cycles += 4;
        self.mmu.tick_stopped();
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

    fn set_pc_tick(&mut self, address: u16) {
        self.tick();
        self.registers.pc = address;
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

    fn test_cc(&mut self, opcode: u8) -> bool {
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
        self.registers.sp = self.registers.sp.wrapping_add(2);
        value
    }

    fn push_stack(&mut self, value: u16) {
        self.registers.sp = self.registers.sp.wrapping_sub(2);
        self.write_dbyte(self.registers.sp, value);
    }
}

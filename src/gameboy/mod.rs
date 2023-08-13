mod apu;
mod cartridge;
mod cpu;
mod interrupts;
mod io;
mod mmu;
mod ppu;

use std::io::Write;

use clap::Parser;
use serde::{Deserialize, Serialize};

use crate::{error::Error, gameboy::mmu::MemoryOps};
use cpu::Cpu;

pub use io::Button;

use self::cartridge::{MapperOps, SaveData};

#[derive(Serialize, Deserialize)]
struct DebugStatus {
    breakpoints: Vec<u16>,
    should_break: bool,
}
#[derive(Serialize, Deserialize)]
pub struct Gameboy {
    cpu: Cpu,
    debug_status: DebugStatus,
}

impl Gameboy {
    pub const CYCLES_PER_SECOND: u64 = 4_194_304;

    pub fn new(
        rom: Vec<u8>,
        bootrom: Option<Vec<u8>>,
        save_data: Option<Vec<u8>>,
        debug: bool,
    ) -> Result<Self, Error> {
        let cpu = Cpu::new(rom, bootrom, save_data)?;
        let debug_status = DebugStatus {
            breakpoints: vec![],
            should_break: debug,
        };
        Ok(Self { cpu, debug_status })
    }

    pub fn run_instruction(&mut self) -> u64 {
        self.run_debugger();
        let cycles_start = self.cpu.cycles;
        let cycles_end = self.cpu.next_instruction();
        cycles_end - cycles_start
    }

    pub const fn screen(&self) -> &[ppu::Color; 160 * 144] {
        self.cpu.mmu.ppu.screen()
    }

    pub fn samples(&mut self) -> ([i16; 6], [i16; 6], usize) {
        self.cpu.mmu.apu.samples()
    }

    pub fn can_save(&self) -> bool {
        self.cpu.mmu.cartridge.mapper.has_battery()
    }

    pub fn save_data(&self) -> SaveData {
        self.cpu.mmu.cartridge.save_data()
    }

    pub fn reinit(&mut self, mut gameboy: Self) -> Result<(), Error> {
        if gameboy.cpu.mmu.cartridge.bootrom_enabled && self.cpu.mmu.cartridge.bootrom.is_none() {
            return Err(Error::MissingBootrom);
        }
        std::mem::swap(
            &mut self.cpu.mmu.cartridge.rom,
            &mut gameboy.cpu.mmu.cartridge.rom,
        );
        std::mem::swap(
            &mut self.cpu.mmu.cartridge.bootrom,
            &mut gameboy.cpu.mmu.cartridge.bootrom,
        );
        std::mem::swap(self, &mut gameboy);
        Ok(())
    }

    pub const fn rom_header(&self) -> &cartridge::Header {
        &self.cpu.mmu.cartridge.header
    }

    pub const fn mapper(&self) -> &cartridge::Mapper {
        &self.cpu.mmu.cartridge.mapper
    }

    pub fn set_button(&mut self, button: Button, set: bool) {
        self.cpu.mmu.io.buttons.set_button(button, set);
    }

    pub fn debug_break(&mut self) {
        self.debug_status.should_break = true;
    }

    // TODO: Probably move to Emulator
    fn run_debugger(&mut self) {
        if !self
            .debug_status
            .breakpoints
            .contains(&self.cpu.registers.pc)
            && !self.debug_status.should_break
        {
            return;
        }
        self.debug_status.should_break = false;
        let mut buf = String::new();
        loop {
            let pc = self.cpu.registers.pc;
            println!(
                "Breaked on 0x{pc:04X} (op 0x{:02X})",
                self.cpu.mmu.read_byte(pc)
            );
            buf.clear();
            print!("> ");
            let _ = std::io::stdout().flush();
            std::io::stdin().read_line(&mut buf).unwrap();
            let Some(line) = buf.lines().next() else {
                continue;
            };
            let arg = match DebugCommand::try_parse_from(line.split_whitespace()) {
                Ok(arg) => arg,
                Err(err) => {
                    println!("{err}");
                    continue;
                }
            };
            match arg {
                DebugCommand::Breakpoint { address } => {
                    if !self.debug_status.breakpoints.contains(&address) {
                        self.debug_status.breakpoints.push(address);
                    }
                }
                DebugCommand::Delete { address } => {
                    self.debug_status.breakpoints.retain(|&a| a != address);
                }
                DebugCommand::List => {
                    println!(
                        "{}",
                        if self.debug_status.breakpoints.is_empty() {
                            "No breakpoints"
                        } else {
                            "Current breakpoints:"
                        }
                    );
                    for breakpoint in &self.debug_status.breakpoints {
                        println!("0x{breakpoint:04X}");
                    }
                }
                DebugCommand::Read { address } => {
                    let value = self.cpu.mmu.read_byte(address);
                    println!("(0x{address:04X}) = 0x{value:02X}");
                }
                DebugCommand::Registers => {
                    // TODO Better debug print
                    println!(
                        "{:X?}\nIE: {:?}\nIF: {:?}\n State: {:?}",
                        self.cpu.registers,
                        self.cpu.mmu.interrupt_enable,
                        self.cpu.mmu.interrupt_flags,
                        self.cpu.execution_state,
                    );
                }
                DebugCommand::Step => {
                    self.cpu.next_instruction();
                }
                DebugCommand::Continue => break,
            };
        }
    }
}

fn parse_address(address: &str) -> Result<u16, &'static str> {
    let (address, radix) = if let Some(s_address) = address.strip_prefix("0x") {
        (s_address, 16)
    } else {
        (address, 10)
    };
    u16::from_str_radix(address, radix).map_err(|_| "Invalid address")
}

#[derive(Parser, Clone)]
#[command(multicall = true)]
enum DebugCommand {
    /// Set a breakpoint to an address
    #[command(visible_alias = "b", arg_required_else_help = true)]
    Breakpoint {
        /// The address to break on, either in decimal or in hexadecimal prefixed by "0x"
        #[arg(value_parser=parse_address)]
        address: u16,
    },
    /// Delete a breakpoint at an address
    #[command(visible_alias = "d", arg_required_else_help = true)]
    Delete {
        /// The address for which the breakpoint must be deleted
        #[arg(value_parser=parse_address)]
        address: u16,
    },
    Read {
        /// The address to read from, either in decimal or in hexadecimal prefixed by "0x"
        #[arg(value_parser=parse_address)]
        address: u16,
    },
    /// List all breakpoints
    #[command(visible_alias = "l")]
    List,
    /// Display the current registers state
    #[command(visible_alias = "r")]
    Registers,
    /// Step one instruction
    #[command(visible_alias = "s")]
    Step,
    /// Resume execution
    #[command(visible_alias = "c")]
    Continue,
}

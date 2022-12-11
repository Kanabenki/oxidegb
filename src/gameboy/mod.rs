mod cartridge;
mod cpu;
mod interrupts;
mod io;
mod mmu;
mod ppu;

use std::io::Write;

use crate::{error::Error, gameboy::mmu::MemoryOps};
use cpu::Cpu;

pub use io::Button;

struct DebugStatus {
    breakpoints: Vec<u16>,
    should_break: bool,
}

pub struct Gameboy {
    cpu: Cpu,
    debug_status: DebugStatus,
}

impl Gameboy {
    const TICKS_PER_FRAME: u32 = 70_224;

    pub fn new(rom: Vec<u8>, bootrom: Option<Vec<u8>>, debug: bool) -> Result<Self, Error> {
        let cpu = Cpu::new(rom, bootrom)?;
        let debug_status = DebugStatus {
            breakpoints: vec![],
            should_break: debug,
        };
        Ok(Self { cpu, debug_status })
    }

    pub fn run_frame(&mut self, _delta: u32) -> u32 {
        loop {
            self.run_debugger();
            let cycles_elapsed = self.cpu.next_instruction();
            if cycles_elapsed >= Self::TICKS_PER_FRAME {
                self.cpu.reset_cycles();
                return cycles_elapsed - Self::TICKS_PER_FRAME;
            }
        }
    }

    pub const fn screen(&self) -> &[ppu::Color; 160 * 144] {
        self.cpu.mmu.ppu.screen()
    }

    pub const fn rom_header(&self) -> &cartridge::Header {
        self.cpu.mmu.cartridge.header()
    }

    pub fn set_button(&mut self, button: Button, set: bool) {
        self.cpu.mmu.io.buttons.set_button(button, set);
    }

    pub fn debug_break(&mut self) {
        self.debug_status.should_break = true;
    }

    fn run_debugger(&mut self) {
        if !self
            .debug_status
            .breakpoints
            .contains(&self.cpu.registers().pc)
            && !self.debug_status.should_break
        {
            return;
        }
        self.debug_status.should_break = false;
        let mut buf = String::new();
        loop {
            let pc = self.cpu.registers().pc;
            println!(
                "Breaked on 0x{pc:04X} (op 0x{:02X})",
                self.cpu.mmu.read_byte(pc)
            );
            buf.clear();
            print!("> ");
            let _ = std::io::stdout().flush();
            std::io::stdin().read_line(&mut buf).unwrap();
            let mut words = buf.as_str().split_whitespace();
            match (words.next(), words.next()) {
                (Some("b" | "breakpoint"), Some(address)) => {
                    let (address, radix) = if let Some(s_address) = address.strip_prefix("0x") {
                        (s_address, 16)
                    } else {
                        (address, 10)
                    };
                    let Ok(address) = u16::from_str_radix(address, radix) else {
                                println!("Invalid address \"{address}\"");
                                continue;
                            };
                    if !self.debug_status.breakpoints.contains(&address) {
                        self.debug_status.breakpoints.push(address);
                    }
                }
                (Some("d" | "delete"), Some(address)) => {
                    let (address, radix) = if let Some(s_address) = address.strip_prefix("0x") {
                        (s_address, 16)
                    } else {
                        (address, 10)
                    };
                    let Ok(address) = u16::from_str_radix(address, radix) else {
                                println!("Invalid address \"{address}\"");
                                continue;
                            };
                    self.debug_status.breakpoints.retain(|&a| a != address);
                }
                (Some("l" | "list"), _) => {
                    println!(
                        "{}",
                        if self.debug_status.breakpoints.is_empty() {
                            "Current breakpoints:"
                        } else {
                            "No breakpoints"
                        }
                    );
                    for breakpoint in &self.debug_status.breakpoints {
                        println!("0x{breakpoint:04X}");
                    }
                }
                (Some("r" | "registers"), _) => {
                    // TODO Better debug print
                    println!(
                        "{:X?}\nIE: {:?}\nIF: {:?}",
                        self.cpu.registers(),
                        self.cpu.mmu.interrupt_enable(),
                        self.cpu.mmu.interrupt_flags()
                    );
                }
                (Some("s" | "step"), _) => {
                    self.cpu.next_instruction();
                }
                (Some("c" | "continue"), _) => break,
                (None, None) => continue,
                (Some(command), _) => println!("Unknown command \"{command}\""),
                (None, Some(_)) => unreachable!(),
            };
        }
    }
}

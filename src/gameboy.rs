use crate::cpu::Cpu;

pub struct Gameboy {
    cpu: Cpu,
}

impl Gameboy {
    pub fn new() -> Self {
        Self { cpu: Cpu::new() }
    }
}

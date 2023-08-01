use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub enum Mode {
    HBlank = 0,
    VBlank = 1,
    OamSearch = 2,
    PixelTransfer = 3,
}

impl Default for Mode {
    fn default() -> Self {
        Self::HBlank
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Copy, Clone)]
pub struct LcdStatus {
    coincidence_interrupt_enabled: bool,
    oam_interrupt_enabled: bool,
    vblank_interrupt_enabled: bool,
    hblank_interrupt_enabled: bool,
    pub lyc_coincidence: bool,
    pub mode: Mode,
}

impl LcdStatus {
    pub fn new() -> Self {
        Self {
            mode: Mode::OamSearch,
            ..Default::default()
        }
    }

    pub const fn value(&self) -> u8 {
        (1 << 7)
            | (self.coincidence_interrupt_enabled as u8) << 6
            | (self.oam_interrupt_enabled as u8) << 5
            | (self.vblank_interrupt_enabled as u8) << 4
            | (self.hblank_interrupt_enabled as u8) << 3
            | self.mode as u8
    }

    pub fn set_value(&mut self, value: u8) {
        self.coincidence_interrupt_enabled = value & (1 << 6) != 0;
        self.oam_interrupt_enabled = value & (1 << 5) != 0;
        self.vblank_interrupt_enabled = value & (1 << 4) != 0;
        self.hblank_interrupt_enabled = value & (1 << 3) != 0;
    }

    pub const fn coincidence_interrupt_enabled(&self) -> bool {
        self.coincidence_interrupt_enabled
    }

    pub const fn oam_interrupt_enabled(&self) -> bool {
        self.oam_interrupt_enabled
    }

    pub const fn vblank_interrupt_enabled(&self) -> bool {
        self.vblank_interrupt_enabled
    }

    pub const fn hblank_interrupt_enabled(&self) -> bool {
        self.hblank_interrupt_enabled
    }
}

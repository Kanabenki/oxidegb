use flagset::flags;

flags! {
    pub(crate) enum Interrupt: u8 {
        VBlank  = 0b00001,
        LcdStat = 0b00010,
        Timer   = 0b00100,
        Serial  = 0b01000,
        Joypad  = 0b10000
    }
}

impl Interrupt {
    const VBLANK_ADDRESS: u16 = 0x40;
    const LCD_STAT_ADDRESS: u16 = 0x48;
    const TIMER_ADDRESS: u16 = 0x50;
    const SERIAL_ADDRESS: u16 = 0x58;
    const JOYPAD_ADDRESS: u16 = 0x60;

    pub(crate) const fn address(&self) -> u16 {
        match self {
            Self::VBlank => Self::VBLANK_ADDRESS,
            Self::LcdStat => Self::LCD_STAT_ADDRESS,
            Self::Timer => Self::TIMER_ADDRESS,
            Self::Serial => Self::SERIAL_ADDRESS,
            Self::Joypad => Self::JOYPAD_ADDRESS,
        }
    }
}

use flagset::{flags, FlagSet};

flags! {
    enum Interrupt: u8 {
        VBlank = 0b0000,
        LcdStat = 0b0010,
        Serial = 0b0100,
        Joypad = 0b1000
    }
}

// TODO: Remove when everything is defined
#![allow(dead_code)]

enum ChannelIdx {
    C1,
    C2,
    C3,
    C4,
}

#[derive(Debug, Default)]
struct Channels<T> {
    channel_1: T,
    channel_2: T,
    channel_3: T,
    channel_4: T,
}

impl Channels<bool> {
    fn value(&self) -> u8 {
        self.channel_1 as u8
            | (self.channel_2 as u8) << 1
            | (self.channel_3 as u8) << 2
            | (self.channel_4 as u8) << 3
    }

    fn from_nibble(nibble: u8) -> Self {
        let channel_1 = nibble & 0b0001 != 0;
        let channel_2 = nibble & 0b0010 != 0;
        let channel_3 = nibble & 0b0100 != 0;
        let channel_4 = nibble & 0b1000 != 0;
        Self {
            channel_1,
            channel_2,
            channel_3,
            channel_4,
        }
    }
}

#[derive(Debug, Default)]
struct SoundEnable {
    channels: Channels<bool>,
    all: bool,
}

impl SoundEnable {
    fn value(&self) -> u8 {
        // TODO find out unused bits behaviour
        (self.all as u8) << 7 | self.channels.value()
    }
}

#[derive(Debug, Default)]
struct SoundPanning {
    left: Channels<bool>,
    right: Channels<bool>,
}

impl SoundPanning {
    fn value(&self) -> u8 {
        self.left.value() << 4 | self.right.value()
    }

    fn set_value(&mut self, value: u8) {
        self.left = Channels::from_nibble(value >> 4);
        self.right = Channels::from_nibble(value & 0b1111);
    }
}

#[derive(Debug, Default)]
struct MasterVolVinPan {
    left_volume: u8,
    right_volume: u8,
    mix_vin_left: bool,
    mix_vin_right: bool,
}
impl MasterVolVinPan {
    fn value(&self) -> u8 {
        (self.mix_vin_left as u8) << 7
            | (self.left_volume & 0b111) << 4
            | (self.mix_vin_right as u8) << 3
            | self.right_volume & 0b111
    }

    fn set_value(&mut self, value: u8) {
        self.mix_vin_left = value & 0b1000_0000 != 0;
        self.left_volume = (value >> 4) & 0b111;
        self.mix_vin_right = value & 0b1000 != 0;
        self.right_volume = value & 0b111;
    }
}

enum SweepOp {
    Increase = 0,
    Decrease = 1,
}

struct Sweep {
    pace: u8,
    op: SweepOp,
    slope_ctrl: u8,
}

struct LenDutyCycle {
    wave_duty: u8,
    init_len_timer: u8,
}

enum EnvelopeDir {
    Decrease = 0,
    Increase = 1,
}

struct VolumeEnvelope {
    initial: u8,
    direction: EnvelopeDir,
    sweep_pace: u8,
}

#[derive(Debug, Default)]
struct WavelenCtrl {
    trigger: bool,
    sound_len_enable: bool,
    wavelen_high: u8,
}

struct Channel1 {
    sweep: Sweep,
    len_duty_cycle: LenDutyCycle,
    vol_env: VolumeEnvelope,
    wavelen_low: u8,
    wavelen_high_ctrl: WavelenCtrl,
}

struct Channel2 {
    len_duty_cycle: LenDutyCycle,
    vol_env: VolumeEnvelope,
    wavelen_low: u8,
    wavelen_high_ctrl: WavelenCtrl,
}

struct Channel3 {
    enable: bool,
    len_timer: u8,
    out_level: u8,
    wavelen_low: u8,
}

#[derive(Debug, Default)]
pub struct Apu {
    master_vol_vin_pan: MasterVolVinPan,
    sound_panning: SoundPanning,
    sound_enable: SoundEnable,
}

impl Apu {
    const MASTER_VOL_VIN_PAN_ADDRESS: u16 = 0xFF24;
    const SOUND_PANNING_ADDRESS: u16 = 0xFF25;
    const SOUND_ENABLE_ADDRESS: u16 = 0xFF26;

    pub fn new() -> Self {
        Default::default()
    }

    pub fn read(&self, address: u16) -> u8 {
        match address {
            Self::MASTER_VOL_VIN_PAN_ADDRESS => self.master_vol_vin_pan.value(),
            Self::SOUND_PANNING_ADDRESS => self.sound_panning.value(),
            Self::SOUND_ENABLE_ADDRESS => self.sound_enable.value(),
            _ => unreachable!("Tried to read invalid address {address:04X} in apu"),
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            Self::MASTER_VOL_VIN_PAN_ADDRESS => self.master_vol_vin_pan.set_value(value),
            Self::SOUND_PANNING_ADDRESS => self.sound_panning.set_value(value),
            Self::SOUND_ENABLE_ADDRESS => self.sound_enable.all = value & 0b1000_0000 != 0,
            _ => unreachable!("Tried to write invalid address {address:04X} in apu"),
        }
    }
}

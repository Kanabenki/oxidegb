use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
struct ChannelToggles {
    channel_1: bool,
    channel_2: bool,
    channel_3: bool,
    channel_4: bool,
}

impl ChannelToggles {
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

#[derive(Serialize, Deserialize, Debug, Default)]
struct SoundEnable {
    channels: ChannelToggles,
    all: bool,
}

impl SoundEnable {
    fn value(&self) -> u8 {
        // TODO find out unused bits behaviour
        (self.all as u8) << 7 | self.channels.value()
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct SoundPanning {
    left: ChannelToggles,
    right: ChannelToggles,
}

impl SoundPanning {
    fn value(&self) -> u8 {
        self.left.value() << 4 | self.right.value()
    }

    fn set_value(&mut self, value: u8) {
        self.left = ChannelToggles::from_nibble(value >> 4);
        self.right = ChannelToggles::from_nibble(value & 0b1111);
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
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

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Default)]
enum SweepOp {
    #[default]
    Increase = 0,
    Decrease = 1,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct Sweep {
    pace: u8,
    op: SweepOp,
    slope_ctrl: u8,
}
impl Sweep {
    fn value(&self) -> u8 {
        self.pace & 0b111 << 4 | (self.op as u8) << 3 | self.slope_ctrl & 0b111
    }

    fn set_value(&mut self, value: u8) {
        self.pace = (value >> 4) & 0b111;
        self.op = if value & 0b1000 == 0 {
            SweepOp::Increase
        } else {
            SweepOp::Decrease
        };
        self.slope_ctrl = value & 0b111;
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default)]
enum WaveDuty {
    #[default]
    W0 = 0,
    W1 = 1,
    W2 = 2,
    W3 = 3,
}

type _Waveform = [bool; 8];

impl WaveDuty {
    const _WAVEFORMS: [_Waveform; 4] = [
        [true, true, true, true, true, true, true, false],
        [false, true, true, true, true, true, true, false],
        [false, true, true, true, true, false, false, false],
        [true, false, false, false, false, false, false, true],
    ];

    const fn _waveform(&self) -> &_Waveform {
        &Self::_WAVEFORMS[*self as usize]
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct WaveDutyTimerLen {
    wave_duty: WaveDuty,
    init_len_timer: u8,
}

impl WaveDutyTimerLen {
    fn value(&self) -> u8 {
        // TODO: check low bits value
        (self.wave_duty as u8) << 6
    }

    fn set_value(&mut self, value: u8) {
        self.wave_duty = match value >> 6 {
            0 => WaveDuty::W0,
            1 => WaveDuty::W1,
            2 => WaveDuty::W2,
            3 => WaveDuty::W3,
            _ => unreachable!(),
        };

        self.init_len_timer = value & 0b11111;
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default)]
enum EnvelopeDir {
    #[default]
    Decrease = 0,
    Increase = 1,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct VolumeEnvelope {
    initial: u8,
    direction: EnvelopeDir,
    sweep_pace: u8,
}
impl VolumeEnvelope {
    fn value(&self) -> u8 {
        self.initial << 4 | (self.direction as u8) << 3 | self.sweep_pace & 0b111
    }

    fn set_value(&mut self, value: u8) {
        self.initial = value >> 4;
        self.direction = if value & 0b1000 == 0 {
            EnvelopeDir::Decrease
        } else {
            EnvelopeDir::Increase
        };
        self.sweep_pace = value & 0b111;
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct WavelenCtrl {
    trigger: bool,
    sound_len_enable: bool,
    wavelen: u16,
}
impl WavelenCtrl {
    fn value_ctrl(&self) -> u8 {
        // TODO check unused/write only bits read value
        (self.sound_len_enable as u8) << 6
    }

    fn set_value_wavelen_h_ctrl(&mut self, value: u8) {
        self.trigger = value & 0b1000_0000 != 0;
        self.sound_len_enable = value & 0b100_0000 != 0;
        self.wavelen = (self.wavelen & 0x00FF) | ((value as u16 & 0b111) << 8);
    }

    fn set_value_wavelen_l(&mut self, value: u8) {
        self.wavelen = (self.wavelen & 0xFF00) | value as u16;
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct Channel1 {
    sweep: Sweep,
    wave_duty_timer_len: WaveDutyTimerLen,
    vol_env: VolumeEnvelope,
    wavelen_ctrl: WavelenCtrl,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct Channel2 {
    wave_duty_timer_len: WaveDutyTimerLen,
    vol_env: VolumeEnvelope,
    wavelen_ctrl: WavelenCtrl,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct Channel3 {
    enable: bool,
    cycles: u32,
    len_timer: u8,
    out_level: u8,
    wavelen_ctrl: WavelenCtrl,
    wave_pattern: [u8; 16],
    wave_pattern_index: usize,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy)]
enum LfsrWidth {
    #[default]
    B15 = 0,
    B7 = 1,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct FreqRand {
    clock_shift: u8,
    lfsr_width: LfsrWidth,
    clock_divider: u8,
}
impl FreqRand {
    fn value(&self) -> u8 {
        (self.clock_shift & 0b1111) << 4 | (self.lfsr_width as u8) << 3 | self.clock_divider & 0b111
    }

    fn set_value(&mut self, value: u8) {
        self.clock_shift = value >> 4;
        self.lfsr_width = if value & 0b1000 == 0 {
            LfsrWidth::B15
        } else {
            LfsrWidth::B7
        };
        self.clock_divider = value & 0b111;
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct Channel4 {
    len_timer: u8,
    vol_env: VolumeEnvelope,
    freq_rand: FreqRand,
    trigger: bool,
    sound_len_enable: bool,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub(crate) struct Apu {
    left_deltas: [i32; 6],
    right_deltas: [i32; 6],
    delta_offsets: [usize; 6],
    delta_offset: usize,
    delta_count: usize,
    master_vol_vin_pan: MasterVolVinPan,
    sound_panning: SoundPanning,
    sound_enable: SoundEnable,
    channel_1: Channel1,
    channel_2: Channel2,
    channel_3: Channel3,
    channel_4: Channel4,
    div: u8,
    amplitude: i32,
}

impl Apu {
    const CH1_SWEEP_ADDRESS: u16 = 0xFF10;
    const CH1_WAVE_DUTY_TIMER_LEN_ADDRESS: u16 = 0xFF11;
    const CH1_VOLUME_ENVELOPPE_ADDRESS: u16 = 0xFF12;
    const CH1_WAVELEN_LOW_ADDRESS: u16 = 0xFF13;
    const CH1_WAVELEN_HIGH_CTRL_ADDRESS: u16 = 0xFF14;

    const CH2_UNUSED_ADDRESS: u16 = 0xFF15;
    const CH2_WAVE_DUTY_TIMER_LEN_ADDRESS: u16 = 0xFF16;
    const CH2_VOLUME_ENVELOPPE_ADDRESS: u16 = 0xFF17;
    const CH2_WAVELEN_LOW_ADDRESS: u16 = 0xFF18;
    const CH2_WAVELEN_HIGH_CTRL_ADDRESS: u16 = 0xFF19;

    const CH4_LEN_TIMER_ADDRESS: u16 = 0xFF20;
    const CH4_VOLUME_ENVELOPPE_ADDRESS: u16 = 0xFF21;
    const CH4_FREQ_RAND_ADDRESS: u16 = 0xFF22;
    const CH4_CTRL_ADDRESS: u16 = 0xFF23;

    const CH3_ENABLE_ADDRESS: u16 = 0xFF1A;
    const CH3_LEN_TIMER_ADDRESS: u16 = 0xFF1B;
    const CH3_OUT_LEVEL_ADDRESS: u16 = 0xFF1C;
    const CH3_WAVELEN_LOW_ADDRESS: u16 = 0xFF1D;
    const CH3_WAVELEN_HIGH_CTRL_ADDRESS: u16 = 0xFF1E;

    const MASTER_VOL_VIN_PAN_ADDRESS: u16 = 0xFF24;
    const SOUND_PANNING_ADDRESS: u16 = 0xFF25;
    const SOUND_ENABLE_ADDRESS: u16 = 0xFF26;

    const CH3_WAVE_PATTERN_START_ADDRESS: u16 = 0xFF30;
    const CH3_WAVE_PATTERN_END_ADDRESS: u16 = 0xFF3F;

    pub(crate) fn new() -> Self {
        Default::default()
    }

    pub(crate) fn inc_div(&mut self) {
        self.div = self.div.wrapping_add(1);
        if self.div & 0b1 == 0 {
            // Tick sound length
        }
        if self.div & 0b11 == 0 {
            // Tick channel 1 frequency sweep
        }
        if self.div & 0b111 == 0 {
            // Tick enveloppe sweep
        }
    }

    pub(crate) fn tick(&mut self) {
        if self.delta_offset >= 6 * 4 {
            // Samples were not fetched, we skip them.
            self.delta_count = 0;
            self.delta_offset = 0;
        }

        for _ in 0..2 {
            let amplitude = if self.channel_3.enable {
                self.channel_3.cycles += 1;
                if self.channel_3.cycles >= self.channel_3.wavelen_ctrl.wavelen as u32 {
                    self.channel_3.cycles = 0;
                    self.channel_3.wave_pattern_index =
                        (self.channel_3.wave_pattern_index + 1) % 32;
                }

                let index = self.channel_3.wave_pattern_index / 2;
                let high_nibble = (self.channel_3.wave_pattern_index % 2) == 0;
                let sample = if high_nibble {
                    self.channel_3.wave_pattern[index] >> 4
                } else {
                    self.channel_3.wave_pattern[index] & 0b1111
                };
                let sample = if self.channel_3.out_level == 0 {
                    0
                } else {
                    sample >> (self.channel_3.out_level - 1)
                };
                // TODO: Find proper remapping.
                (sample as i32 - 8) * (i32::MAX / 16)
            } else {
                0
            };

            let delta = amplitude - self.amplitude;
            self.amplitude = amplitude;

            if delta != 0 {
                self.left_deltas[self.delta_count] = delta;
                self.right_deltas[self.delta_count] = delta;
                self.delta_offsets[self.delta_count] = self.delta_offset;
                self.delta_count += 1;
            }
            self.delta_offset += 2;
        }
    }

    pub(crate) fn deltas(&mut self) -> (&[i32], &[i32], &[usize]) {
        let count = self.delta_count;
        self.delta_count = 0;
        self.delta_offset = 0;
        (
            &self.left_deltas[0..count],
            &self.right_deltas[0..count],
            &self.delta_offsets[0..count],
        )
    }

    pub(crate) fn read(&self, address: u16) -> u8 {
        match address {
            Self::CH1_SWEEP_ADDRESS => self.channel_1.sweep.value(),
            Self::CH1_WAVE_DUTY_TIMER_LEN_ADDRESS => self.channel_1.wave_duty_timer_len.value(),
            Self::CH1_VOLUME_ENVELOPPE_ADDRESS => self.channel_1.vol_env.value(),
            Self::CH1_WAVELEN_LOW_ADDRESS => 0xFF,
            Self::CH1_WAVELEN_HIGH_CTRL_ADDRESS => self.channel_1.wavelen_ctrl.value_ctrl(),

            Self::CH2_UNUSED_ADDRESS => 0xFF,
            Self::CH2_WAVE_DUTY_TIMER_LEN_ADDRESS => self.channel_2.wave_duty_timer_len.value(),
            Self::CH2_VOLUME_ENVELOPPE_ADDRESS => self.channel_2.vol_env.value(),
            Self::CH2_WAVELEN_LOW_ADDRESS => 0xFF,
            Self::CH2_WAVELEN_HIGH_CTRL_ADDRESS => self.channel_2.wavelen_ctrl.value_ctrl(),

            Self::CH4_LEN_TIMER_ADDRESS => self.channel_4.len_timer & 0b11_1111,
            Self::CH4_VOLUME_ENVELOPPE_ADDRESS => self.channel_4.vol_env.value(),
            Self::CH4_FREQ_RAND_ADDRESS => self.channel_4.freq_rand.value(),
            Self::CH4_CTRL_ADDRESS => (self.channel_4.sound_len_enable as u8) << 6,

            Self::CH3_ENABLE_ADDRESS => (self.channel_3.enable as u8) << 7,
            Self::CH3_LEN_TIMER_ADDRESS => 0xFF,
            Self::CH3_OUT_LEVEL_ADDRESS => self.channel_3.out_level,
            Self::CH3_WAVELEN_LOW_ADDRESS => 0xFF,
            Self::CH3_WAVELEN_HIGH_CTRL_ADDRESS => self.channel_3.wavelen_ctrl.value_ctrl(),
            Self::CH3_WAVE_PATTERN_START_ADDRESS..=Self::CH3_WAVE_PATTERN_END_ADDRESS => {
                self.channel_3.wave_pattern[(address & 0xF) as usize]
            }

            Self::MASTER_VOL_VIN_PAN_ADDRESS => self.master_vol_vin_pan.value(),
            Self::SOUND_PANNING_ADDRESS => self.sound_panning.value(),
            Self::SOUND_ENABLE_ADDRESS => self.sound_enable.value(),
            _ => unreachable!("Tried to read invalid address {address:04X} in apu"),
        }
    }

    pub(crate) fn write(&mut self, address: u16, value: u8) {
        match address {
            Self::CH1_SWEEP_ADDRESS => self.channel_1.sweep.set_value(value),
            Self::CH1_VOLUME_ENVELOPPE_ADDRESS => self.channel_1.vol_env.set_value(value),
            Self::CH1_WAVE_DUTY_TIMER_LEN_ADDRESS => {
                self.channel_1.wave_duty_timer_len.set_value(value)
            }
            Self::CH1_WAVELEN_LOW_ADDRESS => self.channel_1.wavelen_ctrl.set_value_wavelen_l(value),
            Self::CH1_WAVELEN_HIGH_CTRL_ADDRESS => {
                self.channel_1.wavelen_ctrl.set_value_wavelen_h_ctrl(value)
            }

            Self::CH2_UNUSED_ADDRESS => {}
            Self::CH2_VOLUME_ENVELOPPE_ADDRESS => self.channel_2.vol_env.set_value(value),
            Self::CH2_WAVE_DUTY_TIMER_LEN_ADDRESS => {
                self.channel_2.wave_duty_timer_len.set_value(value)
            }
            Self::CH2_WAVELEN_LOW_ADDRESS => self.channel_2.wavelen_ctrl.set_value_wavelen_l(value),
            Self::CH2_WAVELEN_HIGH_CTRL_ADDRESS => {
                self.channel_2.wavelen_ctrl.set_value_wavelen_h_ctrl(value)
            }

            Self::CH4_LEN_TIMER_ADDRESS => self.channel_4.len_timer = value & 0b11_1111,
            Self::CH4_VOLUME_ENVELOPPE_ADDRESS => self.channel_4.vol_env.set_value(value),
            Self::CH4_FREQ_RAND_ADDRESS => self.channel_4.freq_rand.set_value(value),
            Self::CH4_CTRL_ADDRESS => {
                self.channel_4.trigger = value & 0b1000_0000 != 0;
                self.channel_4.sound_len_enable = value & 0b100_0000 != 0;
            }

            Self::CH3_ENABLE_ADDRESS => self.channel_3.enable = value & 0b1000_0000 != 0,
            Self::CH3_LEN_TIMER_ADDRESS => self.channel_3.len_timer = value,
            Self::CH3_OUT_LEVEL_ADDRESS => self.channel_3.out_level = (value & 0b11) << 5,
            Self::CH3_WAVELEN_LOW_ADDRESS => self.channel_3.wavelen_ctrl.set_value_wavelen_l(value),
            Self::CH3_WAVELEN_HIGH_CTRL_ADDRESS => {
                self.channel_3.wavelen_ctrl.set_value_wavelen_h_ctrl(value)
            }
            Self::CH3_WAVE_PATTERN_START_ADDRESS..=Self::CH3_WAVE_PATTERN_END_ADDRESS => {
                self.channel_3.wave_pattern[(address & 0xF) as usize] = value
            }

            Self::MASTER_VOL_VIN_PAN_ADDRESS => self.master_vol_vin_pan.set_value(value),
            Self::SOUND_PANNING_ADDRESS => self.sound_panning.set_value(value),
            Self::SOUND_ENABLE_ADDRESS => self.sound_enable.all = value & 0b1000_0000 != 0,
            _ => unreachable!("Tried to write invalid address {address:04X} in apu"),
        }
    }
}

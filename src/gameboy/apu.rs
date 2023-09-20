use std::ops::SubAssign;

use num::PrimInt;
use serde::{Deserialize, Serialize};

use super::cpu::Cpu;

// TODO: Find write only/unused bit read value

#[derive(Serialize, Deserialize, Debug, Default)]
struct ChannelToggles {
    ch_1: bool,
    ch_2: bool,
    ch_3: bool,
    ch_4: bool,
}

impl ChannelToggles {
    fn value(&self) -> u8 {
        ((self.ch_4 as u8) << 3)
            | ((self.ch_3 as u8) << 2)
            | ((self.ch_2 as u8) << 1)
            | (self.ch_1 as u8)
    }

    fn from_nibble(nibble: u8) -> Self {
        let ch_1 = nibble & 0b0001 != 0;
        let ch_2 = nibble & 0b0010 != 0;
        let ch_3 = nibble & 0b0100 != 0;
        let ch_4 = nibble & 0b1000 != 0;
        Self {
            ch_1,
            ch_2,
            ch_3,
            ch_4,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct SoundEnable {
    channels: ChannelToggles,
    enable_apu: bool,
}

impl SoundEnable {
    fn value(&self) -> u8 {
        (self.enable_apu as u8) << 7 | 0b111_0000 | self.channels.value()
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct SoundPanning {
    left: ChannelToggles,
    right: ChannelToggles,
}

impl SoundPanning {
    fn value(&self) -> u8 {
        self.left.value() << 4 | (self.right.value() & 0b1111)
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
        0b1000_0000 | (self.pace & 0b111) << 4 | (self.op as u8) << 3 | self.slope_ctrl & 0b111
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

type Waveform = [bool; 8];

impl WaveDuty {
    const WAVEFORMS: [Waveform; 4] = [
        [true, true, true, true, true, true, true, false],
        [false, true, true, true, true, true, true, false],
        [false, true, true, true, true, false, false, false],
        [true, false, false, false, false, false, false, true],
    ];

    const fn waveform(&self) -> &Waveform {
        &Self::WAVEFORMS[*self as usize]
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct WaveDutyTimerLen {
    wave_duty: WaveDuty,
    len_timer: u8,
}

impl WaveDutyTimerLen {
    fn value(&self) -> u8 {
        (self.wave_duty as u8) << 6 | 0b11_1111
    }

    fn set_value(&mut self, value: u8) {
        self.wave_duty = match value >> 6 {
            0 => WaveDuty::W0,
            1 => WaveDuty::W1,
            2 => WaveDuty::W2,
            3 => WaveDuty::W3,
            _ => unreachable!(),
        };

        self.len_timer = (!value & 0b11_1111) + 1;
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Default)]
enum EnvelopeDir {
    #[default]
    Decrease = 0,
    Increase = 1,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct VolumeEnvelope {
    initial: u8,
    direction: EnvelopeDir,
    pace: u8,
}

impl VolumeEnvelope {
    fn value(&self) -> u8 {
        self.initial << 4 | (self.direction as u8) << 3 | self.pace & 0b111
    }

    fn set_value(&mut self, value: u8) {
        self.initial = value >> 4;
        self.direction = if value & 0b1000 == 0 {
            EnvelopeDir::Decrease
        } else {
            EnvelopeDir::Increase
        };
        self.pace = value & 0b111;
    }

    fn dac_enabled(&self) -> bool {
        self.initial != 0 || self.direction != EnvelopeDir::Decrease
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct WavelenCtrl {
    trigger: bool,
    len_enable: bool,
    wavelen: u16,
}

impl WavelenCtrl {
    fn value_ctrl(&self) -> u8 {
        (self.len_enable as u8) << 6 | 0b1011_1111
    }

    fn set_value_wavelen_h_ctrl(&mut self, value: u8) {
        self.trigger = value & 0b1000_0000 != 0;
        self.len_enable = value & 0b100_0000 != 0;
        self.wavelen = (self.wavelen & 0x00FF) | ((value as u16 & 0b111) << 8);
    }

    fn set_value_wavelen_l(&mut self, value: u8) {
        self.wavelen = (self.wavelen & 0xFF00) | value as u16;
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct Channel1 {
    cycles: u16,
    wave_idx: usize,
    volume: u8,
    sweep: Sweep,
    wave_duty_len_timer: WaveDutyTimerLen,
    vol_env: VolumeEnvelope,
    wavelen_ctrl: WavelenCtrl,
}

impl Channel1 {
    fn reset(&mut self) {
        *self = Self {
            wave_duty_len_timer: WaveDutyTimerLen {
                wave_duty: WaveDuty::W0,
                len_timer: self.wave_duty_len_timer.len_timer,
            },
            ..Default::default()
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct Channel2 {
    cycles: u16,
    wave_idx: usize,
    volume: u8,
    wave_duty_len_timer: WaveDutyTimerLen,
    vol_env: VolumeEnvelope,
    wavelen_ctrl: WavelenCtrl,
}

impl Channel2 {
    fn reset(&mut self) {
        *self = Self {
            wave_duty_len_timer: WaveDutyTimerLen {
                wave_duty: WaveDuty::W0,
                len_timer: self.wave_duty_len_timer.len_timer,
            },
            ..Default::default()
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct Channel3 {
    dac_enable: bool,
    cycles: u16,
    len_timer: u16,
    out_level: u8,
    wavelen_ctrl: WavelenCtrl,
    wave_pattern: [u8; 16],
    wave_pattern_index: usize,
}

impl Channel3 {
    fn reset(&mut self) {
        *self = Self {
            len_timer: self.len_timer,
            wave_pattern: self.wave_pattern,
            ..Default::default()
        };
    }
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
    clock_div: u8,
}
impl FreqRand {
    fn value(&self) -> u8 {
        (self.clock_shift & 0b1111) << 4 | (self.lfsr_width as u8) << 3 | self.clock_div & 0b111
    }

    fn set_value(&mut self, value: u8) {
        self.clock_shift = value >> 4;
        self.lfsr_width = if value & 0b1000 == 0 {
            LfsrWidth::B15
        } else {
            LfsrWidth::B7
        };
        self.clock_div = value & 0b111;
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct Channel4 {
    cycles: u32,
    len_timer: u8,
    volume: u8,
    vol_env: VolumeEnvelope,
    freq_rand: FreqRand,
    lfsr: u16,
    trigger: bool,
    len_enable: bool,
}

impl Channel4 {
    fn reset(&mut self) {
        *self = Self {
            len_timer: self.len_timer,
            ..Default::default()
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub(crate) struct Apu {
    left_deltas: [i32; 12],
    right_deltas: [i32; 12],
    delta_offsets: [usize; 12],
    delta_offset: usize,
    delta_count: usize,
    master_vol_vin_pan: MasterVolVinPan,
    sound_panning: SoundPanning,
    sound_enable: SoundEnable,
    ch_1: Channel1,
    ch_2: Channel2,
    ch_3: Channel3,
    ch_4: Channel4,
    div: u8,
    amplitude_left: i32,
    amplitude_right: i32,
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

    const CH3_ENABLE_ADDRESS: u16 = 0xFF1A;
    const CH3_LEN_TIMER_ADDRESS: u16 = 0xFF1B;
    const CH3_OUT_LEVEL_ADDRESS: u16 = 0xFF1C;
    const CH3_WAVELEN_LOW_ADDRESS: u16 = 0xFF1D;
    const CH3_WAVELEN_HIGH_CTRL_ADDRESS: u16 = 0xFF1E;

    const CH4_UNUSED_ADDRESS: u16 = 0xFF1F;
    const CH4_LEN_TIMER_ADDRESS: u16 = 0xFF20;
    const CH4_VOLUME_ENVELOPPE_ADDRESS: u16 = 0xFF21;
    const CH4_FREQ_RAND_ADDRESS: u16 = 0xFF22;
    const CH4_CTRL_ADDRESS: u16 = 0xFF23;

    const MASTER_VOL_VIN_PAN_ADDRESS: u16 = 0xFF24;
    const SOUND_PANNING_ADDRESS: u16 = 0xFF25;
    const SOUND_ENABLE_ADDRESS: u16 = 0xFF26;

    const UNUSED_START_ADDRESS: u16 = 0xFF27;
    const UNUSED_END_ADDRESS: u16 = 0xFF2F;

    const CH3_WAVE_PATTERN_START_ADDRESS: u16 = 0xFF30;
    const CH3_WAVE_PATTERN_END_ADDRESS: u16 = 0xFF3F;

    const WAVELEN_MAX: u16 = 1 << 11;

    pub(crate) fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    fn tick_len<N: PrimInt + SubAssign>(len_timer: &mut N, len_enable: bool, ch_enable: &mut bool) {
        if len_enable && *len_timer > N::zero() {
            *len_timer -= N::one();
            if *len_timer == N::zero() {
                *ch_enable = false;
            }
        }
    }

    pub(crate) fn inc_div(&mut self) {
        if !self.sound_enable.enable_apu {
            return;
        }

        // Tick sound length.
        if self.div & 0b1 == 0 {
            Self::tick_len(
                &mut self.ch_1.wave_duty_len_timer.len_timer,
                self.ch_1.wavelen_ctrl.len_enable,
                &mut self.sound_enable.channels.ch_1,
            );
            Self::tick_len(
                &mut self.ch_2.wave_duty_len_timer.len_timer,
                self.ch_2.wavelen_ctrl.len_enable,
                &mut self.sound_enable.channels.ch_2,
            );
            Self::tick_len(
                &mut self.ch_3.len_timer,
                self.ch_3.wavelen_ctrl.len_enable,
                &mut self.sound_enable.channels.ch_3,
            );
            Self::tick_len(
                &mut self.ch_4.len_timer,
                self.ch_4.len_enable,
                &mut self.sound_enable.channels.ch_4,
            );
        }

        // Tick channel 1 frequency sweep.
        if self.div & 0b11 == 0 {
            let pace = if self.ch_1.sweep.pace != 0 {
                self.ch_1.sweep.pace
            } else {
                8
            };
            if self.ch_1.wavelen_ctrl.wavelen > 0 && (self.div >> 2) % pace == 0 {
                let offset =
                    self.ch_1.wavelen_ctrl.wavelen / 2u16.pow(self.ch_1.sweep.slope_ctrl as u32);
                match self.ch_1.sweep.op {
                    SweepOp::Increase => {
                        if self.ch_1.wavelen_ctrl.wavelen + offset >= Self::WAVELEN_MAX {
                            self.sound_enable.channels.ch_1 = false;
                        } else {
                            self.ch_1.wavelen_ctrl.wavelen += offset;
                        }
                    }
                    SweepOp::Decrease => {
                        if offset > self.ch_1.wavelen_ctrl.wavelen {
                            self.sound_enable.channels.ch_1 = false;
                        } else {
                            self.ch_1.wavelen_ctrl.wavelen -= offset;
                        }
                    }
                }
            }
        }

        // Tick enveloppe sweep.
        if self.div & 0b111 == 0 {
            let tick_enveloppe = |vol_env: &VolumeEnvelope, volume: &mut u8| {
                if vol_env.pace != 0 && (self.div % vol_env.pace) == 0 {
                    match vol_env.direction {
                        EnvelopeDir::Increase => *volume = u8::max(*volume + 1, 0b1111),
                        EnvelopeDir::Decrease => {
                            if *volume > 0 {
                                *volume -= 1;
                            }
                        }
                    }
                }
            };
            tick_enveloppe(&self.ch_1.vol_env, &mut self.ch_1.volume);
            tick_enveloppe(&self.ch_2.vol_env, &mut self.ch_2.volume);
            tick_enveloppe(&self.ch_4.vol_env, &mut self.ch_4.volume);
        }

        self.div = self.div.wrapping_add(1);
    }

    pub(crate) fn tick(&mut self) {
        // TODO: Refactor once all channels work properly.
        if self.delta_offset >= Cpu::MAX_TICKS_PER_INSTR * 2 {
            // Samples were not fetched, we skip them.
            self.delta_count = 0;
            self.delta_offset = 0;
        }

        if !self.sound_enable.enable_apu {
            return;
        }

        if self.ch_1.vol_env.dac_enabled() && self.ch_1.wavelen_ctrl.trigger {
            self.ch_1.wavelen_ctrl.trigger = false;
            self.ch_1.volume = self.ch_1.vol_env.initial;
            self.ch_1.wave_idx = 0;
            if self.ch_1.wave_duty_len_timer.len_timer == 0 {
                self.ch_1.wave_duty_len_timer.len_timer = 0b100_0000;
            }
            self.sound_enable.channels.ch_1 = true;
            if (self.div & 1) == 1 {
                Self::tick_len(
                    &mut self.ch_1.wave_duty_len_timer.len_timer,
                    self.ch_1.wavelen_ctrl.len_enable,
                    &mut self.sound_enable.channels.ch_1,
                );
            }
        }

        if self.ch_2.vol_env.dac_enabled() && self.ch_2.wavelen_ctrl.trigger {
            self.ch_2.wavelen_ctrl.trigger = false;
            self.ch_2.volume = self.ch_2.vol_env.initial;
            self.ch_2.wave_idx = 0;
            if self.ch_2.wave_duty_len_timer.len_timer == 0 {
                self.ch_2.wave_duty_len_timer.len_timer = 0b100_0000;
            }
            self.sound_enable.channels.ch_2 = true;
            if self.div & 0b1 == 1 {
                Self::tick_len(
                    &mut self.ch_2.wave_duty_len_timer.len_timer,
                    self.ch_2.wavelen_ctrl.len_enable,
                    &mut self.sound_enable.channels.ch_2,
                );
            }
        }

        if self.ch_3.dac_enable && self.ch_3.wavelen_ctrl.trigger {
            self.ch_3.wavelen_ctrl.trigger = false;
            self.ch_3.wave_pattern_index = 0;
            if self.ch_3.len_timer == 0 {
                self.ch_3.len_timer = 0b1_0000_0000;
            }
            self.sound_enable.channels.ch_3 = true;
            if self.div & 0b1 == 1 {
                Self::tick_len(
                    &mut self.ch_3.len_timer,
                    self.ch_3.wavelen_ctrl.len_enable,
                    &mut self.sound_enable.channels.ch_3,
                );
            }
        }

        if self.ch_4.vol_env.dac_enabled() && self.ch_4.trigger {
            self.ch_4.trigger = false;
            self.ch_4.volume = self.ch_4.vol_env.initial;
            self.ch_4.lfsr = 0;
            self.ch_4.cycles = 0;
            if self.ch_4.len_timer == 0 {
                self.ch_4.len_timer = 0b100_0000;
            }
            self.sound_enable.channels.ch_4 = true;
            if self.div & 0b1 == 1 {
                Self::tick_len(
                    &mut self.ch_4.len_timer,
                    self.ch_4.len_enable,
                    &mut self.sound_enable.channels.ch_4,
                );
            }
        }

        let amp_ch1 = if self.sound_enable.channels.ch_1 {
            self.ch_1.cycles += 1;
            if self.ch_1.cycles == Self::WAVELEN_MAX {
                self.ch_1.cycles = self.ch_1.wavelen_ctrl.wavelen;
                self.ch_1.wave_idx = (self.ch_1.wave_idx + 1) % 8;
            }
            let val = if self.ch_1.wave_duty_len_timer.wave_duty.waveform()[self.ch_1.wave_idx] {
                self.ch_1.volume as i32
            } else {
                0
            };
            -val + 8
        } else {
            0
        };

        let amp_ch2 = if self.sound_enable.channels.ch_2 {
            self.ch_2.cycles += 1;
            if self.ch_2.cycles == Self::WAVELEN_MAX {
                self.ch_2.cycles = self.ch_2.wavelen_ctrl.wavelen;
                self.ch_2.wave_idx = (self.ch_2.wave_idx + 1) % 8;
            }
            let val = if self.ch_2.wave_duty_len_timer.wave_duty.waveform()[self.ch_2.wave_idx] {
                self.ch_2.volume as i32
            } else {
                0
            };
            -val + 8
        } else {
            0
        };

        let amp_ch4 = if self.sound_enable.channels.ch_4 {
            self.ch_4.cycles += 4;
            let clock_factor = if self.ch_4.freq_rand.clock_div == 0 {
                16 * (1 << (self.ch_4.freq_rand.clock_shift as u32)) / 2
            } else {
                16 * self.ch_4.freq_rand.clock_div as u32
                    * (1 << (self.ch_4.freq_rand.clock_shift as u32))
            };
            if self.ch_4.cycles % clock_factor == 0 {
                self.ch_4.cycles = 0;
                let b0 = self.ch_4.lfsr & 1;
                let b1 = (self.ch_4.lfsr >> 1) & 1;
                let res = !(b0 ^ b1) & 1;
                self.ch_4.lfsr = (self.ch_4.lfsr & !(1 << 15)) | (res << 15);
                if let LfsrWidth::B7 = self.ch_4.freq_rand.lfsr_width {
                    self.ch_4.lfsr = (self.ch_4.lfsr & !(1 << 7)) | (res << 7);
                }
                self.ch_4.lfsr >>= 1;
            }

            let val = if self.ch_4.lfsr & 1 != 0 {
                self.ch_4.volume as i32
            } else {
                0
            };
            -val + 8
        } else {
            0
        };

        for _ in 0..2 {
            let amp_ch3 = if self.sound_enable.channels.ch_3 {
                self.ch_3.cycles += 1;
                if self.ch_3.cycles == Self::WAVELEN_MAX {
                    self.ch_3.cycles = self.ch_3.wavelen_ctrl.wavelen;
                    self.ch_3.wave_pattern_index = (self.ch_3.wave_pattern_index + 1) % 32;
                }

                let index = self.ch_3.wave_pattern_index / 2;
                let high_nibble = (self.ch_3.wave_pattern_index % 2) == 0;
                let sample = if high_nibble {
                    self.ch_3.wave_pattern[index] >> 4
                } else {
                    self.ch_3.wave_pattern[index] & 0b1111
                };
                let sample = if self.ch_3.out_level == 0 {
                    0
                } else {
                    sample >> (self.ch_3.out_level - 1)
                };
                -(sample as i32) + 8
            } else {
                0
            };

            let l = &self.sound_panning.left;
            let r = &self.sound_panning.right;
            let amplitude_left = (if l.ch_1 { amp_ch1 } else { 0 }
                + if l.ch_2 { amp_ch2 } else { 0 }
                + if l.ch_3 { amp_ch3 } else { 0 }
                + if l.ch_4 { amp_ch4 } else { 0 })
                * self.master_vol_vin_pan.left_volume as i32
                * (1 << 6);
            let delta_left = amplitude_left - self.amplitude_left;
            self.amplitude_left = amplitude_left;
            let amplitude_right = (if r.ch_1 { amp_ch1 } else { 0 }
                + if r.ch_2 { amp_ch2 } else { 0 }
                + if r.ch_3 { amp_ch3 } else { 0 }
                + if r.ch_4 { amp_ch4 } else { 0 })
                * self.master_vol_vin_pan.right_volume as i32
                * (1 << 6);
            let delta_right = amplitude_right - self.amplitude_right;
            self.amplitude_right = amplitude_right;

            if delta_left != 0 || delta_right != 0 {
                self.left_deltas[self.delta_count] = delta_left;
                self.right_deltas[self.delta_count] = delta_right;
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
            Self::CH1_SWEEP_ADDRESS => self.ch_1.sweep.value(),
            Self::CH1_WAVE_DUTY_TIMER_LEN_ADDRESS => self.ch_1.wave_duty_len_timer.value(),
            Self::CH1_VOLUME_ENVELOPPE_ADDRESS => self.ch_1.vol_env.value(),
            Self::CH1_WAVELEN_LOW_ADDRESS => 0xFF,
            Self::CH1_WAVELEN_HIGH_CTRL_ADDRESS => self.ch_1.wavelen_ctrl.value_ctrl(),

            Self::CH2_UNUSED_ADDRESS => 0xFF,
            Self::CH2_WAVE_DUTY_TIMER_LEN_ADDRESS => self.ch_2.wave_duty_len_timer.value(),
            Self::CH2_VOLUME_ENVELOPPE_ADDRESS => self.ch_2.vol_env.value(),
            Self::CH2_WAVELEN_LOW_ADDRESS => 0xFF,
            Self::CH2_WAVELEN_HIGH_CTRL_ADDRESS => self.ch_2.wavelen_ctrl.value_ctrl(),

            Self::CH3_ENABLE_ADDRESS => (self.ch_3.dac_enable as u8) << 7 | 0b0111_1111,
            Self::CH3_LEN_TIMER_ADDRESS => 0xFF,
            Self::CH3_OUT_LEVEL_ADDRESS => 0b1001_1111 | (self.ch_3.out_level & 0b11) << 5,
            Self::CH3_WAVELEN_LOW_ADDRESS => 0xFF,
            Self::CH3_WAVELEN_HIGH_CTRL_ADDRESS => self.ch_3.wavelen_ctrl.value_ctrl(),
            Self::CH3_WAVE_PATTERN_START_ADDRESS..=Self::CH3_WAVE_PATTERN_END_ADDRESS => {
                self.ch_3.wave_pattern[(address & 0xF) as usize]
            }

            Self::CH4_UNUSED_ADDRESS => 0xFF,
            Self::CH4_LEN_TIMER_ADDRESS => 0xFF,
            Self::CH4_VOLUME_ENVELOPPE_ADDRESS => self.ch_4.vol_env.value(),
            Self::CH4_FREQ_RAND_ADDRESS => self.ch_4.freq_rand.value(),
            Self::CH4_CTRL_ADDRESS => 0b1011_1111 | ((self.ch_4.len_enable as u8) << 6),

            Self::MASTER_VOL_VIN_PAN_ADDRESS => self.master_vol_vin_pan.value(),
            Self::SOUND_PANNING_ADDRESS => self.sound_panning.value(),
            Self::SOUND_ENABLE_ADDRESS => self.sound_enable.value(),
            Self::UNUSED_START_ADDRESS..=Self::UNUSED_END_ADDRESS => 0xFF,
            _ => unreachable!("Tried to read invalid address {address:04X} in apu"),
        }
    }

    pub(crate) fn write(&mut self, address: u16, value: u8) {
        // Ignore register writes when APU is turned off.
        // On DMG models, timer registers are still writable.
        if !self.sound_enable.enable_apu
            && address != Self::SOUND_ENABLE_ADDRESS
            && !(Self::CH3_WAVE_PATTERN_START_ADDRESS..=Self::CH3_WAVE_PATTERN_END_ADDRESS)
                .contains(&address)
        {
            return;
        }

        match address {
            Self::CH1_SWEEP_ADDRESS => self.ch_1.sweep.set_value(value),
            Self::CH1_VOLUME_ENVELOPPE_ADDRESS => {
                self.ch_1.vol_env.set_value(value);
                if !self.ch_1.vol_env.dac_enabled() {
                    self.sound_enable.channels.ch_1 = false;
                }
            }
            Self::CH1_WAVE_DUTY_TIMER_LEN_ADDRESS => self.ch_1.wave_duty_len_timer.set_value(value),
            Self::CH1_WAVELEN_LOW_ADDRESS => self.ch_1.wavelen_ctrl.set_value_wavelen_l(value),
            Self::CH1_WAVELEN_HIGH_CTRL_ADDRESS => {
                self.ch_1.wavelen_ctrl.set_value_wavelen_h_ctrl(value);
            }

            Self::CH2_UNUSED_ADDRESS => {}
            Self::CH2_VOLUME_ENVELOPPE_ADDRESS => {
                self.ch_2.vol_env.set_value(value);
                if !self.ch_2.vol_env.dac_enabled() {
                    self.sound_enable.channels.ch_2 = false;
                }
            }
            Self::CH2_WAVE_DUTY_TIMER_LEN_ADDRESS => self.ch_2.wave_duty_len_timer.set_value(value),
            Self::CH2_WAVELEN_LOW_ADDRESS => self.ch_2.wavelen_ctrl.set_value_wavelen_l(value),
            Self::CH2_WAVELEN_HIGH_CTRL_ADDRESS => {
                self.ch_2.wavelen_ctrl.set_value_wavelen_h_ctrl(value)
            }

            Self::CH3_ENABLE_ADDRESS => {
                self.ch_3.dac_enable = value & 0b1000_0000 != 0;
                if !self.ch_3.dac_enable {
                    self.sound_enable.channels.ch_3 = false;
                }
            }
            Self::CH3_LEN_TIMER_ADDRESS => self.ch_3.len_timer = (!value as u16) + 1,
            Self::CH3_OUT_LEVEL_ADDRESS => self.ch_3.out_level = (value >> 5) & 0b11,
            Self::CH3_WAVELEN_LOW_ADDRESS => self.ch_3.wavelen_ctrl.set_value_wavelen_l(value),
            Self::CH3_WAVELEN_HIGH_CTRL_ADDRESS => {
                self.ch_3.wavelen_ctrl.set_value_wavelen_h_ctrl(value)
            }
            Self::CH3_WAVE_PATTERN_START_ADDRESS..=Self::CH3_WAVE_PATTERN_END_ADDRESS => {
                self.ch_3.wave_pattern[(address & 0xF) as usize] = value
            }

            Self::CH4_UNUSED_ADDRESS => {}
            Self::CH4_LEN_TIMER_ADDRESS => self.ch_4.len_timer = (!value & 0b11_1111) + 1,
            Self::CH4_VOLUME_ENVELOPPE_ADDRESS => {
                self.ch_4.vol_env.set_value(value);
                if !self.ch_4.vol_env.dac_enabled() {
                    self.sound_enable.channels.ch_4 = false;
                }
            }
            Self::CH4_FREQ_RAND_ADDRESS => self.ch_4.freq_rand.set_value(value),
            Self::CH4_CTRL_ADDRESS => {
                self.ch_4.trigger = value & 0b1000_0000 != 0;
                self.ch_4.len_enable = value & 0b100_0000 != 0;
            }

            Self::MASTER_VOL_VIN_PAN_ADDRESS => self.master_vol_vin_pan.set_value(value),
            Self::SOUND_PANNING_ADDRESS => self.sound_panning.set_value(value),
            Self::SOUND_ENABLE_ADDRESS => {
                self.sound_enable.enable_apu = value >> 7 != 0;
                if !self.sound_enable.enable_apu {
                    self.div = 0;
                    self.ch_1.reset();
                    self.ch_2.reset();
                    self.ch_3.reset();
                    self.ch_4.reset();
                    self.sound_enable.channels = Default::default();
                    self.master_vol_vin_pan = Default::default();
                    self.sound_panning = Default::default();
                }
            }
            Self::UNUSED_START_ADDRESS..=Self::UNUSED_END_ADDRESS => {}
            _ => unreachable!("Tried to write invalid address {address:04X} in apu"),
        }
    }
}

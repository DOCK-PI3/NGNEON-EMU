//! YM2610 (OPNB) FM/ADPCM sound chip emulation
//!
//! The YM2610 is the NeoGeo's sound chip, driven by the Z80 coprocessor.
//! It features:
//!   - 4 FM channels (4 operators each, 8 algorithms)
//!   - 6 ADPCM-A channels (fixed pitch, 4-bit ADPCM)
//!   - 1 ADPCM-B channel (variable pitch)
//!   - 3 SSG square wave + 1 noise channel (YM2149 compatible)
//!   - Built-in timers
//!
//! Register interface:
//!   - Port 0 (Z80 ports 0x04/0x05): SSG, ADPCM-A, FM ch 1-2, timers
//!   - Port 1 (Z80 ports 0x06/0x07): ADPCM-B, FM ch 3-4
//!
//! Z80 writes: address → port 0x04/0x06, then data → port 0x05/0x07
//! Z80 reads:  address → port 0x04/0x06, then data ← port 0x05/0x07
//!
//! Master clock: 8 MHz → sample rate: 55,555 Hz (8M / 144)

pub const YM2610_MASTER_CLOCK: u32 = 8_000_000;
pub const YM2610_SAMPLE_RATE: u32 = YM2610_MASTER_CLOCK / 144; // ~55,555 Hz
const YM2610_OUTPUT_GAIN_SHIFT: u8 = 1;

use crate::memory::Memory;
use std::cell::{Cell, RefCell};
use std::ptr;
use std::rc::Rc;

unsafe extern "C" {
    fn ng_ymfm_init();
    fn ng_ymfm_reset();
    fn ng_ymfm_set_roms(v1: *const u8, v1_size: usize, v2: *const u8, v2_size: usize);
    fn ng_ymfm_read(offset: u32) -> u8;
    fn ng_ymfm_write(offset: u32, data: u8);
    fn ng_ymfm_generate(dst: *mut i16, sample_pairs: usize);
    fn ng_ymfm_irq_asserted() -> u8;
    fn ng_ymfm_timer_remaining(tnum: u32) -> i32;
    fn ng_ymfm_busy_remaining() -> i32;
    fn ng_ymfm_adpcm_wrap(wrap: i32);
    fn ng_ymfm_state_save(dst: *mut u8, capacity: usize) -> usize;
    fn ng_ymfm_state_load(src: *const u8, size: usize);
}

// ─── Constants ───────────────────────────────────────────────────────

const NUM_CHANNELS: usize = 4;
const NUM_OPERATORS: usize = 4;
const OPERATORS_PER_BANK: usize = 2; // ch1-2 on port 0, ch3-4 on port 1
pub const NUM_ADPCM_A_CHANNELS: usize = 6;
pub const YM2610_LEGACY_SAVE_STATE_SIZE: usize = 612;
const GEOLITH_YMFM_STATE_MAGIC: [u8; 4] = *b"GYMF";
const GEOLITH_YMFM_STATE_CAPACITY: usize = 64 * 1024;
const EOS_FLAGS_MASK: u8 = 0xBF;
const ADPCM_B_EOS_HIDDEN: u8 = 0x40;
const ADPCM_B_EOS_VISIBLE: u8 = 0x80;
const ATTACK: usize = 0;
const DECAY: usize = 1;
const SUSTAIN: usize = 2;
const RELEASE: usize = 3;

// ─── FM Algorithms (operator connections) ────────────────────────────

/// 8 algorithms defining how 4 operators are connected.
/// Each algorithm specifies in[N] = which operator feeds into op N (-1 = none).
/// Out[N] = which operators feed from op N (-1 = none, goes to output instead).
const ALGORITHMS: [[i32; 4]; 8] = [
    // alg 0: 1→2→3→4 (serial)
    [-1, 0, 1, 2],
    // alg 1: 1→(2+3)→4
    [-1, 0, 0, 2],
    // alg 2: (1+3)→2→4
    [-1, 0, -1, 1],
    // alg 3: 1→2→3, 4 parallel
    [-1, 0, 1, -1],
    // alg 4: 1→(2+3+4) — op0 feeds all three others
    [-1, 0, 0, 0],
    // alg 5: (1+2+3)→4 — all three modulators feed the carrier (op3)
    [-1, -1, -1, -2],
    // alg 6: (1+2)→3→4
    [-1, -1, 0, 2],
    // alg 7: all parallel (1+2+3+4)
    [-1, -1, -1, -1],
];

// ─── Envelope rate / attenuation tables ──────────────────────────────

/// Attack rate table: 0-63 → time constant (in samples)
/// Higher values = faster attack.
const AR_TABLE: [f32; 63] = [
    8191.5, 5461.0, 4095.8, 3276.6, 2730.5, 2340.4, 2047.9, 1820.3, 1638.3, 1489.4, 1365.3, 1260.2,
    1170.2, 1092.2, 1023.9, 963.7, 910.2, 862.3, 819.2, 780.1, 744.7, 712.3, 682.6, 655.4, 630.1,
    606.8, 585.1, 564.9, 546.1, 528.5, 512.0, 496.5, 481.9, 468.1, 455.1, 442.8, 431.1, 420.1,
    409.6, 399.6, 390.1, 381.0, 372.4, 364.1, 356.2, 348.6, 341.3, 334.3, 327.7, 321.3, 315.1,
    309.2, 303.4, 297.9, 292.6, 287.4, 282.4, 277.6, 273.1, 268.6, 264.3, 260.1, 256.0,
];
/// Decay rate table: 0-63 → dB/second
const DR_TABLE: [f32; 64] = [
    22.97, 43.51, 61.89, 78.42, 93.42, 107.10, 119.63, 131.13, 141.73, 151.50, 160.53, 168.88,
    176.61, 183.77, 190.39, 196.52, 202.19, 207.44, 212.28, 216.75, 220.87, 224.66, 228.13, 231.31,
    234.21, 236.85, 239.24, 241.39, 243.31, 245.01, 246.50, 247.79, 248.88, 249.78, 250.50, 251.03,
    251.38, 251.56, 251.56, 251.56, 251.56, 251.56, 251.56, 251.56, 251.56, 251.56, 251.56, 251.56,
    251.56, 251.56, 251.56, 251.56, 251.56, 251.56, 251.56, 251.56, 251.56, 251.56, 251.56, 251.56,
    251.56, 251.56, 251.56, 251.56,
];

/// Sin table: 1024 entries for sin(x) where x in [0, 2*PI)
const SIN_TABLE_SIZE: usize = 1024;
fn build_sin_table() -> [i16; SIN_TABLE_SIZE] {
    let mut table = [0i16; SIN_TABLE_SIZE];
    for (i, sample) in table.iter_mut().enumerate() {
        let phase = (i as f64) * 2.0 * std::f64::consts::PI / (SIN_TABLE_SIZE as f64);
        *sample = ((phase.sin() * 16383.0) as i16).clamp(-16384, 16383);
    }
    table
}

lazy_static::lazy_static! {
    static ref SIN_TABLE: [i16; SIN_TABLE_SIZE] = build_sin_table();
}

fn sin_lookup(phase: u32) -> i16 {
    let idx = (phase >> 10) as usize & (SIN_TABLE_SIZE - 1);
    SIN_TABLE[idx]
}

/// Convert logarithmic attenuation (0.75 dB/step) to linear amplitude.
/// Total Level is 0-127, where 0 = loudest, 127 = silent.
fn attenuation_to_amplitude(tl: u8) -> f32 {
    // 0.75 dB per step, -96 dB max
    10.0f32.powf(-0.75 * tl as f32 / 20.0)
}

// ─── Envelope Generator ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
struct EnvelopeGenerator {
    state: u8,        // 0=Attack, 1=Decay, 2=Sustain, 3=Release
    attenuation: f32, // current attenuation in linear
    ar: u8,
    dr: u8,
    sr: u8,
    rr: u8,
    sl: u8, // sustain level (0-15, 0 = max, 15 = min)
    key_on: bool,
    ssg_eg: u8, // SSG-EG mode (0-7 for special envelope shapes)
    ssg_dir: bool,
}

impl EnvelopeGenerator {
    fn new() -> Self {
        Self {
            state: RELEASE as u8,
            attenuation: 0.0,
            ar: 0,
            dr: 0,
            sr: 0,
            rr: 0,
            sl: 0,
            key_on: false,
            ssg_eg: 0,
            ssg_dir: true,
        }
    }

    fn reset(&mut self) {
        self.state = RELEASE as u8;
        self.attenuation = 0.0;
        self.key_on = false;
    }

    fn key_on(&mut self) {
        // Always restart attack phase, even if already key-on.
        // NeoGeo sound drivers often re-trigger the same channel
        // for new notes without an explicit key-off — without this
        // the EG stays stuck in SUSTAIN/RELEASE and produces silence.
        self.key_on = true;
        self.state = ATTACK as u8;
        // Start from max attenuation in attack
        self.attenuation = 1.0;
        self.ssg_dir = true;
    }

    fn key_off(&mut self) {
        if !self.key_on {
            return;
        }
        self.key_on = false;
        self.state = RELEASE as u8;
    }

    /// Advance the envelope by one sample. Returns the current amplitude multiplier [0, 1].
    fn clock(&mut self) -> f32 {
        match self.state as usize {
            ATTACK => {
                // AR_TABLE stores time constants in samples.  AR=0 is slowest
                // (largest constant), AR=31 is fastest (smallest).  The per-
                // sample decrement is 1.0 / time_constant.
                let rate = 1.0 / AR_TABLE[self.ar as usize];
                self.attenuation -= rate;
                if self.attenuation <= 0.0 {
                    self.attenuation = 0.0;
                    self.state = DECAY as u8;
                }
            }
            DECAY => {
                let dr_db = DR_TABLE[self.dr as usize];
                let rate = dr_db / (YM2610_SAMPLE_RATE as f32) / 8.691;
                self.attenuation += rate;
                // SL=0 means max sustain (loudest).  The decay phase still
                // runs for at least one sample before transitioning to
                // sustain — otherwise the EG skips decay entirely when SL=0
                // and the note starts at full decay attenuation immediately.
                let sl_attn = self.sl as f32 * 3.0 / 127.0;
                if self.attenuation >= sl_attn && sl_attn > 0.0 {
                    self.attenuation = sl_attn;
                    self.state = SUSTAIN as u8;
                } else if self.attenuation >= 1.0 {
                    // SL=0: let attenuation grow all the way before transitioning
                    self.attenuation = 1.0;
                    self.state = SUSTAIN as u8;
                }
            }
            SUSTAIN if self.sr > 0 => {
                let dr_db = DR_TABLE[self.sr as usize];
                let rate = dr_db / (YM2610_SAMPLE_RATE as f32) / 8.691;
                self.attenuation += rate;
                if self.attenuation >= 1.0 {
                    self.attenuation = 1.0;
                }
            }
            SUSTAIN => {}
            RELEASE => {
                if self.rr > 0 {
                    let dr_db = DR_TABLE[self.rr as usize];
                    let rate = dr_db / (YM2610_SAMPLE_RATE as f32) / 8.691 * 2.0;
                    self.attenuation += rate;
                    if self.attenuation >= 1.0 {
                        self.attenuation = 1.0;
                    }
                } else {
                    // rr=0: instant release
                    self.attenuation = 1.0;
                }
            }
            _ => {}
        }

        // SSG-EG support
        if self.ssg_eg > 0 && self.ssg_eg < 8 {
            self.apply_ssg_eg();
        }

        // Convert to amplitude
        1.0 - self.attenuation.min(1.0)
    }

    fn apply_ssg_eg(&mut self) {
        match self.ssg_eg {
            1 | 3 | 5 | 7 if self.state == SUSTAIN as u8 => {
                // Saw-like envelope
                if self.ssg_dir {
                    self.attenuation += 0.001;
                    if self.attenuation >= 1.0 {
                        self.attenuation = 1.0;
                        self.ssg_dir = false;
                    }
                } else {
                    self.attenuation -= 0.001;
                    if self.attenuation <= 0.0 {
                        self.attenuation = 0.0;
                        self.ssg_dir = true;
                    }
                }
            }
            _ => {} // Other SSG-EG modes simplified
        }
    }
}

// ─── Operator ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct Operator {
    // Phase generator
    phase: u32,    // current phase (10.10 fixed point for sin table)
    freq_num: u16, // F-number (frequency)
    block: u8,     // octave block (0-7)
    mul: u8,       // frequency multiplier (0-15, 0 = 0.5)
    dt: u8,        // detune (0-7)

    // Envelope
    eg: EnvelopeGenerator,

    // Level
    tl: u8,   // total level (0-127)
    ks: u8,   // key scaling (0-3)
    am: bool, // amplitude modulation enable

    // Previous output for feedback calculation
    prev_out: i16,
}

impl Operator {
    fn new() -> Self {
        Self {
            phase: 0,
            freq_num: 0,
            block: 0,
            mul: 0,
            dt: 0,
            eg: EnvelopeGenerator::new(),
            tl: 0,
            ks: 0,
            am: false,
            prev_out: 0,
        }
    }

    fn reset(&mut self) {
        self.phase = 0;
        self.freq_num = 0;
        self.block = 0;
        self.mul = 0;
        self.dt = 0;
        self.eg.reset();
        self.tl = 127;
        self.ks = 0;
        self.am = false;
        self.prev_out = 0;
    }

    /// Compute the phase increment for this operator.
    /// YM2610 formula: Fnum * multiplier * 2^(block-1) for block >= 1,
    ///                Fnum * multiplier / 2 for block == 0.
    /// The phase accumulator uses 20 fractional bits (sin table = 1024 entries = 10 bits).
    fn phase_step(&self) -> u32 {
        let mul = if self.mul == 0 { 1 } else { self.mul as u32 }; // 0 = 0.5x, approximated as 1x
        let fnum = self.freq_num as u32;
        let step = fnum * mul;
        if self.block == 0 {
            step >> 1
        } else {
            step << (self.block as u32 - 1).min(16)
        }
    }

    /// Generate operator output sample. `modulation` is the phase modulation input.
    fn clock(&mut self, modulation: i32) -> i32 {
        let step = self.phase_step();
        self.phase = self.phase.wrapping_add(step);

        // Apply phase modulation from carrier/modulator
        let phase = self.phase.wrapping_add(modulation as u32);

        // Look up sin table
        let raw = sin_lookup(phase) as i32;

        // Apply envelope
        let eg_amp = self.eg.clock();
        let tl_amp = attenuation_to_amplitude(self.tl);
        let output = (raw as f32 * eg_amp * tl_amp) as i32;

        self.prev_out = output as i16;
        output
    }
}

// ─── FM Channel ───────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct FmChannel {
    operators: [Operator; NUM_OPERATORS],
    algo: u8,          // algorithm (0-7)
    feedback: u8,      // feedback level (0-7)
    feedback_buf: i32, // persistent feedback buffer across samples
    pan_left: bool,
    pan_right: bool,
    ams: u8,   // AM sensitivity (0-3)
    pms: u8,   // FM sensitivity (0-7, high 3 bits)
    fnum: u16, // channel frequency number
    block: u8, // channel octave block
    key_on: bool,
}

impl FmChannel {
    fn new() -> Self {
        Self {
            operators: [
                Operator::new(),
                Operator::new(),
                Operator::new(),
                Operator::new(),
            ],
            algo: 0,
            feedback: 0,
            feedback_buf: 0,
            pan_left: true,
            pan_right: true,
            ams: 0,
            pms: 0,
            fnum: 0,
            block: 0,
            key_on: false,
        }
    }

    fn reset(&mut self) {
        for op in &mut self.operators {
            op.reset();
        }
        self.algo = 0;
        self.feedback = 0;
        self.feedback_buf = 0;
        self.pan_left = true;
        self.pan_right = true;
        self.ams = 0;
        self.pms = 0;
        self.fnum = 0;
        self.block = 0;
        self.key_on = false;
    }

    /// Distribute F-Num and Block to all operators in the channel.
    fn update_freq(&mut self) {
        for op in &mut self.operators {
            op.freq_num = self.fnum;
            op.block = self.block;
        }
    }

    /// Compute one sample for this FM channel.
    /// Returns (left, right) stereo sample pair.
    fn clock(&mut self, lfo_am: f32, lfo_fm: i32) -> (i32, i32) {
        self.update_freq();

        let algo = &ALGORITHMS[self.algo as usize];
        let mut op_outputs = [0i32; NUM_OPERATORS];
        // Snapshot current feedback before the loop (avoids split-borrow
        // issues when updating the persistent field after clocking op 0).
        let current_feedback = self.feedback_buf;

        for op_idx in 0..NUM_OPERATORS {
            let mut modulation = lfo_fm; // LFO phase modulation

            // Apply algorithm-specific modulation
            let in_op = algo[op_idx];
            if in_op >= 0 {
                modulation += op_outputs[in_op as usize];
            } else if in_op == -2 {
                // Multiple inputs: sum from all previous operators
                for j in 0..op_idx {
                    if algo[j] < 0 {
                        modulation += op_outputs[j];
                    }
                }
            }

            // Apply feedback for operator 0 (uses persistent buffer
            // that carries state across samples).  We snapshot the buffer
            // before the loop so that the same value feeds all samples.
            if op_idx == 0 && self.feedback > 0 {
                let fb_shift = 8 - self.feedback as i32;
                modulation += current_feedback >> fb_shift.max(1);
            }

            op_outputs[op_idx] = self.operators[op_idx].clock(modulation);

            // Update persistent feedback buffer after computing operator 0
            if op_idx == 0 && self.feedback > 0 {
                self.feedback_buf = (op_outputs[0] + current_feedback) / 2;
            }
        }

        // Determine which operators feed output.
        // An operator contributes to output if no later operator takes it as input.
        // - algo[j] == N   → operator j takes N directly → N doesn't feed output
        // - algo[j] == -2  → operator j takes all previous "free" operators
        //                    (those with algo[k] < 0) → N doesn't feed output if N is free
        // Compute the output sum — which operators contribute to the mix
        let output_sum = {
            let mut sum = 0i32;
            for op_idx in 0..NUM_OPERATORS {
                let mut feeds_output = true;
                for j in (op_idx + 1)..NUM_OPERATORS {
                    if algo[j] == op_idx as i32 {
                        feeds_output = false;
                        break;
                    }
                    if algo[j] == -2 && algo[op_idx] < 0 {
                        feeds_output = false;
                        break;
                    }
                }
                if feeds_output {
                    sum += op_outputs[op_idx];
                }
            }
            (sum as f32 * (1.0 + lfo_am * self.ams as f32 * 0.25)) as i32
        };

        let out_left = if self.pan_left { output_sum } else { 0 };
        let out_right = if self.pan_right { output_sum } else { 0 };

        (out_left, out_right)
    }
}

// ─── SSG (YM2149 compatible) ─────────────────────────────────────────

#[derive(Debug, Clone)]
struct SsgChannel {
    fine_tune: u8,
    coarse_tune: u8,
    volume: u8,
}

impl SsgChannel {
    fn new() -> Self {
        Self {
            fine_tune: 0,
            coarse_tune: 0,
            volume: 0,
        }
    }
}

#[derive(Debug, Clone)]
struct Ssg {
    channels: [SsgChannel; 3],
    noise_tune: u8,
    noise_enable: [bool; 3],
    tone_enable: [bool; 3],
    envelope_period: u16,
    envelope_shape: u8,
    tone_count: [u32; 3],
    tone_state: [u32; 3],
    envelope_count: u32,
    envelope_state: u32,
    noise_count: u32,
    noise_state: u32,
    resample_index: u32,
    resample_last: [i32; 3],
}

impl Ssg {
    const AMPLITUDES: [i32; 32] = [
        0, 32, 78, 141, 178, 222, 262, 306, 369, 441, 509, 585, 701, 836, 965, 1112, 1334, 1595,
        1853, 2146, 2576, 3081, 3576, 4135, 5000, 6006, 7023, 8155, 9963, 11976, 14132, 16382,
    ];

    fn new() -> Self {
        Self {
            channels: [SsgChannel::new(), SsgChannel::new(), SsgChannel::new()],
            noise_tune: 0,
            noise_enable: [false; 3],
            tone_enable: [false; 3],
            envelope_period: 0,
            envelope_shape: 0,
            tone_count: [0; 3],
            tone_state: [0; 3],
            envelope_count: 0,
            envelope_state: 0,
            noise_count: 0,
            noise_state: 1,
            resample_index: 0,
            resample_last: [0; 3],
        }
    }

    fn clock(&mut self) -> i32 {
        let mut sum = [0i32; 3];
        if self.resample_index & 1 != 0 {
            Self::add_scaled(&mut sum, self.resample_last, 1);
        }
        self.clock_and_add(&mut sum, 2);
        self.clock_and_add(&mut sum, 2);
        self.clock_and_add(&mut sum, 2);
        self.clock_and_add(&mut sum, 2);
        if self.resample_index & 1 == 0 {
            self.clock_and_add(&mut sum, 1);
        }
        self.resample_index = self.resample_index.wrapping_add(1);

        (sum[0] + sum[1] + sum[2]) * 2 / (3 * 9)
    }

    fn add_scaled(sum: &mut [i32; 3], value: [i32; 3], scale: i32) {
        for ch in 0..3 {
            sum[ch] += value[ch] * scale;
        }
    }

    fn clock_and_add(&mut self, sum: &mut [i32; 3], scale: i32) {
        self.clock_internal();
        self.resample_last = self.output_channels();
        Self::add_scaled(sum, self.resample_last, scale);
    }

    fn clock_internal(&mut self) {
        for ch in 0..3 {
            self.tone_count[ch] = self.tone_count[ch].wrapping_add(1);
            if self.tone_count[ch] >= self.tone_period(ch) {
                self.tone_state[ch] ^= 1;
                self.tone_count[ch] = 0;
            }
        }

        self.noise_count = self.noise_count.wrapping_add(1);
        if (self.noise_count >> 1) >= self.noise_tune as u32 && self.noise_count != 1 {
            let feedback = ((self.noise_state & 1) ^ ((self.noise_state >> 3) & 1)) << 17;
            self.noise_state = (self.noise_state ^ feedback) >> 1;
            self.noise_count = 0;
        }

        self.envelope_count = self.envelope_count.wrapping_add(1);
        if self.envelope_count >= self.envelope_period as u32 {
            self.envelope_state = self.envelope_state.wrapping_add(1);
            self.envelope_count = 0;
        }
    }

    fn output_channels(&mut self) -> [i32; 3] {
        let mut output = [0i32; 3];
        let envelope_volume = self.envelope_volume();

        for (ch, out) in output.iter_mut().enumerate() {
            let noise_on = (!self.noise_enable[ch] as u32) | (self.noise_state & 1);
            let tone_on = (!self.tone_enable[ch] as u32) | self.tone_state[ch];
            let volume = if (noise_on & tone_on) == 0 {
                0
            } else if self.channels[ch].volume & 0x10 != 0 {
                envelope_volume
            } else {
                let mut fixed = (self.channels[ch].volume & 0x0F) as u32 * 2;
                if fixed != 0 {
                    fixed |= 1;
                }
                fixed
            };
            *out = Self::AMPLITUDES[volume.min(31) as usize];
        }

        output
    }

    fn envelope_volume(&mut self) -> u32 {
        let hold = self.envelope_shape & 0x01 != 0;
        let alternate = self.envelope_shape & 0x02 != 0;
        let attack = self.envelope_shape & 0x04 != 0;
        let cont = self.envelope_shape & 0x08 != 0;

        if (hold || !cont) && self.envelope_state >= 32 {
            self.envelope_state = 32;
            if (attack ^ alternate) && cont {
                31
            } else {
                0
            }
        } else {
            let mut effective_attack = attack;
            if alternate {
                effective_attack ^= (self.envelope_state & 0x20) != 0;
            }
            (self.envelope_state & 31) ^ if effective_attack { 0 } else { 31 }
        }
    }

    fn tone_period(&self, ch: usize) -> u32 {
        ((self.channels[ch].coarse_tune as u32 & 0x0F) << 8) | self.channels[ch].fine_tune as u32
    }
}

// ─── ADPCM-A ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct AdpcmAChannel {
    start_addr: u32, // sample start in V-ROM (in 256-byte units)
    stop_addr: u32,  // sample stop in V-ROM (in 256-byte units)
    volume: u8,      // volume (0-15)
    pan_left: bool,
    pan_right: bool,
    playing: bool,
    current_addr: u32,
    nibble_pos: u8, // 0 = high nibble, 1 = low nibble
    decoder: AdpcmADecoder,
    output: i16,
}

impl AdpcmAChannel {
    fn new() -> Self {
        Self {
            start_addr: 0,
            stop_addr: 0,
            volume: 0x1F,
            pan_left: true,
            pan_right: true,
            playing: false,
            current_addr: 0,
            nibble_pos: 0,
            decoder: AdpcmADecoder::new(),
            output: 0,
        }
    }

    fn start_playback(&mut self) {
        self.playing = true;
        self.current_addr = self.start_addr << 8;
        self.nibble_pos = 0;
        self.decoder.reset();
        self.output = 0;
    }
}

// ─── ADPCM-B ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct AdpcmBChannel {
    start_addr: u32,
    stop_addr: u32,
    limit_addr: u32,
    delta_n: u16,
    volume: u8,
    repeat: bool,
    pan_left: bool,
    pan_right: bool,
    playing: bool,
    current_addr: u32,
    nibble_pos: u8,
    position: u32,
    decoder: AdpcmBDecoder,
    output: i16,
    prev_output: i16,
}

impl AdpcmBChannel {
    fn new() -> Self {
        Self {
            start_addr: 0,
            stop_addr: 0,
            limit_addr: 0xFFFF,
            delta_n: 0,
            volume: 0,
            repeat: false,
            pan_left: true,
            pan_right: true,
            playing: false,
            current_addr: 0,
            nibble_pos: 0,
            position: 0,
            decoder: AdpcmBDecoder::new(),
            output: 0,
            prev_output: 0,
        }
    }

    fn reset(&mut self) {
        self.playing = false;
        self.current_addr = self.start_addr * 256;
        self.nibble_pos = 0;
        self.position = 0;
        self.decoder.reset();
        self.output = 0;
        self.prev_output = 0;
    }

    fn start_playback(&mut self) {
        self.playing = true;
        self.current_addr = self.start_addr << 8;
        self.nibble_pos = 0;
        self.position = 0;
        self.decoder.reset();
        self.output = 0;
        self.prev_output = 0;
    }
}

// ─── ADPCM Decoders (4-bit Yamaha ADPCM) ─────────────────────────────

#[derive(Debug, Clone)]
struct AdpcmADecoder {
    accumulator: i32,
    step_index: u8,
}

/// ADPCM-A step size table (Yamaha 4-bit ADPCM)
const ADPCM_A_STEP_TABLE: [i32; 49] = [
    16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66, 73, 80, 88, 97, 107, 118, 130,
    143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449, 494, 544, 598, 658, 724, 796,
    876, 963, 1060, 1166, 1282, 1411, 1552,
];

impl AdpcmADecoder {
    fn new() -> Self {
        Self {
            accumulator: 0,
            step_index: 0,
        }
    }

    fn reset(&mut self) {
        self.accumulator = 0;
        self.step_index = 0;
    }

    fn decode_nibble(&mut self, nibble: u8) -> i16 {
        let step = ADPCM_A_STEP_TABLE[self.step_index as usize];
        let mut delta = ((2 * (nibble & 7) as i32) + 1) * step / 8;
        if nibble & 8 != 0 {
            delta = -delta;
        }

        // YM2610 ADPCM-A is a 12-bit wrapping accumulator, not a 16-bit
        // saturating IMA-style decoder. The sign extension is done when
        // converting the 12-bit accumulator to a sample for output.
        self.accumulator = (self.accumulator + delta) & 0x0FFF;

        let adjust: i16 = match nibble & 7 {
            0..=3 => -1,
            4 => 2,
            5 => 5,
            6 => 7,
            7 => 9,
            _ => unreachable!(),
        };
        self.step_index = (self.step_index as i16 + adjust).clamp(0, 48) as u8;

        self.output()
    }

    fn output(&self) -> i16 {
        (self.accumulator as i16).wrapping_shl(4)
    }

    fn restore_from_output(&mut self, output: i16, step_index: u8) {
        self.accumulator = ((output as i32) >> 4) & 0x0FFF;
        self.step_index = step_index.min(48);
    }
}

#[derive(Debug, Clone)]
struct AdpcmBDecoder {
    accumulator: i32,
    step: i32,
}

const ADPCM_B_STEP_MIN: i32 = 127;
const ADPCM_B_STEP_MAX: i32 = 24576;
const ADPCM_B_STEP_SCALE: [i32; 8] = [57, 57, 57, 57, 77, 102, 128, 153];

impl AdpcmBDecoder {
    fn new() -> Self {
        Self {
            accumulator: 0,
            step: ADPCM_B_STEP_MIN,
        }
    }

    fn reset(&mut self) {
        self.accumulator = 0;
        self.step = ADPCM_B_STEP_MIN;
    }

    fn decode_nibble(&mut self, nibble: u8) -> i16 {
        let mut delta = ((2 * (nibble & 7) as i32) + 1) * self.step / 8;
        if nibble & 8 != 0 {
            delta = -delta;
        }

        self.accumulator = (self.accumulator + delta).clamp(i16::MIN as i32, i16::MAX as i32);
        self.step = (self.step * ADPCM_B_STEP_SCALE[(nibble & 7) as usize] / 64)
            .clamp(ADPCM_B_STEP_MIN, ADPCM_B_STEP_MAX);

        self.accumulator as i16
    }

    fn output(&self) -> i16 {
        self.accumulator as i16
    }

    fn restore_from_output(&mut self, output: i16) {
        self.accumulator = output as i32;
    }
}

// ─── LFO ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct Lfo {
    phase: u32,
    am_depth: u8, // 0-3 (0 = 0 dB, 3 = 4.8 dB)
    fm_depth: u8, // 0-7 (3 bits)
    freq: u8,     // LFO frequency (0-7)
}

impl Lfo {
    fn new() -> Self {
        Self {
            phase: 0,
            am_depth: 0,
            fm_depth: 0,
            freq: 0,
        }
    }

    /// Clock LFO and return (am_value, fm_value)
    fn clock(&mut self) -> (f32, i32) {
        let step = if self.freq > 0 {
            (self.freq as u32 + 1) * 1024
        } else {
            512
        };
        self.phase = self.phase.wrapping_add(step);

        let wave = sin_lookup(self.phase) as f32 / 16384.0;

        let am = wave * self.am_depth as f32 * 0.015; // AM in range [0, 0.045]
        let fm = (wave * self.fm_depth as f32 * 512.0) as i32;

        (am, fm)
    }
}

// ─── YM2610 Chip ─────────────────────────────────────────────────────

pub struct Ym2610 {
    // 2 ports × 256 registers
    pub(crate) reg: [[u8; 256]; 2],

    // Current register address for each port
    addr_latch: [u8; 2],

    // FM synthesis
    channels: [FmChannel; NUM_CHANNELS],

    // SSG
    ssg: Ssg,

    // ADPCM
    adpcm_a: [AdpcmAChannel; NUM_ADPCM_A_CHANNELS],
    adpcm_a_total_level: u8,
    adpcm_b: AdpcmBChannel,

    // LFO
    lfo: Lfo,

    // Timers
    timer_a_counter: u16,
    timer_b_counter: u8,
    timer_a_load: u16,
    timer_b_load: u8,
    timer_a_enable: bool,
    timer_b_enable: bool,
    timer_a_flag: Cell<bool>,
    timer_b_flag: Cell<bool>,
    timer_b_fractional: f32,

    // Status
    busy: bool,
    eos_status: u8,
    eos_mask: u8,

    // Sample counter for timing
    sample_count: u64,

    // Reference to V-ROM for ADPCM playback
    vrom: Rc<RefCell<Memory>>,

    // Runtime backend. The Rust implementation remains available for focused
    // unit tests; normal emulation uses Geolith's YMFM core through FFI.
    geolith_backend: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ym2610AudioDebug {
    pub geolith_backend: bool,
    pub timer_mode: u8,
    pub timer_remaining: [i32; 2],
    pub busy_remaining: i32,
    pub irq_asserted: bool,
    pub adpcm_a_playing: [bool; NUM_ADPCM_A_CHANNELS],
    pub adpcm_a_output: [i16; NUM_ADPCM_A_CHANNELS],
    pub adpcm_a_addr: [u32; NUM_ADPCM_A_CHANNELS],
    pub adpcm_b_playing: bool,
    pub adpcm_b_output: i16,
    pub adpcm_b_prev_output: i16,
    pub adpcm_b_addr: u32,
    pub adpcm_b_start: u32,
    pub adpcm_b_stop: u32,
    pub adpcm_b_delta_n: u16,
    pub adpcm_b_volume: u8,
}

impl Ym2610 {
    pub fn new(vrom: Rc<RefCell<Memory>>) -> Self {
        Self {
            reg: [[0u8; 256]; 2],
            addr_latch: [0; 2],
            channels: [
                FmChannel::new(),
                FmChannel::new(),
                FmChannel::new(),
                FmChannel::new(),
            ],
            ssg: Ssg::new(),
            adpcm_a: [
                AdpcmAChannel::new(),
                AdpcmAChannel::new(),
                AdpcmAChannel::new(),
                AdpcmAChannel::new(),
                AdpcmAChannel::new(),
                AdpcmAChannel::new(),
            ],
            adpcm_a_total_level: 0,
            adpcm_b: AdpcmBChannel::new(),
            lfo: Lfo::new(),
            timer_a_counter: 0,
            timer_b_counter: 0,
            timer_a_load: 0,
            timer_b_load: 0,
            timer_a_enable: false,
            timer_b_enable: false,
            timer_a_flag: Cell::new(false),
            timer_b_flag: Cell::new(false),
            timer_b_fractional: 0.0,
            busy: false,
            eos_status: 0,
            eos_mask: EOS_FLAGS_MASK,
            sample_count: 0,
            vrom,
            geolith_backend: false,
        }
    }

    pub fn enable_geolith_backend(&mut self) {
        self.geolith_backend = true;
        unsafe {
            ng_ymfm_init();
            ng_ymfm_reset();
            ng_ymfm_adpcm_wrap(1);
        }
        self.update_geolith_roms();
    }

    pub fn reset(&mut self) {
        self.reg = [[0u8; 256]; 2];
        self.addr_latch = [0; 2];
        for ch in &mut self.channels {
            ch.reset();
        }
        self.ssg = Ssg::new();
        for ch in &mut self.adpcm_a {
            *ch = AdpcmAChannel::new();
        }
        self.adpcm_a_total_level = 0;
        self.adpcm_b = AdpcmBChannel::new();
        self.lfo = Lfo::new();
        self.timer_a_counter = 0;
        self.timer_b_counter = 0;
        self.timer_a_load = 0;
        self.timer_b_load = 0;
        self.timer_a_enable = false;
        self.timer_b_enable = false;
        self.timer_a_flag.set(false);
        self.timer_b_flag.set(false);
        self.timer_b_fractional = 0.0;
        self.busy = false;
        self.eos_status = 0;
        self.eos_mask = EOS_FLAGS_MASK;
        self.sample_count = 0;
        if self.geolith_backend {
            unsafe {
                ng_ymfm_reset();
                ng_ymfm_adpcm_wrap(1);
            }
            self.update_geolith_roms();
        }
    }

    fn update_geolith_roms(&self) {
        if !self.geolith_backend {
            return;
        }

        let vrom = self.vrom.borrow();
        if vrom.vrom.is_empty() {
            unsafe {
                ng_ymfm_set_roms(ptr::null(), 0, ptr::null(), 0);
            }
            return;
        }

        let base = vrom.vrom.as_ptr();
        let split = if vrom.vrom_b_offset == 0 {
            vrom.vrom.len()
        } else {
            vrom.vrom_b_offset.min(vrom.vrom.len())
        };
        let v1_size = split;
        let (v2_ptr, v2_size) = if vrom.vrom_b_offset == 0 || split >= vrom.vrom.len() {
            (base, vrom.vrom.len())
        } else {
            unsafe { (base.add(split), vrom.vrom.len() - split) }
        };

        unsafe {
            ng_ymfm_set_roms(base, v1_size, v2_ptr, v2_size);
        }
    }

    pub fn audio_debug(&self) -> Ym2610AudioDebug {
        let mut adpcm_a_playing = [false; NUM_ADPCM_A_CHANNELS];
        let mut adpcm_a_output = [0i16; NUM_ADPCM_A_CHANNELS];
        let mut adpcm_a_addr = [0u32; NUM_ADPCM_A_CHANNELS];
        for (idx, ch) in self.adpcm_a.iter().enumerate() {
            adpcm_a_playing[idx] = ch.playing;
            adpcm_a_output[idx] = ch.output;
            adpcm_a_addr[idx] = ch.current_addr;
        }

        Ym2610AudioDebug {
            geolith_backend: self.geolith_backend,
            timer_mode: self.reg[0][0x27],
            timer_remaining: if self.geolith_backend {
                unsafe { [ng_ymfm_timer_remaining(0), ng_ymfm_timer_remaining(1)] }
            } else {
                [-1, -1]
            },
            busy_remaining: if self.geolith_backend {
                unsafe { ng_ymfm_busy_remaining() }
            } else {
                -1
            },
            irq_asserted: self.timer_irq_pending(),
            adpcm_a_playing,
            adpcm_a_output,
            adpcm_a_addr,
            adpcm_b_playing: self.adpcm_b.playing,
            adpcm_b_output: self.adpcm_b.output,
            adpcm_b_prev_output: self.adpcm_b.prev_output,
            adpcm_b_addr: self.adpcm_b.current_addr,
            adpcm_b_start: self.adpcm_b.start_addr,
            adpcm_b_stop: self.adpcm_b.stop_addr,
            adpcm_b_delta_n: self.adpcm_b.delta_n,
            adpcm_b_volume: self.adpcm_b.volume,
        }
    }

    /// Read the status register for the given port.
    /// Port 0: bit 7 = BUSY, bit 1 = Timer B, bit 0 = Timer A
    /// Port 1: latched ADPCM end-of-sample flags masked by register 0x1C
    ///
    /// **IMPORTANT**: On real YM2610 hardware, reading the status register
    /// **clears** the timer A and timer B flags. This is how the Z80 sound
    /// driver distinguishes which timer fired and processes each ISR path.
    pub fn read_status(&self, port: u8) -> u8 {
        debug_assert!(port <= 1, "YM2610 port must be 0 or 1");
        if self.geolith_backend {
            let offset = if port == 0 { 0 } else { 2 };
            return unsafe { ng_ymfm_read(offset) };
        }
        match port {
            0 => {
                let mut status = 0u8;
                if self.busy {
                    status |= 0x80;
                }
                if self.timer_b_flag.get() {
                    status |= 0x02;
                }
                if self.timer_a_flag.get() {
                    status |= 0x01;
                }
                // Hardware clears timer flags on status read
                self.timer_b_flag.set(false);
                self.timer_a_flag.set(false);
                status
            }
            1 => self.eos_status & self.eos_mask,
            _ => 0,
        }
    }

    /// Write register address to one of the two ports.
    pub fn write_address(&mut self, port: u8, addr: u8) {
        debug_assert!(port <= 1, "YM2610 port must be 0 or 1");
        self.addr_latch[port as usize] = addr;
        if self.geolith_backend {
            let offset = if port == 0 { 0 } else { 2 };
            unsafe {
                ng_ymfm_write(offset, addr);
            }
        }
    }

    /// Write data to the currently latched register on the given port.
    pub fn write_data(&mut self, port: u8, data: u8) {
        debug_assert!(port <= 1);
        let reg = self.addr_latch[port as usize] as usize;
        self.reg[port as usize][reg] = data;
        if self.geolith_backend {
            let offset = if port == 0 { 1 } else { 3 };
            unsafe {
                ng_ymfm_write(offset, data);
            }
        }

        match (port, reg) {
            // SSG registers (port 0, 0x00-0x0F)
            (0, 0x00) => {
                self.ssg.channels[0].fine_tune = data;
            }
            (0, 0x01) => {
                self.ssg.channels[0].coarse_tune = data & 0x0F;
            }
            (0, 0x02) => {
                self.ssg.channels[1].fine_tune = data;
            }
            (0, 0x03) => {
                self.ssg.channels[1].coarse_tune = data & 0x0F;
            }
            (0, 0x04) => {
                self.ssg.channels[2].fine_tune = data;
            }
            (0, 0x05) => {
                self.ssg.channels[2].coarse_tune = data & 0x0F;
            }
            (0, 0x06) => {
                self.ssg.noise_tune = data & 0x1F;
            }
            (0, 0x07) => {
                self.ssg.tone_enable[0] = data & 0x01 == 0;
                self.ssg.tone_enable[1] = data & 0x02 == 0;
                self.ssg.tone_enable[2] = data & 0x04 == 0;
                self.ssg.noise_enable[0] = data & 0x08 == 0;
                self.ssg.noise_enable[1] = data & 0x10 == 0;
                self.ssg.noise_enable[2] = data & 0x20 == 0;
            }
            (0, 0x08) => {
                self.ssg.channels[0].volume = data & 0x1F;
            }
            (0, 0x09) => {
                self.ssg.channels[1].volume = data & 0x1F;
            }
            (0, 0x0A) => {
                self.ssg.channels[2].volume = data & 0x1F;
            }
            (0, 0x0B) => {
                self.ssg.envelope_period = (self.ssg.envelope_period & 0xFF00) | data as u16;
            }
            (0, 0x0C) => {
                self.ssg.envelope_period =
                    (self.ssg.envelope_period & 0x00FF) | ((data as u16) << 8);
            }
            (0, 0x0D) => {
                self.ssg.envelope_shape = data & 0x0F;
            }

            // ADPCM-B registers (port 0, 0x10-0x1B)
            (0, 0x10..=0x1B) => self.write_adpcm_b(reg - 0x10, data),

            // ADPCM end-of-sample flag control, matching Geolith/ymfm.
            // Written 1 bits clear latched EOS; written 0 bits keep those
            // flags visible through the high status port.
            (0, 0x1C) => {
                self.eos_mask = !data & EOS_FLAGS_MASK;
                self.eos_status &= !(data & EOS_FLAGS_MASK);
            }

            // ADPCM-A registers (port 1, 0x00-0x2F)
            (1, 0x00..=0x2F) => self.write_adpcm_a(reg, data),

            // Timer registers (port 0, 0x24-0x27)
            (0, 0x24) => {
                self.timer_a_load = (self.timer_a_load & 0x00FF) | ((data as u16) << 8);
            }
            (0, 0x25) => {
                self.timer_a_load = (self.timer_a_load & 0xFF00) | data as u16;
            }
            (0, 0x26) => {
                self.timer_b_load = data;
            }
            (0, 0x27) => {
                let prev_ta = self.timer_a_enable;
                let prev_tb = self.timer_b_enable;
                self.timer_a_enable = data & 0x01 != 0;
                self.timer_b_enable = data & 0x02 != 0;
                if data & 0x04 != 0 {
                    self.timer_a_counter = self.timer_a_load;
                }
                if data & 0x08 != 0 {
                    self.timer_b_counter = self.timer_b_load;
                }
                if data & 0x10 != 0 {
                    self.timer_a_flag.set(false);
                }
                if data & 0x20 != 0 {
                    self.timer_b_flag.set(false);
                }
                // Reset on enable transition
                if self.timer_a_enable && !prev_ta {
                    self.timer_a_counter = self.timer_a_load;
                }
                if self.timer_b_enable && !prev_tb {
                    self.timer_b_counter = self.timer_b_load;
                }
            }

            // Key-on/off (port 0, 0x28)
            (0, 0x28) => self.write_key_on_off(data),

            // FM operator registers (0x30-0x9F)
            _ => self.write_fm_register(port, reg, data),
        }
    }

    /// Read data from the currently latched register on the given port.
    pub fn read_data(&mut self, port: u8) -> u8 {
        if self.geolith_backend {
            let offset = if port == 0 { 1 } else { 3 };
            return unsafe { ng_ymfm_read(offset) };
        }
        let reg = self.addr_latch[port as usize] as usize;

        match (port, reg) {
            // SSG registers are readable
            (0, 0x00..=0x0F) => self.reg[0][reg],
            // Geolith/ymfm returns open zero for FM/ADPCM data reads on YM2610.
            _ => 0,
        }
    }

    fn write_adpcm_a(&mut self, reg: usize, data: u8) {
        match reg {
            0x00 => {
                let key_on = data & 0x80 == 0;
                for ch in 0..NUM_ADPCM_A_CHANNELS {
                    if data & (1 << ch) != 0 {
                        if key_on {
                            self.adpcm_a[ch].start_playback();
                        } else {
                            self.adpcm_a[ch].playing = false;
                            self.adpcm_a[ch].output = 0;
                        }
                    }
                }
            }
            0x01 => {
                self.adpcm_a_total_level = data & 0x3F;
            }
            0x08..=0x0D => {
                let ch = reg - 0x08;
                self.adpcm_a[ch].pan_left = data & 0x80 != 0;
                self.adpcm_a[ch].pan_right = data & 0x40 != 0;
                self.adpcm_a[ch].volume = data & 0x1F;
            }
            0x10..=0x15 => {
                let ch = reg - 0x10;
                self.adpcm_a[ch].start_addr = (self.adpcm_a[ch].start_addr & 0xFF00) | data as u32;
            }
            0x18..=0x1D => {
                let ch = reg - 0x18;
                self.adpcm_a[ch].start_addr =
                    (self.adpcm_a[ch].start_addr & 0x00FF) | ((data as u32) << 8);
            }
            0x20..=0x25 => {
                let ch = reg - 0x20;
                self.adpcm_a[ch].stop_addr = (self.adpcm_a[ch].stop_addr & 0xFF00) | data as u32;
            }
            0x28..=0x2D => {
                let ch = reg - 0x28;
                self.adpcm_a[ch].stop_addr =
                    (self.adpcm_a[ch].stop_addr & 0x00FF) | ((data as u32) << 8);
            }
            _ => {}
        }
    }

    fn write_adpcm_b(&mut self, reg: usize, data: u8) {
        match reg {
            0x00 => {
                // YM2610 forces external ROM playback mode and disables recording.
                let control = (data | 0x20) & !0x40;
                self.adpcm_b.repeat = control & 0x10 != 0;
                if control & 0x80 != 0 {
                    self.adpcm_b.start_playback();
                    self.eos_status &= !(ADPCM_B_EOS_HIDDEN | ADPCM_B_EOS_VISIBLE);
                } else {
                    self.adpcm_b.playing = false;
                    self.eos_status &= !(ADPCM_B_EOS_HIDDEN | ADPCM_B_EOS_VISIBLE);
                }
                if control & 0x01 != 0 {
                    self.adpcm_b.reset();
                }
            }
            0x01 => {
                self.adpcm_b.pan_left = data & 0x80 != 0;
                self.adpcm_b.pan_right = data & 0x40 != 0;
            }
            0x02 => {
                self.adpcm_b.start_addr = (self.adpcm_b.start_addr & 0xFF00) | data as u32;
            }
            0x03 => {
                self.adpcm_b.start_addr = (self.adpcm_b.start_addr & 0x00FF) | ((data as u32) << 8);
            }
            0x04 => {
                self.adpcm_b.stop_addr = (self.adpcm_b.stop_addr & 0xFF00) | data as u32;
            }
            0x05 => {
                self.adpcm_b.stop_addr = (self.adpcm_b.stop_addr & 0x00FF) | ((data as u32) << 8);
            }
            0x09 => {
                self.adpcm_b.delta_n = (self.adpcm_b.delta_n & 0xFF00) | data as u16;
            }
            0x0A => {
                self.adpcm_b.delta_n = (self.adpcm_b.delta_n & 0x00FF) | ((data as u16) << 8);
            }
            0x0B => {
                self.adpcm_b.volume = data;
            }
            0x0C => {
                self.adpcm_b.limit_addr = (self.adpcm_b.limit_addr & 0xFF00) | data as u32;
            }
            0x0D => {
                self.adpcm_b.limit_addr = (self.adpcm_b.limit_addr & 0x00FF) | ((data as u32) << 8);
            }
            _ => {}
        }
    }

    fn write_key_on_off(&mut self, data: u8) {
        let opmask = (data >> 4) & 0x0F;
        let Some(channel_in_group) = Self::fm_logical_channel_in_port((data & 0x03) as usize)
        else {
            return;
        };
        let channel_group = if data & 0x04 != 0 {
            OPERATORS_PER_BANK
        } else {
            0
        };
        let ch = channel_group + channel_in_group;

        if ch >= NUM_CHANNELS {
            return;
        }

        self.channels[ch].key_on = opmask != 0;
        for op in 0..NUM_OPERATORS {
            if opmask & (1 << op) != 0 {
                self.channels[ch].operators[op].eg.key_on();
            } else {
                self.channels[ch].operators[op].eg.key_off();
            }
        }
    }

    fn write_fm_register(&mut self, port: u8, reg: usize, data: u8) {
        let reg_addr = reg;

        if (0x30..=0x3F).contains(&reg_addr) {
            if let Some((ch, op)) = Self::fm_operator_target(port, reg_addr, 0x30) {
                self.channels[ch].operators[op].dt = (data >> 4) & 0x07;
                self.channels[ch].operators[op].mul = data & 0x0F;
            }
        } else if (0x40..=0x4F).contains(&reg_addr) {
            if let Some((ch, op)) = Self::fm_operator_target(port, reg_addr, 0x40) {
                self.channels[ch].operators[op].tl = data & 0x7F;
            }
        } else if (0x50..=0x5F).contains(&reg_addr) {
            if let Some((ch, op)) = Self::fm_operator_target(port, reg_addr, 0x50) {
                self.channels[ch].operators[op].ks = (data >> 6) & 0x03;
                self.channels[ch].operators[op].eg.ar = data & 0x1F;
            }
        } else if (0x60..=0x6F).contains(&reg_addr) {
            if let Some((ch, op)) = Self::fm_operator_target(port, reg_addr, 0x60) {
                self.channels[ch].operators[op].am = data & 0x80 != 0;
                self.channels[ch].operators[op].eg.dr = data & 0x1F;
            }
        } else if (0x70..=0x7F).contains(&reg_addr) {
            if let Some((ch, op)) = Self::fm_operator_target(port, reg_addr, 0x70) {
                self.channels[ch].operators[op].eg.sr = data & 0x1F;
            }
        } else if (0x80..=0x8F).contains(&reg_addr) {
            if let Some((ch, op)) = Self::fm_operator_target(port, reg_addr, 0x80) {
                self.channels[ch].operators[op].eg.sl = (data >> 4) & 0x0F;
                self.channels[ch].operators[op].eg.rr = data & 0x0F;
            }
        } else if (0x90..=0x9F).contains(&reg_addr) {
            if let Some((ch, op)) = Self::fm_operator_target(port, reg_addr, 0x90) {
                self.channels[ch].operators[op].eg.ssg_eg = data & 0x0F;
            }
        } else if (0xA0..=0xA3).contains(&reg_addr) {
            if let Some(ch) = Self::fm_channel_target(port, reg_addr, 0xA0) {
                if ch < NUM_CHANNELS {
                    self.channels[ch].fnum = (self.channels[ch].fnum & 0xFF00) | data as u16;
                    self.channels[ch].update_freq();
                }
            }
        } else if (0xA4..=0xA7).contains(&reg_addr) {
            if let Some(ch) = Self::fm_channel_target(port, reg_addr, 0xA4) {
                if ch < NUM_CHANNELS {
                    self.channels[ch].block = (data >> 3) & 0x07;
                    self.channels[ch].fnum =
                        (self.channels[ch].fnum & 0x00FF) | ((data as u16 & 0x07) << 8);
                    self.channels[ch].update_freq();
                }
            }
        } else if (0xB0..=0xB3).contains(&reg_addr) {
            if let Some(ch) = Self::fm_channel_target(port, reg_addr, 0xB0) {
                if ch < NUM_CHANNELS {
                    self.channels[ch].feedback = (data >> 3) & 0x07;
                    self.channels[ch].algo = data & 0x07;
                }
            }
        } else if (0xB4..=0xB7).contains(&reg_addr) {
            if let Some(ch) = Self::fm_channel_target(port, reg_addr, 0xB4) {
                if ch < NUM_CHANNELS {
                    self.channels[ch].pan_left = data & 0x80 != 0;
                    self.channels[ch].pan_right = data & 0x40 != 0;
                    self.channels[ch].ams = (data >> 4) & 0x03; // actually PMS is LFO freq, AMS is here
                }
            }
        } else if reg_addr == 0x22 {
            // LFO frequency (port 0, 0x22)
            self.lfo.freq = data & 0x07;
            self.lfo.am_depth = (data >> 4) & 0x03;
            self.lfo.fm_depth = data & 0x07;
        }
    }

    fn fm_channel_target(port: u8, reg_addr: usize, base: usize) -> Option<usize> {
        let offset = reg_addr.checked_sub(base)?;
        if offset >= 0x04 {
            return None;
        }
        let logical = Self::fm_logical_channel_in_port(offset & 0x03)?;
        let ch_offset = if port == 0 { 0 } else { OPERATORS_PER_BANK };
        let channel = ch_offset + logical;
        (channel < NUM_CHANNELS).then_some(channel)
    }

    fn fm_operator_target(port: u8, reg_addr: usize, base: usize) -> Option<(usize, usize)> {
        let offset = reg_addr.checked_sub(base)?;
        if offset >= 0x10 {
            return None;
        }

        // OPN operator registers are arranged as:
        // base+0/1/2 = operator 0 for OPN channel slots 0/1/2,
        // base+4/5/6 = operator 1, base+8/9/A = operator 2,
        // base+C/D/E = operator 3. YM2610 exposes four FM channels via
        // OPN slots 1/2 on each port (Geolith's FM mask is 0x36), so slot
        // 0 is intentionally ignored here.
        let channel_in_port = Self::fm_logical_channel_in_port(offset & 0x03)?;
        let operator = (offset >> 2) & 0x03;
        let ch_offset = if port == 0 { 0 } else { OPERATORS_PER_BANK };
        let channel = ch_offset + channel_in_port;
        (channel < NUM_CHANNELS).then_some((channel, operator))
    }

    fn fm_logical_channel_in_port(opn_channel_slot: usize) -> Option<usize> {
        match opn_channel_slot {
            1 => Some(0),
            2 => Some(1),
            _ => None,
        }
    }

    /// Whether any timer IRQ is pending (connected to Z80 INT line).
    /// The NeoGeo sound driver polls this to drive the music sequencer.
    pub fn timer_irq_pending(&self) -> bool {
        if self.geolith_backend {
            return unsafe { ng_ymfm_irq_asserted() != 0 };
        }
        (self.timer_a_enable && self.timer_a_flag.get())
            || (self.timer_b_enable && self.timer_b_flag.get())
    }

    /// Serialize YM2610 state for save states.
    /// Returns a compact binary representation of all registers and internal state.
    pub fn save_state(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(1024);
        // reg: [[u8; 256]; 2]
        buf.extend_from_slice(&self.reg[0]);
        buf.extend_from_slice(&self.reg[1]);
        // addr_latch
        buf.push(self.addr_latch[0]);
        buf.push(self.addr_latch[1]);
        // Timers
        buf.extend_from_slice(&self.timer_a_counter.to_le_bytes());
        buf.push(self.timer_b_counter);
        buf.extend_from_slice(&self.timer_a_load.to_le_bytes());
        buf.push(self.timer_b_load);
        // Flags
        buf.push(self.timer_a_enable as u8);
        buf.push(self.timer_b_enable as u8);
        buf.push(self.timer_a_flag.get() as u8);
        buf.push(self.timer_b_flag.get() as u8);
        buf.extend_from_slice(&self.timer_b_fractional.to_le_bytes());
        buf.push(self.busy as u8);
        buf.push(self.eos_status);
        buf.push(self.eos_mask);
        // Sample count
        buf.extend_from_slice(&self.sample_count.to_le_bytes());
        // ADPCM-A channels: playing + current_addr + nibble_pos + decoder
        for ch in &self.adpcm_a {
            buf.push(ch.playing as u8);
            buf.extend_from_slice(&ch.current_addr.to_le_bytes());
            buf.push(ch.nibble_pos);
            buf.extend_from_slice(&ch.decoder.output().to_le_bytes());
            buf.push(ch.decoder.step_index);
        }
        // ADPCM-B
        buf.push(self.adpcm_b.playing as u8);
        buf.extend_from_slice(&self.adpcm_b.current_addr.to_le_bytes());
        buf.push(self.adpcm_b.nibble_pos);
        buf.extend_from_slice(&self.adpcm_b.decoder.output().to_le_bytes());
        buf.push(0);
        buf.extend_from_slice(&self.adpcm_b.position.to_le_bytes());
        buf.extend_from_slice(&self.adpcm_b.prev_output.to_le_bytes());
        buf.extend_from_slice(&self.adpcm_b.decoder.step.to_le_bytes());
        if self.geolith_backend {
            let mut geolith_state = vec![0u8; GEOLITH_YMFM_STATE_CAPACITY];
            let state_len =
                unsafe { ng_ymfm_state_save(geolith_state.as_mut_ptr(), geolith_state.len()) };
            geolith_state.truncate(state_len.min(GEOLITH_YMFM_STATE_CAPACITY));
            buf.extend_from_slice(&GEOLITH_YMFM_STATE_MAGIC);
            buf.extend_from_slice(&(geolith_state.len() as u32).to_le_bytes());
            buf.extend_from_slice(&geolith_state);
        }
        buf
    }

    /// Deserialize YM2610 state from save state data.
    /// Returns Ok(()) on success, Err with description on failure.
    pub fn load_state(&mut self, data: &[u8]) -> Result<(), &'static str> {
        if data.len() < 600 {
            return Err("YM2610: datos insuficientes");
        }
        let mut off = 0;
        self.reg[0].copy_from_slice(&data[off..off + 256]);
        off += 256;
        self.reg[1].copy_from_slice(&data[off..off + 256]);
        off += 256;
        self.addr_latch[0] = data[off];
        off += 1;
        self.addr_latch[1] = data[off];
        off += 1;
        self.timer_a_counter = u16::from_le_bytes(data[off..off + 2].try_into().unwrap());
        off += 2;
        self.timer_b_counter = data[off];
        off += 1;
        self.timer_a_load = u16::from_le_bytes(data[off..off + 2].try_into().unwrap());
        off += 2;
        self.timer_b_load = data[off];
        off += 1;
        self.timer_a_enable = data[off] != 0;
        off += 1;
        self.timer_b_enable = data[off] != 0;
        off += 1;
        self.timer_a_flag.set(data[off] != 0);
        off += 1;
        self.timer_b_flag.set(data[off] != 0);
        off += 1;
        self.timer_b_fractional = f32::from_le_bytes(data[off..off + 4].try_into().unwrap());
        off += 4;
        self.busy = data[off] != 0;
        off += 1;
        if data.len() >= 608 {
            self.eos_status = data[off];
            off += 1;
            self.eos_mask = data[off];
            off += 1;
        } else {
            self.eos_status = 0;
            self.eos_mask = EOS_FLAGS_MASK;
        }
        self.sample_count = u64::from_le_bytes(data[off..off + 8].try_into().unwrap());
        off += 8;
        for ch in &mut self.adpcm_a {
            ch.playing = data[off] != 0;
            off += 1;
            ch.current_addr = u32::from_le_bytes(data[off..off + 4].try_into().unwrap());
            off += 4;
            ch.nibble_pos = data[off];
            off += 1;
            let output = i16::from_le_bytes(data[off..off + 2].try_into().unwrap());
            off += 2;
            let step_index = data[off];
            off += 1;
            ch.decoder.restore_from_output(output, step_index);
            ch.output = output;
        }
        self.adpcm_b.playing = data[off] != 0;
        off += 1;
        self.adpcm_b.current_addr = u32::from_le_bytes(data[off..off + 4].try_into().unwrap());
        off += 4;
        self.adpcm_b.nibble_pos = data[off];
        off += 1;
        let adpcm_b_output = i16::from_le_bytes(data[off..off + 2].try_into().unwrap());
        off += 2;
        let _legacy_step_index = data[off];
        off += 1;
        self.adpcm_b.decoder.restore_from_output(adpcm_b_output);
        self.adpcm_b.output = adpcm_b_output;
        self.adpcm_b.position = if data.len() >= off + 4 {
            let value = u32::from_le_bytes(data[off..off + 4].try_into().unwrap());
            off += 4;
            value
        } else {
            0
        };
        self.adpcm_b.prev_output = if data.len() >= off + 2 {
            let value = i16::from_le_bytes(data[off..off + 2].try_into().unwrap());
            off += 2;
            value
        } else {
            self.adpcm_b.output
        };
        self.adpcm_b.decoder.step = if data.len() >= off + 4 {
            i32::from_le_bytes(data[off..off + 4].try_into().unwrap())
                .clamp(ADPCM_B_STEP_MIN, ADPCM_B_STEP_MAX)
        } else {
            ADPCM_B_STEP_MIN
        };
        self.adpcm_b.repeat = self.reg[0][0x10] & 0x10 != 0;
        self.adpcm_b.limit_addr = 0xFFFF;
        if self.geolith_backend
            && data.len() >= off + 8
            && data[off..off + 4] == GEOLITH_YMFM_STATE_MAGIC
        {
            off += 4;
            let geolith_len = u32::from_le_bytes(data[off..off + 4].try_into().unwrap()) as usize;
            off += 4;
            if data.len() < off + geolith_len {
                return Err("YM2610: estado Geolith incompleto");
            }
            unsafe {
                ng_ymfm_state_load(data[off..off + geolith_len].as_ptr(), geolith_len);
                ng_ymfm_adpcm_wrap(1);
            }
        } else if self.geolith_backend {
            unsafe {
                ng_ymfm_reset();
                ng_ymfm_adpcm_wrap(1);
            }
        }
        Ok(())
    }

    /// Generate `count` stereo samples into the provided buffer.
    /// Buffer must be at least `count * 2` elements long.
    pub fn generate(&mut self, buffer: &mut [i16], count: usize) {
        if self.geolith_backend {
            self.update_geolith_roms();
            let writable_pairs = count.min(buffer.len() / 2);
            if writable_pairs > 0 {
                unsafe {
                    ng_ymfm_generate(buffer.as_mut_ptr(), writable_pairs);
                }
            }
            return;
        }

        for i in 0..count {
            // Clock LFO
            let (lfo_am, lfo_fm) = self.lfo.clock();

            // Clock timers
            self.clock_timers();

            // Mix FM + ADPCM first, then add SSG with saturation. This
            // mirrors Geolith's OPNB path more closely than globally
            // attenuating the final buffer, and keeps short ADPCM effects
            // from being buried under FM music.
            let mut fm_adpcm_left = 0i32;
            let mut fm_adpcm_right = 0i32;

            for ch in &mut self.channels {
                let (l, r) = ch.clock(lfo_am, lfo_fm);
                fm_adpcm_left += l;
                fm_adpcm_right += r;
            }

            // Mix SSG as Geolith does: the YM2610 exposes it as a mono
            // third bus with logarithmic volume, then it is clipped into
            // both stereo FM/ADPCM channels.
            let ssg_mono = self.ssg.clock();

            // Mix ADPCM
            let (adpcm_l, adpcm_r) = self.clock_adpcm();
            fm_adpcm_left += adpcm_l as i32;
            fm_adpcm_right += adpcm_r as i32;

            let mixed_l = fm_adpcm_left
                .clamp(i16::MIN as i32, i16::MAX as i32)
                .saturating_add(ssg_mono)
                .clamp(i16::MIN as i32, i16::MAX as i32);
            let mixed_r = fm_adpcm_right
                .clamp(i16::MIN as i32, i16::MAX as i32)
                .saturating_add(ssg_mono)
                .clamp(i16::MIN as i32, i16::MAX as i32);
            let out_l = (mixed_l >> YM2610_OUTPUT_GAIN_SHIFT) as i16;
            let out_r = (mixed_r >> YM2610_OUTPUT_GAIN_SHIFT) as i16;

            let idx = i * 2;
            if idx + 1 < buffer.len() {
                buffer[idx] = out_l;
                buffer[idx + 1] = out_r;
            }
        }
    }

    fn clock_timers(&mut self) {
        // Timer A: 10-bit counter at master clock / 2
        // Timer B: 8-bit counter at master clock / 2 / 16
        // Multiple timer ticks occur per output sample.
        self.sample_count += 1;

        // Each sample represents ~144 master clock cycles.
        // Timer A ticks every 2 master clocks → ~72 ticks per sample.
        // Timer B ticks every 32 master clocks → ~4.5 ticks per sample.
        const TIMER_A_TICKS_PER_SAMPLE: u32 = YM2610_MASTER_CLOCK / 2 / YM2610_SAMPLE_RATE;
        const TIMER_B_TICKS_PER_SAMPLE: u32 = YM2610_MASTER_CLOCK / 2 / 16 / YM2610_SAMPLE_RATE;

        // Timer A
        if self.timer_a_enable {
            for _ in 0..TIMER_A_TICKS_PER_SAMPLE {
                self.timer_a_counter = self.timer_a_counter.wrapping_sub(1);
                if self.timer_a_counter == 0xFFFF {
                    self.timer_a_flag.set(true);
                    self.timer_a_counter = self.timer_a_load;
                }
            }
        }

        // Timer B
        if self.timer_b_enable {
            // Fractional accumulation for timer B (4.5 ticks per sample)
            self.timer_b_fractional += TIMER_B_TICKS_PER_SAMPLE as f32;
            while self.timer_b_fractional >= 1.0 {
                self.timer_b_fractional -= 1.0;
                self.timer_b_counter = self.timer_b_counter.wrapping_sub(1);
                if self.timer_b_counter == 0xFF {
                    self.timer_b_flag.set(true);
                    self.timer_b_counter = self.timer_b_load;
                }
            }
        }
    }

    fn clock_adpcm(&mut self) -> (i16, i16) {
        let mut left = 0i32;
        let mut right = 0i32;

        // ADPCM-A channels
        for (ch_index, ch) in self.adpcm_a.iter_mut().enumerate() {
            if !ch.playing {
                continue;
            }

            // ADPCM-A is fixed-rate (~18.5 kHz), about one nibble for every
            // three YM2610 output samples at the medium-fidelity 55.5 kHz rate.
            let divisor = 3;
            if self.sample_count.is_multiple_of(divisor) {
                if ch.nibble_pos == 0 {
                    let end = (ch.stop_addr + 1) << 8;
                    if ((ch.current_addr ^ end) & 0x0F_FFFF) == 0 {
                        ch.playing = false;
                        ch.output = 0;
                        self.eos_status |= 1 << ch_index;
                        continue;
                    }
                }

                let addr = ch.current_addr as usize;
                let byte = {
                    let vrom = self.vrom.borrow();
                    let adpcm_a_len = if vrom.vrom_b_offset == 0 {
                        vrom.vrom.len()
                    } else {
                        vrom.vrom_b_offset
                    };
                    if addr < adpcm_a_len {
                        vrom.vrom.get(addr).copied().unwrap_or(0)
                    } else {
                        0
                    }
                };
                let nibble = if ch.nibble_pos == 0 {
                    byte >> 4
                } else {
                    byte & 0x0F
                };
                ch.output = ch.decoder.decode_nibble(nibble);

                ch.nibble_pos ^= 1;
                if ch.nibble_pos == 0 {
                    ch.current_addr += 1;
                }
            }

            let attenuation = (ch.volume ^ 0x1F) as i32 + (self.adpcm_a_total_level ^ 0x3F) as i32;
            if attenuation >= 63 {
                continue;
            }
            let mul = 15 - (attenuation & 7);
            let shift = 5 + (attenuation >> 3);
            let sample = ((ch.output as i32 * mul) >> shift) & !3;
            if ch.pan_left {
                left += sample;
            }
            if ch.pan_right {
                right += sample;
            }
        }

        // ADPCM-B (variable pitch). The YM2610 advances the channel by
        // accumulating Delta-N and decoding a nibble whenever it overflows
        // 16 bits. Larger Delta-N therefore means faster playback.
        if self.adpcm_b.playing {
            self.adpcm_b.position += self.adpcm_b.delta_n as u32;
            while self.adpcm_b.position >= 0x10000 && self.adpcm_b.playing {
                self.adpcm_b.position -= 0x10000;

                let byte = {
                    let vrom = self.vrom.borrow();
                    let addr = vrom
                        .vrom_b_offset
                        .saturating_add(self.adpcm_b.current_addr as usize);
                    vrom.vrom.get(addr).copied().unwrap_or(0)
                };
                let nibble = if self.adpcm_b.nibble_pos == 0 {
                    byte >> 4
                } else {
                    byte & 0x0F
                };
                self.adpcm_b.prev_output = self.adpcm_b.output;
                self.adpcm_b.output = self.adpcm_b.decoder.decode_nibble(nibble);

                self.adpcm_b.nibble_pos ^= 1;
                if self.adpcm_b.nibble_pos == 0 {
                    let end = ((self.adpcm_b.stop_addr + 1) << 8).saturating_sub(1);
                    let limit = ((self.adpcm_b.limit_addr + 1) << 8).saturating_sub(1);
                    if self.adpcm_b.current_addr == end {
                        if self.adpcm_b.repeat {
                            self.adpcm_b.start_playback();
                        } else {
                            self.adpcm_b.playing = false;
                            self.adpcm_b.output = 0;
                            self.adpcm_b.prev_output = 0;
                            self.eos_status = (self.eos_status
                                & !(ADPCM_B_EOS_HIDDEN | ADPCM_B_EOS_VISIBLE))
                                | ADPCM_B_EOS_HIDDEN
                                | ADPCM_B_EOS_VISIBLE;
                        }
                    } else if self.adpcm_b.current_addr == limit {
                        self.adpcm_b.current_addr = 0;
                    } else {
                        self.adpcm_b.current_addr = (self.adpcm_b.current_addr + 1) & 0x00FF_FFFF;
                    }
                }
            }

            let interp = if self.adpcm_b.delta_n == 0 {
                self.adpcm_b.output as i32
            } else {
                let pos = self.adpcm_b.position.min(0xFFFF) as i32;
                let prev = self.adpcm_b.prev_output as i32;
                let cur = self.adpcm_b.output as i32;
                (prev * (0x10000 - pos) + cur * pos) >> 16
            };
            let vol = self.adpcm_b.volume as i32;
            let sample = (interp * vol / 256) / 2;
            if self.adpcm_b.pan_left {
                left += sample;
            }
            if self.adpcm_b.pan_right {
                right += sample;
            }
        }

        (
            left.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
            right.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_test_ym2610() -> Ym2610 {
        let mem = Rc::new(RefCell::new(crate::memory::Memory::new()));
        Ym2610::new(mem)
    }

    #[test]
    fn ym2610_register_write_read() {
        let mut chip = new_test_ym2610();

        // Write to port 0: address = 0x30, data = 0xAB
        chip.write_address(0, 0x30);
        chip.write_data(0, 0xAB);

        assert_eq!(chip.reg[0][0x30], 0xAB);
    }

    #[test]
    fn ym2610_port_0_and_1_are_independent() {
        let mut chip = new_test_ym2610();

        chip.write_address(0, 0x40);
        chip.write_data(0, 0x11);

        chip.write_address(1, 0x40);
        chip.write_data(1, 0x22);

        assert_eq!(chip.reg[0][0x40], 0x11);
        assert_eq!(chip.reg[1][0x40], 0x22);
    }

    #[test]
    fn ym2610_ssg_registers_affect_channels() {
        let mut chip = new_test_ym2610();

        // Set SSG channel A fine tune
        chip.write_address(0, 0x00);
        chip.write_data(0, 0x55);

        assert_eq!(chip.ssg.channels[0].fine_tune, 0x55);
    }

    #[test]
    fn ym2610_timer_a_load_and_enable() {
        let mut chip = new_test_ym2610();

        // Set timer A load = 0x1234
        chip.write_address(0, 0x24);
        chip.write_data(0, 0x12); // high byte
        chip.write_address(0, 0x25);
        chip.write_data(0, 0x34); // low byte

        // Enable timer A + load
        chip.write_address(0, 0x27);
        chip.write_data(0, 0x05); // enable TA + load TA

        assert_eq!(chip.timer_a_load, 0x1234);
        assert!(chip.timer_a_enable);
        assert_eq!(chip.timer_a_counter, 0x1234);
    }

    #[test]
    fn ym2610_fm_operator_dt_mul_write() {
        let mut chip = new_test_ym2610();

        // Write DT/MUL for first active FM channel, operator 1
        chip.write_address(0, 0x31);
        chip.write_data(0, 0x3F); // DT=3, MUL=15

        assert_eq!(chip.channels[0].operators[0].dt, 3);
        assert_eq!(chip.channels[0].operators[0].mul, 15);
    }

    #[test]
    fn ym2610_fm_total_level_write() {
        let mut chip = new_test_ym2610();

        // Write TL for first active FM channel, operator 1
        chip.write_address(0, 0x41);
        chip.write_data(0, 0x2A);

        assert_eq!(chip.channels[0].operators[0].tl, 0x2A);
    }

    #[test]
    fn ym2610_fm_operator_registers_are_channel_then_slot() {
        let mut chip = new_test_ym2610();

        for (reg, expected_tl) in [(0x41, 0x10), (0x45, 0x20), (0x49, 0x30), (0x4D, 0x40)] {
            chip.write_address(0, reg);
            chip.write_data(0, expected_tl);
        }

        assert_eq!(chip.channels[0].operators[0].tl, 0x10);
        assert_eq!(chip.channels[0].operators[1].tl, 0x20);
        assert_eq!(chip.channels[0].operators[2].tl, 0x30);
        assert_eq!(chip.channels[0].operators[3].tl, 0x40);
        assert_eq!(
            chip.channels[1].operators[0].tl, 0,
            "operator slots for channel 0 must not spill into channel 1"
        );

        chip.write_address(0, 0x42);
        chip.write_data(0, 0x22);
        chip.write_address(1, 0x46);
        chip.write_data(1, 0x33);

        assert_eq!(chip.channels[1].operators[0].tl, 0x22);
        assert_eq!(chip.channels[3].operators[1].tl, 0x33);
    }

    #[test]
    fn ym2610_fm_algorithm_and_feedback() {
        let mut chip = new_test_ym2610();

        // Write algo/feedback for first active FM channel
        chip.write_address(0, 0xB1);
        chip.write_data(0, 0x3D); // feedback=7, algo=5

        assert_eq!(chip.channels[0].feedback, 7);
        assert_eq!(chip.channels[0].algo, 5);
    }

    #[test]
    fn ym2610_pan_write() {
        let mut chip = new_test_ym2610();

        // Write pan for first active FM channel
        chip.write_address(0, 0xB5);
        chip.write_data(0, 0x80); // Left only

        assert!(chip.channels[0].pan_left);
        assert!(!chip.channels[0].pan_right);
    }

    #[test]
    fn ym2610_key_on() {
        let mut chip = new_test_ym2610();

        // Key on channel 1 (slot bits all set)
        chip.write_address(0, 0x28);
        chip.write_data(0, 0xF1); // slot 0x0F, first active FM channel

        assert!(chip.channels[0].key_on);
        for op in &chip.channels[0].operators {
            assert!(op.eg.key_on);
        }
    }

    #[test]
    fn ym2610_key_off() {
        let mut chip = new_test_ym2610();

        // First key on
        chip.write_address(0, 0x28);
        chip.write_data(0, 0xF1);
        assert!(chip.channels[0].key_on);

        // Then key off
        chip.write_address(0, 0x28);
        chip.write_data(0, 0x01); // slot 0, first active FM channel
        assert!(!chip.channels[0].key_on);
        for op in &chip.channels[0].operators {
            assert!(!op.eg.key_on);
        }
    }

    #[test]
    fn ym2610_lfo_settings() {
        let mut chip = new_test_ym2610();

        // Write LFO frequency (port 0, reg 0x22)
        chip.write_address(0, 0x22);
        chip.write_data(0, 0x75); // freq=5, am_depth=3, fm_depth=5

        assert_eq!(chip.lfo.freq, 5);
        assert_eq!(chip.lfo.am_depth, 3);
        assert_eq!(chip.lfo.fm_depth, 5);
    }

    #[test]
    fn ym2610_adpcm_a_start_stop() {
        let mut chip = new_test_ym2610();

        // ADPCM-A ch0: start = 0x1234, stop = 0x5678
        chip.write_address(1, 0x10);
        chip.write_data(1, 0x34);
        chip.write_address(1, 0x18);
        chip.write_data(1, 0x12);
        chip.write_address(1, 0x20);
        chip.write_data(1, 0x78);
        chip.write_address(1, 0x28);
        chip.write_data(1, 0x56);

        assert_eq!(chip.adpcm_a[0].start_addr, 0x1234);
        assert_eq!(chip.adpcm_a[0].stop_addr, 0x5678);
    }

    #[test]
    fn ym2610_adpcm_control_starts_playback() {
        let mut chip = new_test_ym2610();

        // Start ADPCM-A ch0 (port 1, reg 0x00)
        chip.write_address(1, 0x00);
        chip.write_data(1, 0x01);

        assert!(chip.adpcm_a[0].playing);
    }

    #[test]
    fn ym2610_adpcm_a_generates_nonzero_samples() {
        let mut chip = new_test_ym2610();
        chip.vrom.borrow_mut().vrom = vec![0x11; 0x400];

        chip.write_address(1, 0x01); // total level: loudest
        chip.write_data(1, 0x3F);
        chip.write_address(1, 0x08); // ch0: left + right, loudest instrument level
        chip.write_data(1, 0xDF);
        chip.write_address(1, 0x10); // start low
        chip.write_data(1, 0x00);
        chip.write_address(1, 0x18); // start high
        chip.write_data(1, 0x00);
        chip.write_address(1, 0x20); // end low
        chip.write_data(1, 0x03);
        chip.write_address(1, 0x28); // end high
        chip.write_data(1, 0x00);
        chip.write_address(1, 0x00); // key on ch0
        chip.write_data(1, 0x01);

        let mut buffer = vec![0i16; 256];
        chip.generate(&mut buffer, 128);

        assert!(buffer.iter().any(|&sample| sample != 0));
    }

    #[test]
    fn ym2610_adpcm_a_sets_and_clears_eos_latch() {
        let mut chip = new_test_ym2610();
        chip.vrom.borrow_mut().vrom = vec![0x11; 0x200];

        chip.write_address(1, 0x01);
        chip.write_data(1, 0x3F);
        chip.write_address(1, 0x08);
        chip.write_data(1, 0xDF);
        chip.write_address(1, 0x10);
        chip.write_data(1, 0x00);
        chip.write_address(1, 0x18);
        chip.write_data(1, 0x00);
        chip.write_address(1, 0x20);
        chip.write_data(1, 0x00);
        chip.write_address(1, 0x28);
        chip.write_data(1, 0x00);
        chip.write_address(1, 0x00);
        chip.write_data(1, 0x01);

        let mut buffer = vec![0i16; 3400];
        chip.generate(&mut buffer, 1700);

        assert_eq!(chip.read_status(1) & 0x01, 0x01);

        chip.write_address(0, 0x1C);
        chip.write_data(0, 0x01);

        assert_eq!(chip.read_status(1) & 0x01, 0x00);
    }

    #[test]
    fn ym2610_adpcm_b_delta_n_advances_playback() {
        let mut chip = new_test_ym2610();
        chip.vrom.borrow_mut().vrom = vec![0x11; 0x800];

        chip.write_address(0, 0x11); // pan L+R
        chip.write_data(0, 0xC0);
        chip.write_address(0, 0x12); // start low
        chip.write_data(0, 0x00);
        chip.write_address(0, 0x13); // start high
        chip.write_data(0, 0x00);
        chip.write_address(0, 0x14); // end low
        chip.write_data(0, 0x03);
        chip.write_address(0, 0x15); // end high
        chip.write_data(0, 0x00);
        chip.write_address(0, 0x19); // delta low
        chip.write_data(0, 0x00);
        chip.write_address(0, 0x1A); // delta high: one nibble every 2 output samples
        chip.write_data(0, 0x80);
        chip.write_address(0, 0x1B); // level
        chip.write_data(0, 0xFF);
        chip.write_address(0, 0x10); // execute, external ROM mode is forced internally
        chip.write_data(0, 0x80);

        let mut buffer = vec![0i16; 64];
        chip.generate(&mut buffer, 32);

        assert!(
            chip.adpcm_b.current_addr > 0,
            "ADPCM-B should advance through V-ROM when Delta-N overflows"
        );
        assert!(
            buffer.iter().any(|&sample| sample != 0),
            "ADPCM-B should produce audible output"
        );
    }

    #[test]
    fn ym2610_adpcm_b_reads_from_v2_offset_when_present() {
        let mut chip = new_test_ym2610();
        {
            let mut mem = chip.vrom.borrow_mut();
            mem.vrom = vec![0x00; 0x100];
            mem.vrom[0x80] = 0x77;
            mem.vrom_b_offset = 0x80;
        }

        chip.write_address(0, 0x11);
        chip.write_data(0, 0xC0);
        chip.write_address(0, 0x12);
        chip.write_data(0, 0x00);
        chip.write_address(0, 0x13);
        chip.write_data(0, 0x00);
        chip.write_address(0, 0x14);
        chip.write_data(0, 0x01);
        chip.write_address(0, 0x15);
        chip.write_data(0, 0x00);
        chip.write_address(0, 0x19);
        chip.write_data(0, 0xFF);
        chip.write_address(0, 0x1A);
        chip.write_data(0, 0xFF);
        chip.write_address(0, 0x1B);
        chip.write_data(0, 0xFF);
        chip.write_address(0, 0x10);
        chip.write_data(0, 0x80);

        let mut buffer = vec![0i16; 4];
        chip.generate(&mut buffer, 2);

        assert!(
            chip.adpcm_b.output > 100,
            "ADPCM-B should fetch from V2 offset, not from V1 offset 0"
        );
    }

    #[test]
    fn ym2610_adpcm_a_does_not_read_into_v2_region() {
        let mut chip = new_test_ym2610();
        {
            let mut mem = chip.vrom.borrow_mut();
            mem.vrom = vec![0x00; 0x100];
            mem.vrom[0x80] = 0x88;
            mem.vrom_b_offset = 0x80;
        }

        chip.adpcm_a_total_level = 0x3F;
        chip.adpcm_a[0].playing = true;
        chip.adpcm_a[0].current_addr = 0x80;
        chip.adpcm_a[0].stop_addr = 0x01;
        chip.adpcm_a[0].volume = 0x1F;
        chip.adpcm_a[0].pan_left = true;
        chip.adpcm_a[0].pan_right = false;

        let (left, right) = chip.clock_adpcm();
        assert!(
            left > 0,
            "ADPCM-A should read zero past V1, not the negative V2 byte"
        );
        assert_eq!(right, 0);
    }

    #[test]
    fn ym2610_adpcm_mix_saturates_instead_of_wrapping() {
        let mut chip = new_test_ym2610();
        chip.sample_count = 1;
        chip.adpcm_a_total_level = 0x3F;
        for ch in &mut chip.adpcm_a {
            ch.playing = true;
            ch.volume = 0x1F;
            ch.pan_left = true;
            ch.pan_right = true;
            ch.output = i16::MAX;
        }

        let (left, right) = chip.clock_adpcm();

        assert_eq!(left, i16::MAX);
        assert_eq!(right, i16::MAX);
    }

    #[test]
    fn ym2610_adpcm_b_status_latches_eos_instead_of_playing() {
        let mut chip = new_test_ym2610();
        chip.vrom.borrow_mut().vrom = vec![0x11; 0x200];

        chip.write_address(0, 0x11);
        chip.write_data(0, 0xC0);
        chip.write_address(0, 0x12);
        chip.write_data(0, 0x00);
        chip.write_address(0, 0x13);
        chip.write_data(0, 0x00);
        chip.write_address(0, 0x14);
        chip.write_data(0, 0x00);
        chip.write_address(0, 0x15);
        chip.write_data(0, 0x00);
        chip.write_address(0, 0x19);
        chip.write_data(0, 0xFF);
        chip.write_address(0, 0x1A);
        chip.write_data(0, 0xFF);
        chip.write_address(0, 0x1B);
        chip.write_data(0, 0xFF);
        chip.write_address(0, 0x10);
        chip.write_data(0, 0x80);

        assert_eq!(
            chip.read_status(1) & ADPCM_B_EOS_VISIBLE,
            0,
            "high status reports EOS, not the live playing bit"
        );

        let mut buffer = vec![0i16; 1400];
        chip.generate(&mut buffer, 700);

        assert_eq!(
            chip.read_status(1) & ADPCM_B_EOS_VISIBLE,
            ADPCM_B_EOS_VISIBLE
        );

        chip.write_address(0, 0x1C);
        chip.write_data(0, ADPCM_B_EOS_VISIBLE);

        assert_eq!(chip.read_status(1) & ADPCM_B_EOS_VISIBLE, 0);
    }

    #[test]
    fn ym2610_adpcm_b_repeat_restarts_at_end() {
        let mut chip = new_test_ym2610();
        chip.vrom.borrow_mut().vrom = vec![0x11; 0x200];

        chip.write_address(0, 0x11);
        chip.write_data(0, 0xC0);
        chip.write_address(0, 0x12);
        chip.write_data(0, 0x00);
        chip.write_address(0, 0x13);
        chip.write_data(0, 0x00);
        chip.write_address(0, 0x14);
        chip.write_data(0, 0x00);
        chip.write_address(0, 0x15);
        chip.write_data(0, 0x00);
        chip.write_address(0, 0x19);
        chip.write_data(0, 0xFF);
        chip.write_address(0, 0x1A);
        chip.write_data(0, 0xFF);
        chip.write_address(0, 0x1B);
        chip.write_data(0, 0xFF);
        chip.write_address(0, 0x10);
        chip.write_data(0, 0x90);

        let mut buffer = vec![0i16; 1600];
        chip.generate(&mut buffer, 800);

        assert!(chip.adpcm_b.playing, "ADPCM-B repeat should keep playing");
        assert_eq!(
            chip.read_status(1) & ADPCM_B_EOS_VISIBLE,
            0,
            "repeat should not latch EOS"
        );
    }

    #[test]
    fn ym2610_status_register() {
        let chip = new_test_ym2610();

        // Initial status
        let status = chip.read_status(0);
        assert_eq!(status & 0x03, 0); // No timer flags

        // Set timer A flag
        chip.timer_a_flag.set(true);
        let status = chip.read_status(0);
        assert!(status & 0x01 != 0);
    }

    #[test]
    fn geolith_busy_period_restarts_instead_of_accumulating() {
        let mut chip = new_test_ym2610();
        chip.enable_geolith_backend();

        for value in 0..=0xff {
            chip.write_address(0, 0x30);
            chip.write_data(0, value);
        }

        assert_ne!(
            chip.read_status(0) & 0x80,
            0,
            "last data write should leave YM2610 briefly busy"
        );

        let mut buffer = [0i16; 4];
        chip.generate(&mut buffer, 2);

        assert_eq!(
            chip.read_status(0) & 0x80,
            0,
            "busy duration must be based on the latest write, not all prior writes"
        );
    }

    #[test]
    fn geolith_timer_b_sets_timer_b_status_bit() {
        let mut chip = new_test_ym2610();
        chip.enable_geolith_backend();

        chip.write_address(0, 0x26);
        chip.write_data(0, 0xE7);
        chip.write_address(0, 0x27);
        chip.write_data(0, 0x0A);

        let mut buffer = vec![0i16; 1_200];
        chip.generate(&mut buffer, 600);

        assert_eq!(
            chip.read_status(0) & 0x03,
            0x02,
            "Digger Man's Timer B polling loop must observe Timer B, not Timer A"
        );
    }

    #[test]
    fn ym2610_reset_clears_all_state() {
        let mut chip = new_test_ym2610();

        // Set some state
        chip.write_address(0, 0x30);
        chip.write_data(0, 0xFF);
        chip.timer_a_enable = true;
        chip.write_address(0, 0x28);
        chip.write_data(0, 0xF1);

        // Reset
        chip.reset();

        assert!(!chip.channels[0].key_on);
        assert!(!chip.timer_a_enable);
        assert_eq!(chip.reg[0][0x30], 0);
    }

    #[test]
    fn ym2610_generate_produces_stereo_samples() {
        let mut chip = new_test_ym2610();
        let count = 16;
        let mut buffer = vec![0i16; count * 2];

        // Should not panic even with no registers configured
        chip.generate(&mut buffer, count);

        // All zeros expected with no key-on channels
        for s in &buffer {
            assert_eq!(*s, 0);
        }
    }

    #[test]
    fn ym2610_generate_with_channel_active() {
        let mut chip = new_test_ym2610();

        // Configure channel 1 with a simple tone
        // Set algorithm, feedback, pan
        chip.write_address(0, 0xB1);
        chip.write_data(0, 0x00); // algo 0, fb 0
        chip.write_address(0, 0xB5);
        chip.write_data(0, 0xC0); // L+R

        // Set frequency
        chip.write_address(0, 0xA1);
        chip.write_data(0, 0x69); // F-num low
        chip.write_address(0, 0xA5);
        chip.write_data(0, 0x24); // block=4, F-num high=4

        // Set operator TLs
        for reg in [0x41, 0x45, 0x49, 0x4D] {
            chip.write_address(0, reg);
            chip.write_data(0, 0x20); // moderate volume
        }

        // Set operator MULs
        for reg in [0x31, 0x35, 0x39, 0x3D] {
            chip.write_address(0, reg);
            chip.write_data(0, 0x01); // MUL=1
        }

        // Set envelope rates (fast attack, long sustain)
        for reg in [0x51, 0x55, 0x59, 0x5D] {
            chip.write_address(0, reg);
            chip.write_data(0, 0x1F); // AR=31 (fast)
        }
        for reg in [0x61, 0x65, 0x69, 0x6D] {
            chip.write_address(0, reg);
            chip.write_data(0, 0x00); // DR=0 (no decay)
        }
        for reg in [0x81, 0x85, 0x89, 0x8D] {
            chip.write_address(0, reg);
            chip.write_data(0, 0x00); // SL=0, RR=0
        }

        // Key on
        chip.write_address(0, 0x28);
        chip.write_data(0, 0xF1);

        let count = 64;
        let mut buffer = vec![0i16; count * 2];
        chip.generate(&mut buffer, count);

        // Some samples should be non-zero due to active FM channel
        let has_sound = buffer.iter().any(|&s| s != 0);
        assert!(
            has_sound,
            "Expected non-zero audio output with active FM channel"
        );
    }

    #[test]
    fn ym2610_read_write_roundtrip() {
        let mut chip = new_test_ym2610();

        // Write then read SSG register
        chip.write_address(0, 0x08);
        chip.write_data(0, 0x0A);

        chip.write_address(0, 0x08);
        assert_eq!(chip.read_data(0), 0x0A);
    }

    #[test]
    fn adpcm_a_decoder_roundtrip() {
        let mut decoder = AdpcmADecoder::new();

        // Decode a sequence of nibbles
        let nibbles = [0x0, 0x8, 0x2, 0xA, 0x4, 0xC, 0x6, 0xE];
        let mut samples = Vec::new();
        for &n in &nibbles {
            samples.push(decoder.decode_nibble(n));
        }

        // Samples should change from initial 0
        assert!(samples.iter().any(|&s| s != 0));
    }

    #[test]
    fn adpcm_b_decoder_uses_variable_step_scale() {
        let mut decoder = AdpcmBDecoder::new();

        let first = decoder.decode_nibble(0x7);
        let step_after_large_delta = decoder.step;
        let second = decoder.decode_nibble(0x0);

        assert_ne!(first, 0);
        assert_ne!(second, first);
        assert!(
            step_after_large_delta > ADPCM_B_STEP_MIN,
            "ADPCM-B must use its YM2610 step scaler, not ADPCM-A's 49-step table"
        );
    }
}

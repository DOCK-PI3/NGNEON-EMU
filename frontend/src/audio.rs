//! SDL2 audio streaming for the YM2610 sound chip
//!
//! Architecture:
//!   1. Emulation thread generates YM2610 samples at 55.5 kHz each frame
//!   2. Samples are pushed into a `Resampler` that uses a phase accumulator for
//!      sample-accurate rate conversion to 44.1 kHz
//!   3. Resampled output is pushed into a ring buffer
//!   4. SDL2 audio callback pulls from the ring buffer on a separate thread

use sdl2::audio::AudioCallback;
use std::sync::{Arc, Mutex};

/// Target output sample rate (44.1 kHz is universally supported by SDL2).
pub const AUDIO_OUTPUT_RATE: i32 = 44100;
/// Number of output channels (stereo).
pub const AUDIO_CHANNELS: u8 = 2;
/// Short fade used when the host audio callback briefly outruns emulation.
const UNDERRUN_FADE_SAMPLES: usize = 512;
const MVS_MASTER_CLOCK_HZ: u64 = 24_000_000;
const MVS_MASTER_CYCLES_PER_FRAME: u64 = 405_504;
const MVS_Z80_CLOCK_HZ: f64 = 4_000_000.0;
const YM2610_TSTATES_PER_SAMPLE: f64 = 72.0;

// ---------------------------------------------------------------------------
//  Manual fixed-size ring buffer (power-of-2 capacity)
// ---------------------------------------------------------------------------

/// Replaces `VecDeque<i16>` with direct `(tail + offset) & mask` indexing.
/// Stores pre-converted `f64` samples so the convolution hot loop never
/// pays for `cvtsi2sd` (i16 → f64) conversion — samples are converted
/// once on `push()` instead of 6+ times per frame during `pull()`.
pub struct RingBuffer {
    data: Vec<f64>,
    head: usize,
    tail: usize,
    len: usize,
    mask: usize,
}

impl RingBuffer {
    /// Round `capacity` up to the next power of two (minimum 1).
    pub fn new(capacity: usize) -> Self {
        let cap = capacity.max(1).next_power_of_two();
        Self {
            data: vec![0.0_f64; cap],
            head: 0,
            tail: 0,
            len: 0,
            mask: cap - 1,
        }
    }

    /// Index from the logical start (`tail`).  Must be `#[inline(always)]` so
    /// the convolution loop compiles to a single `add` + `and` instruction.
    #[inline(always)]
    pub fn get(&self, offset: usize) -> f64 {
        self.data[(self.tail + offset) & self.mask]
    }

    /// x86_64 only: load an interleaved (L,R) pair as a packed `__m128d`
    /// in one `movupd` instruction, skipping both `cvtsi2sd` conversions
    /// and the `_mm_set_pd` shuffle.  When the pair straddles the ring
    /// wrap boundary (`idx == mask`), falls back to `_mm_set_pd`.
    ///
    /// # Safety
    ///
    /// This uses x86_64 SIMD intrinsics and pointer arithmetic. Callers must
    /// only invoke it on x86_64 and ensure `offset` points at a valid stereo
    /// pair in the ring buffer.
    #[cfg(target_arch = "x86_64")]
    #[inline(always)]
    pub unsafe fn load_pd(&self, offset: usize) -> std::arch::x86_64::__m128d {
        let idx = (self.tail + offset) & self.mask;
        if idx == self.mask {
            // Pair straddles wrap boundary: L at data[mask], R at data[0]
            std::arch::x86_64::_mm_set_pd(self.data[0], self.data[self.mask])
        } else {
            std::arch::x86_64::_mm_loadu_pd(self.data.as_ptr().add(idx))
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Append samples, converting i16 → f64 on the way in.
    /// If the buffer fills up, oldest samples are silently overwritten.
    pub fn extend(&mut self, samples: &[i16]) {
        for &s in samples {
            self.data[self.head] = s as f64;
            self.head = (self.head + 1) & self.mask;
            if self.len == self.data.len() {
                self.tail = (self.tail + 1) & self.mask;
            } else {
                self.len += 1;
            }
        }
    }

    /// Drop the oldest sample.
    pub fn pop_front(&mut self) {
        if self.len > 0 {
            self.tail = (self.tail + 1) & self.mask;
            self.len -= 1;
        }
    }

    /// Discard all buffered samples.
    pub fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
        self.len = 0;
    }
}

// ---------------------------------------------------------------------------
//  Sample-accurate resampler (phase-accumulator, Lanczos windowed-sinc)
// ---------------------------------------------------------------------------

/// Lanczos window size (a=3 → 6 taps per channel).
const LANCZOS_A: f64 = 3.0;
/// Fractional-phase quantization steps for the precomputed weight table.
const LANCZOS_TABLE_STEPS: usize = 256;

// ---- SIMD-accelerated 6-tap Lanczos-3 convolution -----------------------

#[cfg(target_arch = "x86_64")]
mod simd {
    use super::RingBuffer;
    use std::arch::x86_64::*;

    /// SSE2 baseline 6-tap convolution.
    /// Each tap loads an interleaved (L,R) pair via `_mm_loadu_pd` —
    /// one instruction replaces two `cvtsi2sd` + `_mm_set_pd`.
    #[inline(always)]
    pub unsafe fn conv6(buff: &RingBuffer, base_offset: isize, w: &[f64; 6]) -> (f64, f64) {
        let mut acc = _mm_setzero_pd();
        for j in 0isize..6 {
            let off = (base_offset + j * 2) as usize;
            // _mm_loadu_pd loads [lane0, lane1] = [L, R] from memory
            let sample = buff.load_pd(off);
            let weight = _mm_set1_pd(w[j as usize]);
            acc = _mm_add_pd(acc, _mm_mul_pd(sample, weight));
        }
        // transmute: lane0 → arr[0] (L), lane1 → arr[1] (R)
        let arr: [f64; 2] = std::mem::transmute(acc);
        (arr[0], arr[1])
    }

    /// FMA3-accelerated 6-tap convolution (Haswell+, 2013+).
    /// Uses `_mm_fmadd_pd` to fuse multiply-add into a single instruction
    /// with one rounding step instead of two.
    /// Caller must guard with `is_x86_feature_detected!("fma")`.
    #[inline]
    #[target_feature(enable = "fma")]
    pub unsafe fn conv6_fma(buff: &RingBuffer, base_offset: isize, w: &[f64; 6]) -> (f64, f64) {
        let mut acc = _mm_setzero_pd();
        for j in 0isize..6 {
            let off = (base_offset + j * 2) as usize;
            let sample = buff.load_pd(off);
            let weight = _mm_set1_pd(w[j as usize]);
            acc = _mm_fmadd_pd(sample, weight, acc);
        }
        let arr: [f64; 2] = std::mem::transmute(acc);
        (arr[0], arr[1])
    }
}

/// Scalar fallback for non-x86_64 targets (ARM, WASM, etc.).
#[cfg(not(target_arch = "x86_64"))]
mod simd {
    use super::RingBuffer;

    #[inline(always)]
    pub fn conv6(buff: &RingBuffer, base_offset: isize, w: &[f64; 6]) -> (f64, f64) {
        let mut lacc = 0.0_f64;
        let mut racc = 0.0_f64;
        for j in 0isize..6 {
            let off = (base_offset + j * 2) as usize;
            // get() returns f64 directly — no i16→f64 cast needed
            lacc += buff.get(off) * w[j as usize];
            racc += buff.get(off + 1) * w[j as usize];
        }
        (lacc, racc)
    }
}

/// Platform-optimal 6-tap convolution.
/// x86_64: FMA3 if detected at runtime, otherwise SSE2.
/// Other targets: scalar fallback.
#[inline(always)]
fn conv6(buff: &RingBuffer, base_offset: isize, w: &[f64; 6], use_fma: bool) -> (f64, f64) {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        if use_fma {
            simd::conv6_fma(buff, base_offset, w)
        } else {
            simd::conv6(buff, base_offset, w)
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = use_fma;
        simd::conv6(buff, base_offset, w)
    }
}

/// Normalised sinc function: sinc(0) = 1.
fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-7 {
        1.0
    } else {
        let px = std::f64::consts::PI * x;
        px.sin() / px
    }
}

/// Lanczos windowed-sinc kernel: sinc(x) · sinc(x / a) for |x| < a.
fn lanczos(x: f64, a: f64) -> f64 {
    if x.abs() < a {
        sinc(x) * sinc(x / a)
    } else {
        0.0
    }
}

/// Converts a source sample-rate to a target sample-rate using a persistent
/// phase accumulator and a Lanczos-3 windowed-sinc interpolation kernel,
/// so the conversion is seamless across frame boundaries.
///
/// The drain threshold is `phase >= 3.0` (instead of 1.0) so the buffer
/// always retains up to 2 *stale* stereo pairs preceding the current
/// interpolation centre.  When 2 stale pairs are available the resampler
/// uses a **symmetric** Lanczos-3 kernel (offsets -2..+3) for full
/// anti-aliasing quality; otherwise it falls back to a causal kernel.
pub struct Resampler {
    /// FIFO of interleaved stereo source samples (L0, R0, L1, R1, …).
    buffer: RingBuffer,
    /// Fractional position in the source stream (in stereo-pair units).
    /// `floor(phase)` gives the number of stale pairs retained before the
    /// interpolation centre.
    phase: f64,
    /// Step size per output stereo pair: `source_rate / target_rate`.
    step: f64,
    /// Precomputed **causal** Lanczos-3 weights (offsets 0..+5).
    /// Used when fewer than 2 stale pairs are available.
    lanczos_table_causal: Vec<[f64; 6]>,
    /// Precomputed **symmetric** Lanczos-3 weights (offsets -2..+3).
    /// Used when 2 stale pairs are available for full anti-aliasing.
    lanczos_table_sym: Vec<[f64; 6]>,
    /// Diagnostic counter: how many output stereo pairs used the
    /// symmetric Lanczos-3 path.
    pub symmetric_hits: usize,
    /// Diagnostic counter: how many output stereo pairs used the
    /// causal Lanczos-3 path (Lanczos-2 and linear are not counted).
    pub causal_hits: usize,
    /// Diagnostic counter: Lanczos-2 fallback path.
    pub lanczos2_hits: usize,
    /// Diagnostic counter: linear-interpolation fallback path.
    pub linear_hits: usize,
    /// Whether the CPU supports FMA3 (fused multiply-add).
    /// Detected once at construction via `is_x86_feature_detected!("fma")`.
    has_fma: bool,
    /// Fractional SDL output cadence carried between MVS video frames.
    output_frame_accumulator: u64,
    /// Output stereo pairs that could not be reconstructed from source data.
    starved_output_pairs: usize,
    /// Fixed startup history used by the exact MVS stream so the Lanczos
    /// kernel keeps enough interpolation margin at every frame boundary.
    startup_history_pairs: usize,
    startup_history_primed: bool,
}

impl Resampler {
    /// Create a new resampler.
    ///
    /// `source_rate` – native YM2610 rate (≈ 55 555 Hz).
    /// `target_rate` – SDL2 output rate (44 100 Hz).
    pub fn new(source_rate: u32, target_rate: i32) -> Self {
        // ── causal Lanczos-3 (offsets 0..+5) ─────────────────────────
        let mut table_causal = vec![[0.0_f64; 6]; LANCZOS_TABLE_STEPS];
        for (i, table_row) in table_causal.iter_mut().enumerate() {
            let frac = i as f64 / LANCZOS_TABLE_STEPS as f64;
            let mut row = [0.0_f64; 6];
            let mut sum = 0.0;
            for (j, weight) in row.iter_mut().enumerate() {
                let x = frac - j as f64;
                let w = lanczos(x, LANCZOS_A);
                *weight = w;
                sum += w;
            }
            if sum > 0.0 {
                for w in row.iter_mut() {
                    *w /= sum;
                }
            }
            *table_row = row;
        }

        // ── symmetric Lanczos-3 (offsets -2..+3) ─────────────────────
        let mut table_sym = vec![[0.0_f64; 6]; LANCZOS_TABLE_STEPS];
        for (i, table_row) in table_sym.iter_mut().enumerate() {
            let frac = i as f64 / LANCZOS_TABLE_STEPS as f64;
            let mut row = [0.0_f64; 6];
            let mut sum = 0.0;
            for (j, weight) in row.iter_mut().enumerate() {
                // j=0→-2, j=1→-1, j=2→0, j=3→+1, j=4→+2, j=5→+3
                let x = frac - (j as f64 - 2.0);
                let w = lanczos(x, LANCZOS_A);
                *weight = w;
                sum += w;
            }
            if sum > 0.0 {
                for w in row.iter_mut() {
                    *w /= sum;
                }
            }
            *table_row = row;
        }

        Self {
            buffer: RingBuffer::new((source_rate / 25) as usize * 2),
            phase: 0.0,
            step: source_rate as f64 / target_rate as f64,
            lanczos_table_causal: table_causal,
            lanczos_table_sym: table_sym,
            symmetric_hits: 0,
            causal_hits: 0,
            lanczos2_hits: 0,
            linear_hits: 0,
            output_frame_accumulator: 0,
            starved_output_pairs: 0,
            startup_history_pairs: 0,
            startup_history_primed: false,
            has_fma: {
                #[cfg(target_arch = "x86_64")]
                {
                    std::is_x86_feature_detected!("fma")
                }
                #[cfg(not(target_arch = "x86_64"))]
                {
                    false
                }
            },
        }
    }

    /// Create the resampler using the exact MVS YM2610 clock ratio.
    ///
    /// The native stream is generated once per 72 Z80 tstates at 4 MHz,
    /// which is 55_555.555... Hz. Using the rounded public sample-rate
    /// constant (55_555 Hz) would accumulate roughly 2_000 source pairs per
    /// hour and eventually force the internal FIFO to overwrite audio.
    pub fn new_mvs(target_rate: i32) -> Self {
        let target_rate = target_rate.max(1);
        let mut resampler = Self::new(55_556, target_rate);
        resampler.step = (MVS_Z80_CLOCK_HZ / YM2610_TSTATES_PER_SAMPLE) / target_rate as f64;
        // Four native pairs are only 72 µs at the YM2610 rate. Repeating the
        // first frame here gives the six-tap filter permanent boundary
        // headroom without inserting periodic silence or accumulating drift.
        resampler.startup_history_pairs = 4;
        resampler
    }

    /// Feed interleaved stereo source samples at the native rate.
    pub fn push(&mut self, samples: &[i16]) {
        if !self.startup_history_primed && self.startup_history_pairs > 0 && samples.len() >= 2 {
            let first_pair = [samples[0], samples[1]];
            for _ in 0..self.startup_history_pairs {
                self.buffer.extend(&first_pair);
            }
            self.startup_history_primed = true;
        }
        self.buffer.extend(samples);
    }

    /// Return the exact integer number of stereo pairs due for the next MVS
    /// frame while preserving the fractional remainder between frames.
    pub fn next_mvs_output_pairs(&mut self) -> usize {
        self.output_frame_accumulator = self
            .output_frame_accumulator
            .saturating_add(AUDIO_OUTPUT_RATE as u64 * MVS_MASTER_CYCLES_PER_FRAME);
        let pairs = self.output_frame_accumulator / MVS_MASTER_CLOCK_HZ;
        self.output_frame_accumulator %= MVS_MASTER_CLOCK_HZ;
        pairs as usize
    }

    /// Pull `out.len()` output samples (interleaved stereo).
    ///
    /// The drain threshold is `phase >= 3.0` so 2 stale pairs are retained,
    /// enabling a symmetric Lanczos-3 kernel when available.
    ///
    /// Tiered interpolation (higher rows preferred when `frac ≠ 0`):
    ///
    /// | Stale | Pairs avail | Method                     |
    /// |-------|-------------|----------------------------|
    /// | —     | 0           | Silence                    |
    /// | any   | frac≈0      | Direct passthrough         |
    /// | ≥2    | ≥6          | Symmetric Lanczos-3 (-2..+3) |
    /// | <2    | ≥stale+6    | Causal Lanczos-3 (0..+5)   |
    /// | any   | ≥stale+4    | Lanczos-2 (0..+3)          |
    /// | any   | ≥stale+2    | Linear (0..+1)             |
    /// | any   | ≥stale+1    | Silence (no interpolation) |
    ///
    /// `out.len()` must be even (one `i16` per channel).
    pub fn pull(&mut self, out: &mut [i16]) {
        debug_assert!(
            out.len().is_multiple_of(2),
            "output buffer length must be even (stereo)"
        );
        let step = self.step;

        for chunk in out.chunks_exact_mut(2) {
            let pairs_avail = self.buffer.len() / 2;
            // Number of stale pairs already retained before the
            // interpolation centre (0, 1, or 2).
            let stale = (self.phase as usize).min(2);
            let frac = self.phase.fract();
            let (l, r);

            // ── silence (not enough source data even for passthrough) ──
            if pairs_avail < stale + 1 {
                self.starved_output_pairs += 1;
                // During prolonged starvation we cap the phase at 4.0
                // and reset to 2.0.  The stale=2 assumption is a "lie"
                // — there is no real history — but it lets the resampler
                // recover smoothly when new data arrives without a huge
                // phase jump.
                if self.phase > 4.0 {
                    self.phase = 2.0;
                }
                chunk[0] = 0;
                chunk[1] = 0;
                self.phase += step;
                continue;
            }

            // ── exact-integer phase → direct passthrough ──────────────
            if frac < 1e-7 {
                l = self.buffer.get(stale * 2);
                r = self.buffer.get(stale * 2 + 1);
            }
            // ── symmetric Lanczos-3 (offsets -2..+3) ──────────────────
            else if stale >= 2 && pairs_avail >= stale + 4 {
                self.symmetric_hits += 1;
                let idx = (frac * LANCZOS_TABLE_STEPS as f64) as usize % LANCZOS_TABLE_STEPS;
                let w = &self.lanczos_table_sym[idx];
                // base = (stale - 2) * 2  →  j=0 maps to offset -2, j=2 to 0, j=5 to +3
                let base = (stale as isize - 2) * 2;
                (l, r) = conv6(&self.buffer, base, w, self.has_fma);
            }
            // ── causal Lanczos-3 (offsets 0..+5) ──────────────────────
            else if pairs_avail >= stale + 6 {
                self.causal_hits += 1;
                let idx = (frac * LANCZOS_TABLE_STEPS as f64) as usize % LANCZOS_TABLE_STEPS;
                let w = &self.lanczos_table_causal[idx];
                let base = (stale as isize) * 2;
                (l, r) = conv6(&self.buffer, base, w, self.has_fma);
            }
            // ── Lanczos-2 (causal, offsets 0..+3) ─────────────────────
            else if pairs_avail >= stale + 4 {
                self.lanczos2_hits += 1;
                let mut lacc = 0.0_f64;
                let mut racc = 0.0_f64;
                let mut wsum = 0.0_f64;
                for j in 0..4 {
                    let x = frac - j as f64;
                    let w = lanczos(x, 2.0);
                    let offset = (stale + j) * 2;
                    lacc += self.buffer.get(offset) * w;
                    racc += self.buffer.get(offset + 1) * w;
                    wsum += w;
                }
                l = if wsum > 0.0 { lacc / wsum } else { 0.0 };
                r = if wsum > 0.0 { racc / wsum } else { 0.0 };
            }
            // ── linear (offsets 0..+1) ─────────────────────────────────
            else if pairs_avail >= stale + 2 {
                self.linear_hits += 1;
                let omf = 1.0 - frac;
                l = self.buffer.get(stale * 2) * omf + self.buffer.get((stale + 1) * 2) * frac;
                r = self.buffer.get(stale * 2 + 1) * omf
                    + self.buffer.get((stale + 1) * 2 + 1) * frac;
            }
            // ── not enough pairs for interpolation at this frac ───────
            else {
                self.starved_output_pairs += 1;
                if self.phase > 4.0 {
                    self.phase = 2.0;
                }
                l = 0.0;
                r = 0.0;
            }

            chunk[0] = l.clamp(i16::MIN as f64, i16::MAX as f64) as i16;
            chunk[1] = r.clamp(i16::MIN as f64, i16::MAX as f64) as i16;

            self.phase += step;

            // Drain consumed pairs, but keep the last 2 as stale history
            while self.phase >= 3.0 && self.buffer.len() >= 2 {
                self.buffer.pop_front();
                self.buffer.pop_front();
                self.phase -= 1.0;
            }
        }
    }

    /// Discard all buffered source samples and reset the phase.
    /// Call when loading a new ROM.
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.phase = 0.0;
        self.symmetric_hits = 0;
        self.causal_hits = 0;
        self.lanczos2_hits = 0;
        self.linear_hits = 0;
        self.output_frame_accumulator = 0;
        self.starved_output_pairs = 0;
        self.startup_history_primed = false;
    }

    /// Number of native stereo pairs retained for interpolation/history.
    pub fn buffered_source_pairs(&self) -> usize {
        self.buffer.len() / 2
    }

    /// Number of output stereo pairs emitted without sufficient source data.
    pub fn starved_output_pairs(&self) -> usize {
        self.starved_output_pairs
    }
}

/// Thread-safe ring buffer for audio samples flowing from the emulation
/// thread to the SDL2 audio callback thread.
pub struct AudioRingBuffer {
    data: Vec<i16>,
    /// Next position to read from (audio callback thread).
    read_pos: usize,
    /// Next position to write to (emulation thread).
    write_pos: usize,
    /// Number of valid samples available to read.
    available: usize,
    /// Last complete stereo frame sent to SDL, used to smooth short underruns.
    last_frame: [i16; 2],
    /// Next interleaved channel index to read/fill (0 = L, 1 = R).
    next_channel: usize,
    /// Remaining samples in the current underrun fade-to-silence ramp.
    underrun_fade_remaining: usize,
    /// Samples discarded because the producer filled the ring completely.
    overrun_samples: u64,
    /// Samples requested by SDL when no real buffered sample was available.
    underrun_samples: u64,
}

impl AudioRingBuffer {
    /// Returns the number of valid samples in the buffer.
    pub fn len(&self) -> usize {
        self.available
    }

    /// Returns true when no samples are available to read.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.available == 0
    }

    /// Gets the sample at logical offset (0 = oldest sample).
    /// Returns None if out of bounds.
    pub fn get(&self, offset: usize) -> Option<i16> {
        if offset >= self.available {
            return None;
        }
        let idx = (self.read_pos + offset) % self.data.len();
        Some(self.data[idx])
    }
    /// Create a ring buffer that holds `capacity` i16 values.
    /// A good capacity is 4 × frames of audio (~8192 for stereo 44100 Hz).
    /// Minimum capacity is 1.
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            data: vec![0i16; capacity],
            read_pos: 0,
            write_pos: 0,
            available: 0,
            last_frame: [0, 0],
            next_channel: 0,
            underrun_fade_remaining: 0,
            overrun_samples: 0,
            underrun_samples: 0,
        }
    }

    /// Push interleaved stereo samples from the emulation thread.
    /// If the buffer is full, samples are dropped (shouldn't happen with
    /// adequate capacity).
    pub fn write(&mut self, samples: &[i16]) {
        for &sample in samples {
            if self.available >= self.data.len() {
                // Buffer full — drop oldest sample to make room
                self.read_pos = (self.read_pos + 1) % self.data.len();
                self.available -= 1;
                self.overrun_samples += 1;
            }
            self.data[self.write_pos] = sample;
            self.write_pos = (self.write_pos + 1) % self.data.len();
            self.available += 1;
        }
    }

    /// Read samples into `out` from the audio callback thread.
    /// If there aren't enough samples, the last stereo frame fades out
    /// briefly instead of snapping to hard silence.
    pub fn read(&mut self, out: &mut [i16]) {
        let to_copy = out.len().min(self.available);
        for sample in out.iter_mut().take(to_copy) {
            *sample = self.data[self.read_pos];
            self.last_frame[self.next_channel] = *sample;
            self.next_channel = (self.next_channel + 1) % AUDIO_CHANNELS as usize;
            self.read_pos = (self.read_pos + 1) % self.data.len();
        }
        self.available -= to_copy;

        if to_copy == out.len() {
            self.underrun_fade_remaining = 0;
            return;
        }
        self.underrun_samples += (out.len() - to_copy) as u64;

        if self.underrun_fade_remaining == 0 {
            self.underrun_fade_remaining = UNDERRUN_FADE_SAMPLES;
        }

        // Fade from the last real stereo frame to silence. This avoids
        // audible clicks/dropouts for short host scheduling hiccups.
        for sample in out.iter_mut().skip(to_copy) {
            if self.underrun_fade_remaining == 0 {
                *sample = 0;
            } else {
                let source = self.last_frame[self.next_channel] as i32;
                let faded =
                    source * self.underrun_fade_remaining as i32 / UNDERRUN_FADE_SAMPLES as i32;
                *sample = faded.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                self.underrun_fade_remaining -= 1;
            }
            self.next_channel = (self.next_channel + 1) % AUDIO_CHANNELS as usize;
        }
    }

    /// Discard all buffered samples. Call when loading a new ROM to
    /// prevent stale audio from the previous ROM playing briefly.
    pub fn clear(&mut self) {
        self.read_pos = 0;
        self.write_pos = 0;
        self.available = 0;
        self.last_frame = [0, 0];
        self.next_channel = 0;
        self.underrun_fade_remaining = 0;
        self.overrun_samples = 0;
        self.underrun_samples = 0;
    }

    /// Number of producer samples lost because the ring was full.
    pub fn overrun_samples(&self) -> u64 {
        self.overrun_samples
    }

    /// Number of callback samples synthesized by the underrun fade/silence.
    pub fn underrun_samples(&self) -> u64 {
        self.underrun_samples
    }
}

/// SDL2 audio callback that streams from a ring buffer.
pub struct Ym2610AudioCallback {
    pub buffer: Arc<Mutex<AudioRingBuffer>>,
}

impl AudioCallback for Ym2610AudioCallback {
    type Channel = i16;

    fn callback(&mut self, out: &mut [i16]) {
        if let Ok(mut buf) = self.buffer.lock() {
            buf.read(out);
        } else {
            out.fill(0);
        }
    }
}

// ---------------------------------------------------------------------------
//  Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── helpers ───────────────────────────────────────────────────────────

    /// Create a stereo sine-ish pattern so we can spot discontinuities.
    fn make_stereo_ramp(n: usize) -> Vec<i16> {
        (0..n)
            .flat_map(|i| {
                let pair = i as i16;
                [pair * 10 + 100, pair * 10 + 200]
            })
            .collect()
    }

    /// All samples in `buf` are zero.
    fn all_zero(buf: &[i16]) -> bool {
        buf.iter().all(|&s| s == 0)
    }

    #[test]
    fn mvs_output_cadence_preserves_fractional_samples() {
        let mut resampler = Resampler::new(55_555, AUDIO_OUTPUT_RATE);
        let frames = 10_000usize;
        let produced: usize = (0..frames).map(|_| resampler.next_mvs_output_pairs()).sum();
        let expected = (frames as u64 * AUDIO_OUTPUT_RATE as u64 * MVS_MASTER_CYCLES_PER_FRAME
            / MVS_MASTER_CLOCK_HZ) as usize;

        assert_eq!(produced, expected);
        assert!(produced > 745 * frames);
        assert!(produced < 746 * frames);
    }

    #[test]
    fn resampler_reset_clears_mvs_output_cadence() {
        let mut resampler = Resampler::new(55_555, AUDIO_OUTPUT_RATE);
        let first = resampler.next_mvs_output_pairs();
        let _ = resampler.next_mvs_output_pairs();
        resampler.reset();

        assert_eq!(resampler.next_mvs_output_pairs(), first);
    }

    #[test]
    fn exact_mvs_clock_ratio_has_no_long_run_source_drift() {
        const Z80_TSTATES_PER_FRAME: u32 = 67_584;
        const FRAMES: usize = 10_000;

        let mut resampler = Resampler::new_mvs(AUDIO_OUTPUT_RATE);
        let source = vec![1i16; 939 * 2];
        let mut ym_remainder = 0u32;

        for _ in 0..FRAMES {
            let total = ym_remainder + Z80_TSTATES_PER_FRAME;
            let native_pairs = (total / YM2610_TSTATES_PER_SAMPLE as u32) as usize;
            ym_remainder = total % YM2610_TSTATES_PER_SAMPLE as u32;
            resampler.push(&source[..native_pairs * 2]);

            let output_pairs = resampler.next_mvs_output_pairs();
            let mut output = vec![0i16; output_pairs * 2];
            resampler.pull(&mut output);
        }

        assert_eq!(
            resampler.starved_output_pairs(),
            0,
            "the exact clock ratio must never outrun the native stream"
        );
        assert!(
            resampler.buffered_source_pairs() <= 6,
            "native source drift accumulated to {} pairs",
            resampler.buffered_source_pairs()
        );
    }

    // ── 1:1 rate conversion ──────────────────────────────────────────────

    #[test]
    fn identity_rate_passthrough() {
        // source == target → step == 1.0 → output == input (exact)
        let mut r = Resampler::new(44100, 44100);
        let src = make_stereo_ramp(8); // 16 i16 = 8 stereo pairs
        r.push(&src);

        let mut out = vec![0i16; 16];
        r.pull(&mut out);

        assert_eq!(out, src, "1:1 resampling must be identity");
        // 2 stale pairs are retained after draining
        assert_eq!(
            r.buffer.len(),
            4,
            "2 stale pairs (4 i16) should be retained"
        );
    }

    #[test]
    fn identity_rate_multiple_push_pull() {
        let mut r = Resampler::new(44100, 44100);

        // Frame 1
        let src1 = make_stereo_ramp(5);
        r.push(&src1);
        let mut out1 = vec![0i16; 10];
        r.pull(&mut out1);
        assert_eq!(out1, src1);

        // Frame 2 — continuity
        let src2 = make_stereo_ramp(5);
        r.push(&src2);
        let mut out2 = vec![0i16; 10];
        r.pull(&mut out2);
        assert_eq!(out2, src2);
    }

    // ── fractional rate conversion ────────────────────────────────────────

    #[test]
    fn downsampling_produces_fewer_output_pairs() {
        // 55.5 kHz → 44.1 kHz  ⇒  step ≈ 1.26
        let mut r = Resampler::new(55555, 44100);
        let src = make_stereo_ramp(926); // one frame worth
        r.push(&src);

        let mut out = vec![0i16; 735 * 2]; // expected output for one frame
        r.pull(&mut out);

        // Should produce non-silence output for most samples
        let non_zero = out.iter().filter(|&&s| s != 0).count();
        assert!(
            non_zero > 700 * 2,
            "expected > 1400 non-zero output samples, got {non_zero}"
        );

        // Phase should be fractional after draining (not exactly 0)
        assert!(
            r.buffer.len() < 10,
            "buffer should be nearly drained, got {} samples",
            r.buffer.len()
        );
    }

    #[test]
    fn upsampling_preserves_content() {
        // 22.05 kHz → 44.1 kHz  ⇒  step = 0.5
        let mut r = Resampler::new(22050, 44100);
        let src: Vec<i16> = vec![100, 200, 300, 400]; // 2 stereo pairs
        r.push(&src);

        // With step=0.5 we get 4 output pairs from 2 source pairs.
        //   out0: frac=0.000 → (100, 200)
        //   out1: frac=0.500 → (200, 300)  mid-point between the two pairs
        //   out2: frac=0.000 → drained → (300, 400)
        //   out3: frac=0.500 → only 1 pair left → silence
        let mut out = vec![0i16; 8];
        r.pull(&mut out);

        assert_eq!(out[0], 100);
        assert_eq!(out[1], 200);
        assert_eq!(out[2], 200); // (100*0.5 + 300*0.5) = 200
        assert_eq!(out[3], 300); // (200*0.5 + 400*0.5) = 300
                                 // After draining pair0, pair1 is at position 0 (300,400), frac=0
        assert_eq!(out[4], 300);
        assert_eq!(out[5], 400);
        // Last pair: only 1 source pair left with frac=0.5 → silence
        assert_eq!(out[6], 0);
        assert_eq!(out[7], 0);
    }

    // ── buffer starvation / silence ───────────────────────────────────────

    #[test]
    fn empty_buffer_produces_silence() {
        let mut r = Resampler::new(55555, 44100);
        let mut out = vec![0i16; 32];
        r.pull(&mut out);
        assert!(all_zero(&out), "empty resampler must output silence");
    }

    #[test]
    fn phase_capped_during_prolonged_starvation() {
        let mut r = Resampler::new(55555, 44100);

        // Pull many frames of silence — phase would drift without the cap
        for _ in 0..10 {
            let mut out = vec![0i16; 735 * 2];
            r.pull(&mut out);
            assert!(all_zero(&out));
        }

        // Now push source data — recovery should be gradual, not instant-drain
        let src = make_stereo_ramp(926);
        r.push(&src);
        let mut out = vec![0i16; 735 * 2];
        r.pull(&mut out);

        // After phase-capped recovery, most output should be real audio.
        // With the cap, almost all 735 output pairs should be valid.
        let non_zero = out.iter().filter(|&&s| s != 0).count();
        assert!(
            non_zero > 720 * 2,
            "recovery should produce mostly real audio, got {non_zero} non-zero"
        );
    }

    #[test]
    fn partial_buffer_starvation_mid_pull() {
        // Push only enough for a few output samples, then pull many
        let mut r = Resampler::new(55555, 44100);
        let src = make_stereo_ramp(10); // 10 stereo pairs (~20 i16 samples)
        r.push(&src);

        // Try to pull far more than we have source for
        let mut out = vec![0i16; 200];
        r.pull(&mut out);

        // First samples should be non-zero, tail should be silence
        let first_silent = out.iter().position(|&s| s == 0).unwrap_or(out.len());
        assert!(first_silent > 0, "first sample should be non-zero");
        assert!(first_silent < out.len(), "some silence expected at the end");
        assert!(all_zero(&out[first_silent..]));
    }

    // ── reset ─────────────────────────────────────────────────────────────

    #[test]
    fn reset_clears_buffer_and_phase() {
        let mut r = Resampler::new(55555, 44100);

        // Push and partially consume
        let src = make_stereo_ramp(100);
        r.push(&src);
        let mut out = vec![0i16; 10];
        r.pull(&mut out);

        r.reset();

        // After reset, push the same data and pull — should get the
        // beginning of the ramp again, not a continuation
        let src2 = make_stereo_ramp(5);
        r.push(&src2);
        let mut out2 = vec![0i16; 10];
        r.pull(&mut out2);

        // First pair should be exactly the ramp start
        assert_eq!(out2[0], 100);
        assert_eq!(out2[1], 200);
    }

    // ── path coverage ───────────────────────────────────────────────────

    #[test]
    fn symmetric_lanczos_path_is_used_during_normal_operation() {
        // At 55.5 kHz → 44.1 kHz (step ≈ 1.26), after ~2 output pairs
        // the phase accumulator crosses 2.0 and `stale` reaches 2,
        // enabling the symmetric Lanczos-3 kernel.
        let mut r = Resampler::new(55555, 44100);

        // Push enough source pairs to sustain several symmetric outputs
        r.push(&make_stereo_ramp(10)); // 10 stereo pairs

        let mut out = vec![0i16; 6 * 2]; // 6 output stereo pairs
        r.pull(&mut out);

        assert!(
            r.symmetric_hits > 0,
            "expected symmetric Lanczos-3 to be used at least once, got {}",
            r.symmetric_hits
        );
        // Out1 uses causal (stale=1, frac≈0.26), so causal should fire too
        assert!(
            r.causal_hits > 0,
            "expected causal Lanczos-3 to be used at least once (stale<2 warmup), got {}",
            r.causal_hits
        );
        // With 10 source pairs and plenty of headroom, neither Lanczos-2
        // nor linear should fire during normal operation.
        assert_eq!(
            r.lanczos2_hits, 0,
            "Lanczos-2 should not fire with abundant source pairs"
        );
        assert_eq!(
            r.linear_hits, 0,
            "linear should not fire with abundant source pairs"
        );
    }

    #[test]
    fn fallback_paths_fire_when_source_runs_low() {
        // 6 pairs at step=1.5 traces:
        //   out0: frac=0.0 → passthrough
        //   out1: stale=1 frac=0.5, pairs=6, causal(≥7)?NO, l2(≥5)?YES → Lanczos-2
        //   out2: frac=0.0 → passthrough (drain → pairs=4)
        //   out3: stale=2 frac=0.5, pairs=4, sym(≥6)?NO, causal?NO, l2?NO, lin(≥4)?YES → linear
        //   out4: stale=2 frac=0.0, pairs=2 < stale+1(3) → silence
        let mut r = Resampler::new(66150, 44100); // step = 1.5
        r.push(&make_stereo_ramp(6));

        let mut out = vec![0i16; 5 * 2]; // 5 output pairs
        r.pull(&mut out);

        assert_eq!(
            r.symmetric_hits, 0,
            "symmetric should not fire (never have stale≥2 with ≥6 pairs)"
        );
        assert_eq!(
            r.causal_hits, 0,
            "causal should not fire (never have stale<2 with ≥stale+6 pairs)"
        );
        assert_eq!(
            r.lanczos2_hits, 1,
            "Lanczos-2 should fire exactly once (out1: stale=1, 6 pairs)"
        );
        assert_eq!(
            r.linear_hits, 1,
            "linear should fire exactly once (out3: stale=2, 4 pairs)"
        );
    }

    // ── continuity across push boundaries ─────────────────────────────────

    #[test]
    fn interpolation_bridges_push_boundaries() {
        // With step=1.5 (source_rate=66150, target_rate=44100) the resampler
        // needs the "next" source pair for interpolation, which may come
        // from a separate push().
        let mut r = Resampler::new(66150, 44100); // step = 1.5

        // Push 2 pairs, then a second batch
        r.push(&[100, 200, 300, 400]); // pair0, pair1
        r.push(&[500, 600, 700, 800]); // pair2, pair3

        let mut out = vec![0i16; 6]; // 3 output pairs
        r.pull(&mut out);

        // With step=1.5, phase starts at 0:
        // out0: interpolate pair0 (100,200) and pair1 (300,400) at frac=0 → (100,200)
        //   phase += 1.5 → 1.5, drain 1 pair (pair0), phase → 0.5
        // out1: interpolate pair1 (300,400) and pair2 (500,600) at frac=0.5
        //   left = 300*0.5 + 500*0.5 = 400
        //   right = 400*0.5 + 600*0.5 = 500
        //   phase += 1.5 → 2.0, drain 2 pairs (pair1, pair2), phase → 0.0
        // out2: interpolate pair3 (700,800) and ??? at frac=0 → buffer has only 2 samples → silence
        //   Wait, after draining pair1 and pair2, buffer has pair3 (700,800) - only 2 samples
        //   So out2 should be silence

        // out0 should be the first source pair directly (frac=0)
        assert_eq!(out[0], 100);
        assert_eq!(out[1], 200);
        // out1 should interpolate across the push boundary (pair1 from first push, pair2 from second)
        assert_eq!(out[2], 400); // left channel of output pair 1
        assert_eq!(out[3], 500); // right channel of output pair 1
    }

    // ── fixed-step drain verification ─────────────────────────────────────

    #[test]
    fn step_2_drains_exactly_two_pairs_per_output() {
        // step = 2.0 → each output consumes exactly 2 source pairs
        let mut r = Resampler::new(88200, 44100); // step = 2.0
        r.push(&[
            100, 200, // pair0
            300, 400, // pair1
            500, 600, // pair2
            700, 800, // pair3
        ]);

        let mut out = vec![0i16; 4]; // 2 output pairs
        r.pull(&mut out);

        // Should consume all 4 source pairs, producing 2 output pairs
        // out0: frac=0, pair0=100,200
        // out1: frac=0 → drain 2 pairs → pair1=300,400
        // Then drain 2 more → pair2, pair3. Phase back to 0.
        // Actually: out0: frac=0 → (100,200), phase += 2.0 → 2.0, drain 2 pairs (pair0,pair1) → phase 0.0
        // out1: frac=0 → pair2=500,600, phase += 2.0 → 2.0, drain 2 pairs (pair2,pair3) → phase 0.0
        assert_eq!(out[0], 100);
        assert_eq!(out[1], 200);
        assert_eq!(out[2], 500);
        assert_eq!(out[3], 600);
        // 2 stale pairs retained
        assert_eq!(r.buffer.len(), 4);
    }

    // ── debug_assert / edge cases ────────────────────────────────────────

    #[test]
    fn pull_empty_output_buffer_is_noop() {
        let mut r = Resampler::new(44100, 44100);
        r.push(&[10, 20, 30, 40]);
        // Zero-length output — should not panic and should not consume source
        r.pull(&mut []);
        assert_eq!(r.buffer.len(), 4);
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn pull_with_odd_length_does_not_panic_in_release() {
        let mut r = Resampler::new(44100, 44100);
        r.push(&[10, 20, 30, 40]);

        // Odd-length buffer: the last sample is silently dropped by chunks_exact_mut(2).
        // In debug this panics via debug_assert; in release it must not panic.
        let mut out = vec![0i16; 5]; // odd!
        r.pull(&mut out);
        // The 5th sample simply stays 0
    }

    // ──────────────────────────────────────────────────────────────────────
    //  AudioRingBuffer tests
    // ──────────────────────────────────────────────────────────────────────

    #[test]
    fn ring_write_read_roundtrip() {
        let mut buf = AudioRingBuffer::new(8);
        assert!(buf.is_empty());
        buf.write(&[10, 20, 30, 40]);
        assert!(!buf.is_empty());

        let mut out = vec![0i16; 4];
        buf.read(&mut out);

        assert_eq!(out, [10, 20, 30, 40]);
        assert_eq!(buf.available, 0);
        assert!(buf.is_empty());
    }

    #[test]
    fn ring_full_drops_oldest() {
        let mut buf = AudioRingBuffer::new(4);
        // Fill exactly
        buf.write(&[10, 20, 30, 40]);

        // Overflow — oldest (10, 20) should be dropped
        buf.write(&[50, 60]);

        // Remaining: [30, 40, 50, 60]
        let mut out = vec![0i16; 4];
        buf.read(&mut out);
        assert_eq!(out, [30, 40, 50, 60]);
        assert_eq!(buf.available, 0);
    }

    #[test]
    fn ring_clear() {
        let mut buf = AudioRingBuffer::new(8);
        buf.write(&[10, 20, 30, 40]);
        buf.clear();

        let mut out = vec![0i16; 4];
        buf.read(&mut out);

        assert!(all_zero(&out));
        assert_eq!(buf.available, 0);
    }

    #[test]
    fn ring_partial_read_fades_before_silence() {
        let mut buf = AudioRingBuffer::new(8);
        buf.write(&[10, 20]);

        // Request more than available
        let mut out = vec![0i16; UNDERRUN_FADE_SAMPLES + 4];
        buf.read(&mut out);

        assert_eq!(out[0], 10);
        assert_eq!(out[1], 20);
        assert_ne!(out[2], 0);
        assert_ne!(out[3], 0);
        assert!(all_zero(&out[UNDERRUN_FADE_SAMPLES + 2..]));
        assert_eq!(buf.available, 0);
    }

    #[test]
    fn ring_multiple_write_read_cycles() {
        let mut buf = AudioRingBuffer::new(8);

        // Write 4, read 2
        buf.write(&[10, 20, 30, 40]);
        let mut out = vec![0i16; 2];
        buf.read(&mut out);
        assert_eq!(out, [10, 20]);

        // Write 2 more, read remaining 4
        buf.write(&[50, 60]);
        let mut out = vec![0i16; 4];
        buf.read(&mut out);
        assert_eq!(out, [30, 40, 50, 60]);
        assert_eq!(buf.available, 0);
    }

    #[test]
    fn ring_empty_read_is_silence() {
        let mut buf = AudioRingBuffer::new(8);
        let mut out = vec![0i16; 4];
        buf.read(&mut out);
        assert!(all_zero(&out));
    }

    #[test]
    fn ring_exact_capacity_boundary() {
        let cap = 8;
        let mut buf = AudioRingBuffer::new(cap);
        let src: Vec<i16> = (0..cap as i16).map(|i| i * 10 + 10).collect();
        buf.write(&src);

        let mut out = vec![0i16; cap];
        buf.read(&mut out);
        assert_eq!(out, src);
        assert_eq!(buf.available, 0);
    }

    #[test]
    fn ring_wraparound_read() {
        let mut buf = AudioRingBuffer::new(8);

        // Fill 6, consume 4 → read_pos at 4, available=2
        buf.write(&[10, 20, 30, 40, 50, 60]);
        let mut out = vec![0i16; 4];
        buf.read(&mut out);
        assert_eq!(out, [10, 20, 30, 40]);

        // Write 4 more (wraps around write_pos past end)
        buf.write(&[70, 80, 90, 100]);

        // Should read [50, 60, 70, 80, 90, 100] — crossing the wrap point
        let mut out = vec![0i16; 6];
        buf.read(&mut out);
        assert_eq!(out, [50, 60, 70, 80, 90, 100]);
        assert_eq!(buf.available, 0);
    }

    #[test]
    fn ring_clear_then_write_is_fresh() {
        let mut buf = AudioRingBuffer::new(8);
        buf.write(&[99, 99, 99, 99]);
        buf.clear();
        buf.write(&[10, 20]);

        let mut out = vec![0i16; 4];
        buf.read(&mut out);
        assert_eq!(out[0], 10);
        assert_eq!(out[1], 20);
        assert_ne!(out[2], 99);
        assert_ne!(out[3], 99);
    }

    #[test]
    fn ring_zero_capacity_silently_becomes_one() {
        // new(0) is guarded to capacity=1 — no panic, no divide-by-zero
        let mut buf = AudioRingBuffer::new(0);
        buf.write(&[10, 20, 30]); // only the last sample (30) survives
        let mut out = vec![0i16; 2];
        buf.read(&mut out);
        assert_eq!(out, [30, 0]);
    }

    #[test]
    fn ring_drop_more_than_capacity() {
        // Write more than 2× capacity in one call
        let mut buf = AudioRingBuffer::new(4);
        buf.write(&[10, 20, 30, 40, 50, 60, 70, 80]);

        // Should keep only the last 4: [50, 60, 70, 80]
        let mut out = vec![0i16; 4];
        buf.read(&mut out);
        assert_eq!(out, [50, 60, 70, 80]);
    }

    #[test]
    fn ring_available_tracks_correctly_after_full() {
        let mut buf = AudioRingBuffer::new(4);
        buf.write(&[10, 20, 30, 40]); // available=4
        assert_eq!(buf.available, 4);

        buf.write(&[50]); // oldest dropped, available stays 4
        assert_eq!(buf.available, 4);

        let mut out = vec![0i16; 2];
        buf.read(&mut out); // available becomes 2
        assert_eq!(buf.available, 2);
        assert_eq!(out, [20, 30]);

        buf.write(&[60, 70]); // available=4 again, write_pos may wrap
        assert_eq!(buf.available, 4);

        let mut out = vec![0i16; 4];
        buf.read(&mut out);
        assert_eq!(out, [40, 50, 60, 70]);
        assert_eq!(buf.available, 0);
    }

    #[test]
    fn ring_diagnostics_count_and_reset_overrun_and_underrun() {
        let mut buf = AudioRingBuffer::new(2);
        buf.write(&[10, 20, 30]);
        assert_eq!(buf.overrun_samples(), 1);

        let mut out = [0i16; 4];
        buf.read(&mut out);
        assert_eq!(buf.underrun_samples(), 2);

        buf.clear();
        assert_eq!(buf.overrun_samples(), 0);
        assert_eq!(buf.underrun_samples(), 0);
    }
}

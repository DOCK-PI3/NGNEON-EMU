//! Central audio mixer for the NeoGeo emulator core.
//!
//! The `AudioMixer` unifies audio generation from:
//!   - **YM2610** (OPNB): 4 FM channels, 6 ADPCM-A channels, 1 ADPCM-B channel, 3 SSG channels
//!   - **PCM2 DAC**: ADPCM samples from V-ROM (played through YM2610 ADPCM channels after
//!     PCM2 protection descrambling, which is handled in the ROM loading phase)
//!
//! Audio flow:
//!   1. Z80 co-processor drives YM2610 registers via I/O ports 0x04–0x07
//!   2. `AudioMixer::generate()` calls `Ym2610::generate()` to produce stereo i16 samples
//!   3. Samples are stored internally for the frontend to consume via `samples()`
//!   4. Frontend resamples (55.5 kHz → 44.1 kHz) and pushes to SDL2 ring buffer
//!
//! Reference clock: YM2610 runs at 8 MHz master clock / 144 ≈ 55,555 Hz.

use crate::ym2610::Ym2610;

/// Number of samples generated per frame at 60 FPS.
/// YM2610 runs at ~55,555 Hz, so per frame: 55,555 / 60 ≈ 926.
pub const YM2610_SAMPLES_PER_FRAME: usize = (crate::ym2610::YM2610_SAMPLE_RATE as usize) / 60;

/// Geolith clocks the YM2610 once per 72 Z80 tstates in medium-fidelity mode.
pub const YM2610_TSTATES_PER_SAMPLE: u32 = 72;

/// Stereo interleaved buffer size per frame: samples_per_frame * 2 channels.
pub const STEREO_BUFFER_SIZE: usize = YM2610_SAMPLES_PER_FRAME * 2;

/// Central audio mixer that generates and stores stereo audio samples.
///
/// This struct owns a persistent buffer so allocations are minimized —
/// the buffer is reused every frame.
pub struct AudioMixer {
    /// Reusable stereo i16 buffer for generated samples.
    buffer: Vec<i16>,
    /// Number of stereo sample pairs generated in the last `generate()` call.
    samples_generated: usize,
    /// Fractional Z80 tstates carried between frames for YM sample cadence.
    ym_tstate_remainder: u32,
}

impl AudioMixer {
    /// Create a new `AudioMixer` with a pre-allocated buffer.
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            samples_generated: 0,
            ym_tstate_remainder: 0,
        }
    }

    /// Generate one frame of audio from the YM2610 sound chip.
    ///
    /// This calls `Ym2610::generate()` to produce stereo i16 samples
    /// at the native YM2610 rate (~55.5 kHz), stored in the internal buffer.
    ///
    /// Returns a reference to the generated (interleaved) samples:
    /// `[L0, R0, L1, R1, L2, R2, ...]`.
    ///
    /// The buffer is valid until the next `generate()` call.
    pub fn generate(&mut self, ym2610: &mut Ym2610) -> &[i16] {
        // Resize buffer to fit one frame and reuse the allocation
        self.buffer.resize(STEREO_BUFFER_SIZE, 0);

        // Generate YM2610 audio directly into the buffer slice
        ym2610.generate(&mut self.buffer, YM2610_SAMPLES_PER_FRAME);
        self.samples_generated = YM2610_SAMPLES_PER_FRAME;

        &self.buffer
    }

    /// Generate audio for the number of Z80 tstates actually executed.
    ///
    /// This mirrors Geolith's medium-fidelity path where YM2610 output is
    /// produced every 72 Z80 cycles and fractional remainder carries across
    /// frames.  On MVS timing this yields a 938/939/939 sample pattern instead
    /// of forcing the chip into an artificial 60 FPS cadence.
    pub fn generate_for_tstates(&mut self, ym2610: &mut Ym2610, z80_tstates: u32) -> &[i16] {
        self.begin_frame();
        self.append_for_tstates(ym2610, z80_tstates)
    }

    /// Start accumulating a new frame from interleaved Z80/YM2610 slices.
    pub fn begin_frame(&mut self) {
        self.buffer.clear();
        self.samples_generated = 0;
    }

    /// Advance the YM2610 for an additional Z80 execution slice and append
    /// the generated samples to the current frame buffer.
    pub fn append_for_tstates(&mut self, ym2610: &mut Ym2610, z80_tstates: u32) -> &[i16] {
        let total = self.ym_tstate_remainder.saturating_add(z80_tstates);
        let sample_pairs = (total / YM2610_TSTATES_PER_SAMPLE) as usize;
        self.ym_tstate_remainder = total % YM2610_TSTATES_PER_SAMPLE;

        let old_len = self.buffer.len();
        self.buffer.resize(old_len + sample_pairs * 2, 0);
        if sample_pairs > 0 {
            ym2610.generate(&mut self.buffer[old_len..], sample_pairs);
        }
        self.samples_generated += sample_pairs;

        &self.buffer
    }

    /// Get a reference to the most recently generated audio samples.
    ///
    /// Returns an empty slice if `generate()` has not been called yet.
    pub fn samples(&self) -> &[i16] {
        &self.buffer
    }

    /// Number of stereo sample pairs generated in the last frame.
    pub fn samples_generated(&self) -> usize {
        self.samples_generated
    }

    /// Reset the mixer. Call when loading a new ROM to clear stale audio.
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.samples_generated = 0;
        self.ym_tstate_remainder = 0;
    }
}

impl Default for AudioMixer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::Memory;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn new_test_mixer() -> (AudioMixer, Ym2610) {
        let mem = Rc::new(RefCell::new(Memory::new()));
        let ym2610 = Ym2610::new(mem);
        (AudioMixer::new(), ym2610)
    }

    #[test]
    fn mixer_new_has_empty_buffer() {
        let (mixer, _) = new_test_mixer();
        assert!(mixer.samples().is_empty());
        assert_eq!(mixer.samples_generated(), 0);
    }

    #[test]
    fn mixer_generates_expected_number_of_samples() {
        let (mut mixer, mut ym2610) = new_test_mixer();

        let samples = mixer.generate(&mut ym2610);

        // Should produce ~926 stereo pairs = ~1852 samples
        assert_eq!(samples.len(), STEREO_BUFFER_SIZE);
        assert_eq!(mixer.samples_generated(), YM2610_SAMPLES_PER_FRAME);
    }

    #[test]
    fn mixer_geolith_cadence_uses_z80_tstates_with_remainder() {
        let (mut mixer, mut ym2610) = new_test_mixer();

        mixer.generate_for_tstates(&mut ym2610, crate::DEFAULT_Z80_TSTATES_PER_FRAME);
        assert_eq!(mixer.samples_generated(), 938);
        assert_eq!(mixer.samples().len(), 938 * 2);

        mixer.generate_for_tstates(&mut ym2610, crate::DEFAULT_Z80_TSTATES_PER_FRAME);
        assert_eq!(mixer.samples_generated(), 939);
        assert_eq!(mixer.samples().len(), 939 * 2);

        mixer.generate_for_tstates(&mut ym2610, crate::DEFAULT_Z80_TSTATES_PER_FRAME);
        assert_eq!(mixer.samples_generated(), 939);
        assert_eq!(mixer.samples().len(), 939 * 2);
    }

    #[test]
    fn mixer_interleaved_slices_preserve_frame_cadence() {
        let (mut mixer, mut ym2610) = new_test_mixer();
        let first = crate::DEFAULT_Z80_TSTATES_PER_FRAME / 2;
        let second = crate::DEFAULT_Z80_TSTATES_PER_FRAME - first;

        mixer.begin_frame();
        mixer.append_for_tstates(&mut ym2610, first);
        mixer.append_for_tstates(&mut ym2610, second);

        assert_eq!(mixer.samples_generated(), 938);
        assert_eq!(mixer.samples().len(), 938 * 2);
    }

    #[test]
    fn mixer_generated_samples_are_stereo_interleaved() {
        let (mut mixer, mut ym2610) = new_test_mixer();

        // Configure YM2610 with a simple tone
        ym2610.write_address(0, 0xB1);
        ym2610.write_data(0, 0x00); // algo 0, fb 0
        ym2610.write_address(0, 0xB5);
        ym2610.write_data(0, 0xC0); // L+R
        ym2610.write_address(0, 0xA1);
        ym2610.write_data(0, 0x69); // F-num low
        ym2610.write_address(0, 0xA5);
        ym2610.write_data(0, 0x24); // block=4
        for reg in [0x41, 0x45, 0x49, 0x4D] {
            ym2610.write_address(0, reg);
            ym2610.write_data(0, 0x20);
        }
        for reg in [0x31, 0x35, 0x39, 0x3D] {
            ym2610.write_address(0, reg);
            ym2610.write_data(0, 0x01);
        }
        for reg in [0x51, 0x55, 0x59, 0x5D] {
            ym2610.write_address(0, reg);
            ym2610.write_data(0, 0x1F);
        }
        ym2610.write_address(0, 0x28);
        ym2610.write_data(0, 0xF1); // Key on first active FM channel

        let samples = mixer.generate(&mut ym2610);

        // Should have non-zero samples with stereo channels
        let has_left = samples.iter().step_by(2).any(|&s| s != 0);
        let has_right = samples.iter().skip(1).step_by(2).any(|&s| s != 0);
        assert!(has_left, "Expected non-zero left channel samples");
        assert!(has_right, "Expected non-zero right channel samples");
    }

    #[test]
    fn mixer_generates_silence_when_no_audio_configured() {
        let (mut mixer, mut ym2610) = new_test_mixer();

        let samples = mixer.generate(&mut ym2610);

        // Without any channels configured, all samples should be 0 (silence)
        assert!(samples.iter().all(|&s| s == 0));
    }

    #[test]
    fn mixer_reuses_buffer_across_frames() {
        let (mut mixer, mut ym2610) = new_test_mixer();

        // First frame (silence)
        let first = mixer.generate(&mut ym2610);
        assert_eq!(first.len(), STEREO_BUFFER_SIZE);

        // Second frame — buffer should be reused, still correct size
        let second = mixer.generate(&mut ym2610);
        assert_eq!(second.len(), STEREO_BUFFER_SIZE);
    }

    #[test]
    fn mixer_reset_clears_buffer() {
        let (mut mixer, mut ym2610) = new_test_mixer();

        mixer.generate(&mut ym2610);
        assert!(!mixer.samples().is_empty());

        mixer.reset();
        assert!(mixer.samples().is_empty());
        assert_eq!(mixer.samples_generated(), 0);
    }

    #[test]
    fn default_equals_new() {
        let a = AudioMixer::new();
        let b = AudioMixer::default();
        assert_eq!(a.samples_generated(), b.samples_generated());
        assert_eq!(a.samples().len(), b.samples().len());
    }
}

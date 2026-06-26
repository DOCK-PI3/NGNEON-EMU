//! Criterion benchmarks comparing Lanczos-3 windowed-sinc resampling against
//! plain linear interpolation across different rate-conversion scenarios.
//!
//! Run with:  cargo bench --manifest-path frontend/Cargo.toml

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use frontend::audio::{Resampler, RingBuffer};

// ---------------------------------------------------------------------------
//  Minimal linear-only resampler (replicates the old behaviour)
// ---------------------------------------------------------------------------

struct LinearResampler {
    buffer: RingBuffer,
    phase: f64,
    step: f64,
}

impl LinearResampler {
    fn new(source_rate: u32, target_rate: i32) -> Self {
        Self {
            buffer: RingBuffer::new((source_rate / 25) as usize * 2),
            phase: 0.0,
            step: source_rate as f64 / target_rate as f64,
        }
    }

    fn push(&mut self, samples: &[i16]) {
        self.buffer.extend(samples);
    }

    fn pull(&mut self, out: &mut [i16]) {
        for chunk in out.chunks_exact_mut(2) {
            let pairs_avail = self.buffer.len() / 2;
            let frac = self.phase.fract();

            let (l, r): (f64, f64) = if pairs_avail == 0 {
                // ─ silence ─
                if self.phase > 2.0 {
                    self.phase = 0.0;
                }
                (0.0, 0.0)
            } else if frac < 1e-7 {
                // ─ direct passthrough ─
                (self.buffer.get(0), self.buffer.get(1))
            } else if pairs_avail >= 2 {
                // ─ linear interpolation ─
                let omf = 1.0 - frac;
                (
                    self.buffer.get(0) * omf + self.buffer.get(2) * frac,
                    self.buffer.get(1) * omf + self.buffer.get(3) * frac,
                )
            } else {
                (0.0, 0.0)
            };

            chunk[0] = l.clamp(i16::MIN as f64, i16::MAX as f64) as i16;
            chunk[1] = r.clamp(i16::MIN as f64, i16::MAX as f64) as i16;

            self.phase += self.step;
            while self.phase >= 1.0 && self.buffer.len() >= 2 {
                self.buffer.pop_front();
                self.buffer.pop_front();
                self.phase -= 1.0;
            }
        }
    }
}

// ── the real Lanczos-3 resampler ──────────────────────────────────────

// ── test-data generators ──────────────────────────────────────────────

/// Approximate one frame of YM2610 source samples at 55 555 Hz (926 stereo pairs).
fn ym2610_source_frame() -> Vec<i16> {
    let mut v = Vec::with_capacity(926 * 2);
    let mut phase = 0.0f64;
    for _ in 0..926 {
        // Sine-like stereo ramp so the resampler sees real-ish data
        let l = (phase.sin() * 8000.0) as i16;
        let r = (phase.cos() * 8000.0) as i16;
        v.push(l);
        v.push(r);
        phase += 0.03;
    }
    v
}

/// Pull outputs at 44 100 Hz for one \"frame\" (~735 stereo pairs).
fn output_buffer() -> Vec<i16> {
    vec![0i16; 735 * 2]
}

// ── benchmarks ────────────────────────────────────────────────────────

fn bench_identity_rate(c: &mut Criterion) {
    let src = ym2610_source_frame();
    let mut out = output_buffer();

    let mut group = c.benchmark_group("resampler/identity (1:1)");
    group.bench_function("Lanczos-3", |b| {
        let mut r = Resampler::new(44100, 44100);
        b.iter(|| {
            r.push(black_box(&src));
            r.pull(black_box(&mut out));
        });
    });
    group.bench_function("Linear", |b| {
        let mut r = LinearResampler::new(44100, 44100);
        b.iter(|| {
            r.push(black_box(&src));
            r.pull(black_box(&mut out));
        });
    });
    group.finish();
}

fn bench_ym2610_downsample(c: &mut Criterion) {
    // 55 555 → 44 100 Hz — the typical YM2610 use case
    let src = ym2610_source_frame();
    let mut out = output_buffer();

    let mut group = c.benchmark_group("resampler/55.5k→44.1k");
    group.bench_function("Lanczos-3", |b| {
        let mut r = Resampler::new(55555, 44100);
        b.iter(|| {
            r.push(black_box(&src));
            r.pull(black_box(&mut out));
        });
    });
    group.bench_function("Linear", |b| {
        let mut r = LinearResampler::new(55555, 44100);
        b.iter(|| {
            r.push(black_box(&src));
            r.pull(black_box(&mut out));
        });
    });
    group.finish();
}

fn bench_upsample_2x(c: &mut Criterion) {
    // 22 050 → 44 100 Hz (2× upsampling)
    // About half as many source samples → 463 pairs
    let src: Vec<i16> = ym2610_source_frame();
    let src_half = &src[..463 * 2];
    let mut out_half = vec![0i16; 463 * 2 * 2]; // 926 output pairs

    let mut group = c.benchmark_group("resampler/22.05k→44.1k (2×)");
    group.bench_function("Lanczos-3", |b| {
        let mut r = Resampler::new(22050, 44100);
        b.iter(|| {
            r.push(black_box(src_half));
            r.pull(black_box(&mut out_half));
        });
    });
    group.bench_function("Linear", |b| {
        let mut r = LinearResampler::new(22050, 44100);
        b.iter(|| {
            r.push(black_box(src_half));
            r.pull(black_box(&mut out_half));
        });
    });
    group.finish();
}

// ── starvation / recovery benchmarks ──────────────────────────────────

/// Pull `count` output stereo pairs (producing silence).
fn pull_silence(r: &mut Resampler, count: usize) {
    let mut out = vec![0i16; count * 2];
    r.pull(&mut out);
}

fn pull_silence_linear(r: &mut LinearResampler, count: usize) {
    let mut out = vec![0i16; count * 2];
    r.pull(&mut out);
}

/// Benchmark: after prolonged starvation (many frames of silence),
/// how fast is the recovery pull when source data finally arrives?
fn bench_starvation_recovery(c: &mut Criterion) {
    let src = ym2610_source_frame();
    let mut out = output_buffer();

    let mut group = c.benchmark_group("resampler/starvation-recovery");
    group.bench_function("Lanczos-3", |b| {
        let mut r = Resampler::new(55555, 44100);
        // Drive phase into cap territory every iteration, then push+pull
        b.iter(|| {
            pull_silence(&mut r, 735);
            r.push(black_box(&src));
            r.pull(black_box(&mut out));
        });
    });
    group.bench_function("Linear", |b| {
        let mut r = LinearResampler::new(55555, 44100);
        b.iter(|| {
            pull_silence_linear(&mut r, 735);
            r.push(black_box(&src));
            r.pull(black_box(&mut out));
        });
    });
    group.finish();
}

/// Benchmark: push a trickle of source (20 pairs) then pull a full
/// output frame.  The first ~15 outputs use real data; the remaining
/// ~720 fall into the silence path mid-pull.  This measures the cost of
/// the branch transition.
fn bench_mixed_starvation_mid_pull(c: &mut Criterion) {
    // 20 stereo pairs of sine data (enough to produce ~15 valid outputs
    // before the buffer drains and silence takes over).
    let trickle_small: Vec<i16> = {
        let full = ym2610_source_frame();
        full[..20 * 2].to_vec()
    };
    let mut out = output_buffer();

    let mut group = c.benchmark_group("resampler/mixed-starvation");
    group.bench_function("Lanczos-3", |b| {
        let mut r = Resampler::new(55555, 44100);
        b.iter(|| {
            r.push(black_box(&trickle_small));
            r.pull(black_box(&mut out));
        });
    });
    group.bench_function("Linear", |b| {
        let mut r = LinearResampler::new(55555, 44100);
        b.iter(|| {
            r.push(black_box(&trickle_small));
            r.pull(black_box(&mut out));
        });
    });
    group.finish();
}

criterion_group!(
    resampler,
    bench_identity_rate,
    bench_ym2610_downsample,
    bench_upsample_2x,
    bench_starvation_recovery,
    bench_mixed_starvation_mid_pull,
);
criterion_main!(resampler);

//! Manual profiling harness — measures each resampler path independently
//! using high-resolution timing on the hot loop.
//!
//! Run with:  cargo run --release --bin profile_resampler

use frontend::audio::Resampler;
use std::hint::black_box;
use std::time::Instant;

/// Minimum measurement iterations for statistical stability.
const WARMUP_ITERS: usize = 20;
const MEASURE_ITERS: usize = 500;

fn main() {
    let src: Vec<i16> = (0..926_i16)
        .flat_map(|i| {
            let phase = i as f64 * 0.03;
            let l = (phase.sin() * 8000.0) as i16;
            let r = (phase.cos() * 8000.0) as i16;
            [l, r]
        })
        .collect();
    let mut out = vec![0i16; 735 * 2];
    let tiny_src: Vec<i16> = src[..6 * 2].to_vec(); // 6 pairs for casual
    let mut tiny_out = vec![0i16; 4 * 2]; // 4 output pairs

    println!("=== Manual Resampler Path Timing ===");
    println!(
        "Source: 55555→44100 Hz (step≈1.26), {} iterations each\n",
        MEASURE_ITERS
    );

    // ── symmetric Lanczos-3 (stale=2, ≥6 pairs) ─────────────────
    {
        let mut r = Resampler::new(55555, 44100);
        // Warm up until stale=2
        r.push(&src);
        r.pull(&mut out);
        // Now in symmetric territory — measure
        for _ in 0..WARMUP_ITERS {
            r.push(&src);
            r.pull(&mut out);
        }
        let t0 = Instant::now();
        for _ in 0..MEASURE_ITERS {
            r.push(black_box(&src));
            r.pull(black_box(&mut out));
        }
        let elapsed = t0.elapsed();
        println!(
            "Lanczos-3 symmetric:  {:8.3} µs/frame  (hits={})",
            elapsed.as_secs_f64() / MEASURE_ITERS as f64 * 1e6,
            r.symmetric_hits / (WARMUP_ITERS + MEASURE_ITERS),
        );
        black_box(&out);
    }

    // ── causal Lanczos-3 (stale<2, ≥6 pairs) ────────────────────
    {
        let mut r = Resampler::new(55555, 44100);
        for _ in 0..WARMUP_ITERS {
            r.push(&tiny_src);
            r.pull(&mut tiny_out);
        }
        let t0 = Instant::now();
        for _ in 0..MEASURE_ITERS {
            r.push(black_box(&tiny_src));
            let mut out2 = tiny_out.clone();
            r.pull(black_box(&mut out2));
        }
        let elapsed = t0.elapsed();
        println!(
            "Lanczos-3 causal:     {:8.3} µs/frame  (hits={})",
            elapsed.as_secs_f64() / MEASURE_ITERS as f64 * 1e6,
            r.causal_hits / (WARMUP_ITERS + MEASURE_ITERS),
        );
    }

    // ── Lanczos-2 on-the-fly (4 pairs) ──────────────────────────
    {
        let mut r = Resampler::new(55555, 44100);
        let l2_src: Vec<i16> = src[..4 * 2].to_vec();
        let l2_out = vec![0i16; 2 * 2];
        for _ in 0..WARMUP_ITERS {
            r.push(&l2_src);
            r.pull(&mut l2_out.clone());
        }
        let t0 = Instant::now();
        for _ in 0..MEASURE_ITERS {
            r.push(black_box(&l2_src));
            let mut out2 = l2_out.clone();
            r.pull(black_box(&mut out2));
        }
        let elapsed = t0.elapsed();
        println!(
            "Lanczos-2 on-the-fly: {:8.3} µs/frame  (hits={})",
            elapsed.as_secs_f64() / MEASURE_ITERS as f64 * 1e6,
            r.lanczos2_hits / (WARMUP_ITERS + MEASURE_ITERS),
        );
    }

    // ── linear interpolation (2 pairs) ───────────────────────────
    {
        let mut r = Resampler::new(55555, 44100);
        let lin_src: Vec<i16> = src[..2 * 2].to_vec();
        let lin_out = vec![0i16; 2];
        for _ in 0..WARMUP_ITERS {
            r.push(&lin_src);
            r.pull(&mut lin_out.clone());
        }
        let t0 = Instant::now();
        for _ in 0..MEASURE_ITERS {
            r.push(black_box(&lin_src));
            let mut out2 = lin_out.clone();
            r.pull(black_box(&mut out2));
        }
        let elapsed = t0.elapsed();
        println!(
            "Linear (2-tap):       {:8.3} µs/frame  (hits={})",
            elapsed.as_secs_f64() / MEASURE_ITERS as f64 * 1e6,
            r.linear_hits / (WARMUP_ITERS + MEASURE_ITERS),
        );
    }

    // ── direct passthrough (frac=0) ──────────────────────────────
    {
        let mut r = Resampler::new(44100, 44100); // step=1.0 → frac always 0
        let id_out = vec![0i16; 2 * 2];
        let id_src: Vec<i16> = src[..2 * 2].to_vec();
        for _ in 0..WARMUP_ITERS {
            r.push(&id_src);
            r.pull(&mut id_out.clone());
        }
        let t0 = Instant::now();
        for _ in 0..MEASURE_ITERS {
            r.push(black_box(&id_src));
            let mut out2 = id_out.clone();
            r.pull(black_box(&mut out2));
        }
        let elapsed = t0.elapsed();
        println!(
            "Direct passthrough:   {:8.3} µs/frame",
            elapsed.as_secs_f64() / MEASURE_ITERS as f64 * 1e6,
        );
    }

    // ── pure silence (starvation) ────────────────────────────────
    {
        let mut r = Resampler::new(55555, 44100);
        let sil_out = vec![0i16; 735 * 2];
        for _ in 0..WARMUP_ITERS {
            r.pull(&mut sil_out.clone());
        }
        let t0 = Instant::now();
        for _ in 0..MEASURE_ITERS {
            let mut out2 = sil_out.clone();
            r.pull(black_box(&mut out2));
        }
        let elapsed = t0.elapsed();
        println!(
            "Silence (starvation): {:8.3} µs/frame",
            elapsed.as_secs_f64() / MEASURE_ITERS as f64 * 1e6,
        );
    }

    // ── breakdown: convolution vs table lookup vs drain ──────────
    println!("\n=== Convolution Loop Micro-benchmark ===");
    {
        let mut r = Resampler::new(55555, 44100);
        r.push(&src);
        r.pull(&mut out);

        // Measure full push+pull
        for _ in 0..WARMUP_ITERS {
            r.push(&src);
            r.pull(&mut out);
        }
        let t0 = Instant::now();
        for _ in 0..MEASURE_ITERS {
            r.push(black_box(&src));
            r.pull(black_box(&mut out));
        }
        let elapsed = t0.elapsed();
        let per_frame = elapsed.as_secs_f64() / MEASURE_ITERS as f64 * 1e6;
        let per_output = per_frame / 735.0;
        let per_output_ns = per_output * 1000.0;
        println!(
            "Full frame (push+pull): {:.3} µs  =  {:.2} ns per output pair",
            per_frame, per_output_ns,
        );
        println!(
            "  symmetric_hits={}  causal={}  l2={}  linear={}",
            r.symmetric_hits / (WARMUP_ITERS + MEASURE_ITERS),
            r.causal_hits / (WARMUP_ITERS + MEASURE_ITERS),
            r.lanczos2_hits / (WARMUP_ITERS + MEASURE_ITERS),
            r.linear_hits / (WARMUP_ITERS + MEASURE_ITERS),
        );
    }

    println!("\nDone. Run `cargo asm` or `objdump` to inspect the generated conv loop.");
}

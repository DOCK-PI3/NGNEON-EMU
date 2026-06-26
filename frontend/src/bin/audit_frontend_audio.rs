//! Long-run `.neo` audio pipeline audit.
//!
//! Exercises the same core -> exact MVS resampler -> ring buffer path used by
//! the SDL frontend, while consuming audio in callback-sized blocks.

use core_emulator::{bios, rom::RomData, NeoGeo};
use frontend::audio::{AudioRingBuffer, Resampler, AUDIO_OUTPUT_RATE};
use std::path::{Path, PathBuf};

const SDL_CALLBACK_PAIRS: usize = 1024;
const RING_CAPACITY_SAMPLES: usize = 32_768;
const MVS_MASTER_CLOCK_HZ: u64 = 24_000_000;
const MVS_MASTER_CYCLES_PER_FRAME: u64 = 405_504;

fn main() {
    if let Err(error) = run() {
        eprintln!("[ERROR] {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args_os().skip(1);
    let rom_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "Uso: audit_frontend_audio <rom.neo> [bios_dir] [frames]".to_string())?;
    if !rom_path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("neo"))
    {
        return Err("Esta auditoría acepta exclusivamente ROMs .neo".to_string());
    }
    let bios_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("bios"));
    let frames = args
        .next()
        .map(|value| value.to_string_lossy().parse::<usize>())
        .transpose()
        .map_err(|error| format!("Frames inválidos: {error}"))?
        .unwrap_or(3_600);

    let mut rom = RomData::from_neo(&rom_path)?;
    let dirs = [bios_dir.as_path()];
    let selected_bios = bios::load_bios_from_multi(&dirs)?
        .ok_or_else(|| format!("No se encontró BIOS en {:?}", bios_dir))?;
    let zoom = bios::load_zoom_rom_for_bios_from_multi(&dirs, &selected_bios.label)?;
    let sfix = bios::load_sfix_rom_for_bios_from_multi(&dirs, &selected_bios.label)?;
    let sm1 = bios::load_sm1_rom_for_bios_from_multi(&dirs, &selected_bios.label)?;

    let mut machine = NeoGeo::new();
    machine.set_bios(selected_bios.data);
    if let Some(zoom) = zoom {
        machine.set_zoom_rom(zoom.data);
    }
    if let Some(sfix) = sfix {
        machine.set_sfix_rom(sfix.data);
    }
    if let Some(sm1) = sm1 {
        machine.set_sm1_rom(sm1.data);
    }
    machine.load_rom_and_connect(&mut rom);

    let mut resampler = Resampler::new_mvs(AUDIO_OUTPUT_RATE);
    let mut ring = AudioRingBuffer::new(RING_CAPACITY_SAMPLES);
    let mut native_pairs = 0usize;
    let mut output_pairs = 0usize;
    let mut callbacks = 0usize;
    let mut nonzero_samples = 0usize;
    let mut peak = 0u16;

    for frame in 0..frames {
        machine
            .step()
            .map_err(|error| format!("Fallo CPU en frame {}: {error}", frame + 1))?;

        let native = machine.audio_mixer.samples();
        native_pairs += native.len() / 2;
        resampler.push(native);

        let due_pairs = resampler.next_mvs_output_pairs();
        output_pairs += due_pairs;
        let mut resampled = vec![0i16; due_pairs * 2];
        resampler.pull(&mut resampled);
        ring.write(&resampled);

        let callbacks_due = output_pairs / SDL_CALLBACK_PAIRS;
        while callbacks < callbacks_due {
            let mut callback = [0i16; SDL_CALLBACK_PAIRS * 2];
            ring.read(&mut callback);
            nonzero_samples += callback.iter().filter(|sample| **sample != 0).count();
            peak = peak.max(
                callback
                    .iter()
                    .map(|sample| sample.unsigned_abs())
                    .max()
                    .unwrap_or(0),
            );
            callbacks += 1;
        }
    }

    let expected_output_pairs =
        (frames as u64 * AUDIO_OUTPUT_RATE as u64 * MVS_MASTER_CYCLES_PER_FRAME
            / MVS_MASTER_CLOCK_HZ) as usize;
    let expected_ring_samples = output_pairs * 2 - callbacks * SDL_CALLBACK_PAIRS * 2;
    let passed = output_pairs == expected_output_pairs
        && ring.len() == expected_ring_samples
        && resampler.starved_output_pairs() == 0
        && ring.overrun_samples() == 0
        && ring.underrun_samples() == 0;

    println!(
        "[FRONTEND_AUDIO] verdict={} rom={} frames={} native_pairs={} output_pairs={} expected_pairs={} callbacks={} ring_samples={} expected_ring={} source_buffer_pairs={} starved_pairs={} overruns={} underruns={} nonzero={} peak={}",
        if passed { "PASS" } else { "FAIL" },
        display_name(&rom_path),
        frames,
        native_pairs,
        output_pairs,
        expected_output_pairs,
        callbacks,
        ring.len(),
        expected_ring_samples,
        resampler.buffered_source_pairs(),
        resampler.starved_output_pairs(),
        ring.overrun_samples(),
        ring.underrun_samples(),
        nonzero_samples,
        peak,
    );

    if passed {
        Ok(())
    } else {
        Err("La tubería de audio del frontend perdió sincronía o muestras".to_string())
    }
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("<rom.neo>")
        .to_string()
}

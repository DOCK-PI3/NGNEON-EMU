use core_emulator::{bios, rom::RomData, EmuAction, NeoGeo};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

const NATIVE_SAMPLE_RATE: u32 = 55_555;

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
        .ok_or_else(|| "Falta la ruta de la ROM .neo".to_string())?;
    let bios_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("bios"));
    let frames = args
        .next()
        .map(|value| value.to_string_lossy().parse::<usize>())
        .transpose()
        .map_err(|error| format!("Frames inválidos: {error}"))?
        .unwrap_or(1_800);
    let output_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/ngneon_capture.wav"));
    let standard_stimulus = args
        .next()
        .is_some_and(|value| value.to_string_lossy().eq_ignore_ascii_case("stimulus"));

    let mut rom = RomData::from_neo(&rom_path)?;
    let dirs = [bios_dir.as_path()];
    let bios = bios::load_bios_from_multi(&dirs)?
        .ok_or_else(|| format!("No se encontró BIOS en {:?}", bios_dir))?;
    let zoom = bios::load_zoom_rom_for_bios_from_multi(&dirs, &bios.label)?;
    let sfix = bios::load_sfix_rom_for_bios_from_multi(&dirs, &bios.label)?;
    let sm1 = bios::load_sm1_rom_for_bios_from_multi(&dirs, &bios.label)?;

    let mut machine = NeoGeo::new();
    machine.set_bios(bios.data);
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

    let mut audio = Vec::with_capacity(frames.saturating_mul(1_880));
    let mut video_metrics = Vec::with_capacity(frames);
    for frame in 0..frames {
        apply_standard_stimulus(&mut machine, frame, standard_stimulus);
        machine
            .step()
            .map_err(|error| format!("Fallo CPU en frame {}: {error}", frame + 1))?;
        audio.extend_from_slice(machine.audio_mixer.samples());
        video_metrics.push(frame_metrics(&machine.video.framebuffer));
    }

    write_wav(&output_path, &audio, NATIVE_SAMPLE_RATE)?;
    let video_path = output_path.with_extension("video.csv");
    write_video_metrics(&video_path, &video_metrics)?;
    let ram_path = output_path.with_extension("ram.bin");
    std::fs::write(&ram_path, &machine.memory.borrow().ram)
        .map_err(|error| format!("No se pudo guardar {:?}: {error}", ram_path))?;
    let cart_path = output_path.with_extension("cart.bin");
    std::fs::write(&cart_path, &machine.memory.borrow().pvc_cart_ram)
        .map_err(|error| format!("No se pudo guardar {:?}: {error}", cart_path))?;
    let nonzero = audio.iter().filter(|sample| **sample != 0).count();
    let peak = audio
        .iter()
        .map(|sample| sample.unsigned_abs())
        .max()
        .unwrap_or(0);
    let status_a = machine
        .memory
        .borrow()
        .read8(core_emulator::memory::STATUS_A_PORT);
    println!(
        "[NGNEON_PCM] frames={} rate={} stereo_pairs={} nonzero={} peak={} status_a=0x{:02X} output={:?} video={:?} ram={:?} cart={:?}",
        frames,
        NATIVE_SAMPLE_RATE,
        audio.len() / 2,
        nonzero,
        peak,
        status_a,
        output_path,
        video_path,
        ram_path,
        cart_path
    );
    Ok(())
}

fn apply_standard_stimulus(machine: &mut NeoGeo, frame: usize, enabled: bool) {
    for action in [EmuAction::Coin, EmuAction::Start, EmuAction::A] {
        machine.set_input(action, false);
    }
    if !enabled {
        return;
    }
    machine.set_input(EmuAction::Coin, (489..494).contains(&frame));
    machine.set_input(EmuAction::Start, (504..509).contains(&frame));
    machine.set_input(EmuAction::A, (669..674).contains(&frame));
}

fn frame_metrics(framebuffer: &[u32]) -> (usize, u64) {
    let mut nonblack = 0usize;
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for pixel in framebuffer {
        let rgb = *pixel & 0x00ff_ffff;
        nonblack += usize::from(rgb != 0);
        hash ^= rgb as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    (nonblack, hash)
}

fn write_video_metrics(path: &Path, metrics: &[(usize, u64)]) -> Result<(), String> {
    let mut file =
        File::create(path).map_err(|error| format!("No se pudo crear {:?}: {error}", path))?;
    file.write_all(b"frame,nonblack,hash\n")
        .map_err(|error| error.to_string())?;
    for (frame, (nonblack, hash)) in metrics.iter().enumerate() {
        writeln!(file, "{},{},0x{:016X}", frame + 1, nonblack, hash)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn write_wav(path: &Path, samples: &[i16], sample_rate: u32) -> Result<(), String> {
    let data_size = u32::try_from(samples.len().saturating_mul(2))
        .map_err(|_| "La captura PCM excede el límite WAV de 4 GiB".to_string())?;
    let mut file =
        File::create(path).map_err(|error| format!("No se pudo crear {:?}: {error}", path))?;
    file.write_all(b"RIFF").map_err(|error| error.to_string())?;
    file.write_all(&(36u32 + data_size).to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(b"WAVEfmt ")
        .map_err(|error| error.to_string())?;
    file.write_all(&16u32.to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(&1u16.to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(&2u16.to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(&sample_rate.to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(&(sample_rate * 4).to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(&4u16.to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(&16u16.to_le_bytes())
        .map_err(|error| error.to_string())?;
    file.write_all(b"data").map_err(|error| error.to_string())?;
    file.write_all(&data_size.to_le_bytes())
        .map_err(|error| error.to_string())?;
    for sample in samples {
        file.write_all(&sample.to_le_bytes())
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

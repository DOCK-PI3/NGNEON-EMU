use core_emulator::{bios, rom::RomData, EmuAction, NeoGeo};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const DEFAULT_MAX_FRAMES: usize = 1_800;
const VIDEO_SAMPLE_INTERVAL: usize = 30;
const MASTER_CYCLES_PER_FRAME: u64 = 405_504;
const Z80_MASTER_DIV: u64 = 6;
const YM2610_TSTATES_PER_SAMPLE: u64 = 72;

#[derive(Debug, Default)]
struct RuntimeMetrics {
    frames_run: usize,
    first_cart_frame: Option<usize>,
    first_video_frame: Option<usize>,
    first_audio_frame: Option<usize>,
    audio_pairs: u64,
    min_audio_pairs: usize,
    max_audio_pairs: usize,
    max_nonblack_pixels: usize,
    sampled_pcs: BTreeSet<u32>,
    cpu_error: Option<String>,
}

fn main() -> Result<(), String> {
    let rom_dir = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("roms"));
    let max_frames = std::env::args()
        .nth(2)
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|error| format!("Frames inválidos: {error}"))?
        .unwrap_or(DEFAULT_MAX_FRAMES);
    let selected_roms = std::env::args()
        .nth(3)
        .map(|value| parse_rom_selection(&value))
        .unwrap_or_default();

    let rom_paths = collect_rom_paths(&rom_dir, &selected_roms)?;
    if rom_paths.is_empty() {
        return Err(format!("No se encontraron ROMs .neo/.zip en {:?}", rom_dir));
    }

    let system_roms = SystemRoms::load("bios")?;
    println!(
        "[CONFIG] dir={:?} roms={} max_frames={} bios={}",
        rom_dir,
        rom_paths.len(),
        max_frames,
        system_roms.bios_label
    );

    let mut loaded = 0usize;
    let mut load_failed = 0usize;
    let mut runtime_passed = 0usize;
    let mut runtime_review = 0usize;
    let mut runtime_failed = 0usize;

    for path in rom_paths {
        let label = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("<nombre-inválido>");

        let mut rom = match load_rom_data(&path) {
            Ok(rom) => rom,
            Err(error) => {
                load_failed += 1;
                println!("[LOAD_FAIL] rom={label} error={}", compact(&error));
                continue;
            }
        };
        loaded += 1;

        let metadata = rom.metadata.as_ref();
        let ngh = metadata.map_or(0, |metadata| metadata.ngh);
        let audio_payload = if is_placeholder_audio_payload(&rom.vrom) {
            "placeholder"
        } else {
            "present"
        };
        let expected_silent_attract = is_expected_silent_attract(label);
        let board = metadata
            .map(|metadata| format!("{:?}", metadata.board_type))
            .unwrap_or_else(|| "Unknown".to_string());
        let fix = metadata
            .map(|metadata| format!("{:?}", metadata.fix_banksw))
            .unwrap_or_else(|| "Unknown".to_string());

        if max_frames == 0 {
            println!(
                "[LOAD_OK] rom={label} ngh=0x{ngh:03X} board={board} fix={fix} P={} S={} M={} V={} C={}",
                rom.prom.len(),
                rom.srom.len(),
                rom.mrom.len(),
                rom.vrom.len(),
                rom.crom.len()
            );
            continue;
        }

        let mut machine = system_roms.create_machine();
        machine.load_rom_and_connect(&mut rom);
        let metrics = run_machine(&mut machine, max_frames);
        let control = machine.memory.borrow().system_control_snapshot();
        let final_nonblack = count_nonblack(&machine.video.framebuffer);
        let sample_error = audio_sample_error(metrics.frames_run, metrics.audio_pairs);

        let verdict = if metrics.cpu_error.is_some() {
            runtime_failed += 1;
            "FAIL"
        } else if control.use_cart_vectors
            && metrics.max_nonblack_pixels > 0
            && (metrics.first_audio_frame.is_some()
                || audio_payload == "placeholder"
                || expected_silent_attract)
            && sample_error.abs() <= 16.0
        {
            runtime_passed += 1;
            "PASS"
        } else {
            runtime_review += 1;
            "REVIEW"
        };

        println!(
            "[{verdict}] rom={label} ngh=0x{ngh:03X} board={board} fix={fix} frames={} cart={:?} video={:?} audio={:?} audio_payload={audio_payload} audio_pairs={} cadence_error={sample_error:.3} minmax={}/{} max_nonblack={} final_nonblack={} pcs={} error={}",
            metrics.frames_run,
            metrics.first_cart_frame,
            metrics.first_video_frame,
            metrics.first_audio_frame,
            metrics.audio_pairs,
            metrics.min_audio_pairs,
            metrics.max_audio_pairs,
            metrics.max_nonblack_pixels,
            final_nonblack,
            metrics.sampled_pcs.len(),
            metrics.cpu_error.as_deref().map(compact).unwrap_or("-")
        );
    }

    println!(
        "[SUMMARY] total={} loaded={} load_failed={} runtime_passed={} runtime_review={} runtime_failed={} max_frames={}",
        loaded + load_failed,
        loaded,
        load_failed,
        runtime_passed,
        runtime_review,
        runtime_failed,
        max_frames
    );

    if load_failed > 0 || runtime_failed > 0 {
        std::process::exit(1);
    }
    Ok(())
}

fn collect_rom_paths(dir: &Path, selected_roms: &BTreeSet<String>) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    for entry in
        std::fs::read_dir(dir).map_err(|error| format!("No se pudo leer {:?}: {error}", dir))?
    {
        let entry = entry.map_err(|error| format!("Entrada de directorio inválida: {error}"))?;
        let path = entry.path();
        if path.is_file()
            && is_supported_rom_path(&path)
            && selected_rom_matches(&path, selected_roms)
        {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

fn parse_rom_selection(value: &str) -> BTreeSet<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| {
            name.trim_end_matches(".neo")
                .trim_end_matches(".zip")
                .to_ascii_lowercase()
        })
        .collect()
}

fn is_supported_rom_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("neo") || extension.eq_ignore_ascii_case("zip")
        })
}

fn selected_rom_matches(path: &Path, selected_roms: &BTreeSet<String>) -> bool {
    selected_roms.is_empty()
        || path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_some_and(|stem| selected_roms.contains(&stem.to_ascii_lowercase()))
}

fn load_rom_data(path: &Path) -> Result<RomData, String> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
        .as_deref()
    {
        Some("neo") => RomData::from_neo(path),
        Some("zip") => RomData::from_zip(path),
        _ => Err(format!("Extensión ROM no soportada: {}", path.display())),
    }
}

fn run_machine(machine: &mut NeoGeo, max_frames: usize) -> RuntimeMetrics {
    let mut metrics = RuntimeMetrics {
        min_audio_pairs: usize::MAX,
        ..RuntimeMetrics::default()
    };

    for frame in 0..max_frames {
        apply_standard_stimulus(machine, frame, metrics.first_cart_frame);
        if let Err(error) = machine.step() {
            metrics.cpu_error = Some(error);
            break;
        }
        metrics.frames_run = frame + 1;

        let status = machine.status();
        metrics.sampled_pcs.insert(status.pc);

        let control = machine.memory.borrow().system_control_snapshot();
        if control.use_cart_vectors && metrics.first_cart_frame.is_none() {
            metrics.first_cart_frame = Some(frame + 1);
        }

        let audio = machine.audio_mixer.samples();
        let pairs = audio.len() / 2;
        metrics.audio_pairs += pairs as u64;
        metrics.min_audio_pairs = metrics.min_audio_pairs.min(pairs);
        metrics.max_audio_pairs = metrics.max_audio_pairs.max(pairs);
        if metrics.first_audio_frame.is_none() && audio.iter().any(|sample| *sample != 0) {
            metrics.first_audio_frame = Some(frame + 1);
        }

        if frame % VIDEO_SAMPLE_INTERVAL == 0 || frame + 1 == max_frames {
            let nonblack = count_nonblack(&machine.video.framebuffer);
            metrics.max_nonblack_pixels = metrics.max_nonblack_pixels.max(nonblack);
            if nonblack > 0 && metrics.first_video_frame.is_none() {
                metrics.first_video_frame = Some(frame + 1);
            }
        }
    }

    if metrics.min_audio_pairs == usize::MAX {
        metrics.min_audio_pairs = 0;
    }
    metrics
}

fn apply_standard_stimulus(machine: &mut NeoGeo, frame: usize, cart_frame: Option<usize>) {
    for action in [EmuAction::Coin, EmuAction::Start, EmuAction::A] {
        machine.set_input(action, false);
    }

    let Some(cart_frame) = cart_frame else {
        return;
    };
    let cart_age = frame.saturating_sub(cart_frame);
    match cart_age {
        120 => machine.set_input(EmuAction::Coin, true),
        135 => machine.set_input(EmuAction::Start, true),
        300 => machine.set_input(EmuAction::A, true),
        _ => {}
    }
}

fn is_placeholder_audio_payload(vrom: &[u8]) -> bool {
    let Some(&first) = vrom.first() else {
        return false;
    };
    matches!(first, 0x00 | 0xFF) && vrom.iter().all(|value| *value == first)
}

fn is_expected_silent_attract(label: &str) -> bool {
    let label = label
        .trim_end_matches(".neo")
        .trim_end_matches(".zip")
        .to_ascii_lowercase();

    // Digger Man's prototype attract remains silent in Geolith with the same
    // MVS/Universe BIOS configuration, despite carrying a non-empty V-ROM.
    //
    // Dragon's Heaven is a development-board set whose FBNeo descriptor marks
    // the sound sample ROM as NODUMP (sram.v1), so a silent boot is expected
    // with the available archive data.
    matches!(label.as_str(), "diggerma" | "dragonsh")
}

#[cfg(test)]
mod tests {
    use super::is_expected_silent_attract;

    #[test]
    fn expected_silent_attract_accepts_neo_and_zip_labels() {
        assert!(is_expected_silent_attract("diggerma.neo"));
        assert!(is_expected_silent_attract("diggerma.zip"));
        assert!(is_expected_silent_attract("dragonsh.zip"));
        assert!(!is_expected_silent_attract("mslug.zip"));
    }
}

fn count_nonblack(framebuffer: &[u32]) -> usize {
    framebuffer
        .iter()
        .filter(|pixel| (**pixel & 0x00FF_FFFF) != 0)
        .count()
}

fn audio_sample_error(frames: usize, actual_pairs: u64) -> f64 {
    if frames == 0 {
        return 0.0;
    }
    let expected = frames as f64 * MASTER_CYCLES_PER_FRAME as f64
        / Z80_MASTER_DIV as f64
        / YM2610_TSTATES_PER_SAMPLE as f64;
    actual_pairs as f64 - expected
}

fn compact(value: &str) -> &str {
    value.lines().next().unwrap_or(value)
}

struct SystemRoms {
    bios: Vec<u8>,
    bios_label: String,
    zoom: Option<Vec<u8>>,
    sfix: Option<Vec<u8>>,
    sm1: Option<Vec<u8>>,
}

impl SystemRoms {
    fn load<P: AsRef<Path>>(bios_dir: P) -> Result<Self, String> {
        let dirs = [bios_dir.as_ref()];
        let bios = bios::load_bios_from_multi(&dirs)?
            .ok_or_else(|| format!("No se encontró BIOS en {:?}", bios_dir.as_ref()))?;
        let zoom = bios::load_zoom_rom_for_bios_from_multi(&dirs, &bios.label)?.map(|rom| rom.data);
        let sfix = bios::load_sfix_rom_for_bios_from_multi(&dirs, &bios.label)?.map(|rom| rom.data);
        let sm1 = bios::load_sm1_rom_for_bios_from_multi(&dirs, &bios.label)?.map(|rom| rom.data);

        Ok(Self {
            bios: bios.data,
            bios_label: bios.label,
            zoom,
            sfix,
            sm1,
        })
    }

    fn create_machine(&self) -> NeoGeo {
        let mut machine = NeoGeo::new();
        machine.set_bios(self.bios.clone());
        if let Some(zoom) = &self.zoom {
            machine.set_zoom_rom(zoom.clone());
        }
        if let Some(sfix) = &self.sfix {
            machine.set_sfix_rom(sfix.clone());
        }
        if let Some(sm1) = &self.sm1 {
            machine.set_sm1_rom(sm1.clone());
        }
        machine
    }
}

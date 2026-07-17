use core_emulator::{bios, rom::RomData, NeoGeo};
use std::path::{Path, PathBuf};

struct Firmware {
    label: String,
    bios: Vec<u8>,
    zoom: Vec<u8>,
    sfix: Vec<u8>,
    sm1: Vec<u8>,
    system_is_mvs: bool,
    coin_slots_34_high: bool,
}

fn main() -> Result<(), String> {
    let directory = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("roms_zip"));
    let frames = std::env::args()
        .nth(2)
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|error| format!("Frames invalidos: {error}"))?
        .unwrap_or(300);
    let firmware = load_firmware()?;
    println!(
        "[AUDIT] BIOS={} frames_por_juego={}",
        firmware.label, frames
    );

    let mut paths = zip_paths(&directory)?;
    paths.sort();
    if let Ok(filter) = std::env::var("NGNEON_AUDIT_SETS") {
        let wanted: Vec<String> = filter
            .split(',')
            .map(|name| name.trim().to_ascii_lowercase())
            .filter(|name| !name.is_empty())
            .collect();
        paths.retain(|path| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .is_some_and(|stem| wanted.iter().any(|name| stem.eq_ignore_ascii_case(name)))
        });
    }
    let mut passed = 0usize;
    let mut failed = Vec::new();

    for path in &paths {
        match audit_game(path, frames, &firmware) {
            Ok(summary) => {
                passed += 1;
                println!("[OK] {} {}", display_name(path), summary);
            }
            Err(error) => {
                println!("[FAIL] {}: {}", display_name(path), error);
                failed.push((path.clone(), error));
            }
        }
    }

    println!(
        "[SUMMARY] total={} passed={} failed={}",
        paths.len(),
        passed,
        failed.len()
    );
    for (path, error) in &failed {
        println!("  {}: {}", display_name(path), error);
    }

    if failed.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{} juegos fallaron la auditoria runtime",
            failed.len()
        ))
    }
}

fn audit_game(path: &Path, frames: usize, firmware: &Firmware) -> Result<String, String> {
    let mut rom = RomData::from_zip(path)?;
    let mut neogeo = NeoGeo::new();
    neogeo.set_bios(firmware.bios.clone());
    neogeo.set_zoom_rom(firmware.zoom.clone());
    neogeo.set_sfix_rom(firmware.sfix.clone());
    neogeo.set_sm1_rom(firmware.sm1.clone());
    neogeo.set_system_is_mvs(firmware.system_is_mvs);
    neogeo.set_coin_slots_34_high(firmware.coin_slots_34_high);
    neogeo.load_rom_and_connect(&mut rom);

    let mut previous_hash = framebuffer_hash(&neogeo.video.framebuffer);
    let mut changed_frames = 0usize;
    let mut audio_frames = 0usize;
    for frame in 0..frames {
        neogeo
            .step()
            .map_err(|error| format!("frame {frame}: {error}"))?;
        let hash = framebuffer_hash(&neogeo.video.framebuffer);
        if hash != previous_hash {
            changed_frames += 1;
            previous_hash = hash;
        }
        if neogeo
            .audio_mixer
            .samples()
            .iter()
            .any(|&sample| sample != 0)
        {
            audio_frames += 1;
        }
    }

    let status = neogeo.status();
    if let Some(error) = status.last_error {
        return Err(error);
    }
    Ok(format!(
        "pc=0x{:06X} pbank=0x{:X} video_changes={} audio_frames={}",
        status.pc, status.prom_bank_offset, changed_frames, audio_frames
    ))
}

fn load_firmware() -> Result<Firmware, String> {
    let dirs = ["bios"];
    let candidates = bios::list_available_bios_multi(&dirs)?;
    let requested = std::env::var("NGNEON_AUDIT_BIOS_LABEL").ok();
    let selected = if let Some(label) = requested.as_deref() {
        candidates.iter().find(|candidate| candidate.label == label)
    } else {
        bios::select_bios_for_kind_and_region(
            &candidates,
            bios::BiosKind::Mvs,
            bios::BiosRegion::Europe,
        )
        .or_else(|| candidates.first())
    }
    .ok_or_else(|| "No se encontro una BIOS para la auditoria".to_string())?;

    let label = selected.label.clone();
    let kind = bios::bios_kind(&label);
    let zoom = bios::load_zoom_rom_for_bios_from_multi(&dirs, &label)?
        .map(|image| image.data)
        .unwrap_or_default();
    let (sfix, sm1) = if matches!(kind, bios::BiosKind::Aes) {
        (Vec::new(), Vec::new())
    } else {
        (
            bios::load_sfix_rom_for_bios_from_multi(&dirs, &label)?
                .map(|image| image.data)
                .unwrap_or_default(),
            bios::load_sm1_rom_for_bios_from_multi(&dirs, &label)?
                .map(|image| image.data)
                .unwrap_or_default(),
        )
    };
    let unibios_is_mvs = std::env::var("NGNEON_AUDIT_UNIBIOS_HARDWARE")
        .map(|value| !value.eq_ignore_ascii_case("aes"))
        .unwrap_or(true);

    Ok(Firmware {
        label,
        bios: selected.data.clone(),
        zoom,
        sfix,
        sm1,
        system_is_mvs: !matches!(kind, bios::BiosKind::Aes),
        coin_slots_34_high: match kind {
            bios::BiosKind::Aes => false,
            bios::BiosKind::Unibios => unibios_is_mvs,
            _ => true,
        },
    })
}

fn framebuffer_hash(framebuffer: &[u32]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for &pixel in framebuffer {
        hash ^= pixel as u64;
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    hash
}

fn zip_paths(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = std::fs::read_dir(directory)
        .map_err(|error| format!("No se pudo leer {}: {error}", directory.display()))?;
    let mut paths = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|error| format!("Error leyendo entrada: {error}"))?
            .path();
        if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
        {
            paths.push(path);
        }
    }
    Ok(paths)
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

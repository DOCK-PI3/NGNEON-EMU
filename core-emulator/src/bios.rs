use crate::memory::BIOS_ROM_SIZE;
use std::path::{Path, PathBuf};

const BIOS_EXTENSIONS: &[&str] = &["bin", "rom", "sp1"];
const BIOS_ZIP_NAMES: &[&str] = &["neogeo.zip", "aes.zip", "uni-bios.zip", "unibios.zip"];
pub const ZOOM_ROM_SIZE: usize = 0x20000;
pub const SFIX_ROM_SIZE: usize = 0x20000;
pub const SM1_ROM_SIZE: usize = 0x20000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BiosImage {
    pub data: Vec<u8>,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZoomRom {
    pub data: Vec<u8>,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SfixRom {
    pub data: Vec<u8>,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sm1Rom {
    pub data: Vec<u8>,
    pub label: String,
}

/// Scan the bios directory and return all BIOS candidates, sorted by priority
/// (best match first: MVS > region-specific > AES > UniBIOS).
pub fn list_available_bios<P: AsRef<Path>>(dir: P) -> Result<Vec<BiosImage>, String> {
    let dir = dir.as_ref();
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut candidates = Vec::new();
    for entry in std::fs::read_dir(dir)
        .map_err(|error| format!("No se pudo leer carpeta BIOS {:?}: {error}", dir))?
    {
        let entry = entry.map_err(|error| format!("No se pudo leer entrada BIOS: {error}"))?;
        let path = entry.path();
        if path.is_file() {
            collect_bios_candidates(&path, &mut candidates)?;
        }
    }

    // Sort by priority (best first)
    candidates.sort_by(|a, b| {
        bios_priority(&a.label)
            .cmp(&bios_priority(&b.label))
            .then_with(|| a.label.cmp(&b.label))
    });

    Ok(candidates)
}

/// Load the best BIOS from the given directory (used on startup).
///
/// When `NGNEON_BIOS_HINT` is not set, returns the highest-priority BIOS
/// (MVS `sp-s2.sp1` first). When set, the hint string is matched against
/// BIOS labels (substring search) — matching candidates are promoted to
/// the front while preserving `bios_priority` order within both groups.
pub fn load_bios_from_dir<P: AsRef<Path>>(dir: P) -> Result<Option<BiosImage>, String> {
    let mut candidates = list_available_bios(dir)?;
    let hint = std::env::var("NGNEON_BIOS_HINT")
        .ok()
        .map(|value| value.to_ascii_lowercase())
        .filter(|value| !value.trim().is_empty());

    // Only re-sort when a hint is provided; otherwise keep the
    // bios_priority order from list_available_bios (MVS first).
    if let Some(ref hint) = hint {
        candidates.sort_by(|a, b| {
            bios_hint_priority(&a.label, Some(hint))
                .cmp(&bios_hint_priority(&b.label, Some(hint)))
                .then_with(|| bios_priority(&a.label).cmp(&bios_priority(&b.label)))
                .then_with(|| a.label.cmp(&b.label))
        });
    }

    Ok(candidates.into_iter().next())
}

pub fn load_zoom_rom_from_dir<P: AsRef<Path>>(dir: P) -> Result<Option<ZoomRom>, String> {
    let dir = dir.as_ref();
    if !dir.exists() {
        return Ok(None);
    }

    let mut candidates = Vec::new();
    for entry in std::fs::read_dir(dir)
        .map_err(|error| format!("No se pudo leer carpeta BIOS {:?}: {error}", dir))?
    {
        let entry = entry.map_err(|error| format!("No se pudo leer entrada BIOS: {error}"))?;
        let path = entry.path();
        if path.is_file() {
            collect_zoom_rom_candidates(&path, &mut candidates)?;
        }
    }

    candidates.sort_by(|a, b| a.label.cmp(&b.label));
    Ok(candidates.into_iter().next())
}

/// Scan multiple directories for BIOS candidates and return merged results
/// (deduplicated by label, sorted by priority).
pub fn list_available_bios_multi<P: AsRef<Path>>(dirs: &[P]) -> Result<Vec<BiosImage>, String> {
    let mut all = Vec::new();
    for dir in dirs {
        if let Ok(candidates) = list_available_bios(dir) {
            for c in candidates {
                if !all
                    .iter()
                    .any(|existing: &BiosImage| existing.label == c.label)
                {
                    all.push(c);
                }
            }
        }
    }
    all.sort_by(|a, b| {
        bios_priority(&a.label)
            .cmp(&bios_priority(&b.label))
            .then_with(|| a.label.cmp(&b.label))
    });
    Ok(all)
}

/// Load the best BIOS from multiple directories (tries each dir in order).
pub fn load_bios_from_multi<P: AsRef<Path>>(dirs: &[P]) -> Result<Option<BiosImage>, String> {
    if dirs.is_empty() {
        return load_bios_from_dir("bios");
    }
    // Try explicit hint via NGNEON_BIOS_HINT env var — scan all dirs together
    let hint = std::env::var("NGNEON_BIOS_HINT")
        .ok()
        .map(|value| value.to_ascii_lowercase())
        .filter(|value| !value.trim().is_empty());

    let mut candidates = list_available_bios_multi(dirs)?;

    if let Some(ref hint) = hint {
        candidates.sort_by(|a, b| {
            bios_hint_priority(&a.label, Some(hint))
                .cmp(&bios_hint_priority(&b.label, Some(hint)))
                .then_with(|| bios_priority(&a.label).cmp(&bios_priority(&b.label)))
                .then_with(|| a.label.cmp(&b.label))
        });
    }

    Ok(candidates.into_iter().next())
}

/// Load the ZOOM ROM from multiple directories (tries each dir in order).
pub fn load_zoom_rom_from_multi<P: AsRef<Path>>(dirs: &[P]) -> Result<Option<ZoomRom>, String> {
    for dir in dirs {
        if let Some(zoom) = load_zoom_rom_from_dir(dir)? {
            return Ok(Some(zoom));
        }
    }
    Ok(None)
}

/// Load the ZOOM ROM from the same BIOS archive that supplied `bios_label`
/// when possible, matching Geolith's "one active BIOS zip" behavior.
pub fn load_zoom_rom_for_bios_from_multi<P: AsRef<Path>>(
    dirs: &[P],
    bios_label: &str,
) -> Result<Option<ZoomRom>, String> {
    if let Some(path) = find_archive_for_bios_label(dirs, bios_label) {
        let mut candidates = Vec::new();
        collect_zoom_rom_candidates(&path, &mut candidates)?;
        candidates.sort_by(|a, b| a.label.cmp(&b.label));
        if let Some(zoom) = candidates.into_iter().next() {
            return Ok(Some(zoom));
        }
    }

    load_zoom_rom_from_multi(dirs)
}

/// Load the SFIX ROM from multiple directories (tries each dir in order).
pub fn load_sfix_rom_from_multi<P: AsRef<Path>>(dirs: &[P]) -> Result<Option<SfixRom>, String> {
    for dir in dirs {
        if let Some(sfix) = load_sfix_rom_from_dir(dir)? {
            return Ok(Some(sfix));
        }
    }
    Ok(None)
}

/// Load the SFIX ROM from the same BIOS archive that supplied `bios_label`
/// when possible.
pub fn load_sfix_rom_for_bios_from_multi<P: AsRef<Path>>(
    dirs: &[P],
    bios_label: &str,
) -> Result<Option<SfixRom>, String> {
    if let Some(path) = find_archive_for_bios_label(dirs, bios_label) {
        let mut candidates = Vec::new();
        collect_sfix_rom_candidates(&path, &mut candidates)?;
        candidates.sort_by(|a, b| a.label.cmp(&b.label));
        if let Some(sfix) = candidates.into_iter().next() {
            return Ok(Some(sfix));
        }
    }

    load_sfix_rom_from_multi(dirs)
}

/// Load the SM1 sound BIOS from multiple directories (tries each dir in order).
pub fn load_sm1_rom_from_multi<P: AsRef<Path>>(dirs: &[P]) -> Result<Option<Sm1Rom>, String> {
    for dir in dirs {
        if let Some(sm1) = load_sm1_rom_from_dir(dir)? {
            return Ok(Some(sm1));
        }
    }
    Ok(None)
}

/// Load the SM1 sound BIOS from the same BIOS archive that supplied
/// `bios_label` when possible.
pub fn load_sm1_rom_for_bios_from_multi<P: AsRef<Path>>(
    dirs: &[P],
    bios_label: &str,
) -> Result<Option<Sm1Rom>, String> {
    if let Some(path) = find_archive_for_bios_label(dirs, bios_label) {
        let mut candidates = Vec::new();
        collect_sm1_rom_candidates(&path, &mut candidates)?;
        candidates.sort_by(|a, b| a.label.cmp(&b.label));
        if let Some(sm1) = candidates.into_iter().next() {
            return Ok(Some(sm1));
        }
    }

    load_sm1_rom_from_multi(dirs)
}

pub fn load_sfix_rom_from_dir<P: AsRef<Path>>(dir: P) -> Result<Option<SfixRom>, String> {
    let dir = dir.as_ref();
    if !dir.exists() {
        return Ok(None);
    }

    let mut candidates = Vec::new();
    for entry in std::fs::read_dir(dir)
        .map_err(|error| format!("No se pudo leer carpeta BIOS {:?}: {error}", dir))?
    {
        let entry = entry.map_err(|error| format!("No se pudo leer entrada BIOS: {error}"))?;
        let path = entry.path();
        if path.is_file() {
            collect_sfix_rom_candidates(&path, &mut candidates)?;
        }
    }

    candidates.sort_by(|a, b| a.label.cmp(&b.label));
    Ok(candidates.into_iter().next())
}

pub fn load_sm1_rom_from_dir<P: AsRef<Path>>(dir: P) -> Result<Option<Sm1Rom>, String> {
    let dir = dir.as_ref();
    if !dir.exists() {
        return Ok(None);
    }

    let mut candidates = Vec::new();
    for entry in std::fs::read_dir(dir)
        .map_err(|error| format!("No se pudo leer carpeta BIOS {:?}: {error}", dir))?
    {
        let entry = entry.map_err(|error| format!("No se pudo leer entrada BIOS: {error}"))?;
        let path = entry.path();
        if path.is_file() {
            collect_sm1_rom_candidates(&path, &mut candidates)?;
        }
    }

    candidates.sort_by(|a, b| a.label.cmp(&b.label));
    Ok(candidates.into_iter().next())
}

fn collect_bios_candidates(path: &Path, candidates: &mut Vec<BiosImage>) -> Result<(), String> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "zip" if is_bios_zip_path(path) => collect_zip_bios_candidates(path, candidates),
        "zip" => Ok(()),
        extension if BIOS_EXTENSIONS.contains(&extension) => {
            let bytes = std::fs::read(path)
                .map_err(|error| format!("No se pudo leer BIOS {:?}: {error}", path))?;
            if let Some(data) = prepare_bios_data(bytes) {
                candidates.push(BiosImage {
                    data,
                    label: path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("bios")
                        .to_string(),
                });
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn collect_zoom_rom_candidates(path: &Path, candidates: &mut Vec<ZoomRom>) -> Result<(), String> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if extension == "zip" && is_bios_zip_path(path) {
        return collect_zip_zoom_rom_candidates(path, candidates);
    }

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if file_name != "000-lo.lo" {
        return Ok(());
    }

    let bytes = std::fs::read(path)
        .map_err(|error| format!("No se pudo leer tabla L0 {:?}: {error}", path))?;
    if let Some(data) = prepare_zoom_rom_data(bytes) {
        candidates.push(ZoomRom {
            data,
            label: path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("000-lo.lo")
                .to_string(),
        });
    }

    Ok(())
}

fn collect_sfix_rom_candidates(path: &Path, candidates: &mut Vec<SfixRom>) -> Result<(), String> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if extension == "zip" && is_bios_zip_path(path) {
        return collect_zip_sfix_rom_candidates(path, candidates);
    }

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if file_name != "sfix.sfix" {
        return Ok(());
    }

    let bytes =
        std::fs::read(path).map_err(|error| format!("No se pudo leer SFIX {:?}: {error}", path))?;
    if let Some(data) = prepare_sfix_rom_data(bytes) {
        candidates.push(SfixRom {
            data,
            label: path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("sfix.sfix")
                .to_string(),
        });
    }

    Ok(())
}

fn collect_sm1_rom_candidates(path: &Path, candidates: &mut Vec<Sm1Rom>) -> Result<(), String> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if extension == "zip" && is_bios_zip_path(path) {
        return collect_zip_sm1_rom_candidates(path, candidates);
    }

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if file_name != "sm1.sm1" {
        return Ok(());
    }

    let bytes =
        std::fs::read(path).map_err(|error| format!("No se pudo leer SM1 {:?}: {error}", path))?;
    if let Some(data) = prepare_sm1_rom_data(bytes) {
        candidates.push(Sm1Rom {
            data,
            label: path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("sm1.sm1")
                .to_string(),
        });
    }

    Ok(())
}

fn collect_zip_bios_candidates(path: &Path, candidates: &mut Vec<BiosImage>) -> Result<(), String> {
    let file = std::fs::File::open(path)
        .map_err(|error| format!("No se pudo abrir {:?}: {error}", path))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|error| format!("No se pudo leer {:?}: {error}", path))?;

    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(|error| error.to_string())?;
        if file.is_dir() {
            continue;
        }

        let name = file.name().replace('\\', "/");
        let file_name = name
            .rsplit('/')
            .next()
            .unwrap_or(&name)
            .to_ascii_lowercase();
        if !is_bios_candidate_name(&file_name) {
            continue;
        }

        let mut bytes = Vec::new();
        use std::io::Read;
        file.read_to_end(&mut bytes)
            .map_err(|error| error.to_string())?;
        if let Some(data) = prepare_bios_data(bytes) {
            candidates.push(BiosImage {
                data,
                label: format!(
                    "{}:{}",
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("bios.zip"),
                    file_name
                ),
            });
        }
    }

    Ok(())
}

fn collect_zip_zoom_rom_candidates(
    path: &Path,
    candidates: &mut Vec<ZoomRom>,
) -> Result<(), String> {
    let file = std::fs::File::open(path)
        .map_err(|error| format!("No se pudo abrir {:?}: {error}", path))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|error| format!("No se pudo leer {:?}: {error}", path))?;

    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(|error| error.to_string())?;
        if file.is_dir() {
            continue;
        }

        let name = file.name().replace('\\', "/");
        let file_name = name
            .rsplit('/')
            .next()
            .unwrap_or(&name)
            .to_ascii_lowercase();
        if file_name != "000-lo.lo" {
            continue;
        }

        let mut bytes = Vec::new();
        use std::io::Read;
        file.read_to_end(&mut bytes)
            .map_err(|error| error.to_string())?;
        if let Some(data) = prepare_zoom_rom_data(bytes) {
            candidates.push(ZoomRom {
                data,
                label: format!(
                    "{}:{}",
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("bios.zip"),
                    file_name
                ),
            });
        }
    }

    Ok(())
}

fn collect_zip_sfix_rom_candidates(
    path: &Path,
    candidates: &mut Vec<SfixRom>,
) -> Result<(), String> {
    let file = std::fs::File::open(path)
        .map_err(|error| format!("No se pudo abrir {:?}: {error}", path))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|error| format!("No se pudo leer {:?}: {error}", path))?;

    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(|error| error.to_string())?;
        if file.is_dir() {
            continue;
        }

        let name = file.name().replace('\\', "/");
        let file_name = name
            .rsplit('/')
            .next()
            .unwrap_or(&name)
            .to_ascii_lowercase();
        if file_name != "sfix.sfix" {
            continue;
        }

        let mut bytes = Vec::new();
        use std::io::Read;
        file.read_to_end(&mut bytes)
            .map_err(|error| error.to_string())?;
        if let Some(data) = prepare_sfix_rom_data(bytes) {
            candidates.push(SfixRom {
                data,
                label: format!(
                    "{}:{}",
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("bios.zip"),
                    file_name
                ),
            });
        }
    }

    Ok(())
}

fn collect_zip_sm1_rom_candidates(path: &Path, candidates: &mut Vec<Sm1Rom>) -> Result<(), String> {
    let file = std::fs::File::open(path)
        .map_err(|error| format!("No se pudo abrir {:?}: {error}", path))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|error| format!("No se pudo leer {:?}: {error}", path))?;

    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(|error| error.to_string())?;
        if file.is_dir() {
            continue;
        }

        let name = file.name().replace('\\', "/");
        let file_name = name
            .rsplit('/')
            .next()
            .unwrap_or(&name)
            .to_ascii_lowercase();
        if file_name != "sm1.sm1" {
            continue;
        }

        let mut bytes = Vec::new();
        use std::io::Read;
        file.read_to_end(&mut bytes)
            .map_err(|error| error.to_string())?;
        if let Some(data) = prepare_sm1_rom_data(bytes) {
            candidates.push(Sm1Rom {
                data,
                label: format!(
                    "{}:{}",
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("bios.zip"),
                    file_name
                ),
            });
        }
    }

    Ok(())
}

fn is_bios_candidate_name(name: &str) -> bool {
    if matches!(name, "000-lo.lo" | "sfix.sfix" | "sm1.sm1") {
        return false;
    }

    name.starts_with("sp-")
        || name.starts_with("sp1")
        || name.starts_with("uni-bios")
        || matches!(
            name,
            "japan-j3.bin" | "vs-bios.rom" | "neo-epo.bin" | "neo-po.bin"
        )
}

fn is_bios_zip_path(path: &Path) -> bool {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    BIOS_ZIP_NAMES.contains(&file_name.as_str())
        || file_name.starts_with("uni-bios")
        || file_name.starts_with("unibios")
}

fn find_archive_for_bios_label<P: AsRef<Path>>(dirs: &[P], bios_label: &str) -> Option<PathBuf> {
    let archive_name = bios_label
        .split_once(':')
        .map(|(archive, _)| archive.to_ascii_lowercase())?;

    for dir in dirs {
        let candidate = dir.as_ref().join(&archive_name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    None
}

fn prepare_bios_data(mut data: Vec<u8>) -> Option<Vec<u8>> {
    if data.len() < 8 {
        return None;
    }

    if data.len() > BIOS_ROM_SIZE {
        data.truncate(BIOS_ROM_SIZE);
    }

    normalize_bios_byte_order(&mut data);
    Some(mirror_to_bios_region(&data))
}

fn prepare_zoom_rom_data(data: Vec<u8>) -> Option<Vec<u8>> {
    (data.len() == ZOOM_ROM_SIZE).then_some(data)
}

fn prepare_sfix_rom_data(data: Vec<u8>) -> Option<Vec<u8>> {
    (data.len() == SFIX_ROM_SIZE).then_some(data)
}

fn prepare_sm1_rom_data(data: Vec<u8>) -> Option<Vec<u8>> {
    (data.len() == SM1_ROM_SIZE).then_some(data)
}

fn normalize_bios_byte_order(data: &mut [u8]) {
    if data.len() < 4 {
        return;
    }

    let raw_sp = read_u32_be(data, 0);
    let swapped_sp = u32::from_be_bytes([data[1], data[0], data[3], data[2]]);
    if !looks_like_stack(raw_sp) && looks_like_stack(swapped_sp) {
        for word in data.chunks_exact_mut(2) {
            word.swap(0, 1);
        }
    }
}

fn mirror_to_bios_region(data: &[u8]) -> Vec<u8> {
    let mut mirrored = Vec::with_capacity(BIOS_ROM_SIZE);
    while mirrored.len() < BIOS_ROM_SIZE {
        let remaining = BIOS_ROM_SIZE - mirrored.len();
        let copy_len = data.len().min(remaining);
        mirrored.extend_from_slice(&data[..copy_len]);
    }
    mirrored
}

fn looks_like_stack(value: u32) -> bool {
    value.is_multiple_of(2) && (0x0010_0000..=0x0010_FFFE).contains(&value)
}

fn read_u32_be(data: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

fn bios_priority(label: &str) -> (u8, String) {
    let lower = label.to_ascii_lowercase();
    // Match Geolith's libretro behavior: Universe BIOS is loaded from
    // neogeo.zip, while aes.zip is reserved for real AES BIOS images.
    let priority = if lower.contains("neogeo.zip:uni-bios_4_0") {
        0
    } else if lower.contains("neogeo.zip:uni-bios") {
        1
    } else if lower.contains("uni-bios_4_0") {
        2
    } else if lower.contains("uni-bios") {
        3
    } else if lower.contains("sp-s2.sp1") {
        4
    } else if lower.contains("sp-s.sp1") {
        5
    } else if lower.contains("sp-u") || lower.contains("sp1-u") {
        6
    } else if lower.contains("sp-e") {
        7
    } else if lower.contains("sp-j") || lower.contains("japan") {
        8
    } else {
        9
    };

    (priority, lower)
}

fn bios_hint_priority(label: &str, hint: Option<&str>) -> u8 {
    match hint {
        Some(hint) if label.to_ascii_lowercase().contains(hint) => 0,
        Some(_) => 1,
        None => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    #[test]
    fn skips_non_bios_zip_entries() {
        assert!(!is_bios_candidate_name("sfix.sfix"));
        assert!(!is_bios_candidate_name("sm1.sm1"));
        assert!(is_bios_candidate_name("sp-s2.sp1"));
        assert!(is_bios_candidate_name("uni-bios_4_0.rom"));
    }

    #[test]
    fn only_known_system_archives_are_scanned_as_bios_zips() {
        assert!(is_bios_zip_path(Path::new("neogeo.zip")));
        assert!(is_bios_zip_path(Path::new("aes.zip")));
        assert!(is_bios_zip_path(Path::new("uni-bios-40.zip")));
        assert!(!is_bios_zip_path(Path::new("kof2002.zip")));
        assert!(!is_bios_zip_path(Path::new("mslug3.zip")));
    }

    #[test]
    fn ignores_bios_entries_inside_game_zip_when_scanning_firmware() {
        let dir = unique_temp_dir("ngneon-bios-game-zip-filter");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("kof2002.zip");
        let file = std::fs::File::create(&path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("uni-bios_4_0.rom", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&[0x00; 8]).unwrap();
        zip.start_file("000-lo.lo", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&vec![0x5A; ZOOM_ROM_SIZE]).unwrap();
        zip.start_file("sfix.sfix", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&vec![0xC7; SFIX_ROM_SIZE]).unwrap();
        zip.finish().unwrap();

        assert!(list_available_bios(&dir).unwrap().is_empty());
        assert!(load_zoom_rom_from_dir(&dir).unwrap().is_none());
        assert!(load_sfix_rom_from_dir(&dir).unwrap().is_none());
        assert!(load_sm1_rom_from_dir(&dir).unwrap().is_none());

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn mirrors_small_bios_to_region_size() {
        let data = prepare_bios_data(vec![0x00, 0x10, 0xF3, 0x00, 0xAA, 0x55, 0x12, 0x34]).unwrap();

        assert_eq!(data.len(), BIOS_ROM_SIZE);
        assert_eq!(
            &data[..8],
            &[0x00, 0x10, 0xF3, 0x00, 0xAA, 0x55, 0x12, 0x34]
        );
        assert_eq!(
            &data[8..16],
            &[0x00, 0x10, 0xF3, 0x00, 0xAA, 0x55, 0x12, 0x34]
        );
    }

    #[test]
    fn normalizes_word_byte_swapped_bios() {
        let data = prepare_bios_data(vec![0x10, 0x00, 0x00, 0xF3, 0xC0, 0x00, 0x02, 0x04]).unwrap();

        assert_eq!(
            &data[..8],
            &[0x00, 0x10, 0xF3, 0x00, 0x00, 0xC0, 0x04, 0x02]
        );
    }

    #[test]
    fn bios_hint_prioritizes_matching_labels() {
        assert_eq!(
            bios_hint_priority("neogeo.zip:uni-bios_4_0.rom", Some("uni-bios_4_0")),
            0
        );
        assert_eq!(
            bios_hint_priority("neogeo.zip:sp-s2.sp1", Some("uni-bios_4_0")),
            1
        );
        assert_eq!(bios_hint_priority("neogeo.zip:sp-s2.sp1", None), 0);
    }

    #[test]
    fn loads_l0_zoom_rom_from_bios_zip() {
        let dir = unique_temp_dir("ngneon-bios-zoom");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("neogeo.zip");
        let file = std::fs::File::create(&path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("000-lo.lo", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&vec![0x5A; ZOOM_ROM_SIZE]).unwrap();
        zip.start_file("sp-s2.sp1", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&[0x00; 8]).unwrap();
        zip.finish().unwrap();

        let zoom_rom = load_zoom_rom_from_dir(&dir).unwrap().unwrap();

        assert_eq!(zoom_rom.label, "neogeo.zip:000-lo.lo");
        assert_eq!(zoom_rom.data.len(), ZOOM_ROM_SIZE);
        assert_eq!(zoom_rom.data[0], 0x5A);

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn loads_sfix_rom_from_bios_zip() {
        let dir = unique_temp_dir("ngneon-bios-sfix");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("neogeo.zip");
        let file = std::fs::File::create(&path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("sfix.sfix", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&vec![0xC7; SFIX_ROM_SIZE]).unwrap();
        zip.start_file("000-lo.lo", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&vec![0x5A; ZOOM_ROM_SIZE]).unwrap();
        zip.finish().unwrap();

        let sfix_rom = load_sfix_rom_from_dir(&dir).unwrap().unwrap();

        assert_eq!(sfix_rom.label, "neogeo.zip:sfix.sfix");
        assert_eq!(sfix_rom.data.len(), SFIX_ROM_SIZE);
        assert_eq!(sfix_rom.data[0], 0xC7);

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn loads_sm1_rom_from_bios_zip() {
        let dir = unique_temp_dir("ngneon-bios-sm1");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("neogeo.zip");
        let file = std::fs::File::create(&path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("sm1.sm1", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&vec![0xA9; SM1_ROM_SIZE]).unwrap();
        zip.finish().unwrap();

        let sm1_rom = load_sm1_rom_from_dir(&dir).unwrap().unwrap();

        assert_eq!(sm1_rom.label, "neogeo.zip:sm1.sm1");
        assert_eq!(sm1_rom.data.len(), SM1_ROM_SIZE);
        assert_eq!(sm1_rom.data[0], 0xA9);

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rejects_wrong_sized_l0_zoom_rom() {
        assert!(prepare_zoom_rom_data(vec![0; ZOOM_ROM_SIZE - 1]).is_none());
        assert!(prepare_zoom_rom_data(vec![0; ZOOM_ROM_SIZE]).is_some());
    }

    #[test]
    fn rejects_wrong_sized_sfix_rom() {
        assert!(prepare_sfix_rom_data(vec![0; SFIX_ROM_SIZE - 1]).is_none());
        assert!(prepare_sfix_rom_data(vec![0; SFIX_ROM_SIZE]).is_some());
    }

    #[test]
    fn rejects_wrong_sized_sm1_rom() {
        assert!(prepare_sm1_rom_data(vec![0; SM1_ROM_SIZE - 1]).is_none());
        assert!(prepare_sm1_rom_data(vec![0; SM1_ROM_SIZE]).is_some());
    }

    fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "{}-{}-{}",
            prefix,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        path
    }

    /// Integration test: verify that the actual BIOS files on disk are detected
    /// and that UniBIOS is selected by the priority system.
    ///
    /// Note: `cargo test -p core-emulator` runs from `core-emulator/` subdirectory,
    /// so we use `../bios` to reference the workspace root's bios/ directory.
    #[test]
    fn real_bios_directory_detects_labels_and_prefers_unibios() {
        let bios_path = "../bios";
        let bios_dir = std::path::Path::new(bios_path);
        if !bios_dir.exists() {
            eprintln!("SKIP: {bios_path}/ directory not found");
            return;
        }

        // List all available BIOS images
        let bios_list = list_available_bios(bios_path).unwrap();
        assert!(
            !bios_list.is_empty(),
            "{bios_path}/ directory should contain at least one BIOS"
        );

        eprintln!("--- BIOS found in {bios_path}/ ---");
        for (i, bios) in bios_list.iter().enumerate() {
            eprintln!(
                "  {}. {} ({} bytes) priority={:?}",
                i + 1,
                bios.label,
                bios.data.len(),
                bios_priority(&bios.label)
            );
        }

        // The first entry should be UniBIOS (priority 0 or 1)
        let first_label = &bios_list[0].label;
        assert!(
            first_label.to_ascii_lowercase().contains("uni-bios"),
            "First BIOS should be UniBIOS (priority 0/1), got: {}",
            first_label
        );

        // Verify all entries have valid BIOS data (mirrored to BIOS_ROM_SIZE)
        for bios in &bios_list {
            assert!(
                bios.data.len() == BIOS_ROM_SIZE,
                "BIOS '{}' should be mirrored to {} bytes, got {}",
                bios.label,
                BIOS_ROM_SIZE,
                bios.data.len()
            );
        }
    }

    /// Integration test: verify that multi-directory scanning works correctly
    /// with the actual bios/ and roms/ directories.
    ///
    /// Note: `cargo test -p core-emulator` runs from `core-emulator/` subdirectory,
    /// so we use `../bios` to reference the workspace root's bios/ directory.
    #[test]
    fn real_bios_multi_directory_scanning() {
        let bios_path = "../bios";
        let roms_path = "../roms";
        let dirs = [bios_path, roms_path];

        let has_bios = std::path::Path::new(bios_path).exists();
        let has_roms = std::path::Path::new(roms_path).exists();
        if !has_bios && !has_roms {
            eprintln!("SKIP: neither {bios_path}/ nor {roms_path}/ directories found");
            return;
        }

        // Multi-directory scan
        let all = list_available_bios_multi(&dirs).unwrap();
        assert!(
            !all.is_empty(),
            "Should find BIOS in at least one directory"
        );

        eprintln!("--- BIOS found across {bios_path}/ + {roms_path}/ ---");
        for (i, bios) in all.iter().enumerate() {
            eprintln!("  {}. {} ({} bytes)", i + 1, bios.label, bios.data.len());
        }

        // load_bios_from_multi should return the best one
        let best = load_bios_from_multi(&dirs).unwrap();
        assert!(
            best.is_some(),
            "Should load a BIOS from multi-directory scan"
        );
        let best = best.unwrap();
        assert!(
            best.label.to_ascii_lowercase().contains("uni-bios"),
            "Best BIOS should be UniBIOS, got: {}",
            best.label
        );
        eprintln!(
            "\nBest BIOS selected: {} ({} bytes)",
            best.label,
            best.data.len()
        );

        // Single directory scan of bios/ only (should find the same content)
        let single = list_available_bios(bios_path).unwrap();
        // Every single-directory entry should appear in the multi result
        for single_bios in &single {
            assert!(
                all.iter().any(|m| m.label == single_bios.label),
                "Multi-directory result should include '{}' from {bios_path}/",
                single_bios.label
            );
        }
    }
}

use core_emulator::rom::RomData;
use std::path::{Path, PathBuf};

#[derive(Default)]
struct BankResult {
    exact: usize,
    different: Vec<String>,
}

fn main() -> Result<(), String> {
    let zip_dir = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("roms_zip"));
    let neo_dir = std::env::args_os()
        .nth(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("roms"));

    let mut zip_paths = rom_paths(&zip_dir, "zip")?;
    zip_paths.sort();
    let mut checked = 0usize;
    let mut missing_reference = Vec::new();
    let mut result = BankResult::default();

    for zip_path in zip_paths {
        let stem = zip_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| format!("Nombre ZIP invalido: {}", zip_path.display()))?;
        let neo_path = neo_dir.join(format!("{stem}.neo"));
        if !neo_path.is_file() {
            missing_reference.push(stem.to_string());
            continue;
        }

        let zip = RomData::from_zip(&zip_path)
            .map_err(|error| format!("{}: {error}", zip_path.display()))?;
        let neo = RomData::from_neo(&neo_path)
            .map_err(|error| format!("{}: {error}", neo_path.display()))?;
        checked += 1;

        let mut differences = [
            compare_bank("P", &zip.prom, &neo.prom),
            compare_bank("C", &zip.crom, &neo.crom),
            compare_bank("S", &zip.srom, &neo.srom),
            compare_bank("M", &zip.mrom, &neo.mrom),
            compare_bank("V", &zip.vrom, &neo.vrom),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
        if zip.vrom_b_offset != neo.vrom_b_offset {
            differences.push(format!("VB={}/{}", zip.vrom_b_offset, neo.vrom_b_offset));
        }

        if differences.is_empty() {
            result.exact += 1;
            println!("[EXACT] {stem}");
        } else {
            let detail = differences.join(" ");
            println!("[DIFF] {stem} {detail}");
            result.different.push(format!("{stem}: {detail}"));
        }
    }

    println!(
        "[SUMMARY] checked={} exact={} different={} missing_reference={}",
        checked,
        result.exact,
        result.different.len(),
        missing_reference.len()
    );
    for difference in &result.different {
        println!("  {difference}");
    }
    if !missing_reference.is_empty() {
        println!("[MISSING] {}", missing_reference.join(","));
    }

    Ok(())
}

fn compare_bank(label: &str, left: &[u8], right: &[u8]) -> Option<String> {
    if left == right {
        return None;
    }

    let common = left.len().min(right.len());
    let matches = left[..common]
        .iter()
        .zip(&right[..common])
        .filter(|(left, right)| left == right)
        .count();
    let percentage = if common == 0 {
        100.0
    } else {
        matches as f64 * 100.0 / common as f64
    };
    Some(format!(
        "{label}={}/{},match={percentage:.2}%",
        left.len(),
        right.len()
    ))
}

fn rom_paths(directory: &Path, extension: &str) -> Result<Vec<PathBuf>, String> {
    let entries = std::fs::read_dir(directory)
        .map_err(|error| format!("No se pudo leer {}: {error}", directory.display()))?;
    let mut paths = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|error| format!("Error leyendo entrada: {error}"))?
            .path();
        if path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case(extension))
        {
            paths.push(path);
        }
    }
    Ok(paths)
}

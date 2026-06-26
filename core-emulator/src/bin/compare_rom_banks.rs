use core_emulator::rom::RomData;
use std::path::{Path, PathBuf};

fn main() -> Result<(), String> {
    let left_path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or_else(|| "Uso: compare_rom_banks <left.neo|zip> <right.neo|zip>".to_string())?;
    let right_path = std::env::args_os()
        .nth(2)
        .map(PathBuf::from)
        .ok_or_else(|| "Uso: compare_rom_banks <left.neo|zip> <right.neo|zip>".to_string())?;

    let left = load_rom(&left_path)?;
    let right = load_rom(&right_path)?;

    println!("LEFT  {:?}: {}", left_path, bank_summary(&left));
    println!("RIGHT {:?}: {}", right_path, bank_summary(&right));
    compare_bank("P", &left.prom, &right.prom);
    compare_bank("C", &left.crom, &right.crom);
    compare_bank("S", &left.srom, &right.srom);
    compare_bank("M", &left.mrom, &right.mrom);
    compare_bank("V", &left.vrom, &right.vrom);

    Ok(())
}

fn load_rom(path: &Path) -> Result<RomData, String> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
        .as_deref()
    {
        Some("zip") => RomData::from_zip(path),
        _ => RomData::from_neo(path),
    }
}

fn bank_summary(rom: &RomData) -> String {
    format!(
        "P={} C={} S={} M={} V={} VB={}",
        rom.prom.len(),
        rom.crom.len(),
        rom.srom.len(),
        rom.mrom.len(),
        rom.vrom.len(),
        rom.vrom_b_offset
    )
}

fn compare_bank(label: &str, left: &[u8], right: &[u8]) {
    let len = left.len().min(right.len());
    let matches = left
        .iter()
        .zip(right.iter())
        .filter(|(left, right)| left == right)
        .count();
    let first_diff = left
        .iter()
        .zip(right.iter())
        .position(|(left, right)| left != right);
    let extra = left.len().abs_diff(right.len());

    println!(
        "{label}: common={len} matches={} ({:.2}%) first_diff={} size_delta={}",
        matches,
        if len == 0 {
            100.0
        } else {
            matches as f64 * 100.0 / len as f64
        },
        first_diff
            .map(|offset| format!("0x{offset:X}"))
            .unwrap_or_else(|| "none".to_string()),
        extra
    );

    if label == "C" && len >= 128 {
        compare_c_tiles(left, right);
    }
}

fn compare_c_tiles(left: &[u8], right: &[u8]) {
    let tiles = left.len().min(right.len()) / 128;
    let identical = (0..tiles)
        .filter(|tile| {
            let start = tile * 128;
            left[start..start + 128] == right[start..start + 128]
        })
        .count();
    println!(
        "C tiles: total={} identical={} ({:.2}%)",
        tiles,
        identical,
        if tiles == 0 {
            100.0
        } else {
            identical as f64 * 100.0 / tiles as f64
        }
    );
    print_first_diff_window(left, right);
    print_best_quad_permutations(left, right);
    print_best_tile_offset(left, right, tiles);
}

fn print_first_diff_window(left: &[u8], right: &[u8]) {
    let Some(diff) = left
        .iter()
        .zip(right.iter())
        .position(|(left, right)| left != right)
    else {
        return;
    };
    let start = diff.saturating_sub(16);
    let end = (diff + 48).min(left.len()).min(right.len());
    println!("C first diff window 0x{start:X}..0x{end:X}");
    println!("  left : {}", hex_line(&left[start..end]));
    println!("  right: {}", hex_line(&right[start..end]));
}

fn hex_line(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn print_best_quad_permutations(left: &[u8], right: &[u8]) {
    const PERMS: [[usize; 4]; 24] = [
        [0, 1, 2, 3],
        [0, 1, 3, 2],
        [0, 2, 1, 3],
        [0, 2, 3, 1],
        [0, 3, 1, 2],
        [0, 3, 2, 1],
        [1, 0, 2, 3],
        [1, 0, 3, 2],
        [1, 2, 0, 3],
        [1, 2, 3, 0],
        [1, 3, 0, 2],
        [1, 3, 2, 0],
        [2, 0, 1, 3],
        [2, 0, 3, 1],
        [2, 1, 0, 3],
        [2, 1, 3, 0],
        [2, 3, 0, 1],
        [2, 3, 1, 0],
        [3, 0, 1, 2],
        [3, 0, 2, 1],
        [3, 1, 0, 2],
        [3, 1, 2, 0],
        [3, 2, 0, 1],
        [3, 2, 1, 0],
    ];
    let quads = left.len().min(right.len()) / 4;
    let mut best = ([0, 1, 2, 3], 0usize);
    for perm in PERMS {
        let mut matches = 0usize;
        for quad in 0..quads {
            let base = quad * 4;
            for byte in 0..4 {
                if left[base + byte] == right[base + perm[byte]] {
                    matches += 1;
                }
            }
        }
        if matches > best.1 {
            best = (perm, matches);
        }
    }
    let total = quads * 4;
    println!(
        "C best local 4-byte permutation {:?}: {} / {} ({:.2}%)",
        best.0,
        best.1,
        total,
        if total == 0 {
            100.0
        } else {
            best.1 as f64 * 100.0 / total as f64
        }
    );
}

fn print_best_tile_offset(left: &[u8], right: &[u8], tiles: usize) {
    let sample_tiles = tiles.min(4096);
    let max_offset = tiles.min(65536);
    let mut best = (0usize, 0usize);
    for offset in 0..max_offset {
        let mut matches = 0usize;
        for tile in 0..sample_tiles {
            let left_start = tile * 128;
            let right_start = ((tile + offset) % tiles) * 128;
            if left[left_start..left_start + 128] == right[right_start..right_start + 128] {
                matches += 1;
            }
        }
        if matches > best.1 {
            best = (offset, matches);
        }
    }
    println!(
        "C best tile offset over first {} tiles: offset={} identical={}",
        sample_tiles, best.0, best.1
    );
}

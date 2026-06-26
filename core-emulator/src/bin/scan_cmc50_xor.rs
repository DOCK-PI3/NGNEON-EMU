use core_emulator::cmc::{
    decrypt_cmc42_graphics, decrypt_cmc50_graphics, interleave_cmc_graphics_banks,
};
use core_emulator::rom::RomData;
use std::cmp::Reverse;
use std::io::Read;
use std::path::{Path, PathBuf};

fn main() -> Result<(), String> {
    let reference_path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or_else(|| {
            "Uso: scan_cmc50_xor <reference.neo> <encrypted.zip> [cmc42|cmc50] [xor,...]"
                .to_string()
        })?;
    let zip_path = std::env::args_os()
        .nth(2)
        .map(PathBuf::from)
        .ok_or_else(|| {
            "Uso: scan_cmc50_xor <reference.neo> <encrypted.zip> [cmc42|cmc50] [xor,...]"
                .to_string()
        })?;
    let chip = std::env::args()
        .nth(3)
        .unwrap_or_else(|| "cmc50".to_string());
    let requested_xors = std::env::args()
        .nth(4)
        .map(|value| parse_xor_list(&value))
        .transpose()?
        .unwrap_or_else(|| (0u8..=u8::MAX).collect());

    let reference = RomData::from_neo(&reference_path)?;
    let encrypted = load_encrypted_crom_from_zip(&zip_path)?;
    println!(
        "reference C={} encrypted C={}",
        reference.crom.len(),
        encrypted.len()
    );

    let mut best = Vec::new();
    for extra_xor in requested_xors {
        let decrypted = match chip.as_str() {
            "cmc42" => decrypt_cmc42_graphics(&encrypted, extra_xor),
            "cmc50" => decrypt_cmc50_graphics(&encrypted, extra_xor),
            other => return Err(format!("Chip CMC no soportado: {other}")),
        };
        let score = compare_sampled(&reference.crom, &decrypted);
        best.push((extra_xor, score));
    }

    best.sort_by_key(|(_, score)| Reverse(score.matches));
    for (rank, (extra_xor, score)) in best.iter().take(16).enumerate() {
        println!(
            "#{:02} xor=0x{:02X} sampled_matches={} / {} ({:.2}%) first_diff={}",
            rank + 1,
            extra_xor,
            score.matches,
            score.total,
            if score.total == 0 {
                100.0
            } else {
                score.matches as f64 * 100.0 / score.total as f64
            },
            score
                .first_diff
                .map(|offset| format!("0x{offset:X}"))
                .unwrap_or_else(|| "none".to_string())
        );
    }

    Ok(())
}

fn parse_xor_list(value: &str) -> Result<Vec<u8>, String> {
    value
        .split(',')
        .map(|item| {
            let item = item.trim();
            let digits = item
                .strip_prefix("0x")
                .or_else(|| item.strip_prefix("0X"))
                .unwrap_or(item);
            u8::from_str_radix(digits, 16)
                .map_err(|error| format!("XOR CMC inválido '{item}': {error}"))
        })
        .collect()
}

fn load_encrypted_crom_from_zip(path: &Path) -> Result<Vec<u8>, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("Error abriendo zip: {e}"))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Error leyendo zip: {e}"))?;
    let mut graphics_banks = Vec::new();
    let has_root_entries = (0..archive.len()).any(|index| {
        archive
            .by_index(index)
            .ok()
            .is_some_and(|entry| !entry.is_dir() && !entry.name().contains(['/', '\\']))
    });

    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(|e| e.to_string())?;
        if file.is_dir() {
            continue;
        }

        let name = file.name().to_ascii_lowercase();
        if has_root_entries && name.contains(['/', '\\']) {
            continue;
        }
        if let Some(bank_index) = graphics_bank_index(&name) {
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).map_err(|e| e.to_string())?;
            graphics_banks.push((bank_index, buf));
        }
    }

    if graphics_banks.is_empty() {
        return Err("El zip no contiene bancos C-ROM reconocibles".to_string());
    }

    Ok(interleave_cmc_graphics_banks(graphics_banks))
}

fn graphics_bank_index(name: &str) -> Option<u8> {
    let file_name = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let (stem, extension) = file_name
        .rsplit_once('.')
        .map_or((file_name, ""), |(stem, ext)| (stem, ext));
    let chip = stem.rsplit(['-', '_']).next().unwrap_or(stem);

    chip.strip_prefix('c')
        .or_else(|| extension.strip_prefix('c'))
        .and_then(parse_leading_index)
}

fn parse_leading_index(value: &str) -> Option<u8> {
    let digits_len = value
        .bytes()
        .take_while(|byte| byte.is_ascii_digit())
        .count();

    value
        .get(..digits_len)?
        .parse::<u8>()
        .ok()
        .filter(|index| *index > 0)
}

#[derive(Clone, Copy)]
struct Score {
    matches: usize,
    total: usize,
    first_diff: Option<usize>,
}

fn compare_sampled(reference: &[u8], candidate: &[u8]) -> Score {
    let len = reference.len().min(candidate.len());
    let sample_stride = if len > 0x10_0000 { 16 } else { 1 };
    let mut matches = 0usize;
    let mut total = 0usize;
    let mut first_diff = None;

    for offset in (0..len).step_by(sample_stride) {
        total += 1;
        if reference[offset] == candidate[offset] {
            matches += 1;
        } else if first_diff.is_none() {
            first_diff = Some(offset);
        }
    }

    Score {
        matches,
        total,
        first_diff,
    }
}

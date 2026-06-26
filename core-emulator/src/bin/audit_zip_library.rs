use core_emulator::rom::RomData;
use std::path::{Path, PathBuf};

fn main() -> Result<(), String> {
    let directory = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("roms_zip"));

    let mut paths = zip_paths(&directory)?;
    paths.sort();

    let mut loaded = 0usize;
    let mut failed = Vec::new();

    for path in &paths {
        match RomData::from_zip(path) {
            Ok(rom) => {
                loaded += 1;
                let metadata = rom
                    .metadata
                    .as_ref()
                    .map(|metadata| {
                        format!(
                            "NGH={:03X} board={:?} fix={:?}",
                            metadata.ngh, metadata.board_type, metadata.fix_banksw
                        )
                    })
                    .unwrap_or_else(|| "NGH=? board=? fix=?".to_string());
                println!(
                    "[OK] {} P={} S={} M={} V={} C={} {}",
                    display_name(path),
                    rom.prom.len(),
                    rom.srom.len(),
                    rom.mrom.len(),
                    rom.vrom.len(),
                    rom.crom.len(),
                    metadata
                );
            }
            Err(error) => {
                println!("[FAIL] {}: {}", display_name(path), error);
                failed.push((path.clone(), error));
            }
        }
    }

    println!(
        "\n[SUMMARY] total={} loaded={} failed={}",
        paths.len(),
        loaded,
        failed.len()
    );
    for (path, error) in &failed {
        println!("  {}: {}", display_name(path), error);
    }

    if failed.is_empty() {
        Ok(())
    } else {
        Err(format!("{} ZIP ROMs failed to load", failed.len()))
    }
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

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=assets/ngneon_icon.ico");
    println!("cargo:rerun-if-changed=assets/ngneon_icon.rc");

    if !cfg!(target_os = "windows") {
        return;
    }

    let Some(rc_exe) = find_rc_exe() else {
        println!("cargo:warning=rc.exe not found; Windows executable icon was not embedded");
        return;
    };

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let assets_dir = manifest_dir.join("assets");
    let rc_file = assets_dir.join("ngneon_icon.rc");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let res_file = out_dir.join("ngneon_icon.res");

    let status = Command::new(rc_exe)
        .current_dir(&assets_dir)
        .arg("/nologo")
        .arg("/fo")
        .arg(&res_file)
        .arg(&rc_file)
        .status();

    match status {
        Ok(status) if status.success() => {
            println!("cargo:rustc-link-arg-bin=ngneon-emu={}", res_file.display());
        }
        Ok(status) => {
            println!(
                "cargo:warning=rc.exe failed with status {status}; executable icon not embedded"
            );
        }
        Err(error) => {
            println!("cargo:warning=Could not run rc.exe: {error}; executable icon not embedded");
        }
    }
}

fn find_rc_exe() -> Option<PathBuf> {
    if let Some(path) = find_on_path("rc.exe") {
        return Some(path);
    }

    let kits_dir = Path::new(r"C:\Program Files (x86)\Windows Kits\10\bin");
    let entries = std::fs::read_dir(kits_dir).ok()?;
    let mut versions = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    versions.sort();
    versions.reverse();

    for version in versions {
        for arch in ["x64", "x86", "arm64"] {
            let candidate = version.join(arch).join("rc.exe");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

fn find_on_path(exe_name: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    env::split_paths(&path_var)
        .map(|path| path.join(exe_name))
        .find(|candidate| candidate.is_file())
}

//! Build script for core-emulator crate.
//!
//! Compiles Musashi 68000 C sources into a static library linked to Rust.

use std::env;
use std::path::PathBuf;

fn main() {
    let project_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let musashi_dir = project_dir.join("musashi");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let ops_c = musashi_dir.join("m68kops.c");
    let ops_h = musashi_dir.join("m68kops.h");

    // ── Step 1: Generate opcode tables (if needed) ──────────────────
    if !ops_c.exists() || !ops_h.exists() {
        let make_src = musashi_dir.join("m68kmake.c");
        let make_bin = out_dir
            .join("m68kmake")
            .with_extension(if cfg!(target_os = "windows") {
                "exe"
            } else {
                ""
            });

        // Use the cc crate's compiler detection to build m68kmake
        let compiler = cc::Build::new().get_compiler();
        let mut cmd = compiler.to_command();
        cmd.arg("-o")
            .arg(&make_bin)
            .arg(&make_src)
            .arg("-lm")
            .current_dir(&musashi_dir);

        let status = cmd
            .status()
            .expect("Failed to invoke C compiler for m68kmake");

        if !status.success() {
            panic!(
                "Musashi build failed: could not compile m68kmake.c.\n\
                 Install a C compiler (MSVC Build Tools or gcc) and try again."
            );
        }

        // Run m68kmake to generate opcode files
        let run_status = std::process::Command::new(&make_bin)
            .current_dir(&musashi_dir)
            .status()
            .expect("Failed to run m68kmake");
        assert!(run_status.success(), "m68kmake exited with error");

        assert!(ops_c.exists(), "m68kops.c not generated");
        assert!(ops_h.exists(), "m68kops.h not generated");
    }

    // ── Step 2: Compile Musashi static library ──────────────────────
    let mut build = cc::Build::new();
    build
        .include(&musashi_dir)
        .warnings(false)
        .flag_if_supported("-Wno-unused-function") // quiet for generated code
        .file(musashi_dir.join("m68kcpu.c")) /* includes m68k_in.c internally */
        .file(musashi_dir.join("m68kops.c")) /* generated dispatch table */
        .file(musashi_dir.join("musashi_stubs.c")) /* FPU/MMU stubs for linker */
        .define("MUSASHI_CNF", "\"m68kconf.h\"")
        .compile("musashi");

    // ── Step 3: Compile rcheevos (RetroAchievements) static library ─
    let rcheevos_dir = project_dir.join("..").join("deps").join("rcheevos");
    let rcheevos_include = rcheevos_dir.join("include");
    let rcheevos_src = rcheevos_dir.join("src");
    let rcheevos_rcheevos = rcheevos_src.join("rcheevos");
    let rcheevos_rapi = rcheevos_src.join("rapi");
    let rcheevos_rhash = rcheevos_src.join("rhash");

    if rcheevos_dir.join("CMakeLists.txt").exists() || rcheevos_dir.join("README.md").exists() {
        let mut rc_build = cc::Build::new();
        rc_build
            .include(&rcheevos_include)
            .include(&rcheevos_src)
            .include(&rcheevos_rcheevos)
            .include(&rcheevos_rapi)
            .include(&rcheevos_rhash)
            .warnings(false)
            .flag_if_supported("-Wno-unused-function")
            .flag_if_supported("-Wno-unused-variable")
            .flag_if_supported("-Wno-sign-compare")
            .flag_if_supported("-Wno-missing-field-initializers")
            .define("RC_STATIC", None)
            .define("RC_CLIENT_SUPPORTS_HASH", None)
            .define("_CRT_SECURE_NO_WARNINGS", None)
            // Core source files
            .file(rcheevos_src.join("rc_client.c"))
            .file(rcheevos_src.join("rc_client_external.c"))
            .file(rcheevos_src.join("rc_client_raintegration.c"))
            .file(rcheevos_src.join("rc_compat.c"))
            .file(rcheevos_src.join("rc_util.c"))
            .file(rcheevos_src.join("rc_version.c"))
            // API layer
            .file(rcheevos_rapi.join("rc_api_common.c"))
            .file(rcheevos_rapi.join("rc_api_editor.c"))
            .file(rcheevos_rapi.join("rc_api_info.c"))
            .file(rcheevos_rapi.join("rc_api_runtime.c"))
            .file(rcheevos_rapi.join("rc_api_user.c"))
            // Core runtime
            .file(rcheevos_rcheevos.join("alloc.c"))
            .file(rcheevos_rcheevos.join("condition.c"))
            .file(rcheevos_rcheevos.join("condset.c"))
            .file(rcheevos_rcheevos.join("consoleinfo.c"))
            .file(rcheevos_rcheevos.join("format.c"))
            .file(rcheevos_rcheevos.join("lboard.c"))
            .file(rcheevos_rcheevos.join("memref.c"))
            .file(rcheevos_rcheevos.join("operand.c"))
            .file(rcheevos_rcheevos.join("rc_validate.c"))
            .file(rcheevos_rcheevos.join("richpresence.c"))
            .file(rcheevos_rcheevos.join("runtime.c"))
            .file(rcheevos_rcheevos.join("runtime_progress.c"))
            .file(rcheevos_rcheevos.join("trigger.c"))
            .file(rcheevos_rcheevos.join("value.c"))
            // Hash module
            .file(rcheevos_rhash.join("aes.c"))
            .file(rcheevos_rhash.join("cdreader.c"))
            .file(rcheevos_rhash.join("hash.c"))
            .file(rcheevos_rhash.join("hash_disc.c"))
            .file(rcheevos_rhash.join("hash_encrypted.c"))
            .file(rcheevos_rhash.join("hash_rom.c"))
            .file(rcheevos_rhash.join("hash_zip.c"))
            .file(rcheevos_rhash.join("md5.c"))
            .compile("rcheevos");
    }

    // ── Step 4: Compile Geolith ymfm YM2610 backend ─────────────────
    let ymfm_dir = project_dir.join("geolith_ymfm");
    let mut ymfm_build = cc::Build::new();
    ymfm_build
        .include(&ymfm_dir)
        .warnings(false)
        .flag_if_supported("-Wno-unused-function")
        .flag_if_supported("-Wno-unused-variable")
        .flag_if_supported("-Wno-sign-compare")
        .file(ymfm_dir.join("ymfm_opn.c"))
        .file(ymfm_dir.join("ymfm_ssg.c"))
        .file(ymfm_dir.join("ymfm_adpcm.c"))
        .file(ymfm_dir.join("ng_ymfm_bridge.c"))
        .compile("geolith_ymfm");

    // ── Step 5: Tell cargo when to re-run ────────────────────────────
    println!("cargo:rerun-if-changed=musashi/");
    println!("cargo:rerun-if-changed=../deps/rcheevos/src/");
    println!("cargo:rerun-if-changed=../deps/rcheevos/include/");
    println!("cargo:rerun-if-changed=geolith_ymfm/");
}

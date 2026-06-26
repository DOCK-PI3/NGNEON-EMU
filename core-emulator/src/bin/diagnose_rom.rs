use core_emulator::{rom::RomData, screenshot, video, NeoGeo};
use std::path::{Path, PathBuf};

fn main() -> Result<(), String> {
    let rom_path = std::env::args_os()
        .nth(1)
        .unwrap_or_else(|| "roms/aof.neo".into());
    let output_path = std::env::args_os()
        .nth(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("screenshots/diagnose_aof.bmp"));
    let frames: u32 = std::env::args_os()
        .nth(3)
        .and_then(|s| s.to_string_lossy().parse().ok())
        .unwrap_or(480); // 480 frames = ~8 seconds

    println!("=== NGNEON-EMU Advanced Diagnostic ===");
    println!("ROM: {:?}", rom_path);
    println!("Output: {:?}", output_path);
    println!("Frames: {}", frames);

    // ── 1. Load ROM ────────────────────────────────────────────────────
    let mut rom =
        load_rom_path(Path::new(&rom_path)).map_err(|e| format!("Failed to load ROM: {e}"))?;
    println!(
        "ROM loaded: P={} C={} S={} M={} V={}",
        rom.prom.len(),
        rom.crom.len(),
        rom.srom.len(),
        rom.mrom.len(),
        rom.vrom.len()
    );

    // ── 2. Create emulator and load ROM ────────────────────────────────
    let mut neogeo = NeoGeo::new();
    neogeo.load_rom_and_connect(&mut rom);

    // Show the cart USER entry bytes exactly as loaded.
    {
        let mem = neogeo.memory.borrow();
        if mem.prom.len() >= 0x128 {
            let user_entry = u32::from_be_bytes([
                mem.prom[0x122],
                mem.prom[0x123],
                mem.prom[0x124],
                mem.prom[0x125],
            ]);
            println!("PROM[0x122] USER entry: 0x{:08X}", user_entry);
        }
    }

    // ── 3. Load BIOS (UniBIOS) ─────────────────────────────────────────
    let dirs = ["bios", "roms"];
    let mut bios_label = String::from("Diagnóstica interna");
    if let Some(bios) = core_emulator::bios::load_bios_from_multi(&dirs).unwrap_or(None) {
        println!("BIOS loaded: {} ({} bytes)", bios.label, bios.data.len());
        bios_label = bios.label.clone();
        neogeo.set_bios(bios.data);
    } else {
        println!("WARNING: No external BIOS found, using internal diagnostic BIOS");
    }

    if let Some(zoom) =
        core_emulator::bios::load_zoom_rom_for_bios_from_multi(&dirs, &bios_label).unwrap_or(None)
    {
        println!(
            "Zoom ROM loaded: {} ({} bytes)",
            zoom.label,
            zoom.data.len()
        );
        neogeo.set_zoom_rom(zoom.data);
    }

    if let Some(sfix) =
        core_emulator::bios::load_sfix_rom_for_bios_from_multi(&dirs, &bios_label).unwrap_or(None)
    {
        println!("SFIX loaded: {} ({} bytes)", sfix.label, sfix.data.len());
        neogeo.set_sfix_rom(sfix.data);
    }

    if let Some(sm1) =
        core_emulator::bios::load_sm1_rom_for_bios_from_multi(&dirs, &bios_label).unwrap_or(None)
    {
        println!("SM1 loaded: {} ({} bytes)", sm1.label, sm1.data.len());
        neogeo.set_sm1_rom(sm1.data);
    }

    // Reset CPU after BIOS change
    neogeo.reset();

    // ── 4. Detailed frame-by-frame analysis ────────────────────────────
    let mut prev_pc = 0u32;
    let mut stuck_count = 0;
    let mut max_non_black = 0usize;

    for frame in 0..frames {
        match neogeo.step() {
            Ok(_) => {}
            Err(e) => {
                println!("[CPU ERROR at frame {}] {}", frame, e);
                break;
            }
        }

        // Detailed analysis every frame for first 20 frames, then every 60
        let detailed =
            frame < 20 || (380..=430).contains(&frame) || frame % 60 == 0 || frame == frames - 1;

        if detailed {
            let mem = neogeo.memory.borrow();
            let fb = &neogeo.video.framebuffer;
            let snap = neogeo.status();

            // Count REAL non-black pixels (!= 0xFF000000)
            let non_black = fb.iter().filter(|&&p| p != 0xFF000000).count();
            if non_black > max_non_black {
                max_non_black = non_black;
            }

            // System latch state
            let sys = mem.system_control_snapshot();

            // Palette RAM: count non-zero words
            let pal_words = mem
                .palette_ram
                .chunks(2)
                .filter(|c| c.len() == 2 && (c[0] != 0 || c[1] != 0))
                .count();

            // Fix map: count non-zero entries
            let fix_entries: usize = (0..40 * 32)
                .filter(|&i| {
                    let addr = (0x7000usize + i) * 2;
                    addr + 1 < mem.vram.len() && (mem.vram[addr] != 0 || mem.vram[addr + 1] != 0)
                })
                .count();

            // SCB3 entries (sprites with height)
            let scb3_entries: usize = (1..382)
                .filter(|&s| {
                    let addr = (0x8200usize + s * 2) * 2;
                    addr + 1 < mem.vram.len() && (mem.vram[addr] != 0 || mem.vram[addr + 1] != 0)
                })
                .count();

            // Video debug stats
            let vstats = video::debug_vram_stats(&mem);

            println!(
                "Frame {:3}: PC=0x{:06X} | nBlk={:5} | palW={:3} | fixM={:3} | scb3={:3} | vSprH={:3} | visible={:3} maxLine={:3} | pBank={} | disp={} cartVec={} cartFix={} | vRAMmod=0x{:04X}",
                frame, snap.pc, non_black, pal_words, fix_entries, scb3_entries,
                vstats.sprites_with_height,
                vstats.generated_visible_sprites,
                vstats.generated_max_sprites_per_scanline,
                sys.palette_bank,
                sys.display_enabled as u8,
                sys.use_cart_vectors as u8,
                sys.use_cart_fix as u8,
                mem.vram_mod,
            );
            if (360..=390).contains(&frame) {
                print_cpu_regs(&neogeo);
            }
            drop(mem);
        }

        // Detect stuck PC (CPU not progressing)
        let current_pc = neogeo.status().pc;
        if current_pc == prev_pc {
            stuck_count += 1;
        } else {
            stuck_count = 0;
        }
        if stuck_count >= 10 && detailed {
            println!(
                "  ⚠️  PC stuck at 0x{:06X} for {} frames!",
                current_pc, stuck_count
            );
        }
        prev_pc = current_pc;
    }

    if let Ok(steps) = std::env::var("NGNEON_TRACE_STEPS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .ok_or(())
    {
        println!();
        println!("=== CPU Instruction Trace ({steps} steps) ===");
        for step in 0..steps {
            let pc = neogeo.status().pc;
            if pc < 0x00_8000 || (0x00_8000..0x05_0000).contains(&pc) || pc >= 0xC0_0000 {
                println!("Step {step:06}: PC=0x{pc:06X}");
                print_cpu_regs(&neogeo);
            }
            match neogeo.cpu.step() {
                Ok(_) => {}
                Err(e) => {
                    println!("[CPU TRACE ERROR at step {step}] {e}");
                    break;
                }
            }
        }
    }

    // ── 5. Final detailed state dump ───────────────────────────────────
    {
        let mem = neogeo.memory.borrow();
        let snap = neogeo.status();
        let vstats = video::debug_vram_stats(&mem);

        println!();
        println!("=== Final Detailed State ===");
        println!("PC=0x{:06X} SR=0x{:04X}", snap.pc, snap.sr);
        println!(
            "display_enabled={} use_cart_vectors={} use_cart_fix={} use_cart_audio={}",
            mem.display_enabled, mem.use_cart_vectors, mem.use_cart_fix, mem.use_cart_audio
        );
        println!(
            "palette_bank={} save_ram_unlocked={}",
            mem.palette_bank, mem.save_ram_unlocked
        );
        println!(
            "vram_addr=0x{:04X} vram_mod=0x{:04X} lspc_mode=0x{:04X}",
            mem.vram_addr.get(),
            mem.vram_mod,
            mem.lspc_mode
        );
        println!();
        println!("Video debug stats:");
        println!(
            "  sprites_with_height:        {}",
            vstats.sprites_with_height
        );
        println!(
            "  visible_sprites:            {}",
            vstats.generated_visible_sprites
        );
        println!(
            "  sprite_scanlines:           {}",
            vstats.generated_sprite_scanlines
        );
        println!(
            "  max_sprites_per_scanline:   {}",
            vstats.generated_max_sprites_per_scanline
        );
        println!(
            "  overflow_scanlines:         {}",
            vstats.generated_overflow_scanlines
        );
        println!("  visible_fix_tiles:          {}", vstats.visible_fix_tiles);
        println!(
            "  drawable_fix_tiles:         {}",
            vstats.drawable_fix_tiles
        );
        println!("  fix_opaque_pixels:          {}", vstats.fix_opaque_pixels);
        println!(
            "  initialized_palette_banks:  {}",
            vstats.initialized_palette_banks
        );

        // Palette RAM first 64 bytes hex dump
        println!();
        println!("Palette RAM (first 64 bytes, bank 0):");
        for row in 0..4 {
            let start = row * 16;
            let hex: String = mem.palette_ram[start..start + 16]
                .iter()
                .map(|b| format!("{:02X} ", b))
                .collect();
            println!("  {:04X}: {}", start, hex);
        }

        // Fix map first 16 entries hex dump
        println!();
        println!("Fix map (first 16 entries, row 2):");
        for col in 0..16 {
            let addr = (0x7000usize + col * 32 + 2) * 2;
            if addr + 1 < mem.vram.len() {
                let entry = u16::from_be_bytes([mem.vram[addr], mem.vram[addr + 1]]);
                println!("  col {}: entry=0x{:04X}", col, entry);
            }
        }

        let mut fix_freq = std::collections::BTreeMap::<u16, usize>::new();
        let mut row_drawable = [0usize; 32];
        for (row, drawable_count) in row_drawable.iter_mut().enumerate() {
            for col in 0..40 {
                let addr = (0x7000usize + col * 32 + row) * 2;
                if addr + 1 < mem.vram.len() {
                    let entry = u16::from_be_bytes([mem.vram[addr], mem.vram[addr + 1]]);
                    *fix_freq.entry(entry).or_default() += 1;
                    if entry != 0 && entry != 0x00ff {
                        *drawable_count += 1;
                    }
                }
            }
        }
        let mut fix_freq: Vec<(u16, usize)> = fix_freq.into_iter().collect();
        fix_freq.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
        println!();
        println!("Fix map most common entries:");
        for (entry, count) in fix_freq.into_iter().take(12) {
            println!("  entry=0x{entry:04X} count={count}");
        }
        println!();
        println!("Fix map drawable entries per row (excluding 0 and 0x00FF):");
        for (row, count) in row_drawable.iter().enumerate() {
            if *count != 0 {
                println!("  row {row:02}: {count}");
            }
        }

        let active_palette_base =
            mem.palette_bank as usize * core_emulator::memory::PALETTE_RAM_BANK_SIZE;
        let palette_f = active_palette_base + 0x0f * 32;
        if palette_f + 31 < mem.palette_ram.len() {
            println!();
            println!(
                "Active palette F raw bytes (palette_bank={}):",
                mem.palette_bank
            );
            let hex: String = mem.palette_ram[palette_f..palette_f + 32]
                .iter()
                .map(|b| format!("{:02X} ", b))
                .collect();
            println!("  {}", hex);
            for color in 0..4 {
                let base = palette_f + color * 2;
                let word = u16::from_be_bytes([mem.palette_ram[base], mem.palette_ram[base + 1]]);
                println!("  color {color}: 0x{word:04X}");
            }
        }

        // SCB3 first 10 sprites
        println!();
        println!("SCB3 (first 10 sprites):");
        for s in 1..11 {
            let addr = (0x8200usize + s * 2) * 2;
            if addr + 1 < mem.vram.len() {
                let entry = u16::from_be_bytes([mem.vram[addr], mem.vram[addr + 1]]);
                println!("  sprite {}: SCB3=0x{:04X}", s, entry);
            }
        }

        // SROM/SFIX sizes
        println!();
        println!(
            "SROM: {} bytes  SFIX: {} bytes  use_cart_fix: {}",
            mem.srom.len(),
            mem.sfix.len(),
            mem.use_cart_fix
        );
        drop(mem);
    }

    // ── 6. Capture framebuffer ─────────────────────────────────────────
    screenshot::save_framebuffer_bmp(
        &output_path,
        &neogeo.video.framebuffer,
        neogeo.video.width,
        neogeo.video.height,
    )?;

    // ── 7. Analysis summary ────────────────────────────────────────────
    let fb = &neogeo.video.framebuffer;
    let real_non_black = fb.iter().filter(|&&p| p != 0xFF000000).count();
    let total = fb.len();
    println!();
    println!("=== Final Analysis ===");
    println!(
        "Real non-black pixels: {} / {} ({:.1}%)",
        real_non_black,
        total,
        real_non_black as f64 / total as f64 * 100.0
    );
    println!("Max non-black any frame: {}", max_non_black);
    println!("Framebuffer saved to: {:?}", output_path);

    if real_non_black > 0 {
        println!("RESULT: GAME IS RENDERING!");
        // Show first non-black pixel location
        let idx = fb.iter().position(|&p| p != 0xFF000000).unwrap();
        let x = idx % video::SCREEN_WIDTH;
        let y = idx / video::SCREEN_WIDTH;
        println!(
            "  First non-black pixel at ({}, {}): 0x{:08X}",
            x, y, fb[idx]
        );
    } else {
        println!("RESULT: All black - no game content rendered.");
    }

    Ok(())
}

fn print_cpu_regs(neogeo: &NeoGeo) {
    use core_emulator::musashi_ffi::{
        M68K_REG_A0, M68K_REG_A1, M68K_REG_A2, M68K_REG_A3, M68K_REG_A4, M68K_REG_A5, M68K_REG_A6,
        M68K_REG_A7, M68K_REG_D0, M68K_REG_D1, M68K_REG_D2, M68K_REG_D3, M68K_REG_D4, M68K_REG_D5,
        M68K_REG_D6, M68K_REG_D7,
    };

    println!(
        "  D: {:08X} {:08X} {:08X} {:08X}  {:08X} {:08X} {:08X} {:08X}",
        neogeo.cpu.get_reg(M68K_REG_D0),
        neogeo.cpu.get_reg(M68K_REG_D1),
        neogeo.cpu.get_reg(M68K_REG_D2),
        neogeo.cpu.get_reg(M68K_REG_D3),
        neogeo.cpu.get_reg(M68K_REG_D4),
        neogeo.cpu.get_reg(M68K_REG_D5),
        neogeo.cpu.get_reg(M68K_REG_D6),
        neogeo.cpu.get_reg(M68K_REG_D7),
    );
    println!(
        "  A: {:08X} {:08X} {:08X} {:08X}  {:08X} {:08X} {:08X} {:08X}",
        neogeo.cpu.get_reg(M68K_REG_A0),
        neogeo.cpu.get_reg(M68K_REG_A1),
        neogeo.cpu.get_reg(M68K_REG_A2),
        neogeo.cpu.get_reg(M68K_REG_A3),
        neogeo.cpu.get_reg(M68K_REG_A4),
        neogeo.cpu.get_reg(M68K_REG_A5),
        neogeo.cpu.get_reg(M68K_REG_A6),
        neogeo.cpu.get_reg(M68K_REG_A7),
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RomFileKind {
    Neo,
    Zip,
}

fn rom_file_kind(path: &Path) -> Result<RomFileKind, String> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "neo" => Ok(RomFileKind::Neo),
        "zip" => Ok(RomFileKind::Zip),
        _ => Err(format!("Extensión no soportada para {:?}", path)),
    }
}

fn load_rom_path(path: &Path) -> Result<RomData, String> {
    match rom_file_kind(path)? {
        RomFileKind::Neo => RomData::from_neo(path),
        RomFileKind::Zip => RomData::from_zip(path),
    }
}

#[cfg(test)]
mod tests {
    use super::{rom_file_kind, RomFileKind};
    use std::path::Path;

    #[test]
    fn rom_file_kind_accepts_neo_and_zip_case_insensitive() {
        assert_eq!(rom_file_kind(Path::new("aof.neo")), Ok(RomFileKind::Neo));
        assert_eq!(
            rom_file_kind(Path::new("kof2000.ZIP")),
            Ok(RomFileKind::Zip)
        );
    }

    #[test]
    fn rom_file_kind_rejects_unknown_extensions() {
        assert!(rom_file_kind(Path::new("readme.txt")).is_err());
        assert!(rom_file_kind(Path::new("rom_without_extension")).is_err());
    }
}

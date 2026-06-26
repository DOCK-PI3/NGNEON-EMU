use core_emulator::{
    bios,
    memory::{BusAccess, BusAccessKind, Z80IoAccess},
    rom::RomData,
    screenshot, video, EmuAction, NeoGeo,
};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const DEFAULT_FRAMES: usize = 3;

fn main() -> Result<(), String> {
    let rom_path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or_else(|| {
            "Uso: probe_rom <rom.neo|rom.zip> [captura.bmp] [frames] [inputs] [input_frames]"
                .to_string()
        })?;
    let output_path = std::env::args_os()
        .nth(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| default_output_path(&rom_path));
    let frames = std::env::args()
        .nth(3)
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|error| format!("Frames inválidos: {error}"))?
        .unwrap_or(DEFAULT_FRAMES);
    let inputs = std::env::args()
        .nth(4)
        .map(|value| parse_inputs(&value))
        .transpose()?
        .unwrap_or_default();
    let input_frames = std::env::args()
        .nth(5)
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|error| format!("input_frames inválido: {error}"))?
        .unwrap_or(30);
    let input_schedule = std::env::args()
        .nth(6)
        .map(|value| parse_input_schedule(&value))
        .transpose()?
        .unwrap_or_default();
    let load_state_path = std::env::args_os().nth(7).map(PathBuf::from);
    let trace_frames = std::env::var("NGNEON_PROBE_TRACE_FRAMES")
        .ok()
        .map(|value| value != "0")
        .unwrap_or(false);
    let frame_dumps = std::env::var("NGNEON_PROBE_FRAME_DUMPS")
        .ok()
        .map(|value| parse_frame_dumps(&value))
        .transpose()?
        .unwrap_or_default();

    let mut rom = load_rom_path(&rom_path)?;
    print_rom_info(&rom_path, &rom);

    let mut neogeo = NeoGeo::new();
    let mut bios_label = String::from("Diagnóstica interna");
    if let Some(bios) = bios::load_bios_from_dir("bios")? {
        println!("[INFO] BIOS activa: {}", bios.label);
        bios_label = bios.label.clone();
        neogeo.set_bios(bios.data);
    } else {
        println!("[INFO] BIOS activa: diagnóstica interna");
    }
    let dirs = ["bios"];
    if let Some(zoom_rom) = bios::load_zoom_rom_for_bios_from_multi(&dirs, &bios_label)? {
        println!("[INFO] Tabla L0 activa: {}", zoom_rom.label);
        neogeo.set_zoom_rom(zoom_rom.data);
    } else {
        println!("[INFO] Tabla L0 activa: aproximación interna");
    }
    if let Some(sfix_rom) = bios::load_sfix_rom_for_bios_from_multi(&dirs, &bios_label)? {
        println!("[INFO] SFIX activa: {}", sfix_rom.label);
        neogeo.set_sfix_rom(sfix_rom.data);
    } else {
        println!("[INFO] SFIX activa: no encontrada, usando S-ROM de cartucho");
    }
    if let Some(sm1_rom) = bios::load_sm1_rom_for_bios_from_multi(&dirs, &bios_label)? {
        println!("[INFO] SM1 activa: {}", sm1_rom.label);
        neogeo.set_sm1_rom(sm1_rom.data);
    } else {
        println!("[INFO] SM1 activa: no encontrada, usando M-ROM de cartucho");
    }
    neogeo.load_rom_and_connect(&mut rom);
    if let Some(path) = load_state_path {
        let data = std::fs::read(&path)
            .map_err(|error| format!("No se pudo leer savestate {:?}: {error}", path))?;
        neogeo
            .load_state(&data)
            .map_err(|error| format!("No se pudo cargar savestate {:?}: {error}", path))?;
        println!("[INFO] Savestate cargado: {:?}", path);
    }
    let mut audio_frames_nonzero = 0usize;
    let mut audio_nonzero_total = 0usize;
    let mut audio_peak_run = 0u16;
    for frame in 0..frames {
        apply_probe_inputs(&mut neogeo, &inputs, frame, input_frames, &input_schedule);
        if let Err(error) = neogeo.step() {
            println!("[CPU] frame={frame}: {error}");
            break;
        }
        if frame_dumps.contains(&frame) {
            let frame_path = diagnostic_output_path(&output_path, &format!("frame_{frame:04}"));
            screenshot::save_framebuffer_bmp(
                &frame_path,
                &neogeo.video.framebuffer,
                neogeo.video.width,
                neogeo.video.height,
            )?;
            let visual = visual_stats(&neogeo.video.framebuffer);
            println!(
                "[FRAME_DUMP] frame={} nonblack={} nonblack_pct_x100={} path={:?}",
                frame, visual.nonblack, visual.nonblack_pct_x100, frame_path
            );
        }
        if trace_frames {
            let status = neogeo.status();
            println!(
                "[TRACE_FRAME] frame={} pc=0x{:06X} sr=0x{:04X} pbank=0x{:X} cart_vectors={}",
                frame,
                status.pc,
                status.sr,
                status.prom_bank_offset,
                neogeo.memory.borrow().use_cart_vectors
            );
        }
        let audio = neogeo.audio_mixer.samples();
        let nonzero = audio.iter().filter(|&&sample| sample != 0).count();
        if nonzero > 0 {
            audio_frames_nonzero += 1;
            audio_nonzero_total += nonzero;
        }
        audio_peak_run = audio_peak_run.max(
            audio
                .iter()
                .map(|sample| sample.unsigned_abs())
                .max()
                .unwrap_or(0),
        );
    }

    let status = neogeo.status();
    println!(
        "[STATUS] mode={:?} pc=0x{:06X} sr=0x{:04X} cycles={}/{} pbank=0x{:X}",
        status.mode,
        status.pc,
        status.sr,
        status.last_cpu_cycles,
        status.target_cpu_cycles,
        status.prom_bank_offset
    );
    if let Some(error) = status.last_error {
        println!("[STATUS] last_error={error}");
    }
    let audio = neogeo.audio_mixer.samples();
    let nonzero_audio = audio.iter().filter(|&&sample| sample != 0).count();
    let peak_audio = audio
        .iter()
        .map(|sample| sample.unsigned_abs())
        .max()
        .unwrap_or(0);
    println!(
        "[AUDIO] samples={} nonzero={} peak={}",
        audio.len(),
        nonzero_audio,
        peak_audio
    );
    println!(
        "[AUDIO_RUN] frames_nonzero={} nonzero_total={} peak={}",
        audio_frames_nonzero, audio_nonzero_total, audio_peak_run
    );
    let mem = neogeo.memory.borrow();
    let control = mem.system_control_snapshot();
    println!(
        "[CTRL] display={} cart_vectors={} cart_audio={} cart_fix={} sram_unlocked={} pbank={} pvc_bank=0x{:06X} memcard_unlocked={} memcard_regsel={}",
        control.display_enabled,
        control.use_cart_vectors,
        control.use_cart_audio,
        control.use_cart_fix,
        control.save_ram_unlocked,
        control.palette_bank,
        mem.pvc_bank_addr,
        control.memcard_unlocked,
        control.memcard_register_select
    );
    drop(mem);
    let sound = neogeo.memory.borrow().sound_port_snapshot();
    println!(
        "[SOUND] command=0x{:02X} reply=0x{:02X}",
        sound.command, sound.reply
    );
    let audio_debug = neogeo.ym2610.borrow().audio_debug();
    println!(
        "[YM2610] backend={} adpcm_a_playing={:?} adpcm_a_output={:?} adpcm_a_addr={:?} adpcm_b_playing={} adpcm_b_output={} prev={} addr=0x{:06X} start=0x{:04X} stop=0x{:04X} delta=0x{:04X} vol=0x{:02X}",
        if audio_debug.geolith_backend {
            "geolith"
        } else {
            "rust"
        },
        audio_debug.adpcm_a_playing,
        audio_debug.adpcm_a_output,
        audio_debug.adpcm_a_addr,
        audio_debug.adpcm_b_playing,
        audio_debug.adpcm_b_output,
        audio_debug.adpcm_b_prev_output,
        audio_debug.adpcm_b_addr,
        audio_debug.adpcm_b_start,
        audio_debug.adpcm_b_stop,
        audio_debug.adpcm_b_delta_n,
        audio_debug.adpcm_b_volume
    );
    println!(
        "[YM_TIMER] mode=0x{:02X} remaining_a={} remaining_b={} busy={} irq={}",
        audio_debug.timer_mode,
        audio_debug.timer_remaining[0],
        audio_debug.timer_remaining[1],
        audio_debug.busy_remaining,
        audio_debug.irq_asserted
    );
    let ym_status = neogeo.ym2610.borrow().read_status(0);
    let ym_irq = neogeo.ym2610.borrow().timer_irq_pending();
    let (iff1, iff2, interrupt_mode) = neogeo.z80_cpu.interrupt_state();
    let memory = neogeo.memory.borrow();
    println!(
        "[Z80] pc=0x{:04X} halt={} tstates={} iff1={} iff2={} im={} ym_status=0x{:02X} ym_irq={} nmi_enabled={} nmi_pending={} mrom={} bytes",
        neogeo.z80_cpu.pc(),
        neogeo.z80_cpu.is_halt(),
        neogeo.last_z80_tstates,
        iff1,
        iff2,
        interrupt_mode,
        ym_status,
        ym_irq,
        memory.z80_nmi_enabled.get(),
        memory.z80_nmi_pending.get(),
        memory.mrom.len()
    );
    drop(memory);
    let vram = video::debug_vram_stats(&neogeo.memory.borrow());
    println!(
        "[VRAM] sprites_h={} visible_sprites={} gen_lines={} gen_max={} gen_overflow={} h_shrink={} v_shrink={} fix_visible={} fix_drawable={} fix_unique={} fix_pixels={} palette_banks={}",
        vram.sprites_with_height,
        vram.generated_visible_sprites,
        vram.generated_sprite_scanlines,
        vram.generated_max_sprites_per_scanline,
        vram.generated_overflow_scanlines,
        vram.sprites_with_horizontal_shrink,
        vram.sprites_with_vertical_shrink,
        vram.visible_fix_tiles,
        vram.drawable_fix_tiles,
        vram.unique_fix_tiles,
        vram.fix_opaque_pixels,
        vram.initialized_palette_banks
    );
    if std::env::var("NGNEON_PROBE_SPRITES").is_ok_and(|value| value != "0") {
        print_sprite_summary(&neogeo.memory.borrow());
    }
    if std::env::var("NGNEON_PROBE_DUMP_VRAM").is_ok_and(|value| value != "0") {
        let vram_path = output_path.with_file_name(format!(
            "{}_vram.bin",
            output_path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("probe")
        ));
        std::fs::write(&vram_path, &neogeo.memory.borrow().vram)
            .map_err(|error| format!("No se pudo guardar VRAM: {error}"))?;
        println!("[INFO] VRAM guardada en {:?}", vram_path);
        let palette_path = output_path.with_file_name(format!(
            "{}_palette.bin",
            output_path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("probe")
        ));
        std::fs::write(&palette_path, &neogeo.memory.borrow().palette_ram)
            .map_err(|error| format!("No se pudo guardar la paleta: {error}"))?;
        println!("[INFO] Paleta guardada en {:?}", palette_path);
        let ram_path = output_path.with_file_name(format!(
            "{}_ram.bin",
            output_path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("probe")
        ));
        std::fs::write(&ram_path, &neogeo.memory.borrow().ram)
            .map_err(|error| format!("No se pudo guardar RAM: {error}"))?;
        println!("[INFO] RAM guardada en {:?}", ram_path);
    }
    let visual = visual_stats(&neogeo.video.framebuffer);
    println!(
        "[VIDEO] pixels={} nonblack={} nonblack_pct_x100={} unique_rgb={}",
        visual.pixels, visual.nonblack, visual.nonblack_pct_x100, visual.unique_rgb
    );
    if std::env::var("NGNEON_PROBE_RERENDER").is_ok_and(|value| value != "0") {
        let mut rerender = video::Video::new();
        rerender.render_frame(&neogeo.memory.borrow());
        let rerender_visual = visual_stats(&rerender.framebuffer);
        println!(
            "[VIDEO_RERENDER] pixels={} nonblack={} nonblack_pct_x100={} unique_rgb={}",
            rerender_visual.pixels,
            rerender_visual.nonblack,
            rerender_visual.nonblack_pct_x100,
            rerender_visual.unique_rgb
        );
        let rerender_path = diagnostic_output_path(&output_path, "rerender");
        screenshot::save_framebuffer_bmp(
            &rerender_path,
            &rerender.framebuffer,
            rerender.width,
            rerender.height,
        )?;
        println!("[INFO] Re-render guardado en {:?}", rerender_path);
    }
    if std::env::var("NGNEON_PROBE_NO_FIX").is_ok_and(|value| value != "0") {
        let mut saved_srom = Vec::new();
        let mut saved_sfix = Vec::new();
        let mut saved_dynamic_fix = Vec::new();
        {
            let mut memory = neogeo.memory.borrow_mut();
            std::mem::swap(&mut saved_srom, &mut memory.srom);
            std::mem::swap(&mut saved_sfix, &mut memory.sfix);
            std::mem::swap(&mut saved_dynamic_fix, &mut memory.dynamic_fix_rom);
        }
        let mut no_fix = video::Video::new();
        no_fix.render_frame(&neogeo.memory.borrow());
        let no_fix_visual = visual_stats(&no_fix.framebuffer);
        println!(
            "[VIDEO_NO_FIX] pixels={} nonblack={} nonblack_pct_x100={} unique_rgb={}",
            no_fix_visual.pixels,
            no_fix_visual.nonblack,
            no_fix_visual.nonblack_pct_x100,
            no_fix_visual.unique_rgb
        );
        let no_fix_path = diagnostic_output_path(&output_path, "nofix");
        screenshot::save_framebuffer_bmp(
            &no_fix_path,
            &no_fix.framebuffer,
            no_fix.width,
            no_fix.height,
        )?;
        println!("[INFO] Re-render sin FIX guardado en {:?}", no_fix_path);
        {
            let mut memory = neogeo.memory.borrow_mut();
            std::mem::swap(&mut saved_srom, &mut memory.srom);
            std::mem::swap(&mut saved_sfix, &mut memory.sfix);
            std::mem::swap(&mut saved_dynamic_fix, &mut memory.dynamic_fix_rom);
        }
    }
    print_bus_trace(&neogeo.memory.borrow().take_bus_trace());
    print_z80_io_trace(&neogeo.memory.borrow().take_z80_io_trace());

    screenshot::save_framebuffer_bmp(
        &output_path,
        &neogeo.video.framebuffer,
        neogeo.video.width,
        neogeo.video.height,
    )?;
    println!("[INFO] Captura guardada en {:?}", output_path);

    Ok(())
}

fn print_sprite_summary(mem: &core_emulator::memory::Memory) {
    fn vram_word(mem: &core_emulator::memory::Memory, word_addr: usize) -> u16 {
        let byte_addr = word_addr * 2;
        if byte_addr + 1 >= mem.vram.len() {
            return 0;
        }
        u16::from_be_bytes([mem.vram[byte_addr], mem.vram[byte_addr + 1]])
    }

    println!("[SPRITES] first visible/control entries:");
    let mut printed = 0usize;
    for sprite in 1..382usize {
        let scb3 = vram_word(mem, 0x8200 + sprite);
        let height = scb3 & 0x003F;
        let sticky = scb3 & 0x0040 != 0;
        if height == 0 && !sticky {
            continue;
        }
        let scb2 = vram_word(mem, 0x8000 + sprite);
        let scb4 = vram_word(mem, 0x8400 + sprite);
        let tile_lsb = vram_word(mem, sprite * 64);
        let attr = vram_word(mem, sprite * 64 + 1);
        let xpos = (scb4 >> 7) & 0x01FF;
        let ypos = (scb3 >> 7) & 0x01FF;
        println!(
            "  #{sprite:03} scb2=0x{scb2:04X} scb3=0x{scb3:04X} scb4=0x{scb4:04X} x={xpos:03} y={ypos:03} h={height:02} sticky={} tile=0x{tile_lsb:04X} attr=0x{attr:04X}",
            sticky as u8
        );
        printed += 1;
        if printed >= 96 {
            break;
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct VisualStats {
    pixels: usize,
    nonblack: usize,
    nonblack_pct_x100: usize,
    unique_rgb: usize,
}

fn visual_stats(framebuffer: &[u32]) -> VisualStats {
    let mut colors = BTreeSet::new();
    let mut nonblack = 0usize;

    for &pixel in framebuffer {
        let rgb = pixel & 0x00ff_ffff;
        if rgb != 0 {
            nonblack += 1;
            colors.insert(rgb);
        }
    }

    let pixels = framebuffer.len();
    let nonblack_pct_x100 = nonblack
        .checked_mul(10_000)
        .and_then(|value| value.checked_div(pixels))
        .unwrap_or(0);

    VisualStats {
        pixels,
        nonblack,
        nonblack_pct_x100,
        unique_rgb: colors.len(),
    }
}

fn apply_probe_inputs(
    neogeo: &mut NeoGeo,
    inputs: &[EmuAction],
    frame: usize,
    input_frames: usize,
    schedule: &[ScheduledInput],
) {
    for action in inputs {
        neogeo.set_input(*action, false);
    }
    for event in schedule {
        for action in &event.actions {
            neogeo.set_input(*action, false);
        }
    }

    if frame < input_frames {
        for action in inputs {
            neogeo.set_input(*action, true);
        }
    }
    for event in schedule {
        if frame >= event.start && frame < event.end {
            for action in &event.actions {
                neogeo.set_input(*action, true);
            }
        }
    }
}

#[derive(Debug, Clone)]
struct ScheduledInput {
    start: usize,
    end: usize,
    actions: Vec<EmuAction>,
}

fn parse_input_schedule(value: &str) -> Result<Vec<ScheduledInput>, String> {
    value
        .split(';')
        .filter(|part| !part.trim().is_empty())
        .map(|part| {
            let mut fields = part.split(':');
            let start = fields
                .next()
                .ok_or_else(|| format!("Entrada programada inválida: {part}"))?
                .parse::<usize>()
                .map_err(|error| format!("Frame inicial inválido en {part}: {error}"))?;
            let end = fields
                .next()
                .ok_or_else(|| format!("Entrada programada sin frame final: {part}"))?
                .parse::<usize>()
                .map_err(|error| format!("Frame final inválido en {part}: {error}"))?;
            let actions = fields
                .next()
                .ok_or_else(|| format!("Entrada programada sin acciones: {part}"))
                .and_then(parse_inputs)?;
            if fields.next().is_some() || end < start {
                return Err(format!("Entrada programada inválida: {part}"));
            }
            Ok(ScheduledInput {
                start,
                end,
                actions,
            })
        })
        .collect()
}

fn parse_inputs(value: &str) -> Result<Vec<EmuAction>, String> {
    value
        .split(',')
        .filter(|part| !part.trim().is_empty())
        .map(|part| match part.trim().to_ascii_lowercase().as_str() {
            "up" => Ok(EmuAction::Up),
            "down" => Ok(EmuAction::Down),
            "left" => Ok(EmuAction::Left),
            "right" => Ok(EmuAction::Right),
            "a" => Ok(EmuAction::A),
            "b" => Ok(EmuAction::B),
            "c" => Ok(EmuAction::C),
            "d" => Ok(EmuAction::D),
            "start" => Ok(EmuAction::Start),
            "coin" => Ok(EmuAction::Coin),
            other => Err(format!("Input no soportado para probe_rom: {other}")),
        })
        .collect()
}

fn parse_frame_dumps(value: &str) -> Result<BTreeSet<usize>, String> {
    value
        .split(',')
        .filter(|part| !part.trim().is_empty())
        .map(|part| {
            part.trim()
                .parse::<usize>()
                .map_err(|error| format!("Frame dump inválido '{part}': {error}"))
        })
        .collect()
}

fn print_bus_trace(trace: &[BusAccess]) {
    if trace.is_empty() {
        println!("[BUS] Sin accesos recientes a hardware/unmapped.");
        return;
    }

    println!("[BUS] Últimos accesos hardware/unmapped: {}", trace.len());
    for access in trace.iter().rev().take(24).rev() {
        let kind = match access.kind {
            BusAccessKind::Read => "R",
            BusAccessKind::Write => "W",
        };
        println!("  {kind} 0x{:06X} = 0x{:02X}", access.address, access.value);
    }
}

fn load_rom_path(path: &Path) -> Result<RomData, String> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "neo" => RomData::from_neo(path),
        "zip" => RomData::from_zip(path),
        _ => Err(format!("Extensión no soportada para {:?}", path)),
    }
}

fn print_z80_io_trace(trace: &[Z80IoAccess]) {
    if trace.is_empty() {
        println!("[Z80IO] Sin accesos recientes.");
        return;
    }

    println!("[Z80IO] Últimos accesos Z80 I/O: {}", trace.len());
    for access in trace.iter().rev().take(32).rev() {
        let kind = match access.kind {
            BusAccessKind::Read => "R",
            BusAccessKind::Write => "W",
        };
        println!(
            "  {kind} port=0x{:04X} value=0x{:02X} {}",
            access.port,
            access.value,
            z80_port_label(access.port)
        );
    }
}

fn z80_port_label(port: u16) -> &'static str {
    match port & 0x00FF {
        0x00 => "SOUND_LATCH",
        0x04 => "YM_A0",
        0x05 => "YM_D0",
        0x06 => "YM_A1",
        0x07 => "YM_D1",
        0x08 => "BANK3/NMI_ON",
        0x09 => "BANK2/NMI_ON",
        0x0A => "BANK1/NMI_ON",
        0x0B => "BANK0/NMI_ON",
        0x0C => "SOUND_REPLY",
        0x18 => "NMI_OFF",
        0xC0 => "SOUND_LATCH_CLEAR",
        _ => "UNKNOWN",
    }
}

fn print_rom_info(path: &Path, rom: &RomData) {
    let diagnostics = rom.diagnostics();
    println!("[INFO] ROM: {:?}", path);
    println!("[INFO] {}", rom.bank_summary());
    println!(
        "[INFO] source={:?} recognized_files={}",
        diagnostics.source, diagnostics.recognized_files
    );
    print_program_header(rom);

    if !rom.recognized_files.is_empty() {
        println!("[INFO] Archivos reconocidos:");
        for name in &rom.recognized_files {
            println!("  - {name}");
        }
    }

    if let Some(metadata) = &rom.metadata {
        println!(
            "[INFO] Metadata .neo: '{}' ({}) fabricante='{}' NGH=0x{:X}",
            metadata.name, metadata.year, metadata.manufacturer, metadata.ngh
        );
        println!(
            "[INFO] .neo runtime: board={:?} fix={:?} flags={:?}",
            metadata.board_type, metadata.fix_banksw, metadata.game_flags
        );
    }

    for warning in diagnostics.warnings {
        println!("[WARN] {warning}");
    }
}

fn print_program_header(rom: &RomData) {
    if rom.prom.len() < 0x110 {
        return;
    }

    let initial_sp = read_u32_be(&rom.prom, 0);
    let reset = read_u32_be(&rom.prom, 4);
    let header = std::str::from_utf8(&rom.prom[0x100..0x107]).unwrap_or("<no-ascii>");
    let ngh = read_u16_be(&rom.prom, 0x108);
    println!("[PHEAD] sp=0x{initial_sp:08X} reset=0x{reset:08X} header='{header}' ngh=0x{ngh:04X}");
}

fn read_u16_be(data: &[u8], offset: usize) -> u16 {
    u16::from_be_bytes([data[offset], data[offset + 1]])
}

fn read_u32_be(data: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

fn default_output_path(rom_path: &Path) -> PathBuf {
    let stem = rom_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("rom");
    let mut path = PathBuf::from("screenshots");
    path.push(format!("{}_probe.bmp", sanitize_filename(stem)));
    path
}

fn diagnostic_output_path(output_path: &Path, suffix: &str) -> PathBuf {
    let parent = output_path.parent().unwrap_or_else(|| Path::new(""));
    let stem = output_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("probe");
    let extension = output_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("bmp");
    parent.join(format!("{stem}_{suffix}.{extension}"))
}

fn sanitize_filename(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();

    let trimmed = sanitized.trim_matches('_');
    if trimmed.is_empty() {
        "rom".to_string()
    } else {
        trimmed.to_string()
    }
}

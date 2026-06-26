//! Save/Load States — serialización completa del estado de emulación.
//!
//! Formato binario (little-endian):
//! [HEADER]        12 bytes  (magic + version + flags)
//! [M68K_STATE]    variable (Musashi context, queried via m68k_context_size)
//! [Z80_STATE]     28 bytes  (registros Z80)
//! [YM2610_SIZE]   4 bytes (version >= 8)
//! [YM2610_STATE]  variable (via Ym2610::save_state/load_state)
//! [MEMORY_STATE]  variable (RAM, VRAM, palette, back RAM, memcard, regs video)
//! [MEM_CTRL]      45 bytes  (vram_addr, lspc_mode, prom_bank, flags…)
//! [PVC_BANK]       4 bytes  (version >= 3)
//! [LSPC_TIMING]    8 bytes  (version >= 6, auto-animation + scanline)
//! [KOF10TH_RAM]    variable (version >= 7, extra RAM + dynamic FIX)
//! [SPECIAL_BOARD]  16 bytes  (version >= 10, protection registers)

use crate::audio::AudioMixer;
use crate::cpu::Cpu;
use crate::memory::{Memory, SpecialBoardState};
use crate::video::Video;
use crate::ym2610::{Ym2610, YM2610_LEGACY_SAVE_STATE_SIZE};
use crate::z80::Z80;
use z80emu::{Cpu as _, InterruptMode, Prefix, StkReg16};

// ─── Magic & version ──────────────────────────────────────────────────────

const SAVE_MAGIC: [u8; 8] = *b"NGNEONST";
const SAVE_VERSION: u32 = 10; // bumped for special-board protection state

// ─── Helpers LE ───────────────────────────────────────────────────────────

fn write_u32(buf: &mut [u8], off: usize, v: u32) {
    buf[off..off + 4].copy_from_slice(&v.to_le_bytes());
}
fn read_u32(buf: &[u8], off: usize) -> u32 {
    u32::from_le_bytes(buf[off..off + 4].try_into().unwrap())
}
fn write_u16(buf: &mut [u8], off: usize, v: u16) {
    buf[off..off + 2].copy_from_slice(&v.to_le_bytes());
}
fn read_u16(buf: &[u8], off: usize) -> u16 {
    u16::from_le_bytes(buf[off..off + 2].try_into().unwrap())
}
fn write_u8(buf: &mut [u8], off: usize, v: u8) {
    buf[off] = v;
}
fn read_u8(buf: &[u8], off: usize) -> u8 {
    buf[off]
}

// ─── Serialización principal ─────────────────────────────────────────────

/// Serializa el estado completo de emulación a un buffer binario.
pub fn serialize_all(
    cpu: &mut Cpu, // &mut needed for save_context()
    z80: &Z80,
    ym2610: &Ym2610,
    _video: &Video,
    mem: &Memory,
    _mixer: &AudioMixer,
) -> Vec<u8> {
    let ctx = cpu.save_context();
    let ctx_size = ctx.len() as u32;

    let ym_state = ym2610.save_state();
    let mem_size = 12 + 4 + 4 // cabecera + tamaño de cada sección
        + mem.ram.len() + 4
        + mem.vram.len() + 4
        + mem.palette_ram.len() + 4
        + mem.backup_ram.len() + 4
        + mem.memory_card.len() + 4
        + mem.adpcm_a_ram.len() + 4
        + mem.adpcm_b_ram.len() + 4
        + mem.pvc_cart_ram.len() + 4
        + mem.kof10th_extra_ram.len() + 4
        + mem.dynamic_fix_rom.len() + 4
        + 45 // memoria de control
        + 4  // pvc_bank_addr
        + 8  // LSPC timing
        + 16; // special-board state

    let total = 12 + 4 + ctx.len() + 28 + 4 + ym_state.len() + mem_size;
    let mut buf = vec![0u8; total];
    let mut pos: usize;

    // ── Header ─────────────────────────────────────────────────────────
    buf[0..8].copy_from_slice(&SAVE_MAGIC);
    write_u32(&mut buf, 8, SAVE_VERSION);
    pos = 12;

    // ── 1. M68K state (Musashi context) ────────────────────────────────
    write_u32(&mut buf, pos, ctx_size);
    pos += 4;
    buf[pos..pos + ctx.len()].copy_from_slice(ctx);
    pos += ctx.len();

    // ── 2. Z80 state ───────────────────────────────────────────────────
    {
        let zcpu = &z80.cpu;
        write_u16(&mut buf, pos, zcpu.get_pc());
        write_u16(&mut buf, pos + 2, zcpu.get_sp());
        write_u16(&mut buf, pos + 4, zcpu.get_reg16(StkReg16::AF));
        write_u16(&mut buf, pos + 6, zcpu.get_reg16(StkReg16::BC));
        write_u16(&mut buf, pos + 8, zcpu.get_reg16(StkReg16::DE));
        write_u16(&mut buf, pos + 10, zcpu.get_reg16(StkReg16::HL));
        write_u16(&mut buf, pos + 12, zcpu.get_index16(Prefix::Xdd));
        write_u16(&mut buf, pos + 14, zcpu.get_index16(Prefix::Yfd));
        // Shadow registers
        write_u16(&mut buf, pos + 16, zcpu.get_alt_reg16(StkReg16::AF));
        write_u16(&mut buf, pos + 18, zcpu.get_alt_reg16(StkReg16::BC));
        write_u16(&mut buf, pos + 20, zcpu.get_alt_reg16(StkReg16::DE));
        write_u16(&mut buf, pos + 22, zcpu.get_alt_reg16(StkReg16::HL));
        write_u8(&mut buf, pos + 24, zcpu.get_i());
        write_u8(&mut buf, pos + 25, zcpu.get_r());
        write_u8(&mut buf, pos + 26, zcpu.get_im() as u8);
        write_u8(&mut buf, pos + 27, zcpu.get_iffs().0 as u8);
    }
    pos += 28;

    // ── 3. YM2610 state ────────────────────────────────────────────────
    write_u32(&mut buf, pos, ym_state.len() as u32);
    pos += 4;
    buf[pos..pos + ym_state.len()].copy_from_slice(&ym_state);
    pos += ym_state.len();

    // ── 4. Memory state (memorias planas) ──────────────────────────────
    let mem_sections: [&[u8]; 10] = [
        &mem.ram,
        &mem.vram,
        &mem.palette_ram,
        &mem.backup_ram,
        &mem.memory_card,
        &mem.adpcm_a_ram,
        &mem.adpcm_b_ram,
        &mem.pvc_cart_ram,
        &mem.kof10th_extra_ram,
        &mem.dynamic_fix_rom,
    ];
    for section in &mem_sections {
        write_u32(&mut buf, pos, section.len() as u32);
        pos += 4;
        buf[pos..pos + section.len()].copy_from_slice(section);
        pos += section.len();
    }

    // ── 5. Memory control state ────────────────────────────────────────
    write_u32(&mut buf, pos, mem.vram_addr.get() as u32);
    write_u32(&mut buf, pos + 4, mem.vram_mod as u32);
    write_u32(&mut buf, pos + 8, mem.lspc_mode as u32);
    write_u32(&mut buf, pos + 12, mem.prom_bank_offset as u32);
    for i in 0..4 {
        write_u32(&mut buf, pos + 16 + i * 4, mem.z80_bank[i] as u32);
    }
    write_u8(&mut buf, pos + 32, mem.timer_high as u8);
    write_u8(&mut buf, pos + 33, mem.timer_low as u8);
    write_u8(&mut buf, pos + 34, mem.timer_stop as u8);
    write_u8(&mut buf, pos + 35, mem.display_enabled as u8);
    write_u8(&mut buf, pos + 36, mem.use_cart_vectors as u8);
    write_u8(&mut buf, pos + 37, mem.use_cart_audio as u8);
    write_u8(&mut buf, pos + 38, mem.use_cart_fix as u8);
    write_u8(&mut buf, pos + 39, mem.save_ram_unlocked as u8);
    write_u8(&mut buf, pos + 40, mem.palette_bank);
    write_u8(&mut buf, pos + 41, mem.memcard_unlocked as u8);
    write_u8(&mut buf, pos + 42, mem.memcard_inserted as u8);
    write_u8(&mut buf, pos + 43, mem.memcard_write_protected as u8);
    write_u8(&mut buf, pos + 44, mem.palette_shadow as u8);
    pos += 45;

    // ── 6. NEO-PVC bank address ─────────────────────────────────────
    write_u32(&mut buf, pos, mem.pvc_bank_addr as u32);
    pos += 4;

    // ── 7. LSPC timing state ───────────────────────────────────────
    write_u8(&mut buf, pos, mem.auto_animation_counter.get());
    write_u8(&mut buf, pos + 1, mem.auto_animation_timer.get());
    write_u8(&mut buf, pos + 2, mem.auto_animation_reload.get());
    write_u16(&mut buf, pos + 4, mem.lspc_scanline.get());
    pos += 8;

    // ── 8. Special-board protection state ─────────────────────────
    let board = mem.special_board_state();
    write_u16(&mut buf, pos, board.cart_reg[0]);
    write_u16(&mut buf, pos + 2, board.cart_reg[1]);
    write_u32(&mut buf, pos + 4, board.prot_reg);
    write_u16(&mut buf, pos + 8, board.mslugx_command);
    write_u16(&mut buf, pos + 10, board.mslugx_counter);
    write_u16(&mut buf, pos + 12, board.sma_rng);
    write_u16(
        &mut buf,
        pos + 14,
        board.pending_sma_bank_hi.map_or(u16::MAX, u16::from),
    );
    pos += 16;

    buf.truncate(pos);
    buf
}

// ─── Deserialización ─────────────────────────────────────────────────────

/// Carga el estado completo de emulación desde un buffer binario.
#[allow(clippy::too_many_arguments)]
pub fn deserialize_all(
    data: &[u8],
    cpu: &mut Cpu,
    z80: &mut Z80,
    ym2610: &mut Ym2610,
    _video: &mut Video,
    mem: &mut Memory,
    _mixer: &mut AudioMixer,
) -> Result<usize, &'static str> {
    if data.len() < 12 + 4 {
        return Err("SaveState: datos demasiado cortos");
    }

    // ── Header ─────────────────────────────────────────────────────────
    if data[0..8] != SAVE_MAGIC {
        return Err("SaveState: magic inválido");
    }
    let version = read_u32(data, 8);
    let mut pos: usize = 12;

    // ── 1. M68K state (Musashi context) ────────────────────────────────
    {
        if data.len() < pos + 4 {
            return Err("SaveState: falta tamaño ctx");
        }
        let ctx_size = read_u32(data, pos) as usize;
        pos += 4;
        if data.len() < pos + ctx_size {
            return Err("SaveState: datos ctx insuficientes");
        }
        cpu.restore_context(&data[pos..pos + ctx_size]);
        pos += ctx_size;
    }

    // ── 2. Z80 state ───────────────────────────────────────────────────
    if data.len() < pos + 28 {
        return Err("SaveState: datos Z80 insuficientes");
    }
    {
        let zcpu = &mut z80.cpu;
        zcpu.set_pc(read_u16(data, pos));
        zcpu.set_sp(read_u16(data, pos + 2));
        zcpu.set_reg16(StkReg16::AF, read_u16(data, pos + 4));
        zcpu.set_reg16(StkReg16::BC, read_u16(data, pos + 6));
        zcpu.set_reg16(StkReg16::DE, read_u16(data, pos + 8));
        zcpu.set_reg16(StkReg16::HL, read_u16(data, pos + 10));
        zcpu.set_index16(Prefix::Xdd, read_u16(data, pos + 12));
        zcpu.set_index16(Prefix::Yfd, read_u16(data, pos + 14));
        // Shadow registers
        let af_shadow = read_u16(data, pos + 16);
        let bc_shadow = read_u16(data, pos + 18);
        let de_shadow = read_u16(data, pos + 20);
        let hl_shadow = read_u16(data, pos + 22);
        zcpu.ex_af_af();
        zcpu.set_reg16(StkReg16::AF, af_shadow);
        zcpu.exx();
        zcpu.set_reg16(StkReg16::BC, bc_shadow);
        zcpu.set_reg16(StkReg16::DE, de_shadow);
        zcpu.set_reg16(StkReg16::HL, hl_shadow);
        zcpu.exx();
        zcpu.ex_af_af();
        zcpu.set_i(read_u8(data, pos + 24));
        zcpu.set_r(read_u8(data, pos + 25));
        let im_val = read_u8(data, pos + 26);
        zcpu.set_im(InterruptMode::try_from(im_val).unwrap_or_default());
        let iff1 = read_u8(data, pos + 27) != 0;
        zcpu.set_iffs(iff1, iff1);
    }
    pos += 28;

    // ── 3. YM2610 state ────────────────────────────────────────────────
    if data.len() <= pos {
        return Err("SaveState: datos YM2610 insuficientes");
    }
    let ym_size = if version >= 8 {
        if data.len() < pos + 4 {
            return Err("SaveState: falta tamaño YM2610");
        }
        let size = read_u32(data, pos) as usize;
        pos += 4;
        size
    } else if version >= 5 {
        YM2610_LEGACY_SAVE_STATE_SIZE
    } else if version >= 4 {
        606usize
    } else {
        600usize
    };
    if data.len() < pos + ym_size {
        return Err("SaveState: datos YM2610 insuficientes");
    }
    ym2610.load_state(&data[pos..pos + ym_size])?;
    pos += ym_size;

    // ── 4. Memory state ────────────────────────────────────────────────
    {
        let mem_dests: &mut [&mut [u8]] = &mut [
            &mut mem.ram,
            &mut mem.vram,
            &mut mem.palette_ram,
            &mut mem.backup_ram,
            &mut mem.memory_card,
            &mut mem.adpcm_a_ram,
            &mut mem.adpcm_b_ram,
            &mut mem.pvc_cart_ram,
        ];
        for dest in mem_dests {
            if data.len() < pos + 4 {
                return Err("SaveState: datos memory insuficientes");
            }
            let len = read_u32(data, pos) as usize;
            pos += 4;
            if data.len() < pos + len || len > dest.len() {
                return Err("SaveState: datos memory insuficientes");
            }
            dest[..len].copy_from_slice(&data[pos..pos + len]);
            pos += len;
        }

        if version >= 7 {
            let mem_dests_v7: &mut [&mut [u8]] =
                &mut [&mut mem.kof10th_extra_ram, &mut mem.dynamic_fix_rom];
            for dest in mem_dests_v7 {
                if data.len() < pos + 4 {
                    return Err("SaveState: datos memory v7 insuficientes");
                }
                let len = read_u32(data, pos) as usize;
                pos += 4;
                if data.len() < pos + len || len > dest.len() {
                    return Err("SaveState: datos memory v7 insuficientes");
                }
                dest[..len].copy_from_slice(&data[pos..pos + len]);
                pos += len;
            }
        }
    }

    // ── 5. Memory control state ────────────────────────────────────────
    let mem_ctrl_size = if version >= 9 { 45 } else { 44 };
    if data.len() < pos + mem_ctrl_size {
        return Err("SaveState: datos control memory insuficientes");
    }
    mem.vram_addr.set(read_u32(data, pos) as u16);
    mem.vram_mod = read_u32(data, pos + 4) as u16;
    mem.lspc_mode = read_u32(data, pos + 8) as u16;
    mem.prom_bank_offset = read_u32(data, pos + 12) as usize;
    for i in 0..4 {
        mem.z80_bank[i] = read_u32(data, pos + 16 + i * 4) as usize;
    }
    mem.timer_high = read_u8(data, pos + 32) as u16;
    mem.timer_low = read_u8(data, pos + 33) as u16;
    mem.timer_stop = read_u8(data, pos + 34) as u16;
    mem.display_enabled = read_u8(data, pos + 35) != 0;
    mem.use_cart_vectors = read_u8(data, pos + 36) != 0;
    mem.use_cart_audio = read_u8(data, pos + 37) != 0;
    mem.use_cart_fix = read_u8(data, pos + 38) != 0;
    mem.save_ram_unlocked = read_u8(data, pos + 39) != 0;
    mem.palette_bank = read_u8(data, pos + 40);
    mem.memcard_unlocked = read_u8(data, pos + 41) != 0;
    mem.memcard_lock1 = !mem.memcard_unlocked;
    mem.memcard_lock2 = false;
    mem.memcard_inserted = read_u8(data, pos + 42) != 0;
    mem.memcard_write_protected = read_u8(data, pos + 43) != 0;
    mem.palette_shadow = version >= 9 && read_u8(data, pos + 44) != 0;
    pos += mem_ctrl_size;

    // ── 6. NEO-PVC bank address (version >= 3) ─────────────────────
    if version >= 3 {
        if data.len() < pos + 4 {
            return Err("SaveState: datos PVC bank_addr insuficientes");
        }
        mem.pvc_bank_addr = read_u32(data, pos) as usize;
        pos += 4;
    }

    // ── 7. LSPC timing state (version >= 6) ───────────────────────
    if version >= 6 {
        if data.len() < pos + 8 {
            return Err("SaveState: datos LSPC timing insuficientes");
        }
        mem.auto_animation_counter.set(read_u8(data, pos));
        mem.auto_animation_timer.set(read_u8(data, pos + 1));
        mem.auto_animation_reload.set(read_u8(data, pos + 2));
        mem.lspc_scanline.set(read_u16(data, pos + 4) % 264);
        pos += 8;
    } else {
        mem.auto_animation_reload.set((mem.lspc_mode >> 8) as u8);
    }

    // ── 8. Special-board protection state (version >= 10) ─────────
    if version >= 10 {
        if data.len() < pos + 16 {
            return Err("SaveState: datos placa especial insuficientes");
        }
        let pending_sma = read_u16(data, pos + 14);
        mem.restore_special_board_state(SpecialBoardState {
            cart_reg: [read_u16(data, pos), read_u16(data, pos + 2)],
            prot_reg: read_u32(data, pos + 4),
            mslugx_command: read_u16(data, pos + 8),
            mslugx_counter: read_u16(data, pos + 10),
            sma_rng: read_u16(data, pos + 12),
            pending_sma_bank_hi: (pending_sma != u16::MAX).then_some(pending_sma as u8),
        });
        pos += 16;
    } else {
        mem.restore_special_board_state(SpecialBoardState::default());
    }

    Ok(pos)
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::AudioMixer;
    use crate::cpu::Cpu;
    use crate::memory::Memory;
    use crate::video::Video;
    use crate::ym2610::Ym2610;
    use crate::z80::Z80;
    use std::cell::RefCell;
    use std::rc::Rc;

    type TestComponents = (
        Cpu,
        Z80,
        Rc<RefCell<Ym2610>>,
        Video,
        Rc<RefCell<Memory>>,
        AudioMixer,
    );

    fn setup_test_components() -> TestComponents {
        let mem = Rc::new(RefCell::new(Memory::new()));
        let ym2610 = Rc::new(RefCell::new(Ym2610::new(mem.clone())));
        let cpu = Cpu::new(mem.clone());
        let zbus = Z80::new(mem.clone(), ym2610.clone());
        let video = Video::new();
        let mixer = AudioMixer::new();
        (cpu, zbus, ym2610, video, mem, mixer)
    }

    fn kof98_test_rom() -> crate::rom::RomData {
        let mut rom = crate::rom::RomData::demo();
        rom.is_demo = false;
        rom.prom = vec![0; 0x20_0000];
        rom.prom[0x100..0x104].copy_from_slice(b"NEO-");
        rom.metadata = Some(crate::rom::NeoMetadata {
            version: 1,
            year: 1998,
            genre: 0,
            screenshot: 0,
            ngh: 0x0242,
            name: "savestate-kof98-test".to_string(),
            manufacturer: "SNK".to_string(),
            board_type: crate::rom::NeoBoardType::Kof98,
            fix_banksw: crate::rom::NeoFixBanksw::None,
            game_flags: 0,
        });
        rom
    }

    #[test]
    fn test_save_roundtrip() {
        let (mut cpu, zbus, ym2610_rc, video, mem_rc, mixer) = setup_test_components();

        // Set recognizable values via Musashi registers
        cpu.set_reg(crate::musashi_ffi::M68K_REG_PC, 0x12345678);
        cpu.set_reg(crate::musashi_ffi::M68K_REG_D0, 0xDEADBEEF);
        cpu.set_reg(crate::musashi_ffi::M68K_REG_A7, 0x80000100);

        // Set some YM2610 state
        {
            let mut ym = ym2610_rc.borrow_mut();
            ym.write_address(0, 0x30);
            ym.write_data(0, 0x3F);
            ym.write_address(0, 0x28);
            ym.write_data(0, 0xF1);
        }

        // Serialize
        let mem = mem_rc.borrow();
        let data = serialize_all(&mut cpu, &zbus, &ym2610_rc.borrow(), &video, &mem, &mixer);
        let data_len = data.len();
        drop(mem);

        // Verify magic
        assert!(data_len > 12);
        assert_eq!(&data[0..8], &SAVE_MAGIC);

        // Create fresh components and deserialize
        let mem2 = Rc::new(RefCell::new(Memory::new()));
        let ym2 = Rc::new(RefCell::new(Ym2610::new(mem2.clone())));
        let mut cpu2 = Cpu::new(mem2.clone());
        let mut zbus2 = Z80::new(mem2.clone(), ym2.clone());
        let mut video2 = Video::new();
        let mut mixer2 = AudioMixer::new();

        let result = deserialize_all(
            &data,
            &mut cpu2,
            &mut zbus2,
            &mut ym2.borrow_mut(),
            &mut video2,
            &mut mem2.borrow_mut(),
            &mut mixer2,
        );
        assert!(result.is_ok(), "deserialize_all failed: {:?}", result);

        // Verify key values
        assert_eq!(cpu2.get_reg(crate::musashi_ffi::M68K_REG_PC), 0x12345678);
        assert_eq!(cpu2.get_reg(crate::musashi_ffi::M68K_REG_D0), 0xDEADBEEF);
        assert_eq!(cpu2.get_reg(crate::musashi_ffi::M68K_REG_A7), 0x80000100);

        // Verify YM2610 state restored
        let ym2 = ym2.borrow();
        assert_eq!(ym2.reg[0][0x30], 0x3F);
    }

    #[test]
    fn test_save_invalid_magic() {
        let (mut cpu, mut zbus, ym2610_rc, mut video, mem_rc, mut mixer) = setup_test_components();
        let data = vec![0; 100];
        let result = deserialize_all(
            &data,
            &mut cpu,
            &mut zbus,
            &mut ym2610_rc.borrow_mut(),
            &mut video,
            &mut mem_rc.borrow_mut(),
            &mut mixer,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_save_roundtrip_memory_values() {
        let (mut cpu, zbus, ym2610_rc, video, mem_rc, mixer) = setup_test_components();

        // Write values to memory
        mem_rc.borrow_mut().ram[0] = 0x42;
        mem_rc.borrow_mut().ram[0xFF] = 0xAB;
        mem_rc.borrow_mut().vram[0x100] = 0xCD;
        {
            let mut mem = mem_rc.borrow_mut();
            mem.lspc_mode = 0x0300;
            mem.auto_animation_counter.set(5);
            mem.auto_animation_timer.set(7);
            mem.auto_animation_reload.set(3);
            mem.lspc_scanline.set(123);
            mem.kof10th_extra_ram.resize(0x20000, 0);
            mem.dynamic_fix_rom.resize(0x20000, 0);
            mem.kof10th_extra_ram[0x20] = 0x66;
            mem.dynamic_fix_rom[0x40] = 0x77;
        }

        let mem = mem_rc.borrow();
        let data = serialize_all(&mut cpu, &zbus, &ym2610_rc.borrow(), &video, &mem, &mixer);
        drop(mem);

        // Deserialize into fresh memory
        let mem2 = Rc::new(RefCell::new(Memory::new()));
        {
            let mut mem2_borrow = mem2.borrow_mut();
            mem2_borrow.kof10th_extra_ram.resize(0x20000, 0);
            mem2_borrow.dynamic_fix_rom.resize(0x20000, 0);
        }
        let ym2 = Rc::new(RefCell::new(Ym2610::new(mem2.clone())));
        let mut cpu2 = Cpu::new(mem2.clone());
        let mut zbus2 = Z80::new(mem2.clone(), ym2.clone());
        let mut video2 = Video::new();
        let mut mixer2 = AudioMixer::new();

        deserialize_all(
            &data,
            &mut cpu2,
            &mut zbus2,
            &mut ym2.borrow_mut(),
            &mut video2,
            &mut mem2.borrow_mut(),
            &mut mixer2,
        )
        .unwrap();

        let mem2 = mem2.borrow();
        assert_eq!(mem2.ram[0], 0x42);
        assert_eq!(mem2.ram[0xFF], 0xAB);
        assert_eq!(mem2.vram[0x100], 0xCD);
        assert_eq!(mem2.auto_animation_counter.get(), 5);
        assert_eq!(mem2.auto_animation_timer.get(), 7);
        assert_eq!(mem2.auto_animation_reload.get(), 3);
        assert_eq!(mem2.lspc_scanline.get(), 123);
        assert_eq!(mem2.kof10th_extra_ram[0x20], 0x66);
        assert_eq!(mem2.dynamic_fix_rom[0x40], 0x77);
    }

    #[test]
    fn test_save_roundtrip_special_board_state_and_kof98_overlay() {
        let (mut cpu, zbus, ym2610_rc, video, mem_rc, mixer) = setup_test_components();
        mem_rc.borrow_mut().load_rom(&mut kof98_test_rom());

        let expected = SpecialBoardState {
            cart_reg: [0x0090, 0x55aa],
            prot_reg: 0x8142_2418,
            mslugx_command: 0x0fff,
            mslugx_counter: 0x1234,
            sma_rng: 0xbeef,
            pending_sma_bank_hi: Some(0x7c),
        };
        mem_rc.borrow_mut().restore_special_board_state(expected);
        assert_eq!(
            &mem_rc.borrow().prom[0x100..0x104],
            &[0x00, 0xc2, 0x00, 0xfd]
        );

        let data = serialize_all(
            &mut cpu,
            &zbus,
            &ym2610_rc.borrow(),
            &video,
            &mem_rc.borrow(),
            &mixer,
        );

        let mem2 = Rc::new(RefCell::new(Memory::new()));
        mem2.borrow_mut().load_rom(&mut kof98_test_rom());
        let ym2 = Rc::new(RefCell::new(Ym2610::new(mem2.clone())));
        let mut cpu2 = Cpu::new(mem2.clone());
        let mut zbus2 = Z80::new(mem2.clone(), ym2.clone());
        let mut video2 = Video::new();
        let mut mixer2 = AudioMixer::new();

        deserialize_all(
            &data,
            &mut cpu2,
            &mut zbus2,
            &mut ym2.borrow_mut(),
            &mut video2,
            &mut mem2.borrow_mut(),
            &mut mixer2,
        )
        .expect("special-board savestate should load");

        let mem2 = mem2.borrow();
        assert_eq!(mem2.special_board_state(), expected);
        assert_eq!(&mem2.prom[0x100..0x104], &[0x00, 0xc2, 0x00, 0xfd]);
    }

    #[test]
    fn test_save_roundtrip_geolith_ymfm_state_block() {
        let (mut cpu, zbus, ym2610_rc, video, mem_rc, mixer) = setup_test_components();
        {
            let mut ym = ym2610_rc.borrow_mut();
            ym.enable_geolith_backend();
            ym.write_address(0, 0x27);
            ym.write_data(0, 0x30);
            let mut audio = vec![0i16; 64];
            ym.generate(&mut audio, 32);
        }

        let mem = mem_rc.borrow();
        let data = serialize_all(&mut cpu, &zbus, &ym2610_rc.borrow(), &video, &mem, &mixer);
        drop(mem);

        assert!(
            data.windows(4).any(|window| window == b"GYMF"),
            "Geolith YMFM state block must be embedded in version 8 savestates"
        );

        let mem2 = Rc::new(RefCell::new(Memory::new()));
        let ym2 = Rc::new(RefCell::new(Ym2610::new(mem2.clone())));
        ym2.borrow_mut().enable_geolith_backend();
        let mut cpu2 = Cpu::new(mem2.clone());
        let mut zbus2 = Z80::new(mem2.clone(), ym2.clone());
        let mut video2 = Video::new();
        let mut mixer2 = AudioMixer::new();

        deserialize_all(
            &data,
            &mut cpu2,
            &mut zbus2,
            &mut ym2.borrow_mut(),
            &mut video2,
            &mut mem2.borrow_mut(),
            &mut mixer2,
        )
        .expect("Geolith YMFM savestate should load");
    }
}

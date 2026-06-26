use core_emulator::{memory::PALETTE_RAM_START, rom::NEO_HEADER_SIZE, video};
use std::path::PathBuf;

const DEFAULT_OUTPUT: &str = "examples/ngneon_test.neo";
const PROGRAM_SIZE: usize = 0x20000;
const LSPC_VRAMADDR: u32 = 0x3C0000;
const LSPC_VRAMRW: u32 = 0x3C0002;
const LSPC_VRAMMOD: u32 = 0x3C0004;
const SCB1_START: u16 = 0x0000;
const SCB1_WORDS_PER_SPRITE: u16 = 64;
const SCB2_START: u16 = 0x8000;
const SCB3_START: u16 = 0x8200;
const SCB4_START: u16 = 0x8400;
const FIX_MAP_START: u16 = 0x7000;
const FIX_MAP_ROWS: u16 = 32;
const FIX_HIDDEN_TOP_ROWS: u16 = 2;
const TEST_TILES: usize = (video::SCREEN_WIDTH / video::SPRITE_TILE_WIDTH)
    * (video::SCREEN_HEIGHT / video::SPRITE_TILE_HEIGHT);
const TEST_SPRITE: u16 = 1;
const TEST_COLORS: [u16; 16] = [
    0x8000, 0x4F00, 0x20F0, 0x100F, 0x6FF0, 0x5F0F, 0x30FF, 0x7FFF, 0x0888, 0x0444, 0x0222, 0x0CCC,
    0x2AA0, 0x5A0A, 0x1A5A, 0x6AAA,
];

fn main() -> Result<(), String> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_OUTPUT));

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("No se pudo crear {:?}: {error}", parent))?;
    }

    let neo = build_test_neo();
    std::fs::write(&output, neo)
        .map_err(|error| format!("No se pudo escribir {:?}: {error}", output))?;

    println!("ROM .neo de prueba generada en {:?}", output);
    Ok(())
}

fn build_test_neo() -> Vec<u8> {
    let prom = build_program_rom();
    let srom = build_fix_bank();
    let crom = build_graphics_bank();
    let mut header = vec![0; NEO_HEADER_SIZE];
    header[0] = b'N';
    header[1] = b'E';
    header[2] = b'O';
    header[3] = 1;
    write_u32_le(&mut header, 0x04, prom.len() as u32);
    write_u32_le(&mut header, 0x08, srom.len() as u32);
    write_u32_le(&mut header, 0x18, crom.len() as u32);
    write_u32_le(&mut header, 0x1C, 2026);
    write_u32_le(&mut header, 0x20, 0);
    write_u32_le(&mut header, 0x24, 0);
    write_u32_le(&mut header, 0x28, 0x0E0E);
    write_fixed_string(&mut header, 0x2C, 33, "NGNEON TEST");
    write_fixed_string(&mut header, 0x4D, 17, "NGNEON");

    let mut neo = header;
    neo.extend(prom);
    neo.extend(srom);
    neo.extend(crom);
    neo
}

fn build_program_rom() -> Vec<u8> {
    let mut prom = vec![0xFF; PROGRAM_SIZE];
    write_u32_be(&mut prom, 0x00, 0x0010_FF00);
    write_u32_be(&mut prom, 0x04, 0x0000_0100);

    let mut cursor = 0x100;
    for (index, color) in TEST_COLORS.into_iter().enumerate() {
        append_move_word_immediate_to_absolute_long(
            &mut prom,
            &mut cursor,
            color,
            PALETTE_RAM_START + (index as u32 * 2),
        );
    }

    let scb1 = SCB1_START + TEST_SPRITE * SCB1_WORDS_PER_SPRITE;
    append_vram_word_write(&mut prom, &mut cursor, scb1, 0);
    append_vram_word_write(&mut prom, &mut cursor, scb1 + 1, 0);
    append_vram_word_write(&mut prom, &mut cursor, SCB2_START + TEST_SPRITE, 0x0FFF);
    append_vram_word_write(
        &mut prom,
        &mut cursor,
        SCB3_START + TEST_SPRITE,
        ((496 - 72) << 7) | 1,
    );
    append_vram_word_write(&mut prom, &mut cursor, SCB4_START + TEST_SPRITE, 112 << 7);
    append_vram_word_write(
        &mut prom,
        &mut cursor,
        fix_map_address(16, FIX_HIDDEN_TOP_ROWS),
        1,
    );

    // NOP; BRA.S -2 (loops on the branch instruction)
    prom[cursor] = 0x4E;
    prom[cursor + 1] = 0x71;
    prom[cursor + 2] = 0x60;
    prom[cursor + 3] = 0xFE;
    prom
}

fn fix_map_address(col: u16, row: u16) -> u16 {
    FIX_MAP_START + col * FIX_MAP_ROWS + row
}

fn build_fix_bank() -> Vec<u8> {
    let mut srom = Vec::with_capacity(64);
    srom.extend([0; 32]);
    append_solid_fix_tile(&mut srom, 2);
    srom
}

fn build_graphics_bank() -> Vec<u8> {
    let mut crom = Vec::with_capacity(TEST_TILES * video::BYTES_PER_TILE);
    for tile in 0..TEST_TILES {
        let color = (tile % 15 + 1) as u8;
        append_solid_sprite_tile(&mut crom, color);
    }
    crom
}

fn append_solid_sprite_tile(crom: &mut Vec<u8>, color: u8) {
    for _y in 0..video::SPRITE_TILE_HEIGHT {
        for _half in 0..2 {
            crom.push(if color & 0x01 != 0 { 0xFF } else { 0x00 });
            crom.push(if color & 0x04 != 0 { 0xFF } else { 0x00 });
            crom.push(if color & 0x02 != 0 { 0xFF } else { 0x00 });
            crom.push(if color & 0x08 != 0 { 0xFF } else { 0x00 });
        }
    }
}

fn append_solid_fix_tile(srom: &mut Vec<u8>, color: u8) {
    let start = srom.len();
    srom.resize(start + 32, 0);
    let packed = (color & 0x0f) | ((color & 0x0f) << 4);
    for y in 0..8 {
        for pair in 0..4 {
            srom[start + y * 4 + pair] = packed;
        }
    }
}

fn append_vram_word_write(prom: &mut [u8], cursor: &mut usize, vram_address: u16, value: u16) {
    append_move_word_immediate_to_absolute_long(prom, cursor, 1, LSPC_VRAMMOD);
    append_move_word_immediate_to_absolute_long(prom, cursor, vram_address, LSPC_VRAMADDR);
    append_move_word_immediate_to_absolute_long(prom, cursor, value, LSPC_VRAMRW);
}

fn append_move_word_immediate_to_absolute_long(
    prom: &mut [u8],
    cursor: &mut usize,
    value: u16,
    address: u32,
) {
    write_u16_be(prom, *cursor, 0x33FC);
    write_u16_be(prom, *cursor + 2, value);
    write_u32_be(prom, *cursor + 4, address);
    *cursor += 8;
}

fn write_u32_le(data: &mut [u8], offset: usize, value: u32) {
    data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u32_be(data: &mut [u8], offset: usize, value: u32) {
    data[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

fn write_u16_be(data: &mut [u8], offset: usize, value: u16) {
    data[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
}

fn write_fixed_string(data: &mut [u8], offset: usize, len: usize, value: &str) {
    let bytes = value.as_bytes();
    let copy_len = bytes.len().min(len);
    data[offset..offset + copy_len].copy_from_slice(&bytes[..copy_len]);
}

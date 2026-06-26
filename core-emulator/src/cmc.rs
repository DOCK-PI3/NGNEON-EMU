//! CMC protection helpers for late NeoGeo cartridges.
//!
//! The CMC50 M1 address descramble mirrors FBNeo's documented NeoGeo path:
//! load the encrypted Z80 ROM into a zero-filled 0x80000 buffer, derive the
//! 16-bit key from the first 0x10000 bytes, then copy bytes through the
//! scrambled address map.

#[path = "cmc_gfx_tables.rs"]
mod cmc_gfx_tables;

use cmc_gfx_tables::*;
use std::collections::BTreeMap;

pub const CMC_M1_DECRYPTED_SIZE: usize = 0x80000;
pub const CMC50_KOF2002_EXTRA_XOR: u8 = 0xb0;
const CMC_M1_KEY_SIZE: usize = 0x10000;
const CMC_GFX_MAX_DECRYPT_SIZE: usize = 0x0400_0000;
const CMC_GFX_BLOCK_SIZE: usize = 0x0040_0000;

struct CmcXorTables {
    table0hi: &'static [u8; 256],
    table0lo: &'static [u8; 256],
    table1: &'static [u8; 256],
}

struct CmcGfxTables {
    type0_t03: &'static [u8; 256],
    type0_t12: &'static [u8; 256],
    type1_t03: &'static [u8; 256],
    type1_t12: &'static [u8; 256],
    addr_0_7_xor: &'static [u8; 256],
    addr_8_15_xor1: &'static [u8; 256],
    addr_8_15_xor2: &'static [u8; 256],
    addr_16_23_xor1: &'static [u8; 256],
    addr_16_23_xor2: &'static [u8; 256],
}

pub const CMC42_MSLUG3_EXTRA_XOR: u8 = 0xad;
pub const CMC42_KOF99_EXTRA_XOR: u8 = 0x00;
pub const CMC42_GAROU_EXTRA_XOR: u8 = 0x06;
pub const CMC42_S1945P_EXTRA_XOR: u8 = 0x05;
pub const CMC42_ZUPAPA_EXTRA_XOR: u8 = 0xbd;
pub const CMC42_NITD_EXTRA_XOR: u8 = 0xff;
pub const CMC42_SENGOKU3_EXTRA_XOR: u8 = 0xfe;
pub const CMC42_PREISLE2_EXTRA_XOR: u8 = 0x9f;
pub const CMC42_BANGBEAD_EXTRA_XOR: u8 = 0xf8;
pub const CMC42_GANRYU_EXTRA_XOR: u8 = 0x07;

// CMC50 extra XOR values from FBNeo's NeoGeo CMC table notes.
pub const CMC50_MSLUG4_EXTRA_XOR: u8 = 0x6d;
pub const CMC50_KOF2003_EXTRA_XOR: u8 = 0x9d;
pub const CMC50_MSLUG5_EXTRA_XOR: u8 = 0x19;
pub const CMC50_ROTD_EXTRA_XOR: u8 = 0x3f;
pub const CMC50_MATRIM_EXTRA_XOR: u8 = 0x6a;
pub const CMC50_PNYAA_EXTRA_XOR: u8 = 0x2e;
pub const CMC50_SAMSHO5_EXTRA_XOR: u8 = 0x0f;
pub const CMC50_SAMSH5SP_EXTRA_XOR: u8 = 0x0d;
pub const CMC50_SVC_EXTRA_XOR: u8 = 0x57;
pub const CMC50_KOG_EXTRA_XOR: u8 = 0x5c;
pub const CMC50_JOCKEYGP_EXTRA_XOR: u8 = 0xf0;
pub const CMC50_KOF2001_EXTRA_XOR: u8 = 0x42;

const CMC50_TABLES: CmcGfxTables = CmcGfxTables {
    type0_t03: &KOF2000_TYPE0_T03,
    type0_t12: &KOF2000_TYPE0_T12,
    type1_t03: &KOF2000_TYPE1_T03,
    type1_t12: &KOF2000_TYPE1_T12,
    addr_0_7_xor: &KOF2000_ADDRESS_0_7_XOR,
    addr_8_15_xor1: &KOF2000_ADDRESS_8_15_XOR1,
    addr_8_15_xor2: &KOF2000_ADDRESS_8_15_XOR2,
    addr_16_23_xor1: &KOF2000_ADDRESS_16_23_XOR1,
    addr_16_23_xor2: &KOF2000_ADDRESS_16_23_XOR2,
};

const CMC42_TABLES: CmcGfxTables = CmcGfxTables {
    type0_t03: &KOF99_TYPE0_T03,
    type0_t12: &KOF99_TYPE0_T12,
    type1_t03: &KOF99_TYPE1_T03,
    type1_t12: &KOF99_TYPE1_T12,
    addr_0_7_xor: &KOF99_ADDRESS_0_7_XOR,
    addr_8_15_xor1: &KOF99_ADDRESS_8_15_XOR1,
    addr_8_15_xor2: &KOF99_ADDRESS_8_15_XOR2,
    addr_16_23_xor1: &KOF99_ADDRESS_16_23_XOR1,
    addr_16_23_xor2: &KOF99_ADDRESS_16_23_XOR2,
};

const M1_ADDRESS_8_15_XOR: [u8; 256] = [
    0x0a, 0x72, 0xb7, 0xaf, 0x67, 0xde, 0x1d, 0xb1, 0x78, 0xc4, 0x4f, 0xb5, 0x4b, 0x18, 0x76, 0xdd,
    0x11, 0xe2, 0x36, 0xa1, 0x82, 0x03, 0x98, 0xa0, 0x10, 0x5f, 0x3f, 0xd6, 0x1f, 0x90, 0x6a, 0x0b,
    0x70, 0xe0, 0x64, 0xcb, 0x9f, 0x38, 0x8b, 0x53, 0x04, 0xca, 0xf8, 0xd0, 0x07, 0x68, 0x56, 0x32,
    0xae, 0x1c, 0x2e, 0x48, 0x63, 0x92, 0x9a, 0x9c, 0x44, 0x85, 0x41, 0x40, 0x09, 0xc0, 0xc8, 0xbf,
    0xea, 0xbb, 0xf7, 0x2d, 0x99, 0x21, 0xf6, 0xba, 0x15, 0xce, 0xab, 0xb0, 0x2a, 0x60, 0xbc, 0xf1,
    0xf0, 0x9e, 0xd5, 0x97, 0xd8, 0x4e, 0x14, 0x9d, 0x42, 0x4d, 0x2c, 0x5c, 0x2b, 0xa6, 0xe1, 0xa7,
    0xef, 0x25, 0x33, 0x7a, 0xeb, 0xe7, 0x1b, 0x6d, 0x4c, 0x52, 0x26, 0x62, 0xb6, 0x35, 0xbe, 0x80,
    0x01, 0xbd, 0xfd, 0x37, 0xf9, 0x47, 0x55, 0x71, 0xb4, 0xf2, 0xff, 0x27, 0xfa, 0x23, 0xc9, 0x83,
    0x17, 0x39, 0x13, 0x0d, 0xc7, 0x86, 0x16, 0xec, 0x49, 0x6f, 0xfe, 0x34, 0x05, 0x8f, 0x00, 0xe6,
    0xa4, 0xda, 0x7b, 0xc1, 0xf3, 0xf4, 0xd9, 0x75, 0x28, 0x66, 0x87, 0xa8, 0x45, 0x6c, 0x20, 0xe9,
    0x77, 0x93, 0x7e, 0x3c, 0x1e, 0x74, 0xf5, 0x8c, 0x3e, 0x94, 0xd4, 0xc2, 0x5a, 0x06, 0x0e, 0xe8,
    0x3d, 0xa9, 0xb2, 0xe3, 0xe4, 0x22, 0xcf, 0x24, 0x8e, 0x6b, 0x8a, 0x8d, 0x84, 0x4a, 0xd2, 0x91,
    0x88, 0x79, 0x57, 0xa5, 0x0f, 0xcd, 0xb9, 0xac, 0x3b, 0xaa, 0xb3, 0xd1, 0xee, 0x31, 0x81, 0x7c,
    0xd7, 0x89, 0xd3, 0x96, 0x43, 0xc5, 0xc6, 0xc3, 0x69, 0x7f, 0x46, 0xdf, 0x30, 0x5b, 0x6e, 0xe5,
    0x08, 0x95, 0x9b, 0xfb, 0xb8, 0x58, 0x0c, 0x61, 0x50, 0x5d, 0x3a, 0xa2, 0x29, 0x12, 0xfc, 0x51,
    0x7d, 0x1a, 0x02, 0x65, 0x54, 0x5e, 0x19, 0xcc, 0xdc, 0xdb, 0x73, 0xed, 0xad, 0x59, 0x2f, 0xa3,
];

const M1_ADDRESS_0_7_XOR: [u8; 256] = [
    0xf4, 0xbc, 0x02, 0xf7, 0x2c, 0x3d, 0xe8, 0xd9, 0x50, 0x62, 0xec, 0xbd, 0x53, 0x73, 0x79, 0x61,
    0x00, 0x34, 0xcf, 0xa2, 0x63, 0x28, 0x90, 0xaf, 0x44, 0x3b, 0xc5, 0x8d, 0x3a, 0x46, 0x07, 0x70,
    0x66, 0xbe, 0xd8, 0x8b, 0xe9, 0xa0, 0x4b, 0x98, 0xdc, 0xdf, 0xe2, 0x16, 0x74, 0xf1, 0x37, 0xf5,
    0xb7, 0x21, 0x81, 0x01, 0x1c, 0x1b, 0x94, 0x36, 0x09, 0xa1, 0x4a, 0x91, 0x30, 0x92, 0x9b, 0x9a,
    0x29, 0xb1, 0x38, 0x4d, 0x55, 0xf2, 0x56, 0x18, 0x24, 0x47, 0x9d, 0x3f, 0x80, 0x1f, 0x22, 0xa4,
    0x11, 0x54, 0x84, 0x0d, 0x25, 0x48, 0xee, 0xc6, 0x59, 0x15, 0x03, 0x7a, 0xfd, 0x6c, 0xc3, 0x33,
    0x5b, 0xc4, 0x7b, 0x5a, 0x05, 0x7f, 0xa6, 0x40, 0xa9, 0x5d, 0x41, 0x8a, 0x96, 0x52, 0xd3, 0xf0,
    0xab, 0x72, 0x10, 0x88, 0x6f, 0x95, 0x7c, 0xa8, 0xcd, 0x9c, 0x5f, 0x32, 0xae, 0x85, 0x39, 0xac,
    0xe5, 0xd7, 0xfb, 0xd4, 0x08, 0x23, 0x19, 0x65, 0x6b, 0xa7, 0x93, 0xbb, 0x2b, 0xbf, 0xb8, 0x35,
    0xd0, 0x06, 0x26, 0x68, 0x3e, 0xdd, 0xb9, 0x69, 0x2a, 0xb2, 0xde, 0x87, 0x45, 0x58, 0xff, 0x3c,
    0x9e, 0x7d, 0xda, 0xed, 0x49, 0x8c, 0x14, 0x8e, 0x75, 0x2f, 0xe0, 0x6e, 0x78, 0x6d, 0x20, 0xd2,
    0xfa, 0x2d, 0x51, 0xcc, 0xc7, 0xe7, 0x1d, 0x27, 0x97, 0xfc, 0x31, 0xdb, 0xf8, 0x42, 0xe3, 0x99,
    0x5e, 0x83, 0x0e, 0xb4, 0x2e, 0xf6, 0xc0, 0x0c, 0x4c, 0x57, 0xb6, 0x64, 0x0a, 0x17, 0xa3, 0xc1,
    0x77, 0x12, 0xfe, 0xe6, 0x8f, 0x13, 0x71, 0xe4, 0xf9, 0xad, 0x9f, 0xce, 0xd5, 0x89, 0x7e, 0x0f,
    0xc2, 0x86, 0xf3, 0x67, 0xba, 0x60, 0x43, 0xc9, 0x04, 0xb3, 0xb0, 0x1e, 0xb5, 0xc8, 0xeb, 0xa5,
    0x76, 0xea, 0x5c, 0x82, 0x1a, 0x4f, 0xaa, 0xca, 0xe1, 0x0b, 0x4e, 0xcb, 0x6a, 0xef, 0xd1, 0xd6,
];

const M1_ADDRESS_PERMUTATIONS: [[u8; 16]; 8] = [
    [15, 14, 10, 7, 1, 2, 3, 8, 0, 12, 11, 13, 6, 9, 5, 4],
    [7, 1, 8, 11, 15, 9, 2, 3, 5, 13, 4, 14, 10, 0, 6, 12],
    [8, 6, 14, 3, 10, 7, 15, 1, 4, 0, 2, 5, 13, 11, 12, 9],
    [2, 8, 15, 9, 3, 4, 11, 7, 13, 6, 0, 10, 1, 12, 14, 5],
    [1, 13, 6, 15, 14, 3, 8, 10, 9, 4, 7, 12, 5, 2, 0, 11],
    [11, 15, 3, 4, 7, 0, 9, 2, 6, 14, 12, 1, 8, 5, 10, 13],
    [10, 5, 13, 8, 6, 15, 1, 14, 11, 9, 3, 0, 12, 7, 4, 2],
    [9, 3, 7, 0, 2, 12, 4, 11, 14, 10, 5, 8, 15, 13, 1, 6],
];

pub fn decrypt_cmc50_m1(raw: &[u8]) -> Vec<u8> {
    let mut source = vec![0; CMC_M1_DECRYPTED_SIZE];
    let copy_len = raw.len().min(CMC_M1_DECRYPTED_SIZE);
    source[..copy_len].copy_from_slice(&raw[..copy_len]);

    let key = generate_cs16(&source[..CMC_M1_KEY_SIZE]);
    (0..CMC_M1_DECRYPTED_SIZE)
        .map(|address| source[m1_address_scramble(address, key)])
        .collect()
}

pub fn derive_cmc_gfx_extra_xor_from_m1(raw: &[u8]) -> Option<u8> {
    let mut source = vec![0; CMC_M1_DECRYPTED_SIZE];
    let copy_len = raw.len().min(CMC_M1_DECRYPTED_SIZE);
    source[..copy_len].copy_from_slice(&raw[..copy_len]);

    Some(cmc_gfx_xor_data(&source, 0x10, 0x04)? ^ cmc_gfx_xor_data(&source, 0x1a, 0x18)?)
}

pub fn interleave_cmc_graphics_banks(mut banks: Vec<(u8, Vec<u8>)>) -> Vec<u8> {
    banks = merge_split_graphics_banks(banks);
    banks.sort_by_key(|(index, _)| *index);
    let mut interleaved = Vec::new();
    let mut cursor = 0;

    while cursor < banks.len() {
        let (index, odd) = &banks[cursor];
        if index % 2 == 1 {
            if let Some((_, even)) = banks
                .get(cursor + 1)
                .filter(|(next_index, _)| *next_index == index + 1)
            {
                append_interleaved_pair(&mut interleaved, odd, even);
                cursor += 2;
                continue;
            }
        }

        interleaved.extend_from_slice(odd);
        cursor += 1;
    }

    interleaved
}

fn merge_split_graphics_banks(banks: Vec<(u8, Vec<u8>)>) -> Vec<(u8, Vec<u8>)> {
    let mut chunks_by_bank: BTreeMap<u8, Vec<Vec<u8>>> = BTreeMap::new();
    for (index, bytes) in banks {
        chunks_by_bank.entry(index).or_default().push(bytes);
    }

    chunks_by_bank
        .into_iter()
        .map(|(index, chunks)| {
            let total_len = chunks.iter().map(Vec::len).sum();
            let mut merged = Vec::with_capacity(total_len);
            for chunk in chunks {
                merged.extend_from_slice(&chunk);
            }
            (index, merged)
        })
        .collect()
}

pub fn decrypt_cmc50_graphics(raw: &[u8], extra_xor: u8) -> Vec<u8> {
    decrypt_cmc_graphics_with_tables(raw, extra_xor, &CMC50_TABLES)
}

pub fn decrypt_cmc42_graphics(raw: &[u8], extra_xor: u8) -> Vec<u8> {
    decrypt_cmc_graphics_with_tables(raw, extra_xor, &CMC42_TABLES)
}

fn decrypt_cmc_graphics_with_tables(raw: &[u8], extra_xor: u8, tables: &CmcGfxTables) -> Vec<u8> {
    let decrypt_len = raw.len().min(CMC_GFX_MAX_DECRYPT_SIZE);
    let mut decrypted = vec![0; raw.len()];
    let decrypt_bytes = decrypt_len - (decrypt_len % 4);
    let rom_words = decrypt_bytes / 4;

    for offset in (0..decrypt_bytes).step_by(CMC_GFX_BLOCK_SIZE) {
        let block_len = (decrypt_bytes - offset).min(CMC_GFX_BLOCK_SIZE);
        let block_words = block_len / 4;
        let offset_words = offset / 4;
        let mut block = raw[offset..offset + block_len].to_vec();

        for rpos in 0..block_words {
            cmc_xor(
                &mut block,
                4 * rpos,
                4 * rpos + 3,
                CmcXorTables {
                    table0hi: tables.type0_t03,
                    table0lo: tables.type0_t12,
                    table1: tables.type1_t03,
                },
                tables.addr_0_7_xor,
                rpos,
                ((rpos >> 8) & 1) != 0,
            );
            cmc_xor(
                &mut block,
                4 * rpos + 1,
                4 * rpos + 2,
                CmcXorTables {
                    table0hi: tables.type0_t12,
                    table0lo: tables.type0_t03,
                    table1: tables.type1_t12,
                },
                tables.addr_0_7_xor,
                rpos,
                ((((rpos + offset_words) >> 16)
                    ^ tables.addr_16_23_xor2[(rpos >> 8) & 0xff] as usize)
                    & 1)
                    != 0,
            );
        }

        for rpos in 0..block_words {
            let dst_word =
                cmc_fbneo_destination_word(rpos, offset_words, rom_words, extra_xor, tables);
            if dst_word < rom_words {
                let src = 4 * rpos;
                let dst = 4 * dst_word;
                decrypted[dst..dst + 4].copy_from_slice(&block[src..src + 4]);
            }
        }
    }

    if raw.len() > decrypt_bytes {
        decrypted[decrypt_bytes..].copy_from_slice(&raw[decrypt_bytes..]);
    }

    decrypted
}

pub fn extract_cmc_s_data(rom: &[u8], sdata_size: usize) -> Vec<u8> {
    let mut sdata = vec![0; sdata_size];
    if rom.is_empty() || sdata_size == 0 {
        return sdata;
    }

    if sdata_size == 0x100000 {
        let base = rom.len().saturating_sub(sdata_size / 2);
        for i in 0..sdata_size / 2 {
            let mapped = cmc_s_data_offset(i);
            let first = base as isize + mapped as isize - 0x1000000;
            if first >= 0 {
                if let Some(byte) = rom.get(first as usize) {
                    sdata[i] = *byte;
                }
            }
            if let Some(byte) = rom.get(base + mapped) {
                sdata[i + sdata_size / 2] = *byte;
            }
        }
    } else {
        let base = rom.len().saturating_sub(sdata_size);
        for (i, dst) in sdata.iter_mut().enumerate() {
            if let Some(byte) = rom.get(base + cmc_s_data_offset(i)) {
                *dst = *byte;
            }
        }
    }

    sdata
}

fn append_interleaved_pair(out: &mut Vec<u8>, odd: &[u8], even: &[u8]) {
    let pair_len = odd.len().min(even.len());
    out.reserve(
        pair_len * 2 + odd.len().saturating_sub(pair_len) + even.len().saturating_sub(pair_len),
    );
    for i in 0..pair_len {
        out.push(odd[i]);
        out.push(even[i]);
    }
    out.extend_from_slice(&odd[pair_len..]);
    out.extend_from_slice(&even[pair_len..]);
}

fn cmc_fbneo_destination_word(
    rpos: usize,
    offset_words: usize,
    rom_words: usize,
    extra_xor: u8,
    tables: &CmcGfxTables,
) -> usize {
    let clamp_size = greatest_power_of_two_le(rom_words);
    let global = rpos + offset_words;
    let mut baser = global;

    baser ^= tables.addr_0_7_xor[(baser >> 8) & 0xff] as usize;
    baser ^= (tables.addr_16_23_xor2[(baser >> 8) & 0xff] as usize) << 16;
    baser ^= (tables.addr_16_23_xor1[baser & 0xff] as usize) << 16;

    if global < clamp_size {
        baser &= clamp_size - 1;
    } else {
        baser = clamp_size + (baser & ((clamp_size >> 1) - 1));
    }

    baser ^= (tables.addr_8_15_xor2[baser & 0xff] as usize) << 8;
    baser ^= (tables.addr_8_15_xor1[(baser >> 16) & 0xff] as usize) << 8;
    baser ^ extra_xor as usize
}

fn greatest_power_of_two_le(value: usize) -> usize {
    if value == 0 {
        0
    } else {
        1usize << (usize::BITS - 1 - value.leading_zeros())
    }
}

fn cmc_xor(
    block: &mut [u8],
    idx0: usize,
    idx1: usize,
    tables: CmcXorTables,
    addr_0_7_xor: &[u8; 256],
    base: usize,
    invert: bool,
) {
    let tmp = tables.table1[(base & 0xff) ^ addr_0_7_xor[(base >> 8) & 0xff] as usize];
    let xor0 = (tables.table0hi[(base >> 8) & 0xff] & 0xfe) | (tmp & 0x01);
    let xor1 = (tmp & 0xfe) | (tables.table0lo[(base >> 8) & 0xff] & 0x01);
    let c0 = block[idx0];
    let c1 = block[idx1];

    if invert {
        block[idx0] = c1 ^ xor0;
        block[idx1] = c0 ^ xor1;
    } else {
        block[idx0] = c0 ^ xor0;
        block[idx1] = c1 ^ xor1;
    }
}

fn cmc_s_data_offset(i: usize) -> usize {
    (i & !0x1f) + ((i & 7) << 2) + ((!i & 8) >> 2) + ((i & 0x10) >> 4)
}

fn generate_cs16(bytes: &[u8]) -> u16 {
    bytes
        .iter()
        .fold(0u16, |sum, byte| sum.wrapping_add(*byte as u16))
}

fn m1_address_scramble(address: usize, key: u16) -> usize {
    let block = (address >> 16) & 7;
    let mut aux = (address & 0xffff) as u16;

    aux ^= bitswap16(key, &[12, 0, 2, 4, 8, 15, 7, 13, 10, 1, 3, 6, 11, 9, 14, 5]);
    aux = bitswap16_reversed(aux, &M1_ADDRESS_PERMUTATIONS[block]);
    aux ^= M1_ADDRESS_0_7_XOR[((aux >> 8) & 0xff) as usize] as u16;
    aux ^= (M1_ADDRESS_8_15_XOR[(aux & 0xff) as usize] as u16) << 8;
    aux = bitswap16(aux, &[7, 15, 14, 6, 5, 13, 12, 4, 11, 3, 10, 2, 9, 1, 8, 0]);

    (block << 16) | aux as usize
}

fn cmc_gfx_xor_data(source: &[u8], address_bits: usize, address_xor: usize) -> Option<u8> {
    let base = 0xf9a0;
    let address = u16::from_le_bytes([
        *source.get(base + address_bits)?,
        *source.get(base + address_bits + 1)?,
    ]) ^ u16::from_be_bytes([
        *source.get(base + address_xor)?,
        *source.get(base + address_xor + 1)?,
    ]);
    let address = ((address & 0x8000) >> 15) | ((address & 0x7fff) << 1);
    source.get(address as usize).copied()
}

fn bitswap16(value: u16, order: &[u8; 16]) -> u16 {
    order
        .iter()
        .fold(0u16, |out, bit| (out << 1) | ((value >> *bit) & 1))
}

fn bitswap16_reversed(value: u16, order: &[u8; 16]) -> u16 {
    order
        .iter()
        .rev()
        .fold(0u16, |out, bit| (out << 1) | ((value >> *bit) & 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_wraps_to_16_bits() {
        let bytes = vec![0xff; 0x101];
        assert_eq!(generate_cs16(&bytes), 0xffff);
    }

    #[test]
    fn m1_scramble_matches_kof2002_reference_key() {
        assert_eq!(m1_address_scramble(0, 0x95f3), 0x4e4a);
        assert_eq!(m1_address_scramble(1, 0x95f3), 0x4104);
        assert_eq!(m1_address_scramble(0x20000, 0x95f3), 0x20103);
    }

    #[test]
    fn decrypt_cmc50_m1_expands_to_hardware_buffer_size() {
        let raw: Vec<u8> = (0..0x20000).map(|i| i as u8).collect();
        let decrypted = decrypt_cmc50_m1(&raw);

        assert_eq!(decrypted.len(), CMC_M1_DECRYPTED_SIZE);
        assert_eq!(generate_cs16(&raw[..CMC_M1_KEY_SIZE]), 0x8000);
        assert_eq!(decrypted[0], raw[m1_address_scramble(0, 0x8000)]);
    }

    #[test]
    fn interleaves_cmc_graphics_banks_in_odd_even_pairs() {
        let interleaved = interleave_cmc_graphics_banks(vec![
            (2, vec![0x20, 0x21, 0x22]),
            (1, vec![0x10, 0x11]),
            (3, vec![0x30]),
        ]);

        assert_eq!(interleaved, vec![0x10, 0x20, 0x11, 0x21, 0x22, 0x30]);
    }

    #[test]
    fn interleaves_split_cmc_graphics_bank_chunks_before_pairing() {
        let interleaved = interleave_cmc_graphics_banks(vec![
            (1, vec![0x10, 0x11]),
            (1, vec![0x12, 0x13]),
            (2, vec![0x20, 0x21]),
            (2, vec![0x22, 0x23]),
        ]);

        assert_eq!(
            interleaved,
            vec![0x10, 0x20, 0x11, 0x21, 0x12, 0x22, 0x13, 0x23]
        );
    }

    #[test]
    fn extracts_cmc_s_data_from_c_rom_tail_layout() {
        let rom: Vec<u8> = (0..0x80).map(|i| i as u8).collect();
        let sdata = extract_cmc_s_data(&rom, 0x20);
        let expected: Vec<u8> = (0..0x20)
            .map(|i| rom[0x60 + cmc_s_data_offset(i)])
            .collect();

        assert_eq!(sdata, expected);
    }

    #[test]
    fn decrypt_cmc50_graphics_preserves_length_and_changes_data() {
        let raw: Vec<u8> = (0..0x1000).map(|i| i as u8).collect();
        let decrypted = decrypt_cmc50_graphics(&raw, CMC50_KOF2002_EXTRA_XOR);

        assert_eq!(decrypted.len(), raw.len());
        assert_ne!(decrypted, raw);
    }
}

//! SMA protection helpers for Metal Slug 3 (and potentially other SMA-protected
//! NeoGeo cartridges like kof99, garou, kof2000).
//!
//! The decryption algorithm is ported from MAME's `prot_sma.cpp`
//! (`mslug3_decrypt_68k`), originally credited to Razoola & Mr.K.

/// Bitswap for u16 using MAME/FBNeo-style maps.
///
/// `BITSWAP16(x, a, b, ..., p)` lists the source bits for output bits
/// 15 down to 0, not 0 up to 15.
fn bitswap16(value: u16, map: &[u8; 16]) -> u16 {
    let mut out = 0u16;
    for (i, &bit) in map.iter().enumerate() {
        out |= ((value >> bit) & 1) << (15 - i);
    }
    out
}

/// Bitswap for u16 with 15 meaningful output bits (14 down to 0).
fn bitswap15(value: u16, map: &[u8; 15]) -> u16 {
    let mut out = 0u16;
    for (i, &bit) in map.iter().enumerate() {
        out |= ((value >> bit) & 1) << (14 - i);
    }
    out
}

/// Bitswap for u32 with 19 meaningful output bits (18 down to 0).
fn bitswap19(value: u32, map: &[u8; 19]) -> u32 {
    let mut out = 0u32;
    for (i, &bit) in map.iter().enumerate() {
        out |= ((value >> bit) & 1) << (18 - i);
    }
    out
}

/// Bitswap for u32 with 24 meaningful output bits (23 down to 0).
/// Used by Garou's SMA decryption (BITSWAP24 from FBNeo).
fn bitswap24(value: u32, map: &[u8; 24]) -> u32 {
    let mut out = 0u32;
    for (i, &bit) in map.iter().enumerate() {
        out |= ((value >> bit) & 1) << (23 - i);
    }
    out
}

/// Decrypt the 68k program ROM for Metal Slug 3 (SMA protection).
///
/// This function transforms the raw P-ROM data in place so the NeoGeo 68000
/// sees the correct (decrypted) code and data.  It must be called *after*
/// the P-ROM banks have been concatenated and *before* the ROM is used for
/// emulation.
///
/// The algorithm has three stages:
/// 1. Data‑line swap on the full banked ROM region (offset ≥ 1 MB).
/// 2. Relocate the fixed region (first 768 KB) from a scrambled area inside
///    the bank region.
/// 3. Address‑line scramble inside each 64 KB chunk of the bank region.
///
/// Reference: MAME `src/devices/bus/neogeo/prot_sma.cpp` –
/// `sma_prot_device::mslug3_decrypt_68k`.
pub fn mslug3_decrypt_68k(prom: &mut [u8]) {
    let bank_start = 0x10_0000usize; // 1 MB – start of banked ROM in 68k space

    // ── 1. Data‑line swap on the banked region ──────────────────────────
    const DATA_MAP: [u8; 16] = [4, 11, 14, 3, 1, 13, 0, 7, 2, 8, 12, 15, 10, 9, 5, 6];

    let bank_word_count = prom
        .len()
        .saturating_sub(bank_start)
        .min(0x80_0000) // MAME processes up to 8 MB
        / 2;

    for word_idx in 0..bank_word_count {
        let offset = bank_start + word_idx * 2;
        let word = u16::from_be_bytes([prom[offset], prom[offset + 1]]);
        let swapped = bitswap16(word, &DATA_MAP);
        let [hi, lo] = swapped.to_be_bytes();
        prom[offset] = hi;
        prom[offset + 1] = lo;
    }

    // ── 2. Relocate the fixed region (first 768 KB) ────────────────────
    const FIXED_MAP: [u8; 19] = [
        18, 15, 2, 1, 13, 3, 0, 9, 6, 16, 4, 11, 5, 7, 12, 17, 14, 10, 8,
    ];

    let fixed_word_count = 0xC_0000 / 2; // 768 KB → 384 Kwords
    let source_base_word = 0x5D_0000 / 2; // word index where the fixed data lives
    let mut fixed_region = vec![0u8; 0xC_0000];

    for dest_word_idx in 0..fixed_word_count {
        let src_word_idx = source_base_word + bitswap19(dest_word_idx as u32, &FIXED_MAP) as usize;
        let src_offset = src_word_idx * 2;
        if src_offset + 1 < prom.len() {
            fixed_region[dest_word_idx * 2] = prom[src_offset];
            fixed_region[dest_word_idx * 2 + 1] = prom[src_offset + 1];
        }
    }

    let copy_len = fixed_region.len().min(prom.len());
    prom[..copy_len].copy_from_slice(&fixed_region[..copy_len]);

    // ── 3. Address‑line scramble inside 64 KB bank chunks ──────────────
    const BANK_MAP: [u8; 15] = [2, 11, 0, 14, 6, 4, 13, 8, 9, 3, 10, 7, 5, 12, 1];

    let max_bank = prom.len().saturating_sub(bank_start).min(0x80_0000);
    process_sma_banked_region(prom, bank_start, max_bank, &BANK_MAP);
}

/// Apply SMA address-line scramble inside 64 KB bank chunks.
/// Shared by all SMA-based games; the BANK_MAP differs per game.
fn process_sma_banked_region(
    prom: &mut [u8],
    bank_start: usize,
    max_bank: usize,
    bank_map: &[u8; 15],
) {
    let chunk_size = 0x1_0000usize; // 64 KB
    let mut chunk_buf = vec![0u8; chunk_size];

    for chunk_start in (bank_start..bank_start + max_bank).step_by(chunk_size) {
        let actual_chunk = (chunk_start + chunk_size).min(prom.len()) - chunk_start;
        chunk_buf[..actual_chunk].copy_from_slice(&prom[chunk_start..chunk_start + actual_chunk]);

        let chunk_words = actual_chunk / 2;
        for j in 0..chunk_words {
            let scrambled_idx = bitswap15(j as u16, bank_map) as usize * 2;
            prom[chunk_start + j * 2] = chunk_buf[scrambled_idx];
            prom[chunk_start + j * 2 + 1] = chunk_buf[scrambled_idx + 1];
        }
    }
}

/// Decrypt the 68k program ROM for The King of Fighters '99 (SMA + CMC42).
///
/// Reference: FBNeo `kof99SMADecrypt()`.
pub fn kof99_decrypt_68k(prom: &mut [u8]) {
    let bank_start = 0x10_0000usize;
    const BANK_SIZE: usize = 0x80_0000;

    const DATA_MAP: [u8; 16] = [13, 7, 3, 0, 9, 4, 5, 6, 1, 12, 8, 14, 10, 11, 2, 15];
    let bank_words = prom.len().saturating_sub(bank_start).min(BANK_SIZE) / 2;
    for word_idx in 0..bank_words {
        let offset = bank_start + word_idx * 2;
        let word = u16::from_be_bytes([prom[offset], prom[offset + 1]]);
        let [hi, lo] = bitswap16(word, &DATA_MAP).to_be_bytes();
        prom[offset] = hi;
        prom[offset + 1] = lo;
    }

    const FIXED_MAP: [u8; 24] = [
        23, 22, 21, 20, 19, 18, 11, 6, 14, 17, 16, 5, 8, 10, 12, 0, 4, 3, 2, 7, 9, 15, 13, 1,
    ];
    const SOURCE_BASE_WORD: usize = 0x700000 / 2;
    let fixed_word_count = 0xC_0000 / 2;
    let mut fixed_region = vec![0u8; 0xC_0000];
    for dest_word_idx in 0..fixed_word_count {
        let src_word_idx = SOURCE_BASE_WORD + bitswap24(dest_word_idx as u32, &FIXED_MAP) as usize;
        let src_offset = src_word_idx * 2;
        if src_offset + 1 < prom.len() {
            fixed_region[dest_word_idx * 2] = prom[src_offset];
            fixed_region[dest_word_idx * 2 + 1] = prom[src_offset + 1];
        }
    }
    let copy_len = fixed_region.len().min(prom.len());
    prom[..copy_len].copy_from_slice(&fixed_region[..copy_len]);

    const BANK_MAP: [u8; 24] = [
        23, 22, 21, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 6, 2, 4, 9, 8, 3, 1, 7, 0, 5,
    ];
    let max_bank = prom.len().saturating_sub(bank_start).min(0x60_0000);
    let chunk_size = 0x0800usize;
    let mut chunk_buf = vec![0u8; chunk_size];
    for chunk_start in (bank_start..bank_start + max_bank).step_by(chunk_size) {
        let actual_chunk = (chunk_start + chunk_size).min(prom.len()) - chunk_start;
        chunk_buf[..actual_chunk].copy_from_slice(&prom[chunk_start..chunk_start + actual_chunk]);
        let chunk_words = actual_chunk / 2;
        for j in 0..chunk_words {
            let scrambled_idx = bitswap24(j as u32, &BANK_MAP) as usize * 2;
            prom[chunk_start + j * 2] = chunk_buf[scrambled_idx];
            prom[chunk_start + j * 2 + 1] = chunk_buf[scrambled_idx + 1];
        }
    }
}

/// Decrypt the 68k program ROM for The King of Fighters 2000 (SMA + CMC42).
///
/// KOF 2000 uses SMA protection for P-ROM + CMC42 for C-ROM / fix layer.
/// The bit-swap tables below are verified from FBNeo's `kof2000SMADecrypt()`
/// in `src/burn/drv/neogeo/d_neogeo.cpp`.
///
/// Key differences from other SMA games:
/// - DATA_MAP: BITSWAP16 with KOF2000-specific map
/// - FIXED_MAP: BITSWAP24, source_base = 0x73A000/2
/// - BANK_MAP: BITSWAP24, processes 0x63A000 bytes in 2KB chunks (not 64KB)
pub fn kof2000_decrypt_68k(prom: &mut [u8]) {
    let bank_start = 0x10_0000usize;

    // ── 1. Data‑line swap on the banked region ──────────────────────────
    const DATA_MAP: [u8; 16] = [12, 8, 11, 3, 15, 14, 7, 0, 10, 13, 6, 5, 9, 2, 1, 4];

    let bank_words = prom.len().saturating_sub(bank_start).min(0x80_0000) / 2;
    for word_idx in 0..bank_words {
        let offset = bank_start + word_idx * 2;
        let word = u16::from_be_bytes([prom[offset], prom[offset + 1]]);
        let swapped = bitswap16(word, &DATA_MAP);
        let [hi, lo] = swapped.to_be_bytes();
        prom[offset] = hi;
        prom[offset + 1] = lo;
    }

    // ── 2. Relocate the fixed region (first 768 KB) ────────────────────
    const FIXED_MAP: [u8; 24] = [
        23, 22, 21, 20, 19, 18, 8, 4, 15, 13, 3, 14, 16, 2, 6, 17, 7, 12, 10, 0, 5, 11, 1, 9,
    ];
    const SOURCE_BASE_WORD: usize = 0x73A000 / 2;

    let fixed_word_count = 0xC_0000 / 2;
    let mut fixed_region = vec![0u8; 0xC_0000];

    for dest_word_idx in 0..fixed_word_count {
        let src_word_idx = SOURCE_BASE_WORD + bitswap24(dest_word_idx as u32, &FIXED_MAP) as usize;
        let src_offset = src_word_idx * 2;
        if src_offset + 1 < prom.len() {
            fixed_region[dest_word_idx * 2] = prom[src_offset];
            fixed_region[dest_word_idx * 2 + 1] = prom[src_offset + 1];
        }
    }

    let copy_len = fixed_region.len().min(prom.len());
    prom[..copy_len].copy_from_slice(&fixed_region[..copy_len]);

    // ── 3. Address‑line scramble inside 2 KB chunks ────────────────────
    // KOF2000 uses 2KB chunks (0x800 bytes = 0x400 words), unlike mslug3's 64KB
    const BANK_MAP: [u8; 24] = [
        23, 22, 21, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 4, 1, 3, 8, 6, 2, 7, 0, 9, 5,
    ];

    let max_bank = prom.len().saturating_sub(bank_start).min(0x63A000);
    let chunk_size = 0x0800usize; // 2 KB (unique to KOF2000)
    let mut chunk_buf = vec![0u8; chunk_size];

    for chunk_start in (bank_start..bank_start + max_bank).step_by(chunk_size) {
        let actual_chunk = (chunk_start + chunk_size).min(prom.len()) - chunk_start;
        chunk_buf[..actual_chunk].copy_from_slice(&prom[chunk_start..chunk_start + actual_chunk]);

        let chunk_words = actual_chunk / 2;
        for j in 0..chunk_words {
            let scrambled_idx = bitswap24(j as u32, &BANK_MAP) as usize * 2;
            prom[chunk_start + j * 2] = chunk_buf[scrambled_idx];
            prom[chunk_start + j * 2 + 1] = chunk_buf[scrambled_idx + 1];
        }
    }
}

/// Decrypt the 68k program ROM for Garou: Mark of the Wolves (SMA + CMC50).
///
/// Garou uses SMA protection for P-ROM + CMC50 for C-ROM / fix layer.
/// The bit-swap tables below are verified from FBNeo's `garouSMADecrypt()`
/// in `src/burn/drv/neogeo/d_neogeo.cpp`.
///
/// Key differences from Mslug3 SMA:
/// - Different DATA_MAP (BITSWAP16)
/// - Different FIXED_MAP (BITSWAP24 instead of BITSWAP19)
/// - Different BANK_MAP (BITSWAP24 instead of BITSWAP15)
/// - Source base for fixed region: 0x710000/2 (vs 0x5D0000/2)
pub fn garou_decrypt_68k(prom: &mut [u8]) {
    let bank_start = 0x10_0000usize;
    const BANK_SIZE: usize = 0x80_0000;

    // ── 1. Data‑line swap on the banked region ──────────────────────────
    const DATA_MAP: [u8; 16] = [13, 12, 14, 10, 8, 2, 3, 1, 5, 9, 11, 4, 15, 0, 6, 7];

    let bank_words = prom.len().saturating_sub(bank_start).min(BANK_SIZE) / 2;
    for word_idx in 0..bank_words {
        let offset = bank_start + word_idx * 2;
        let word = u16::from_be_bytes([prom[offset], prom[offset + 1]]);
        let swapped = bitswap16(word, &DATA_MAP);
        let [hi, lo] = swapped.to_be_bytes();
        prom[offset] = hi;
        prom[offset + 1] = lo;
    }

    // ── 2. Relocate the fixed region (first 768 KB) ────────────────────
    const FIXED_MAP: [u8; 24] = [
        23, 22, 21, 20, 19, 18, 4, 5, 16, 14, 7, 9, 6, 13, 17, 15, 3, 1, 2, 12, 11, 8, 10, 0,
    ];
    const SOURCE_BASE_WORD: usize = 0x710000 / 2;

    let fixed_word_count = 0xC_0000 / 2;
    let mut fixed_region = vec![0u8; 0xC_0000];

    for dest_word_idx in 0..fixed_word_count {
        let src_word_idx = SOURCE_BASE_WORD + bitswap24(dest_word_idx as u32, &FIXED_MAP) as usize;
        let src_offset = src_word_idx * 2;
        if src_offset + 1 < prom.len() {
            fixed_region[dest_word_idx * 2] = prom[src_offset];
            fixed_region[dest_word_idx * 2 + 1] = prom[src_offset + 1];
        }
    }

    let copy_len = fixed_region.len().min(prom.len());
    prom[..copy_len].copy_from_slice(&fixed_region[..copy_len]);

    // ── 3. Address‑line scramble inside 64 KB bank chunks ──────────────
    const BANK_MAP: [u8; 24] = [
        23, 22, 21, 20, 19, 18, 17, 16, 15, 14, 9, 4, 8, 3, 13, 6, 2, 7, 0, 12, 1, 11, 10, 5,
    ];

    let max_bank = prom.len().saturating_sub(bank_start).min(BANK_SIZE);
    let chunk_size = 0x1_0000usize; // 64 KB
    let mut chunk_buf = vec![0u8; chunk_size];

    for chunk_start in (bank_start..bank_start + max_bank).step_by(chunk_size) {
        let actual_chunk = (chunk_start + chunk_size).min(prom.len()) - chunk_start;
        chunk_buf[..actual_chunk].copy_from_slice(&prom[chunk_start..chunk_start + actual_chunk]);

        let chunk_words = actual_chunk / 2;
        for j in 0..chunk_words {
            let scrambled_idx = bitswap24(j as u32, &BANK_MAP) as usize * 2;
            prom[chunk_start + j * 2] = chunk_buf[scrambled_idx];
            prom[chunk_start + j * 2 + 1] = chunk_buf[scrambled_idx + 1];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitswap16_matches_first_word_of_mslug3_data_map() {
        let input: u16 = 0b1100_0000_0000_0001;
        let map: [u8; 16] = [4, 11, 14, 3, 1, 13, 0, 7, 2, 8, 12, 15, 10, 9, 5, 6];
        let result = bitswap16(input, &map);
        assert_eq!(result, 0x2210);
    }

    #[test]
    fn bitswap15_zero_input_yields_zero() {
        assert_eq!(
            bitswap15(0, &[2, 11, 0, 14, 6, 4, 13, 8, 9, 3, 10, 7, 5, 12, 1]),
            0
        );
    }

    #[test]
    fn bitswap19_zero_input_yields_zero() {
        assert_eq!(
            bitswap19(
                0,
                &[18, 15, 2, 1, 13, 3, 0, 9, 6, 16, 4, 11, 5, 7, 12, 17, 14, 10, 8]
            ),
            0
        );
    }

    #[test]
    fn mslug3_decrypt_preserves_length() {
        let mut prom = vec![0xFFu8; 0x90_0000]; // 9 MB
        let len_before = prom.len();
        mslug3_decrypt_68k(&mut prom);
        assert_eq!(prom.len(), len_before);
    }

    #[test]
    fn mslug3_decrypt_changes_data() {
        // Use 6 MB so the fixed-region source at 0x5D_0000 falls within bounds.
        // Real mslug3 ROMs are 8-9 MB; this tests the common path.
        let mut prom = vec![0u8; 0x60_0000];
        // Fill with non-zero pattern so we can detect change
        for (i, byte) in prom.iter_mut().enumerate() {
            *byte = (i & 0xFF) as u8;
        }
        let original = prom.clone();
        mslug3_decrypt_68k(&mut prom);
        // The first 768 KB (fixed region) should have changed after relocation
        assert_ne!(&prom[..0xC_0000], &original[..0xC_0000]);
        // Ensure it wasn't just zeroed out (verifies actual relocation happened)
        assert!(!prom[..0xC_0000].iter().all(|&b| b == 0));
    }

    #[test]
    fn kof99_decrypt_preserves_length_and_relocates_fixed_region() {
        let mut prom: Vec<u8> = (0..0x90_0000).map(|i| i as u8).collect();
        let original = prom.clone();

        kof99_decrypt_68k(&mut prom);

        assert_eq!(prom.len(), original.len());
        assert_ne!(&prom[..0xC_0000], &original[..0xC_0000]);
    }
}

//! PCM2 protection helpers for late NeoGeo cartridges.

const PCM2_P_FIXED_SIZE: usize = 0x100000;
const PCM2_P_SCRAMBLED_SIZE: usize = 0x400000;
const PCM2_P_CHUNK_SIZE: usize = 0x080000;
const PCM2_P_TOTAL_SIZE: usize = PCM2_P_FIXED_SIZE + PCM2_P_SCRAMBLED_SIZE;
const PCM2_P2_TOTAL_SIZE: usize = 0x800000;
const PCM2_P2_CHUNK_SIZE: usize = 0x080000;
const PCM2_V2_SIZE: usize = 0x1000000;

#[derive(Debug, Clone, Copy)]
pub struct Pcm2V2Info {
    pub address_offset: usize,
    pub address_xor: usize,
    pub data_xor: [u8; 8],
}

#[derive(Debug, Clone, Copy)]
pub struct Pcm2P2Info {
    pub address_offsets: [usize; 16],
}

pub const KOF2002_PCM2_V2_INFO: Pcm2V2Info = Pcm2V2Info {
    address_offset: 0x0a5000,
    address_xor: 0x000000,
    data_xor: [0xf9, 0xe0, 0x5d, 0xf3, 0xea, 0x92, 0xbe, 0xef],
};

// PCM2 info for KOF2003 — verified from FBNeo d_neogeo.cpp (kof2003Init)
pub const KOF2003_PCM2_V2_INFO: Pcm2V2Info = Pcm2V2Info {
    address_offset: 0x0a7001,
    address_xor: 0xff14ea,
    data_xor: [0x4b, 0xa4, 0x63, 0x46, 0xf0, 0x91, 0xea, 0x62],
};

// PCM2 info for SVC Chaos — verified from FBNeo d_neogeo.cpp (svcpcbInit)
pub const SVC_PCM2_V2_INFO: Pcm2V2Info = Pcm2V2Info {
    address_offset: 0x0c2000,
    address_xor: 0xffac28,
    data_xor: [0xc3, 0xfd, 0x81, 0xac, 0x6d, 0xe7, 0xbf, 0x9e],
};

// PCM2 info for Metal Slug 5 — verified from FBNeo d_neogeo.cpp (mslug5Init)
pub const MSLUG5_PCM2_V2_INFO: Pcm2V2Info = Pcm2V2Info {
    address_offset: 0x04e001,
    address_xor: 0xfe2cf6,
    data_xor: [0xc3, 0xfd, 0x81, 0xac, 0x6d, 0xe7, 0xbf, 0x9e],
};

// PCM2 info for Matrimelee — verified from FBNeo d_neogeo.cpp (matrimInit)
pub const MATRIM_PCM2_V2_INFO: Pcm2V2Info = Pcm2V2Info {
    address_offset: 0x001000,
    address_xor: 0xffce20,
    data_xor: [0xc4, 0x83, 0xa8, 0x5f, 0x21, 0x27, 0x64, 0xaf],
};

// PCM2 P/V info for Samurai Shodown V — verified from FBNeo d_neogeo.cpp (samsho5Init)
pub const SAMSHO5_PCM2_P2_INFO: Pcm2P2Info = Pcm2P2Info {
    address_offsets: [
        0x000000, 0x080000, 0x700000, 0x680000, 0x500000, 0x180000, 0x200000, 0x480000, 0x300000,
        0x780000, 0x600000, 0x280000, 0x100000, 0x580000, 0x400000, 0x380000,
    ],
};

pub const SAMSHO5_PCM2_V2_INFO: Pcm2V2Info = Pcm2V2Info {
    address_offset: 0x00a000,
    address_xor: 0xfeb2c0,
    data_xor: [0xcb, 0x29, 0x7d, 0x43, 0xd2, 0x3a, 0xc2, 0xb4],
};

// PCM2 P/V info for Samurai Shodown V Special — verified from FBNeo d_neogeo.cpp (samsh5spInit)
pub const SAMSH5SP_PCM2_P2_INFO: Pcm2P2Info = Pcm2P2Info {
    address_offsets: [
        0x000000, 0x080000, 0x500000, 0x480000, 0x600000, 0x580000, 0x700000, 0x280000, 0x100000,
        0x680000, 0x400000, 0x780000, 0x200000, 0x380000, 0x300000, 0x180000,
    ],
};

pub const SAMSH5SP_PCM2_V2_INFO: Pcm2V2Info = Pcm2V2Info {
    address_offset: 0x002000,
    address_xor: 0xffb440,
    data_xor: [0x4b, 0xa4, 0x63, 0x46, 0xf0, 0x91, 0xea, 0x62],
};

pub fn decrypt_pcm2_p(prom: &mut [u8]) {
    if prom.len() < PCM2_P_TOTAL_SIZE {
        return;
    }

    let scrambled = prom[PCM2_P_FIXED_SIZE..PCM2_P_TOTAL_SIZE].to_vec();
    for i in 0..4 {
        let dst_a = PCM2_P_FIXED_SIZE + i * 0x100000;
        let src_a = ((((i + 2) & 1) << 2) | ((i + 2) & 2)) << 19;
        prom[dst_a..dst_a + PCM2_P_CHUNK_SIZE]
            .copy_from_slice(&scrambled[src_a..src_a + PCM2_P_CHUNK_SIZE]);

        let dst_b = dst_a + PCM2_P_CHUNK_SIZE;
        let src_b = PCM2_P_CHUNK_SIZE + (((((i + 1) & 1) << 2) | ((i + 1) & 2)) << 19);
        prom[dst_b..dst_b + PCM2_P_CHUNK_SIZE]
            .copy_from_slice(&scrambled[src_b..src_b + PCM2_P_CHUNK_SIZE]);
    }
}

pub fn decrypt_pcm2_p2(prom: &mut [u8], info: Pcm2P2Info) {
    if prom.len() < PCM2_P2_TOTAL_SIZE {
        return;
    }

    let scrambled = prom[..PCM2_P2_TOTAL_SIZE].to_vec();
    for (chunk, source_offset) in info.address_offsets.iter().copied().enumerate() {
        let dst = chunk * PCM2_P2_CHUNK_SIZE;
        prom[dst..dst + PCM2_P2_CHUNK_SIZE]
            .copy_from_slice(&scrambled[source_offset..source_offset + PCM2_P2_CHUNK_SIZE]);
    }
}

pub fn decrypt_pcm2_v2(vrom: &mut [u8], info: Pcm2V2Info) {
    if vrom.len() < PCM2_V2_SIZE {
        return;
    }

    let scrambled = vrom[..PCM2_V2_SIZE].to_vec();
    for i in 0..PCM2_V2_SIZE {
        let address =
            ((i & 0x00fe_fffe) | ((i & 0x0001_0000) >> 16) | ((i & 1) << 16)) ^ info.address_offset;
        vrom[address] =
            scrambled[(i + info.address_xor) & 0x00ff_ffff] ^ info.data_xor[address & 7];
    }
}

pub fn decrypt_pcm2_v(vrom: &mut [u8], size: usize, bit: u8) {
    if size == 0 || vrom.len() < size {
        return;
    }

    let group_words = 2usize << bit;
    let swap_word = 1usize << bit;
    let total_words = size / 2;
    if total_words <= group_words {
        return;
    }

    for word_index in (0..total_words - group_words).step_by(group_words) {
        let byte_start = word_index * 2;
        let byte_len = group_words * 2;
        let buffer = vrom[byte_start..byte_start + byte_len].to_vec();
        for j in (0..group_words).rev() {
            let src_word = j ^ swap_word;
            let dst = byte_start + j * 2;
            let src = src_word * 2;
            vrom[dst..dst + 2].copy_from_slice(&buffer[src..src + 2]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decrypt_pcm2_p_reorders_kof2002_512k_chunks() {
        let mut prom = vec![0; PCM2_P_TOTAL_SIZE];
        prom[..PCM2_P_FIXED_SIZE].fill(0xee);
        for chunk in 0..8 {
            let start = PCM2_P_FIXED_SIZE + chunk * PCM2_P_CHUNK_SIZE;
            prom[start..start + PCM2_P_CHUNK_SIZE].fill(chunk as u8);
        }

        decrypt_pcm2_p(&mut prom);

        assert_eq!(prom[0], 0xee);
        let chunks: Vec<u8> = (0..8)
            .map(|chunk| prom[PCM2_P_FIXED_SIZE + chunk * PCM2_P_CHUNK_SIZE])
            .collect();
        assert_eq!(chunks, [2, 5, 6, 3, 0, 7, 4, 1]);
    }

    #[test]
    fn decrypt_pcm2_p_ignores_short_buffers() {
        let mut prom = vec![0xaa; PCM2_P_TOTAL_SIZE - 1];
        decrypt_pcm2_p(&mut prom);
        assert!(prom.iter().all(|byte| *byte == 0xaa));
    }

    #[test]
    fn decrypt_pcm2_p2_reorders_samsho5_512k_chunks() {
        let mut prom = vec![0; PCM2_P2_TOTAL_SIZE];
        for chunk in 0..16 {
            let start = chunk * PCM2_P2_CHUNK_SIZE;
            prom[start..start + PCM2_P2_CHUNK_SIZE].fill(chunk as u8);
        }

        decrypt_pcm2_p2(&mut prom, SAMSHO5_PCM2_P2_INFO);

        let chunks: Vec<u8> = (0..16)
            .map(|chunk| prom[chunk * PCM2_P2_CHUNK_SIZE])
            .collect();
        assert_eq!(
            chunks,
            [0, 1, 14, 13, 10, 3, 4, 9, 6, 15, 12, 5, 2, 11, 8, 7]
        );
    }

    #[test]
    fn decrypt_pcm2_p2_ignores_short_buffers() {
        let mut prom = vec![0xaa; PCM2_P2_TOTAL_SIZE - 1];
        decrypt_pcm2_p2(&mut prom, SAMSH5SP_PCM2_P2_INFO);
        assert!(prom.iter().all(|byte| *byte == 0xaa));
    }

    #[test]
    fn decrypt_pcm2_v2_applies_kof2002_address_and_data_xor() {
        let mut vrom: Vec<u8> = (0..PCM2_V2_SIZE).map(|i| i as u8).collect();

        decrypt_pcm2_v2(&mut vrom, KOF2002_PCM2_V2_INFO);

        assert_eq!(vrom[0x0a5000], 0xf9, "KOF2002 vrom[0x0a5000]");
        assert_eq!(vrom[0x0b5000], 0xf8, "KOF2002 vrom[0x0b5000]");
    }

    #[test]
    fn decrypt_pcm2_v2_kof2003_differs_from_kof2002() {
        let mut vrom_kof2003: Vec<u8> = (0..PCM2_V2_SIZE).map(|i| i as u8).collect();
        let mut vrom_kof2002 = vrom_kof2003.clone();

        decrypt_pcm2_v2(&mut vrom_kof2003, KOF2003_PCM2_V2_INFO);
        decrypt_pcm2_v2(&mut vrom_kof2002, KOF2002_PCM2_V2_INFO);

        // Both transform the data, but KOF2003 uses different parameters than KOF2002
        assert_ne!(
            vrom_kof2003, vrom_kof2002,
            "KOF2003 and KOF2002 PCM2 V2 should use different parameters"
        );
    }

    #[test]
    fn decrypt_pcm2_v2_svc_differs_from_kof2002() {
        let mut vrom_svc: Vec<u8> = (0..PCM2_V2_SIZE).map(|i| i as u8).collect();
        let mut vrom_kof2002 = vrom_svc.clone();

        decrypt_pcm2_v2(&mut vrom_svc, SVC_PCM2_V2_INFO);
        decrypt_pcm2_v2(&mut vrom_kof2002, KOF2002_PCM2_V2_INFO);

        assert_ne!(
            vrom_svc, vrom_kof2002,
            "SVC and KOF2002 PCM2 V2 should use different parameters"
        );
    }

    #[test]
    fn decrypt_pcm2_v2_all_sets_transform_data() {
        for info in [
            KOF2002_PCM2_V2_INFO,
            KOF2003_PCM2_V2_INFO,
            SVC_PCM2_V2_INFO,
            MSLUG5_PCM2_V2_INFO,
            MATRIM_PCM2_V2_INFO,
            SAMSHO5_PCM2_V2_INFO,
            SAMSH5SP_PCM2_V2_INFO,
        ] {
            let mut vrom: Vec<u8> = (0..PCM2_V2_SIZE).map(|i| i as u8).collect();
            let original = vrom.clone();
            decrypt_pcm2_v2(&mut vrom, info);
            assert_ne!(
                vrom, original,
                "PCM2 V2 decryption should transform data for all sets"
            );
        }
    }

    #[test]
    fn decrypt_pcm2_v2_ignores_short_buffers() {
        let mut vrom = vec![0xaa; PCM2_V2_SIZE - 1];
        decrypt_pcm2_v2(&mut vrom, KOF2002_PCM2_V2_INFO);
        assert!(vrom.iter().all(|byte| *byte == 0xaa));
    }

    #[test]
    fn decrypt_pcm2_v_swaps_word_groups_like_fbneo() {
        let mut vrom = Vec::new();
        for word in 0u16..16 {
            vrom.extend_from_slice(&word.to_be_bytes());
        }

        decrypt_pcm2_v(&mut vrom, 32, 2);

        let words: Vec<u16> = vrom
            .chunks_exact(2)
            .map(|bytes| u16::from_be_bytes([bytes[0], bytes[1]]))
            .collect();
        assert_eq!(
            &words[..8],
            &[4, 5, 6, 7, 0, 1, 2, 3],
            "bit=2 swaps the lower and upper 4-word halves inside each 8-word group"
        );
    }

    #[test]
    fn decrypt_pcm2_v_ignores_short_buffers() {
        let mut vrom = vec![0xaa; 0x1000 - 1];
        decrypt_pcm2_v(&mut vrom, 0x1000, 0);
        assert!(vrom.iter().all(|byte| *byte == 0xaa));
    }
}

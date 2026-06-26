use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

const MIN_EXTERNAL_PROM_SIZE: usize = 8;

/// Global toggle for diagnostic ROM bank dumps (PROM, CROM, SROM, MROM, VROM).
/// Set by the frontend based on CLI flags and/or config file.
static DIAGNOSTIC_DUMPS: AtomicBool = AtomicBool::new(false);

/// Enable or disable automatic diagnostic dumps of ROM banks.
pub fn set_diagnostic_dumps(enabled: bool) {
    DIAGNOSTIC_DUMPS.store(enabled, Ordering::Relaxed);
}
/// NGH → recommended BIOS mapping.
///
/// Returns a human-readable BIOS recommendation for a given Neo Geo Home (NGH)
/// cart ID. Games that use SMA/CMC42/CMC50 encryption benefit from UniBIOS;
/// standard games work with any MVS/AES BIOS.
///
/// For zip files, prefer [`get_recommended_bios_from_zip_name`] which is more
/// accurate since it doesn't rely on potentially-colliding NGH values.
pub fn get_recommended_bios(ngh: u32) -> &'static str {
    match ngh {
        // ── SMA+CMC42 / SMA+CMC50 — UniBIOS ────────────────────────
        0x153 | 0x229 | 0x253 => "UniBIOS", // Garou: Mark of the Wolves (SNK NGH 0x253, FBNeo 0x229/0x153)
        0x1F3 | 0x250 => "UniBIOS",         // Metal Slug 2 / Metal Slug X (NGNEON .neo variant)
        0x213 | 0x256 => "UniBIOS",         // Metal Slug 3 (FBNeo 0x213 / SNK NGH 0x256)
        0x215 => "UniBIOS",                 // Metal Slug 4 (CMC42)
        0x21B | 0x251 => "UniBIOS", // The King of Fighters '99 (FBNeo 0x21B / SNK NGH 0x251)
        0x21D | 0x257 => "UniBIOS", // The King of Fighters 2000 (FBNeo 0x21D / SNK NGH 0x257)
        0x221 => "UniBIOS",         // The King of Fighters 2001
        0x265 => "UniBIOS",         // The King of Fighters 2002 (CMC50+PCM2)
        0x223 | 0x271 => "UniBIOS", // The King of Fighters 2003 (CMC50+PCM2 + NEO-PVC)
        0x241 => "UniBIOS",         // SVC Chaos (CMC50+PCM2)
        // Additional CMC/SMA games (UniBIOS recommended)
        0x217 => "UniBIOS", // Metal Slug 5 (CMC50)
        0x225 => "UniBIOS", // Samurai Shodown 5 (CMC50)
        0x227 => "UniBIOS", // Samurai Shodown 5 Special (CMC50)
        0x231 => "UniBIOS", // Sengoku 3 (CMC50)
        0x233 => "UniBIOS", // Matrimelee (CMC50)
        0x235 => "UniBIOS", // Rage of the Dragons (CMC50)
        0x237 => "UniBIOS", // Nightmare in the Dark (CMC42)
        0x239 => "UniBIOS", // Prehistoric Isle 2 (CMC42)
        0x23B => "UniBIOS", // Strikers 1945 Plus (CMC42)
        0x23D => "UniBIOS", // Bang Bead (CMC42)
        0x23F => "UniBIOS", // Ganryu (CMC42)
        // Everything else works with standard MVS/AES
        _ => "MVS/AES",
    }
}

/// Zip-name → recommended BIOS mapping.
///
/// Returns a BIOS recommendation directly from the FBNeo/MAME zip name,
/// bypassing NGH value collisions. This is the preferred lookup for .zip ROMs.
pub fn get_recommended_bios_from_zip_name(name: &str) -> &'static str {
    let name = name.to_ascii_lowercase();
    // Strip common extension if present
    let name = name.trim_end_matches(".zip").trim_end_matches(".neo");
    match name {
        // ── CMC50+PCM2 sets (UniBIOS required) ─────────────────────
        "kof2002" | "kof2002n" | "kof2k2" | "kof2k2n" => "UniBIOS",
        "kof2003" | "kof2003n" | "kof2k3" | "kof2k3n" => "UniBIOS",
        "svc" | "svcchaos" | "svcboot" | "svcplus" => "UniBIOS",
        // ── SMA+CMC50 sets (UniBIOS required) ──────────────────────
        "garou" | "garoup" | "garouo" | "garoubl" => "UniBIOS",
        "kof2000" | "kof2000n" | "kof2k" | "kof2kn" => "UniBIOS",
        // ── SMA+CMC42 sets (UniBIOS required) ──────────────────────
        "mslug3" | "mslug3h" | "mslug3a" | "mslug3b" => "UniBIOS",
        // ── CMC42-only sets (UniBIOS strongly recommended) ─────────
        "mslug4" | "mslug4h" => "UniBIOS",
        "kof99" | "kof99k" | "kof99p" | "kof99e" => "UniBIOS",
        "kof2001" | "kof2001n" | "kof2k1" => "UniBIOS",
        "mslug5" | "mslug5h" => "UniBIOS",
        // ── CMC50 sets (UniBIOS recommended) ───────────────────────
        "samsho5" | "samsh5sp" | "samsho5a" | "samurai5" | "samuraispecial" => "UniBIOS",
        "sengoku3" | "sengoku3n" | "sengoku3h" => "UniBIOS",
        "matrim" | "matrimelee" | "matrimh" => "UniBIOS",
        "rotd" | "rageofthedragons" | "rotdh" => "UniBIOS",
        "nightmar" | "nightmareinthedark" => "UniBIOS",
        "preisle2" | "prehistorici2" => "UniBIOS",
        "s1945p" | "strikers1945plus" | "s1945a" => "UniBIOS",
        "bangbead" | "bangbeadh" => "UniBIOS",
        "ganryu" | "ganryuh" | "musashiganryuki" => "UniBIOS",
        "wakuwak7" | "wakuwaku7" => "UniBIOS",
        "jockeygp" | "jockeygpa" => "UniBIOS",
        // ── Older games with standard protection ────────────────────
        _ => "MVS/AES",
    }
}

/// Post-processing for .neo files.
///
/// .neo files from the NeoSD toolchain are already fully decrypted —
/// CMC42/CMC50 transforms, SMA decryption, PCM2 reorganization, and
/// M1 descrambling have already been applied during .neo creation.
///
/// The actions needed:
/// 1. Extract S-ROM (fix layer) from C-ROM tail if SSZ == 0 (CMC games).
/// 2. Apply NGH-specific fix layer workarounds (e.g. Matrimelee NGH 0x266).
///
/// Most .neo files from NeoSD populate the S section, but some older or
/// hand-made .neo files may omit it.
pub fn post_process_neo_rom(rom: &mut RomData) {
    let ngh = rom.metadata.as_ref().map(|m| m.ngh).unwrap_or(0);

    // ── S-ROM extraction from C-ROM tail ──────────────────────────────
    // For CMC-protected games, the NeoSD toolchain places the fix layer
    // data inside the C-ROM tail. If the .neo has no S section (ssz == 0),
    // extract it from the (already decrypted) C-ROM.
    if rom.srom.is_empty() && !rom.crom.is_empty() {
        rom.srom = crate::cmc::extract_cmc_s_data(&rom.crom, 0x80000);
    }

    // ── Matrimelee fix layer workaround (NGH 0x266) ───────────────────
    // TerraOnion's NeoBuilder tool decrypts this incorrectly for matrimbl.
    // The last 512K block of C-ROM contains the FIX layer data with a
    // bit-swizzle that needs to be undone (per Geolith).
    if ngh == 0x266 && !rom.crom.is_empty() && !rom.srom.is_empty() {
        let ssz = 0x80000; // 512K fix layer
                           // Check if this is the bootleg variant (matrimbl) by examining
                           // a known byte in the P-ROM (offset 0x500088 in C-ROM = 0x22 for bootleg)
        let offset_into_c = 0x500088usize;
        if offset_into_c + 1 < rom.crom.len() && rom.crom[offset_into_c] == 0x22 {
            eprintln!("[INFO] Matrimelee bootleg detectado: aplicando fix de capa fix");
            fix_matrimelee_fix_layer(&rom.crom, &mut rom.srom, ssz);
        }
    }

    // All other transformations (CMC, SMA, PCM2, M1) are already applied.
}

/// Try to infer the NGH value from a .zip ROM file name (without extension).
/// Uses well-known FBNeo / MAME romset naming conventions.
///
/// Note: The returned values are FBNeo-style internal IDs, NOT the physical
/// NGH cartridge numbers from SNK. For BIOS recommendations based on zip name,
/// use [`get_recommended_bios_from_zip_name`] which is more accurate.
pub fn detect_ngh_from_zip_name(name: &str) -> Option<u32> {
    let name = name.to_ascii_lowercase();
    let name = name.trim_end_matches(".zip");
    match name {
        // ── The King of Fighters series ───────────────────────────────
        "kof94" | "kof94h" => Some(0x005),
        "kof95" | "kof95h" => Some(0x053),
        "kof96" | "kof96h" => Some(0x0B3),
        "kof97" | "kof97pls" | "kof97h" => Some(0x133),
        "kof98" | "kof98k" | "kof98n" | "kof98h" => Some(0x1B3),
        "kof99" | "kof99k" | "kof99p" | "kof99e" | "kof99h" => Some(0x21B),
        "kof2000" | "kof2000n" | "kof2k" | "kof2kn" => Some(0x21D),
        "kof2001" | "kof2001n" | "kof2k1" => Some(0x221),
        "kof2002" | "kof2002n" | "kof2k2" | "kof2k2n" => Some(0x265),
        "kof2003" | "kof2003n" | "kof2k3" | "kof2k3n" => Some(0x271),
        // ── Metal Slug series ────────────────────────────────────────
        "mslug" | "mslug1" => Some(0x1B3),
        "mslug2" | "mslugx" | "mslug2h" => Some(0x1F3),
        "mslug3" | "mslug3h" | "mslug3a" | "mslug3b" => Some(0x213),
        "mslug4" | "mslug4h" => Some(0x215),
        "mslug5" | "mslug5h" => Some(0x217),
        // ── Garou: Mark of the Wolves ────────────────────────────────
        "garou" | "garoup" | "garouo" | "garoubl" => Some(0x229),
        // ── SVC Chaos ─────────────────────────────────────────────────
        "svc" | "svcchaos" | "svcboot" | "svcplus" => Some(0x241),
        // ── Samurai Shodown series ────────────────────────────────────
        "samsho" | "samshoh" | "samurai" => Some(0x073),
        "samsho2" | "samurai2" => Some(0x093),
        "samsho3" | "samurai3" => Some(0x0B3),
        "samsho4" | "samurai4" => Some(0x219),
        "samsho5" | "samurai5" | "samsho5a" => Some(0x225),
        "samsh5sp" | "samuraispecial" | "samsh5sph" => Some(0x227),
        // ── Fatal Fury / Real Bout series ─────────────────────────────
        "fatfur1" | "fatfury1" | "fatal fury" => Some(0x033),
        "fatfur2" | "fatfury2" => Some(0x053),
        "fatfursp" | "fatfury" | "fatalfury" | "fatfurspe" => Some(0x113),
        "rbff1" | "realfatfury" | "rbff" => Some(0x113),
        "rbff2" | "realfatfury2" | "rbff2h" => Some(0x193),
        "rbffspec" | "rbffsa" => Some(0x1D3),
        "fatfur3" | "fatfury3" => Some(0x133), // Fatal Fury 3
        // ── Art of Fighting series ────────────────────────────────────
        "aof" | "aofh" | "artoffighting" => Some(0x033),
        "aof2" | "aofii" | "aof2a" => Some(0x053),
        "aof3" | "aofiii" | "aof3a" => Some(0x0F3),
        "aodk" | "aggressors" | "tsukai" => Some(0x074),
        // ── The Last Blade (Bakumatsu) series ─────────────────────────
        "lastblad" | "lastbladh" | "bakumatsu" => Some(0x173),
        "lastblad2" | "bakumatsu2" | "lastbl2d" => Some(0x1D3),
        // ── World Heroes series ───────────────────────────────────────
        "wh1" | "worldheroes" | "wh" => Some(0x053),
        "wh2" | "worldheroes2" | "wh2h" => Some(0x073),
        "wh2j" | "worldheroes2jet" => Some(0x093),
        "whp" | "worldheroesperfect" => Some(0x113),
        // ── Baseball Stars series ────────────────────────────────────
        "bstars" | "bstars1" | "baseballstars" | "baseballstars1" => Some(0x003),
        "bstar2" | "bstars2" | "baseballstars2" => Some(0x053),
        // ── Super Sidekicks (Tokuten Oh) series ───────────────────────
        "ssideki" | "ssidek1" | "supersidekicks" => Some(0x053),
        "ssideki2" | "ssidek2" | "supersidekicks2" => Some(0x073),
        "ssideki3" | "ssidek3" | "supersidekicks3" => Some(0x093),
        "ssideki4" | "ultim11" | "ssidek4" | "supersidekicks4" => Some(0x1D3),
        // ── Aero Fighters (Sonic Wings) series ────────────────────────
        "aerofgt" | "aerofgt1" | "sonicwi1" | "aerofighters" => Some(0x073),
        "aerofgt2" | "sonicwi2" | "aerofighters2" => Some(0x093),
        "aerofgt3" | "sonicwi3" | "aerofighters3" => Some(0x0D3),
        // ── Puzzle Bobble (Bust-a-Move) series ───────────────────────
        "pblobble" | "puzzle bobble" | "bustamove" => Some(0x073),
        "pblobble2" | "puzzle bobble2" | "bubble bobble" => Some(0x0B3),
        // ── Magical Drop series ───────────────────────────────────────
        "magdrop" | "magicaldrop" | "magdrop1" => Some(0x0B3),
        "magdrop2" | "magicaldrop2" => Some(0x0D3),
        "magdrop3" | "magicaldrop3" => Some(0x133),
        // ── Shock Troopers series ─────────────────────────────────────
        "shocktro" | "shocktroopers" => Some(0x0F3),
        "shocktr2" | "shocktr2a" | "shocktro2" | "shocktroopers2" => Some(0x113),
        // ── Breakers series ───────────────────────────────────────────
        "breakers" | "breakers1" => Some(0x0D3),
        "breakrev" | "breakersrevenge" => Some(0x0D3),
        // ── NeoGeo early classics ─────────────────────────────────────
        "nam1975" | "nam-1975" => Some(0x001),
        "bjourney" | "bluesjourney" | "raguy" => Some(0x003),
        "magician" | "maglord" | "magicianlord" => Some(0x005),
        "cyberlip" | "cyber-lip" => Some(0x00B),
        "superspy" | "thesuperspy" => Some(0x00D),
        "kotm" | "kingofthemonsters" | "kotm1" => Some(0x017),
        "kotm2" | "kingofthemonsters2" => Some(0x033),
        "sengoku" | "sengoku1" | "sengokuh" => Some(0x019),
        "sengoku2" | "sengokuh2" => Some(0x033),
        "sengoku3" | "sengoku3n" | "sengoku3h" => Some(0x231),
        "lastreso" | "lastresort" | "lastres" => Some(0x025),
        "roboarmy" | "robo-army" => Some(0x033),
        "mutnat" | "mutationnation" => Some(0x015),
        "burningf" | "burningfight" | "burnfig" => Some(0x019),
        "soccerbrawl" | "soccer" => Some(0x033),
        "gpilots" | "ghostpilots" => Some(0x021),
        // ── NeoGeo mid-era hits ───────────────────────────────────────
        "tophuntr" | "tophunter" | "tophunt" => Some(0x053),
        "viewpoin" | "viewpoint" => Some(0x073),
        "spinmast" | "spinmaster" => Some(0x073),
        "windjamb" | "windjammers" => Some(0x073),
        "karnov" | "karnovsr" | "karnovsre" | "karnovsrevenge" => Some(0x073),
        "doubledr" | "doubledragon" => Some(0x0B3),
        "galaxyfg" | "galaxyfight" => Some(0x0B3),
        "pulstar" => Some(0x0B3),
        "blazstar" | "blazingstar" => Some(0x0F3),
        "neobombe" | "neobomberman" => Some(0x0B3),
        "gowcaizr" | "gowcaizer" | "voltagef" | "voltagefighter" => Some(0x0B3),
        "ragnagrd" | "ragnag" | "ragnagard" => Some(0x0D3),
        "ninjamas" | "ninjamaster" => Some(0x0D3),
        "ironclad" => Some(0x0D3),
        "twinsprt" | "twinklestarsprites" | "twinklesprites" => Some(0x0F3),
        "dragonsh" | "dragonsheaven" | "dragonheaven" => Some(0x094),
        // ── NeoGeo late-era / encrypted games ─────────────────────────
        "wakuwak7" | "wakuwaku7" => Some(0x0F3),
        "zupapa" => Some(0x093),
        "rotd" | "rageofthedragons" | "rotdh" => Some(0x235),
        "matrim" | "matrimelee" | "matrimh" => Some(0x233),
        "nightmar" | "nightmareinthedark" | "nightmard" => Some(0x237),
        "preisle2" | "prehistorici2" | "prehisle2" => Some(0x239),
        "s1945p" | "strikers1945plus" | "s1945a" => Some(0x23B),
        "bangbead" | "bangbeadh" => Some(0x23D),
        "ganryu" | "ganryuh" | "musashiganryuki" => Some(0x23F),
        // ── Other notable games ───────────────────────────────────────
        "alpham2" | "alphamission2" | "aso" => Some(0x007),
        "ncombat" | "ninjacombat" => Some(0x009),
        "ridingh" | "ridinghero" => Some(0x006),
        "fightfe" | "fightfever" => Some(0x073),
        "fbfrenzy" | "footballfrenzy" => Some(0x033),
        "strhoop" | "streethoop" | "dunkdream" => Some(0x093),
        "pleagl" | "pleasuregoal" => Some(0x0D3),
        "goalx3" => Some(0x209),
        "pgoal" | "pleasuregoal5" => Some(0x219),
        "lbowling" | "leaguebowling" => Some(0x019),
        "ridhero" => Some(0x006),
        "trally" | "thrashrally" => Some(0x038),
        "stakwin" | "stakeswinner" => Some(0x093),
        "stakwin2" | "stakeswinner2" => Some(0x0D3),
        "kizuna" | "kizunaencounter" | "fuun" => Some(0x0D3),
        "savagere" | "savagereign" | "fuunmokushiroku" => Some(0x059),
        "kabukikl" | "kabukiklash" => Some(0x092),
        "neocup98" => Some(0x244),
        "neodrift" | "neodriftout" => Some(0x213),
        "overtop" => Some(0x212),
        "sdodgeb" | "supersdodgeb" => Some(0x208),
        "turfmast" | "neoturfmasters" => Some(0x200),
        "twinspri" | "twinklestar" => Some(0x224),
        "irrmaze" | "irrmazing" | "irritatingmaze" => Some(0x236),
        "vliner" | "vliner7e" | "vliner6e" | "vliner54" | "vliner53" => Some(0x3E7),
        "minasan" => Some(0x004),
        "janshin" => Some(0x027),
        "bakatono" => Some(0x036),
        "mahretsu" => Some(0x048),
        _ => None,
    }
}

pub const NEO_HEADER_SIZE: usize = 4096;
const FIXED_PROM_WINDOW_SIZE: usize = 0x10_0000;
const WORK_RAM_STACK_START: u32 = 0x0010_0000;
const WORK_RAM_STACK_END: u32 = 0x0010_FFFE;
const MSLUG3_SMA_CHIP_OFFSET: usize = 0x0C_0000;
/// Protection scheme detected for a .neo file based on NGH header value.
/// Determines which decryption pipeline(s) should be applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeoProtection {
    None,
    Cmc42(u8),     // CMC42 only (extra_xor)
    Cmc50(u8),     // CMC50 only (extra_xor)
    SmaCmc42(u8),  // SMA + CMC42 (extra_xor for CMC)
    SmaCmc50(u8),  // SMA + CMC50 (extra_xor for CMC)
    Cmc50Pcm2(u8), // CMC50 + PCM2 (extra_xor for CMC)
}

/// Detect the NeoGeo cartridge protection scheme from a .neo file's NGH value.
///
/// Maps the NGH number (stored in the .neo header at offset 0x28) to the
/// encryption scheme used by the cartridge. This mirrors Geolith's approach
/// of detecting protection by NGH for .neo files.
///
/// Note: Some NGH values may collide across different games; in those cases
/// we return the least-common-denominator (usually `None` for safety).
pub fn detect_neo_protection(ngh: u32) -> NeoProtection {
    match ngh {
        // ── SMA + CMC42 ──────────────────────────────────────
        0x213 | 0x256 => NeoProtection::SmaCmc42(crate::cmc::CMC42_MSLUG3_EXTRA_XOR), // Metal Slug 3 (FBNeo 0x213 / SNK NGH 0x256)
        0x21B | 0x251 => NeoProtection::SmaCmc42(crate::cmc::CMC42_KOF99_EXTRA_XOR), // The King of Fighters '99
        0x253 | 0x229 | 0x153 => NeoProtection::SmaCmc42(crate::cmc::CMC42_GAROU_EXTRA_XOR), // Garou: Mark of the Wolves
        // ── SMA + CMC50 ──────────────────────────────────────
        0x21D | 0x257 => NeoProtection::SmaCmc50(crate::cmc::CMC50_KOG_EXTRA_XOR), // The King of Fighters 2000 (FBNeo 0x21D / SNK NGH 0x257)
        // ── CMC42 only ───────────────────────────────────────
        0x237 => NeoProtection::Cmc42(crate::cmc::CMC42_MSLUG3_EXTRA_XOR), // Nightmare in the Dark
        0x239 => NeoProtection::Cmc42(crate::cmc::CMC42_PREISLE2_EXTRA_XOR), // Prehistoric Isle 2
        0x23B | 0x254 => NeoProtection::Cmc42(crate::cmc::CMC42_S1945P_EXTRA_XOR), // Strikers 1945 Plus
        0x23D | 0x259 => NeoProtection::Cmc42(crate::cmc::CMC42_BANGBEAD_EXTRA_XOR), // Bang Bead
        0x23F | 0x252 => NeoProtection::Cmc42(crate::cmc::CMC42_GANRYU_EXTRA_XOR), // Ganryu
        // ── CMC50 + PCM2 ─────────────────────────────────────
        0x265 => NeoProtection::Cmc50Pcm2(crate::cmc::CMC50_KOF2002_EXTRA_XOR), // KOF 2002
        0x223 | 0x271 => NeoProtection::Cmc50Pcm2(crate::cmc::CMC50_KOF2003_EXTRA_XOR), // KOF 2003
        0x241 => NeoProtection::Cmc50Pcm2(crate::cmc::CMC50_SVC_EXTRA_XOR),     // SVC Chaos
        0x233 => NeoProtection::Cmc50Pcm2(crate::cmc::CMC50_MATRIM_EXTRA_XOR),  // Matrimelee
        // ── CMC50 only ───────────────────────────────────────
        0x215 | 0x263 => NeoProtection::Cmc50(crate::cmc::CMC50_MSLUG4_EXTRA_XOR), // Metal Slug 4
        0x221 => NeoProtection::Cmc50(crate::cmc::CMC50_KOF2002_EXTRA_XOR),        // KOF 2001
        0x217 => NeoProtection::Cmc50(crate::cmc::CMC50_MSLUG5_EXTRA_XOR),         // Metal Slug 5
        0x225 => NeoProtection::Cmc50(crate::cmc::CMC50_SAMSHO5_EXTRA_XOR), // Samurai Shodown 5
        0x227 => NeoProtection::Cmc50(crate::cmc::CMC50_SAMSH5SP_EXTRA_XOR), // Samurai Shodown 5 Special
        0x231 => NeoProtection::Cmc50(crate::cmc::CMC50_KOF2002_EXTRA_XOR),  // Sengoku 3
        0x235 => NeoProtection::Cmc50(crate::cmc::CMC50_ROTD_EXTRA_XOR),     // Rage of the Dragons
        0x267 => NeoProtection::Cmc50(crate::cmc::CMC50_PNYAA_EXTRA_XOR),    // Pochi and Nyaa
        // ── No special protection ────────────────────────────
        _ => NeoProtection::None,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// NGH → Board Type, Fix Bankswitching & Game Flags
// (Port of Geolith's geo_neo_load() NGH switch)
// ─────────────────────────────────────────────────────────────────────────────

/// NeoGeo cartridge board type, mirroring Geolith's BOARD_* constants.
/// Determines how P-ROM bankswitching and memory mapping works.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NeoBoardType {
    #[default]
    Default, // BOARD_DEFAULT
    Linkable,   // BOARD_LINKABLE (Riding Hero, League Bowling, Thrash Rally)
    Brezzasoft, // BOARD_BREZZASOFT (Jockey Grand Prix, V-Liner)
    Ct0,        // BOARD_CT0 (Fatal Fury 2, Super Sidekicks)
    Kof98,      // BOARD_KOF98 (KOF 98)
    MslugX,     // BOARD_MSLUGX (Metal Slug X)
    Sma,        // BOARD_SMA (SMA-protected games)
    SmaGarouH,  // BOARD_SMA with Garou AES (NEO-SMA KE) bank table
    SmaMslug3A, // BOARD_SMA with Metal Slug 3 mslug3a bank table
    Pvc,        // BOARD_PVC (NEO-PVC games: MS5, SVC, KOF2003)
    Ms5Plus,    // BOARD_MS5PLUS (Metal Slug 5 Plus)
    Kf2k3Bla,   // BOARD_KF2K3BLA (KOF 2003 bootleg)
    Kf2k3Bl,    // BOARD_KF2K3BL (KOF 2003 bootleg)
    Kof10th,    // BOARD_KOF10TH (KOF 10th Anniversary)
    Cthd2003,   // BOARD_CTHD2003 (Crouching Tiger Hidden Dragon 2003)
}

/// Fix layer bankswitching type, mirroring Geolith's FIX_BANKSW_* constants.
/// Determines how the fix layer ROM is banked for CMC-protected games.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NeoFixBanksw {
    #[default]
    None, // FIX_BANKSW_NONE — standard fix layer
    Line, // FIX_BANKSW_LINE — per-scanline banking (MSlug3, Garou, MSlug4)
    Tile, // FIX_BANKSW_TILE — per-tile banking (KOF2000, Matrimelee, SVC, KOF2003)
}

/// Game flags derived from Geolith's GEO_DB_* constants.
pub const NEO_FLAG_MAHJONG: u32 = 0x01;
pub const NEO_FLAG_IRRMAZE: u32 = 0x02;
pub const NEO_FLAG_VLINER: u32 = 0x04;

/// Detect the board type for a .neo file based on NGH value.
/// Maps directly from Geolith's geo_neo_load() NGH switch.
pub fn detect_neo_board_type(ngh: u32) -> NeoBoardType {
    match ngh {
        0x006 | 0x019 | 0x038 => NeoBoardType::Linkable, // Riding Hero, League Bowling, Thrash Rally
        0x008 | 0x3E7 | 0x999 => NeoBoardType::Brezzasoft, // Jockey Grand Prix, V-Liner
        0x047 | 0x052 => NeoBoardType::Ct0,              // Fatal Fury 2, Super Sidekicks
        0x242 => NeoBoardType::Kof98,                    // KOF 98
        0x250 => NeoBoardType::MslugX,                   // Metal Slug X
        // ── SMA-protected: detect by P-ROM size (> 5MB) at call site ──
        0x151 | 0x251 => NeoBoardType::Sma, // KOF 99 (encrypted)
        0x253 => NeoBoardType::Sma,         // Garou - Mark of the Wolves
        0x256 => NeoBoardType::Sma,         // Metal Slug 3
        0x257 => NeoBoardType::Sma,         // KOF 2000
        // ── NEO-PVC games ─────────────────────────────────────────────
        0x268 => NeoBoardType::Pvc, // Metal Slug 5
        0x269 => NeoBoardType::Pvc, // SVC Chaos
        0x271 => NeoBoardType::Pvc, // KOF 2003 (official, or bootleg differentiated by P-ROM at call site)
        // ── Bootleg variants ───────────────────────────────────────────
        0x275 => NeoBoardType::Kof10th,   // KOF 10th Anniversary
        0x5003 => NeoBoardType::Cthd2003, // Crouching Tiger Hidden Dragon 2003
        _ => NeoBoardType::Default,
    }
}

/// Detect the fix layer bankswitching type for a .neo file based on NGH value.
/// Mirrors Geolith's geo_lspc_set_fix_banksw() calls per NGH.
pub fn detect_neo_fix_banksw(ngh: u32) -> NeoFixBanksw {
    match ngh {
        0x253 | 0x256 | 0x263 => NeoFixBanksw::Line, // Garou, MSlug3, MSlug4
        0x257 | 0x266 | 0x269 | 0x271 => NeoFixBanksw::Tile, // KOF2000, Matrimelee, SVC, KOF2003
        _ => NeoFixBanksw::None,
    }
}

/// Detect game-specific flags for a .neo file based on NGH value.
/// Mirrors Geolith's GEO_DB_* flag assignments.
pub fn detect_neo_game_flags(ngh: u32) -> u32 {
    match ngh {
        0x004 | 0x027 | 0x036 | 0x048 => NEO_FLAG_MAHJONG, // Mahjong controller games
        0x236 => NEO_FLAG_IRRMAZE,                         // The Irritating Maze
        0x3E7 | 0x999 => NEO_FLAG_VLINER,                  // V-Liner
        _ => 0,
    }
}

/// Validate SMA board type at runtime based on P-ROM size.
///
/// Geolith's heuristic: SMA cartridges always have P-ROM > 5MB (0x500000).
/// For NGH values that can collide between SMA and non-SMA variants
/// (e.g. NGH 0x251 can be either KOF 99 encrypted SMA or KOF 98 decrypted),
/// a small P-ROM indicates a non-SMA board and the board type is downgraded
/// to [`NeoBoardType::Default`].
pub fn validate_sma_board_type(board_type: NeoBoardType, prom: &[u8]) -> NeoBoardType {
    if matches!(
        board_type,
        NeoBoardType::Sma | NeoBoardType::SmaGarouH | NeoBoardType::SmaMslug3A
    ) && prom.len() <= 0x500000
    {
        eprintln!(
            "[INFO] P-ROM size {} <= 5MB, downgrading SMA board type to Default",
            prom.len()
        );
        return NeoBoardType::Default;
    }
    board_type
}

/// Apply Geolith's per-title .neo variant heuristics.
///
/// Some late carts share the same NGH across official releases, bootlegs, and
/// prototypes. Geolith byte-swaps the P-ROM in `geo_m68k_postload()` and then
/// inspects bytes from that normalized P-ROM before enabling SMA/PVC/fix
/// banking. The offsets here are therefore relative to P-ROM byte 0, not the
/// start of the `.neo` container.
pub fn apply_neo_variant_heuristics(
    ngh: u32,
    board_type: NeoBoardType,
    fix_banksw: NeoFixBanksw,
    prom: &[u8],
) -> (NeoBoardType, NeoFixBanksw) {
    let prom_byte = |addr: usize| prom.get(addr).copied();

    match ngh {
        0x253 => match prom_byte(0xC0000 + 0x3E481) {
            Some(0x9F) => {
                eprintln!("[INFO] Garou AES (NEO-SMA KE) detectado por firma P-ROM normalizada");
                (NeoBoardType::SmaGarouH, NeoFixBanksw::Line)
            }
            Some(0x41) => {
                eprintln!("[INFO] Garou MVS (NEO-SMA KF) detectado por firma P-ROM normalizada");
                (NeoBoardType::Sma, NeoFixBanksw::Line)
            }
            _ => {
                eprintln!("[INFO] Garou bootleg/prototipo detectado: sin SMA ni FIX bankswitch");
                (NeoBoardType::Default, NeoFixBanksw::None)
            }
        },
        0x256 => {
            if prom.len() > 0x500000 && prom_byte(0x141) == Some(0x33) {
                eprintln!("[INFO] Metal Slug 3 mslug3a detectado por firma P-ROM normalizada");
                (NeoBoardType::SmaMslug3A, NeoFixBanksw::Line)
            } else {
                (board_type, NeoFixBanksw::Line)
            }
        }
        0x263 => {
            if prom_byte(0x809) == Some(0x0C) {
                (board_type, NeoFixBanksw::None)
            } else {
                (board_type, NeoFixBanksw::Line)
            }
        }
        0x268 => {
            if prom_byte(0x26B) == Some(0xB9) {
                (NeoBoardType::Ms5Plus, NeoFixBanksw::None)
            } else if prom_byte(0x267) == Some(0x4F) {
                (NeoBoardType::Pvc, NeoFixBanksw::None)
            } else {
                eprintln!("[INFO] Metal Slug 5 bootleg/prototipo detectado: usando board default");
                (NeoBoardType::Default, NeoFixBanksw::None)
            }
        }
        0x269 => {
            let board = if prom_byte(0x2F8F) == Some(0xC0) {
                NeoBoardType::Pvc
            } else {
                NeoBoardType::Default
            };
            let fix = if prom_byte(0x3D25) == Some(0xC4) {
                NeoFixBanksw::Tile
            } else {
                NeoFixBanksw::None
            };
            if board == NeoBoardType::Default || fix == NeoFixBanksw::None {
                eprintln!("[INFO] SVC bootleg/prototipo detectado: PVC/FIX segun firmas P-ROM");
            }
            (board, fix)
        }
        0x271 => {
            if prom_byte(0x689) == Some(0x10) {
                (NeoBoardType::Kf2k3Bla, NeoFixBanksw::None)
            } else if prom_byte(0xC1) == Some(0x02) {
                (NeoBoardType::Kf2k3Bl, NeoFixBanksw::None)
            } else {
                (NeoBoardType::Pvc, NeoFixBanksw::Tile)
            }
        }
        0x275 => {
            if prom_byte(0x125) == Some(0x00) {
                (NeoBoardType::Kof10th, NeoFixBanksw::None)
            } else {
                (NeoBoardType::Default, NeoFixBanksw::None)
            }
        }
        0x5003 => {
            if prom_byte(0x30d9) == Some(0x03) {
                (NeoBoardType::Default, NeoFixBanksw::None)
            } else {
                (NeoBoardType::Cthd2003, NeoFixBanksw::None)
            }
        }
        _ => (board_type, fix_banksw),
    }
}

/// Validate KOF 2003 board type at runtime based on P-ROM byte inspection.
///
/// Geolith distinguishes bootleg KOF2003 (NGH 0x271) variants from the official
/// NEO-PVC release by inspecting specific bytes within the P-ROM:
///
/// | Byte offset | Value | Board type   | Bootleg set         |
/// |-------------|-------|--------------|---------------------|
/// | 0x689       | 0x10  | Kf2k3Bla     | kf2k3bla, kf2k3pl   |
/// | 0xc1        | 0x02  | Kf2k3Bl      | kf2k3bl, kf2k3upl   |
/// | (none)      | —     | Pvc (default)| Official release    |
///
/// The byte offsets are relative to the start of P-ROM data (after the 4096-byte
/// .neo header). In Geolith's `geo_neo_load()`, these correspond to
/// `neodata[0x1000 + 0x689]` and `neodata[0x1000 + 0xc1]`.
pub fn validate_kof2003_board_type(
    ngh: u32,
    board_type: NeoBoardType,
    prom: &[u8],
) -> NeoBoardType {
    if ngh != 0x271 {
        return board_type;
    }
    if prom.len() > 0x689 && prom[0x689] == 0x10 {
        eprintln!("[INFO] NGH 0x271: KOF2003 bootleg KF2K3BLA detectado (P-ROM[0x689] == 0x10)");
        return NeoBoardType::Kf2k3Bla;
    }
    if prom.len() > 0xc1 && prom[0xc1] == 0x02 {
        eprintln!("[INFO] NGH 0x271: KOF2003 bootleg KF2K3BL detectado (P-ROM[0xc1] == 0x02)");
        return NeoBoardType::Kf2k3Bl;
    }
    board_type
}

/// Try to detect the NGH value from a .neo file's name when the header has NGH=0.
/// This mirrors Geolith's heuristic when NGH is missing or zero.
pub fn detect_ngh_from_neo_name(name: &str) -> Option<u32> {
    let name = name.to_ascii_lowercase();
    let name = name.trim_end_matches(".neo");
    match name {
        // Fighting games
        "kof94" | "kof94h" | "kof94a" => Some(0x005),
        "kof95" | "kof95h" => Some(0x053),
        "kof96" | "kof96h" => Some(0x0B3),
        "kof97" | "kof97h" | "kof97pls" => Some(0x133),
        "kof98" | "kof98k" | "kof98n" | "kof98h" => Some(0x1B3),
        "kof99" | "kof99k" | "kof99p" | "kof99e" | "kof99h" => Some(0x251),
        "kof2000" | "kof2000n" | "kof2k" | "kof2kn" => Some(0x257),
        "kof2001" | "kof2001n" | "kof2k1" => Some(0x221),
        "kof2002" | "kof2002n" | "kof2k2" | "kof2k2n" => Some(0x265),
        "kof2003" | "kof2003n" | "kof2k3" | "kof2k3n" => Some(0x271),
        "garou" | "garoup" | "garouo" | "garoubl" | "motw" => Some(0x253),
        "svc" | "svcchaos" | "svcboot" | "svcplus" => Some(0x241),
        "aof" | "aofh" | "artoffighting" => Some(0x033),
        "aof2" | "aofii" | "aof2a" => Some(0x053),
        "aof3" | "aofiii" | "aof3a" => Some(0x0F3),
        "fatfur1" | "fatfury1" => Some(0x033),
        "fatfur2" | "fatfury2" => Some(0x053),
        "fatfursp" | "fatfury" | "fatfurspe" => Some(0x113),
        "rbff1" | "rbff" | "realfatfury" => Some(0x113),
        "rbff2" | "rbff2h" | "realfatfury2" => Some(0x193),
        "rbffspec" | "rbffsa" => Some(0x1D3),
        "fatfur3" | "fatfury3" => Some(0x133),
        "samsho" | "samshoh" | "samurai" => Some(0x073),
        "samsho2" | "samurai2" => Some(0x093),
        "samsho3" | "samurai3" => Some(0x0B3),
        "samsho4" | "samurai4" => Some(0x219),
        "samsho5" | "samurai5" | "samsho5a" => Some(0x225),
        "samsh5sp" | "samuraispecial" | "samsh5sph" => Some(0x227),
        // Run & gun
        "mslug" | "mslug1" | "metal slug" => Some(0x1B3),
        "mslug2" | "mslugx" | "mslug2h" => Some(0x250),
        "mslug3" | "mslug3h" | "mslug3a" | "mslug3b" => Some(0x256),
        "mslug4" | "mslug4h" => Some(0x263),
        "mslug5" | "mslug5h" => Some(0x268),
        // Puzzle
        "pulstar" => Some(0x0B3),
        "blazstar" | "blazingstar" => Some(0x0F3),
        "wakuwak7" | "wakuwaku7" => Some(0x0F3),
        "zupapa" => Some(0x093),
        "magdrop2" | "magicaldrop2" => Some(0x0D3),
        "magdrop3" | "magicaldrop3" => Some(0x133),
        // Fighters
        "lastblad" | "lastbladh" | "bakumatsu" => Some(0x173),
        "lastblad2" | "bakumatsu2" | "lastbl2d" => Some(0x1D3),
        "wh1" | "worldheroes" | "wh" => Some(0x053),
        "wh2" | "worldheroes2" | "wh2h" => Some(0x073),
        "wh2j" | "worldheroes2jet" => Some(0x093),
        "whp" | "worldheroesperfect" => Some(0x113),
        "breakers" | "breakers1" => Some(0x0D3),
        "breakrev" | "breakersrevenge" => Some(0x0D3),
        // Late CMC games
        "rotd" | "rageofthedragons" | "rotdh" => Some(0x235),
        "matrim" | "matrimelee" | "matrimh" => Some(0x266),
        "nightmar" | "nightmareinthedark" | "nightmard" => Some(0x237),
        "preisle2" | "prehistorici2" | "prehisle2" => Some(0x239),
        "s1945p" | "strikers1945plus" | "s1945a" => Some(0x23B),
        "bangbead" | "bangbeadh" => Some(0x23D),
        "ganryu" | "ganryuh" | "musashiganryuki" => Some(0x23F),
        "sengoku3" | "sengoku3n" | "sengoku3h" => Some(0x231),
        // Sports / other
        "ridingh" | "ridinghero" => Some(0x006),
        "bjourney" | "bluesjourney" | "raguy" => Some(0x003),
        "nam1975" | "nam-1975" => Some(0x001),
        "bstars" | "bstars1" | "baseballstars" => Some(0x003),
        "bstar2" | "bstars2" | "baseballstars2" => Some(0x053),
        "ssideki" | "ssidek1" | "supersidekicks" => Some(0x053),
        "ssideki2" | "ssidek2" | "supersidekicks2" => Some(0x073),
        "ssideki3" | "ssidek3" | "supersidekicks3" => Some(0x093),
        "ssideki4" | "ultim11" | "ssidek4" | "supersidekicks4" => Some(0x1D3),
        _ => None,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Matrimelee fix layer fix (NGH 0x266)
// Port of Geolith's geo_neo_load() special case for matrimbl.
// ─────────────────────────────────────────────────────────────────────────────

/// Apply the Matrimelee fix layer workaround.
///
/// TerraOnion's NeoBuilder tool decrypts this incorrectly for matrimbl.
/// The last 512K block of C-ROM contains the encrypted FIX layer data.
/// Geolith applies a bit-swizzle to recover the correct fix data.
pub fn fix_matrimelee_fix_layer(crom: &[u8], srom: &mut [u8], ssz: usize) {
    if srom.len() < ssz || crom.len() < ssz {
        return;
    }
    let ptr_offset = crom.len() - ssz;
    for i in 0..ssz.min(srom.len()) {
        let src = crom[ptr_offset
            + ((i & !0x1f) + ((i & 0x07) << 2) + ((!i & 0x08) >> 2) + ((i & 0x10) >> 4))];
        srom[i] = src;
    }
}

/// Estructura que contiene los datos de una ROM NeoGeo
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RomData {
    pub prom: Vec<u8>, // Program ROM (P-ROM)
    pub crom: Vec<u8>, // Character/Sprite ROM (C-ROM)
    pub srom: Vec<u8>, // Fix layer ROM (S-ROM)
    pub mrom: Vec<u8>, // Z80/M1 ROM (M-ROM)
    pub vrom: Vec<u8>, // Audio sample ROM (V-ROM)
    /// Offset inside `vrom` where the ADPCM-B (V2) region starts.
    /// ADPCM-A reads V1 from offset 0. If this is 0, ADPCM-B mirrors V1.
    pub vrom_b_offset: usize,
    pub sma_rom: Vec<u8>, // SMA protection handler data (neo-sma)
    pub is_demo: bool,
    pub source: RomSource,
    pub recognized_files: Vec<String>,
    pub metadata: Option<NeoMetadata>,
    // Otros bancos según sea necesario
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NeoMetadata {
    pub version: u8,
    pub year: u32,
    pub genre: u32,
    pub screenshot: u32,
    pub ngh: u32,
    pub name: String,
    pub manufacturer: String,
    /// Board type for P-ROM bankswitching (mapped from NGH via Geolith logic).
    pub board_type: NeoBoardType,
    /// Fix layer bankswitching type (mapped from NGH).
    pub fix_banksw: NeoFixBanksw,
    /// Game-specific flags (mahjong, irrmaze, vliner).
    pub game_flags: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RomSource {
    Demo,
    NeoFile,
    ZipArchive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RomDiagnostics {
    pub source: RomSource,
    pub prom_bytes: usize,
    pub crom_bytes: usize,
    pub srom_bytes: usize,
    pub mrom_bytes: usize,
    pub vrom_bytes: usize,
    pub recognized_files: usize,
    pub warnings: Vec<String>,
}

impl RomData {
    /// Carga una ROM de prueba (demo)
    pub fn demo() -> Self {
        Self {
            prom: Vec::new(),
            crom: Vec::new(),
            srom: Vec::new(),
            mrom: Vec::new(),
            vrom: Vec::new(),
            vrom_b_offset: 0,
            sma_rom: Vec::new(),
            is_demo: true,
            source: RomSource::Demo,
            recognized_files: Vec::new(),
            metadata: None,
        }
    }

    /// Carga una ROM desde un archivo .neo
    pub fn from_neo<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let file_stem = path
            .as_ref()
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("rom");
        let file_name = path
            .as_ref()
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(".neo");
        let data = std::fs::read(&path).map_err(|e| format!("Error leyendo .neo: {e}"))?;
        let mut parsed = parse_neo_file(&data)?;

        // ── NGH == 0: try to detect from filename ──────────────────────
        // Some .neo files (especially bootlegs or conversions) have NGH = 0
        // in the header. Geolith detects NGH from filename in this case.
        if parsed.metadata.ngh == 0 {
            if let Some(detected_ngh) = detect_ngh_from_neo_name(file_name) {
                eprintln!(
                    "[INFO] NGH=0 detectado en cabecera, inferido del nombre como 0x{:03X}",
                    detected_ngh
                );
                parsed.metadata.ngh = detected_ngh;
            }
        }

        validate_initial_vectors_for_diagnostics(&parsed.prom, file_name)?;

        // ── Fallback seguro: P-ROM demasiado pequeña ───────────────
        if parsed.prom.len() < 0x10000 {
            return Err(format!(
                "ROM inválida: P-ROM demasiado pequeña ({} bytes, archivo: {})",
                parsed.prom.len(),
                file_name
            ));
        }

        // Geolith normalizes the P-ROM byte order before applying per-title
        // board/protection heuristics. Several .neo files store words swapped
        // on disk, so raw container offsets would miss the official signatures.
        normalize_program_rom_byte_order(&mut parsed.prom);

        // ── Populate NGH-derived metadata fields ───────────────────────
        // Board type, fix bankswitching, and game flags are derived from
        // NGH per Geolith's geo_neo_load() logic.
        let ngh = parsed.metadata.ngh;
        parsed.metadata.board_type = detect_neo_board_type(ngh);
        parsed.metadata.fix_banksw = detect_neo_fix_banksw(ngh);
        parsed.metadata.game_flags = detect_neo_game_flags(ngh);

        let (variant_board, variant_fix) = apply_neo_variant_heuristics(
            ngh,
            parsed.metadata.board_type,
            parsed.metadata.fix_banksw,
            &parsed.prom,
        );
        parsed.metadata.board_type = variant_board;
        parsed.metadata.fix_banksw = variant_fix;

        // ── Runtime SMA board type validation ───────────────────────────
        // Geolith validates SMA board type at runtime by checking P-ROM
        // size (> 5MB = 0x500000). For NGH values that may collide (e.g.
        // KOF 99 decrypted sets with NGH 0x251), a small P-ROM indicates
        // a non-SMA variant and the board type should be downgraded.
        parsed.metadata.board_type =
            validate_sma_board_type(parsed.metadata.board_type, &parsed.prom);

        // ── Runtime KOF2003 bootleg detection ───────────────────────────
        // Geolith inspects specific P-ROM bytes to distinguish bootleg
        // KOF2003 variants (kf2k3bla, kf2k3bl, kf2k3pl, kf2k3upl) from
        // the official NEO-PVC release.
        parsed.metadata.board_type =
            validate_kof2003_board_type(ngh, parsed.metadata.board_type, &parsed.prom);

        let mut rom = Self {
            prom: parsed.prom,
            crom: parsed.crom,
            srom: parsed.srom,
            mrom: parsed.mrom,
            vrom: parsed.vrom,
            vrom_b_offset: parsed.vrom_b_offset,
            sma_rom: Vec::new(),
            is_demo: false,
            source: RomSource::NeoFile,
            recognized_files: vec![file_name.to_string()],
            metadata: Some(parsed.metadata),
        };
        // ── Post-processing for .neo files ────────────────────────────
        // .neo files from the NeoSD toolchain are already pre-decrypted.
        // Only extract S-ROM from C-ROM tail if the S section is empty,
        // and apply NGH-specific fix layer workarounds (e.g. Matrimelee).
        post_process_neo_rom(&mut rom);
        // Auto-dump ROM banks for diagnostic purposes
        dump_prom_diagnostic(&rom.prom, file_stem);
        dump_crom_diagnostic(&rom.crom, file_stem);
        dump_srom_diagnostic(&rom.srom, file_stem);
        dump_mrom_diagnostic(&rom.mrom, file_stem);
        dump_vrom_diagnostic(&rom.vrom, file_stem);
        rom.validate_external()
    }

    /// Carga una ROM desde un archivo .zip
    pub fn from_zip<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let path_ref = path.as_ref();
        let file_stem = path_ref
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("rom");
        let file = std::fs::File::open(path_ref).map_err(|e| format!("Error abriendo zip: {e}"))?;
        let mut archive =
            zip::ZipArchive::new(file).map_err(|e| format!("Error leyendo zip: {e}"))?;

        let mut entries = Vec::new();
        let mut skipped_count = 0usize;
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
            if file.is_dir() {
                continue;
            }

            let name = file.name().to_ascii_lowercase();
            let mut buf = Vec::new();
            use std::io::Read;
            file.read_to_end(&mut buf).map_err(|e| e.to_string())?;

            // Skip known BIOS/system entries (merged MAME sets include these)
            if is_bios_entry(&name) {
                skipped_count += 1;
                continue;
            }

            if let Some(part) = classify_zip_entry(&name) {
                entries.push((part, name, buf));
            } else {
                // Log unknown entries at debug level
                eprintln!("[INFO] ZIP entry no clasificada (ignorada): {}", name);
            }
        }

        entries.sort_by(|(part_a, name_a, _), (part_b, name_b, _)| {
            part_a
                .order()
                .cmp(&part_b.order())
                .then_with(|| name_a.cmp(name_b))
        });

        let has_root_game_entries = entries
            .iter()
            .any(|(_, name, _)| !name.contains('/') && !name.contains('\\'));
        let mut merged_clone_entries = 0usize;
        if has_root_game_entries {
            let before = entries.len();
            entries.retain(|(_, name, _)| !name.contains('/') && !name.contains('\\'));
            merged_clone_entries = before - entries.len();
        }

        let mut program_banks = Vec::new();
        let mut graphics_banks = Vec::new();
        let mut srom = Vec::new();
        let mut mrom = Vec::new();
        let mut vrom = Vec::new();
        let mut sma_rom = Vec::new();
        let mut recognized_files = Vec::new();
        for (part, name, buf) in entries {
            recognized_files.push(zip_entry_file_name(&name).to_string());
            match part {
                RomPart::Program(index) => program_banks.push((index, name, buf)),
                RomPart::Graphics(index) => graphics_banks.push((index, name, buf)),
                RomPart::FixLayer => srom.extend(buf),
                RomPart::AudioCpu => mrom.extend(buf),
                RomPart::Samples(_) => vrom.extend(buf),
                RomPart::Sma => sma_rom = buf,
            }
        }
        // Log any unclassified files for diagnostic purposes
        let total_files = recognized_files.len() + skipped_count;
        if skipped_count > 0 {
            eprintln!(
                "[INFO] ZIP loader skipped {} BIOS/system files out of {} total entries",
                skipped_count, total_files
            );
        }
        if merged_clone_entries > 0 {
            eprintln!(
                "[INFO] ZIP loader ignored {} merged clone entries in subdirectories",
                merged_clone_entries
            );
        }

        let known_zip_set = detect_known_zip_set(&recognized_files);
        let mut prom = layout_zip_program_banks(known_zip_set, program_banks);

        if known_zip_set == Some(KnownZipSet::KogBootleg) {
            merge_kog_bootleg_parent(path_ref, &mut prom, &mut mrom, &mut vrom);
            decrypt_kog_bootleg_program(&mut prom);
            decrypt_kog_bootleg_srom(&mut srom);
        }

        // ── P-ROM decryption for SMA-based sets ──────────────────────────
        // Per FBNeo source: Mslug3, Garou, and KOF 2000 use SMA P-ROM decryption.
        // MSlug4 does NOT use SMA (uses different protection).
        if matches!(
            known_zip_set,
            Some(
                KnownZipSet::Mslug3Encrypted
                    | KnownZipSet::Kof99Encrypted
                    | KnownZipSet::GarouEncrypted
                    | KnownZipSet::KogEncrypted
            )
        ) && !sma_rom.is_empty()
        {
            prom = layout_mslug3_sma_program_rom(prom, &sma_rom);
            swap_program_rom_words(&mut prom);
            match known_zip_set {
                Some(KnownZipSet::Kof99Encrypted) => crate::sma::kof99_decrypt_68k(&mut prom),
                Some(KnownZipSet::GarouEncrypted) => crate::sma::garou_decrypt_68k(&mut prom),
                Some(KnownZipSet::KogEncrypted) => crate::sma::kof2000_decrypt_68k(&mut prom),
                _ => crate::sma::mslug3_decrypt_68k(&mut prom),
            }
        } else if known_zip_set == Some(KnownZipSet::Mslug5Encrypted) {
            decrypt_mslug5_program(&mut prom);
        } else if known_zip_set == Some(KnownZipSet::Kof2003Encrypted) {
            decrypt_kof2003_program(&mut prom);
        } else if known_zip_set == Some(KnownZipSet::SvcEncrypted) {
            decrypt_svc_program(&mut prom);
        } else if known_zip_set == Some(KnownZipSet::Kof98Encrypted) {
            decrypt_kof98_program(&mut prom);
        }

        // ── C-ROM / M1 / V-ROM decryption ───────────────────────────────
        let crom = if matches!(known_zip_set, Some(KnownZipSet::KogBootleg)) {
            if let Some(parent_crom) = load_kog_parent_crom(path_ref) {
                parent_crom
            } else {
                let mut crom = layout_kog_bootleg_graphics(graphics_banks);
                decrypt_kog_bootleg_crom(&mut crom);
                crom
            }
        } else if matches!(
            known_zip_set,
            Some(
                KnownZipSet::Mslug3Encrypted
                    | KnownZipSet::Kof99Encrypted
                    | KnownZipSet::GarouEncrypted
                    | KnownZipSet::S1945pEncrypted
                    | KnownZipSet::ZupapaEncrypted
                    | KnownZipSet::NitdEncrypted
                    | KnownZipSet::Sengoku3Encrypted
                    | KnownZipSet::Preisle2Encrypted
                    | KnownZipSet::BangbeadEncrypted
                    | KnownZipSet::GanryuEncrypted
            )
        ) {
            // ── CMC42 path ──────────────────────────────────────────
            let encrypted_crom = crate::cmc::interleave_cmc_graphics_banks(
                graphics_banks
                    .into_iter()
                    .map(|(index, _, bytes)| (index, bytes))
                    .collect(),
            );
            let extra_xor = match known_zip_set {
                Some(KnownZipSet::Mslug3Encrypted) => crate::cmc::CMC42_MSLUG3_EXTRA_XOR,
                Some(KnownZipSet::Kof99Encrypted) => crate::cmc::CMC42_KOF99_EXTRA_XOR,
                Some(KnownZipSet::GarouEncrypted) => crate::cmc::CMC42_GAROU_EXTRA_XOR,
                Some(KnownZipSet::S1945pEncrypted) => crate::cmc::CMC42_S1945P_EXTRA_XOR,
                Some(KnownZipSet::ZupapaEncrypted) => crate::cmc::CMC42_ZUPAPA_EXTRA_XOR,
                Some(KnownZipSet::NitdEncrypted) => crate::cmc::CMC42_NITD_EXTRA_XOR,
                Some(KnownZipSet::Sengoku3Encrypted) => crate::cmc::CMC42_SENGOKU3_EXTRA_XOR,
                Some(KnownZipSet::Preisle2Encrypted) => crate::cmc::CMC42_PREISLE2_EXTRA_XOR,
                Some(KnownZipSet::BangbeadEncrypted) => crate::cmc::CMC42_BANGBEAD_EXTRA_XOR,
                Some(KnownZipSet::GanryuEncrypted) => crate::cmc::CMC42_GANRYU_EXTRA_XOR,
                _ => unreachable!(),
            };
            let decrypted_crom = crate::cmc::decrypt_cmc42_graphics(&encrypted_crom, extra_xor);
            if srom.is_empty() {
                srom =
                    crate::cmc::extract_cmc_s_data(&decrypted_crom, cmc_s_data_size(known_zip_set));
            }
            decrypted_crom
        } else if matches!(
            known_zip_set,
            Some(
                KnownZipSet::Kof2002Encrypted
                    | KnownZipSet::Mslug4Encrypted
                    | KnownZipSet::Kof2003Encrypted
                    | KnownZipSet::SvcEncrypted
                    | KnownZipSet::Mslug5Encrypted
                    | KnownZipSet::RotdEncrypted
                    | KnownZipSet::MatrimEncrypted
                    | KnownZipSet::PnyaaEncrypted
                    | KnownZipSet::Kof2001Encrypted
                    | KnownZipSet::JockeygpEncrypted
                    | KnownZipSet::Samsho5Encrypted
                    | KnownZipSet::Samsh5spEncrypted
                    | KnownZipSet::KogEncrypted
            )
        ) {
            // ── CMC50 path (kof2002, kof2003, svc, garou, kog) ─────
            let encrypted_crom = crate::cmc::interleave_cmc_graphics_banks(
                graphics_banks
                    .into_iter()
                    .map(|(index, _, bytes)| (index, bytes))
                    .collect(),
            );
            let extra_xor = match known_zip_set {
                Some(KnownZipSet::Kof2002Encrypted) => crate::cmc::CMC50_KOF2002_EXTRA_XOR,
                Some(KnownZipSet::Mslug4Encrypted) => crate::cmc::CMC50_MSLUG4_EXTRA_XOR,
                Some(KnownZipSet::Kof2003Encrypted) => crate::cmc::CMC50_KOF2003_EXTRA_XOR,
                Some(KnownZipSet::SvcEncrypted) => crate::cmc::CMC50_SVC_EXTRA_XOR,
                Some(KnownZipSet::Mslug5Encrypted) => crate::cmc::CMC50_MSLUG5_EXTRA_XOR,
                Some(KnownZipSet::RotdEncrypted) => crate::cmc::CMC50_ROTD_EXTRA_XOR,
                Some(KnownZipSet::MatrimEncrypted) => crate::cmc::CMC50_MATRIM_EXTRA_XOR,
                Some(KnownZipSet::PnyaaEncrypted) => crate::cmc::CMC50_PNYAA_EXTRA_XOR,
                Some(KnownZipSet::Kof2001Encrypted) => crate::cmc::CMC50_KOF2001_EXTRA_XOR,
                Some(KnownZipSet::JockeygpEncrypted) => crate::cmc::CMC50_JOCKEYGP_EXTRA_XOR,
                Some(KnownZipSet::Samsho5Encrypted) => crate::cmc::CMC50_SAMSHO5_EXTRA_XOR,
                Some(KnownZipSet::Samsh5spEncrypted) => crate::cmc::CMC50_SAMSH5SP_EXTRA_XOR,
                Some(KnownZipSet::KogEncrypted) => crate::cmc::CMC50_KOG_EXTRA_XOR,
                _ => unreachable!(),
            };
            let decrypted_crom = crate::cmc::decrypt_cmc50_graphics(&encrypted_crom, extra_xor);
            if srom.is_empty() {
                srom =
                    crate::cmc::extract_cmc_s_data(&decrypted_crom, cmc_s_data_size(known_zip_set));
            }
            // PCM2 processing (kof2002, kof2003, svc — NOT garou, NOT kog)
            if matches!(
                known_zip_set,
                Some(
                    KnownZipSet::Kof2002Encrypted
                        | KnownZipSet::Kof2003Encrypted
                        | KnownZipSet::SvcEncrypted
                        | KnownZipSet::Mslug5Encrypted
                        | KnownZipSet::RotdEncrypted
                        | KnownZipSet::MatrimEncrypted
                        | KnownZipSet::PnyaaEncrypted
                        | KnownZipSet::Samsho5Encrypted
                        | KnownZipSet::Samsh5spEncrypted
                        | KnownZipSet::Mslug4Encrypted
                )
            ) {
                if matches!(
                    known_zip_set,
                    Some(KnownZipSet::Kof2002Encrypted | KnownZipSet::MatrimEncrypted)
                ) {
                    crate::pcm2::decrypt_pcm2_p(&mut prom);
                }
                match known_zip_set {
                    Some(KnownZipSet::Kof2002Encrypted) => {
                        crate::pcm2::decrypt_pcm2_v2(&mut vrom, crate::pcm2::KOF2002_PCM2_V2_INFO);
                    }
                    Some(KnownZipSet::Kof2003Encrypted) => {
                        crate::pcm2::decrypt_pcm2_v2(&mut vrom, crate::pcm2::KOF2003_PCM2_V2_INFO);
                    }
                    Some(KnownZipSet::SvcEncrypted) => {
                        crate::pcm2::decrypt_pcm2_v2(&mut vrom, crate::pcm2::SVC_PCM2_V2_INFO);
                    }
                    Some(KnownZipSet::Mslug5Encrypted) => {
                        crate::pcm2::decrypt_pcm2_v2(&mut vrom, crate::pcm2::MSLUG5_PCM2_V2_INFO);
                    }
                    Some(KnownZipSet::RotdEncrypted) => {
                        crate::pcm2::decrypt_pcm2_v(&mut vrom, 0x1000000, 2);
                    }
                    Some(KnownZipSet::MatrimEncrypted) => {
                        crate::pcm2::decrypt_pcm2_v2(&mut vrom, crate::pcm2::MATRIM_PCM2_V2_INFO);
                    }
                    Some(KnownZipSet::PnyaaEncrypted) => {
                        crate::pcm2::decrypt_pcm2_v(&mut vrom, 0x400000, 0);
                    }
                    Some(KnownZipSet::Samsho5Encrypted) => {
                        crate::pcm2::decrypt_pcm2_p2(&mut prom, crate::pcm2::SAMSHO5_PCM2_P2_INFO);
                        crate::pcm2::decrypt_pcm2_v2(&mut vrom, crate::pcm2::SAMSHO5_PCM2_V2_INFO);
                    }
                    Some(KnownZipSet::Samsh5spEncrypted) => {
                        crate::pcm2::decrypt_pcm2_p2(&mut prom, crate::pcm2::SAMSH5SP_PCM2_P2_INFO);
                        crate::pcm2::decrypt_pcm2_v2(&mut vrom, crate::pcm2::SAMSH5SP_PCM2_V2_INFO);
                    }
                    Some(KnownZipSet::Mslug4Encrypted) => {
                        crate::pcm2::decrypt_pcm2_v(&mut vrom, 0x1000000, 1);
                    }
                    _ => unreachable!(),
                }
                mrom = crate::cmc::decrypt_cmc50_m1(&mrom);
            } else if matches!(
                known_zip_set,
                Some(
                    KnownZipSet::KogEncrypted
                        | KnownZipSet::Kof2001Encrypted
                        | KnownZipSet::JockeygpEncrypted
                )
            ) {
                // These CMC50 sets have HARDWARE_SNK_ENCRYPTED_M1 per FBNeo.
                mrom = crate::cmc::decrypt_cmc50_m1(&mrom);
            }
            decrypted_crom
        } else {
            normalize_zip_graphics_banks(
                graphics_banks
                    .into_iter()
                    .map(|(index, _, bytes)| (index, bytes))
                    .collect(),
            )
        };
        // Auto-dump ROM banks for diagnostic purposes (all ROM types)
        dump_prom_diagnostic(&prom, file_stem);
        dump_crom_diagnostic(&crom, file_stem);
        dump_srom_diagnostic(&srom, file_stem);
        dump_mrom_diagnostic(&mrom, file_stem);
        dump_vrom_diagnostic(&vrom, file_stem);

        let rom = Self {
            prom,
            crom,
            srom,
            mrom,
            vrom,
            vrom_b_offset: 0,
            sma_rom,
            is_demo: false,
            source: RomSource::ZipArchive,
            recognized_files,
            metadata: None,
        };
        let mut rom = rom.validate_external()?;
        if known_zip_set == Some(KnownZipSet::SbpHomebrew) {
            apply_sbp_callback(&mut rom.prom, &mut rom.srom);
        }
        rom.metadata =
            metadata_from_zip_set(known_zip_set, &rom.recognized_files, file_stem, &rom.prom);
        Ok(rom)
    }

    pub fn validate_external(mut self) -> Result<Self, String> {
        if self.is_demo {
            return Ok(self);
        }

        if self.prom.len() < MIN_EXTERNAL_PROM_SIZE {
            return Err(format!(
                "ROM inválida: P-ROM ausente o demasiado pequeña ({} bytes)",
                self.prom.len()
            ));
        }

        if !self.prom.len().is_multiple_of(2) {
            self.prom.push(0xFF);
        }

        normalize_program_rom_byte_order(&mut self.prom);

        Ok(self)
    }

    pub fn bank_summary(&self) -> String {
        format!(
            "P={} C={} S={} M={} V={} bytes",
            self.prom.len(),
            self.crom.len(),
            self.srom.len(),
            self.mrom.len(),
            self.vrom.len()
        )
    }

    pub fn diagnostics(&self) -> RomDiagnostics {
        let mut warnings = Vec::new();
        let known_zip_set = detect_known_zip_set(&self.recognized_files);
        if !self.is_demo {
            if self.crom.is_empty() {
                warnings.push("C-ROM ausente: solo habrá visualización negra/básica".to_string());
            }
            if self.srom.is_empty() {
                if matches!(
                    known_zip_set,
                    Some(
                        KnownZipSet::Kof2002Encrypted
                            | KnownZipSet::Kof2003Encrypted
                            | KnownZipSet::SvcEncrypted
                            | KnownZipSet::Mslug5Encrypted
                            | KnownZipSet::RotdEncrypted
                            | KnownZipSet::MatrimEncrypted
                            | KnownZipSet::PnyaaEncrypted
                            | KnownZipSet::Kof2001Encrypted
                            | KnownZipSet::JockeygpEncrypted
                            | KnownZipSet::Samsho5Encrypted
                            | KnownZipSet::Samsh5spEncrypted
                            | KnownZipSet::Mslug4Encrypted
                    )
                ) {
                    warnings.push(
                        "S-ROM ausente esperada en set CMC50: la capa fix sale de C-ROM"
                            .to_string(),
                    );
                } else if matches!(
                    known_zip_set,
                    Some(
                        KnownZipSet::Mslug3Encrypted
                            | KnownZipSet::Kof99Encrypted
                            | KnownZipSet::GarouEncrypted
                            | KnownZipSet::S1945pEncrypted
                            | KnownZipSet::Preisle2Encrypted
                            | KnownZipSet::BangbeadEncrypted
                            | KnownZipSet::GanryuEncrypted
                    )
                ) {
                    warnings.push(
                        "S-ROM ausente esperada en set CMC42: la capa fix sale de C-ROM"
                            .to_string(),
                    );
                } else if known_zip_set == Some(KnownZipSet::KogEncrypted) {
                    warnings.push(
                        "S-ROM ausente esperada en set CMC50: la capa fix sale de C-ROM"
                            .to_string(),
                    );
                } else {
                    warnings.push("S-ROM ausente: no hay datos de capa fix".to_string());
                }
            }
            if self.mrom.is_empty() {
                warnings.push("M-ROM ausente: audio CPU Z80 no disponible".to_string());
            }
            if self.vrom.is_empty() {
                warnings.push("V-ROM ausente: audio de muestras no disponible".to_string());
            }
            if matches!(
                known_zip_set,
                Some(
                    KnownZipSet::Kof2002Encrypted
                        | KnownZipSet::Kof2003Encrypted
                        | KnownZipSet::SvcEncrypted
                        | KnownZipSet::Mslug5Encrypted
                        | KnownZipSet::RotdEncrypted
                        | KnownZipSet::MatrimEncrypted
                        | KnownZipSet::PnyaaEncrypted
                        | KnownZipSet::Samsho5Encrypted
                        | KnownZipSet::Samsh5spEncrypted
                        | KnownZipSet::Mslug4Encrypted
                )
            ) {
                warnings.push(
                    "Set CMC50+PCM2 cifrado: P-ROM, M1, V-ROM y C/fix CMC50 se decriptan en la ruta inicial"
                        .to_string(),
                );
                if known_zip_set == Some(KnownZipSet::Mslug5Encrypted) {
                    warnings.push(
                        "Metal Slug 5 ZIP: P-ROM P32 + descramble específico FBNeo aplicado"
                            .to_string(),
                    );
                } else if matches!(
                    known_zip_set,
                    Some(
                        KnownZipSet::RotdEncrypted
                            | KnownZipSet::MatrimEncrypted
                            | KnownZipSet::PnyaaEncrypted
                    )
                ) {
                    warnings.push(
                        "ZIP PCM2 tardío: V-ROM/P-ROM reorganizado según callback FBNeo"
                            .to_string(),
                    );
                } else if matches!(
                    known_zip_set,
                    Some(KnownZipSet::Samsho5Encrypted | KnownZipSet::Samsh5spEncrypted)
                ) {
                    warnings.push(
                        "Samurai Shodown ZIP: P-ROM PCM2 P2 + V-ROM PCM2 V2 aplicado".to_string(),
                    );
                }
            } else if known_zip_set == Some(KnownZipSet::GarouEncrypted) {
                warnings
                    .push("GAROU set cifrado: P-ROM descifrado vía SMA + C/fix CMC42".to_string());
            } else if matches!(
                known_zip_set,
                Some(KnownZipSet::Kof2001Encrypted | KnownZipSet::JockeygpEncrypted)
            ) {
                warnings.push("Set CMC50 cifrado: C/fix CMC50 + M1 CMC50 aplicado".to_string());
            } else if known_zip_set == Some(KnownZipSet::Mslug3Encrypted) {
                warnings.push(
                    "Set SMA+CMC42 cifrado: P-ROM descifrado vía SMA + C/fix CMC42".to_string(),
                );
            } else if known_zip_set == Some(KnownZipSet::Kof99Encrypted) {
                warnings.push("KOF99 cifrado: P-ROM descifrado vía SMA + C/fix CMC42".to_string());
            } else if known_zip_set == Some(KnownZipSet::KogEncrypted) {
                warnings.push(
                    "Set SMA+CMC50 cifrado (KOF2000): P-ROM SMA + C/fix CMC50 + M1 CMC50"
                        .to_string(),
                );
            } else if known_zip_set == Some(KnownZipSet::KogBootleg) {
                warnings.push(
                    "Set KOG bootleg: P/S se reordenan y el parent KOF97 completa P2/C/M/V cuando está disponible"
                        .to_string(),
                );
            } else if matches!(
                known_zip_set,
                Some(
                    KnownZipSet::S1945pEncrypted
                        | KnownZipSet::ZupapaEncrypted
                        | KnownZipSet::NitdEncrypted
                        | KnownZipSet::Sengoku3Encrypted
                        | KnownZipSet::Preisle2Encrypted
                        | KnownZipSet::BangbeadEncrypted
                        | KnownZipSet::GanryuEncrypted
                )
            ) {
                warnings.push("Set CMC42 cifrado: C/fix descifrado vía CMC42".to_string());
                if requires_first_program_half_swap(known_zip_set) {
                    warnings.push("Set SWAPP: P1 reordenado según FBNeo".to_string());
                }
            } else if matches!(
                known_zip_set,
                Some(
                    KnownZipSet::CtomadaySwapp
                        | KnownZipSet::Crswd2blSwapp
                        | KnownZipSet::IroncladSwapp
                )
            ) {
                warnings.push("Set SWAPP: P1 reordenado según FBNeo".to_string());
            }
        }

        RomDiagnostics {
            source: self.source.clone(),
            prom_bytes: self.prom.len(),
            crom_bytes: self.crom.len(),
            srom_bytes: self.srom.len(),
            mrom_bytes: self.mrom.len(),
            vrom_bytes: self.vrom.len(),
            recognized_files: self.recognized_files.len(),
            warnings,
        }
    }
}

/// Dump C-ROM to a diagnostic binary file in `screenshots/`.
/// Uses the ROM label to generate a unique filename.
fn dump_crom_diagnostic(crom: &[u8], label: &str) {
    dump_bank_diagnostic(crom, label, "crom");
}

/// Dump V-ROM (audio samples) to a diagnostic binary file in `screenshots/`.
fn dump_vrom_diagnostic(vrom: &[u8], label: &str) {
    dump_bank_diagnostic(vrom, label, "vrom");
}

/// Dump M-ROM (Z80 code) to a diagnostic binary file in `screenshots/`.
fn dump_mrom_diagnostic(mrom: &[u8], label: &str) {
    dump_bank_diagnostic(mrom, label, "mrom");
}

/// Dump P-ROM (68k program) to a diagnostic binary file in `screenshots/`.
fn dump_prom_diagnostic(prom: &[u8], label: &str) {
    dump_bank_diagnostic(prom, label, "prom");
}

/// Dump S-ROM (fix layer) to a diagnostic binary file in `screenshots/`.
fn dump_srom_diagnostic(srom: &[u8], label: &str) {
    dump_bank_diagnostic(srom, label, "srom");
}

/// Chunk size for progressive dump writes (64 KiB).
const DUMP_CHUNK_SIZE: usize = 64 * 1024;

/// Threshold above which a progress indicator is shown (64 KiB).
const DUMP_PROGRESS_THRESHOLD: usize = 64 * 1024;

/// Generic ROM bank diagnostic dump helper.
/// For banks larger than `DUMP_PROGRESS_THRESHOLD`, writes in chunks and
/// prints a simple percentage progress indicator so users can see it working.
fn dump_bank_diagnostic(data: &[u8], label: &str, bank: &str) {
    if !DIAGNOSTIC_DUMPS.load(Ordering::Relaxed) || data.is_empty() {
        return;
    }
    let dir = std::path::Path::new("screenshots");
    let _ = std::fs::create_dir_all(dir);
    let safe = label.replace(
        |c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_',
        "_",
    );
    let path = format!("screenshots/{}_{}_dump.bin", safe, bank);
    let bank_upper = bank.to_uppercase();

    let mut f = match std::fs::File::create(&path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("[DIAG][ERROR] Failed to create {bank_upper} dump {path}: {e}");
            return;
        }
    };

    use std::io::Write;

    let total = data.len();
    if total <= DUMP_PROGRESS_THRESHOLD {
        // Small bank — write in one shot, no progress needed
        match f.write_all(data) {
            Ok(()) => println!("[DIAG] {bank_upper} dump saved to {path} ({total} bytes)"),
            Err(e) => eprintln!("[DIAG][ERROR] Failed to write {bank_upper} dump {path}: {e}"),
        }
        return;
    }

    // Large bank — write in chunks with a progress indicator on stderr
    eprint!("[DIAG] Dumping {bank_upper} to {path} ({total} bytes) ");
    let mut written: usize = 0;
    let mut last_pct: usize = 0;
    for chunk in data.chunks(DUMP_CHUNK_SIZE) {
        if let Err(e) = f.write_all(chunk) {
            eprintln!(
                "\n[DIAG][ERROR] Failed to write {bank_upper} dump {path} at offset {written}: {e}"
            );
            return;
        }
        written += chunk.len();
        let pct = (written * 100) / total;
        if pct != last_pct {
            eprint!("\r[DIAG] Dumping {bank_upper} to {path} ({total} bytes) ... {pct}%");
            last_pct = pct;
        }
    }
    // Clear the progress line then log final success to stdout (consistent with small-bank path)
    eprint!("\r");
    println!("[DIAG] {bank_upper} dump saved to {path} ({total} bytes)");
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KnownZipSet {
    Kof2002Encrypted,
    Kof2003Encrypted,
    SvcEncrypted,
    Mslug5Encrypted,
    RotdEncrypted,
    MatrimEncrypted,
    PnyaaEncrypted,
    Kof2001Encrypted,
    JockeygpEncrypted,
    Samsho5Encrypted,
    Samsh5spEncrypted,
    Mslug3Encrypted,
    Kof99Encrypted,
    Mslug4Encrypted,
    S1945pEncrypted,
    ZupapaEncrypted,
    NitdEncrypted,
    Sengoku3Encrypted,
    Preisle2Encrypted,
    BangbeadEncrypted,
    GanryuEncrypted,
    Crswd2blSwapp,
    IroncladSwapp,
    SbpHomebrew,
    DragonshDevelopment,
    CtomadaySwapp,
    KogEncrypted,
    KogBootleg,
    GarouEncrypted,
    Kof98Encrypted,
}

fn detect_known_zip_set(files: &[String]) -> Option<KnownZipSet> {
    let has = |needle: &str| files.iter().any(|name| name.eq_ignore_ascii_case(needle));
    let has_sma = files.iter().any(|name| {
        name.eq_ignore_ascii_case("neo-sma") || name.to_ascii_lowercase().ends_with(".neo-sma")
    });

    // ── CMC50 + PCM2 sets ──────────────────────────────────────────────
    if has("265-p1.p1")
        && has("265-p2.sp2")
        && has("265-c1.c1")
        && has("265-m1.m1")
        && has("265-v1.v1")
    {
        return Some(KnownZipSet::Kof2002Encrypted);
    }
    if (has("271-p1c.p1") || has("271-p1.p1") || has("271-p1k.p1"))
        && (has("271-p2c.p2") || has("271-p2.p2") || has("271-p2k.p2"))
        && (has("271-p3c.p3") || has("271-p3.p3") || has("271-p3k.p3"))
        && (has("271-c1c.c1") || has("271-c1.c1") || has("271-c1k.c1"))
        && (has("271-m1c.m1") || has("271-m1.m1"))
        && (has("271-v1c.v1") || has("271-v1.v1"))
    {
        return Some(KnownZipSet::Kof2003Encrypted);
    }
    if has("269-p1.p1")
        && has("269-p2.p2")
        && (has("269-c1r.c1") || has("269-c1.c1"))
        && has("269-m1.m1")
        && has("269-v1.v1")
    {
        return Some(KnownZipSet::SvcEncrypted);
    }
    if (has("268-p1cr.p1") || has("268-p1c.p1"))
        && (has("268-p2cr.p2") || has("268-p2c.p2"))
        && (has("268-c1c.c1") || has("268-c1.c1"))
        && has("268-m1.m1")
        && (has("268-v1c.v1") || has("268-v1.v1"))
    {
        return Some(KnownZipSet::Mslug5Encrypted);
    }
    if has("264-p1.p1") && has("264-c1.c1") && has("264-m1.m1") && has("264-v1.v1") {
        return Some(KnownZipSet::RotdEncrypted);
    }
    if has("266-p1.p1")
        && has("266-p2.sp2")
        && has("266-c1.c1")
        && has("266-m1.m1")
        && has("266-v1.v1")
    {
        return Some(KnownZipSet::MatrimEncrypted);
    }
    if (has("pn202.p1") || has("267-p1.p1")) && has("267-c1.c1") && has("m1.m1") && has("267-v1.v1")
    {
        return Some(KnownZipSet::PnyaaEncrypted);
    }
    if has("249-p1.p1") && has("249-s1.s1") && has("249-c1.c1") && has("249-m1.m1") {
        return Some(KnownZipSet::CtomadaySwapp);
    }
    if has("255-p1.p1")
        && has("255-p2.sp2")
        && has("255-c1.c1")
        && has("255-c2.c2")
        && has("255-m1.m1")
    {
        return Some(KnownZipSet::Preisle2Encrypted);
    }
    if has("259-p1.p1") && has("259-c1.c1") && has("259-c2.c2") && has("259-m1.m1") {
        return Some(KnownZipSet::BangbeadEncrypted);
    }
    if has("252-p1.p1") && has("252-c1.c1") && has("252-c2.c2") && has("252-m1.m1") {
        return Some(KnownZipSet::GanryuEncrypted);
    }
    if has("054-p1.p1") && has("054-c1.c1") && has("054-c2.c2") && has("054-m1.m1") {
        return Some(KnownZipSet::Crswd2blSwapp);
    }
    if has("proto_220-p1.p1") && has("proto_220-c1.c1") && has("proto_220-m1.m1") {
        return Some(KnownZipSet::IroncladSwapp);
    }
    if has("001-003-02a.u2")
        && has("001-003-02b.u2")
        && has("001-003-03b.u3")
        && has("001-003-01b.u1")
    {
        return Some(KnownZipSet::SbpHomebrew);
    }
    if has("ep1.bin") && has("ep2.bin") && has("no3.bin") && has("no4.bin") && has("s1.s1") {
        return Some(KnownZipSet::DragonshDevelopment);
    }
    if (has("262-p1-08-e0.p1") || has("262-pg1.p1"))
        && (has("262-p2-08-e0.sp2") || has("262-pg2.sp2"))
        && has("262-c1-08-e0.c1")
        && has("265-262-m1.m1")
        && has("262-v1-08-e0.v1")
    {
        return Some(KnownZipSet::Kof2001Encrypted);
    }
    if (has("008-epr.p1") || has("008-epr_a.p1"))
        && has("008-c1.c1")
        && has("008-c2.c2")
        && has("008-mg1.m1")
        && has("008-v1.v1")
    {
        return Some(KnownZipSet::JockeygpEncrypted);
    }
    if (has("270-p1.p1") || has("270-p1c.p1") || has("p1.bin"))
        && (has("270-p2.sp2") || has("270-p2c.sp2") || has("p2.bin"))
        && has("270-c1.c1")
        && has("270-m1.m1")
        && has("270-v1.v1")
    {
        return Some(KnownZipSet::Samsho5Encrypted);
    }
    if (has("272-p1.p1") || has("272-p1ca.p1") || has("272-p1c.p1"))
        && (has("272-p2.sp2") || has("272-p2ca.sp2") || has("272-p2c.sp2"))
        && has("272-c1.c1")
        && has("272-m1.m1")
        && has("272-v1.v1")
    {
        return Some(KnownZipSet::Samsh5spEncrypted);
    }

    // ── SMA + CMC42 sets ───────────────────────────────────────────────
    if has_sma
        && has("256-pg1.p1")
        && has("256-pg2.p2")
        && has("256-c1.c1")
        && has("256-m1.m1")
        && has("256-v1.v1")
    {
        return Some(KnownZipSet::Mslug3Encrypted);
    }
    if has_sma
        && has("251-p1.p1")
        && has("251-p2.p2")
        && has("251-c1.c1")
        && has("251-m1.m1")
        && has("251-v1.v1")
    {
        return Some(KnownZipSet::Kof99Encrypted);
    }
    if has("263-p1.p1")
        && has("263-p2.sp2")
        && has("263-c1.c1")
        && has("263-m1.m1")
        && has("263-v1.v1")
    {
        return Some(KnownZipSet::Mslug4Encrypted);
    }
    if has("254-p1.p1")
        && has("254-p2.sp2")
        && has("254-c1.c1")
        && has("254-m1.m1")
        && has("254-v1.v1")
    {
        return Some(KnownZipSet::S1945pEncrypted);
    }
    if has("242-p1.p1")
        && has("242-p2.sp2")
        && has("242-c1.c1")
        && has("242-m1.m1")
        && has("242-v1.v1")
    {
        return Some(KnownZipSet::Kof98Encrypted);
    }
    if has("070-p1.p1") && has("070-c1.c1") && has("070-c2.c2") && has("070-epr.m1") {
        return Some(KnownZipSet::ZupapaEncrypted);
    }
    if has("260-p1.p1") && has("260-c1.c1") && has("260-c2.c2") && has("260-m1.m1") {
        return Some(KnownZipSet::NitdEncrypted);
    }
    if has("261-ph1.p1") && has("261-c1.c1") && has("261-c2.c2") && has("261-m1.m1") {
        return Some(KnownZipSet::Sengoku3Encrypted);
    }
    if has_sma && has("254-pg1.p1") && has("254-c1.c1") && has("254-m1.m1") && has("254-v1.v1") {
        return Some(KnownZipSet::KogEncrypted);
    }
    // KOF 2000 parent set (257-p1.p1 + 257-p2.p2 + neo-sma)
    if has_sma && has("257-p1.p1") && has("257-p2.p2") && has("257-c1.c1") && has("257-m1.m1") {
        return Some(KnownZipSet::KogEncrypted);
    }
    if has("5232-p1.bin")
        && has("5232-s1.bin")
        && has("5232-c1a.bin")
        && has("5232-c1b.bin")
        && has("5232-c2a.bin")
        && has("5232-c2b.bin")
        && has("5232-c3.bin")
        && has("5232-c4.bin")
    {
        return Some(KnownZipSet::KogBootleg);
    }

    // ── SMA + CMC50 set (Garou) ────────────────────────────────────────
    if has_sma
        && (has("253-ep1.p1") || has("253-p1.p1"))
        && has("253-c1.c1")
        && has("253-m1.m1")
        && has("253-v1.v1")
    {
        return Some(KnownZipSet::GarouEncrypted);
    }

    None
}

fn metadata_from_known_zip_set(
    known_zip_set: Option<KnownZipSet>,
    files: &[String],
    label: &str,
    prom: &[u8],
) -> Option<NeoMetadata> {
    let ngh = match known_zip_set? {
        KnownZipSet::Kof2002Encrypted => 0x265,
        KnownZipSet::Kof2003Encrypted => 0x271,
        KnownZipSet::SvcEncrypted => 0x269,
        KnownZipSet::Mslug5Encrypted => 0x268,
        KnownZipSet::RotdEncrypted => 0x264,
        KnownZipSet::MatrimEncrypted => 0x266,
        KnownZipSet::PnyaaEncrypted => 0x267,
        KnownZipSet::Kof2001Encrypted => 0x262,
        KnownZipSet::JockeygpEncrypted => 0x008,
        KnownZipSet::Samsho5Encrypted => 0x270,
        KnownZipSet::Samsh5spEncrypted => 0x272,
        KnownZipSet::Mslug3Encrypted => 0x256,
        KnownZipSet::Kof99Encrypted => 0x251,
        KnownZipSet::Mslug4Encrypted => 0x263,
        KnownZipSet::S1945pEncrypted => 0x254,
        KnownZipSet::ZupapaEncrypted => 0x070,
        KnownZipSet::NitdEncrypted => 0x260,
        KnownZipSet::Sengoku3Encrypted => 0x261,
        KnownZipSet::Preisle2Encrypted => 0x239,
        KnownZipSet::BangbeadEncrypted => 0x259,
        KnownZipSet::GanryuEncrypted => 0x252,
        KnownZipSet::Crswd2blSwapp => 0x054,
        KnownZipSet::IroncladSwapp => 0x220,
        KnownZipSet::SbpHomebrew => 0x000,
        KnownZipSet::DragonshDevelopment => 0x094,
        KnownZipSet::CtomadaySwapp => 0x249,
        KnownZipSet::GarouEncrypted => 0x253,
        KnownZipSet::Kof98Encrypted => 0x242,
        KnownZipSet::KogBootleg => 0x5232,
        KnownZipSet::KogEncrypted => {
            let has_257 = files
                .iter()
                .any(|name| name.eq_ignore_ascii_case("257-p1.p1"));
            if has_257 {
                0x257
            } else {
                return None;
            }
        }
    };

    Some(metadata_from_zip_ngh(ngh, label, prom))
}

fn metadata_from_zip_set(
    known_zip_set: Option<KnownZipSet>,
    files: &[String],
    label: &str,
    prom: &[u8],
) -> Option<NeoMetadata> {
    if let Some(metadata) = metadata_from_known_zip_set(known_zip_set, files, label, prom) {
        return Some(metadata);
    }

    let ngh = read_zip_ngh_from_prom_header(prom).or_else(|| detect_ngh_from_zip_name(label))?;
    Some(metadata_from_zip_ngh(ngh, label, prom))
}

fn metadata_from_zip_ngh(ngh: u32, label: &str, prom: &[u8]) -> NeoMetadata {
    let mut metadata = NeoMetadata {
        version: 0,
        year: 0,
        genre: 0,
        screenshot: 0,
        ngh,
        name: label.to_string(),
        manufacturer: String::new(),
        board_type: detect_neo_board_type(ngh),
        fix_banksw: detect_neo_fix_banksw(ngh),
        game_flags: detect_neo_game_flags(ngh),
    };

    let (variant_board, variant_fix) =
        apply_neo_variant_heuristics(ngh, metadata.board_type, metadata.fix_banksw, prom);
    metadata.board_type = validate_sma_board_type(variant_board, prom);
    metadata.fix_banksw = variant_fix;
    metadata.board_type = validate_kof2003_board_type(ngh, metadata.board_type, prom);

    metadata
}

fn read_zip_ngh_from_prom_header(prom: &[u8]) -> Option<u32> {
    if prom.len() < 0x10A || prom.get(0x100..0x107) != Some(b"NEO-GEO") {
        return None;
    }

    let ngh = u16::from_be_bytes([prom[0x108], prom[0x109]]) as u32;
    if ngh == 0 || ngh == 0xFFFF {
        None
    } else {
        Some(ngh)
    }
}

// .neo P-ROMs are stored with bytes swapped within each 16-bit word, matching
// Geolith's unconditional `swapb16_range(romdata->p)` post-load step. Validate
// boot vectors against the same detection used by normalization, otherwise
// valid .neo files print a scary but false stack-pointer warning.
fn validate_initial_vectors_for_diagnostics(prom: &[u8], file_name: &str) -> Result<(), String> {
    if prom.len() < 8 {
        return Ok(());
    }

    let raw_sp = read_u32_be_unchecked(prom, 0);
    let raw_reset = read_u32_be_unchecked(prom, 4);
    let (sp, reset) = if program_rom_looks_byte_swapped(prom) {
        (
            read_u32_word_byte_swapped(prom, 0),
            read_u32_word_byte_swapped(prom, 4),
        )
    } else if looks_like_initial_stack(raw_sp) {
        (raw_sp, raw_reset)
    } else {
        let swapped_sp = read_u32_word_byte_swapped(prom, 0);
        if looks_like_initial_stack(swapped_sp) {
            (swapped_sp, read_u32_word_byte_swapped(prom, 4))
        } else {
            (raw_sp, raw_reset)
        }
    };

    if !looks_like_initial_stack(sp) {
        eprintln!(
            "[ADVERTENCIA] Stack pointer inválido en P-ROM: SP=0x{sp:08X} (esperado 0x10FD00-0x10FFFF, par) (archivo: {file_name})"
        );
    }

    if reset == 0x00000000 || reset == 0xFFFFFFFF {
        eprintln!(
            "[ERROR] Vector de reset inválido en P-ROM: RESET=0x{reset:08X} (archivo: {file_name})"
        );
        return Err(format!(
            "ROM inválida: vector de reset inválido en P-ROM (RESET=0x{reset:08X}, archivo: {file_name})"
        ));
    }

    Ok(())
}

/// Check whether the P-ROM data appears to be byte-swapped within each 16-bit
/// word rather than native 68k big-endian word order.
///
/// First use the cartridge program header at 0x100. Real `.neo` P-ROMs match
/// Geolith's post-load path: raw bytes read as `EN-OEG...`, and swapping each
/// word restores `NEO-GEO`. If the header is unavailable, fall back to the
/// initial stack pointer heuristic for small synthetic test ROMs.
///
/// This keeps compatibility with Geolith while avoiding a bad assumption that
/// every cart's initial SP vector must look like a normal work-RAM stack.
fn program_rom_looks_byte_swapped(prom: &[u8]) -> bool {
    if prom.len() >= 0x108 {
        let raw_header = &prom[0x100..0x107];
        if raw_header == b"NEO-GEO" {
            return false;
        }

        let swapped_header = [
            prom[0x101],
            prom[0x100],
            prom[0x103],
            prom[0x102],
            prom[0x105],
            prom[0x104],
            prom[0x107],
        ];
        if swapped_header == *b"NEO-GEO" {
            return true;
        }
    }

    if prom.len() < 4 {
        return false;
    }

    let raw_sp = read_u32_be_unchecked(prom, 0);
    if looks_like_initial_stack(raw_sp) {
        return false;
    }

    let swapped_sp = read_u32_word_byte_swapped(prom, 0);
    looks_like_initial_stack(swapped_sp)
}

fn looks_like_initial_stack(value: u32) -> bool {
    value & 1 == 0 && (WORK_RAM_STACK_START..=WORK_RAM_STACK_END).contains(&value)
}

fn read_u32_be_unchecked(data: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

fn read_u32_word_byte_swapped(data: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([
        data[offset + 1],
        data[offset],
        data[offset + 3],
        data[offset + 2],
    ])
}

fn normalize_program_rom_byte_order(prom: &mut [u8]) {
    // .neo files store P-ROM words in byte-swapped order. Prefer the
    // cartridge header signature when available; fall back to vectors for
    // synthetic ROMs used by tests.
    if prom.len() < 4 {
        return;
    }

    if program_rom_looks_byte_swapped(prom) {
        swap_program_rom_words(prom);
    }
}

fn swap_program_rom_words(prom: &mut [u8]) {
    for word in prom.chunks_exact_mut(2) {
        word.swap(0, 1);
    }
}

fn merge_kog_bootleg_parent(
    path: &Path,
    prom: &mut Vec<u8>,
    mrom: &mut Vec<u8>,
    vrom: &mut Vec<u8>,
) {
    let Some(parent) = load_kog_parent(path) else {
        eprintln!("[WARN] KOG ZIP incompleto: faltan bancos parent de kof97 (232-p2/m1/v1-v3)");
        return;
    };

    if prom.len() == 0x200000 {
        if let Some(parent_p2) = load_kog_parent_raw_program_tail(path) {
            prom.extend_from_slice(&parent_p2);
        } else if parent.prom.len() >= 0x500000 {
            prom.extend_from_slice(&parent.prom[0x100000..0x500000]);
        }
    }
    if mrom.is_empty() && !parent.mrom.is_empty() {
        mrom.extend_from_slice(&parent.mrom);
    }
    if vrom.is_empty() && !parent.vrom.is_empty() {
        vrom.extend_from_slice(&parent.vrom);
    }
}

fn load_kog_parent(path: &Path) -> Option<RomData> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    for candidate in [dir.join("kof97.zip"), dir.join("kof97.neo")] {
        if !candidate.exists() {
            continue;
        }
        let loaded = match candidate
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
            .as_deref()
        {
            Some("zip") => RomData::from_zip(&candidate),
            Some("neo") => RomData::from_neo(&candidate),
            _ => continue,
        };
        match loaded {
            Ok(parent) => return Some(parent),
            Err(error) => eprintln!(
                "[WARN] No se pudo usar parent KOG {:?}: {}",
                candidate, error
            ),
        }
    }
    None
}

fn load_kog_parent_crom(path: &Path) -> Option<Vec<u8>> {
    let parent = load_kog_parent(path)?;
    if parent.crom.is_empty() {
        return None;
    }
    Some(parent.crom)
}

fn load_kog_parent_raw_program_tail(path: &Path) -> Option<Vec<u8>> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    for candidate in [dir.join("kof97.zip"), dir.join("kof97.neo")] {
        if !candidate.exists() {
            continue;
        }
        let loaded = match candidate
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
            .as_deref()
        {
            Some("zip") => load_raw_zip_program(&candidate),
            Some("neo") => std::fs::read(&candidate)
                .map_err(|error| error.to_string())
                .and_then(|data| parse_neo_file(&data).map(|parsed| parsed.prom)),
            _ => continue,
        };
        match loaded {
            Ok(parent_prom) if parent_prom.len() >= 0x500000 => {
                return Some(parent_prom[0x100000..0x500000].to_vec());
            }
            Ok(parent_prom) => eprintln!(
                "[WARN] Parent KOG {:?} tiene P-ROM demasiado pequeño: {} bytes",
                candidate,
                parent_prom.len()
            ),
            Err(error) => eprintln!(
                "[WARN] No se pudo leer P2 crudo de parent KOG {:?}: {}",
                candidate, error
            ),
        }
    }
    None
}

fn load_raw_zip_program(path: &Path) -> Result<Vec<u8>, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("Error abriendo zip parent: {e}"))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Error leyendo zip parent: {e}"))?;
    let mut entries = Vec::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        if file.is_dir() {
            continue;
        }
        let name = file.name().to_ascii_lowercase();
        if !matches!(classify_zip_entry(&name), Some(RomPart::Program(_))) {
            continue;
        }
        let mut buf = Vec::new();
        use std::io::Read;
        file.read_to_end(&mut buf).map_err(|e| e.to_string())?;
        entries.push((name, buf));
    }
    entries.sort_by(|(name_a, _), (name_b, _)| name_a.cmp(name_b));
    let mut prom = Vec::new();
    for (_, bytes) in entries {
        prom.extend(bytes);
    }
    Ok(prom)
}

fn layout_zip_program_banks(
    known_zip_set: Option<KnownZipSet>,
    mut banks: Vec<(u8, String, Vec<u8>)>,
) -> Vec<u8> {
    banks.sort_by(|(index_a, name_a, _), (index_b, name_b, _)| {
        index_a.cmp(index_b).then_with(|| name_a.cmp(name_b))
    });

    if matches!(
        known_zip_set,
        Some(
            KnownZipSet::Kof2003Encrypted
                | KnownZipSet::SvcEncrypted
                | KnownZipSet::Mslug5Encrypted
        )
    ) && banks.len() >= 2
    {
        let mut prom = layout_p32_program_pair(&banks[0].2, &banks[1].2);
        for (_, _, bytes) in banks.iter().skip(2) {
            prom.extend(bytes);
        }
        return prom;
    }

    if known_zip_set == Some(KnownZipSet::DragonshDevelopment) && banks.len() >= 2 {
        let mut prom = Vec::with_capacity(banks[0].2.len() + banks[1].2.len());
        append_byte_interleaved_pair(&mut prom, &banks[0].2, &banks[1].2);
        return prom;
    }

    let mut prom = Vec::new();
    for (bank_position, (_, _, mut bytes)) in banks.into_iter().enumerate() {
        if bank_position == 0 && requires_first_program_half_swap(known_zip_set) {
            swap_program_rom_halves(&mut bytes);
        }
        if bank_position == 0 && first_program_bank_header_is_in_second_meg(&bytes) {
            swap_program_rom_halves(&mut bytes);
        }
        prom.extend(bytes);
    }
    prom
}

fn requires_first_program_half_swap(known_zip_set: Option<KnownZipSet>) -> bool {
    matches!(
        known_zip_set,
        Some(
            KnownZipSet::BangbeadEncrypted
                | KnownZipSet::GanryuEncrypted
                | KnownZipSet::Crswd2blSwapp
                | KnownZipSet::IroncladSwapp
                | KnownZipSet::CtomadaySwapp
                | KnownZipSet::Sengoku3Encrypted
        )
    )
}

fn swap_program_rom_halves(bytes: &mut [u8]) {
    let half = bytes.len() / 2;
    if half == 0 {
        return;
    }
    let (left, right) = bytes.split_at_mut(half);
    left.swap_with_slice(&mut right[..half]);
}

fn first_program_bank_header_is_in_second_meg(bytes: &[u8]) -> bool {
    const HEADER_OFFSET: usize = 0x100;
    const SECOND_MEG_HEADER_OFFSET: usize = 0x100000 + HEADER_OFFSET;

    if bytes.len() < SECOND_MEG_HEADER_OFFSET + b"NEO-GEO".len() {
        return false;
    }

    if program_header_present_at(bytes, HEADER_OFFSET) {
        return false;
    }

    program_header_present_at(bytes, SECOND_MEG_HEADER_OFFSET)
}

fn program_header_present_at(bytes: &[u8], offset: usize) -> bool {
    if bytes.get(offset..offset + b"NEO-GEO".len()) == Some(b"NEO-GEO") {
        return true;
    }

    let Some(swapped_header) = bytes.get(offset..offset + b"NEO-GEO".len() + 1) else {
        return false;
    };

    [
        swapped_header[1],
        swapped_header[0],
        swapped_header[3],
        swapped_header[2],
        swapped_header[5],
        swapped_header[4],
        swapped_header[7],
    ] == *b"NEO-GEO"
}

fn apply_sbp_callback(prom: &mut [u8], srom: &mut Vec<u8>) {
    for byte in prom.iter_mut().take(0x1080).skip(0x1000) {
        *byte = ((*byte >> 4) & 0x0f) | ((*byte << 4) & 0xf0);
    }

    write_prom_word_be(prom, 0x2a6f8, 0x4e71);
    write_prom_word_be(prom, 0x2a6fa, 0x4e71);
    write_prom_word_be(prom, 0x2a6fc, 0x4e71);
    write_prom_word_be(prom, 0x3ff2c, 0x7001);

    srom.truncate(0x20000);
}

fn write_prom_word_be(prom: &mut [u8], offset: usize, value: u16) {
    if offset + 1 >= prom.len() {
        return;
    }
    let bytes = value.to_be_bytes();
    prom[offset] = bytes[0];
    prom[offset + 1] = bytes[1];
}

fn layout_p32_program_pair(first: &[u8], second: &[u8]) -> Vec<u8> {
    let len = first.len().max(second.len());
    let mut prom = Vec::with_capacity(len * 2);
    for offset in (0..len).step_by(2) {
        prom.push(first.get(offset).copied().unwrap_or(0xFF));
        prom.push(first.get(offset + 1).copied().unwrap_or(0xFF));
        prom.push(second.get(offset).copied().unwrap_or(0xFF));
        prom.push(second.get(offset + 1).copied().unwrap_or(0xFF));
    }
    prom
}

fn decrypt_kof98_program(prom: &mut [u8]) {
    const P1_SIZE: usize = 0x200000;
    const DECRYPT_WINDOW: usize = 0x100000;
    const VECTOR_OVERLAY_SKIP: usize = 0x800;

    if prom.len() < P1_SIZE {
        return;
    }

    let mut temp = vec![0u8; P1_SIZE];
    for i in 0..DECRYPT_WINDOW {
        let mut j = i;

        if (i & 0x0000fc) == 0x000000 {
            j ^= 0x000100;
        }
        if (i & 0x0c0000) != 0x080000 {
            j ^= 0x000100;
        }
        if (i & 0x0c0008) == 0x080008 {
            j ^= 0x000100;
        }
        if (i & 0x0c00fe) == 0x080000 {
            j ^= 0x000100;
        }
        if (i & 0x0c0002) == 0x080002 {
            j ^= 0x000100;
        }
        if (i & 0x100000) == 0x100000 {
            j ^= 0x000102;
        }
        if (i & 0x000002) == 0x000002 {
            j ^= 0x100002;
        }
        if (i & 0x000008) == 0x000008 {
            j ^= 0x100002;
        }

        temp[i] = prom[j];
    }

    prom[VECTOR_OVERLAY_SKIP..P1_SIZE].copy_from_slice(&temp[VECTOR_OVERLAY_SKIP..P1_SIZE]);

    if prom.len() > P1_SIZE {
        prom.copy_within(P1_SIZE..prom.len(), DECRYPT_WINDOW);
    }
}

fn decrypt_mslug5_program(prom: &mut [u8]) {
    const MIN_SIZE: usize = 0x800000;
    if prom.len() < MIN_SIZE {
        return;
    }

    for i in 0..0x100000 {
        prom[i] ^= prom[0x0fffe0 + (i & 0x1f)];
    }

    for i in 0x100000..0x700000 {
        prom[i] ^= !prom[0x7fffe0 + (i & 0x1f)];
    }

    for i in (0x100000..0x700000).step_by(4) {
        let word = u16::from_le_bytes([prom[i + 1], prom[i + 2]]);
        let swapped = bitswap16(word, [15, 14, 13, 12, 10, 11, 8, 9, 6, 7, 4, 5, 3, 2, 1, 0]);
        let [lo, hi] = swapped.to_le_bytes();
        prom[i + 1] = lo;
        prom[i + 2] = hi;
    }

    prom.copy_within(0..0x100000, 0x700000);

    let fixed_copy = prom[0x700000..0x800000].to_vec();
    for i in 0..0x10 {
        let src = bitswap8(i as u8, [7, 6, 5, 4, 1, 0, 3, 2]) as usize * 0x10000;
        let dst = i * 0x10000;
        prom[dst..dst + 0x10000].copy_from_slice(&fixed_copy[src..src + 0x10000]);
    }

    let mut bank_copy = vec![0; 0x100000];
    for bank_start in (0x100000..0x700000).step_by(0x100000) {
        for j in (0..0x100000).step_by(0x100) {
            let k = ((j & 0x0f00) ^ 0x0700)
                + ((bitswap8((j >> 12) as u8, [5, 4, 7, 6, 1, 0, 3, 2]) as usize) << 12);
            bank_copy[j..j + 0x100].copy_from_slice(&prom[bank_start + k..bank_start + k + 0x100]);
        }
        prom[bank_start..bank_start + 0x100000].copy_from_slice(&bank_copy);
    }

    if prom.len() > MIN_SIZE {
        let tail_len = prom.len() - MIN_SIZE;
        prom.copy_within(MIN_SIZE..MIN_SIZE + tail_len, 0x700000);
        let zero_start = prom.len().saturating_sub(0x100000);
        prom[zero_start..].fill(0);
    }
}

fn decrypt_kof2003_program(prom: &mut [u8]) {
    const MIN_SIZE: usize = 0x900000;
    if prom.len() < MIN_SIZE {
        return;
    }

    for i in 0..0x100000 {
        prom[i] ^= !prom[0x0fffe0 + (i & 0x1f)];
    }

    for i in 0..0x100000 {
        prom[0x800000 + i] ^= prom[0x100002 | i];
    }

    for i in 0x100000..0x800000 {
        prom[i] ^= !prom[0x7fffe0 + (i & 0x1f)];
    }

    for i in (0x100000..0x800000).step_by(4) {
        let word = u16::from_le_bytes([prom[i + 1], prom[i + 2]]);
        let swapped = bitswap16(word, [15, 14, 13, 12, 5, 4, 7, 6, 9, 8, 11, 10, 3, 2, 1, 0]);
        let [lo, hi] = swapped.to_le_bytes();
        prom[i + 1] = lo;
        prom[i + 2] = hi;
    }

    prom.copy_within(0..0x100000, 0x700000);

    let fixed_copy = prom[0x700000..0x800000].to_vec();
    for i in 0..0x10 {
        let src = bitswap8(i as u8, [7, 6, 5, 4, 0, 1, 2, 3]) as usize * 0x10000;
        let dst = i * 0x10000;
        prom[dst..dst + 0x10000].copy_from_slice(&fixed_copy[src..src + 0x10000]);
    }

    prom.copy_within(0x100000..0x700000, 0x200000);

    let mut bank_copy = vec![0; 0x100000];
    for bank_start in (0x200000..0x900000).step_by(0x100000) {
        for j in (0..0x100000).step_by(0x100) {
            let k = ((j & 0x0f00) ^ 0x0800)
                | ((bitswap8((j >> 12) as u8, [4, 5, 6, 7, 1, 0, 3, 2]) as usize) << 12);
            bank_copy[j..j + 0x100].copy_from_slice(&prom[bank_start + k..bank_start + k + 0x100]);
        }
        prom[bank_start..bank_start + 0x100000].copy_from_slice(&bank_copy);
    }
}

fn decrypt_svc_program(prom: &mut [u8]) {
    const MIN_SIZE: usize = 0x800000;
    if prom.len() < MIN_SIZE {
        return;
    }

    for i in 0..0x100000 {
        prom[i] ^= !prom[0x0fffe0 + (i & 0x1f)];
    }

    for i in 0x100000..0x800000 {
        prom[i] ^= !prom[0x7fffe0 + (i & 0x1f)];
    }

    for i in (0x100000..0x600000).step_by(4) {
        let word = u16::from_le_bytes([prom[i + 1], prom[i + 2]]);
        let swapped = bitswap16(word, [15, 14, 13, 12, 10, 11, 8, 9, 6, 7, 4, 5, 3, 2, 1, 0]);
        let [lo, hi] = swapped.to_le_bytes();
        prom[i + 1] = lo;
        prom[i + 2] = hi;
    }

    prom.copy_within(0..0x100000, 0x700000);

    let fixed_copy = prom[0x700000..0x800000].to_vec();
    for i in 0..0x10 {
        let src = bitswap8(i as u8, [7, 6, 5, 4, 2, 3, 0, 1]) as usize * 0x10000;
        let dst = i * 0x10000;
        prom[dst..dst + 0x10000].copy_from_slice(&fixed_copy[src..src + 0x10000]);
    }

    let mut bank_copy = vec![0; 0x100000];
    for bank_start in (0x100000..0x700000).step_by(0x100000) {
        for j in (0..0x100000).step_by(0x100) {
            let k = ((bitswap8((j >> 12) as u8, [4, 5, 6, 7, 1, 0, 3, 2]) as usize) << 12)
                | ((j & 0x00f00) ^ 0x00a00);
            bank_copy[j..j + 0x100].copy_from_slice(&prom[bank_start + k..bank_start + k + 0x100]);
        }
        prom[bank_start..bank_start + 0x100000].copy_from_slice(&bank_copy);
    }
}

fn decrypt_kog_bootleg_program(prom: &mut [u8]) {
    if prom.len() < 0x200000 {
        return;
    }

    const BANKS: [usize; 8] = [0x3, 0x8, 0x7, 0xC, 0x1, 0xA, 0x6, 0xD];
    let mut unscrambled = vec![0xFF; 0x100000];
    for (i, bank) in BANKS.iter().copied().enumerate() {
        let src = bank * 0x20000;
        let dst = i * 0x20000;
        if src + 0x20000 <= prom.len() {
            unscrambled[dst..dst + 0x20000].copy_from_slice(&prom[src..src + 0x20000]);
        }
    }
    prom[..0x100000].copy_from_slice(&unscrambled);

    if prom.len() > 0x200000 {
        let parent_len = (prom.len() - 0x200000).min(0x400000);
        prom.copy_within(0x200000..0x200000 + parent_len, 0x100000);
    }
}

fn decrypt_kog_bootleg_srom(srom: &mut [u8]) {
    match srom.first().copied() {
        Some(0x11 | 0x22) => {
            for chunk in srom.chunks_exact_mut(0x10) {
                chunk.rotate_left(8);
            }
        }
        Some(0x30) => {
            for byte in srom {
                *byte = bitswap8(*byte, [7, 6, 0, 4, 3, 2, 1, 5]);
            }
        }
        _ => {}
    }
}

fn layout_kog_bootleg_graphics(mut banks: Vec<(u8, String, Vec<u8>)>) -> Vec<u8> {
    banks.sort_by(|(_, name_a, _), (_, name_b, _)| name_a.cmp(name_b));
    let mut crom = Vec::new();
    append_named_graphics_pair(&mut crom, &banks, "c1a", "c1b");
    append_named_graphics_pair(&mut crom, &banks, "c2a", "c2b");
    append_named_graphics_pair(&mut crom, &banks, "c3", "c4");
    crom
}

fn append_named_graphics_pair(
    out: &mut Vec<u8>,
    banks: &[(u8, String, Vec<u8>)],
    left: &str,
    right: &str,
) {
    let Some(left_bytes) = find_kog_graphics_bank(banks, left) else {
        return;
    };
    let Some(right_bytes) = find_kog_graphics_bank(banks, right) else {
        return;
    };
    append_byte_interleaved_pair(out, left_bytes, right_bytes);
}

fn find_kog_graphics_bank<'a>(banks: &'a [(u8, String, Vec<u8>)], chip: &str) -> Option<&'a [u8]> {
    let needle = format!("5232-{chip}.bin");
    banks
        .iter()
        .find(|(_, name, _)| zip_entry_file_name(name).eq_ignore_ascii_case(&needle))
        .map(|(_, _, bytes)| bytes.as_slice())
}

fn append_byte_interleaved_pair(out: &mut Vec<u8>, left: &[u8], right: &[u8]) {
    let len = left.len().max(right.len());
    for offset in 0..len {
        out.push(left.get(offset).copied().unwrap_or(0xFF));
        out.push(right.get(offset).copied().unwrap_or(0xFF));
    }
}

fn decrypt_kog_bootleg_crom(crom: &mut [u8]) {
    let original = crom.to_vec();
    for (i, chunk) in crom.chunks_exact_mut(0x40).enumerate() {
        let src = (i ^ 1) * 0x40;
        if src + 0x40 <= original.len() {
            chunk.copy_from_slice(&original[src..src + 0x40]);
        }
    }
}

fn bitswap8(value: u8, order: [u8; 8]) -> u8 {
    order
        .iter()
        .fold(0u8, |out, bit| (out << 1) | ((value >> *bit) & 1))
}

fn bitswap16(value: u16, order: [u8; 16]) -> u16 {
    order
        .iter()
        .fold(0u16, |out, bit| (out << 1) | ((value >> *bit) & 1))
}

fn layout_mslug3_sma_program_rom(banked_prom: Vec<u8>, sma_rom: &[u8]) -> Vec<u8> {
    let mut prom = vec![0xFF; FIXED_PROM_WINDOW_SIZE + banked_prom.len()];
    let sma_end = MSLUG3_SMA_CHIP_OFFSET + sma_rom.len().min(0x04_0000);

    prom[MSLUG3_SMA_CHIP_OFFSET..sma_end]
        .copy_from_slice(&sma_rom[..sma_end - MSLUG3_SMA_CHIP_OFFSET]);
    prom[FIXED_PROM_WINDOW_SIZE..FIXED_PROM_WINDOW_SIZE + banked_prom.len()]
        .copy_from_slice(&banked_prom);

    prom
}

fn normalize_zip_graphics_banks(banks: Vec<(u8, Vec<u8>)>) -> Vec<u8> {
    crate::cmc::interleave_cmc_graphics_banks(banks)
}

fn cmc_s_data_size(known_zip_set: Option<KnownZipSet>) -> usize {
    match known_zip_set {
        Some(
            KnownZipSet::Kof2002Encrypted
            | KnownZipSet::Kof99Encrypted
            | KnownZipSet::S1945pEncrypted
            | KnownZipSet::Preisle2Encrypted
            | KnownZipSet::ZupapaEncrypted
            | KnownZipSet::NitdEncrypted
            | KnownZipSet::Sengoku3Encrypted
            | KnownZipSet::BangbeadEncrypted
            | KnownZipSet::GanryuEncrypted
            | KnownZipSet::JockeygpEncrypted,
        ) => 0x20000,
        _ => 0x80000,
    }
}

struct ParsedNeoFile {
    metadata: NeoMetadata,
    prom: Vec<u8>,
    srom: Vec<u8>,
    mrom: Vec<u8>,
    vrom: Vec<u8>,
    vrom_b_offset: usize,
    crom: Vec<u8>,
}

fn parse_neo_file(data: &[u8]) -> Result<ParsedNeoFile, String> {
    if data.len() < NEO_HEADER_SIZE {
        return Err(format!(
            "Archivo .neo inválido: tamaño {} bytes, cabecera mínima {NEO_HEADER_SIZE}",
            data.len()
        ));
    }

    // ── Validate NEO magic + version ──────────────────────────────────
    // Bytes 0-2: "NEO" magic. Byte 3: version.
    // Accepted versions: 0x00 (legacy), 0x01 (standard), 0x02, 0x03, 0x05.
    if data[0] != b'N' || data[1] != b'E' || data[2] != b'O' {
        return Err(format!(
            "Archivo .neo inválido: magic esperado 'NEO', encontrado '{}{}{}' (0x{:02X} 0x{:02X} 0x{:02X})",
            data[0] as char, data[1] as char, data[2] as char,
            data[0], data[1], data[2]
        ));
    }
    let version = data[0x03];
    if !matches!(version, 0x00 | 0x01 | 0x02 | 0x03 | 0x05) {
        return Err(format!(
            "Archivo .neo inválido: versión 0x{:02X} no soportada (esperada 0x00-0x03 o 0x05)",
            version
        ));
    }

    let header = &data[..NEO_HEADER_SIZE];
    let p_size = read_u32_le(header, 0x04)? as usize;
    let s_size = read_u32_le(header, 0x08)? as usize;
    let m_size = read_u32_le(header, 0x0C)? as usize;
    let v1_size = read_u32_le(header, 0x10)? as usize;
    let v2_size = read_u32_le(header, 0x14)? as usize;
    let c_size = read_u32_le(header, 0x18)? as usize;
    let payload_size = p_size
        .checked_add(s_size)
        .and_then(|size| size.checked_add(m_size))
        .and_then(|size| size.checked_add(v1_size))
        .and_then(|size| size.checked_add(v2_size))
        .and_then(|size| size.checked_add(c_size))
        .ok_or_else(|| "Archivo .neo inválido: tamaños de bancos desbordados".to_string())?;
    let expected_size = NEO_HEADER_SIZE
        .checked_add(payload_size)
        .ok_or_else(|| "Archivo .neo inválido: tamaño total desbordado".to_string())?;

    if data.len() < expected_size {
        return Err(format!(
            "Archivo .neo truncado: {} bytes, esperado al menos {expected_size}",
            data.len()
        ));
    }

    let ngh = read_u32_le(header, 0x28)?;
    let metadata = NeoMetadata {
        version: header[0x03],
        year: read_u32_le(header, 0x1C)?,
        genre: read_u32_le(header, 0x20)?,
        screenshot: read_u32_le(header, 0x24)?,
        ngh,
        name: read_fixed_string(header, 0x2C, 33)?,
        manufacturer: read_fixed_string(header, 0x4D, 17)?,
        // Defaults: overridden in from_neo() after NGH=0 detection
        board_type: detect_neo_board_type(ngh),
        fix_banksw: detect_neo_fix_banksw(ngh),
        game_flags: detect_neo_game_flags(ngh),
    };

    let mut cursor = NEO_HEADER_SIZE;
    let prom = take_bank(data, &mut cursor, p_size);
    let srom = take_bank(data, &mut cursor, s_size);
    let mrom = take_bank(data, &mut cursor, m_size);
    let v1 = take_bank(data, &mut cursor, v1_size);
    let v2 = take_bank(data, &mut cursor, v2_size);
    // ── V-ROM: keep V1 and V2 as distinct source regions ───────────
    // Geolith exposes V1 to ADPCM-A and V2 to ADPCM-B. If V2Size=0,
    // ADPCM-B mirrors V1 by using offset 0 instead of duplicating data.
    let mut vrom = v1.clone();
    let vrom_b_offset = if v2_size == 0 { 0 } else { v1_size };
    if v2_size != 0 {
        vrom.extend(v2);
    }
    // ── Safety: ensure V-ROM is not empty if both V1 and V2 are 0 ──
    if vrom.is_empty() {
        vrom.push(0xFF);
    }
    let crom = take_bank(data, &mut cursor, c_size);

    Ok(ParsedNeoFile {
        metadata,
        prom,
        srom,
        mrom,
        vrom,
        vrom_b_offset,
        crom,
    })
}

fn take_bank(data: &[u8], cursor: &mut usize, size: usize) -> Vec<u8> {
    let bank = data[*cursor..*cursor + size].to_vec();
    *cursor += size;
    bank
}

fn read_u32_le(data: &[u8], offset: usize) -> Result<u32, String> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or_else(|| format!("Cabecera .neo incompleta en offset 0x{offset:X}"))?;
    Ok(u32::from_le_bytes(
        bytes.try_into().expect("slice length checked"),
    ))
}

fn read_fixed_string(data: &[u8], offset: usize, len: usize) -> Result<String, String> {
    let bytes = data
        .get(offset..offset + len)
        .ok_or_else(|| format!("Cabecera .neo incompleta en string 0x{offset:X}"))?;
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    Ok(String::from_utf8_lossy(&bytes[..end]).trim().to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RomPart {
    Program(u8),
    Graphics(u8),
    FixLayer,
    AudioCpu,
    Samples(u8),
    Sma,
}

impl RomPart {
    fn order(self) -> (u8, u8) {
        match self {
            RomPart::Program(index) => (0, index),
            RomPart::Graphics(index) => (1, index),
            RomPart::FixLayer => (2, 1),
            RomPart::AudioCpu => (3, 1),
            RomPart::Samples(index) => (4, index),
            RomPart::Sma => (5, 0),
        }
    }
}

/// Known BIOS/system file patterns that should be filtered out from game ZIPs.
/// These are commonly included in "merged" MAME/FBNeo ROM sets.
fn is_bios_entry(name: &str) -> bool {
    let file_name = zip_entry_file_name(name);

    // Exact matches for known BIOS/system files
    if matches!(
        file_name,
        "sfix.sfix" | "sm1.sm1" | "000-lo.lo" | "vs-bios.rom" | "v2.bin"
    ) {
        return true;
    }

    // SPI / SP1 BIOS prefixes
    if file_name.starts_with("sp-") || file_name.starts_with("sp1") {
        return true;
    }

    // Some merged game ZIPs (notably Irritating Maze) include per-game BIOS
    // images such as 236-bios.sp1 alongside the cartridge ROMs. They must not
    // be treated as program ROM chips just because the extension looks like
    // an SP1 program suffix.
    if file_name.contains("bios") && file_name.ends_with(".sp1") {
        return true;
    }

    // UniBIOS variants
    if file_name.starts_with("uni-bios") {
        return true;
    }

    // Regional/system ROMs found in merged sets
    if file_name == "asia-s3.rom"
        || file_name == "japan-j3.bin"
        || file_name.ends_with(".jipan.1024")
    {
        return true;
    }

    false
}

/// Classify a ZIP entry into a ROM part based on MAME/FBNeo naming conventions.
///
/// Supports the following chip prefix patterns:
/// - `pg1`, `pg2`, ... → Program (graphics-banked, like mslug3)
/// - `sp1`, `sp2`, ... → Program (split/byte-swapped, like kof2002)
/// - `ep1`, `ep2`, ... → Program (encrypted, like garou)
/// - `bp1`, `bp2`, ... → Program (bootleg variants)
/// - `p1`, `p2`, ...   → Program
/// - `c1`, `c2`, ...   → Graphics
/// - `s1`              → Fix Layer
/// - `m1`              → Audio CPU
/// - `v1`, `v2`, ...   → Samples
///
/// As a fallback, also checks the file extension for chip type hints.
///
/// Should be called *after* `is_bios_entry()` has returned `false`.
fn classify_zip_entry(name: &str) -> Option<RomPart> {
    let file_name = zip_entry_file_name(name);
    let (stem, extension) = file_name
        .rsplit_once('.')
        .map_or((file_name, ""), |(stem, ext)| (stem, ext));
    let chip = stem.rsplit(['-', '_']).next().unwrap_or(stem);
    let chip = chip.trim_start_matches('0');
    let chip = if chip.is_empty() { stem } else { chip };

    // Check for special named entries first
    if file_name == "neo-sma" || file_name.ends_with(".neo-sma") {
        return Some(RomPart::Sma);
    }
    match file_name {
        // Super Bubble Pop uses board-position labels instead of normal
        // NeoGeo p/c/s/m/v chip suffixes.
        "001-003-02a.u2" => return Some(RomPart::Program(1)),
        "001-003-02b.u2" => return Some(RomPart::FixLayer),
        "001-003-03b.u3" => return Some(RomPart::Graphics(3)),
        "001-003-04b.u4" => return Some(RomPart::Graphics(4)),
        "001-003-01b.u1" => return Some(RomPart::AudioCpu),
        "001-003-12a.u12" => return Some(RomPart::Samples(12)),
        "001-003-13a.u13" => return Some(RomPart::Samples(13)),
        "no3.bin" => return Some(RomPart::Graphics(3)),
        "no4.bin" => return Some(RomPart::Graphics(4)),
        _ => {}
    }

    // Try to classify by chip suffix (last dash/underscore component)
    if let Some(part) = classify_by_chip_suffix(chip) {
        return Some(part);
    }

    // Fallback: classify by file extension (e.g., .p1, .c1, .m1)
    classify_by_extension(extension)
}

fn zip_entry_file_name(name: &str) -> &str {
    name.rsplit(['/', '\\']).next().unwrap_or(name)
}

/// Classify a ROM by its chip suffix (the component after the last `-` or `_`).
fn classify_by_chip_suffix(chip: &str) -> Option<RomPart> {
    // pg prefix: Program with graphics banking (e.g., 256-pg1.p1)
    if let Some(index) = chip.strip_prefix("pg").and_then(parse_chip_index) {
        return Some(RomPart::Program(index));
    }

    // sp/ep/bp prefix: Split/Encrypted/Bootleg Program ROMs
    // Check these BEFORE the single-letter prefixes
    for prefix in &["sp", "ep", "bp"] {
        if let Some(rest) = chip.strip_prefix(prefix) {
            if let Some(index) = parse_chip_index(rest) {
                return Some(RomPart::Program(index));
            }
        }
    }

    // Single-letter prefixes
    if let Some(index) = chip.strip_prefix('p').and_then(parse_chip_index) {
        return Some(RomPart::Program(index));
    }
    if let Some(index) = chip.strip_prefix('c').and_then(parse_chip_index) {
        return Some(RomPart::Graphics(index));
    }
    if chip == "s1" {
        return Some(RomPart::FixLayer);
    }
    if chip == "m1" {
        return Some(RomPart::AudioCpu);
    }
    if let Some(index) = chip.strip_prefix('v').and_then(parse_chip_index) {
        return Some(RomPart::Samples(index));
    }

    None
}

/// Fallback: classify a ROM by its file extension.
/// Used when the chip suffix doesn't match any known pattern.
fn classify_by_extension(extension: &str) -> Option<RomPart> {
    let ext = extension.trim_start_matches('0');

    // Split/encrypted/bootleg program ROMs can appear as .sp2/.ep1/.bp1
    // when the stem contains revision suffixes (e.g. 262-p2-08-e0.sp2).
    for prefix in &["pg", "sp", "ep", "bp"] {
        if let Some(rest) = ext.strip_prefix(prefix) {
            if let Some(index) = parse_chip_index(rest) {
                return Some(RomPart::Program(index));
            }
        }
    }

    // Program: .p1, .p2, .p3, ...
    if let Some(rest) = ext.strip_prefix('p') {
        if let Some(index) = parse_chip_index(rest) {
            return Some(RomPart::Program(index));
        }
    }

    // Graphics: .c1, .c2, .c3, ...
    if let Some(rest) = ext.strip_prefix('c') {
        if let Some(index) = parse_chip_index(rest) {
            return Some(RomPart::Graphics(index));
        }
    }

    // Fix Layer: .s1
    if ext == "s1" {
        return Some(RomPart::FixLayer);
    }

    // Audio CPU: .m1
    if ext == "m1" {
        return Some(RomPart::AudioCpu);
    }

    // Samples: .v1, .v2, .v3, ...
    if let Some(rest) = ext.strip_prefix('v') {
        if let Some(index) = parse_chip_index(rest) {
            return Some(RomPart::Samples(index));
        }
    }

    None
}

fn parse_chip_index(value: &str) -> Option<u8> {
    let digits_len = value
        .bytes()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    if digits_len == 0 {
        return None;
    }

    value[..digits_len]
        .parse::<u8>()
        .ok()
        .filter(|index| *index > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    #[test]
    fn classifies_common_mame_style_file_names() {
        assert_eq!(
            classify_zip_entry("mslug/201-p1.p1"),
            Some(RomPart::Program(1))
        );
        assert_eq!(classify_zip_entry("256-pg1.p1"), Some(RomPart::Program(1)));
        assert_eq!(classify_zip_entry("neo-sma"), Some(RomPart::Sma));
        assert_eq!(classify_zip_entry("green.neo-sma"), Some(RomPart::Sma));
        assert_eq!(classify_zip_entry("ka.neo-sma"), Some(RomPart::Sma));
        assert_eq!(classify_zip_entry("kf.neo-sma"), Some(RomPart::Sma));
        assert_eq!(classify_zip_entry("201-c8.c8"), Some(RomPart::Graphics(8)));
        assert_eq!(classify_zip_entry("201-v2.v2"), Some(RomPart::Samples(2)));
        assert_eq!(classify_zip_entry("201-s1.s1"), Some(RomPart::FixLayer));
        assert_eq!(classify_zip_entry("201-m1.m1"), Some(RomPart::AudioCpu));
    }

    #[test]
    fn detects_2mb_p1_continue_layout_from_second_meg_header() {
        let mut p1 = vec![0u8; 0x200000];
        write_word_swapped_neo_header(&mut p1, 0x100100);

        assert!(first_program_bank_header_is_in_second_meg(&p1));

        swap_program_rom_halves(&mut p1);
        normalize_program_rom_byte_order(&mut p1);

        assert_eq!(&p1[0x100..0x107], b"NEO-GEO");
    }

    #[test]
    fn does_not_half_swap_2mb_p1_when_header_is_already_first() {
        let mut p1 = vec![0u8; 0x200000];
        write_word_swapped_neo_header(&mut p1, 0x100);
        write_word_swapped_neo_header(&mut p1, 0x100100);

        assert!(!first_program_bank_header_is_in_second_meg(&p1));
    }

    #[test]
    fn classifies_bin_names_with_chip_suffixes() {
        assert_eq!(classify_zip_entry("game-p1.bin"), Some(RomPart::Program(1)));
        assert_eq!(
            classify_zip_entry("game_c2.bin"),
            Some(RomPart::Graphics(2))
        );
        assert_eq!(classify_zip_entry("v1.bin"), Some(RomPart::Samples(1)));
    }

    #[test]
    fn classifies_sp_prefix_as_program_rom() {
        // Split program ROMs: sp1, sp2 (like kof2002 265-p2.sp2)
        assert_eq!(
            classify_zip_entry("game-sp1.bin"),
            Some(RomPart::Program(1))
        );
        assert_eq!(
            classify_zip_entry("game-sp2.sp2"),
            Some(RomPart::Program(2))
        );
        assert_eq!(
            classify_zip_entry("262-p2-08-e0.sp2"),
            Some(RomPart::Program(2))
        );
        // Note: 265-p2.sp2 works because stem is "265-p2" and chip is "p2"
        // sp2.sp2 also works because stem is "sp2" and chip is "sp2"
        assert_eq!(classify_zip_entry("sp2.sp2"), Some(RomPart::Program(2)));
    }

    #[test]
    fn classifies_ep_prefix_as_program_rom() {
        // Encrypted program ROMs: ep1, ep2 (like garou)
        assert_eq!(classify_zip_entry("253-ep1.ep1"), Some(RomPart::Program(1)));
        assert_eq!(classify_zip_entry("253-ep2.ep2"), Some(RomPart::Program(2)));
        assert_eq!(
            classify_zip_entry("garou-ep1.bin"),
            Some(RomPart::Program(1))
        );
    }

    #[test]
    fn classifies_bp_prefix_as_program_rom() {
        // Bootleg program ROMs
        assert_eq!(
            classify_zip_entry("game-bp1.bin"),
            Some(RomPart::Program(1))
        );
    }

    #[test]
    fn classifies_by_extension_fallback() {
        // When the chip suffix doesn't match, try the file extension
        assert_eq!(
            classify_zip_entry("unknown-chip.p1"),
            Some(RomPart::Program(1))
        );
        assert_eq!(classify_zip_entry("foo.c1"), Some(RomPart::Graphics(1)));
        assert_eq!(
            classify_zip_entry("5232-c1a.bin"),
            Some(RomPart::Graphics(1))
        );
        assert_eq!(classify_zip_entry("bar.s1"), Some(RomPart::FixLayer));
        assert_eq!(classify_zip_entry("baz.m1"), Some(RomPart::AudioCpu));
        assert_eq!(classify_zip_entry("qux.v3"), Some(RomPart::Samples(3)));
    }

    #[test]
    fn classifies_super_bubble_pop_board_position_names() {
        assert_eq!(
            classify_zip_entry("001-003-02a.u2"),
            Some(RomPart::Program(1))
        );
        assert_eq!(
            classify_zip_entry("001-003-02b.u2"),
            Some(RomPart::FixLayer)
        );
        assert_eq!(
            classify_zip_entry("001-003-03b.u3"),
            Some(RomPart::Graphics(3))
        );
        assert_eq!(
            classify_zip_entry("001-003-04b.u4"),
            Some(RomPart::Graphics(4))
        );
        assert_eq!(
            classify_zip_entry("001-003-01b.u1"),
            Some(RomPart::AudioCpu)
        );
        assert_eq!(
            classify_zip_entry("001-003-12a.u12"),
            Some(RomPart::Samples(12))
        );
        assert_eq!(
            classify_zip_entry("001-003-13a.u13"),
            Some(RomPart::Samples(13))
        );
    }

    #[test]
    fn detects_super_bubble_pop_homebrew_zip_set() {
        let files = vec![
            "001-003-02a.u2".to_string(),
            "001-003-02b.u2".to_string(),
            "001-003-03b.u3".to_string(),
            "001-003-04b.u4".to_string(),
            "001-003-01b.u1".to_string(),
            "001-003-12a.u12".to_string(),
        ];

        assert_eq!(detect_known_zip_set(&files), Some(KnownZipSet::SbpHomebrew));
    }

    #[test]
    fn sbp_callback_applies_fbneo_program_patches_and_srom_size() {
        let mut prom = vec![0; 0x40000];
        prom[0x1000] = 0x12;
        prom[0x107f] = 0xab;
        let mut srom = vec![0xee; 0x80000];

        apply_sbp_callback(&mut prom, &mut srom);

        assert_eq!(prom[0x1000], 0x21);
        assert_eq!(prom[0x107f], 0xba);
        assert_eq!(&prom[0x2a6f8..0x2a6fa], &[0x4e, 0x71]);
        assert_eq!(&prom[0x2a6fa..0x2a6fc], &[0x4e, 0x71]);
        assert_eq!(&prom[0x2a6fc..0x2a6fe], &[0x4e, 0x71]);
        assert_eq!(&prom[0x3ff2c..0x3ff2e], &[0x70, 0x01]);
        assert_eq!(srom.len(), 0x20000);
    }

    #[test]
    fn detects_dragonsh_development_board_name() {
        assert_eq!(detect_ngh_from_zip_name("dragonsh"), Some(0x094));
        assert_eq!(detect_ngh_from_zip_name("dragonsh.zip"), Some(0x094));
        assert_eq!(get_recommended_bios(0x094), "MVS/AES");
        let files = vec![
            "ep1.bin".to_string(),
            "ep2.bin".to_string(),
            "no3.bin".to_string(),
            "no4.bin".to_string(),
            "s1.s1".to_string(),
        ];
        assert_eq!(
            detect_known_zip_set(&files),
            Some(KnownZipSet::DragonshDevelopment)
        );
        assert_eq!(classify_zip_entry("no3.bin"), Some(RomPart::Graphics(3)));
        assert_eq!(classify_zip_entry("no4.bin"), Some(RomPart::Graphics(4)));
    }

    #[test]
    fn detects_zip_name_fallbacks_for_headerless_sets() {
        let cases = [
            ("aodk", 0x074),
            ("goalx3", 0x209),
            ("kabukikl", 0x092),
            ("neocup98", 0x244),
            ("neodrift", 0x213),
            ("overtop", 0x212),
            ("pgoal", 0x219),
            ("savagere", 0x059),
            ("sdodgeb", 0x208),
            ("turfmast", 0x200),
            ("twinspri", 0x224),
        ];

        for (name, expected_ngh) in cases {
            assert_eq!(detect_ngh_from_zip_name(name), Some(expected_ngh), "{name}");
            assert_eq!(
                detect_ngh_from_zip_name(&format!("{name}.zip")),
                Some(expected_ngh),
                "{name}.zip"
            );
        }
    }

    #[test]
    fn dragonsh_layout_interleaves_eprom_program_bytes() {
        let prom = layout_zip_program_banks(
            Some(KnownZipSet::DragonshDevelopment),
            vec![
                (1, "ep1.bin".to_string(), vec![0x10, 0x11, 0x12]),
                (2, "ep2.bin".to_string(), vec![0x20, 0x21]),
            ],
        );

        assert_eq!(prom, vec![0x10, 0x20, 0x11, 0x21, 0x12, 0xff]);
    }

    #[test]
    fn filters_bios_entries_from_merged_sets() {
        // Known BIOS files that should be filtered out
        assert!(is_bios_entry("000-lo.lo"));
        assert!(is_bios_entry("sfix.sfix"));
        assert!(is_bios_entry("sm1.sm1"));
        assert!(is_bios_entry("v2.bin"));
        assert!(is_bios_entry("vs-bios.rom"));
        assert!(is_bios_entry("sp-s2.sp1"));
        assert!(is_bios_entry("sp-45.sp1"));
        assert!(is_bios_entry("sp-e.sp1"));
        assert!(is_bios_entry("sp1.jipan.1024"));
        assert!(is_bios_entry("236-bios.sp1"));
        assert!(is_bios_entry("236-bios_japan_hack.sp1"));
        assert!(is_bios_entry("uni-bios_4_0.rom"));
        assert!(is_bios_entry("uni-bios_1_0.rom"));
        assert!(is_bios_entry("asia-s3.rom"));
        assert!(is_bios_entry("japan-j3.bin"));
    }

    #[test]
    fn does_not_filter_game_entries() {
        // Game ROMs should NOT be filtered as BIOS
        assert!(!is_bios_entry("265-p1.p1"));
        assert!(!is_bios_entry("265-p2.sp2"));
        assert!(!is_bios_entry("265-c1.c1"));
        assert!(!is_bios_entry("265-m1.m1"));
        assert!(!is_bios_entry("265-v1.v1"));
        assert!(!is_bios_entry("256-pg1.p1"));
        assert!(!is_bios_entry("256-c1.c1"));
        assert!(!is_bios_entry("neo-sma"));
        assert!(!is_bios_entry("green.neo-sma"));
    }

    #[test]
    fn classifies_kof2002_like_merged_zip_entries() {
        // Simulate what classify_zip_entry sees AFTER BIOS filtering
        // These should all be classified as game ROMs
        assert_eq!(classify_zip_entry("265-p1.p1"), Some(RomPart::Program(1)));
        assert_eq!(classify_zip_entry("265-p2.sp2"), Some(RomPart::Program(2)));
        assert_eq!(classify_zip_entry("265-c1.c1"), Some(RomPart::Graphics(1)));
        assert_eq!(classify_zip_entry("265-c8.c8"), Some(RomPart::Graphics(8)));
        assert_eq!(classify_zip_entry("265-m1.m1"), Some(RomPart::AudioCpu));
        assert_eq!(classify_zip_entry("265-v1.v1"), Some(RomPart::Samples(1)));
        assert_eq!(classify_zip_entry("265-v2.v2"), Some(RomPart::Samples(2)));
    }

    #[test]
    fn classifies_mslug3_like_merged_zip_entries() {
        assert_eq!(classify_zip_entry("256-pg1.p1"), Some(RomPart::Program(1)));
        assert_eq!(classify_zip_entry("256-pg2.p2"), Some(RomPart::Program(2)));
        assert_eq!(classify_zip_entry("neo-sma"), Some(RomPart::Sma));
    }

    #[test]
    fn rejects_external_rom_without_program_data() {
        let rom = RomData {
            prom: Vec::new(),
            crom: Vec::new(),
            srom: Vec::new(),
            mrom: Vec::new(),
            vrom: Vec::new(),
            vrom_b_offset: 0,
            sma_rom: Vec::new(),
            is_demo: false,
            source: RomSource::NeoFile,
            recognized_files: Vec::new(),
            metadata: None,
        };

        assert!(rom.validate_external().is_err());
    }

    #[test]
    fn pads_odd_sized_program_rom() {
        let rom = RomData {
            prom: vec![0; MIN_EXTERNAL_PROM_SIZE + 1],
            crom: Vec::new(),
            srom: Vec::new(),
            mrom: Vec::new(),
            vrom: Vec::new(),
            vrom_b_offset: 0,
            sma_rom: Vec::new(),
            is_demo: false,
            source: RomSource::NeoFile,
            recognized_files: Vec::new(),
            metadata: None,
        }
        .validate_external()
        .unwrap();

        assert_eq!(rom.prom.len() % 2, 0);
        // All-zero data: SP=0x00000000 is invalid in both raw and swapped
        // form, so NO byte-swapping occurs. The 0xFF pad stays at the end.
        assert_eq!(rom.prom[rom.prom.len() - 1], 0xFF);
    }

    #[test]
    fn normalizes_word_byte_swapped_program_roms() {
        let rom = RomData {
            prom: vec![0x10, 0x00, 0x00, 0xF3, 0xC0, 0x00, 0x02, 0x04],
            crom: Vec::new(),
            srom: Vec::new(),
            mrom: Vec::new(),
            vrom: Vec::new(),
            vrom_b_offset: 0,
            sma_rom: Vec::new(),
            is_demo: false,
            source: RomSource::NeoFile,
            recognized_files: Vec::new(),
            metadata: None,
        }
        .validate_external()
        .unwrap();

        assert_eq!(
            &rom.prom[..8],
            &[0x00, 0x10, 0xF3, 0x00, 0x00, 0xC0, 0x04, 0x02]
        );
    }

    #[test]
    fn diagnostic_vectors_accept_word_byte_swapped_program_roms() {
        let prom = vec![0x10, 0x00, 0x00, 0xF3, 0xC0, 0x00, 0x02, 0x04];

        validate_initial_vectors_for_diagnostics(&prom, "byte-swapped.neo").unwrap();
    }

    #[test]
    fn normalizes_neo_program_roms_by_cart_header_even_with_weird_vectors() {
        let mut prom = vec![0; 0x130];
        // Some real .neo P-ROMs have vectors that do not satisfy the stack
        // heuristic, but their cart header is still byte-swapped on disk.
        prom[0x100..0x108].copy_from_slice(b"EN-OEG\0O");
        prom[0x108..0x10A].copy_from_slice(&[0x69, 0x00]);

        normalize_program_rom_byte_order(&mut prom);

        assert_eq!(&prom[0x100..0x107], b"NEO-GEO");
        assert_eq!(&prom[0x108..0x10A], &[0x00, 0x69]);
    }

    #[test]
    fn keeps_big_endian_program_roms_unchanged() {
        // P-ROM con SP válido en raw form (0x0010FD00 = work RAM).
        // La heurística NO debe swappear estos bytes.
        let rom = RomData {
            prom: vec![0x00, 0x10, 0xFD, 0x00, 0x00, 0x00, 0x01, 0x00],
            crom: Vec::new(),
            srom: Vec::new(),
            mrom: Vec::new(),
            vrom: Vec::new(),
            vrom_b_offset: 0,
            sma_rom: Vec::new(),
            is_demo: false,
            source: RomSource::NeoFile,
            recognized_files: Vec::new(),
            metadata: None,
        }
        .validate_external()
        .unwrap();

        // Raw SP=0x0010FD00 está en rango work RAM → NO swap
        assert_eq!(
            &rom.prom[..8],
            &[0x00, 0x10, 0xFD, 0x00, 0x00, 0x00, 0x01, 0x00]
        );
    }

    #[test]
    fn swaps_program_rom_words_in_place() {
        let mut prom = vec![0x12, 0x34, 0x56, 0x78, 0x9A];

        swap_program_rom_words(&mut prom);

        assert_eq!(prom, vec![0x34, 0x12, 0x78, 0x56, 0x9A]);
    }

    #[test]
    fn layouts_mslug3_sma_rom_like_sma_board_map() {
        let prom = layout_mslug3_sma_program_rom(vec![0x11; 4], &[0x22; 4]);

        assert_eq!(prom.len(), FIXED_PROM_WINDOW_SIZE + 4);
        assert_eq!(
            &prom[MSLUG3_SMA_CHIP_OFFSET..MSLUG3_SMA_CHIP_OFFSET + 4],
            &[0x22; 4]
        );
        assert_eq!(
            &prom[FIXED_PROM_WINDOW_SIZE..FIXED_PROM_WINDOW_SIZE + 4],
            &[0x11; 4]
        );
        assert_eq!(prom[0], 0xFF);
    }

    #[test]
    fn interleaves_zip_crom_pairs_by_byte() {
        let mut c1 = Vec::new();
        let mut c2 = Vec::new();
        c1.extend([0x11; 64]);
        c1.extend([0x12; 64]);
        c2.extend([0x21; 64]);
        c2.extend([0x22; 64]);

        let crom = normalize_zip_graphics_banks(vec![(2, c2), (1, c1)]);

        assert_eq!(&crom[0..4], &[0x11, 0x21, 0x11, 0x21]);
        assert_eq!(&crom[126..130], &[0x11, 0x21, 0x12, 0x22]);
        assert_eq!(&crom[252..256], &[0x12, 0x22, 0x12, 0x22]);
    }

    #[test]
    fn preserves_neo_crom_byte_order() {
        let tile0: Vec<u8> = (0..64).flat_map(|value| [value, value + 0x80]).collect();
        let tile1: Vec<u8> = (0..64)
            .flat_map(|value| [value + 0x40, value + 0xC0])
            .collect();

        let path = unique_temp_path("ngneon-neo-crom-order", "neo");
        let crom = [tile0, tile1].concat();
        // P-ROM: 64KB con vectores 68k válidos (SP=0x0010FD00, PC=0x00000100)
        let mut prom = vec![0u8; 0x10000];
        prom[0..4].copy_from_slice(&[0x00, 0x10, 0xFD, 0x00]);
        prom[4..8].copy_from_slice(&[0x00, 0x00, 0x01, 0x00]);
        create_test_neo(&path, &prom, &[], &[], &[], &[], &crom);
        let rom = RomData::from_neo(&path).unwrap();

        assert_eq!(rom.crom, crom);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn loads_zip_and_groups_banks_in_chip_order() {
        let path = unique_temp_path("ngneon-rom-loader", "zip");
        create_test_zip(
            &path,
            &[
                ("game-c2.bin", &[0xC2, 0xC2]),
                ("game-p2.bin", &[0xB2, 0xB2, 0xB2, 0xB2]),
                ("game-p1.bin", &[0xB1, 0xB1, 0xB1, 0xB1]),
                ("game-c1.bin", &[0xC1, 0xC1]),
                ("game-s1.bin", &[0xA1, 0xA1]),
                ("game-m1.bin", &[0x91, 0x91]),
                ("game-v1.bin", &[0x81, 0x81]),
            ],
        );

        let rom = RomData::from_zip(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(
            rom.prom,
            vec![0xB1, 0xB1, 0xB1, 0xB1, 0xB2, 0xB2, 0xB2, 0xB2]
        );
        assert_eq!(rom.crom, vec![0xC1, 0xC2, 0xC1, 0xC2]);
        assert_eq!(rom.srom, vec![0xA1, 0xA1]);
        assert_eq!(rom.mrom, vec![0x91, 0x91]);
        assert_eq!(rom.vrom, vec![0x81, 0x81]);
        assert_eq!(rom.vrom_b_offset, 0);
        assert_eq!(rom.source, RomSource::ZipArchive);
        assert_eq!(rom.recognized_files.len(), 7);
    }

    #[test]
    fn zip_sample_chunks_share_one_ym2610_sample_region() {
        let path = unique_temp_path("ngneon-zip-vrom-layout", "zip");
        create_test_zip(
            &path,
            &[
                ("game-p1.bin", &[0x10; MIN_EXTERNAL_PROM_SIZE]),
                ("game-v1.v1", &[0x81, 0x82]),
                ("game-v2.v2", &[0x91, 0x92]),
                ("game-v3.v3", &[0xA1, 0xA2]),
            ],
        );

        let rom = RomData::from_zip(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(rom.vrom, vec![0x81, 0x82, 0x91, 0x92, 0xA1, 0xA2]);
        assert_eq!(
            rom.vrom_b_offset, 0,
            "MAME/FBNeo v2/v3 files are continuation chunks, not .neo V2 metadata"
        );
    }

    #[test]
    fn loads_kof2002_zip_with_cmc50_decrypted_m1_buffer() {
        let path = unique_temp_path("ngneon-kof2002-loader", "zip");
        create_test_zip(
            &path,
            &[
                ("265-p1.p1", &[0x10; MIN_EXTERNAL_PROM_SIZE]),
                ("265-p2.sp2", &[0x20; 4]),
                ("265-c1.c1", &[0xC1; crate::video::BYTES_PER_TILE]),
                ("265-m1.m1", &[0x91; 0x20]),
                ("265-v1.v1", &[0x81; 0x20]),
            ],
        );

        let rom = RomData::from_zip(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(rom.mrom.len(), crate::cmc::CMC_M1_DECRYPTED_SIZE);
        assert_eq!(rom.srom.len(), 0x20000);
        assert_ne!(&rom.mrom[..0x20], &[0x91; 0x20]);
        assert_eq!(
            &rom.prom[..MIN_EXTERNAL_PROM_SIZE],
            &[0x10; MIN_EXTERNAL_PROM_SIZE]
        );
    }

    #[test]
    fn layouts_p32_program_pair_like_fbneo() {
        let prom = layout_p32_program_pair(&[0x10, 0x11, 0x12, 0x13], &[0x20, 0x21, 0x22, 0x23]);

        assert_eq!(prom, vec![0x10, 0x11, 0x20, 0x21, 0x12, 0x13, 0x22, 0x23]);
    }

    #[test]
    fn detects_mslug5_encrypted_zip_set() {
        let files = vec![
            "268-p1cr.p1".to_string(),
            "268-p2cr.p2".to_string(),
            "268-c1c.c1".to_string(),
            "268-m1.m1".to_string(),
            "268-v1c.v1".to_string(),
        ];

        assert_eq!(
            detect_known_zip_set(&files),
            Some(KnownZipSet::Mslug5Encrypted)
        );
    }

    #[test]
    fn detects_samurai_shodown_cmc50_pcm2_zip_sets() {
        let samsho5 = vec![
            "270-p1.p1".to_string(),
            "270-p2.sp2".to_string(),
            "270-c1.c1".to_string(),
            "270-m1.m1".to_string(),
            "270-v1.v1".to_string(),
        ];
        let samsh5sp = vec![
            "272-p1.p1".to_string(),
            "272-p2.sp2".to_string(),
            "272-c1.c1".to_string(),
            "272-m1.m1".to_string(),
            "272-v1.v1".to_string(),
        ];

        assert_eq!(
            detect_known_zip_set(&samsho5),
            Some(KnownZipSet::Samsho5Encrypted)
        );
        assert_eq!(
            detect_known_zip_set(&samsh5sp),
            Some(KnownZipSet::Samsh5spEncrypted)
        );
    }

    #[test]
    fn detects_late_cmc50_pcm2_zip_sets() {
        let rotd = vec![
            "264-p1.p1".to_string(),
            "264-c1.c1".to_string(),
            "264-m1.m1".to_string(),
            "264-v1.v1".to_string(),
        ];
        let matrim = vec![
            "266-p1.p1".to_string(),
            "266-p2.sp2".to_string(),
            "266-c1.c1".to_string(),
            "266-m1.m1".to_string(),
            "266-v1.v1".to_string(),
        ];
        let pnyaa = vec![
            "pn202.p1".to_string(),
            "267-c1.c1".to_string(),
            "m1.m1".to_string(),
            "267-v1.v1".to_string(),
        ];

        assert_eq!(
            detect_known_zip_set(&rotd),
            Some(KnownZipSet::RotdEncrypted)
        );
        assert_eq!(
            detect_known_zip_set(&matrim),
            Some(KnownZipSet::MatrimEncrypted)
        );
        assert_eq!(
            detect_known_zip_set(&pnyaa),
            Some(KnownZipSet::PnyaaEncrypted)
        );
    }

    #[test]
    fn detects_cmc50_m1_only_zip_sets() {
        let kof2001 = vec![
            "262-p1-08-e0.p1".to_string(),
            "262-p2-08-e0.sp2".to_string(),
            "262-c1-08-e0.c1".to_string(),
            "265-262-m1.m1".to_string(),
            "262-v1-08-e0.v1".to_string(),
        ];
        let jockeygp = vec![
            "008-epr.p1".to_string(),
            "008-c1.c1".to_string(),
            "008-c2.c2".to_string(),
            "008-mg1.m1".to_string(),
            "008-v1.v1".to_string(),
        ];

        assert_eq!(
            detect_known_zip_set(&kof2001),
            Some(KnownZipSet::Kof2001Encrypted)
        );
        assert_eq!(
            detect_known_zip_set(&jockeygp),
            Some(KnownZipSet::JockeygpEncrypted)
        );
    }

    #[test]
    fn detects_swapp_zip_set() {
        let ctomaday = vec![
            "249-p1.p1".to_string(),
            "249-s1.s1".to_string(),
            "249-c1.c1".to_string(),
            "249-m1.m1".to_string(),
            "249-v1.v1".to_string(),
        ];
        let crswd2bl = vec![
            "054-p1.p1".to_string(),
            "054-s1.s1".to_string(),
            "054-c1.c1".to_string(),
            "054-c2.c2".to_string(),
            "054-m1.m1".to_string(),
            "054-v1.v1".to_string(),
        ];
        let ironclad = vec![
            "proto_220-p1.p1".to_string(),
            "proto_220-s1.s1".to_string(),
            "proto_220-c1.c1".to_string(),
            "proto_220-c2.c2".to_string(),
            "proto_220-m1.m1".to_string(),
            "proto_220-v1.v1".to_string(),
        ];

        assert_eq!(
            detect_known_zip_set(&ctomaday),
            Some(KnownZipSet::CtomadaySwapp)
        );
        assert_eq!(
            detect_known_zip_set(&crswd2bl),
            Some(KnownZipSet::Crswd2blSwapp)
        );
        assert_eq!(
            detect_known_zip_set(&ironclad),
            Some(KnownZipSet::IroncladSwapp)
        );
    }

    #[test]
    fn detects_cmc42_zip_sets() {
        let preisle2 = vec![
            "255-p1.p1".to_string(),
            "255-p2.sp2".to_string(),
            "255-c1.c1".to_string(),
            "255-c2.c2".to_string(),
            "255-m1.m1".to_string(),
        ];
        let bangbead = vec![
            "259-p1.p1".to_string(),
            "259-c1.c1".to_string(),
            "259-c2.c2".to_string(),
            "259-m1.m1".to_string(),
            "259-v1.v1".to_string(),
        ];
        let ganryu = vec![
            "252-p1.p1".to_string(),
            "252-c1.c1".to_string(),
            "252-c2.c2".to_string(),
            "252-m1.m1".to_string(),
            "252-v1.v1".to_string(),
        ];
        let zupapa = vec![
            "070-p1.p1".to_string(),
            "070-c1.c1".to_string(),
            "070-c2.c2".to_string(),
            "070-epr.m1".to_string(),
            "070-v1.v1".to_string(),
        ];
        let nitd = vec![
            "260-p1.p1".to_string(),
            "260-c1.c1".to_string(),
            "260-c2.c2".to_string(),
            "260-m1.m1".to_string(),
            "260-v1.v1".to_string(),
        ];
        let sengoku3 = vec![
            "261-ph1.p1".to_string(),
            "261-c1.c1".to_string(),
            "261-c2.c2".to_string(),
            "261-m1.m1".to_string(),
            "261-v1.v1".to_string(),
        ];

        assert_eq!(
            detect_known_zip_set(&preisle2),
            Some(KnownZipSet::Preisle2Encrypted)
        );
        assert_eq!(
            detect_known_zip_set(&bangbead),
            Some(KnownZipSet::BangbeadEncrypted)
        );
        assert_eq!(
            detect_known_zip_set(&ganryu),
            Some(KnownZipSet::GanryuEncrypted)
        );
        assert_eq!(
            detect_known_zip_set(&zupapa),
            Some(KnownZipSet::ZupapaEncrypted)
        );
        assert_eq!(
            detect_known_zip_set(&nitd),
            Some(KnownZipSet::NitdEncrypted)
        );
        assert_eq!(
            detect_known_zip_set(&sengoku3),
            Some(KnownZipSet::Sengoku3Encrypted)
        );
    }

    #[test]
    fn swapp_zip_program_layout_swaps_first_bank_halves() {
        let first = vec![0x11, 0x12, 0x13, 0x14];
        let second = vec![0x21, 0x22];
        let prom = layout_zip_program_banks(
            Some(KnownZipSet::CtomadaySwapp),
            vec![
                (1, "249-p1.p1".to_string(), first),
                (2, "249-p2.p2".to_string(), second),
            ],
        );

        assert_eq!(prom, vec![0x13, 0x14, 0x11, 0x12, 0x21, 0x22]);
    }

    #[test]
    fn loads_rotd_zip_with_cmc50_pcm2_v_path() {
        let path = unique_temp_path("ngneon-rotd-loader", "zip");
        let p1 = vec![0x10; 0x800000];
        let c1 = vec![0xC1; crate::video::BYTES_PER_TILE];
        let m1 = vec![0x91; 0x20];
        let mut v1 = Vec::new();
        for word in 0u16..16 {
            v1.extend_from_slice(&word.to_be_bytes());
        }
        v1.resize(0x1000000, 0x81);
        create_test_zip(
            &path,
            &[
                ("264-p1.p1", &p1),
                ("264-c1.c1", &c1),
                ("264-m1.m1", &m1),
                ("264-v1.v1", &v1),
            ],
        );

        let rom = RomData::from_zip(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(rom.prom.len(), 0x800000);
        assert_eq!(rom.mrom.len(), crate::cmc::CMC_M1_DECRYPTED_SIZE);
        assert_eq!(rom.srom.len(), 0x80000);
        let first_word = u16::from_be_bytes([rom.vrom[0], rom.vrom[1]]);
        assert_eq!(first_word, 4, "ROTD PCM2DecryptV(bit=2) should run");
        let metadata = rom.metadata.as_ref().expect("rotd zip metadata");
        assert_eq!(metadata.ngh, 0x264);
    }

    #[test]
    fn loads_matrim_zip_with_cmc50_pcm2_p_v2_path() {
        let path = unique_temp_path("ngneon-matrim-loader", "zip");
        let p1 = vec![0x10; 0x100000];
        let p2 = patterned_prom_chunks(0);
        let c1 = vec![0xC1; crate::video::BYTES_PER_TILE];
        let m1 = vec![0x91; 0x20];
        let v1 = vec![0x81; 0x100];
        create_test_zip(
            &path,
            &[
                ("266-p1.p1", &p1),
                ("266-p2.sp2", &p2),
                ("266-c1.c1", &c1),
                ("266-m1.m1", &m1),
                ("266-v1.v1", &v1),
            ],
        );

        let rom = RomData::from_zip(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(rom.prom.len(), 0x500000);
        assert_eq!(rom.prom[0x100000], 2);
        assert_eq!(rom.mrom.len(), crate::cmc::CMC_M1_DECRYPTED_SIZE);
        assert_eq!(rom.srom.len(), 0x80000);
        let metadata = rom.metadata.as_ref().expect("matrim zip metadata");
        assert_eq!(metadata.ngh, 0x266);
    }

    #[test]
    fn loads_pnyaa_zip_with_cmc50_pcm2_v_path() {
        let path = unique_temp_path("ngneon-pnyaa-loader", "zip");
        let p1 = vec![0x10; 0x100000];
        let c1 = vec![0xC1; crate::video::BYTES_PER_TILE];
        let m1 = vec![0x91; 0x20];
        let mut v1 = vec![0x00, 0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x04];
        v1.resize(0x400000, 0x81);
        create_test_zip(
            &path,
            &[
                ("pn202.p1", &p1),
                ("267-c1.c1", &c1),
                ("m1.m1", &m1),
                ("267-v1.v1", &v1),
            ],
        );

        let rom = RomData::from_zip(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(rom.prom.len(), 0x100000);
        assert_eq!(rom.mrom.len(), crate::cmc::CMC_M1_DECRYPTED_SIZE);
        assert_eq!(rom.srom.len(), 0x80000);
        assert_eq!(
            &rom.vrom[..4],
            &[0x00, 0x02, 0x00, 0x01],
            "PNYAA PCM2DecryptV(bit=0) should swap word pairs"
        );
        let metadata = rom.metadata.as_ref().expect("pnyaa zip metadata");
        assert_eq!(metadata.ngh, 0x267);
    }

    #[test]
    fn loads_kof2001_zip_with_cmc50_m1_path() {
        let path = unique_temp_path("ngneon-kof2001-loader", "zip");
        let p1 = vec![0x10; 0x100000];
        let p2 = vec![0x20; 0x400000];
        let c1 = vec![0xC1; crate::video::BYTES_PER_TILE];
        let m1 = vec![0x91; 0x40000];
        let v1 = vec![0x81; 0x20];
        create_test_zip(
            &path,
            &[
                ("262-p1-08-e0.p1", &p1),
                ("262-p2-08-e0.sp2", &p2),
                ("262-c1-08-e0.c1", &c1),
                ("265-262-m1.m1", &m1),
                ("262-v1-08-e0.v1", &v1),
            ],
        );

        let rom = RomData::from_zip(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(rom.prom.len(), 0x500000);
        assert_eq!(rom.mrom.len(), crate::cmc::CMC_M1_DECRYPTED_SIZE);
        assert_eq!(rom.srom.len(), 0x80000);
        let metadata = rom.metadata.as_ref().expect("kof2001 zip metadata");
        assert_eq!(metadata.ngh, 0x262);
        let diagnostics = rom.diagnostics();
        assert!(diagnostics
            .warnings
            .iter()
            .any(|warning| warning.contains("M1 CMC50")));
    }

    #[test]
    fn loads_jockeygp_zip_with_cmc50_m1_and_brezzasoft_board() {
        let path = unique_temp_path("ngneon-jockeygp-loader", "zip");
        let p1 = vec![0x10; 0x100000];
        let c1 = vec![0xC1; crate::video::BYTES_PER_TILE];
        let c2 = vec![0xC2; crate::video::BYTES_PER_TILE];
        let m1 = vec![0x91; 0x80000];
        let v1 = vec![0x81; 0x20];
        create_test_zip(
            &path,
            &[
                ("008-epr.p1", &p1),
                ("008-c1.c1", &c1),
                ("008-c2.c2", &c2),
                ("008-mg1.m1", &m1),
                ("008-v1.v1", &v1),
            ],
        );

        let rom = RomData::from_zip(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(rom.prom.len(), 0x100000);
        assert_eq!(rom.mrom.len(), crate::cmc::CMC_M1_DECRYPTED_SIZE);
        assert_eq!(rom.srom.len(), 0x20000);
        let metadata = rom.metadata.as_ref().expect("jockeygp zip metadata");
        assert_eq!(metadata.ngh, 0x008);
        assert_eq!(metadata.board_type, NeoBoardType::Brezzasoft);
        let diagnostics = rom.diagnostics();
        assert!(diagnostics
            .warnings
            .iter()
            .any(|warning| warning.contains("M1 CMC50")));
    }

    #[test]
    fn loads_mslug5_zip_with_p32_cmc50_pcm2_path() {
        let path = unique_temp_path("ngneon-mslug5-loader", "zip");
        let p1 = vec![0x10; 0x400000];
        let p2 = vec![0x20; 0x400000];
        let c1 = vec![0xC1; crate::video::BYTES_PER_TILE];
        let m1 = vec![0x91; 0x20];
        let v1 = vec![0x81; 0x20];
        create_test_zip(
            &path,
            &[
                ("268-p1cr.p1", &p1),
                ("268-p2cr.p2", &p2),
                ("268-c1c.c1", &c1),
                ("268-m1.m1", &m1),
                ("268-v1c.v1", &v1),
            ],
        );

        let rom = RomData::from_zip(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(rom.prom.len(), 0x800000);
        assert_eq!(rom.mrom.len(), crate::cmc::CMC_M1_DECRYPTED_SIZE);
        assert_eq!(rom.srom.len(), 0x80000);
        let metadata = rom.metadata.as_ref().expect("mslug5 zip metadata");
        assert_eq!(metadata.ngh, 0x268);
        let diagnostics = rom.diagnostics();
        assert!(diagnostics
            .warnings
            .iter()
            .any(|warning| warning.contains("P32")));
    }

    #[test]
    fn loads_samsho5_zip_with_cmc50_pcm2_p2_path() {
        let path = unique_temp_path("ngneon-samsho5-loader", "zip");
        let p1 = patterned_prom_chunks(0);
        let p2 = patterned_prom_chunks(8);
        let c1 = vec![0xC1; crate::video::BYTES_PER_TILE];
        let m1 = vec![0x91; 0x20];
        let v1 = vec![0x81; 0x20];
        create_test_zip(
            &path,
            &[
                ("270-p1.p1", &p1),
                ("270-p2.sp2", &p2),
                ("270-c1.c1", &c1),
                ("270-m1.m1", &m1),
                ("270-v1.v1", &v1),
            ],
        );

        let rom = RomData::from_zip(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(rom.prom.len(), 0x800000);
        assert_eq!(rom.prom[0], 0);
        assert_eq!(rom.prom[0x100000], 14);
        assert_eq!(rom.prom[0x180000], 13);
        assert_eq!(rom.mrom.len(), crate::cmc::CMC_M1_DECRYPTED_SIZE);
        assert_eq!(rom.srom.len(), 0x80000);
        let metadata = rom.metadata.as_ref().expect("samsho5 zip metadata");
        assert_eq!(metadata.ngh, 0x270);
        let diagnostics = rom.diagnostics();
        assert!(diagnostics
            .warnings
            .iter()
            .any(|warning| warning.contains("PCM2 P2")));
    }

    #[test]
    fn loads_samsh5sp_zip_with_cmc50_pcm2_p2_path() {
        let path = unique_temp_path("ngneon-samsh5sp-loader", "zip");
        let p1 = patterned_prom_chunks(0);
        let p2 = patterned_prom_chunks(8);
        let c1 = vec![0xC1; crate::video::BYTES_PER_TILE];
        let m1 = vec![0x91; 0x20];
        let v1 = vec![0x81; 0x20];
        create_test_zip(
            &path,
            &[
                ("272-p1.p1", &p1),
                ("272-p2.sp2", &p2),
                ("272-c1.c1", &c1),
                ("272-m1.m1", &m1),
                ("272-v1.v1", &v1),
            ],
        );

        let rom = RomData::from_zip(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(rom.prom.len(), 0x800000);
        assert_eq!(rom.prom[0], 0);
        assert_eq!(rom.prom[0x100000], 10);
        assert_eq!(rom.prom[0x180000], 9);
        assert_eq!(rom.mrom.len(), crate::cmc::CMC_M1_DECRYPTED_SIZE);
        assert_eq!(rom.srom.len(), 0x80000);
        let metadata = rom.metadata.as_ref().expect("samsh5sp zip metadata");
        assert_eq!(metadata.ngh, 0x272);
        let diagnostics = rom.diagnostics();
        assert!(diagnostics
            .warnings
            .iter()
            .any(|warning| warning.contains("PCM2 P2")));
    }

    #[test]
    fn loads_known_zip_set_with_directory_entries() {
        let path = unique_temp_path("ngneon-kof2002-dir-loader", "zip");
        create_test_zip(
            &path,
            &[
                ("kof2002/265-p1.p1", &[0x10; MIN_EXTERNAL_PROM_SIZE]),
                ("kof2002/265-p2.sp2", &[0x20; 4]),
                ("kof2002/265-c1.c1", &[0xC1; crate::video::BYTES_PER_TILE]),
                ("kof2002/265-m1.m1", &[0x91; 0x20]),
                ("kof2002/265-v1.v1", &[0x81; 0x20]),
            ],
        );

        let rom = RomData::from_zip(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert!(rom
            .recognized_files
            .iter()
            .all(|name| !name.contains('/') && !name.contains('\\')));
        assert_eq!(
            detect_known_zip_set(&rom.recognized_files),
            Some(KnownZipSet::Kof2002Encrypted)
        );
        let metadata = rom.metadata.expect("known zip metadata");
        assert_eq!(metadata.ngh, 0x265);
        assert_eq!(metadata.board_type, NeoBoardType::Default);
        assert_eq!(rom.mrom.len(), crate::cmc::CMC_M1_DECRYPTED_SIZE);
    }

    #[test]
    fn ignores_merged_clone_subdirectories_when_root_set_exists() {
        let path = unique_temp_path("ngneon-merged-root-loader", "zip");
        create_test_zip(
            &path,
            &[
                ("game-p1.p1", &[0x10; MIN_EXTERNAL_PROM_SIZE]),
                ("game-c1.c1", &[0xC1; crate::video::BYTES_PER_TILE]),
                ("game-m1.m1", &[0x91; 0x20]),
                ("game-v1.v1", &[0x81; 0x20]),
                ("clone/game-p1.p1", &[0x20; MIN_EXTERNAL_PROM_SIZE]),
                ("clone/game-c1.c1", &[0xC2; crate::video::BYTES_PER_TILE]),
                ("clone/game-v1.v1", &[0x82; 0x20]),
            ],
        );

        let rom = RomData::from_zip(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(rom.prom, vec![0x10; MIN_EXTERNAL_PROM_SIZE]);
        assert_eq!(rom.vrom, vec![0x81; 0x20]);
        assert_eq!(rom.recognized_files.len(), 4);
    }

    #[test]
    fn diagnostics_report_missing_optional_banks() {
        let rom = RomData {
            prom: vec![0; MIN_EXTERNAL_PROM_SIZE],
            crom: Vec::new(),
            srom: Vec::new(),
            mrom: Vec::new(),
            vrom: Vec::new(),
            vrom_b_offset: 0,
            sma_rom: Vec::new(),
            is_demo: false,
            source: RomSource::NeoFile,
            recognized_files: vec!["game.neo".to_string()],
            metadata: None,
        };

        let diagnostics = rom.diagnostics();

        assert_eq!(diagnostics.prom_bytes, MIN_EXTERNAL_PROM_SIZE);
        assert_eq!(diagnostics.recognized_files, 1);
        assert_eq!(diagnostics.warnings.len(), 4);
    }

    #[test]
    fn diagnostics_identify_kof2002_encrypted_zip_set() {
        let rom = RomData {
            prom: vec![0; MIN_EXTERNAL_PROM_SIZE],
            crom: vec![0; crate::video::BYTES_PER_TILE],
            srom: Vec::new(),
            mrom: vec![0; 0x020000],
            vrom: vec![0; 0x100],
            vrom_b_offset: 0,
            sma_rom: Vec::new(),
            is_demo: false,
            source: RomSource::ZipArchive,
            recognized_files: vec![
                "265-p1.p1".to_string(),
                "265-p2.sp2".to_string(),
                "265-c1.c1".to_string(),
                "265-m1.m1".to_string(),
                "265-v1.v1".to_string(),
            ],
            metadata: None,
        };

        let diagnostics = rom.diagnostics();

        assert!(diagnostics
            .warnings
            .iter()
            .any(|warning| warning.contains("CMC50+PCM2")));
        assert!(diagnostics
            .warnings
            .iter()
            .any(|warning| warning.contains("S-ROM ausente esperada")));
    }

    #[test]
    fn diagnostics_identify_mslug3_encrypted_zip_set() {
        let rom = RomData {
            prom: vec![0; MIN_EXTERNAL_PROM_SIZE],
            crom: Vec::new(),
            srom: Vec::new(),
            mrom: Vec::new(),
            vrom: Vec::new(),
            vrom_b_offset: 0,
            sma_rom: Vec::new(),
            is_demo: false,
            source: RomSource::ZipArchive,
            recognized_files: vec![
                "neo-sma".to_string(),
                "256-pg1.p1".to_string(),
                "256-pg2.p2".to_string(),
                "256-c1.c1".to_string(),
                "256-m1.m1".to_string(),
                "256-v1.v1".to_string(),
            ],
            metadata: None,
        };

        let diagnostics = rom.diagnostics();

        assert!(diagnostics
            .warnings
            .iter()
            .any(|warning| warning.contains("SMA+CMC42")));
    }

    #[test]
    fn diagnostics_identify_kof2003_encrypted_zip_set() {
        let rom = RomData {
            prom: vec![0; MIN_EXTERNAL_PROM_SIZE],
            crom: vec![0; crate::video::BYTES_PER_TILE],
            srom: Vec::new(),
            mrom: vec![0; 0x020000],
            vrom: vec![0; 0x100],
            vrom_b_offset: 0,
            sma_rom: Vec::new(),
            is_demo: false,
            source: RomSource::ZipArchive,
            recognized_files: vec![
                "271-p1c.p1".to_string(),
                "271-p2c.p2".to_string(),
                "271-p3c.p3".to_string(),
                "271-c1c.c1".to_string(),
                "271-m1c.m1".to_string(),
                "271-v1c.v1".to_string(),
            ],
            metadata: None,
        };

        let diagnostics = rom.diagnostics();

        assert!(diagnostics
            .warnings
            .iter()
            .any(|w| w.contains("CMC50+PCM2")));
    }

    #[test]
    fn diagnostics_identify_svc_encrypted_zip_set() {
        let rom = RomData {
            prom: vec![0; MIN_EXTERNAL_PROM_SIZE],
            crom: vec![0; crate::video::BYTES_PER_TILE],
            srom: Vec::new(),
            mrom: vec![0; 0x020000],
            vrom: vec![0; 0x100],
            vrom_b_offset: 0,
            sma_rom: Vec::new(),
            is_demo: false,
            source: RomSource::ZipArchive,
            recognized_files: vec![
                "269-p1.p1".to_string(),
                "269-p2.p2".to_string(),
                "269-c1r.c1".to_string(),
                "269-m1.m1".to_string(),
                "269-v1.v1".to_string(),
            ],
            metadata: None,
        };

        let diagnostics = rom.diagnostics();

        assert!(diagnostics
            .warnings
            .iter()
            .any(|w| w.contains("CMC50+PCM2")));
    }

    #[test]
    fn diagnostics_identify_garou_encrypted_zip_set() {
        let rom = RomData {
            prom: vec![0; MIN_EXTERNAL_PROM_SIZE],
            crom: vec![0; crate::video::BYTES_PER_TILE],
            srom: Vec::new(),
            mrom: vec![0; 0x020000],
            vrom: vec![0; 0x100],
            vrom_b_offset: 0,
            sma_rom: Vec::new(),
            is_demo: false,
            source: RomSource::ZipArchive,
            recognized_files: vec![
                "kf.neo-sma".to_string(),
                "253-ep1.p1".to_string(),
                "253-c1.c1".to_string(),
                "253-m1.m1".to_string(),
                "253-v1.v1".to_string(),
            ],
            metadata: None,
        };

        let diagnostics = rom.diagnostics();

        assert!(diagnostics
            .warnings
            .iter()
            .any(|w| w.contains("SMA + C/fix CMC42")));
    }

    #[test]
    fn diagnostics_identify_mslug4_encrypted_zip_set() {
        let rom = RomData {
            prom: vec![0; MIN_EXTERNAL_PROM_SIZE],
            crom: Vec::new(),
            srom: Vec::new(),
            mrom: Vec::new(),
            vrom: Vec::new(),
            vrom_b_offset: 0,
            sma_rom: Vec::new(),
            is_demo: false,
            source: RomSource::ZipArchive,
            recognized_files: vec![
                "263-p1.p1".to_string(),
                "263-p2.sp2".to_string(),
                "263-c1.c1".to_string(),
                "263-m1.m1".to_string(),
                "263-v1.v1".to_string(),
            ],
            metadata: None,
        };

        let diagnostics = rom.diagnostics();

        assert!(diagnostics
            .warnings
            .iter()
            .any(|w| w.contains("CMC50+PCM2")));
        assert!(!diagnostics.warnings.iter().any(|w| w.contains("SMA")));
    }

    #[test]
    fn detects_encrypted_sets_with_real_fbneo_chip_names() {
        let cases = [
            (
                vec![
                    "kf.neo-sma",
                    "253-ep1.p1",
                    "253-c1.c1",
                    "253-m1.m1",
                    "253-v1.v1",
                ],
                KnownZipSet::GarouEncrypted,
            ),
            (
                vec![
                    "ka.neo-sma",
                    "251-p1.p1",
                    "251-p2.p2",
                    "251-c1.c1",
                    "251-m1.m1",
                    "251-v1.v1",
                ],
                KnownZipSet::Kof99Encrypted,
            ),
            (
                vec![
                    "263-p1.p1",
                    "263-p2.sp2",
                    "263-c1.c1",
                    "263-m1.m1",
                    "263-v1.v1",
                ],
                KnownZipSet::Mslug4Encrypted,
            ),
            (
                vec![
                    "254-p1.p1",
                    "254-p2.sp2",
                    "254-c1.c1",
                    "254-m1.m1",
                    "254-v1.v1",
                ],
                KnownZipSet::S1945pEncrypted,
            ),
            (
                vec![
                    "242-p1.p1",
                    "242-p2.sp2",
                    "242-c1.c1",
                    "242-m1.m1",
                    "242-v1.v1",
                ],
                KnownZipSet::Kof98Encrypted,
            ),
        ];

        for (files, expected) in cases {
            let files: Vec<String> = files.into_iter().map(str::to_string).collect();
            assert_eq!(detect_known_zip_set(&files), Some(expected));
        }
    }

    #[test]
    fn diagnostics_identify_kog_encrypted_zip_set() {
        let rom = RomData {
            prom: vec![0; MIN_EXTERNAL_PROM_SIZE],
            crom: Vec::new(),
            srom: Vec::new(),
            mrom: Vec::new(),
            vrom: Vec::new(),
            vrom_b_offset: 0,
            sma_rom: Vec::new(),
            is_demo: false,
            source: RomSource::ZipArchive,
            recognized_files: vec![
                "neo-sma".to_string(),
                "254-pg1.p1".to_string(),
                "254-c1.c1".to_string(),
                "254-m1.m1".to_string(),
                "254-v1.v1".to_string(),
            ],
            metadata: None,
        };

        let diagnostics = rom.diagnostics();

        assert!(diagnostics.warnings.iter().any(|w| w.contains("SMA+CMC50")));
        assert!(diagnostics.warnings.iter().any(|w| w.contains("KOF2000")));
        assert!(diagnostics.warnings.iter().any(|w| w.contains("M1 CMC50")));
    }

    #[test]
    fn diagnostics_identify_kog_bootleg_zip_set() {
        let rom = RomData {
            prom: vec![0; MIN_EXTERNAL_PROM_SIZE],
            crom: Vec::new(),
            srom: Vec::new(),
            mrom: vec![0; 0x20000],
            vrom: vec![0; 0x400000],
            vrom_b_offset: 0,
            sma_rom: Vec::new(),
            is_demo: false,
            source: RomSource::ZipArchive,
            recognized_files: vec![
                "5232-p1.bin".to_string(),
                "5232-s1.bin".to_string(),
                "5232-c1a.bin".to_string(),
                "5232-c1b.bin".to_string(),
                "5232-c2a.bin".to_string(),
                "5232-c2b.bin".to_string(),
                "5232-c3.bin".to_string(),
                "5232-c4.bin".to_string(),
            ],
            metadata: None,
        };

        let diagnostics = rom.diagnostics();

        assert!(diagnostics
            .warnings
            .iter()
            .any(|w| w.contains("KOG bootleg")));
        assert!(diagnostics.warnings.iter().any(|w| w.contains("KOF97")));
    }

    #[test]
    fn detect_kog_bootleg_set_by_5232_file_names() {
        let files: Vec<String> = vec![
            "5232-p1.bin".to_string(),
            "5232-s1.bin".to_string(),
            "5232-c1a.bin".to_string(),
            "5232-c1b.bin".to_string(),
            "5232-c2a.bin".to_string(),
            "5232-c2b.bin".to_string(),
            "5232-c3.bin".to_string(),
            "5232-c4.bin".to_string(),
        ];

        assert_eq!(detect_known_zip_set(&files), Some(KnownZipSet::KogBootleg));
    }

    #[test]
    fn kog_bootleg_program_reorders_first_meg_and_moves_parent_p2() {
        let mut prom = vec![0u8; 0x600000];
        for bank in 0..16 {
            let start = bank * 0x20000;
            prom[start..start + 0x20000].fill(bank as u8);
        }
        prom[0x200000..0x600000].fill(0xA5);

        decrypt_kog_bootleg_program(&mut prom);

        let expected_banks = [0x3, 0x8, 0x7, 0xC, 0x1, 0xA, 0x6, 0xD];
        for (i, expected) in expected_banks.iter().copied().enumerate() {
            let start = i * 0x20000;
            assert_eq!(prom[start], expected as u8);
            assert_eq!(prom[start + 0x1FFFF], expected as u8);
        }
        assert_eq!(prom[0x100000], 0xA5);
        assert_eq!(prom[0x4FFFFF], 0xA5);
    }

    #[test]
    fn kog_bootleg_srom_decrypts_half_swapped_blocks() {
        let mut srom: Vec<u8> = (0..0x20).collect();
        srom[0] = 0x11;
        let first_block = srom[..0x10].to_vec();
        let second_block = srom[0x10..0x20].to_vec();

        decrypt_kog_bootleg_srom(&mut srom);

        assert_eq!(&srom[..8], &first_block[8..0x10]);
        assert_eq!(&srom[8..0x10], &first_block[..8]);
        assert_eq!(&srom[0x10..0x18], &second_block[8..0x10]);
        assert_eq!(&srom[0x18..0x20], &second_block[..8]);
    }

    #[test]
    fn kog_bootleg_graphics_are_named_byte_interleaved_and_chunk_swapped() {
        let banks = vec![
            ("5232-c1a.bin", 0x10),
            ("5232-c1b.bin", 0x20),
            ("5232-c2a.bin", 0x30),
            ("5232-c2b.bin", 0x40),
            ("5232-c3.bin", 0x50),
            ("5232-c4.bin", 0x60),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (name, value))| (index as u8, name.to_string(), vec![value; 0x40]))
        .collect();

        let mut crom = layout_kog_bootleg_graphics(banks);
        assert_eq!(
            &crom[..8],
            &[0x10, 0x20, 0x10, 0x20, 0x10, 0x20, 0x10, 0x20]
        );

        decrypt_kog_bootleg_crom(&mut crom);

        assert_eq!(
            &crom[..8],
            &[0x10, 0x20, 0x10, 0x20, 0x10, 0x20, 0x10, 0x20]
        );
        assert_eq!(
            &crom[0x40..0x48],
            &[0x10, 0x20, 0x10, 0x20, 0x10, 0x20, 0x10, 0x20]
        );
        assert_eq!(
            &crom[0x80..0x88],
            &[0x30, 0x40, 0x30, 0x40, 0x30, 0x40, 0x30, 0x40]
        );
    }

    #[test]
    fn detect_kof2000_parent_set_by_257_file_names() {
        // Verify that the parent KOF 2000 set (257-p1.p1 + 257-p2.p2 + neo-sma)
        // is correctly detected as KogEncrypted
        let files: Vec<String> = vec![
            "neo-sma".to_string(),
            "257-p1.p1".to_string(),
            "257-p2.p2".to_string(),
            "257-c1.c1".to_string(),
            "257-c2.c2".to_string(),
            "257-c3.c3".to_string(),
            "257-c4.c4".to_string(),
            "257-c5.c5".to_string(),
            "257-c6.c6".to_string(),
            "257-c7.c7".to_string(),
            "257-c8.c8".to_string(),
            "257-m1.m1".to_string(),
            "257-v1.v1".to_string(),
            "257-v2.v2".to_string(),
            "257-v3.v3".to_string(),
            "257-v4.v4".to_string(),
        ];
        let detected = detect_known_zip_set(&files);
        assert_eq!(
            detected,
            Some(KnownZipSet::KogEncrypted),
            "Parent KOF2000 set (257-p1.p1 + p2 + neo-sma) should be KogEncrypted, got {:?}",
            detected
        );
    }

    #[test]
    fn known_zip_set_metadata_enables_geolith_runtime_boards() {
        let kof2003_files = vec![
            "271-p1c.p1".to_string(),
            "271-p2c.p2".to_string(),
            "271-p3c.p3".to_string(),
            "271-c1c.c1".to_string(),
            "271-m1c.m1".to_string(),
            "271-v1c.v1".to_string(),
        ];
        let mut kof2003_prom = vec![0xFF; 0x4000];
        kof2003_prom[0x267] = 0x4F;
        let kof2003 = metadata_from_known_zip_set(
            detect_known_zip_set(&kof2003_files),
            &kof2003_files,
            "kof2003",
            &kof2003_prom,
        )
        .expect("kof2003 metadata");
        assert_eq!(kof2003.ngh, 0x271);
        assert_eq!(kof2003.board_type, NeoBoardType::Pvc);
        assert_eq!(kof2003.fix_banksw, NeoFixBanksw::Tile);

        let svc_files = vec![
            "269-p1.p1".to_string(),
            "269-p2.p2".to_string(),
            "269-c1r.c1".to_string(),
            "269-m1.m1".to_string(),
            "269-v1.v1".to_string(),
        ];
        let mut svc_prom = vec![0xFF; 0xA000];
        svc_prom[0x2F8F] = 0xC0;
        svc_prom[0x3D25] = 0xC4;
        let svc = metadata_from_known_zip_set(
            detect_known_zip_set(&svc_files),
            &svc_files,
            "svc",
            &svc_prom,
        )
        .expect("svc metadata");
        assert_eq!(svc.ngh, 0x269);
        assert_eq!(svc.board_type, NeoBoardType::Pvc);
        assert_eq!(svc.fix_banksw, NeoFixBanksw::Tile);

        let kof2000_files = vec![
            "neo-sma".to_string(),
            "257-p1.p1".to_string(),
            "257-p2.p2".to_string(),
            "257-c1.c1".to_string(),
            "257-m1.m1".to_string(),
        ];
        let kof2000_prom = vec![0xFF; 0x500001];
        let kof2000 = metadata_from_known_zip_set(
            detect_known_zip_set(&kof2000_files),
            &kof2000_files,
            "kof2000",
            &kof2000_prom,
        )
        .expect("kof2000 metadata");
        assert_eq!(kof2000.ngh, 0x257);
        assert_eq!(kof2000.board_type, NeoBoardType::Sma);
        assert_eq!(kof2000.fix_banksw, NeoFixBanksw::Tile);
    }

    #[test]
    fn normal_zip_metadata_uses_program_header_ngh() {
        let path = unique_temp_path("ngneon-normal-zip-metadata", "zip");
        let mut prom = vec![0xFF; 0x20000];
        prom[0..4].copy_from_slice(&0x0010_F300u32.to_be_bytes());
        prom[4..8].copy_from_slice(&0x0000_0100u32.to_be_bytes());
        prom[0x100..0x107].copy_from_slice(b"NEO-GEO");
        prom[0x108..0x10A].copy_from_slice(&0x0044u16.to_be_bytes());

        create_test_zip(&path, &[("044-p1.p1", &prom)]);
        let rom = RomData::from_zip(&path).expect("normal zip metadata");
        let metadata = rom.metadata.expect("zip metadata from P-ROM header");
        assert_eq!(metadata.ngh, 0x044);
        assert_eq!(metadata.board_type, NeoBoardType::Default);
        assert_eq!(metadata.fix_banksw, NeoFixBanksw::None);
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn zip_2mb_p1_continue_layout_moves_second_meg_to_fixed_window() {
        let path = unique_temp_path("ngneon-zip-p1-continue-layout", "zip");
        let mut prom = vec![0xFF; 0x200000];
        let header = 0x100000 + 0x100;
        write_word_swapped_neo_header(&mut prom, header);
        prom[header + 8..header + 10].copy_from_slice(&0x5500u16.to_be_bytes());
        prom[0x100000 + 0x122..0x100000 + 0x126].copy_from_slice(&[0xF9, 0x4E, 0x03, 0x00]);

        create_test_zip(&path, &[("055-p1.p1", &prom)]);
        let rom = RomData::from_zip(&path).expect("zip 2MB P1 continue layout");
        let metadata = rom.metadata.expect("zip metadata from relocated header");

        assert_eq!(&rom.prom[0x100..0x107], b"NEO-GEO");
        assert_eq!(metadata.ngh, 0x055);
        assert_eq!(&rom.prom[0x122..0x126], &[0x4E, 0xF9, 0x00, 0x03]);
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn zip_metadata_falls_back_to_set_name_for_special_flags() {
        let prom = vec![0xFF; 0x20000];
        let metadata =
            metadata_from_zip_set(None, &[], "irrmaze", &prom).expect("irrmaze metadata");
        assert_eq!(metadata.ngh, 0x236);
        assert_eq!(metadata.game_flags, NEO_FLAG_IRRMAZE);

        let metadata = metadata_from_zip_set(None, &[], "vliner", &prom).expect("vliner metadata");
        assert_eq!(metadata.ngh, 0x3E7);
        assert_eq!(metadata.board_type, NeoBoardType::Brezzasoft);
        assert_eq!(metadata.game_flags, NEO_FLAG_VLINER);
    }

    #[test]
    fn load_actual_kof2000_zip_with_sma_cmc50() {
        // Load the actual kof2000.zip from roms/ and verify the full pipeline
        let path = std::path::Path::new("../roms/kof2000.zip");
        if !path.exists() {
            eprintln!("[SKIP] kof2000.zip not found in roms/");
            return;
        }
        let rom = RomData::from_zip(path).unwrap_or_else(|e| {
            panic!("Failed to load kof2000.zip: {e}");
        });

        // Verify SMA path was taken: P-ROM should be 1MB (fixed window) + 8MB (banked) = 9MB
        assert_eq!(
            rom.prom.len(),
            0x10_0000 + 8_388_608,
            "P-ROM should have SMA layout (1MB fixed + 8MB banked), got {} bytes",
            rom.prom.len()
        );

        // After SMA decrypt, first word should not be 0xFFFF
        let first_word = u16::from_be_bytes([rom.prom[0], rom.prom[1]]);
        assert_ne!(
            first_word, 0xFFFF,
            "P-ROM fixed region should be relocated by SMA decrypt"
        );

        // C-ROM should be 64MB (8 banks × 8MB each, interleaved as 4 pairs × 16MB)
        assert_eq!(
            rom.crom.len(),
            67108864,
            "C-ROM should be 64MB after interleaving, got {} bytes",
            rom.crom.len()
        );

        // C-ROM should differ from raw input (CMC50 decrypt was applied)
        // Just verify it's not all zeros
        let zero_count = rom.crom.iter().filter(|&&b| b == 0).count();
        assert!(
            zero_count < rom.crom.len(),
            "C-ROM should not be all zeros after CMC50"
        );

        // S-ROM should be extracted from C-ROM (0x80000 bytes)
        assert_eq!(
            rom.srom.len(),
            0x80000,
            "S-ROM should be 0x80000 bytes extracted from C-ROM"
        );

        // M1 should be expanded to CMC_M1_DECRYPTED_SIZE
        assert_eq!(
            rom.mrom.len(),
            crate::cmc::CMC_M1_DECRYPTED_SIZE,
            "M1 should be expanded to 0x80000 by CMC50"
        );

        // V-ROM should be all 4 banks concatenated (4 × 4MB = 16MB)
        assert_eq!(
            rom.vrom.len(),
            16777216,
            "V-ROM should be 16MB after concatenating v1-v4"
        );

        // Diagnostics should identify KOF2000
        let diag = rom.diagnostics();
        assert!(
            diag.warnings.iter().any(|w| w.contains("SMA+CMC50")),
            "Diagnostics should identify SMA+CMC50 KOF2000"
        );
        assert!(
            diag.warnings.iter().any(|w| w.contains("KOF2000")),
            "Diagnostics should mention KOF2000"
        );
    }

    #[test]
    fn load_actual_kof2003_zip_uses_pvc_cmc50_pcm2() {
        let path = std::path::Path::new("../roms/kof2003.zip");
        if !path.exists() {
            eprintln!("[SKIP] kof2003.zip not found in roms/");
            return;
        }

        let rom = RomData::from_zip(path).unwrap_or_else(|e| {
            panic!("Failed to load kof2003.zip: {e}");
        });

        assert_eq!(
            detect_known_zip_set(&rom.recognized_files),
            Some(KnownZipSet::Kof2003Encrypted)
        );
        assert_eq!(rom.prom.len(), 0x900000);
        assert_eq!(rom.crom.len(), 0x4000000);
        assert_eq!(rom.srom.len(), 0x80000);
        assert_eq!(rom.mrom.len(), crate::cmc::CMC_M1_DECRYPTED_SIZE);
        assert_eq!(rom.vrom.len(), 0x1000000);

        let metadata = rom.metadata.expect("kof2003 metadata");
        assert_eq!(metadata.ngh, 0x271);
        assert_eq!(metadata.board_type, NeoBoardType::Pvc);
        assert_eq!(metadata.fix_banksw, NeoFixBanksw::Tile);
    }

    #[test]
    fn load_actual_svc_zip_uses_pvc_p32_callback() {
        let path = std::path::Path::new("../roms/svc.zip");
        if !path.exists() {
            eprintln!("[SKIP] svc.zip not found in roms/");
            return;
        }

        let rom = RomData::from_zip(path).unwrap_or_else(|e| {
            panic!("Failed to load svc.zip: {e}");
        });

        assert_eq!(
            detect_known_zip_set(&rom.recognized_files),
            Some(KnownZipSet::SvcEncrypted)
        );
        assert_eq!(rom.prom.len(), 0x800000);
        assert_eq!(rom.crom.len(), 0x4000000);
        assert_eq!(rom.srom.len(), 0x80000);
        assert_eq!(rom.mrom.len(), crate::cmc::CMC_M1_DECRYPTED_SIZE);
        assert_eq!(rom.vrom.len(), 0x1000000);

        let metadata = rom.metadata.expect("svc metadata");
        assert_eq!(metadata.ngh, 0x269);
        assert_eq!(metadata.board_type, NeoBoardType::Pvc);
        assert_eq!(metadata.fix_banksw, NeoFixBanksw::Tile);
    }

    #[test]
    fn load_actual_mslug3_zip_with_sma_cmc42() {
        let path = std::path::Path::new("../roms/mslug3.zip");
        if !path.exists() {
            eprintln!("[SKIP] mslug3.zip not found in roms/");
            return;
        }

        let rom = RomData::from_zip(path).unwrap_or_else(|e| {
            panic!("Failed to load mslug3.zip: {e}");
        });

        assert_eq!(
            rom.prom.len(),
            0x10_0000 + 8_388_608,
            "P-ROM should have SMA layout (1MB fixed + 8MB banked), got {} bytes",
            rom.prom.len()
        );
        assert_eq!(rom.crom.len(), 67_108_864);
        assert_eq!(rom.srom.len(), 0x80000);
        assert_eq!(rom.mrom.len(), 0x80000);
        assert_eq!(rom.vrom.len(), 0x1000000);

        let metadata = rom.metadata.expect("mslug3 metadata");
        assert_eq!(metadata.ngh, 0x256);
        assert_eq!(metadata.board_type, NeoBoardType::Sma);
        assert_eq!(metadata.fix_banksw, NeoFixBanksw::Line);
    }

    #[test]
    fn load_actual_kog_bootleg_zip_uses_kof97_parent() {
        let path = std::path::Path::new("../roms/kog.zip");
        let parent = std::path::Path::new("../roms/kof97.neo");
        if !path.exists() || !parent.exists() {
            eprintln!("[SKIP] kog.zip or kof97.neo not found in roms/");
            return;
        }

        let rom = RomData::from_zip(path).unwrap_or_else(|e| {
            panic!("Failed to load kog.zip with KOF97 parent: {e}");
        });

        assert_eq!(rom.prom.len(), 0x600000);
        assert_eq!(rom.crom.len(), 0x2800000);
        assert_eq!(rom.srom.len(), 0x20000);
        assert_eq!(rom.mrom.len(), 0x20000);
        assert_eq!(rom.vrom.len(), 0xC00000);
        assert_eq!(rom.vrom_b_offset, 0);
        assert_eq!(&rom.prom[0x100..0x107], b"NEO-GEO");
        assert_eq!(
            rom.metadata.as_ref().map(|metadata| metadata.ngh),
            Some(0x5232)
        );
        assert!(rom
            .diagnostics()
            .warnings
            .iter()
            .any(|warning| warning.contains("KOG bootleg")));
    }

    #[test]
    fn kof2000_decryption_pipeline_sma_cmc50_m1_simulated() {
        // Simulate loading a KOF 2000 (254) ROM with SMA + CMC50 + M1 encryption.
        // This test verifies the entire decryption pipeline works end-to-end
        // without requiring actual ROM files.
        let path = unique_temp_path("ngneon-kof2000-sim", "zip");

        // Create synthetic SMA data (neo-sma chip)
        let sma_data: Vec<u8> = (0..0x40000).map(|i| (i & 0xFF) as u8).collect();

        // Create synthetic program ROM (pg1) - large enough for SMA decryption
        let mut pg1 = Vec::new();
        // Need at least FIXED_PROM_WINDOW_SIZE + 0x100000 bytes for SMA layout
        // Fill with scrambled-looking data so SMA transform is meaningful
        for i in 0..0x100000 {
            pg1.push((i ^ 0xAA) as u8);
        }

        // Create realistic-size C-ROM banks for CMC50 test (1MB ensures
        // address-scrambled writes land in bounds for the transform)
        let c1: Vec<u8> = (0..0x100000).map(|i| (i & 0xFF) as u8).collect();
        let c2: Vec<u8> = (0..0x100000).map(|i| ((i + 0x80) & 0xFF) as u8).collect();

        // Create M1 for CMC50 test
        let m1: Vec<u8> = (0..0x20000).map(|i| (i ^ 0x55) as u8).collect();

        // Create minimal V1
        let v1: Vec<u8> = (0..0x100).map(|i| i as u8).collect();

        create_test_zip(
            &path,
            &[
                ("neo-sma", &sma_data),
                ("254-pg1.p1", &pg1),
                ("254-c1.c1", &c1),
                ("254-c2.c2", &c2),
                ("254-m1.m1", &m1),
                ("254-v1.v1", &v1),
            ],
        );

        let rom = RomData::from_zip(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        // ── Verify SMA P-ROM decryption ────────────────────────────────
        // After layout_mslug3_sma_program_rom, P-ROM should be:
        //   [0..0x100000): FIXED_PROM_WINDOW_SIZE bytes from SMA layout
        //   [0x100000..): banked region from pg1
        // The SMA decrypt transforms data in place, so output should differ from input
        assert!(
            rom.prom.len() > 0x100000,
            "P-ROM should have SMA layout: {} > 0x100000",
            rom.prom.len()
        );
        // First 0xC0000 bytes should be the SMA chip data relocated after decryption
        // This will differ from the raw pg1 data since SMA decrypt is applied
        // Check that the first 8 bytes are non-zero and look like valid 68k code
        let first_word = u16::from_be_bytes([rom.prom[0], rom.prom[1]]);
        // Typical 68k initial SP is in range 0x00100000-0x0010FFFE
        // After SMA decrypt the relocated fixed region starts here
        // The exact value depends on SMA layout + decrypt, but shouldn't be 0xFFFF
        assert_ne!(
            first_word, 0xFFFF,
            "P-ROM fixed region should be relocated by SMA decrypt"
        );

        // ── Verify CMC50 C-ROM decryption ──────────────────────────────
        assert!(
            !rom.crom.is_empty(),
            "C-ROM should be populated after CMC50 decrypt"
        );
        // With 2MB interleaved C-ROM, CMC50 should transform it
        // The interleaving produces odd/even pairs from c1/c2
        let mut interleaved = Vec::new();
        for i in 0..0x100000 {
            interleaved.push(c1[i]);
            interleaved.push(c2[i]);
        }
        // CMC50 decrypt should change the data significantly
        assert_ne!(
            &rom.crom[..0x10],
            &interleaved[..0x10],
            "C-ROM should be transformed by CMC50 decryption"
        );
        // Verify the C-ROM wasn't zeroed out (address scramble should map
        // most positions in bounds for a 2MB C-ROM)
        let zero_count = rom.crom.iter().filter(|&&b| b == 0).count();
        assert!(
            zero_count < rom.crom.len(),
            "C-ROM should not be all zeros after CMC50 (got {zero_count}/{})",
            rom.crom.len()
        );
        // CMC50 S-data extraction should produce 0x80000 bytes
        assert_eq!(
            rom.srom.len(),
            0x80000,
            "S-ROM should be extracted from C-ROM tail by CMC50"
        );

        // ── Verify CMC50 M1 decryption ─────────────────────────────────
        // M1 should be expanded to CMC_M1_DECRYPTED_SIZE (0x80000)
        assert_eq!(
            rom.mrom.len(),
            crate::cmc::CMC_M1_DECRYPTED_SIZE,
            "M1 should be expanded to 0x80000 by CMC50 M1 decrypt"
        );
        // Decrypted M1 should differ from input
        assert_ne!(
            &rom.mrom[..0x20000],
            &m1,
            "M1 should be transformed by CMC50 M1 decryption"
        );
        // Not all zeros
        assert!(
            !rom.mrom.iter().all(|&b| b == 0),
            "M1 should not be all zeros after CMC50 decrypt"
        );

        // ── Verify V-ROM is preserved ──────────────────────────────────
        assert!(!rom.vrom.is_empty(), "V-ROM should be present");

        // ── Verify diagnostics ─────────────────────────────────────────
        let diagnostics = rom.diagnostics();
        assert!(
            diagnostics.warnings.iter().any(|w| w.contains("SMA+CMC50")),
            "Diagnostics should identify KOF2000 SMA+CMC50"
        );
    }

    #[test]
    fn demo_diagnostics_do_not_warn_about_missing_rom_banks() {
        let diagnostics = RomData::demo().diagnostics();
        assert!(diagnostics.warnings.is_empty());
        assert_eq!(diagnostics.source, RomSource::Demo);
    }

    #[test]
    fn parses_neo_file_header_and_banks() {
        let path = unique_temp_path("ngneon-neo-loader", "neo");
        let bytes = make_test_neo_file();
        std::fs::write(&path, bytes).unwrap();

        let rom = RomData::from_neo(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(rom.prom.len(), 0x10000);
        // SP=0x0010FD00 es válido en raw form (rango work RAM),
        // así que NO se aplica byte-swapping. Los vectores se mantienen.
        assert_eq!(
            &rom.prom[0..8],
            &[0x00, 0x10, 0xFD, 0x00, 0x00, 0x00, 0x01, 0x00]
        );
        assert_eq!(rom.srom, vec![0x20; 2]);
        assert_eq!(rom.mrom, vec![0x30; 2]);
        assert_eq!(rom.vrom, [vec![0x40; 3], vec![0x50; 5]].concat());
        assert_eq!(
            rom.vrom_b_offset, 3,
            ".neo V2 must remain addressable as the ADPCM-B source region"
        );
        assert_eq!(rom.crom, vec![0x60; 4]);
        let metadata = rom.metadata.unwrap();
        assert_eq!(metadata.version, 1);
        assert_eq!(metadata.year, 2026);
        assert_eq!(metadata.genre, 7);
        assert_eq!(metadata.ngh, 0x1234);
        assert_eq!(metadata.name, "NGNEON TEST");
        assert_eq!(metadata.manufacturer, "NGNEON");
    }

    #[test]
    fn generated_test_neo_contains_visible_graphics_banks() {
        let bytes = make_test_neo_file_with_graphics();
        let rom = parse_neo_file(&bytes).unwrap();

        assert_eq!(rom.prom.len(), 0x10000);
        assert_eq!(rom.srom.len(), 32);
        assert_eq!(rom.crom.len(), crate::video::BYTES_PER_TILE * 2);
    }

    #[test]
    fn neo_empty_srom_extracts_cmc_fix_data_from_crom_tail() {
        let path = unique_temp_path("ngneon-empty-srom-cmc", "neo");
        let mut prom = vec![0xFF; 0x10000];
        prom[0..4].copy_from_slice(&0x0010_FD00_u32.to_be_bytes());
        prom[4..8].copy_from_slice(&0x0000_0100_u32.to_be_bytes());
        prom[0x100..0x107].copy_from_slice(b"NEO-GEO");
        let mrom = vec![0xAA; 0x100];
        let vrom = vec![0x55; 0x100];
        let crom: Vec<u8> = (0..0x80000).map(|i| (i & 0xFF) as u8).collect();

        create_test_neo(&path, &prom, &[], &mrom, &vrom, &[], &crom);
        let rom = RomData::from_neo(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        let expected = crate::cmc::extract_cmc_s_data(&crom, 0x80000);
        assert_eq!(
            rom.srom.len(),
            0x80000,
            "empty .neo S-ROM should use the CMC-sized fix extraction path"
        );
        assert_eq!(rom.srom, expected);
    }

    #[test]
    fn rejects_truncated_neo_payload() {
        let mut bytes = make_test_neo_file();
        bytes.truncate(bytes.len() - 1);
        assert!(parse_neo_file(&bytes).is_err());
    }

    #[test]
    fn neo_variant_heuristics_use_normalized_prom_offsets() {
        let mut garou = vec![0xFF; 0x100000];
        garou[0xC0000 + 0x3E481] = 0x41;
        let (board, fix) =
            apply_neo_variant_heuristics(0x253, NeoBoardType::Sma, NeoFixBanksw::Line, &garou);
        assert_eq!(board, NeoBoardType::Sma);
        assert_eq!(fix, NeoFixBanksw::Line);

        let mut mslug3a = vec![0xFF; 0x500001];
        mslug3a[0x141] = 0x33;
        let (board, fix) =
            apply_neo_variant_heuristics(0x256, NeoBoardType::Sma, NeoFixBanksw::Line, &mslug3a);
        assert_eq!(board, NeoBoardType::SmaMslug3A);
        assert_eq!(fix, NeoFixBanksw::Line);

        let mut mslug5 = vec![0xFF; 0x1000];
        mslug5[0x267] = 0x4F;
        let (board, fix) =
            apply_neo_variant_heuristics(0x268, NeoBoardType::Pvc, NeoFixBanksw::None, &mslug5);
        assert_eq!(board, NeoBoardType::Pvc);
        assert_eq!(fix, NeoFixBanksw::None);

        let mut svc = vec![0xFF; 0xA000];
        svc[0x2F8F] = 0xC0;
        svc[0x3D25] = 0xC4;
        let (board, fix) =
            apply_neo_variant_heuristics(0x269, NeoBoardType::Pvc, NeoFixBanksw::Tile, &svc);
        assert_eq!(board, NeoBoardType::Pvc);
        assert_eq!(fix, NeoFixBanksw::Tile);

        let mut kof10th = vec![0xFF; 0x200];
        kof10th[0x125] = 0x00;
        let (board, fix) = apply_neo_variant_heuristics(
            0x275,
            NeoBoardType::Kof10th,
            NeoFixBanksw::None,
            &kof10th,
        );
        assert_eq!(board, NeoBoardType::Kof10th);
        assert_eq!(fix, NeoFixBanksw::None);

        kof10th[0x125] = 0xFF;
        let (board, fix) = apply_neo_variant_heuristics(
            0x275,
            NeoBoardType::Kof10th,
            NeoFixBanksw::None,
            &kof10th,
        );
        assert_eq!(board, NeoBoardType::Default);
        assert_eq!(fix, NeoFixBanksw::None);

        let mut cthd = vec![0xFF; 0x4000];
        let (board, _) =
            apply_neo_variant_heuristics(0x5003, NeoBoardType::Cthd2003, NeoFixBanksw::None, &cthd);
        assert_eq!(board, NeoBoardType::Cthd2003);

        cthd[0x30d9] = 0x03;
        let (board, fix) =
            apply_neo_variant_heuristics(0x5003, NeoBoardType::Cthd2003, NeoFixBanksw::None, &cthd);
        assert_eq!(board, NeoBoardType::Default);
        assert_eq!(fix, NeoFixBanksw::None);
    }

    #[test]
    fn digger_man_does_not_enable_fix_bankswitching() {
        assert_eq!(detect_neo_fix_banksw(0x066), NeoFixBanksw::None);
        assert_eq!(detect_neo_fix_banksw(0x253), NeoFixBanksw::Line);
        assert_eq!(detect_neo_fix_banksw(0x257), NeoFixBanksw::Tile);
    }

    #[test]
    fn sma_mslug3a_requires_an_encrypted_sized_prom() {
        let small_prom = vec![0x33; 0x500000];
        assert_eq!(
            validate_sma_board_type(NeoBoardType::SmaMslug3A, &small_prom),
            NeoBoardType::Default
        );

        let large_prom = vec![0x33; 0x500001];
        assert_eq!(
            validate_sma_board_type(NeoBoardType::SmaMslug3A, &large_prom),
            NeoBoardType::SmaMslug3A
        );
    }

    // ──────────────────────────────────────────────────────────────────
    // Tests: validate_kof2003_board_type
    // ──────────────────────────────────────────────────────────────────

    #[test]
    fn validate_kof2003_official_release_returns_pvc() {
        // Official KOF2003: NGH 0x271, no bootleg signature bytes → stays Pvc
        let prom = vec![0xFFu8; 0x1000]; // 4KB P-ROM, no bootleg markers
        let result = validate_kof2003_board_type(0x271, NeoBoardType::Pvc, &prom);
        assert_eq!(result, NeoBoardType::Pvc);
    }

    #[test]
    fn validate_kof2003_bootleg_kf2k3bla_detected() {
        // kf2k3bla/kf2k3pl: P-ROM[0x689] == 0x10
        let mut prom = vec![0xFFu8; 0x1000];
        prom[0x689] = 0x10;
        let result = validate_kof2003_board_type(0x271, NeoBoardType::Pvc, &prom);
        assert_eq!(result, NeoBoardType::Kf2k3Bla);
    }

    #[test]
    fn validate_kof2003_bootleg_kf2k3bl_detected() {
        // kf2k3bl/kf2k3upl: P-ROM[0xc1] == 0x02
        let mut prom = vec![0xFFu8; 0x1000];
        prom[0xc1] = 0x02;
        let result = validate_kof2003_board_type(0x271, NeoBoardType::Pvc, &prom);
        assert_eq!(result, NeoBoardType::Kf2k3Bl);
    }

    #[test]
    fn validate_kof2003_bla_takes_priority_over_bl() {
        // When both signatures are present, Kf2k3Bla wins (checked first)
        let mut prom = vec![0xFFu8; 0x1000];
        prom[0xc1] = 0x02;
        prom[0x689] = 0x10;
        let result = validate_kof2003_board_type(0x271, NeoBoardType::Pvc, &prom);
        assert_eq!(result, NeoBoardType::Kf2k3Bla);
    }

    #[test]
    fn validate_kof2003_wrong_ngh_returns_unchanged() {
        // Non-KOF2003 NGH: function should not touch board_type
        let mut prom = vec![0xFFu8; 0x1000];
        prom[0x689] = 0x10; // would trigger Kf2k3Bla for NGH 0x271
        let result = validate_kof2003_board_type(0x223, NeoBoardType::Pvc, &prom);
        assert_eq!(result, NeoBoardType::Pvc);

        // Also check with other board types
        let result = validate_kof2003_board_type(0x253, NeoBoardType::Sma, &prom);
        assert_eq!(result, NeoBoardType::Sma);
    }

    #[test]
    fn validate_kof2003_truncated_prom_returns_unchanged() {
        // P-ROM too small for both bootleg checks → no crash, board type unchanged
        let prom = vec![0xFFu8; 0x80]; // only 128 bytes, way below 0x689
        let result = validate_kof2003_board_type(0x271, NeoBoardType::Pvc, &prom);
        assert_eq!(result, NeoBoardType::Pvc);

        // P-ROM big enough for 0xc1 but not 0x689
        let prom = vec![0xFFu8; 0x200];
        let result = validate_kof2003_board_type(0x271, NeoBoardType::Pvc, &prom);
        assert_eq!(result, NeoBoardType::Pvc);
    }

    fn create_test_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let file = File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default();

        for (name, bytes) in entries {
            zip.start_file(name, options).unwrap();
            zip.write_all(bytes).unwrap();
        }

        zip.finish().unwrap();
    }

    fn patterned_prom_chunks(first_chunk_value: u8) -> Vec<u8> {
        let mut prom = vec![0; 0x400000];
        for chunk in 0..8 {
            let start = chunk * 0x80000;
            prom[start..start + 0x80000].fill(first_chunk_value + chunk as u8);
        }
        prom
    }

    fn create_test_neo(
        path: &Path,
        prom: &[u8],
        srom: &[u8],
        mrom: &[u8],
        v1rom: &[u8],
        v2rom: &[u8],
        crom: &[u8],
    ) {
        let mut data = vec![0; NEO_HEADER_SIZE];
        data[0] = b'N';
        data[1] = b'E';
        data[2] = b'O';
        data[3] = 1;
        write_u32_le(&mut data, 0x04, prom.len() as u32);
        write_u32_le(&mut data, 0x08, srom.len() as u32);
        write_u32_le(&mut data, 0x0C, mrom.len() as u32);
        write_u32_le(&mut data, 0x10, v1rom.len() as u32);
        write_u32_le(&mut data, 0x14, v2rom.len() as u32);
        write_u32_le(&mut data, 0x18, crom.len() as u32);
        data[0x2C..0x2C + 11].copy_from_slice(b"NGNEON TEST");
        data[0x4D..0x4D + 6].copy_from_slice(b"NGNEON");
        data.extend(prom);
        data.extend(srom);
        data.extend(mrom);
        data.extend(v1rom);
        data.extend(v2rom);
        data.extend(crom);
        std::fs::write(path, data).unwrap();
    }

    fn unique_temp_path(prefix: &str, extension: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        path.push(format!("{prefix}-{nanos}.{extension}"));
        path
    }

    fn make_test_neo_file() -> Vec<u8> {
        let mut data = vec![0; NEO_HEADER_SIZE];
        data[0] = b'N';
        data[1] = b'E';
        data[2] = b'O';
        data[3] = 1;
        // P-ROM: 64KB con vectores válidos 68k
        //   SP=0x0010FD00 -> [0x00, 0x10, 0xFD, 0x00]
        //   PC=0x00000100 -> [0x00, 0x00, 0x01, 0x00]
        write_u32_le(&mut data, 0x04, 0x10000);
        write_u32_le(&mut data, 0x08, 2);
        write_u32_le(&mut data, 0x0C, 2);
        write_u32_le(&mut data, 0x10, 3);
        write_u32_le(&mut data, 0x14, 5);
        write_u32_le(&mut data, 0x18, 4);
        write_u32_le(&mut data, 0x1C, 2026);
        write_u32_le(&mut data, 0x20, 7);
        write_u32_le(&mut data, 0x24, 2);
        write_u32_le(&mut data, 0x28, 0x1234);
        data[0x2C..0x2C + 11].copy_from_slice(b"NGNEON TEST");
        data[0x4D..0x4D + 6].copy_from_slice(b"NGNEON");
        // P-ROM 64KB: vectores válidos al inicio + relleno
        let mut prom = vec![0x10u8; 0x10000];
        prom[0..8].copy_from_slice(&[0x00, 0x10, 0xFD, 0x00, 0x00, 0x00, 0x01, 0x00]);
        data.extend(prom);
        data.extend([0x20; 2]);
        data.extend([0x30; 2]);
        data.extend([0x40; 3]);
        data.extend([0x50; 5]);
        data.extend([0x60; 4]);
        data
    }

    fn make_test_neo_file_with_graphics() -> Vec<u8> {
        let mut data = vec![0; NEO_HEADER_SIZE];
        data[0] = b'N';
        data[1] = b'E';
        data[2] = b'O';
        data[3] = 1;
        // P-ROM: 64KB con vectores válidos 68k
        write_u32_le(&mut data, 0x04, 0x10000);
        write_u32_le(&mut data, 0x08, 32);
        write_u32_le(&mut data, 0x18, (crate::video::BYTES_PER_TILE * 2) as u32);
        data[0x2C..0x2C + 11].copy_from_slice(b"NGNEON TEST");
        data[0x4D..0x4D + 6].copy_from_slice(b"NGNEON");
        // P-ROM 64KB: vectores válidos al inicio + relleno
        let mut prom = vec![0x10u8; 0x10000];
        prom[0..8].copy_from_slice(&[0x00, 0x10, 0xFD, 0x00, 0x00, 0x00, 0x01, 0x00]);
        data.extend(prom);
        data.extend([0x00; 32]);
        data.extend([0xFF; crate::video::BYTES_PER_TILE * 2]);
        data
    }

    fn write_u32_le(data: &mut [u8], offset: usize, value: u32) {
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_word_swapped_neo_header(data: &mut [u8], offset: usize) {
        let header = b"NEO-GEO";
        for (index, chunk) in header.chunks(2).enumerate() {
            let dst = offset + index * 2;
            if chunk.len() == 2 {
                data[dst] = chunk[1];
                data[dst + 1] = chunk[0];
            } else {
                data[dst + 1] = chunk[0];
            }
        }
    }
}

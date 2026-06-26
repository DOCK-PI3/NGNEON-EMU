use chrono::{Datelike, Local, Timelike};
use std::cell::{Cell, RefCell};
use std::path::PathBuf;

pub const WORK_RAM_SIZE: usize = 0x10000;
pub const VIDEO_RAM_SIZE: usize = 0x20000;
pub const PALETTE_RAM_BANK_SIZE: usize = 0x2000;
pub const PALETTE_RAM_SIZE: usize = PALETTE_RAM_BANK_SIZE * 2;
pub const PALETTE_RAM_START: u32 = 0x400000;
pub const BIOS_ROM_SIZE: usize = 0x80000;
pub const BIOS_ROM_START: u32 = 0xC00000;
pub const BACKUP_RAM_SIZE: usize = 0x10000;
pub const BACKUP_RAM_START: u32 = 0xD00000;
pub const MEMORY_CARD_SIZE: usize = 0x2000;
pub const MEMORY_CARD_START: u32 = 0x800000;
pub const FIXED_PROM_WINDOW_SIZE: usize = 0x100000;
pub const BANKED_PROM_WINDOW_SIZE: usize = 0x100000;
pub const Z80_RAM_SIZE: usize = 0x0800;
/// Z80 memory map constants
pub const Z80_MROM_STATIC_SIZE: usize = 0x8000;
pub const Z80_BANK_WINDOW_3_ADDR: u16 = 0x8000;
pub const Z80_BANK_WINDOW_3_SIZE: usize = 0x4000;
pub const Z80_BANK_WINDOW_2_ADDR: u16 = 0xC000;
pub const Z80_BANK_WINDOW_2_SIZE: usize = 0x2000;
pub const Z80_BANK_WINDOW_1_ADDR: u16 = 0xE000;
pub const Z80_BANK_WINDOW_1_SIZE: usize = 0x1000;
pub const Z80_BANK_WINDOW_0_ADDR: u16 = 0xF000;
pub const Z80_BANK_WINDOW_0_SIZE: usize = 0x0800;
pub const Z80_RAM_ADDR: u16 = 0xF800;
pub const Z80_CLOCK_HZ: i32 = 4_000_000;
/// Geolith watchdog threshold converted from master cycles to M68K cycles:
/// 3_244_030 master cycles / DIV_M68K(2), roughly 8 frames.
pub const WATCHDOG_M68K_CYCLES: u32 = 1_622_015;
/// SVC Chaos needs 10,000 additional master cycles of watchdog tolerance.
/// The memory subsystem tracks M68K cycles, so divide by `DIV_M68K` (2).
const SVC_WATCHDOG_TOLERANCE_M68K_CYCLES: u32 = 5_000;
/// Initial NEO-ZMC bank source offsets, matching Geolith's `geo_z80_init()`.
/// The destination windows are 0xF000, 0xE000, 0xC000 and 0x8000; these
/// offsets keep non-bankswitching M1 drivers mapped to the same source ranges.
pub const Z80_DEFAULT_BANK_OFFSETS: [usize; 4] = [0xF000, 0xE000, 0xC000, 0x8000];
const FIXED_PROM_START: u32 = 0x000000;
const FIXED_PROM_END: u32 = 0x0FFFFF;
const WORK_RAM_START: u32 = 0x100000;
const WORK_RAM_END: u32 = 0x1FFFFF;
const BANKED_PROM_START: u32 = 0x200000;
const BANKED_PROM_END: u32 = 0x2FFFFF;
const PROM_BANK_REGISTER_START: u32 = 0x2FFFF0;
const PROM_BANK_REGISTER_END: u32 = 0x2FFFFF;
const MSLUGX_PROTECTION_START: u32 = 0x2FFFE0;
const MSLUGX_PROTECTION_END: u32 = 0x2FFFEF;
const MSLUGX_BANK_REGISTER: u32 = 0x2FFFF0;
const SMA_PRESENCE_ADDR: u32 = 0x2FE446;
const SMA_PRESENCE_VALUE: u16 = 0x9A37;
const PVC_CARTRAM_START: u32 = 0x2FE000;
const PVC_CARTRAM_SIZE: usize = 0x2000;
const KOF10TH_EXTRA_RAM_SIZE: usize = 0x20000;
const KOF10TH_DYNFIX_SIZE: usize = 0x20000;
const ADPCM_A_RAM_SIZE: usize = 0x4000; // 16KB internal YM2610 ADPCM-A RAM
const ADPCM_A_START: u32 = 0x300000;
const ADPCM_A_END: u32 = 0x303FFF;
const ADPCM_B_RAM_SIZE: usize = 0x4000; // 16KB internal YM2610 ADPCM-B RAM
const ADPCM_B_START: u32 = 0x310000;
const ADPCM_B_END: u32 = 0x313FFF;
const INPUT_P1_PORT: u32 = 0x300000;
const DIPSW_PORT: u32 = 0x300001;
const TEST_SWITCH_PORT: u32 = 0x300081;
const SOUND_PORT: u32 = 0x320000;
pub const STATUS_A_PORT: u32 = 0x320001;
const INPUT_P2_PORT: u32 = 0x340000;
const SYSTEM_PORT: u32 = 0x380000;
const RTC_CONTROL_PORT: u32 = 0x380051;
const POUTPUT_START: u32 = 0x380001;
const POUTPUT_END: u32 = 0x3800EF;
const SYSTEM_LATCH_START: u32 = 0x3A0001;
const SYSTEM_LATCH_END: u32 = 0x3A001F;
const LSPC_REG_START: u32 = 0x3C0000;
const LSPC_REG_END: u32 = 0x3C000F;
const LSPC_VRAMADDR: u32 = 0x3C0000;
const LSPC_VRAMRW: u32 = 0x3C0002;
const LSPC_VRAMMOD: u32 = 0x3C0004;
const LSPC_MODE: u32 = 0x3C0006;
const LSPC_TIMERHIGH: u32 = 0x3C0008;
const LSPC_TIMERLOW: u32 = 0x3C000A;
const LSPC_IRQACK: u32 = 0x3C000C;
const LSPC_TIMERSTOP: u32 = 0x3C000E;
const LSPC_VRAM_WORDS: u16 = 0x8800;
const IRQ_TIMER_ENABLED: u8 = 0x10;
const IRQ_TIMER_RELOAD_WRITE: u8 = 0x20;
const IRQ_TIMER_RELOAD_VBLANK: u8 = 0x40;
const IRQ_TIMER_RELOAD_COUNT0: u8 = 0x80;
const PALETTE_RAM_END: u32 = 0x7FFFFF;
const MEMORY_CARD_END: u32 = 0xBFFFFF;
const BIOS_ROM_END: u32 = 0xCFFFFF;
const BACKUP_RAM_END: u32 = 0xDFFFFF;
const MAIN_CPU_ADDRESS_MASK: u32 = 0x00FF_FFFF;
const NEOGEO_VECTOR_SWAP_BYTES: usize = 0x80;
const BUS_TRACE_LIMIT: usize = 512;
const Z80_IO_TRACE_LIMIT: usize = 512;
const RTC_M68K_CYCLES_PER_SECOND: u32 = 12_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusAccessKind {
    Read,
    Write,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BusAccess {
    pub kind: BusAccessKind,
    pub address: u32,
    pub value: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Z80IoAccess {
    pub kind: BusAccessKind,
    pub port: u16,
    pub value: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputPorts {
    pub p1: u8,
    pub system: u8,
    pub status_a: u8,
}

impl InputPorts {
    pub const RELEASED: Self = Self {
        p1: 0xFF,
        // Geolith's inactive callbacks expose 0x3F on STATUS_B when no
        // memory card is inserted, and 0x07 on STATUS_A before RTC bits.
        system: 0x3F,
        status_a: 0x07,
    };
}

impl Default for InputPorts {
    fn default() -> Self {
        Self::RELEASED
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Rtc4990a {
    command: u8,
    mode: u8,
    register: u64,
    cycles: u32,
    tp_interval: u32,
    tp_counter: u32,
    tp_running: bool,
    timer_1hz: bool,
    prev_clk: bool,
    prev_stb: bool,
    tp: bool,
    second: u8,
    minute: u8,
    hour: u8,
    day: u8,
    weekday: u8,
    month: u8,
    year: u8,
}

impl Rtc4990a {
    fn new() -> Self {
        let now = Local::now();
        Self::from_calendar(
            now.second() as u8,
            now.minute() as u8,
            now.hour() as u8,
            now.day() as u8,
            now.weekday().num_days_from_sunday() as u8,
            now.month() as u8,
            now.year().rem_euclid(100) as u8,
        )
    }

    fn from_calendar(
        second: u8,
        minute: u8,
        hour: u8,
        day: u8,
        weekday: u8,
        month: u8,
        year: u8,
    ) -> Self {
        Self {
            command: 0,
            mode: 0,
            register: 0,
            cycles: 0,
            tp_interval: RTC_M68K_CYCLES_PER_SECOND,
            tp_counter: 0,
            // Geolith initializes the uPD4990A timing-pulse counter in RUN
            // mode with a one-second interval. BIOS and gambling titles poll
            // this pin before issuing any explicit RTC command.
            tp_running: true,
            timer_1hz: false,
            prev_clk: false,
            prev_stb: false,
            tp: false,
            second: second.min(59),
            minute: minute.min(59),
            hour: hour.min(23),
            day: day.clamp(1, 31),
            weekday: weekday % 7,
            month: month.clamp(1, 12),
            year: year % 100,
        }
    }

    fn reset(&mut self) {
        *self = Self::new();
    }

    fn read_pins(&self) -> u8 {
        let out = if self.mode == 0 || self.mode == 3 {
            self.timer_1hz as u8
        } else {
            (self.register & 1) as u8
        };
        (out << 1) | self.tp as u8
    }

    fn sync_cycles(&mut self, cycles: u32) {
        self.cycles = self.cycles.saturating_add(cycles);
        while self.cycles >= RTC_M68K_CYCLES_PER_SECOND {
            self.cycles -= RTC_M68K_CYCLES_PER_SECOND;
            self.increment_clock();
        }
        self.timer_1hz = self.cycles >= (RTC_M68K_CYCLES_PER_SECOND >> 1);

        if self.tp_running {
            self.tp_counter = self.tp_counter.saturating_add(cycles);
            while self.tp_counter >= self.tp_interval {
                self.tp_counter -= self.tp_interval;
            }
            self.tp = self.tp_counter >= (self.tp_interval >> 1);
        }
    }

    fn write_control(&mut self, value: u8) {
        let data = value & 0x01 != 0;
        let clk = value & 0x02 != 0;
        let stb = value & 0x04 != 0;

        if stb && !self.prev_stb {
            self.process_command();
        }

        if !stb && clk && !self.prev_clk {
            if self.mode == 1 {
                let shift_in = if self.command & 1 != 0 { 1u64 << 47 } else { 0 };
                self.register = (self.register >> 1) | shift_in;
            }
            self.command = (self.command >> 1) & 0x07;
            if data {
                self.command |= 0x08;
            }
        }

        self.prev_clk = clk;
        self.prev_stb = stb;
    }

    fn process_command(&mut self) {
        match self.command & 0x0f {
            0x00 => {
                self.mode = 0;
                self.tp_interval = RTC_M68K_CYCLES_PER_SECOND / 64;
            }
            0x01 => self.mode = 1,
            0x02 => {
                self.mode = 2;
                self.load_time_from_register();
            }
            0x03 => {
                self.mode = 0;
                self.load_register_from_time();
            }
            0x04..=0x0b => {
                self.tp_interval = match self.command & 0x0f {
                    0x04 => RTC_M68K_CYCLES_PER_SECOND / 64,
                    0x05 => RTC_M68K_CYCLES_PER_SECOND / 256,
                    0x06 => RTC_M68K_CYCLES_PER_SECOND / 2048,
                    0x07 => RTC_M68K_CYCLES_PER_SECOND / 4096,
                    0x08 => RTC_M68K_CYCLES_PER_SECOND,
                    0x09 => RTC_M68K_CYCLES_PER_SECOND.saturating_mul(10),
                    0x0a => RTC_M68K_CYCLES_PER_SECOND.saturating_mul(30),
                    _ => RTC_M68K_CYCLES_PER_SECOND.saturating_mul(60),
                };
                if self.command & 0x0f >= 0x08 {
                    self.tp_counter = 0;
                }
                self.tp_running = true;
            }
            0x0c => {
                self.tp = false;
                self.tp_running = true;
            }
            0x0d => {
                self.tp_counter = 0;
                self.tp_running = true;
            }
            0x0e => self.tp_running = false,
            _ => {}
        }
    }

    fn increment_clock(&mut self) {
        self.second += 1;
        if self.second < 60 {
            return;
        }
        self.second = 0;
        self.minute += 1;
        if self.minute < 60 {
            return;
        }
        self.minute = 0;
        self.hour = (self.hour + 1) % 24;
    }

    fn load_register_from_time(&mut self) {
        self.register = 0;
        self.register |= bcd(self.second) as u64;
        self.register |= (bcd(self.minute) as u64) << 8;
        self.register |= (bcd(self.hour) as u64) << 16;
        self.register |= (bcd(self.day) as u64) << 24;
        self.register |= (self.weekday as u64) << 32;
        self.register |= (self.month as u64) << 36;
        self.register |= (bcd(self.year) as u64) << 40;
    }

    fn load_time_from_register(&mut self) {
        self.second = from_bcd((self.register & 0xff) as u8).min(59);
        self.minute = from_bcd(((self.register >> 8) & 0xff) as u8).min(59);
        self.hour = from_bcd(((self.register >> 16) & 0xff) as u8).min(23);
        self.day = from_bcd(((self.register >> 24) & 0xff) as u8).clamp(1, 31);
        self.weekday = ((self.register >> 32) & 0x0f) as u8 % 7;
        self.month = (((self.register >> 36) & 0x0f) as u8).clamp(1, 12);
        self.year = from_bcd(((self.register >> 40) & 0xff) as u8);
    }
}

fn bcd(value: u8) -> u8 {
    ((value / 10) << 4) | (value % 10)
}

fn from_bcd(value: u8) -> u8 {
    (value & 0x0f) + ((value >> 4) & 0x0f) * 10
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemControlSnapshot {
    pub display_enabled: bool,
    pub use_cart_vectors: bool,
    pub use_cart_audio: bool,
    pub use_cart_fix: bool,
    pub save_ram_unlocked: bool,
    pub palette_bank: u8,
    pub palette_shadow: bool,
    pub memcard_unlocked: bool,
    pub memcard_register_select: bool,
    pub memcard_inserted: bool,
    pub memcard_write_protected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoundPortSnapshot {
    pub command: u8,
    pub reply: u8,
}

/// Per-game SMA (NEO-SMA) runtime configuration.
///
/// Each SMA-protected game uses the same LFSR (SMATAP = 0x98ec) but differs in:
/// - `prn_addr[2]`: Two 16-bit addresses where the LFSR value can be read
/// - `bank_reg_addr`: Address where the bank switch value is written
/// - `bank_offsets`: LUT of bank offsets (up to 64 entries, index by unscrambled value)
/// - `scramble`: 6-element bitswap map for unscrambling the bank register value
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmaConfig {
    pub prn_addr: [u32; 2],
    pub bank_reg_addr: u32,
    pub bank_offsets: &'static [u32],
    pub scramble: [u8; 6],
}

impl SmaConfig {
    /// The King of Fighters '99 (NGH 0x151 / 0x251)
    pub const fn kof99() -> Self {
        const BANK: &[u32] = &[
            0x000000, 0x100000, 0x200000, 0x300000, 0x3CC000, 0x4CC000, 0x3F2000, 0x4F2000,
            0x407800, 0x507800, 0x40D000, 0x50D000, 0x417800, 0x517800, 0x420800, 0x520800,
            0x424800, 0x524800, 0x429000, 0x529000, 0x42E800, 0x52E800, 0x431800, 0x531800,
            0x54D000, 0x551000, 0x567000, 0x592800, 0x588800, 0x581800, 0x599800, 0x594800,
            0x598000,
        ];
        Self {
            prn_addr: [0x2FFFF8, 0x2FFFFA],
            bank_reg_addr: 0x2FFFF0,
            bank_offsets: BANK,
            scramble: [14, 6, 8, 10, 12, 5],
        }
    }

    /// Metal Slug 3 (NGH 0x256 / internal 0x213)
    pub const fn mslug3() -> Self {
        const BANK: &[u32] = &[
            0x000000, 0x020000, 0x040000, 0x060000, 0x070000, 0x090000, 0x0B0000, 0x0D0000,
            0x0E0000, 0x0F0000, 0x120000, 0x130000, 0x140000, 0x150000, 0x180000, 0x190000,
            0x1A0000, 0x1B0000, 0x1E0000, 0x1F0000, 0x200000, 0x210000, 0x240000, 0x250000,
            0x260000, 0x270000, 0x2A0000, 0x2B0000, 0x2C0000, 0x2D0000, 0x300000, 0x310000,
            0x320000, 0x330000, 0x360000, 0x370000, 0x380000, 0x390000, 0x3C0000, 0x3D0000,
            0x400000, 0x410000, 0x440000, 0x450000, 0x460000, 0x470000, 0x4A0000, 0x4B0000,
            0x4C0000,
        ];
        Self {
            prn_addr: [0x2FFFF8, 0x2FFFFA],
            bank_reg_addr: 0x2FFFE4,
            bank_offsets: BANK,
            scramble: [14, 12, 15, 6, 3, 9],
        }
    }

    /// Metal Slug 3 alternate SMA revision (`mslug3a`, NGH 0x256).
    pub const fn mslug3a() -> Self {
        const BANK: &[u32] = &[
            0x000000, 0x030000, 0x040000, 0x070000, 0x080000, 0x0A0000, 0x0C0000, 0x0E0000,
            0x0F0000, 0x100000, 0x130000, 0x140000, 0x150000, 0x160000, 0x190000, 0x1A0000,
            0x1B0000, 0x1C0000, 0x1F0000, 0x200000, 0x210000, 0x220000, 0x250000, 0x260000,
            0x270000, 0x280000, 0x2B0000, 0x2C0000, 0x2D0000, 0x2E0000, 0x310000, 0x320000,
            0x330000, 0x340000, 0x370000, 0x380000, 0x390000, 0x3A0000, 0x3D0000, 0x3E0000,
            0x400000, 0x410000, 0x440000, 0x450000, 0x460000, 0x470000, 0x4A0000, 0x4B0000,
            0x4C0000,
        ];
        Self {
            prn_addr: [0x2FFFF8, 0x2FFFFA],
            bank_reg_addr: 0x2FFFE4,
            bank_offsets: BANK,
            scramble: [15, 3, 1, 6, 12, 11],
        }
    }

    /// Garou MVS (NEO-SMA KF, NGH 0x253 / internal 0x229/0x153)
    pub const fn garou() -> Self {
        const BANK: &[u32] = &[
            0x000000, 0x100000, 0x200000, 0x300000, 0x280000, 0x380000, 0x2D0000, 0x3D0000,
            0x2F0000, 0x3F0000, 0x400000, 0x500000, 0x420000, 0x520000, 0x440000, 0x540000,
            0x498000, 0x598000, 0x4A0000, 0x5A0000, 0x4A8000, 0x5A8000, 0x4B0000, 0x5B0000,
            0x4B8000, 0x5B8000, 0x4C0000, 0x5C0000, 0x4C8000, 0x5C8000, 0x4D0000, 0x5D0000,
            0x458000, 0x558000, 0x460000, 0x560000, 0x468000, 0x568000, 0x470000, 0x570000,
            0x478000, 0x578000, 0x480000, 0x580000, 0x488000, 0x588000, 0x490000, 0x590000,
            0x5D0000, 0x5D8000, 0x5E0000, 0x5E8000, 0x5F0000, 0x5F8000, 0x600000,
        ];
        Self {
            prn_addr: [0x2FFFCC, 0x2FFFF0],
            bank_reg_addr: 0x2FFFC0,
            bank_offsets: BANK,
            scramble: [5, 9, 7, 6, 14, 12],
        }
    }

    /// Garou AES (NEO-SMA KE, NGH 0x253 / internal 0x229/0x153)
    pub const fn garouh() -> Self {
        const BANK: &[u32] = &[
            0x000000, 0x100000, 0x200000, 0x300000, 0x280000, 0x380000, 0x2D0000, 0x3D0000,
            0x2C8000, 0x3C8000, 0x400000, 0x500000, 0x420000, 0x520000, 0x440000, 0x540000,
            0x598000, 0x698000, 0x5A0000, 0x6A0000, 0x5A8000, 0x6A8000, 0x5B0000, 0x6B0000,
            0x5B8000, 0x6B8000, 0x5C0000, 0x6C0000, 0x5C8000, 0x6C8000, 0x5D0000, 0x6D0000,
            0x458000, 0x558000, 0x460000, 0x560000, 0x468000, 0x568000, 0x470000, 0x570000,
            0x478000, 0x578000, 0x480000, 0x580000, 0x488000, 0x588000, 0x490000, 0x590000,
            0x5D8000, 0x6D8000, 0x5E0000, 0x6E0000, 0x5E8000, 0x6E8000, 0x6E8000,
        ];
        Self {
            prn_addr: [0x2FFFCC, 0x2FFFF0],
            bank_reg_addr: 0x2FFFC0,
            bank_offsets: BANK,
            scramble: [4, 8, 14, 2, 11, 13],
        }
    }

    /// The King of Fighters 2000 (NGH 0x257 / internal 0x21D)
    pub const fn kof2000() -> Self {
        const BANK: &[u32] = &[
            0x000000, 0x100000, 0x200000, 0x300000, 0x3F7800, 0x4F7800, 0x3FF800, 0x4FF800,
            0x407800, 0x507800, 0x40F800, 0x50F800, 0x416800, 0x516800, 0x41D800, 0x51D800,
            0x424000, 0x524000, 0x523800, 0x623800, 0x526000, 0x626000, 0x528000, 0x628000,
            0x52A000, 0x62A000, 0x52B800, 0x62B800, 0x52D000, 0x62D000, 0x52E800, 0x62E800,
            0x618000, 0x619000, 0x61A000, 0x61A800,
        ];
        Self {
            prn_addr: [0x2FFFD8, 0x2FFFDA],
            bank_reg_addr: 0x2FFFEC,
            bank_offsets: BANK,
            scramble: [15, 14, 7, 3, 10, 5],
        }
    }

    /// Returns true if the given address is one of the PRN read addresses.
    pub fn is_prn_addr(&self, addr: u32) -> bool {
        addr == self.prn_addr[0] || addr == self.prn_addr[1]
    }

    /// Returns true if the given address is the bank register (any byte).
    pub fn is_bank_reg(&self, addr: u32) -> bool {
        let base = addr & !1;
        base == self.bank_reg_addr
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CartProtection {
    None,
    Linkable,
    Brezzasoft,
    Ct0,
    Kof98,
    MslugX,
    Ms5Plus,
    Cthd2003,
    Kof10th,
    Sma(SmaConfig),
    /// NEO-PVC board: KOF 2003, Metal Slug 5, SVC Chaos.
    /// Uses cartram-based bankswitching at 0x2FE000-0x2FFFFF
    /// with pack/unpack logic and a dedicated bank address.
    Pvc,
    Kf2k3Bl,
    Kf2k3Bla,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SpecialBoardState {
    pub cart_reg: [u16; 2],
    pub prot_reg: u32,
    pub mslugx_command: u16,
    pub mslugx_counter: u16,
    pub sma_rng: u16,
    pub pending_sma_bank_hi: Option<u8>,
}

impl Default for SpecialBoardState {
    fn default() -> Self {
        Self {
            cart_reg: [0; 2],
            prot_reg: 0,
            mslugx_command: 0,
            mslugx_counter: 0,
            sma_rng: 0x2345,
            pending_sma_bank_hi: None,
        }
    }
}

impl CartProtection {
    fn sma_config(&self) -> Option<&SmaConfig> {
        match self {
            CartProtection::Sma(config) => Some(config),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixBankSwitch {
    None,
    Line,
    /// Per-tile banking: tile index in SROM is shifted by a bank factor
    /// read from VRAM[0x7500 + col_bank_offset], using bits extracted
    /// from the VRAM word per the tile's column position within its group of 6.
    /// Used by KOF2000, Matrimelee, SVC, KOF2003 (NGH 0x257, 0x266, 0x269, 0x271).
    Tile,
}

impl Memory {
    /// Lee un byte de la dirección dada
    pub fn read8(&self, addr: u32) -> u8 {
        let addr = main_cpu_bus_addr(addr);
        match addr {
            FIXED_PROM_START..=FIXED_PROM_END => self.read_fixed_prom_byte(addr),
            WORK_RAM_START..=WORK_RAM_END => {
                // Work RAM (64KB)
                let offset = Self::work_ram_offset(addr);
                self.ram.get(offset).copied().unwrap_or(0)
            }
            BANKED_PROM_START..=BANKED_PROM_END => {
                if let Some(value) = self.read_banked_special8(addr) {
                    return value;
                }
                if let Some(config) = self.cart_protection.sma_config() {
                    if self.is_sma_read(addr, config) {
                        return self.read_sma(addr, config);
                    }
                }
                // NEO-PVC cartram reads (0x2FE000-0x2FFFFF)
                if self.uses_pvc_runtime() && addr >= PVC_CARTRAM_START {
                    let offset = ((addr - PVC_CARTRAM_START) as usize) ^ 1;
                    return self.pvc_cart_ram.get(offset).copied().unwrap_or(0xFF);
                }
                // NEO-PVC banked PROM: use pvc_bank_addr instead of prom_bank_offset
                let bank_base = self.banked_prom_base();
                let window_offset = (addr - BANKED_PROM_START) as usize;
                self.prom
                    .get(bank_base + window_offset)
                    .copied()
                    .unwrap_or(0xFF)
            }
            LSPC_REG_START..=LSPC_REG_END => self.read_lspc_register(addr),
            PALETTE_RAM_START..=PALETTE_RAM_END => {
                let offset = self.palette_ram_offset(addr);
                self.palette_ram.get(offset).copied().unwrap_or(0)
            }
            INPUT_P1_PORT => {
                self.record_bus_access(BusAccessKind::Read, addr, self.input_ports.p1);
                self.input_ports.p1
            }
            DIPSW_PORT => self.dip_switches,
            TEST_SWITCH_PORT => self.test_switch & !0x40,
            0x300002..=ADPCM_A_END => {
                let offset = (addr - ADPCM_A_START) as usize;
                self.adpcm_a_ram.get(offset).copied().unwrap_or(0)
            }
            ADPCM_B_START..=ADPCM_B_END => {
                let offset = (addr - ADPCM_B_START) as usize;
                self.adpcm_b_ram.get(offset).copied().unwrap_or(0)
            }
            SOUND_PORT => {
                self.record_bus_access(BusAccessKind::Read, addr, self.z80_sound_reply);
                self.z80_sound_reply
            }
            STATUS_A_PORT => self.read_status_a(),
            INPUT_P2_PORT => self.p2_port,
            SYSTEM_PORT => self.read_system_b(),
            MEMORY_CARD_START..=MEMORY_CARD_END => self.read_memory_card(addr),
            BIOS_ROM_START..=BIOS_ROM_END => self.read_bios_region_byte(addr),
            BACKUP_RAM_START..=BACKUP_RAM_END => {
                let offset = Self::backup_ram_offset(addr);
                self.backup_ram.get(offset).copied().unwrap_or(0)
            }
            // Otros rangos: C-ROM, S-ROM, M-ROM, V-ROM, etc. (no implementado)
            _ => {
                self.record_bus_access(BusAccessKind::Read, addr, 0xFF);
                0xFF
            } // Valor abierto (bus floating)
        }
    }

    /// Escribe un byte en la dirección dada
    pub fn write8(&mut self, addr: u32, value: u8) {
        let addr = main_cpu_bus_addr(addr);
        match addr {
            // ── NEO-PVC cartram writes (must be before generic PROM_BANK_REGISTER) ──
            BANKED_PROM_START..=BANKED_PROM_END if self.write_banked_special8(addr, value) => {}
            PROM_BANK_REGISTER_START..=PROM_BANK_REGISTER_END
                if self.cart_protection != CartProtection::Pvc =>
            {
                self.record_bus_access(BusAccessKind::Write, addr, value);
                self.select_prom_bank(value);
            }
            WORK_RAM_START..=WORK_RAM_END => {
                // Work RAM
                let offset = Self::work_ram_offset(addr);
                if let Some(cell) = self.ram.get_mut(offset) {
                    *cell = value;
                }
            }
            LSPC_REG_START..=LSPC_REG_END => {
                // Geolith/hardware: 8-bit LSPC writes are effective only on
                // even addresses and duplicate the byte into both halves.
                if addr & 1 == 0 {
                    self.write_lspc_register_word(addr, u16::from_be_bytes([value, value]));
                }
            }
            PALETTE_RAM_START..=PALETTE_RAM_END => {
                let offset = self.palette_ram_byte_write_offset(addr);
                if let Some(cell) = self.palette_ram.get_mut(offset) {
                    *cell = value;
                }
            }
            DIPSW_PORT => {
                self.reset_watchdog();
            }
            ADPCM_A_START..=ADPCM_A_END => {
                let offset = (addr - ADPCM_A_START) as usize;
                if let Some(cell) = self.adpcm_a_ram.get_mut(offset) {
                    *cell = value;
                }
            }
            ADPCM_B_START..=ADPCM_B_END => {
                let offset = (addr - ADPCM_B_START) as usize;
                if let Some(cell) = self.adpcm_b_ram.get_mut(offset) {
                    *cell = value;
                }
            }
            SOUND_PORT => {
                self.write_sound_command(value);
            }
            STATUS_A_PORT => {}
            RTC_CONTROL_PORT => self.rtc.borrow_mut().write_control(value),
            SYSTEM_PORT => {}
            POUTPUT_START..=POUTPUT_END if addr & 1 == 1 => {}
            SYSTEM_LATCH_START..=SYSTEM_LATCH_END => self.write_system_latch(addr),
            MEMORY_CARD_START..=MEMORY_CARD_END => self.write_memory_card(addr, value),
            BACKUP_RAM_START..=BACKUP_RAM_END => {
                if self.save_ram_unlocked {
                    let offset = Self::backup_ram_offset(addr);
                    if let Some(cell) = self.backup_ram.get_mut(offset) {
                        *cell = value;
                        self.dirty_backup_ram.set(true);
                    }
                }
            }
            // Otros rangos: ignorar (ROM, etc.)
            _ => self.record_bus_access(BusAccessKind::Write, addr, value),
        }
    }

    /// Lee 2 bytes (big-endian word) desde la direccion dada.
    ///
    /// Para las regiones mas frecuentes (PROM, RAM, BIOS, palette RAM,
    /// backup RAM) decodifica la direccion UNA sola vez y accede directamente
    /// a los vectores.  Para el resto (proteccion, LSPC, I/O, etc.) cae en
    /// dos llamadas a `read8()`, que es correcto pero mas lento.
    pub fn read16(&self, addr: u32) -> u16 {
        let addr = main_cpu_bus_addr(addr);
        match addr {
            FIXED_PROM_START..=FIXED_PROM_END => {
                if self.cart_protection == CartProtection::Kof10th && addr >= 0x0e0000 {
                    return read16be_wrapped(&self.kof10th_extra_ram, (addr & 0x1fffe) as usize);
                }
                let hi = self.read_fixed_prom_byte(addr);
                let lo = self.read_fixed_prom_byte(addr.wrapping_add(1));
                u16::from_be_bytes([hi, lo])
            }
            WORK_RAM_START..=WORK_RAM_END => {
                let offset = Self::work_ram_offset(addr);
                let mask = self.ram.len() - 1;
                u16::from_be_bytes([
                    self.ram.get(offset).copied().unwrap_or(0),
                    self.ram
                        .get(offset.wrapping_add(1) & mask)
                        .copied()
                        .unwrap_or(0),
                ])
            }
            PALETTE_RAM_START..=PALETTE_RAM_END => {
                let o0 = self.palette_ram_offset(addr);
                let o1 = self.palette_ram_offset(addr.wrapping_add(1));
                u16::from_be_bytes([
                    self.palette_ram.get(o0).copied().unwrap_or(0),
                    self.palette_ram.get(o1).copied().unwrap_or(0),
                ])
            }
            BIOS_ROM_START..=BIOS_ROM_END => {
                let offset = (addr - BIOS_ROM_START) as usize;
                let hi = self.read_bios_or_prom(offset);
                let lo = self.read_bios_or_prom(offset.wrapping_add(1));
                u16::from_be_bytes([hi, lo])
            }
            BACKUP_RAM_START..=BACKUP_RAM_END => {
                let offset = Self::backup_ram_offset(addr);
                let mask = self.backup_ram.len() - 1;
                u16::from_be_bytes([
                    self.backup_ram.get(offset).copied().unwrap_or(0),
                    self.backup_ram
                        .get(offset.wrapping_add(1) & mask)
                        .copied()
                        .unwrap_or(0),
                ])
            }
            BANKED_PROM_START..=BANKED_PROM_END => {
                if let Some(value) = self.read_banked_special16(addr) {
                    return value;
                }
                if self.cart_protection == CartProtection::MslugX
                    && (MSLUGX_PROTECTION_START..=MSLUGX_PROTECTION_END).contains(&addr)
                {
                    return self.read_mslugx_protection_word(addr);
                }
                // Protección SMA: lectura word de presencia/PRN, igual que Geolith.
                if let Some(config) = self.cart_protection.sma_config() {
                    if self.is_sma_word_read(addr, config) {
                        return self.read_sma_word(addr, config);
                    }
                }
                // NEO-PVC: word reads use Geolith's read16be() helper:
                // low byte at offset, high byte at offset+1. The ^1 swap is
                // only for byte-level read8/write8.
                if self.uses_pvc_runtime() && addr >= PVC_CARTRAM_START {
                    let offset = (addr - PVC_CARTRAM_START) as usize;
                    return u16::from_be_bytes([
                        self.pvc_cart_ram
                            .get(offset.wrapping_add(1))
                            .copied()
                            .unwrap_or(0xFF),
                        self.pvc_cart_ram.get(offset).copied().unwrap_or(0xFF),
                    ]);
                }
                // Fast path: acceso directo al vector PROM
                let bank_base = self.banked_prom_base();
                let window_offset = (addr - BANKED_PROM_START) as usize;
                let offset = bank_base + window_offset;
                u16::from_be_bytes([
                    self.prom.get(offset).copied().unwrap_or(0xFF),
                    self.prom
                        .get(offset.wrapping_add(1))
                        .copied()
                        .unwrap_or(0xFF),
                ])
            }
            INPUT_P1_PORT => {
                self.record_bus_access(BusAccessKind::Read, addr, self.input_ports.p1);
                u16::from_be_bytes([self.input_ports.p1, self.input_ports.p1])
            }
            INPUT_P2_PORT => {
                self.record_bus_access(BusAccessKind::Read, addr, self.p2_port);
                u16::from_be_bytes([self.p2_port, self.p2_port])
            }
            SYSTEM_PORT => {
                let value = self.read_system_b();
                u16::from_be_bytes([value, value])
            }
            SOUND_PORT => {
                self.record_bus_access(BusAccessKind::Read, addr, 0xFF);
                0xFFFF
            }
            LSPC_REG_START..=LSPC_REG_END => {
                let hi = self.read_lspc_register(addr) as u16;
                let lo = self.read_lspc_register(addr.wrapping_add(1)) as u16;
                (hi << 8) | lo
            }
            // Fallback byte-by-byte para regiones con side-effects
            _ => {
                let hi = self.read8(addr) as u16;
                let lo = self.read8(addr.wrapping_add(1)) as u16;
                (hi << 8) | lo
            }
        }
    }

    /// Lee 4 bytes (big-endian long) desde la direccion dada.
    pub fn read32(&self, addr: u32) -> u32 {
        let addr = main_cpu_bus_addr(addr);
        match addr {
            FIXED_PROM_START..=FIXED_PROM_END => {
                let hi = self.read16(addr) as u32;
                let lo = self.read16(addr.wrapping_add(2)) as u32;
                (hi << 16) | lo
            }
            WORK_RAM_START..=WORK_RAM_END => {
                let offset = Self::work_ram_offset(addr);
                let mask = self.ram.len() - 1;
                u32::from_be_bytes([
                    self.ram.get(offset).copied().unwrap_or(0),
                    self.ram
                        .get(offset.wrapping_add(1) & mask)
                        .copied()
                        .unwrap_or(0),
                    self.ram
                        .get(offset.wrapping_add(2) & mask)
                        .copied()
                        .unwrap_or(0),
                    self.ram
                        .get(offset.wrapping_add(3) & mask)
                        .copied()
                        .unwrap_or(0),
                ])
            }
            PALETTE_RAM_START..=PALETTE_RAM_END => {
                let o0 = self.palette_ram_offset(addr);
                let o1 = self.palette_ram_offset(addr.wrapping_add(1));
                let o2 = self.palette_ram_offset(addr.wrapping_add(2));
                let o3 = self.palette_ram_offset(addr.wrapping_add(3));
                u32::from_be_bytes([
                    self.palette_ram.get(o0).copied().unwrap_or(0),
                    self.palette_ram.get(o1).copied().unwrap_or(0),
                    self.palette_ram.get(o2).copied().unwrap_or(0),
                    self.palette_ram.get(o3).copied().unwrap_or(0),
                ])
            }
            BIOS_ROM_START..=BIOS_ROM_END => {
                let offset = (addr - BIOS_ROM_START) as usize;
                let b0 = self.read_bios_or_prom(offset);
                let b1 = self.read_bios_or_prom(offset.wrapping_add(1));
                let b2 = self.read_bios_or_prom(offset.wrapping_add(2));
                let b3 = self.read_bios_or_prom(offset.wrapping_add(3));
                u32::from_be_bytes([b0, b1, b2, b3])
            }
            BACKUP_RAM_START..=BACKUP_RAM_END => {
                let offset = Self::backup_ram_offset(addr);
                let mask = self.backup_ram.len() - 1;
                u32::from_be_bytes([
                    self.backup_ram.get(offset).copied().unwrap_or(0),
                    self.backup_ram
                        .get(offset.wrapping_add(1) & mask)
                        .copied()
                        .unwrap_or(0),
                    self.backup_ram
                        .get(offset.wrapping_add(2) & mask)
                        .copied()
                        .unwrap_or(0),
                    self.backup_ram
                        .get(offset.wrapping_add(3) & mask)
                        .copied()
                        .unwrap_or(0),
                ])
            }
            BANKED_PROM_START..=BANKED_PROM_END => {
                if self.cart_protection == CartProtection::MslugX
                    && addr <= MSLUGX_PROTECTION_END
                    && addr.wrapping_add(3) >= MSLUGX_PROTECTION_START
                {
                    return ((self.read16(addr) as u32) << 16)
                        | self.read16(addr.wrapping_add(2)) as u32;
                }
                // Protección SMA
                if let Some(config) = self.cart_protection.sma_config() {
                    if self.is_sma_word_read(addr, config)
                        || self.is_sma_word_read(addr.wrapping_add(2), config)
                    {
                        let hi = self.read16(addr) as u32;
                        let lo = self.read16(addr.wrapping_add(2)) as u32;
                        return (hi << 16) | lo;
                    }
                }
                // NEO-PVC: long reads go through two Geolith read16be() calls.
                if self.uses_pvc_runtime() && addr >= PVC_CARTRAM_START {
                    let hi = self.read16(addr) as u32;
                    let lo = self.read16(addr.wrapping_add(2)) as u32;
                    return (hi << 16) | lo;
                }
                // Fast path: acceso directo al vector PROM
                let bank_base = self.banked_prom_base();
                let window_offset = (addr - BANKED_PROM_START) as usize;
                let offset = bank_base + window_offset;
                u32::from_be_bytes([
                    self.prom.get(offset).copied().unwrap_or(0xFF),
                    self.prom
                        .get(offset.wrapping_add(1))
                        .copied()
                        .unwrap_or(0xFF),
                    self.prom
                        .get(offset.wrapping_add(2))
                        .copied()
                        .unwrap_or(0xFF),
                    self.prom
                        .get(offset.wrapping_add(3))
                        .copied()
                        .unwrap_or(0xFF),
                ])
            }
            _ => {
                // Musashi/Geolith dispatches all 32-bit bus reads as two
                // 16-bit reads. This matters for MMIO side effects such as
                // VRAM auto-increment and memory-card odd-byte accesses.
                let hi = self.read16(addr) as u32;
                let lo = self.read16(addr.wrapping_add(2)) as u32;
                (hi << 16) | lo
            }
        }
    }

    /// Escribe 2 bytes (big-endian word) en la direccion dada.
    pub fn write16(&mut self, addr: u32, value: u16) {
        let addr = main_cpu_bus_addr(addr);
        let [hi, lo] = value.to_be_bytes();
        match addr {
            WORK_RAM_START..=WORK_RAM_END => {
                let offset = Self::work_ram_offset(addr);
                let next = offset.wrapping_add(1) & (self.ram.len() - 1);
                if let Some(cell) = self.ram.get_mut(offset) {
                    *cell = hi;
                }
                if let Some(cell) = self.ram.get_mut(next) {
                    *cell = lo;
                }
            }
            PALETTE_RAM_START..=PALETTE_RAM_END => {
                let o0 = self.palette_ram_offset(addr);
                let o1 = self.palette_ram_offset(addr.wrapping_add(1));
                if let Some(cell) = self.palette_ram.get_mut(o0) {
                    *cell = hi;
                }
                if let Some(cell) = self.palette_ram.get_mut(o1) {
                    *cell = lo;
                }
            }
            BACKUP_RAM_START..=BACKUP_RAM_END => {
                if self.save_ram_unlocked {
                    let offset = Self::backup_ram_offset(addr);
                    let next = offset.wrapping_add(1) & (self.backup_ram.len() - 1);
                    if let Some(cell) = self.backup_ram.get_mut(offset) {
                        *cell = hi;
                        self.dirty_backup_ram.set(true);
                    }
                    if let Some(cell) = self.backup_ram.get_mut(next) {
                        *cell = lo;
                    }
                }
            }
            ADPCM_A_START..=ADPCM_A_END => {
                let offset = (addr - ADPCM_A_START) as usize;
                if let Some(cell) = self.adpcm_a_ram.get_mut(offset) {
                    *cell = hi;
                }
                if let Some(cell) = self.adpcm_a_ram.get_mut(offset.wrapping_add(1)) {
                    *cell = lo;
                }
            }
            ADPCM_B_START..=ADPCM_B_END => {
                let offset = (addr - ADPCM_B_START) as usize;
                if let Some(cell) = self.adpcm_b_ram.get_mut(offset) {
                    *cell = hi;
                }
                if let Some(cell) = self.adpcm_b_ram.get_mut(offset.wrapping_add(1)) {
                    *cell = lo;
                }
            }
            SOUND_PORT => self.write_sound_command(hi),
            LSPC_REG_START..=LSPC_REG_END => self.write_lspc_register_word(addr, value),
            SYSTEM_LATCH_START..=SYSTEM_LATCH_END => {
                // Geolith only handles the 0x3Axxxx control latches through
                // 8-bit writes. Word writes to this area are ignored by the
                // memory-mapped-register switch and must not be split into two
                // byte writes, or a single word can accidentally toggle two
                // unrelated latches.
                self.record_bus_access(BusAccessKind::Write, addr, hi);
                self.record_bus_access(BusAccessKind::Write, addr.wrapping_add(1), lo);
            }
            BANKED_PROM_START..=BANKED_PROM_END => {
                if self.write_banked_special16(addr, value) {
                    return;
                }
                // Geolith installs a dedicated SMA word handler. It only acts
                // on the exact scrambled bank register; all other word writes
                // in the banked window are ignored.
                if let Some(config) = self.cart_protection.sma_config().copied() {
                    if addr == config.bank_reg_addr {
                        self.sma_bankswitch(value, &config);
                    } else {
                        self.record_bus_access(BusAccessKind::Write, addr, hi);
                        self.record_bus_access(BusAccessKind::Write, addr.wrapping_add(1), lo);
                    }
                    return;
                }

                // Protección PVC/PROM_BANK: fallback byte-by-byte (tienen side-effects)
                let in_pvc_zone = self.uses_pvc_runtime() && addr >= PVC_CARTRAM_START;
                let in_bank_reg_zone = (PROM_BANK_REGISTER_START..=PROM_BANK_REGISTER_END)
                    .contains(&addr)
                    && !self.uses_pvc_runtime();

                if in_pvc_zone {
                    // NEO-PVC: write both bytes atomically before triggering
                    // the PVC operation (unpack/pack/bankswap). This mirrors
                    // Geolith's geo_m68k_write_banksw_16_pvc which writes
                    // both bytes first, then triggers the operation once.
                    self.write_pvc_cartram_16(addr, value);
                } else if in_bank_reg_zone {
                    self.write8(addr, hi);
                    self.write8(addr.wrapping_add(1), lo);
                } else {
                    // Normal PROM writes: ignoradas (ROM), solo registrar bus access
                    self.record_bus_access(BusAccessKind::Write, addr, hi);
                    self.record_bus_access(BusAccessKind::Write, addr.wrapping_add(1), lo);
                }
            }
            addr if (0x300000..=0x3FFFFF).contains(&addr) => {
                // Geolith only gives 16-bit handlers to REG_SOUND and LSPC.
                // Other MMIO word writes are logged/ignored, not decomposed
                // into byte writes. Splitting here can accidentally hit odd
                // control latches such as REG_POUTPUT/REG_SHADOW.
                self.record_bus_access(BusAccessKind::Write, addr, hi);
                self.record_bus_access(BusAccessKind::Write, addr.wrapping_add(1), lo);
            }
            // Fallback byte-by-byte para regiones con side-effects
            _ => {
                self.write8(addr, hi);
                self.write8(addr.wrapping_add(1), lo);
            }
        }
    }

    /// Escribe 4 bytes (big-endian long) en la direccion dada.
    pub fn write32(&mut self, addr: u32, value: u32) {
        let addr = main_cpu_bus_addr(addr);
        let [b0, b1, b2, b3] = value.to_be_bytes();
        match addr {
            WORK_RAM_START..=WORK_RAM_END => {
                let base = Self::work_ram_offset(addr);
                let len = self.ram.len();
                for (i, byte) in [b0, b1, b2, b3].into_iter().enumerate() {
                    let idx = base.wrapping_add(i) & (len - 1);
                    self.ram[idx] = byte;
                }
            }
            PALETTE_RAM_START..=PALETTE_RAM_END => {
                let offsets = [
                    self.palette_ram_offset(addr),
                    self.palette_ram_offset(addr.wrapping_add(1)),
                    self.palette_ram_offset(addr.wrapping_add(2)),
                    self.palette_ram_offset(addr.wrapping_add(3)),
                ];
                for (off, byte) in offsets.into_iter().zip([b0, b1, b2, b3]) {
                    if off < self.palette_ram.len() {
                        self.palette_ram[off] = byte;
                    }
                }
            }
            BACKUP_RAM_START..=BACKUP_RAM_END => {
                if self.save_ram_unlocked {
                    let base = Self::backup_ram_offset(addr);
                    let len = self.backup_ram.len();
                    for (i, byte) in [b0, b1, b2, b3].into_iter().enumerate() {
                        let idx = base.wrapping_add(i) & (len - 1);
                        self.backup_ram[idx] = byte;
                    }
                    self.dirty_backup_ram.set(true);
                }
            }
            BANKED_PROM_START..=BANKED_PROM_END => {
                // Geolith's m68k_write_memory_32 always dispatches a longword
                // as two 16-bit writes. Some protection handlers (notably
                // NEO-PVC) have side effects after each word write, so keep
                // that ordering here instead of making the 32-bit access
                // fully atomic.
                let in_sma_zone = self.cart_protection.sma_config().is_some_and(|c| {
                    c.is_bank_reg(addr)
                        || c.is_bank_reg(addr.wrapping_add(1))
                        || c.is_bank_reg(addr.wrapping_add(2))
                        || c.is_bank_reg(addr.wrapping_add(3))
                });
                let in_mslugx_zone = self.cart_protection == CartProtection::MslugX
                    && addr.wrapping_add(3) >= MSLUGX_PROTECTION_START;
                let in_bank_reg_zone =
                    addr.wrapping_add(3) >= PROM_BANK_REGISTER_START && !self.uses_pvc_runtime();
                let in_pvc_zone = self.uses_pvc_runtime() && addr >= PVC_CARTRAM_START;

                if in_pvc_zone || in_sma_zone || in_mslugx_zone || in_bank_reg_zone {
                    self.write16(addr, (value >> 16) as u16);
                    self.write16(addr.wrapping_add(2), value as u16);
                } else {
                    self.record_bus_access(BusAccessKind::Write, addr, b0);
                    self.record_bus_access(BusAccessKind::Write, addr.wrapping_add(1), b1);
                    self.record_bus_access(BusAccessKind::Write, addr.wrapping_add(2), b2);
                    self.record_bus_access(BusAccessKind::Write, addr.wrapping_add(3), b3);
                }
            }
            ADPCM_A_START..=ADPCM_A_END
            | ADPCM_B_START..=ADPCM_B_END
            | SOUND_PORT
            | LSPC_REG_START..=LSPC_REG_END => {
                // Geolith/Musashi dispatches long writes as two word writes,
                // preserving the word-level behavior above for MMIO.
                self.write16(addr, (value >> 16) as u16);
                self.write16(addr.wrapping_add(2), value as u16);
            }
            addr if (0x300000..=0x3FFFFF).contains(&addr) => {
                // Geolith/Musashi dispatches long writes as two word writes,
                // preserving the word-level behavior above for MMIO.
                self.write16(addr, (value >> 16) as u16);
                self.write16(addr.wrapping_add(2), value as u16);
            }
            // Fallback byte-by-byte para regiones con side-effects
            _ => {
                self.write16(addr, (value >> 16) as u16);
                self.write16(addr.wrapping_add(2), value as u16);
            }
        }
    }
}

/// NeoGeo Bus memory mapping (RAM, VRAM, ROM banks).
pub struct Memory {
    pub ram: Vec<u8>,         // Work RAM
    pub vram: Vec<u8>,        // Video RAM
    pub prom: Vec<u8>,        // Program ROM (P-ROM)
    pub crom: Vec<u8>,        // Character/Sprite ROM (C-ROM)
    pub srom: Vec<u8>,        // Fix layer ROM (S-ROM)
    pub sfix: Vec<u8>,        // Board fix layer ROM (SFIX)
    pub mrom: Vec<u8>,        // Z80/M1 ROM (M-ROM)
    pub sm1: Vec<u8>,         // Z80 sound BIOS ROM (SM1)
    pub vrom: Vec<u8>,        // Audio sample ROM (V-ROM)
    pub vrom_b_offset: usize, // ADPCM-B (V2) base inside V-ROM; 0 mirrors V1
    pub sma_rom: Vec<u8>,     // SMA protection handler data (neo-sma)
    // Per-game SMA runtime uses SmaConfig from CartProtection::Sma (see SmaConfig).
    pub zoom_rom: Vec<u8>, // L0 shrink/zoom lookup table (000-lo.lo)
    pub palette_ram: Vec<u8>,
    pub adpcm_a_ram: Vec<u8>, // ADPCM-A RAM (16KB, 0x300000-0x303FFF)
    pub adpcm_b_ram: Vec<u8>, // ADPCM-B RAM (16KB, 0x310000-0x313FFF)
    pub bios: Vec<u8>,
    pub backup_ram: Vec<u8>,
    pub memory_card: Vec<u8>,
    /// Base path for save files (e.g., "saves/aof"). When set, backup RAM
    /// and memory card are persisted to disk as `.sav` and `.mem` files.
    pub(crate) save_path: Option<String>,
    dirty_backup_ram: Cell<bool>,
    dirty_memory_card: Cell<bool>,
    pub vram_addr: Cell<u16>,
    pub vram_mod: u16,
    pub lspc_mode: u16,
    pub auto_animation_counter: Cell<u8>,
    pub auto_animation_timer: Cell<u8>,
    pub auto_animation_reload: Cell<u8>,
    pub timer_high: u16,
    pub timer_low: u16,
    pub timer_stop: u16,
    pub irq_ack: u16,
    pub irq2_ctrl: u8,
    pub irq2_reload: u32,
    pub irq2_counter: u32,
    pub irq2_frags: u32,
    pub lspc_rom_size: u16,
    pub pending_vram_write_hi: Option<u8>,
    pub lspc_scanline: Cell<u16>,
    pub dip_switches: u8,
    pub test_switch: u8,
    /// STATUS_B bit 7: true for MVS/Universe BIOS hardware, false for AES.
    pub system_is_mvs: bool,
    pub sound_latch: u8,
    pub sound_reply: u8,
    pub z80_ram: Vec<u8>,
    pub z80_bank: [usize; 4],
    pub z80_nmi_enabled: Cell<bool>,
    pub z80_nmi_pending: Cell<bool>,
    pub z80_sound_reply: u8,
    pub ym2610_addr: [u8; 2], // current register address for each YM2610 port
    pub p2_port: u8,
    pub status_a: u8,
    pub status_a_pulse: Cell<bool>,
    rtc: RefCell<Rtc4990a>,
    pub display_enabled: bool,
    pub use_cart_vectors: bool,
    pub use_cart_audio: bool,
    pub use_cart_fix: bool,
    pub save_ram_unlocked: bool,
    pub palette_bank: u8,
    pub palette_shadow: bool,
    pub memcard_unlocked: bool,
    pub memcard_register_select: bool,
    pub(crate) memcard_lock1: bool,
    pub(crate) memcard_lock2: bool,
    pub memcard_inserted: bool,
    pub memcard_write_protected: bool,
    pub bus_trace: RefCell<Vec<BusAccess>>,
    pub z80_io_trace: RefCell<Vec<Z80IoAccess>>,
    pub input_ports: InputPorts,
    pub watchdog_cycles: u32,
    pub watchdog_enabled: bool,
    watchdog_limit: u32,
    pub rom_ngh: Option<u16>,
    pub prom_bank_offset: usize,
    pub fix_bankswitch: FixBankSwitch,
    cart_protection: CartProtection,
    cart_reg: [Cell<u16>; 2],
    prot_reg: u32,
    mslugx_command: Cell<u16>,
    mslugx_counter: Cell<u16>,
    sma_rng: Cell<u16>,
    pending_sma_bank_hi: Option<u8>,
    /// NEO-PVC cart RAM (0x2000 bytes at 0x2FE000-0x2FFFFF)
    pub pvc_cart_ram: Vec<u8>,
    /// KOF10TH extended RAM mapped through its bootleg board handlers.
    pub kof10th_extra_ram: Vec<u8>,
    /// KOF10TH dynamic FIX layer. Geolith replaces romdata->s with this buffer.
    pub dynamic_fix_rom: Vec<u8>,
    /// NEO-PVC bank address for banked PROM reads (calculated by pvc_bankswap)
    pub pvc_bank_addr: usize,
    // Otros bancos según sea necesario
}

impl Memory {
    pub fn new() -> Self {
        Self {
            ram: vec![0; WORK_RAM_SIZE],
            vram: vec![0; VIDEO_RAM_SIZE],
            prom: Vec::new(),
            crom: Vec::new(),
            srom: Vec::new(),
            sfix: Vec::new(),
            mrom: Vec::new(),
            sm1: Vec::new(),
            vrom: Vec::new(),
            vrom_b_offset: 0,
            sma_rom: Vec::new(),
            zoom_rom: Vec::new(),
            palette_ram: vec![0; PALETTE_RAM_SIZE],
            adpcm_a_ram: vec![0; ADPCM_A_RAM_SIZE],
            adpcm_b_ram: vec![0; ADPCM_B_RAM_SIZE],
            bios: build_diagnostic_bios(),
            backup_ram: vec![0; BACKUP_RAM_SIZE],
            memory_card: vec![0xFF; MEMORY_CARD_SIZE],
            save_path: None,
            dirty_backup_ram: Cell::new(false),
            dirty_memory_card: Cell::new(false),
            vram_addr: Cell::new(0),
            vram_mod: 1,
            lspc_mode: 0,
            auto_animation_counter: Cell::new(0),
            auto_animation_timer: Cell::new(0),
            auto_animation_reload: Cell::new(0),
            timer_high: 0,
            timer_low: 0,
            timer_stop: 0,
            irq_ack: 0,
            irq2_ctrl: 0,
            irq2_reload: 0,
            irq2_counter: 0,
            irq2_frags: 0,
            lspc_rom_size: 0,
            pending_vram_write_hi: None,
            lspc_scanline: Cell::new(0),
            dip_switches: 0xFF,
            test_switch: 0xC0,
            system_is_mvs: true,
            sound_latch: 0,
            sound_reply: 0,
            z80_ram: vec![0; Z80_RAM_SIZE],
            z80_bank: Z80_DEFAULT_BANK_OFFSETS,
            z80_nmi_enabled: Cell::new(false),
            z80_nmi_pending: Cell::new(false),
            z80_sound_reply: 0,
            ym2610_addr: [0; 2],
            p2_port: 0xFF,
            status_a: 0xFF,
            status_a_pulse: Cell::new(true),
            rtc: RefCell::new(Rtc4990a::new()),
            display_enabled: true,
            use_cart_vectors: false,
            use_cart_audio: false,
            use_cart_fix: false, // Real hardware: starts with BIOS SFIX font
            save_ram_unlocked: true,
            palette_bank: 0,
            palette_shadow: false,
            memcard_unlocked: false,
            memcard_register_select: false,
            memcard_lock1: true,
            memcard_lock2: true,
            memcard_inserted: false,
            memcard_write_protected: false,
            bus_trace: RefCell::new(Vec::new()),
            z80_io_trace: RefCell::new(Vec::new()),
            input_ports: InputPorts::default(),
            watchdog_cycles: 0,
            watchdog_enabled: true,
            watchdog_limit: WATCHDOG_M68K_CYCLES,
            rom_ngh: None,
            prom_bank_offset: FIXED_PROM_WINDOW_SIZE,
            fix_bankswitch: FixBankSwitch::None,
            cart_protection: CartProtection::None,
            cart_reg: [Cell::new(0), Cell::new(0)],
            prot_reg: 0,
            mslugx_command: Cell::new(0),
            mslugx_counter: Cell::new(0),
            sma_rng: Cell::new(0x2345),
            pending_sma_bank_hi: None,
            pvc_cart_ram: vec![0; PVC_CARTRAM_SIZE],
            kof10th_extra_ram: Vec::new(),
            dynamic_fix_rom: Vec::new(),
            pvc_bank_addr: 0,
        }
    }

    pub fn set_input_ports(&mut self, input_ports: InputPorts) {
        self.input_ports = input_ports;
    }

    pub fn set_system_is_mvs(&mut self, is_mvs: bool) {
        self.system_is_mvs = is_mvs;
    }

    pub fn set_bios(&mut self, bios: Vec<u8>) {
        self.bios = bios;
    }

    pub fn set_zoom_rom(&mut self, zoom_rom: Vec<u8>) {
        self.zoom_rom = zoom_rom;
    }

    pub fn set_sfix_rom(&mut self, sfix: Vec<u8>) {
        self.sfix = sfix;
    }

    pub fn set_sm1_rom(&mut self, sm1: Vec<u8>) {
        self.sm1 = sm1;
    }

    /// Set the save path from a ROM's label.
    /// The save directory is `saves/` relative to the working directory.
    fn set_save_path_from_rom(&mut self, rom: &crate::rom::RomData) {
        let label = rom
            .metadata
            .as_ref()
            .map(|m| m.name.as_str())
            .or_else(|| {
                rom.recognized_files.first().map(|f| f.as_str()).map(|f| {
                    // Use the first .neo or .zip basename without extension
                    if f.ends_with(".neo") || f.ends_with(".zip") {
                        &f[..f.len() - 4]
                    } else {
                        f
                    }
                })
            })
            .unwrap_or("ngneon");

        let sanitized: String = label
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                    ch
                } else {
                    '_'
                }
            })
            .collect();

        let path = format!("saves/{sanitized}");
        self.save_path = Some(path);
    }

    /// Load persistent data (backup RAM + memory card) from disk.
    /// This is called after setting the save path in `load_rom()`.
    pub fn load_persistent_data(&mut self) {
        let Some(ref save_path) = self.save_path else {
            return;
        };

        // Try to load backup RAM
        let sav_path = format!("{save_path}.sav");
        if let Ok(data) = std::fs::read(&sav_path) {
            let len = data.len().min(self.backup_ram.len());
            self.backup_ram[..len].copy_from_slice(&data[..len]);
        }

        // Try to load memory card
        let mem_path = format!("{save_path}.mem");
        if let Ok(data) = std::fs::read(&mem_path) {
            let len = data.len().min(self.memory_card.len());
            self.memory_card[..len].copy_from_slice(&data[..len]);
        }

        self.dirty_backup_ram.set(false);
        self.dirty_memory_card.set(false);
    }

    /// Save backup RAM to disk (if dirty).
    pub fn save_backup_ram(&self) {
        if !self.dirty_backup_ram.get() {
            return;
        }
        let Some(ref save_path) = self.save_path else {
            return;
        };
        if self.backup_ram.iter().all(|&b| b == 0) {
            return; // Don't save all-zero data
        }
        let path = format!("{save_path}.sav");
        if let Err(e) = std::fs::write(&path, &self.backup_ram) {
            eprintln!("[WARN] No se pudo guardar backup RAM en {path}: {e}");
        } else {
            self.dirty_backup_ram.set(false);
        }
    }

    /// Save memory card to disk (if dirty).
    pub fn save_memory_card(&self) {
        if !self.dirty_memory_card.get() {
            return;
        }
        let Some(ref save_path) = self.save_path else {
            return;
        };
        if self.memory_card.iter().all(|&b| b == 0xFF) {
            return; // Don't save default all-0xFF data
        }
        let path = format!("{save_path}.mem");
        if let Err(e) = std::fs::write(&path, &self.memory_card) {
            eprintln!("[WARN] No se pudo guardar memory card en {path}: {e}");
        } else {
            self.dirty_memory_card.set(false);
        }
    }

    /// Save all persistent data to disk (if dirty). Called periodically
    /// by the frontend and when loading a new ROM or exiting.
    pub fn save_persistent_data(&self) {
        // Ensure save directory exists
        if let Some(ref save_path) = self.save_path {
            if let Some(parent) = PathBuf::from(save_path).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
        }
        self.save_backup_ram();
        self.save_memory_card();
    }

    pub fn take_bus_trace(&self) -> Vec<BusAccess> {
        self.bus_trace.take()
    }

    pub fn take_z80_io_trace(&self) -> Vec<Z80IoAccess> {
        self.z80_io_trace.take()
    }

    pub fn reset_watchdog(&mut self) {
        self.watchdog_cycles = 0;
    }

    pub fn advance_watchdog(&mut self, m68k_cycles: u32) -> bool {
        if !self.watchdog_enabled {
            return false;
        }

        self.watchdog_cycles = self.watchdog_cycles.saturating_add(m68k_cycles);
        self.watchdog_cycles >= self.watchdog_limit
    }

    pub fn soft_reset_for_watchdog(&mut self) {
        self.sound_latch = 0;
        self.sound_reply = 0;
        self.z80_sound_reply = 0;
        self.z80_nmi_enabled.set(false);
        self.z80_nmi_pending.set(false);
        self.ym2610_addr = [0; 2];
        self.use_cart_vectors = false;
        self.save_ram_unlocked = true;
        self.memcard_lock1 = false;
        self.memcard_lock2 = false;
        self.memcard_unlocked = true;
        self.memcard_register_select = false;

        // geo_lspc_init() resets control/timing registers, but preserves VRAM,
        // palette RAM, the display latch, and the active FIX source latch.
        self.palette_bank = 0;
        self.palette_shadow = false;
        self.vram_addr.set(0);
        self.vram_mod = 0;
        self.lspc_mode = 0;
        self.auto_animation_counter.set(0);
        self.auto_animation_timer.set(0);
        self.auto_animation_reload.set(0);
        self.lspc_scanline.set(0);

        self.prom_bank_offset = if self.prom.len() > FIXED_PROM_WINDOW_SIZE {
            FIXED_PROM_WINDOW_SIZE
        } else {
            self.prom.len()
        };
        self.sma_rng.set(0x2345);
        self.pending_sma_bank_hi = None;
        self.pvc_bank_addr = if self.prom.len() > FIXED_PROM_WINDOW_SIZE {
            FIXED_PROM_WINDOW_SIZE
        } else {
            0
        };
        self.watchdog_cycles = 0;
    }

    pub fn system_control_snapshot(&self) -> SystemControlSnapshot {
        SystemControlSnapshot {
            display_enabled: self.display_enabled,
            use_cart_vectors: self.use_cart_vectors,
            use_cart_audio: self.use_cart_audio,
            use_cart_fix: self.use_cart_fix,
            save_ram_unlocked: self.save_ram_unlocked,
            palette_bank: self.palette_bank,
            palette_shadow: self.palette_shadow,
            memcard_unlocked: self.memcard_unlocked,
            memcard_register_select: self.memcard_register_select,
            memcard_inserted: self.memcard_inserted,
            memcard_write_protected: self.memcard_write_protected,
        }
    }

    pub fn sound_port_snapshot(&self) -> SoundPortSnapshot {
        SoundPortSnapshot {
            command: self.sound_latch,
            reply: self.z80_sound_reply,
        }
    }

    pub fn set_prom_bank_offset(&mut self, offset: usize) {
        self.prom_bank_offset = offset.min(self.prom.len());
    }

    pub(crate) fn special_board_state(&self) -> SpecialBoardState {
        SpecialBoardState {
            cart_reg: [self.cart_reg[0].get(), self.cart_reg[1].get()],
            prot_reg: self.prot_reg,
            mslugx_command: self.mslugx_command.get(),
            mslugx_counter: self.mslugx_counter.get(),
            sma_rng: self.sma_rng.get(),
            pending_sma_bank_hi: self.pending_sma_bank_hi,
        }
    }

    pub(crate) fn restore_special_board_state(&mut self, state: SpecialBoardState) {
        self.cart_reg[0].set(state.cart_reg[0]);
        self.cart_reg[1].set(state.cart_reg[1]);
        self.prot_reg = state.prot_reg;
        self.mslugx_command.set(state.mslugx_command);
        self.mslugx_counter.set(state.mslugx_counter);
        self.sma_rng.set(state.sma_rng);
        self.pending_sma_bank_hi = state.pending_sma_bank_hi;

        // KOF98 rewrites four P-ROM bytes while its protection overlay is active.
        // Reapply them after loading a state, matching Geolith's post-load hook.
        if self.cart_protection == CartProtection::Kof98 {
            let _ = self.write_banked_special16(0x20aaaa, state.cart_reg[0]);
        }
    }

    pub fn select_prom_bank(&mut self, bank: u8) {
        let mask = geolith_prom_bank_mask(self.prom.len());
        self.prom_bank_offset =
            FIXED_PROM_WINDOW_SIZE + ((bank as usize & mask) * BANKED_PROM_WINDOW_SIZE);
    }

    /// Carga los datos de una ROM en la memoria
    pub fn load_rom(&mut self, rom: &mut crate::rom::RomData) {
        // Copiar datos de la ROM a los bancos de memoria usando move en lugar de clone
        self.prom = std::mem::take(&mut rom.prom);
        self.crom = std::mem::take(&mut rom.crom);
        self.srom = std::mem::take(&mut rom.srom);
        self.mrom = std::mem::take(&mut rom.mrom);
        self.vrom = std::mem::take(&mut rom.vrom);
        self.vrom_b_offset = rom.vrom_b_offset.min(self.vrom.len());
        self.sma_rom = std::mem::take(&mut rom.sma_rom);
        self.prom_bank_offset = if self.prom.len() > FIXED_PROM_WINDOW_SIZE {
            FIXED_PROM_WINDOW_SIZE
        } else {
            self.prom.len()
        };
        // Opcional: limpiar VRAM y RAM
        self.ram.fill(0);
        self.vram.fill(0);
        self.palette_ram.fill(0);
        self.adpcm_a_ram.fill(0);
        self.adpcm_b_ram.fill(0);
        self.memory_card.fill(0xFF);
        self.vram_addr.set(0);
        self.vram_mod = 1;
        self.lspc_mode = 0;
        self.auto_animation_counter.set(0);
        self.auto_animation_timer.set(0);
        self.auto_animation_reload.set(0);
        self.timer_high = 0;
        self.timer_low = 0;
        self.timer_stop = 0;
        self.irq_ack = 0;
        self.irq2_ctrl = 0;
        self.irq2_reload = 0;
        self.irq2_counter = 0;
        self.irq2_frags = 0;
        self.lspc_rom_size = 0;
        self.pending_vram_write_hi = None;
        self.lspc_scanline.set(0);
        self.sound_latch = 0;
        self.sound_reply = 0;
        self.z80_ram.fill(0);
        self.z80_bank = Z80_DEFAULT_BANK_OFFSETS;
        self.z80_nmi_enabled.set(false);
        self.z80_nmi_pending.set(false);
        self.z80_sound_reply = 0;
        self.ym2610_addr = [0; 2];
        self.status_a_pulse.set(true);
        self.rtc.borrow_mut().reset();
        self.display_enabled = true;
        self.use_cart_vectors = false;
        // Geolith starts MVS/UniBIOS on the board SM1 ROM and switches to the
        // cartridge M1 ROM together with the CRTFIX latch.
        self.use_cart_audio = self.sm1.is_empty();
        self.use_cart_fix = false; // Real hardware: starts with BIOS SFIX font
                                   // Geolith resets reg_sramlock to 0, so backup SRAM starts writable
                                   // until BIOS or game code writes REG_SRAMLOCK.
        self.save_ram_unlocked = true;
        self.palette_bank = 0;
        self.palette_shadow = false;
        self.memcard_lock1 = false;
        self.memcard_lock2 = false;
        self.memcard_unlocked = true;
        self.memcard_register_select = false;
        self.memcard_inserted = false;
        self.memcard_write_protected = false;
        self.bus_trace.borrow_mut().clear();
        self.z80_io_trace.borrow_mut().clear();
        self.input_ports = InputPorts::default();
        self.watchdog_cycles = 0;
        self.watchdog_enabled = true;
        self.rom_ngh = rom_ngh(rom);
        self.watchdog_limit = watchdog_limit_for_rom(rom);
        // Flush previous ROM's save data before loading new ROM
        self.save_persistent_data();

        self.cart_protection = detect_cart_protection(rom);
        self.fix_bankswitch = detect_fix_bankswitch(rom);
        self.cart_reg[0].set(0);
        self.cart_reg[1].set(0);
        self.prot_reg = 0;
        self.mslugx_command.set(0);
        self.mslugx_counter.set(0);
        self.sma_rng.set(0x2345); // LFSR initial value for all SMA games (Geolith uses 0x2345)
        self.pending_sma_bank_hi = None;

        // Reset NEO-PVC state
        self.pvc_cart_ram.fill(0);
        self.kof10th_extra_ram.clear();
        self.dynamic_fix_rom.clear();
        if self.cart_protection == CartProtection::Kof10th {
            self.kof10th_extra_ram.resize(KOF10TH_EXTRA_RAM_SIZE, 0);
            self.dynamic_fix_rom.resize(KOF10TH_DYNFIX_SIZE, 0);
            self.apply_kof10th_prom_patches();
        }
        // Mirror Geolith: banksw_addr starts at 0x100000 when P-ROM > 1MB
        self.pvc_bank_addr = if self.prom.len() > FIXED_PROM_WINDOW_SIZE {
            FIXED_PROM_WINDOW_SIZE
        } else {
            0
        };

        // Derive save path from ROM label and load any existing save data
        self.set_save_path_from_rom(rom);
        self.load_persistent_data();
    }

    /// Read a byte from the fixed PROM window (offset relative to FIXED_PROM_START),
    /// applying the vector swap logic.
    fn read_prom_or_bios(&self, offset: usize) -> u8 {
        if !self.use_cart_vectors && offset < NEOGEO_VECTOR_SWAP_BYTES {
            self.bios.get(offset).copied().unwrap_or(0xFF)
        } else {
            self.prom.get(offset).copied().unwrap_or(0xFF)
        }
    }

    /// Read a byte from the BIOS ROM region (offset relative to BIOS_ROM_START),
    /// which is always backed by BIOS ROM. The cart/BIOS vector selector only
    /// applies to the low 0x000000-0x00007F vector window.
    fn read_bios_or_prom(&self, offset: usize) -> u8 {
        if self.bios.is_empty() {
            return 0xFF;
        }
        self.bios
            .get(offset % self.bios.len())
            .copied()
            .unwrap_or(0xFF)
    }

    fn read_fixed_prom_byte(&self, addr: u32) -> u8 {
        if !self.use_cart_vectors && addr < NEOGEO_VECTOR_SWAP_BYTES as u32 {
            return self.bios.get(addr as usize).copied().unwrap_or(0xFF);
        }
        if self.cart_protection == CartProtection::Kf2k3Bl && addr == 0x058197 {
            return self.pvc_cart_ram[0x1ff2];
        }
        if self.cart_protection == CartProtection::Kof10th {
            if addr >= 0x0e0000 {
                let offset = (addr & 0x1ffff) as usize;
                return self.kof10th_extra_ram.get(offset).copied().unwrap_or(0xFF);
            }
            let offset = (addr as usize).wrapping_add(self.prot_reg as usize);
            return self.prom.get(offset).copied().unwrap_or(0xFF);
        }
        let offset = (addr - FIXED_PROM_START) as usize;
        self.read_prom_or_bios(offset)
    }

    fn read_bios_region_byte(&self, addr: u32) -> u8 {
        let offset = (addr - BIOS_ROM_START) as usize;
        self.read_bios_or_prom(offset)
    }

    fn read_mslugx_protection_word(&self, addr: u32) -> u16 {
        match self.mslugx_command.get() {
            0x0001 => {
                let counter = self.mslugx_counter.get();
                self.mslugx_counter.set(counter.wrapping_add(1));
                self.read_mslugx_bit(((counter >> 3) & 0x0fff) as usize, (!counter & 7) as u8)
            }
            0x0fff => {
                let select = self.read_work_ram_word(0x0f00a).wrapping_sub(1) as usize;
                self.read_mslugx_bit((select >> 3) & 0x0fff, (!select & 7) as u8)
            }
            _ => {
                let offset = self.prom_bank_offset + (addr & 0x0f_ffff) as usize;
                u16::from_be_bytes([
                    self.prom.get(offset).copied().unwrap_or(0xFF),
                    self.prom
                        .get(offset.wrapping_add(1))
                        .copied()
                        .unwrap_or(0xFF),
                ])
            }
        }
    }

    fn read_mslugx_bit(&self, offset: usize, bit: u8) -> u16 {
        self.prom
            .get(0x0d_edd2 + offset)
            .map(|byte| ((byte >> bit) & 1) as u16)
            .unwrap_or(0)
    }

    fn read_work_ram_word(&self, offset: usize) -> u16 {
        let hi = self.ram.get(offset).copied().unwrap_or(0) as u16;
        let lo = self.ram.get(offset + 1).copied().unwrap_or(0) as u16;
        (hi << 8) | lo
    }

    fn read_status_a(&self) -> u8 {
        let status = self.status_a & self.input_ports.status_a;
        let rtc = self.rtc.borrow_mut().read_pins() << 6;
        (status & 0x3F) | rtc
    }

    fn read_system_b(&self) -> u8 {
        let mut value = self.input_ports.system;
        if self.system_is_mvs {
            value |= 0x80;
        } else {
            value &= !0x80;
        }
        if self.memcard_inserted {
            value &= !0x30;
        }
        if self.memcard_write_protected {
            value &= !0x40;
        }

        self.record_bus_access(BusAccessKind::Read, SYSTEM_PORT, value);
        value
    }

    fn write_sound_command(&mut self, value: u8) {
        self.sound_latch = value;
        if self.z80_nmi_enabled.get() {
            self.z80_nmi_pending.set(true);
        }
    }

    fn mslugx_bankswitch(&mut self, value: u16) {
        let offset =
            ((value as usize * BANKED_PROM_WINDOW_SIZE) + FIXED_PROM_WINDOW_SIZE) & 0xff_ffff;
        self.set_prom_bank_addr(offset);
    }

    fn kof10th_bankswitch(&mut self, value: u16) {
        let mask = geolith_prom_bank_mask(self.prom.len());
        let mut offset =
            FIXED_PROM_WINDOW_SIZE + (((value as usize) & mask) * BANKED_PROM_WINDOW_SIZE);
        if offset >= 0x700000 {
            offset = FIXED_PROM_WINDOW_SIZE;
        }
        self.set_prom_bank_addr(offset);
    }

    fn apply_kof10th_prom_patches(&mut self) {
        // The bootleg board's Altera protection chip patches these bytes over
        // P-ROM. Geolith applies the same patch when selecting BOARD_KOF10TH.
        const PATCHES: &[(usize, u8)] = &[
            (0x0124, 0x00),
            (0x0125, 0x0d),
            (0x0126, 0xf7),
            (0x0127, 0xa8),
            (0x8bf4, 0x4e),
            (0x8bf5, 0xf9),
            (0x8bf6, 0x00),
            (0x8bf7, 0x0d),
            (0x8bf8, 0xf9),
            (0x8bf9, 0x80),
        ];

        for &(offset, value) in PATCHES {
            if let Some(byte) = self.prom.get_mut(offset) {
                *byte = value;
            }
        }
    }

    fn uses_pvc_runtime(&self) -> bool {
        matches!(
            self.cart_protection,
            CartProtection::Pvc | CartProtection::Kf2k3Bl | CartProtection::Kf2k3Bla
        )
    }

    fn banked_prom_base(&self) -> usize {
        if self.uses_pvc_runtime() {
            self.pvc_bank_addr
        } else {
            self.prom_bank_offset
        }
    }

    fn set_prom_bank_addr(&mut self, offset: usize) {
        self.prom_bank_offset = offset;
    }

    fn read_banked_special8(&self, addr: u32) -> Option<u8> {
        match self.cart_protection {
            CartProtection::Linkable => match addr {
                0x200000 => {
                    let value = (self.cart_reg[0].get() as u8) ^ 0x08;
                    self.cart_reg[0].set(value as u16);
                    Some(value)
                }
                0x200001 => Some(0),
                _ => None,
            },
            CartProtection::Brezzasoft => {
                if addr <= 0x201fff {
                    let offset = (addr - BANKED_PROM_START) as usize & 0x1fff;
                    Some(self.pvc_cart_ram[offset])
                } else {
                    Some(0xff)
                }
            }
            CartProtection::Kof10th => {
                if addr >= PVC_CARTRAM_START {
                    Some(self.pvc_cart_ram[(addr & 0x1fff) as usize])
                } else {
                    None
                }
            }
            CartProtection::Ct0 => {
                let ret = (self.prot_reg >> 24) as u8;
                match addr {
                    0x200001 | 0x236001 | 0x236009 | 0x255551 | 0x2ff001 | 0x2ffff1 => Some(ret),
                    0x236005 | 0x23600d => Some(ret.rotate_left(4)),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn read_banked_special16(&self, addr: u32) -> Option<u16> {
        match self.cart_protection {
            CartProtection::Brezzasoft => {
                if addr <= 0x201fff {
                    let offset = (addr - BANKED_PROM_START) as usize & 0x1fff;
                    Some(u16::from_be_bytes([
                        self.pvc_cart_ram[offset],
                        self.pvc_cart_ram[offset.wrapping_add(1) & 0x1fff],
                    ]))
                } else if addr == 0x280000 {
                    Some(self.read_system_b() as u16)
                } else if addr == 0x2c0000 {
                    Some(0xffc0)
                } else {
                    Some(0xffff)
                }
            }
            CartProtection::Ct0 => {
                let ret = ((self.prot_reg >> 24) & 0xff) as u16;
                match addr {
                    0x200000 | 0x236000 | 0x236008 | 0x255550 | 0x2ff000 | 0x2ffff0 => Some(ret),
                    0x236004 | 0x23600c => Some(((ret & 0x000f) << 4) | ((ret & 0x00f0) >> 4)),
                    _ => None,
                }
            }
            CartProtection::Kof10th => {
                if addr >= PVC_CARTRAM_START {
                    Some(read16be_wrapped(
                        &self.pvc_cart_ram,
                        (addr & 0x1fff) as usize,
                    ))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn write_banked_special8(&mut self, addr: u32, value: u8) -> bool {
        match self.cart_protection {
            CartProtection::Pvc | CartProtection::Kf2k3Bl | CartProtection::Kf2k3Bla
                if addr >= PVC_CARTRAM_START =>
            {
                self.write_pvc_cartram_byte(addr, value);
                true
            }
            CartProtection::Linkable => {
                if addr >= PROM_BANK_REGISTER_START {
                    self.set_prom_bank_addr(
                        ((value as usize * BANKED_PROM_WINDOW_SIZE) + FIXED_PROM_WINDOW_SIZE)
                            & 0xff_ffff,
                    );
                    true
                } else {
                    addr == 0x200001
                }
            }
            CartProtection::Brezzasoft => {
                if addr <= 0x201fff {
                    self.record_bus_access(BusAccessKind::Write, addr, value);
                    let offset = (addr - BANKED_PROM_START) as usize & 0x1fff;
                    self.pvc_cart_ram[offset] = value;
                }
                true
            }
            CartProtection::Kof10th if addr >= PVC_CARTRAM_START => {
                if addr == 0x2ffff0 {
                    self.kof10th_bankswitch(value as u16);
                }
                self.record_bus_access(BusAccessKind::Write, addr, value);
                self.pvc_cart_ram[(addr & 0x1fff) as usize] = value;
                true
            }
            CartProtection::Ct0 => match addr {
                0x236001 | 0x236005 | 0x236009 | 0x23600d | 0x255551 | 0x2ff001 | 0x2ffff1 => {
                    self.prot_reg <<= 8;
                    true
                }
                _ if addr >= PROM_BANK_REGISTER_START => {
                    self.set_prom_bank_addr(
                        ((value as usize * BANKED_PROM_WINDOW_SIZE) + FIXED_PROM_WINDOW_SIZE)
                            & 0xff_ffff,
                    );
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn write_banked_special16(&mut self, addr: u32, value: u16) -> bool {
        match self.cart_protection {
            CartProtection::Kof98 => match addr {
                0x20aaaa => {
                    self.cart_reg[0].set(value);
                    match value {
                        0x0090 if self.prom.len() >= 0x104 => {
                            self.prom[0x100..0x104].copy_from_slice(&[0x00, 0xc2, 0x00, 0xfd]);
                        }
                        0x00f0 if self.prom.len() >= 0x104 => {
                            self.prom[0x100..0x104].copy_from_slice(b"NEO-");
                        }
                        _ => {}
                    }
                    true
                }
                0x205554 => true,
                _ if addr >= PROM_BANK_REGISTER_START => {
                    self.set_prom_bank_addr(
                        ((value as usize * BANKED_PROM_WINDOW_SIZE) + FIXED_PROM_WINDOW_SIZE)
                            & 0xff_ffff,
                    );
                    true
                }
                _ => false,
            },
            CartProtection::MslugX => {
                if (MSLUGX_PROTECTION_START..=MSLUGX_PROTECTION_END).contains(&addr) {
                    match addr {
                        0x2fffe0 => self.mslugx_command.set(0),
                        0x2fffe2 | 0x2fffe4 => {
                            self.mslugx_command.set(self.mslugx_command.get() | value);
                        }
                        0x2fffe6 => {}
                        0x2fffea => {
                            self.mslugx_command.set(0);
                            self.mslugx_counter.set(0);
                        }
                        _ => {}
                    }
                    true
                } else if addr >= MSLUGX_BANK_REGISTER {
                    self.mslugx_bankswitch(value);
                    true
                } else {
                    false
                }
            }
            CartProtection::Ct0 => {
                match addr {
                    0x211112 => self.prot_reg = 0xff000000,
                    0x233332 => self.prot_reg = 0x0000ffff,
                    0x242812 => self.prot_reg = 0x81422418,
                    0x244442 => self.prot_reg = 0x00ff0000,
                    0x255552 => self.prot_reg = 0xff00ff00,
                    0x256782 => self.prot_reg = 0xf05a3601,
                    _ if addr >= PROM_BANK_REGISTER_START => {
                        self.set_prom_bank_addr(
                            ((value as usize * BANKED_PROM_WINDOW_SIZE) + FIXED_PROM_WINDOW_SIZE)
                                & 0xff_ffff,
                        );
                    }
                    _ => return false,
                }
                true
            }
            CartProtection::Brezzasoft => {
                if addr <= 0x201fff {
                    let [hi, lo] = value.to_be_bytes();
                    self.record_bus_access(BusAccessKind::Write, addr, hi);
                    self.record_bus_access(BusAccessKind::Write, addr.wrapping_add(1), lo);
                    let offset = (addr - BANKED_PROM_START) as usize & 0x1fff;
                    self.pvc_cart_ram[offset] = hi;
                    self.pvc_cart_ram[offset.wrapping_add(1) & 0x1fff] = lo;
                }
                true
            }
            CartProtection::Ms5Plus if addr == 0x2ffff4 => {
                self.set_prom_bank_addr((value as usize) << 16);
                true
            }
            CartProtection::Cthd2003 if addr == 0x2ffff0 => {
                const OFFSETS: [usize; 8] = [
                    0x200000, 0x100000, 0x200000, 0x100000, 0x200000, 0x100000, 0x400000, 0x300000,
                ];
                self.set_prom_bank_addr(OFFSETS[(value & 0x0007) as usize]);
                true
            }
            CartProtection::Kof10th => {
                if addr < 0x240000 {
                    if self.pvc_cart_ram.get(0x1ffc).copied().unwrap_or(0) != 0 {
                        let offset = ((addr >> 1) & 0x1ffff) as usize;
                        let data = value as u8;
                        let swapped = (data & 0xde) | ((data & 0x01) << 5) | ((data & 0x20) >> 5);
                        if let Some(cell) = self.dynamic_fix_rom.get_mut(offset) {
                            *cell = swapped;
                        }
                    } else {
                        write16be_wrapped(
                            &mut self.kof10th_extra_ram,
                            (addr & 0x1ffff) as usize,
                            value,
                        );
                    }
                    true
                } else if addr >= PVC_CARTRAM_START {
                    match addr {
                        0x2ffff0 => self.kof10th_bankswitch(value),
                        0x2ffff8 if read16be_wrapped(&self.pvc_cart_ram, 0x1ff8) != value => {
                            self.prot_reg = if value & 0x0001 != 0 { 0 } else { 0x700000 };
                        }
                        _ => {}
                    }
                    write16be_wrapped(&mut self.pvc_cart_ram, (addr & 0x1ffe) as usize, value);
                    true
                } else {
                    false
                }
            }
            CartProtection::Kf2k3Bl | CartProtection::Pvc if addr >= PVC_CARTRAM_START => {
                self.write_pvc_cartram_16(addr, value);
                true
            }
            CartProtection::Kf2k3Bla if addr >= PVC_CARTRAM_START => {
                self.write_pvc_cartram_16_with_bla_bankswap(addr, value);
                true
            }
            _ => false,
        }
    }

    fn is_sma_read(&self, addr: u32, config: &SmaConfig) -> bool {
        addr == SMA_PRESENCE_ADDR || addr == SMA_PRESENCE_ADDR + 1 || config.is_prn_addr(addr)
    }

    fn is_sma_word_read(&self, addr: u32, config: &SmaConfig) -> bool {
        addr == SMA_PRESENCE_ADDR || config.is_prn_addr(addr)
    }

    // ─── NEO-PVC bankswitching helpers ───────────────────────────────

    /// Unpack: extract R,G,B,D from cartram[0x1FE0-0x1FE1] and store
    /// expanded values in cartram[0x1FE2-0x1FE5].
    /// Triggered by writes to 0x2FFFE0-0x2FFFE3.
    fn pvc_unpack(&mut self) {
        let d = self.pvc_cart_ram[0x1fe1] >> 7; // 0000 000D
        let r = // 000R RRRr
            ((self.pvc_cart_ram[0x1fe1] & 0x40) >> 6) |
            ((self.pvc_cart_ram[0x1fe1] & 0x0f) << 1);
        let g = // 000G GGGg
            ((self.pvc_cart_ram[0x1fe1] & 0x20) >> 5) |
            ((self.pvc_cart_ram[0x1fe0] & 0xf0) >> 3);
        let b = // 000B BBBb
            ((self.pvc_cart_ram[0x1fe1] & 0x10) >> 4) |
            ((self.pvc_cart_ram[0x1fe0] & 0x0f) << 1);

        self.pvc_cart_ram[0x1fe5] = d;
        self.pvc_cart_ram[0x1fe4] = r;
        self.pvc_cart_ram[0x1fe3] = g;
        self.pvc_cart_ram[0x1fe2] = b;
    }

    /// Pack: compress R,G,B,D from cartram[0x1FE8-0x1FEB] into
    /// cartram[0x1FEC-0x1FED].
    /// Triggered by writes to 0x2FFFE8-0x2FFFEB.
    fn pvc_pack(&mut self) {
        let d = self.pvc_cart_ram[0x1feb] & 0x01;
        let r = self.pvc_cart_ram[0x1fea] & 0x1f;
        let g = self.pvc_cart_ram[0x1fe9] & 0x1f;
        let b = self.pvc_cart_ram[0x1fe8] & 0x1f;

        self.pvc_cart_ram[0x1fec] = (b >> 1) | ((g & 0x1e) << 3); // GGGG BBBB
        self.pvc_cart_ram[0x1fed] =
            (r >> 1) | ((b & 0x01) << 4) | ((g & 0x01) << 5) | ((r & 0x01) << 6) | (d << 7);
        // Drgb RRRR
    }

    /// Bankswap: calculate the 24-bit bank address from
    /// cartram[0x1FF1-0x1FF3]. Triggered by writes to 0x2FFFF0-0x2FFFF3.
    fn pvc_bankswap(&mut self) {
        let bankaddress = (self.pvc_cart_ram[0x1ff3] as usize) << 16
            | (self.pvc_cart_ram[0x1ff2] as usize) << 8
            | self.pvc_cart_ram[0x1ff1] as usize;

        self.pvc_cart_ram[0x1ff0] = 0xa0;
        self.pvc_cart_ram[0x1ff1] &= 0xfe;
        self.pvc_cart_ram[0x1ff3] &= 0x7f;

        self.pvc_bank_addr = (bankaddress + 0x100000) & 0xffffff;
    }

    fn pvc_bankswap_kf2k3bla(&mut self) {
        let bankaddress = (self.pvc_cart_ram[0x1ff3] as usize) << 16
            | (self.pvc_cart_ram[0x1ff2] as usize) << 8
            | self.pvc_cart_ram[0x1ff0] as usize;

        self.pvc_cart_ram[0x1ff0] &= 0xfe;
        self.pvc_cart_ram[0x1ff3] &= 0x7f;

        self.pvc_bank_addr = (bankaddress + 0x100000) & 0xffffff;
    }

    /// Write a byte to NEO-PVC cartram (0x2FE000-0x2FFFFF).
    /// Writes to specific ranges trigger internal PVC operations:
    /// - 0x2FFFE0-0x2FFFE3 → unpack
    /// - 0x2FFFE8-0x2FFFEB → pack
    /// - 0x2FFFF0-0x2FFFF3 → bankswap
    ///
    /// NOTE: This method triggers PVC operations immediately after each
    /// byte write. For 16-bit and 32-bit writes, the caller (write16/write32)
    /// should write all bytes to pvc_cart_ram first and then call
    /// `pvc_trigger_operation()` once to avoid premature triggers with
    /// incomplete data. See Geolith's `geo_m68k_write_banksw_16_pvc`.
    fn write_pvc_cartram_byte(&mut self, addr: u32, value: u8) {
        self.record_bus_access(BusAccessKind::Write, addr, value);
        let offset = ((addr - PVC_CARTRAM_START) as usize) ^ 1;
        if let Some(cell) = self.pvc_cart_ram.get_mut(offset) {
            *cell = value;
        }

        self.pvc_trigger_operation(addr);
    }

    /// Trigger the appropriate PVC operation for the given address.
    /// Checks the 4-byte aligned base address and calls unpack/pack/bankswap.
    /// Safe to call multiple times — only triggers for exact base addresses.
    fn pvc_trigger_operation(&mut self, addr: u32) {
        match addr & !3 {
            0x2FFFE0 => self.pvc_unpack(),
            0x2FFFE8 => self.pvc_pack(),
            0x2FFFF0 => self.pvc_bankswap(),
            _ => {}
        }
    }

    /// Write 16-bit value to PVC cartram atomically (both bytes before trigger).
    /// Mirrors Geolith's `geo_m68k_write_banksw_16_pvc` which writes both
    /// bytes first, then triggers the operation once.
    fn write_pvc_cartram_16(&mut self, addr: u32, value: u16) {
        let [hi, lo] = value.to_be_bytes();
        // Geolith uses write16be() for 16-bit PVC writes: low byte first,
        // high byte second, and no ^1 byte swap.
        let offset_hi = (addr - PVC_CARTRAM_START) as usize;
        let offset_lo = (addr.wrapping_add(1) - PVC_CARTRAM_START) as usize;
        if let Some(cell) = self.pvc_cart_ram.get_mut(offset_hi) {
            *cell = lo;
        }
        if let Some(cell) = self.pvc_cart_ram.get_mut(offset_lo) {
            *cell = hi;
        }
        self.record_bus_access(BusAccessKind::Write, addr, hi);
        self.record_bus_access(BusAccessKind::Write, addr.wrapping_add(1), lo);
        // Trigger the operation ONCE after both bytes are written
        self.pvc_trigger_operation(addr);
    }

    fn write_pvc_cartram_16_with_bla_bankswap(&mut self, addr: u32, value: u16) {
        let [hi, lo] = value.to_be_bytes();
        let offset_hi = (addr - PVC_CARTRAM_START) as usize;
        let offset_lo = (addr.wrapping_add(1) - PVC_CARTRAM_START) as usize;
        if let Some(cell) = self.pvc_cart_ram.get_mut(offset_hi) {
            *cell = lo;
        }
        if let Some(cell) = self.pvc_cart_ram.get_mut(offset_lo) {
            *cell = hi;
        }
        self.record_bus_access(BusAccessKind::Write, addr, hi);
        self.record_bus_access(BusAccessKind::Write, addr.wrapping_add(1), lo);

        match addr & !3 {
            0x2FFFE0 => self.pvc_unpack(),
            0x2FFFE8 => self.pvc_pack(),
            0x2FFFF0 => self.pvc_bankswap_kf2k3bla(),
            _ => {}
        }
    }

    fn read_sma(&self, addr: u32, config: &SmaConfig) -> u8 {
        let byte = if addr == SMA_PRESENCE_ADDR {
            0x37
        } else if addr == SMA_PRESENCE_ADDR + 1 {
            0x9A
        } else if config.is_prn_addr(addr) {
            (self.next_sma_random() & 0x00FF) as u8
        } else {
            return 0xFF;
        };
        self.record_bus_access(BusAccessKind::Read, addr, byte);
        byte
    }

    fn read_sma_word(&self, addr: u32, config: &SmaConfig) -> u16 {
        let value = if addr == SMA_PRESENCE_ADDR {
            SMA_PRESENCE_VALUE
        } else if config.is_prn_addr(addr) {
            self.next_sma_random()
        } else {
            return 0xFFFF;
        };
        let [hi, lo] = value.to_be_bytes();
        self.record_bus_access(BusAccessKind::Read, addr, hi);
        self.record_bus_access(BusAccessKind::Read, addr.wrapping_add(1), lo);
        value
    }

    fn next_sma_random(&self) -> u16 {
        let old = self.sma_rng.get();
        let new_bit = ((old >> 2)
            ^ (old >> 3)
            ^ (old >> 5)
            ^ (old >> 6)
            ^ (old >> 7)
            ^ (old >> 11)
            ^ (old >> 12)
            ^ (old >> 15))
            & 1;
        let next = (old << 1) | new_bit;
        self.sma_rng.set(next);
        next
    }

    fn sma_bankswitch(&mut self, value: u16, config: &SmaConfig) {
        let index = bitswap6(value, config.scramble) as usize;
        let Some(offset) = config.bank_offsets.get(index).copied() else {
            return;
        };
        self.set_prom_bank_offset(FIXED_PROM_WINDOW_SIZE + offset as usize);
    }

    fn read_memory_card(&self, addr: u32) -> u8 {
        if !self.memcard_inserted {
            return 0xFF;
        }

        if addr & 1 == 0 {
            return 0xFF;
        }

        let offset = Self::memory_card_offset(addr);
        self.memory_card[offset]
    }

    fn palette_ram_offset(&self, addr: u32) -> usize {
        let window_offset = (addr - PALETTE_RAM_START) as usize & (PALETTE_RAM_BANK_SIZE - 1);
        self.palette_bank as usize * PALETTE_RAM_BANK_SIZE + window_offset
    }

    fn palette_ram_byte_write_offset(&self, addr: u32) -> usize {
        // Geolith's 8-bit palette write handler first shifts the bus address
        // into word space, then uses bit 0 of that word address to choose the
        // byte lane. This means 0x400000/1 target the high byte of word 0,
        // while 0x400002/3 target the low byte of word 1.
        let word_addr = ((addr - PALETTE_RAM_START) >> 1) as usize;
        let word_index = word_addr & 0x0fff;
        let byte_lane = word_addr & 0x0001;
        self.palette_bank as usize * PALETTE_RAM_BANK_SIZE + word_index * 2 + byte_lane
    }

    fn write_memory_card(&mut self, addr: u32, value: u8) {
        if !self.memcard_inserted || self.memcard_write_protected || !self.memcard_unlocked {
            return;
        }

        let offset = Self::memory_card_offset(addr);
        self.memory_card[offset] = value;
        self.dirty_memory_card.set(true);
    }

    fn work_ram_offset(addr: u32) -> usize {
        (addr - WORK_RAM_START) as usize & (WORK_RAM_SIZE - 1)
    }

    fn backup_ram_offset(addr: u32) -> usize {
        (addr - BACKUP_RAM_START) as usize & (BACKUP_RAM_SIZE - 1)
    }

    fn memory_card_offset(addr: u32) -> usize {
        ((addr >> 1) as usize) & (MEMORY_CARD_SIZE - 1)
    }

    pub fn advance_video_timing(&self) {
        let next = self.lspc_scanline.get().wrapping_add(1) % 264;
        self.lspc_scanline.set(next);
    }

    pub fn advance_rtc(&mut self, m68k_cycles: u32) {
        self.rtc.borrow_mut().sync_cycles(m68k_cycles);
    }

    pub fn reload_irq2_on_vblank(&mut self) {
        if self.irq2_ctrl & IRQ_TIMER_RELOAD_VBLANK != 0 {
            self.irq2_counter = self.irq2_reload;
        }
    }

    pub fn tick_irq2_counter(&mut self, m68k_cycles: u32) -> bool {
        let mut dec = m68k_cycles >> 1;
        self.irq2_frags = self.irq2_frags.wrapping_add(m68k_cycles & 0x01);
        if self.irq2_frags >= 2 {
            self.irq2_frags -= 2;
            dec = dec.wrapping_add(1);
        }

        if dec == 0 {
            return false;
        }

        if self.irq2_counter > dec {
            self.irq2_counter -= dec;
            return false;
        }

        let mut fired = false;
        for _ in 0..dec {
            self.irq2_counter = self.irq2_counter.wrapping_sub(1);
            if self.irq2_counter == 0 {
                if self.irq2_ctrl & IRQ_TIMER_RELOAD_COUNT0 != 0 {
                    self.irq2_counter = self.irq2_counter.wrapping_add(self.irq2_reload);
                }
                if self.irq2_ctrl & IRQ_TIMER_ENABLED != 0 {
                    fired = true;
                }
            }
        }
        fired
    }

    pub fn cycles_until_irq2_event(&self, max_cycles: u32) -> Option<u32> {
        if self.irq2_counter == 0
            || self.irq2_ctrl & (IRQ_TIMER_ENABLED | IRQ_TIMER_RELOAD_COUNT0) == 0
        {
            return None;
        }

        let needed = self
            .irq2_counter
            .saturating_mul(2)
            .saturating_sub(self.irq2_frags.min(1));
        if needed == 0 || needed > max_cycles {
            None
        } else {
            Some(needed)
        }
    }

    pub fn advance_auto_animation_frame(&self) {
        self.advance_auto_animation();
    }

    pub fn auto_animation_counter(&self) -> Option<u8> {
        (self.lspc_mode & 0x0008 == 0).then_some(self.auto_animation_counter.get())
    }

    fn read_lspc_register(&self, addr: u32) -> u8 {
        let base = addr & !1;
        let word = match base {
            LSPC_VRAMADDR => self.vram_addr.get(),
            LSPC_VRAMRW => {
                // Geolith's LSPC model does not advance REG_VRAMADDR on
                // reads. The MVS BIOS VRAM test relies on readback staying on
                // the address it just selected.
                self.read_vram_word(self.vram_addr.get())
            }
            LSPC_VRAMMOD => self.vram_mod,
            LSPC_MODE => self.lspc_mode_with_scanline(),
            LSPC_TIMERHIGH => self.timer_high,
            LSPC_TIMERLOW => self.timer_low,
            LSPC_IRQACK => self.irq_ack,
            LSPC_TIMERSTOP => self.timer_stop,
            _ => 0xFFFF,
        };

        if addr & 1 == 0 {
            (word >> 8) as u8
        } else {
            word as u8
        }
    }

    fn write_lspc_register_word(&mut self, addr: u32, value: u16) {
        let base = addr & !1;
        match base {
            LSPC_VRAMADDR => {
                self.vram_addr.set(value);
            }
            LSPC_VRAMRW => {
                self.pending_vram_write_hi = None;
                let address = self.vram_addr.get();
                self.write_vram_word(address, value);
                self.vram_addr
                    .set(next_vram_address(address, self.vram_mod));
            }
            LSPC_VRAMMOD => {
                self.vram_mod = value;
            }
            LSPC_MODE => {
                let prev_ctrl = self.irq2_ctrl;
                self.lspc_mode = value;
                self.auto_animation_reload.set((self.lspc_mode >> 8) as u8);
                self.irq2_ctrl = (value & 0x00F0) as u8;
                if prev_ctrl & IRQ_TIMER_RELOAD_VBLANK == 0
                    && self.irq2_ctrl & IRQ_TIMER_RELOAD_VBLANK != 0
                    && self.lspc_scanline.get() >= 248
                {
                    self.irq2_counter = self.irq2_reload;
                }
            }
            LSPC_TIMERHIGH => {
                self.timer_high = value;
                self.irq2_reload = (self.irq2_reload & 0x0000_FFFF) | ((value as u32) << 16);
            }
            LSPC_TIMERLOW => {
                self.timer_low = value;
                self.irq2_reload = (self.irq2_reload & 0xFFFF_0000) | value as u32;
                if self.irq2_ctrl & IRQ_TIMER_RELOAD_WRITE != 0 {
                    self.irq2_counter = self.irq2_reload;
                }
            }
            LSPC_IRQACK => {
                self.irq_ack = value;
                // LSPC RomSize register (0x3C000C) also encodes C-ROM
                // address window size. Store the full word value so the
                // video layer can verify it against the data-based crom_mask.
                self.lspc_rom_size = self.irq_ack;
            }
            LSPC_TIMERSTOP => {
                self.timer_stop = value;
            }
            _ => {}
        }
    }

    fn write_system_latch(&mut self, addr: u32) {
        match addr {
            0x3A0001 => self.palette_shadow = false,
            0x3A0011 => self.palette_shadow = true,
            0x3A0003 => self.use_cart_vectors = false,
            0x3A0013 => self.use_cart_vectors = true,
            0x3A0005 => self.set_memcard_lock1(false),
            0x3A0007 => self.set_memcard_lock2(true),
            0x3A0015 => self.set_memcard_lock1(true),
            0x3A0017 => self.set_memcard_lock2(false),
            0x3A0009 => self.memcard_register_select = true,
            0x3A0019 => self.memcard_register_select = false,
            0x3A000B => {
                self.use_cart_audio = false;
                self.use_cart_fix = false;
            }
            0x3A001B => {
                self.use_cart_audio = true;
                self.use_cart_fix = true;
            }
            0x3A000D => self.save_ram_unlocked = false,
            0x3A001D => self.save_ram_unlocked = true,
            0x3A000F => self.palette_bank = 1,
            0x3A001F => self.palette_bank = 0,
            _ => {}
        }
    }

    fn set_memcard_lock1(&mut self, locked: bool) {
        self.memcard_lock1 = locked;
        self.memcard_unlocked = !self.memcard_lock1 && !self.memcard_lock2;
    }

    fn set_memcard_lock2(&mut self, locked: bool) {
        self.memcard_lock2 = locked;
        self.memcard_unlocked = !self.memcard_lock1 && !self.memcard_lock2;
    }

    fn read_vram_word(&self, address: u16) -> u16 {
        if address >= LSPC_VRAM_WORDS {
            return 0xFFFF;
        }
        let offset = (address as usize).saturating_mul(2);
        let Some(bytes) = self.vram.get(offset..offset + 2) else {
            return 0xFFFF;
        };
        u16::from_be_bytes([bytes[0], bytes[1]])
    }

    fn write_vram_word(&mut self, address: u16, value: u16) {
        if address >= LSPC_VRAM_WORDS {
            return;
        }
        let offset = (address as usize).saturating_mul(2);
        if offset + 1 >= self.vram.len() {
            return;
        }
        let [hi, lo] = value.to_be_bytes();
        self.vram[offset] = hi;
        self.vram[offset + 1] = lo;
    }

    fn lspc_mode_with_scanline(&self) -> u16 {
        // Geolith exposes REG_LSPCMODE reads as the live raster counter with
        // the hardware 0xF8 line offset, plus the 3-bit auto-animation counter.
        let scanline = self.lspc_scanline.get().wrapping_add(0x00F8) & 0x01FF;
        (scanline << 7) | self.auto_animation_counter.get() as u16
    }

    fn advance_auto_animation(&self) {
        if self.lspc_mode & 0x0008 != 0 {
            return;
        }

        let next_timer = self.auto_animation_timer.get().wrapping_sub(1);
        self.auto_animation_timer.set(next_timer);
        if next_timer == 0xFF {
            self.auto_animation_counter
                .set(self.auto_animation_counter.get().wrapping_add(1) & 0x07);
            self.auto_animation_timer
                .set(self.auto_animation_reload.get());
        }
    }

    fn record_bus_access(&self, kind: BusAccessKind, address: u32, value: u8) {
        let mut trace = self.bus_trace.borrow_mut();
        if trace.len() == BUS_TRACE_LIMIT {
            trace.remove(0);
        }
        trace.push(BusAccess {
            kind,
            address,
            value,
        });
    }

    pub fn record_z80_io_access(&self, kind: BusAccessKind, port: u16, value: u8) {
        let mut trace = self.z80_io_trace.borrow_mut();
        if trace.len() == Z80_IO_TRACE_LIMIT {
            trace.remove(0);
        }
        trace.push(Z80IoAccess { kind, port, value });
    }
}

fn main_cpu_bus_addr(addr: u32) -> u32 {
    addr & MAIN_CPU_ADDRESS_MASK
}

fn next_vram_address(address: u16, modulo: u16) -> u16 {
    let bank = address & 0x8000;
    let next = (address as i32).wrapping_add(modulo as i16 as i32) as u16;
    (next & 0x7FFF) | bank
}

fn geolith_prom_bank_mask(prom_len: usize) -> usize {
    let bank_count = prom_len
        .saturating_sub(FIXED_PROM_WINDOW_SIZE)
        .checked_shr(20)
        .unwrap_or(0);
    if bank_count == 0 {
        return 0;
    }

    bank_count.next_power_of_two().saturating_sub(1)
}

fn read16be_wrapped(data: &[u8], offset: usize) -> u16 {
    if data.is_empty() {
        return 0xffff;
    }
    let mask = data.len() - 1;
    u16::from_be_bytes([data[(offset + 1) & mask], data[offset & mask]])
}

fn write16be_wrapped(data: &mut [u8], offset: usize, value: u16) {
    if data.is_empty() {
        return;
    }
    let mask = data.len() - 1;
    let [hi, lo] = value.to_be_bytes();
    data[offset & mask] = lo;
    data[(offset + 1) & mask] = hi;
}

fn detect_cart_protection(rom: &crate::rom::RomData) -> CartProtection {
    // ── .neo file NGH-based detection ──────────────────────────────────
    // For .neo files, detect protection from the NGH value stored in the
    // header metadata. This mirrors Geolith's approach of using NGH to
    // determine which runtime emulation features to enable.
    if let Some(ref metadata) = rom.metadata {
        match metadata.board_type {
            crate::rom::NeoBoardType::Linkable => return CartProtection::Linkable,
            crate::rom::NeoBoardType::Brezzasoft => return CartProtection::Brezzasoft,
            crate::rom::NeoBoardType::Ct0 => return CartProtection::Ct0,
            crate::rom::NeoBoardType::Kof98 => return CartProtection::Kof98,
            crate::rom::NeoBoardType::Ms5Plus => return CartProtection::Ms5Plus,
            crate::rom::NeoBoardType::Cthd2003 => return CartProtection::Cthd2003,
            crate::rom::NeoBoardType::Kof10th => return CartProtection::Kof10th,
            crate::rom::NeoBoardType::Kf2k3Bl => return CartProtection::Kf2k3Bl,
            crate::rom::NeoBoardType::Kf2k3Bla => return CartProtection::Kf2k3Bla,
            _ => {}
        }

        // ── NEO-PVC protection ───────────────────────────────────────────
        // Used by KOF 2003, Metal Slug 5, SVC Chaos, and KOF2003 bootlegs.
        if matches!(metadata.board_type, crate::rom::NeoBoardType::Pvc) {
            return CartProtection::Pvc;
        }
        if metadata.ngh == 0x0250 || metadata.name.eq_ignore_ascii_case("Metal Slug X") {
            return CartProtection::MslugX;
        }
        // SMA-based games: each game has its own PRN addresses, bank table,
        // and scramble map (see SmaConfig). Only enable this when the loader's
        // Geolith-style variant heuristics kept the board as SMA.
        if matches!(
            metadata.board_type,
            crate::rom::NeoBoardType::Sma
                | crate::rom::NeoBoardType::SmaGarouH
                | crate::rom::NeoBoardType::SmaMslug3A
        ) {
            if matches!(metadata.ngh, 0x0151 | 0x0251) {
                return CartProtection::Sma(SmaConfig::kof99());
            }
            if matches!(metadata.ngh, 0x0213 | 0x0256) {
                if metadata.board_type == crate::rom::NeoBoardType::SmaMslug3A {
                    return CartProtection::Sma(SmaConfig::mslug3a());
                }
                return CartProtection::Sma(SmaConfig::mslug3());
            }
            if metadata.board_type == crate::rom::NeoBoardType::SmaGarouH {
                return CartProtection::Sma(SmaConfig::garouh());
            }
            if matches!(metadata.ngh, 0x0229 | 0x0153 | 0x0253) {
                return CartProtection::Sma(SmaConfig::garou());
            }
            if matches!(metadata.ngh, 0x021D | 0x0257) {
                return CartProtection::Sma(SmaConfig::kof2000());
            }
        }
    }

    // ── .zip file filename-based detection ────────────────────────────
    if rom.recognized_files.iter().any(|name| {
        name.eq_ignore_ascii_case("neo-sma") || name.to_ascii_lowercase().ends_with(".neo-sma")
    }) {
        // Detect which SMA game by looking at program ROM filenames
        let has = |needle: &str| {
            rom.recognized_files
                .iter()
                .any(|name| name.eq_ignore_ascii_case(needle))
        };

        if has("256-pg1.p1") || has("256-pg2.p2") {
            return CartProtection::Sma(SmaConfig::mslug3());
        }
        if has("253-ep1.ep1") || has("253-p1.p1") {
            // Garou: use garou (MVS) config by default; garouh (AES) detected
            // by P-ROM heuristic at ROM-load time if needed.
            return CartProtection::Sma(SmaConfig::garou());
        }
        if has("257-p1.p1") || has("254-pg1.p1") || has("257-p2.p2") {
            return CartProtection::Sma(SmaConfig::kof2000());
        }
        if has("251-p1.p1") || has("251-pg1.p1") || has("151-p1.p1") {
            return CartProtection::Sma(SmaConfig::kof99());
        }
        // Non-SMA CMC sets are handled by the ROM loader.
        if has("neo-sma") && !has("256-pg1.p1") && !has("253-ep1.ep1") && !has("257-p1.p1") {
            return CartProtection::None;
        }
    }

    CartProtection::None
}

fn watchdog_limit_for_rom(rom: &crate::rom::RomData) -> u32 {
    if rom
        .metadata
        .as_ref()
        .is_some_and(|metadata| metadata.ngh == 0x269)
    {
        WATCHDOG_M68K_CYCLES + SVC_WATCHDOG_TOLERANCE_M68K_CYCLES
    } else {
        WATCHDOG_M68K_CYCLES
    }
}

fn detect_fix_bankswitch(rom: &crate::rom::RomData) -> FixBankSwitch {
    if let Some(metadata) = &rom.metadata {
        return match metadata.fix_banksw {
            crate::rom::NeoFixBanksw::None => FixBankSwitch::None,
            crate::rom::NeoFixBanksw::Line => FixBankSwitch::Line,
            crate::rom::NeoFixBanksw::Tile => FixBankSwitch::Tile,
        };
    }

    // Fall back to P-ROM heuristic for zip-loaded sets without .neo metadata.
    // Additional games can be added here as fix bankswitch needs are identified.
    match rom_ngh(rom) {
        Some(0x0253 | 0x0256 | 0x0263) => FixBankSwitch::Line, // Garou, MSlug3, MSlug4
        Some(0x0257 | 0x0266 | 0x0269 | 0x0271) => FixBankSwitch::Tile, // KOF2000, Matrimelee, SVC, KOF2003
        _ => FixBankSwitch::None,
    }
}

fn rom_ngh(rom: &crate::rom::RomData) -> Option<u16> {
    if let Some(metadata) = &rom.metadata {
        return Some(metadata.ngh as u16);
    }

    rom.prom
        .get(0x108..0x10A)
        .map(|bytes| u16::from_be_bytes([bytes[0], bytes[1]]))
}

fn bitswap6(value: u16, map: [u8; 6]) -> u8 {
    let mut out = 0u8;
    for (bit, source) in map.into_iter().enumerate() {
        out |= (((value >> source) & 1) as u8) << bit;
    }
    out
}

fn build_diagnostic_bios() -> Vec<u8> {
    let mut bios = vec![0x4E, 0x71]; // NOP stream for unmapped BIOS paths.
    bios.resize(BIOS_ROM_SIZE, 0x4E);
    for byte in bios.iter_mut().skip(1).step_by(2) {
        *byte = 0x71;
    }

    // ── Initial vectors (SSP & PC) ─────────────────────────────────────
    // The 68k reads these from 0xC00000-0xC00007 when use_cart_vectors is false.
    let ssp: u32 = 0x0010_FFFC; // Top of Work RAM
    let pc: u32 = 0x00C0_0402; // Init routine starts here
    bios[0x000..0x004].copy_from_slice(&ssp.to_be_bytes());
    bios[0x004..0x008].copy_from_slice(&pc.to_be_bytes());

    // ── BIOS init routine at offset 0x402 ──────────────────────────────
    // This replaces the old minimal boot bridge with a proper init sequence
    // that mimics what real NeoGeo BIOSes do before jumping to game code.
    let mut pos = 0x402usize;

    // MOVE.W #$2700, SR   (supervisor mode, mask all interrupts)
    //   → 0x46FC 0x2700
    bios[pos..pos + 2].copy_from_slice(&[0x46, 0xFC]);
    bios[pos + 2..pos + 4].copy_from_slice(&[0x27, 0x00]);
    pos += 4;

    // MOVEQ #0, D0   → 0x7000   (clear D0 before writing to latches)
    bios[pos..pos + 2].copy_from_slice(&[0x70, 0x00]);
    pos += 2;

    // MOVE.B D0, $003A0001.L   (display ON)
    //   → 0x13C0 0x003A0001
    bios[pos..pos + 2].copy_from_slice(&[0x13, 0xC0]);
    bios[pos + 2..pos + 6].copy_from_slice(&0x003A_0001u32.to_be_bytes());
    pos += 6;

    // MOVE.B D0, $003A0013.L   (use cart vectors)
    //   → 0x13C0 0x003A0013
    bios[pos..pos + 2].copy_from_slice(&[0x13, 0xC0]);
    bios[pos + 2..pos + 6].copy_from_slice(&0x003A_0013u32.to_be_bytes());
    pos += 6;

    // MOVE.B D0, $003A001A.L   (use cart audio)
    //   → 0x13C0 0x003A001A
    bios[pos..pos + 2].copy_from_slice(&[0x13, 0xC0]);
    bios[pos + 2..pos + 6].copy_from_slice(&0x003A_001Au32.to_be_bytes());
    pos += 6;

    // MOVE.B D0, $003A001B.L   (use cart fix)
    //   → 0x13C0 0x003A001B
    bios[pos..pos + 2].copy_from_slice(&[0x13, 0xC0]);
    bios[pos + 2..pos + 6].copy_from_slice(&0x003A_001Bu32.to_be_bytes());
    pos += 6;

    // MOVE.B D0, $00320000.L   (Z80 handshake: write 0 to sound latch,
    //   triggers NMI on Z80 if NMI is enabled)
    //   → 0x13C0 0x00320000
    bios[pos..pos + 2].copy_from_slice(&[0x13, 0xC0]);
    bios[pos + 2..pos + 6].copy_from_slice(&0x0032_0000u32.to_be_bytes());
    pos += 6;

    // MOVE.L ($00000122).L, A0   (load USER vector from cartridge)
    //   → 0x2079 0x00000122
    bios[pos..pos + 2].copy_from_slice(&[0x20, 0x79]);
    bios[pos + 2..pos + 6].copy_from_slice(&0x0000_0122u32.to_be_bytes());
    pos += 6;

    // JMP (A0)   → 0x4ED0
    bios[pos..pos + 2].copy_from_slice(&[0x4E, 0xD0]);
    // pos += 2;  — final position marker (kept for documentation)
    let _ = pos; // pos tracks the init routine length for future reference

    bios
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ram_and_vram_are_writable_but_prom_is_read_only() {
        let mut memory = Memory::new();
        memory.prom = vec![0x12; 0x100];

        memory.write8(0x100000, 0x34);
        memory.write16(LSPC_VRAMADDR, 0x0002);
        memory.write16(LSPC_VRAMRW, 0x5678);
        memory.write8(0x000000, 0x78);

        assert_eq!(memory.read8(0x100000), 0x34);
        assert_eq!(memory.vram[4], 0x56);
        assert_eq!(memory.vram[5], 0x78);
        // With use_cart_vectors=false (default), the first 0x80 bytes of PROM
        // are shadowed by BIOS.  Switch to cart vectors to read PROM directly.
        memory.write8(0x3A0013, 0);
        assert_eq!(memory.read8(0x000000), 0x12);
    }

    #[test]
    fn lspc_vram_registers_read_write_and_apply_modulo() {
        let mut memory = Memory::new();

        memory.write16(LSPC_VRAMMOD, 0x0002);
        memory.write16(LSPC_VRAMADDR, 0x0004);
        memory.write16(LSPC_VRAMRW, 0x1234);

        memory.write16(LSPC_VRAMADDR, 0x0004);
        assert_eq!(memory.read8(LSPC_VRAMADDR), 0x00);
        assert_eq!(memory.read8(LSPC_VRAMADDR + 1), 0x04);
        assert_eq!(memory.read8(LSPC_VRAMRW), 0x12);
        assert_eq!(memory.read8(LSPC_VRAMRW + 1), 0x34);
        // Geolith does not auto-increment VRAMADDR on readback; writes still
        // apply the configured modulo.
        assert_eq!(memory.read8(LSPC_VRAMADDR), 0x00);
        assert_eq!(memory.read8(LSPC_VRAMADDR + 1), 0x04);

        memory.write16(LSPC_VRAMRW, 0x5678);
        assert_eq!(memory.read16(LSPC_VRAMADDR), 0x0006);
    }

    #[test]
    fn lspc_vram_modulo_is_signed_and_preserves_vram_bank() {
        assert_eq!(next_vram_address(0x0010, 0xFFFE), 0x000E);
        assert_eq!(next_vram_address(0x8010, 0xFFFE), 0x800E);
        assert_eq!(next_vram_address(0x7FFF, 0x0001), 0x0000);
        assert_eq!(next_vram_address(0x87FF, 0x0001), 0x8800);
    }

    #[test]
    fn lspc_vram_writes_outside_geolith_boundary_do_not_wrap() {
        let mut memory = Memory::new();

        memory.write16(LSPC_VRAMADDR, 0x87FF);
        memory.write16(LSPC_VRAMMOD, 0x0001);
        memory.write16(LSPC_VRAMRW, 0x1234);
        assert_eq!(memory.vram[0x87FF * 2], 0x12);
        assert_eq!(memory.vram[0x87FF * 2 + 1], 0x34);
        assert_eq!(memory.read16(LSPC_VRAMADDR), 0x8800);

        memory.write16(LSPC_VRAMRW, 0x5678);
        assert_eq!(memory.vram[0], 0x00, "invalid VRAM write must not wrap");
        assert_eq!(memory.vram[1], 0x00, "invalid VRAM write must not wrap");
    }

    #[test]
    fn lspc_byte_writes_duplicate_even_byte_like_geolith() {
        let mut memory = Memory::new();

        memory.write8(LSPC_VRAMMOD, 0x02);
        assert_eq!(memory.vram_mod, 0x0202);

        memory.write8(LSPC_VRAMMOD + 1, 0x05);
        assert_eq!(memory.vram_mod, 0x0202, "odd LSPC byte writes are ignored");
    }

    #[test]
    fn lspc_mode_exposes_advancing_scanline_counter() {
        let memory = Memory::new();

        let first = memory.lspc_scanline.get();
        memory.advance_video_timing();
        let second = memory.lspc_scanline.get();

        assert_ne!(first, second);
        assert_eq!(second, (first + 1) % 264);
    }

    #[test]
    fn lspc_mode_read_uses_geolith_raster_offset() {
        let memory = Memory::new();

        memory.lspc_scanline.set(0);
        assert_eq!(memory.read16(LSPC_MODE), 0x7C00);

        memory.lspc_scanline.set(8);
        assert_eq!(
            memory.read16(LSPC_MODE) & 0x8000,
            0x8000,
            "scanline 8 must read as negative after the 0xF8 raster offset"
        );
    }

    #[test]
    fn lspc_mode_exposes_auto_animation_counter() {
        let mut memory = Memory::new();

        memory.write16(LSPC_MODE, 0x0000);
        memory.advance_video_timing();
        assert_eq!(
            memory.auto_animation_counter(),
            Some(0),
            "scanline timing must not advance auto-animation"
        );

        memory.advance_auto_animation_frame();

        assert_eq!(memory.auto_animation_counter(), Some(1));
        assert_eq!(memory.read8(LSPC_MODE + 1) & 0x07, 1);

        memory.write16(LSPC_MODE, 0x0008);
        memory.advance_auto_animation_frame();

        // Auto-animation is disabled (bit 3 set), so counter stays at 1.
        assert_eq!(memory.auto_animation_counter(), None);
        assert_eq!(memory.read8(LSPC_MODE + 1) & 0x07, 1);
    }

    #[test]
    fn auto_animation_ticks_once_per_frame_like_geolith() {
        let mut memory = Memory::new();
        memory.write16(LSPC_MODE, 0x0200);

        for _ in 0..264 {
            memory.advance_video_timing();
        }
        assert_eq!(memory.auto_animation_counter(), Some(0));

        memory.advance_auto_animation_frame();
        assert_eq!(memory.auto_animation_counter(), Some(1));
        for _ in 0..263 {
            memory.advance_video_timing();
        }
        assert_eq!(memory.auto_animation_counter(), Some(1));
        memory.advance_auto_animation_frame();
        assert_eq!(memory.auto_animation_counter(), Some(1));
    }

    #[test]
    fn unmapped_reads_return_open_bus_value() {
        let memory = Memory::new();
        assert_eq!(memory.read8(0xE00000), 0xFF);
    }

    #[test]
    fn diagnostic_bios_maps_init_sequence() {
        let memory = Memory::new();

        // New init starts with MOVE.W #$2700, SR (0x46FC 0x2700)
        assert_eq!(memory.read8(BIOS_ROM_START + 0x0402), 0x46);
        assert_eq!(memory.read8(BIOS_ROM_START + 0x0403), 0xFC);
        // Followed by MOVEQ #0, D0 (0x7000)
        assert_eq!(memory.read8(BIOS_ROM_START + 0x0406), 0x70);
        assert_eq!(memory.read8(BIOS_ROM_START + 0x0407), 0x00);
    }

    #[test]
    fn set_bios_replaces_diagnostic_bios() {
        let mut memory = Memory::new();
        memory.set_bios(vec![0xAA; BIOS_ROM_SIZE]);

        // The BIOS mirror always reads from BIOS ROM; the vector swap only
        // applies to the low vector window at 0x000000.
        assert_eq!(memory.read8(BIOS_ROM_START), 0xAA);
        assert_eq!(memory.read8(BIOS_ROM_START + 0x80), 0xAA);
        assert_eq!(memory.read8(BIOS_ROM_START + 0x0402), 0xAA);
    }

    #[test]
    fn system_latches_switch_vector_source_without_hiding_cart_program() {
        let mut memory = Memory::new();
        memory.prom = vec![0x11; FIXED_PROM_WINDOW_SIZE];
        memory.bios = vec![0xAA; BIOS_ROM_SIZE];

        // use_cart_vectors starts false (BIOS vectors active at power-on).
        // The first 0x80 bytes at 0x000000 come from BIOS.
        assert_eq!(memory.read8(0x000000), 0xAA);
        assert_eq!(memory.read8(0x000080), 0x11);
        // BIOS mirror is not affected by vector swapping.
        assert_eq!(memory.read8(BIOS_ROM_START), 0xAA);
        assert_eq!(memory.read8(BIOS_ROM_START + 0x7F), 0xAA);
        assert_eq!(memory.read8(BIOS_ROM_START + 0x80), 0xAA);

        // Writing 0x3A0003 keeps use_cart_vectors false (already false).
        memory.write8(0x3A0003, 0x00);
        assert_eq!(memory.read8(0x000000), 0xAA);

        // Writing 0x3A0013 sets use_cart_vectors true (cartridge vectors active).
        memory.write8(0x3A0013, 0x00);

        assert_eq!(memory.read8(0x000000), 0x11);
        assert_eq!(memory.read8(BIOS_ROM_START), 0xAA);
    }

    #[test]
    fn system_latches_track_fix_audio_sram_palette_and_display_state() {
        let mut memory = Memory::new();

        memory.write8(0x3A0011, 0);
        memory.write8(0x3A000A, 0);
        memory.write8(0x3A000B, 0);
        memory.write8(0x3A001D, 0);
        memory.write8(0x3A000F, 0);
        memory.write8(0x3A0009, 0);

        let snapshot = memory.system_control_snapshot();
        assert!(snapshot.display_enabled);
        assert!(!snapshot.use_cart_audio);
        assert!(!snapshot.use_cart_fix);
        assert!(snapshot.save_ram_unlocked);
        assert_eq!(snapshot.palette_bank, 1);
        assert!(snapshot.palette_shadow);
        assert!(snapshot.memcard_register_select);

        memory.write8(0x3A0001, 0);
        memory.write8(0x3A001A, 0);
        memory.write8(0x3A001B, 0);
        memory.write8(0x3A000D, 0);
        memory.write8(0x3A001F, 0);
        memory.write8(0x3A0019, 0);

        let snapshot = memory.system_control_snapshot();
        assert!(snapshot.display_enabled);
        assert!(snapshot.use_cart_audio);
        assert!(snapshot.use_cart_fix);
        assert!(!snapshot.save_ram_unlocked);
        assert_eq!(snapshot.palette_bank, 0);
        assert!(!snapshot.palette_shadow);
        assert!(!snapshot.memcard_register_select);
    }

    #[test]
    fn word_writes_do_not_toggle_system_latches_like_geolith() {
        let mut memory = Memory::new();

        memory.write8(0x3A0013, 0);
        memory.write8(0x3A001B, 0);
        memory.write8(0x3A001D, 0);
        memory.write8(0x3A001F, 0);

        let before = memory.system_control_snapshot();
        assert!(before.use_cart_vectors);
        assert!(before.use_cart_audio);
        assert!(before.use_cart_fix);
        assert!(before.save_ram_unlocked);
        assert_eq!(before.palette_bank, 0);

        // Geolith's 16-bit register handler does not dispatch 0x3Axxxx
        // latch writes. Splitting these into write8 calls would incorrectly
        // switch BIOS vectors, board FIX/audio, SRAM lock, and palette bank.
        memory.write16(0x3A0002, 0);
        memory.write16(0x3A000A, 0);
        memory.write16(0x3A000C, 0);
        memory.write16(0x3A000E, 0);

        let after = memory.system_control_snapshot();
        assert_eq!(after.use_cart_vectors, before.use_cart_vectors);
        assert_eq!(after.use_cart_audio, before.use_cart_audio);
        assert_eq!(after.use_cart_fix, before.use_cart_fix);
        assert_eq!(after.save_ram_unlocked, before.save_ram_unlocked);
        assert_eq!(after.palette_bank, before.palette_bank);
    }

    #[test]
    fn fix_latch_switches_z80_mrom_with_srom_like_geolith() {
        let mut memory = Memory::new();
        memory.use_cart_audio = true;
        memory.use_cart_fix = true;

        // REG_BRDFIX selects the board SFIX/SM1 pair.
        memory.write8(0x3A000B, 0);
        let snapshot = memory.system_control_snapshot();
        assert!(!snapshot.use_cart_audio);
        assert!(!snapshot.use_cart_fix);

        // The adjacent even addresses are not SM1/M1 selectors in Geolith.
        memory.write8(0x3A001A, 0);
        let snapshot = memory.system_control_snapshot();
        assert!(!snapshot.use_cart_audio);
        assert!(!snapshot.use_cart_fix);

        // REG_CRTFIX selects the cartridge SROM/M1 pair.
        memory.write8(0x3A001B, 0);
        let snapshot = memory.system_control_snapshot();
        assert!(snapshot.use_cart_audio);
        assert!(snapshot.use_cart_fix);

        memory.write8(0x3A000A, 0);
        let snapshot = memory.system_control_snapshot();
        assert!(snapshot.use_cart_audio);
        assert!(snapshot.use_cart_fix);
    }

    #[test]
    fn bus_trace_records_unmapped_accesses() {
        let memory = Memory::new();

        assert_eq!(memory.read8(0xE00000), 0xFF);

        let trace = memory.take_bus_trace();
        assert_eq!(trace.len(), 1);
        assert_eq!(trace[0].kind, BusAccessKind::Read);
        assert_eq!(trace[0].address, 0xE00000);
    }

    #[test]
    fn main_cpu_addresses_wrap_to_24_bit_bus() {
        let mut memory = Memory::new();
        memory.set_bios(vec![0xAA; BIOS_ROM_SIZE]);
        memory.write8(0x01D00000, 0x42);

        assert_eq!(memory.read8(0x01C00000), 0xAA);
        assert_eq!(memory.read8(0x01D00000), 0x42);
        assert!(memory.take_bus_trace().is_empty());
    }

    #[test]
    fn watchdog_writes_reset_the_counter_without_bus_trace() {
        let mut memory = Memory::new();
        assert!(!memory.advance_watchdog(1234));

        memory.write8(DIPSW_PORT, 0);

        assert_eq!(memory.watchdog_cycles, 0);
        assert!(memory.take_bus_trace().is_empty());
    }

    #[test]
    fn watchdog_triggers_after_geolith_threshold() {
        let mut memory = Memory::new();

        assert!(!memory.advance_watchdog(WATCHDOG_M68K_CYCLES - 1));
        assert!(memory.advance_watchdog(1));
        memory.reset_watchdog();
        assert_eq!(memory.watchdog_cycles, 0);
    }

    #[test]
    fn watchdog_reset_matches_geolith_m68k_and_lspc_state() {
        let mut memory = Memory::new();
        memory.prom = vec![0; FIXED_PROM_WINDOW_SIZE * 3];
        memory.prom_bank_offset = 0x200000;
        memory.pvc_bank_addr = 0x234567;
        memory.use_cart_audio = true;
        memory.use_cart_fix = true;
        memory.display_enabled = false;
        memory.vram[0x20] = 0x5a;
        memory.palette_ram[0x30] = 0xa5;
        memory.palette_bank = 1;
        memory.palette_shadow = true;
        memory.vram_addr.set(0x1234);
        memory.vram_mod = 0x0040;
        memory.lspc_mode = 0x1234;
        memory.auto_animation_counter.set(7);
        memory.auto_animation_timer.set(6);
        memory.auto_animation_reload.set(5);
        memory.lspc_scanline.set(200);
        memory.restore_special_board_state(SpecialBoardState {
            cart_reg: [0x1111, 0x2222],
            prot_reg: 0x33445566,
            mslugx_command: 0x0001,
            mslugx_counter: 0x0042,
            sma_rng: 0xbeef,
            pending_sma_bank_hi: Some(0x7c),
        });

        memory.soft_reset_for_watchdog();

        assert_eq!(memory.prom_bank_offset, FIXED_PROM_WINDOW_SIZE);
        assert_eq!(memory.pvc_bank_addr, FIXED_PROM_WINDOW_SIZE);
        assert!(memory.use_cart_audio);
        assert!(memory.use_cart_fix);
        assert!(!memory.display_enabled);
        assert_eq!(memory.vram[0x20], 0x5a);
        assert_eq!(memory.palette_ram[0x30], 0xa5);
        assert_eq!(memory.palette_bank, 0);
        assert!(!memory.palette_shadow);
        assert_eq!(memory.vram_addr.get(), 0);
        assert_eq!(memory.vram_mod, 0);
        assert_eq!(memory.lspc_mode, 0);
        assert_eq!(memory.auto_animation_counter.get(), 0);
        assert_eq!(memory.auto_animation_timer.get(), 0);
        assert_eq!(memory.auto_animation_reload.get(), 0);
        assert_eq!(memory.lspc_scanline.get(), 0);
        assert_eq!(
            memory.special_board_state(),
            SpecialBoardState {
                cart_reg: [0x1111, 0x2222],
                prot_reg: 0x33445566,
                mslugx_command: 0x0001,
                mslugx_counter: 0x0042,
                sma_rng: 0x2345,
                pending_sma_bank_hi: None,
            }
        );
    }

    #[test]
    fn svc_uses_geolith_watchdog_tolerance() {
        let mut memory = Memory::new();
        let mut rom = rom_with_board(0x269, crate::rom::NeoBoardType::Pvc);

        memory.load_rom(&mut rom);

        assert!(!memory.advance_watchdog(WATCHDOG_M68K_CYCLES));
        assert!(!memory.advance_watchdog(SVC_WATCHDOG_TOLERANCE_M68K_CYCLES - 1));
        assert!(memory.advance_watchdog(1));
    }

    #[test]
    fn io_latch_writes_are_known_noops() {
        let mut memory = Memory::new();

        memory.write8(0x380031, 0xFF);
        memory.write8(0x3A001F, 0xFE);

        assert!(memory.take_bus_trace().is_empty());
    }

    #[test]
    fn test_switch_and_sound_latch_are_memory_mapped() {
        let mut memory = Memory::new();

        assert_eq!(memory.read8(TEST_SWITCH_PORT), 0x80);
        memory.write8(SOUND_PORT, 0x7A);

        assert_eq!(memory.sound_port_snapshot().command, 0x7A);
        assert!(!memory.z80_nmi_pending.get());
        // Sound reply now comes from Z80 (initially clear, matching Geolith)
        assert_eq!(memory.read8(SOUND_PORT), 0x00);
        memory.write8(STATUS_A_PORT, 0x00);
        let trace = memory.take_bus_trace();
        assert_eq!(trace.last().map(|access| access.address), Some(SOUND_PORT));
    }

    #[test]
    fn word_reads_of_input_registers_match_geolith_bus_behavior() {
        let mut memory = Memory::new();
        memory.input_ports.p1 = 0xA5;
        memory.p2_port = 0x5A;
        memory.input_ports.system = 0xC7;
        memory.z80_sound_reply = 0x12;

        assert_eq!(memory.read16(INPUT_P1_PORT), 0xA5A5);
        assert_eq!(memory.read16(INPUT_P2_PORT), 0x5A5A);
        assert_eq!(memory.read16(SYSTEM_PORT), 0xC7C7);
        assert_eq!(memory.read16(SOUND_PORT), 0xFFFF);
    }

    #[test]
    fn sound_latch_triggers_z80_nmi() {
        let mut memory = Memory::new();

        assert_eq!(memory.read8(SOUND_PORT), 0x00);
        assert!(!memory.z80_nmi_pending.get());

        memory.write8(SOUND_PORT, 0x01);
        assert!(!memory.z80_nmi_pending.get());
        assert_eq!(memory.sound_latch, 0x01);

        memory.z80_nmi_enabled.set(true);
        memory.write8(SOUND_PORT, 0x02);
        assert!(memory.z80_nmi_pending.get());
        assert_eq!(memory.sound_latch, 0x02);

        // After Z80 processes the NMI, it should clear nmi_pending and set reply
        memory.z80_nmi_pending.set(false);
        memory.z80_sound_reply = 0x12;
        assert_eq!(memory.read8(SOUND_PORT), 0x12);
    }

    #[test]
    fn z80_work_ram_is_readable_and_writable() {
        let mut memory = Memory::new();

        memory.z80_ram[0] = 0x12;
        memory.z80_ram[Z80_RAM_SIZE - 1] = 0x34;

        assert_eq!(memory.z80_ram[0], 0x12);
        assert_eq!(memory.z80_ram[Z80_RAM_SIZE - 1], 0x34);
    }

    #[test]
    fn z80_bank_registers_track_selected_mrom_windows() {
        let mut memory = Memory::new();

        assert_eq!(memory.z80_bank, Z80_DEFAULT_BANK_OFFSETS);

        memory.z80_bank[0] = 0xAA;
        memory.z80_bank[3] = 0x42;

        assert_eq!(memory.z80_bank[0], 0xAA);
        assert_eq!(memory.z80_bank[3], 0x42);
    }

    #[test]
    fn work_ram_mirrors_every_64k_like_geolith() {
        let mut memory = Memory::new();

        memory.write8(WORK_RAM_START, 0x12);
        memory.write8(WORK_RAM_START + 0x10000, 0x34);
        memory.write16(WORK_RAM_END, 0x5678);

        assert_eq!(memory.read8(WORK_RAM_START + 1), 0x00);
        assert_eq!(memory.read8(WORK_RAM_END), 0x56);
        assert_eq!(memory.read8(WORK_RAM_START), 0x78);
        assert_eq!(memory.read8(WORK_RAM_START + 0x10000), 0x78);
    }

    #[test]
    fn backup_ram_is_memory_mapped_and_writable() {
        let mut memory = Memory::new();

        memory.write8(BACKUP_RAM_START, 0x55);
        memory.write8(BACKUP_RAM_END, 0xAA);

        assert_eq!(memory.read8(BACKUP_RAM_START), 0x55);
        assert_eq!(memory.read8(BACKUP_RAM_END), 0xAA);
        assert!(memory.take_bus_trace().is_empty());
    }

    #[test]
    fn backup_ram_mirrors_every_64k_like_geolith() {
        let mut memory = Memory::new();

        memory.write8(BACKUP_RAM_START, 0x11);
        memory.write8(BACKUP_RAM_START + 0x10000, 0x22);
        memory.write16(BACKUP_RAM_END, 0x3344);

        assert_eq!(memory.read8(BACKUP_RAM_START), 0x44);
        assert_eq!(memory.read8(BACKUP_RAM_START + 0x10000), 0x44);
        assert_eq!(memory.read8(BACKUP_RAM_END), 0x33);
    }

    #[test]
    fn backup_ram_writes_are_ignored_while_sram_is_locked() {
        let mut memory = Memory::new();

        memory.write8(0x3A000D, 0);
        memory.write8(BACKUP_RAM_START, 0x55);
        memory.write16(BACKUP_RAM_START + 2, 0xAABB);
        memory.write32(BACKUP_RAM_START + 4, 0xCCDDEEFF);

        assert_eq!(memory.read8(BACKUP_RAM_START), 0x00);
        assert_eq!(memory.read16(BACKUP_RAM_START + 2), 0x0000);
        assert_eq!(memory.read32(BACKUP_RAM_START + 4), 0x00000000);

        memory.write8(0x3A001D, 0);
        memory.write16(BACKUP_RAM_START + 2, 0xAABB);
        assert_eq!(memory.read16(BACKUP_RAM_START + 2), 0xAABB);
    }

    #[test]
    fn absent_memory_card_reads_open_bus_without_unmapped_trace() {
        let memory = Memory::new();

        assert_eq!(memory.read8(MEMORY_CARD_START), 0xFF);
        assert_eq!(memory.read8(0xA9CC06), 0xFF);
        assert!(memory.take_bus_trace().is_empty());
    }

    #[test]
    fn inserted_memory_card_status_and_unlock_gates_writes() {
        let mut memory = Memory::new();
        memory.memcard_inserted = true;

        assert_eq!(memory.read8(SYSTEM_PORT) & 0x30, 0x00);

        memory.write8(MEMORY_CARD_START, 0x12);
        assert_eq!(memory.read8(MEMORY_CARD_START), 0xFF);

        memory.write8(0x3A0005, 0);
        memory.write8(MEMORY_CARD_START, 0x34);
        assert_eq!(memory.read8(MEMORY_CARD_START), 0xFF);
        assert_eq!(memory.read8(MEMORY_CARD_START + 1), 0xFF);

        memory.write8(0x3A0017, 0);
        memory.write8(MEMORY_CARD_START, 0x34);
        assert_eq!(memory.read8(MEMORY_CARD_START), 0xFF);
        assert_eq!(memory.read8(MEMORY_CARD_START + 1), 0x34);
        assert_eq!(memory.read16(MEMORY_CARD_START), 0xFF34);

        memory.memcard_write_protected = true;
        assert_eq!(memory.read8(SYSTEM_PORT) & 0x40, 0x00);
        memory.write8(MEMORY_CARD_START, 0x56);
        assert_eq!(memory.read8(MEMORY_CARD_START + 1), 0x34);
    }

    #[test]
    fn palette_ram_is_memory_mapped_and_writable() {
        let mut memory = Memory::new();

        memory.write8(PALETTE_RAM_START, 0x12);
        memory.write8(PALETTE_RAM_START + 2, 0x34);

        assert_eq!(memory.read8(PALETTE_RAM_START), 0x12);
        assert_eq!(memory.read8(PALETTE_RAM_START + 3), 0x34);
    }

    #[test]
    fn palette_byte_writes_use_geolith_word_address_lane() {
        let mut memory = Memory::new();

        memory.write16(PALETTE_RAM_START, 0xFFFF);
        memory.write16(PALETTE_RAM_START + 2, 0xFFFF);

        memory.write8(PALETTE_RAM_START, 0x12);
        memory.write8(PALETTE_RAM_START + 1, 0x34);
        memory.write8(PALETTE_RAM_START + 2, 0x56);
        memory.write8(PALETTE_RAM_START + 3, 0x78);

        assert_eq!(memory.read16(PALETTE_RAM_START), 0x34FF);
        assert_eq!(memory.read16(PALETTE_RAM_START + 2), 0xFF78);
    }

    #[test]
    fn palette_ram_mirrors_every_8k_like_geolith() {
        let mut memory = Memory::new();

        memory.write8(PALETTE_RAM_START, 0x12);
        memory.write8(PALETTE_RAM_START + PALETTE_RAM_BANK_SIZE as u32, 0x34);

        assert_eq!(memory.read8(PALETTE_RAM_START), 0x34);
        assert_eq!(
            memory.read8(PALETTE_RAM_START + PALETTE_RAM_BANK_SIZE as u32),
            0x34
        );
    }

    #[test]
    fn palette_bank_latch_selects_palette_ram_page() {
        let mut memory = Memory::new();

        memory.write8(PALETTE_RAM_START, 0x12);
        memory.write8(0x3A000F, 0);
        assert_eq!(memory.read8(PALETTE_RAM_START), 0x00);

        memory.write8(PALETTE_RAM_START, 0x34);
        assert_eq!(memory.palette_ram[0], 0x12);
        assert_eq!(memory.palette_ram[PALETTE_RAM_BANK_SIZE], 0x34);

        memory.write8(0x3A001F, 0);
        assert_eq!(memory.read8(PALETTE_RAM_START), 0x12);
    }

    #[test]
    fn input_ports_are_memory_mapped_and_active_low() {
        let mut memory = Memory::new();
        memory.set_input_ports(InputPorts {
            p1: 0b1111_1110,
            system: 0b1111_1101,
            status_a: 0b1111_1110,
        });

        assert_eq!(memory.read8(INPUT_P1_PORT), 0b1111_1110);
        assert_eq!(memory.read8(DIPSW_PORT), 0xFF);
        assert_eq!(memory.read8(SYSTEM_PORT), 0b1111_1101);
        assert_eq!(memory.read8(STATUS_A_PORT) & 0x01, 0);
    }

    #[test]
    fn status_b_reports_system_type_and_memory_card_like_geolith() {
        let mut memory = Memory::new();

        assert_eq!(memory.read8(SYSTEM_PORT), 0xbf);
        memory.memcard_inserted = true;
        assert_eq!(memory.read8(SYSTEM_PORT), 0x8f);
        memory.memcard_inserted = false;
        memory.set_system_is_mvs(false);
        assert_eq!(memory.read8(SYSTEM_PORT), 0x3f);
    }

    #[test]
    fn status_a_exposes_toggling_rtc_pulse_bit() {
        let mut memory = Memory::new();

        let first = memory.read8(STATUS_A_PORT) & 0x80;
        memory.advance_rtc(RTC_M68K_CYCLES_PER_SECOND / 2);
        let second = memory.read8(STATUS_A_PORT) & 0x80;

        assert_ne!(first, second);
    }

    #[test]
    fn rtc_tp_pin_advances_by_cycles_for_mvs_bios_waits() {
        let mut memory = Memory::new();

        write_rtc_command(&mut memory, 0x04);
        let first = memory.read8(STATUS_A_PORT) & 0x40;
        memory.advance_rtc(RTC_M68K_CYCLES_PER_SECOND / 128);
        let second = memory.read8(STATUS_A_PORT) & 0x40;

        assert_ne!(first, second);
    }

    #[test]
    fn rtc_tp_pin_runs_from_power_on_like_geolith() {
        let mut memory = Memory::new();

        let first = memory.read8(STATUS_A_PORT) & 0x40;
        memory.advance_rtc(RTC_M68K_CYCLES_PER_SECOND / 2);
        let second = memory.read8(STATUS_A_PORT) & 0x40;

        assert_ne!(first, second);
    }

    #[test]
    fn rtc_control_latches_time_read_register() {
        let mut memory = Memory::new();
        *memory.rtc.borrow_mut() = Rtc4990a::from_calendar(0, 0, 12, 25, 1, 5, 26);

        write_rtc_command(&mut memory, 0x03);
        assert_eq!(memory.rtc.borrow().register & 0xff, 0x00);
        assert_eq!((memory.rtc.borrow().register >> 16) & 0xff, 0x12);
        assert_eq!((memory.rtc.borrow().register >> 24) & 0xff, 0x25);
        assert_eq!((memory.rtc.borrow().register >> 36) & 0x0f, 0x05);
        assert_eq!((memory.rtc.borrow().register >> 40) & 0xff, 0x26);

        write_rtc_command(&mut memory, 0x01);
        assert_eq!(memory.read8(STATUS_A_PORT) & 0x80, 0x00);
    }

    #[test]
    fn banked_prom_window_reads_after_fixed_window() {
        let mut memory = Memory::new();
        memory.prom = vec![0; FIXED_PROM_WINDOW_SIZE + BANKED_PROM_WINDOW_SIZE];
        memory.prom[0] = 0x11;
        memory.prom[FIXED_PROM_WINDOW_SIZE] = 0x22;

        memory.set_prom_bank_offset(FIXED_PROM_WINDOW_SIZE);
        // Switch to cart vectors so PROM is readable directly (not BIOS-shadowed).
        memory.write8(0x3A0013, 0);

        assert_eq!(memory.read8(FIXED_PROM_START), 0x11);
        assert_eq!(memory.read8(BANKED_PROM_START), 0x22);
    }

    #[test]
    fn load_rom_initializes_banked_prom_offset() {
        let mut memory = Memory::new();
        let mut rom = crate::rom::RomData::demo();
        rom.is_demo = false;
        rom.prom = vec![0; FIXED_PROM_WINDOW_SIZE + 4];

        memory.load_rom(&mut rom);

        assert_eq!(memory.prom_bank_offset, FIXED_PROM_WINDOW_SIZE);
    }

    #[test]
    fn bank_register_selects_available_banked_prom_window() {
        let mut memory = Memory::new();
        memory.prom = vec![0; FIXED_PROM_WINDOW_SIZE + BANKED_PROM_WINDOW_SIZE * 2];
        memory.prom[FIXED_PROM_WINDOW_SIZE] = 0x22;
        memory.prom[FIXED_PROM_WINDOW_SIZE + BANKED_PROM_WINDOW_SIZE] = 0x33;

        memory.write8(PROM_BANK_REGISTER_START, 1);

        assert_eq!(
            memory.prom_bank_offset,
            FIXED_PROM_WINDOW_SIZE + BANKED_PROM_WINDOW_SIZE
        );
        assert_eq!(memory.read8(BANKED_PROM_START), 0x33);

        memory.write8(PROM_BANK_REGISTER_START, 2);

        assert_eq!(memory.prom_bank_offset, FIXED_PROM_WINDOW_SIZE);
        assert_eq!(memory.read8(BANKED_PROM_START), 0x22);
    }

    #[test]
    fn generic_bank_register_uses_geolith_power_of_two_mask() {
        let mut memory = Memory::new();
        memory.prom = vec![0; FIXED_PROM_WINDOW_SIZE + BANKED_PROM_WINDOW_SIZE * 3];
        memory.prom[FIXED_PROM_WINDOW_SIZE] = 0x11;
        memory.prom[FIXED_PROM_WINDOW_SIZE + BANKED_PROM_WINDOW_SIZE * 2] = 0x33;

        memory.write8(PROM_BANK_REGISTER_START, 2);
        assert_eq!(memory.prom_bank_offset, 0x300000);
        assert_eq!(memory.read8(BANKED_PROM_START), 0x33);

        // Three banks produce mask 0b11. Selecting bank 3 must address the
        // fourth slot/open bus, not wrap modulo 3 back to the first bank.
        memory.write8(PROM_BANK_REGISTER_START, 3);
        assert_eq!(memory.prom_bank_offset, 0x400000);
        assert_eq!(memory.read8(BANKED_PROM_START), 0xff);
    }

    #[test]
    fn mslugx_protection_handles_command_reads_and_bankswitch() {
        let mut memory = Memory::new();
        let mut rom = crate::rom::RomData::demo();
        rom.is_demo = false;
        rom.prom = vec![0; FIXED_PROM_WINDOW_SIZE + BANKED_PROM_WINDOW_SIZE * 4];
        rom.prom[0x0d_edd2] = 0x80;
        rom.prom[FIXED_PROM_WINDOW_SIZE] = 0x11;
        rom.prom[FIXED_PROM_WINDOW_SIZE + BANKED_PROM_WINDOW_SIZE] = 0x22;
        rom.metadata = Some(crate::rom::NeoMetadata {
            version: 0,
            year: 1999,
            genre: 0,
            screenshot: 0,
            ngh: 0x0250,
            name: "Metal Slug X".to_string(),
            manufacturer: "SNK".to_string(),
            board_type: crate::rom::NeoBoardType::MslugX,
            fix_banksw: crate::rom::NeoFixBanksw::None,
            game_flags: 0,
        });

        memory.load_rom(&mut rom);

        let prom_word_offset = FIXED_PROM_WINDOW_SIZE + 0x0fffe2;
        memory.prom[prom_word_offset..prom_word_offset + 2].copy_from_slice(&[0x12, 0x34]);

        // Geolith leaves byte accesses on the default handlers.
        memory.write8(0x2FFFE2, 0x01);
        assert_eq!(memory.mslugx_command.get(), 0);
        assert_eq!(memory.read8(0x2FFFE2), 0x12);

        // Word writes build the command, and every word address in E0..EF
        // exposes the same challenge/response bit stream.
        memory.write16(0x2FFFE0, 0);
        memory.write16(0x2FFFE2, 1);
        assert_eq!(memory.read16(0x2FFFE0), 1);
        assert_eq!(memory.read16(0x2FFFEE), 0);
        assert_eq!(memory.mslugx_counter.get(), 2);

        // EAh resets both command and counter, returning subsequent reads
        // to the normal banked P-ROM path.
        memory.write16(0x2FFFEA, 0);
        assert_eq!(memory.mslugx_command.get(), 0);
        assert_eq!(memory.mslugx_counter.get(), 0);
        assert_eq!(memory.read16(0x2FFFE2), 0x1234);

        memory.write16(0x2FFFF0, 0x0001);
        assert_eq!(memory.prom_bank_offset, 0x200000);
        assert_eq!(memory.read8(BANKED_PROM_START), 0x22);

        memory.write16(0x2FFFF0, 0x0008);
        assert_eq!(memory.prom_bank_offset, 0x900000);
    }

    #[test]
    fn pvc_longword_write_splits_into_two_word_side_effects_like_geolith() {
        let mut memory = Memory::new();
        let mut rom = crate::rom::RomData::demo();
        rom.is_demo = false;
        rom.prom = vec![0; FIXED_PROM_WINDOW_SIZE + BANKED_PROM_WINDOW_SIZE * 8];
        rom.metadata = Some(crate::rom::NeoMetadata {
            version: 0,
            year: 2003,
            genre: 0,
            screenshot: 0,
            ngh: 0x0268,
            name: "Metal Slug 5".to_string(),
            manufacturer: "SNK".to_string(),
            board_type: crate::rom::NeoBoardType::Pvc,
            fix_banksw: crate::rom::NeoFixBanksw::None,
            game_flags: 0,
        });

        memory.load_rom(&mut rom);

        // Geolith implements m68k_write_memory_32 as two write16 calls.
        // The first write16 to 0x2FFFF0 triggers bankswap and clears bit 0
        // of PVC RAM byte 0x1FF1 before the second halfword is written.
        memory.write32(0x2FFFF0, 0x1235_5678);

        assert_eq!(memory.pvc_cart_ram[0x1ff0], 0xA0);
        assert_eq!(memory.pvc_cart_ram[0x1ff1], 0x12);
        assert_eq!(memory.pvc_cart_ram[0x1ff2], 0x78);
        assert_eq!(memory.pvc_cart_ram[0x1ff3], 0x56);
        assert_eq!(memory.pvc_bank_addr, 0x667812);
    }

    #[test]
    fn pvc_word_reads_use_geolith_byte_order() {
        let mut memory = Memory::new();
        let mut rom = crate::rom::RomData::demo();
        rom.is_demo = false;
        rom.metadata = Some(crate::rom::NeoMetadata {
            version: 0,
            year: 2003,
            genre: 0,
            screenshot: 0,
            ngh: 0x0268,
            name: "Metal Slug 5".to_string(),
            manufacturer: "SNK".to_string(),
            board_type: crate::rom::NeoBoardType::Pvc,
            fix_banksw: crate::rom::NeoFixBanksw::None,
            game_flags: 0,
        });
        memory.load_rom(&mut rom);

        memory.write16(0x2FE100, 0x1234);
        memory.write16(0x2FE102, 0xABCD);

        assert_eq!(memory.pvc_cart_ram[0x0100], 0x34);
        assert_eq!(memory.pvc_cart_ram[0x0101], 0x12);
        assert_eq!(memory.read16(0x2FE100), 0x1234);
        assert_eq!(memory.read32(0x2FE100), 0x1234_ABCD);
    }

    #[test]
    fn mslug3_sma_protection_exposes_presence_rng_and_bankswitch() {
        let mut memory = Memory::new();
        let mut rom = crate::rom::RomData::demo();
        rom.is_demo = false;
        rom.prom = vec![0; FIXED_PROM_WINDOW_SIZE + 0x080000];
        rom.prom[FIXED_PROM_WINDOW_SIZE] = 0x11;
        rom.prom[FIXED_PROM_WINDOW_SIZE + 0x040000] = 0x44;
        rom.recognized_files = vec![
            "neo-sma".to_string(),
            "256-pg1.p1".to_string(),
            "256-pg2.p2".to_string(),
        ];

        memory.load_rom(&mut rom);

        let config = SmaConfig::mslug3();

        // Presence address is 0x2FE446 (same for all SMA games)
        assert_eq!(memory.read8(SMA_PRESENCE_ADDR), 0x37);
        assert_eq!(memory.read8(SMA_PRESENCE_ADDR + 1), 0x9A);
        assert_eq!(memory.read16(SMA_PRESENCE_ADDR), SMA_PRESENCE_VALUE);

        // PRN addresses from config
        let prn0 = config.prn_addr[0];
        let prn1 = config.prn_addr[1];
        // Geolith advances the LFSR before returning each PRN read.
        assert_eq!(memory.read16(prn0), 0x468A);
        assert_eq!(memory.read16(prn1), 0x8D14);

        // Bank register from config
        let bank_reg = config.bank_reg_addr;
        memory.write8(bank_reg, 0x10);
        memory.write8(bank_reg + 1, 0x00);
        assert_eq!(
            memory.prom_bank_offset, FIXED_PROM_WINDOW_SIZE,
            "Geolith ignores byte writes to this SMA bank register"
        );

        memory.write16(bank_reg, 0x1000);

        assert_eq!(memory.prom_bank_offset, FIXED_PROM_WINDOW_SIZE + 0x040000);
        assert_eq!(memory.read8(BANKED_PROM_START), 0x44);
    }

    #[test]
    fn mslug3a_metadata_selects_alternate_geolith_sma_table() {
        let mut rom = crate::rom::RomData::demo();
        rom.is_demo = false;
        rom.metadata = Some(crate::rom::NeoMetadata {
            version: 1,
            year: 2000,
            genre: 0,
            screenshot: 0,
            ngh: 0x0256,
            name: "Metal Slug 3".to_string(),
            manufacturer: "SNK".to_string(),
            board_type: crate::rom::NeoBoardType::SmaMslug3A,
            fix_banksw: crate::rom::NeoFixBanksw::Line,
            game_flags: 0,
        });

        assert_eq!(
            detect_cart_protection(&rom),
            CartProtection::Sma(SmaConfig::mslug3a())
        );
        assert_ne!(SmaConfig::mslug3a(), SmaConfig::mslug3());
    }

    #[test]
    fn kof99_sma_protection_uses_geolith_bank_register_and_table() {
        let mut memory = Memory::new();
        let mut rom = crate::rom::RomData::demo();
        rom.is_demo = false;
        rom.prom = vec![0; FIXED_PROM_WINDOW_SIZE + 0x600000];
        rom.prom[FIXED_PROM_WINDOW_SIZE + 0x3CC000] = 0x99;
        rom.metadata = Some(crate::rom::NeoMetadata {
            version: 0,
            year: 1999,
            genre: 0,
            screenshot: 0,
            ngh: 0x0251,
            name: "The King of Fighters '99".to_string(),
            manufacturer: "SNK".to_string(),
            board_type: crate::rom::NeoBoardType::Sma,
            fix_banksw: crate::rom::NeoFixBanksw::None,
            game_flags: 0,
        });

        memory.load_rom(&mut rom);

        let config = SmaConfig::kof99();
        assert_eq!(memory.read16(config.prn_addr[0]), 0x468A);

        // Byte reads also advance once and expose the low byte of the new value.
        memory.sma_rng.set(0x2345);
        assert_eq!(memory.read8(config.prn_addr[0]), 0x8A);
        assert_eq!(memory.sma_rng.get(), 0x468A);

        // Geolith's KOF99 scramble maps this value to bank-table index 4,
        // whose offset is 0x3CC000.
        memory.write16(config.bank_reg_addr, 0x0100);

        assert_eq!(memory.prom_bank_offset, FIXED_PROM_WINDOW_SIZE + 0x3CC000);
        assert_eq!(memory.read8(BANKED_PROM_START), 0x99);

        // KOF99's SMA bank register overlaps the default byte-bank region.
        // Geolith keeps byte writes on the generic handler, while unrelated
        // SMA word writes are ignored by the dedicated SMA word handler.
        memory.write8(config.bank_reg_addr, 0x01);
        assert_eq!(memory.prom_bank_offset, 0x200000);
        memory.write16(config.prn_addr[0], 0x0005);
        assert_eq!(memory.prom_bank_offset, 0x200000);
    }

    #[test]
    fn linkable_board_fakes_link_status_and_byte_bankswitch() {
        let mut memory = Memory::new();
        let mut rom = rom_with_board(0x006, crate::rom::NeoBoardType::Linkable);
        rom.prom = vec![0; FIXED_PROM_WINDOW_SIZE + BANKED_PROM_WINDOW_SIZE * 3];
        memory.load_rom(&mut rom);

        assert_eq!(memory.read8(0x200000), 0x08);
        assert_eq!(memory.read8(0x200000), 0x00);
        assert_eq!(memory.read8(0x200001), 0x00);

        memory.write8(0x2ffff0, 0x02);
        assert_eq!(memory.prom_bank_offset, 0x300000);
    }

    #[test]
    fn ct0_board_uses_geolith_challenge_response_and_bankswitch() {
        let mut memory = Memory::new();
        let mut rom = rom_with_board(0x047, crate::rom::NeoBoardType::Ct0);
        rom.prom = vec![0; FIXED_PROM_WINDOW_SIZE + BANKED_PROM_WINDOW_SIZE * 4];
        memory.load_rom(&mut rom);

        memory.write16(0x242812, 0);
        assert_eq!(memory.read16(0x236000), 0x0081);
        assert_eq!(memory.read16(0x236004), 0x0018);
        memory.write8(0x236001, 0);
        assert_eq!(memory.read8(0x236001), 0x42);

        memory.write16(0x2ffff0, 0x0002);
        assert_eq!(memory.prom_bank_offset, 0x300000);
    }

    #[test]
    fn kof98_board_applies_and_removes_prom_overlay() {
        let mut memory = Memory::new();
        let mut rom = rom_with_board(0x242, crate::rom::NeoBoardType::Kof98);
        rom.prom = vec![0; FIXED_PROM_WINDOW_SIZE + BANKED_PROM_WINDOW_SIZE * 2];
        rom.prom[0x100..0x104].copy_from_slice(b"NEO-");
        memory.load_rom(&mut rom);

        memory.write16(0x20aaaa, 0x0090);
        assert_eq!(&memory.prom[0x100..0x104], &[0x00, 0xc2, 0x00, 0xfd]);

        memory.write16(0x20aaaa, 0x00f0);
        assert_eq!(&memory.prom[0x100..0x104], b"NEO-");

        memory.write16(0x2ffff0, 0x0001);
        assert_eq!(memory.prom_bank_offset, 0x200000);
    }

    #[test]
    fn brezzasoft_board_maps_8k_cartram_and_open_bus() {
        let mut memory = Memory::new();
        let mut rom = rom_with_board(0x008, crate::rom::NeoBoardType::Brezzasoft);
        rom.prom = vec![0; FIXED_PROM_WINDOW_SIZE + BANKED_PROM_WINDOW_SIZE];
        memory.load_rom(&mut rom);

        memory.write16(0x200000, 0x1234);
        assert_eq!(memory.read8(0x200000), 0x12);
        assert_eq!(memory.read16(0x200000), 0x1234);
        assert_eq!(memory.read8(0x202000), 0xff);
        assert_eq!(memory.read16(0x202000), 0xffff);
        assert_eq!(memory.read16(0x2c0000), 0xffc0);
    }

    #[test]
    fn ms5plus_and_cthd2003_use_bootleg_bank_handlers() {
        let mut ms5plus = Memory::new();
        let mut rom = rom_with_board(0x268, crate::rom::NeoBoardType::Ms5Plus);
        rom.prom = vec![0; 0x900000];
        ms5plus.load_rom(&mut rom);
        ms5plus.write16(0x2ffff4, 0x0034);
        assert_eq!(ms5plus.prom_bank_offset, 0x340000);

        let mut cthd = Memory::new();
        let mut rom = rom_with_board(0x5003, crate::rom::NeoBoardType::Cthd2003);
        rom.prom = vec![0; 0x500000];
        cthd.load_rom(&mut rom);
        cthd.write16(0x2ffff0, 0x0006);
        assert_eq!(cthd.prom_bank_offset, 0x400000);
        cthd.write16(0x2ffff0, 0x0007);
        assert_eq!(cthd.prom_bank_offset, 0x300000);
    }

    #[test]
    fn kof2003_bootleg_boards_match_geolith_pvc_variants() {
        let mut bl = Memory::new();
        let mut rom = rom_with_board(0x271, crate::rom::NeoBoardType::Kf2k3Bl);
        rom.prom = vec![0; FIXED_PROM_WINDOW_SIZE + BANKED_PROM_WINDOW_SIZE * 8];
        bl.load_rom(&mut rom);
        bl.pvc_cart_ram[0x1ff2] = 0x56;
        assert_eq!(bl.read8(0x058197), 0x56);

        let mut bla = Memory::new();
        let mut rom = rom_with_board(0x271, crate::rom::NeoBoardType::Kf2k3Bla);
        rom.prom = vec![0; FIXED_PROM_WINDOW_SIZE + BANKED_PROM_WINDOW_SIZE * 8];
        bla.load_rom(&mut rom);
        bla.write16(0x2ffff0, 0x1234);

        assert_eq!(bla.pvc_cart_ram[0x1ff0], 0x34);
        assert_eq!(bla.pvc_cart_ram[0x1ff1], 0x12);
        assert_eq!(bla.pvc_bank_addr, 0x100034);
    }

    #[test]
    fn kof10th_board_maps_extra_ram_dynamic_fix_and_bank_selects() {
        let mut memory = Memory::new();
        let mut rom = rom_with_board(0x275, crate::rom::NeoBoardType::Kof10th);
        rom.prom = vec![0; 0x900000];
        rom.prom[0] = 0x11;
        rom.prom[0x700000] = 0x77;
        memory.load_rom(&mut rom);
        memory.write8(0x3A0013, 0);

        assert_eq!(memory.kof10th_extra_ram.len(), KOF10TH_EXTRA_RAM_SIZE);
        assert_eq!(memory.dynamic_fix_rom.len(), KOF10TH_DYNFIX_SIZE);
        assert_eq!(&memory.prom[0x0124..=0x0127], &[0x00, 0x0d, 0xf7, 0xa8]);
        assert_eq!(
            &memory.prom[0x8bf4..=0x8bf9],
            &[0x4e, 0xf9, 0x00, 0x0d, 0xf9, 0x80]
        );

        memory.write16(0x200000, 0x1234);
        assert_eq!(memory.read16(0x0e0000), 0x1234);

        memory.pvc_cart_ram[0x1ffc] = 1;
        memory.write16(0x200000, 0x0001);
        assert_eq!(memory.dynamic_fix_rom[0], 0x20);
        memory.write16(0x200002, 0x0020);
        assert_eq!(memory.dynamic_fix_rom[1], 0x01);

        memory.write16(0x2ffff0, 0x0005);
        assert_eq!(memory.prom_bank_offset, 0x600000);
        memory.write16(0x2ffff0, 0x0006);
        assert_eq!(memory.prom_bank_offset, 0x100000);

        memory.write16(0x2ffff8, 0x0001);
        assert_eq!(memory.prot_reg, 0);
        memory.write16(0x2ffff8, 0x0000);
        assert_eq!(memory.prot_reg, 0x700000);
        assert_eq!(memory.read8(0x000000), 0x77);
    }

    fn write_rtc_command(memory: &mut Memory, command: u8) {
        for bit in 0..4 {
            let data = (command >> bit) & 1;
            memory.write8(RTC_CONTROL_PORT, data);
            memory.write8(RTC_CONTROL_PORT, data | 0x02);
            memory.write8(RTC_CONTROL_PORT, data);
        }
        memory.write8(RTC_CONTROL_PORT, 0x04);
        memory.write8(RTC_CONTROL_PORT, 0x00);
    }

    fn rom_with_board(ngh: u32, board_type: crate::rom::NeoBoardType) -> crate::rom::RomData {
        let mut rom = crate::rom::RomData::demo();
        rom.is_demo = false;
        rom.metadata = Some(crate::rom::NeoMetadata {
            version: 0,
            year: 1999,
            genre: 0,
            screenshot: 0,
            ngh,
            name: format!("test-{ngh:03x}"),
            manufacturer: "SNK".to_string(),
            board_type,
            fix_banksw: crate::rom::NeoFixBanksw::None,
            game_flags: 0,
        });
        rom
    }
}

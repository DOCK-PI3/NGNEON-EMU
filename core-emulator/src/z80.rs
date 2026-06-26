//! Z80 co-processor emulation (sound CPU)
//!
//! NeoGeo uses a Z80 at 4 MHz to drive the YM2610 FM/ADPCM sound chip.
//! The 68000 communicates with the Z80 via a hardware-latched sound port:
//!   - 68000 writes command to 0x320000 → triggers Z80 NMI
//!   - Z80 reads command from I/O port 0x00
//!   - Z80 writes reply to I/O port 0x0C → 68000 reads from 0x320000

use std::cell::RefCell;
use std::rc::Rc;

use crate::memory::{
    BusAccessKind, Memory, Z80_BANK_WINDOW_0_ADDR, Z80_BANK_WINDOW_0_SIZE, Z80_BANK_WINDOW_1_ADDR,
    Z80_BANK_WINDOW_1_SIZE, Z80_BANK_WINDOW_2_ADDR, Z80_BANK_WINDOW_2_SIZE, Z80_BANK_WINDOW_3_ADDR,
    Z80_BANK_WINDOW_3_SIZE, Z80_DEFAULT_BANK_OFFSETS, Z80_MROM_STATIC_SIZE, Z80_RAM_ADDR,
    Z80_RAM_SIZE,
};
use crate::ym2610::Ym2610;
use z80emu::{host::TsCounter, Clock, Cpu, Io, Memory as Z80Mem};

/// I/O port assignments
const PORT_SOUND_LATCH: u16 = 0x00;
const PORT_YM2610_A0: u16 = 0x04;
const PORT_YM2610_D0: u16 = 0x05;
const PORT_YM2610_A1: u16 = 0x06;
const PORT_YM2610_D1: u16 = 0x07;
const PORT_BANK_W0: u16 = 0x08;
const PORT_BANK_W1: u16 = 0x09;
const PORT_BANK_W2: u16 = 0x0A;
const PORT_BANK_W3: u16 = 0x0B;
const PORT_SOUND_REPLY: u16 = 0x0C;
const PORT_NMI_DISABLE: u16 = 0x18;
const PORT_SOUND_LATCH_CLEAR: u16 = 0xC0;
const HALT_IDLE_TSTATES: u32 = 4;

/// Bus adapter implementing both `Memory` and `Io` traits for `z80emu`.
pub struct Z80Bus {
    pub mem: Rc<RefCell<Memory>>,
    pub ym2610: Rc<RefCell<Ym2610>>,
}

impl Z80Bus {
    fn read_mrom_banked(&self, addr: u16) -> u8 {
        let mem = self.mem.borrow();
        let using_sm1 = !mem.use_cart_audio && !mem.sm1.is_empty();
        let mrom = if using_sm1 { &mem.sm1 } else { &mem.mrom };

        // Static region: 0x0000–0x7FFF (first 32 KiB of M-ROM)
        if (addr as usize) < Z80_MROM_STATIC_SIZE {
            return mrom.get(addr as usize).copied().unwrap_or(0xFF);
        }

        // Banked windows — check from highest address to lowest
        let (window, base_addr, size) = if addr >= Z80_BANK_WINDOW_0_ADDR {
            (0, Z80_BANK_WINDOW_0_ADDR as usize, Z80_BANK_WINDOW_0_SIZE)
        } else if addr >= Z80_BANK_WINDOW_1_ADDR {
            (1, Z80_BANK_WINDOW_1_ADDR as usize, Z80_BANK_WINDOW_1_SIZE)
        } else if addr >= Z80_BANK_WINDOW_2_ADDR {
            (2, Z80_BANK_WINDOW_2_ADDR as usize, Z80_BANK_WINDOW_2_SIZE)
        } else {
            (3, Z80_BANK_WINDOW_3_ADDR as usize, Z80_BANK_WINDOW_3_SIZE)
        };

        let window_offset = addr as usize - base_addr;
        if window_offset >= size {
            return 0xFF;
        }

        let bank_base = mem.z80_bank[window];

        mrom.get(bank_base + window_offset).copied().unwrap_or(0xFF)
    }

    fn bankswap_from_port(&mut self, port: u16) -> bool {
        let Some((window, mask, size)) = bank_window_for_port(port) else {
            return false;
        };

        let selected = ((port >> 8) as usize) & mask;
        self.mem.borrow_mut().z80_bank[window] = selected * size;
        true
    }

    fn record_io(&self, kind: BusAccessKind, port: u16, value: u8) {
        self.mem.borrow().record_z80_io_access(kind, port, value);
    }
}

fn bank_window_for_port(port: u16) -> Option<(usize, usize, usize)> {
    match port & 0x00FF {
        PORT_BANK_W0 => Some((0, 0x7F, Z80_BANK_WINDOW_0_SIZE)),
        PORT_BANK_W1 => Some((1, 0x3F, Z80_BANK_WINDOW_1_SIZE)),
        PORT_BANK_W2 => Some((2, 0x1F, Z80_BANK_WINDOW_2_SIZE)),
        PORT_BANK_W3 => Some((3, 0x0F, Z80_BANK_WINDOW_3_SIZE)),
        _ => None,
    }
}

// ─── z80emu::Memory trait ─────────────────────────────────────────────

impl Z80Mem for Z80Bus {
    type Timestamp = i32;

    fn read_mem(&self, address: u16, _ts: i32) -> u8 {
        // Z80 work RAM at 0xF800–0xFFFF
        if address >= Z80_RAM_ADDR {
            let offset = (address - Z80_RAM_ADDR) as usize % Z80_RAM_SIZE;
            return self
                .mem
                .borrow()
                .z80_ram
                .get(offset)
                .copied()
                .unwrap_or(0xFF);
        }

        // Everything else is M-ROM (banked)
        self.read_mrom_banked(address)
    }

    fn read_mem16(&self, address: u16, _ts: i32) -> u16 {
        u16::from_le_bytes([
            self.read_mem(address, 0),
            self.read_mem(address.wrapping_add(1), 0),
        ])
    }

    fn read_opcode(&mut self, pc: u16, _ir: u16, _ts: i32) -> u8 {
        // read_opcode just delegates to read_mem for our simple bus.
        // read_mem takes &self, and &mut self coerces to &self.
        self.read_mem(pc, 0)
    }

    fn write_mem(&mut self, address: u16, value: u8, _ts: i32) {
        // Only Z80 work RAM is writable
        if address >= Z80_RAM_ADDR {
            let offset = (address - Z80_RAM_ADDR) as usize % Z80_RAM_SIZE;
            let mut mem = self.mem.borrow_mut();
            if let Some(cell) = mem.z80_ram.get_mut(offset) {
                *cell = value;
            }
        }
        // M-ROM is read-only — writes are ignored
    }

    fn read_debug(&self, address: u16) -> u8 {
        self.read_mem(address, 0)
    }
}

// ─── z80emu::Io trait ─────────────────────────────────────────────────

impl Io for Z80Bus {
    type Timestamp = i32;
    type WrIoBreak = ();
    type RetiBreak = ();

    fn read_io(&mut self, port: u16, _ts: i32) -> (u8, Option<std::num::NonZeroU16>) {
        if self.bankswap_from_port(port) {
            self.record_io(BusAccessKind::Read, port, 0);
            return (0, None);
        }

        let port_low = port & 0x00FF;
        let val = match port_low {
            PORT_SOUND_LATCH => {
                // Read the command byte the 68000 wrote to 0x320000
                self.mem.borrow().sound_latch
            }
            // YM2610 address port reads return the chip status register.
            // Port 0: bit 7 = BUSY, bit 1 = Timer B, bit 0 = Timer A
            // Port 1: bit 7 = ADPCM-B playing, bits 0-5 = ADPCM-A channels
            // Real Z80 sound drivers poll this to detect BUSY flag clearing.
            PORT_YM2610_A0 => self.ym2610.borrow().read_status(0),
            PORT_YM2610_A1 => self.ym2610.borrow().read_status(1),
            PORT_YM2610_D0 => {
                let mut ym = self.ym2610.borrow_mut();
                ym.write_address(0, self.mem.borrow().ym2610_addr[0]);
                ym.read_data(0)
            }
            PORT_YM2610_D1 => {
                let mut ym = self.ym2610.borrow_mut();
                ym.write_address(1, self.mem.borrow().ym2610_addr[1]);
                ym.read_data(1)
            }
            // Geolith returns open zero for unhandled sound ports.  Several
            // M1 drivers use the 0x0D/0x0E pair as an auxiliary handshake.
            _ => 0x00,
        };
        self.record_io(BusAccessKind::Read, port, val);
        (val, None)
    }

    fn write_io(
        &mut self,
        port: u16,
        data: u8,
        _ts: i32,
    ) -> (Option<Self::WrIoBreak>, Option<std::num::NonZeroU16>) {
        let port_low = port & 0x00FF;
        match port_low {
            PORT_SOUND_LATCH | PORT_SOUND_LATCH_CLEAR => {
                self.mem.borrow_mut().sound_latch = 0;
            }
            PORT_SOUND_REPLY => {
                // Z80 sets the reply byte the 68000 reads from 0x320000.
                // Geolith preserves zero writes here; some games use them as
                // part of the sound/protection handshake.
                self.mem.borrow_mut().z80_sound_reply = data;
            }
            PORT_YM2610_A0 => {
                self.mem.borrow_mut().ym2610_addr[0] = data;
            }
            PORT_YM2610_A1 => {
                self.mem.borrow_mut().ym2610_addr[1] = data;
            }
            PORT_YM2610_D0 => {
                let mut ym = self.ym2610.borrow_mut();
                ym.write_address(0, self.mem.borrow().ym2610_addr[0]);
                ym.write_data(0, data);
            }
            PORT_YM2610_D1 => {
                let mut ym = self.ym2610.borrow_mut();
                ym.write_address(1, self.mem.borrow().ym2610_addr[1]);
                ym.write_data(1, data);
            }
            PORT_BANK_W0 => {
                self.mem.borrow_mut().z80_nmi_enabled.set(true);
            }
            PORT_NMI_DISABLE => {
                self.mem.borrow_mut().z80_nmi_enabled.set(false);
            }
            _ => {
                // Unknown port — ignore
            }
        }
        self.record_io(BusAccessKind::Write, port, data);
        (None, None)
    }

    fn is_irq(&mut self, _ts: i32) -> bool {
        self.ym2610.borrow().timer_irq_pending()
    }

    fn irq_data(&mut self, _pc: u16, _ts: i32) -> (u8, Option<std::num::NonZeroU16>) {
        // The Z80 on NeoGeo uses IM 1 for YM2610 timer interrupts, which
        // means the CPU automatically vectors to 0x0038 (RST 38H) regardless
        // of the data byte. Return 0xFF as the open-bus default.
        (0xFF, None)
    }

    fn reti(&mut self, _address: u16, _ts: i32) -> Option<Self::RetiBreak> {
        None
    }
}

// ─── Z80 wrapper ──────────────────────────────────────────────────────

/// Z80 co-processor wrapper around the `z80emu` crate.
pub struct Z80 {
    pub cpu: z80emu::Z80NMOS,
    bus: Z80Bus,
    tsc: TsCounter<i32>,
}

impl Z80 {
    pub fn new(mem: Rc<RefCell<Memory>>, ym2610: Rc<RefCell<Ym2610>>) -> Self {
        Self {
            cpu: z80emu::Z80NMOS::default(),
            bus: Z80Bus { mem, ym2610 },
            tsc: TsCounter::default(),
        }
    }

    /// Reset the Z80 — reads reset vector from address 0x0000 (M-ROM).
    pub fn reset(&mut self) {
        self.cpu.reset();
        self.tsc = TsCounter::default();
        let mut mem = self.bus.mem.borrow_mut();
        mem.z80_bank = Z80_DEFAULT_BANK_OFFSETS;
        mem.z80_nmi_enabled.set(false);
        mem.z80_nmi_pending.set(false);
    }

    /// Execute a single Z80 instruction.
    /// Returns the number of T-states consumed.
    pub fn step(&mut self) -> u32 {
        // Service NMI if pending (NMI wakes the CPU from HALT)
        let nmi = self.bus.mem.borrow().z80_nmi_pending.get();
        if nmi {
            self.bus.mem.borrow().z80_nmi_pending.set(false);
            self.cpu.nmi(&mut self.bus, &mut self.tsc);
        }

        // Always call execute_next(), even when halted.  The z80emu crate
        // checks is_irq() inside execute_next() to wake the CPU from HALT
        // on a timer interrupt.  Skipping execution while halted made the
        // music sequencer freeze after the first HALT — only NMI (ADPCM
        // commands from the 68k) could wake it up.
        let ts_before = self.tsc.as_timestamp();
        let _ = self
            .cpu
            .execute_next(&mut self.bus, &mut self.tsc, None::<fn(z80emu::CpuDebug)>);
        let elapsed = self.tsc.as_timestamp().saturating_sub(ts_before) as u32;
        if elapsed == 0 && self.cpu.is_halt() {
            // HALT does not stop the NeoGeo sound subsystem clock. Geolith
            // keeps advancing the Z80/YM2610 timebase while the sound driver
            // waits for YM timer IRQs; returning zero here would freeze those
            // timers until a later NMI, causing round/music dropouts.
            HALT_IDLE_TSTATES
        } else {
            elapsed
        }
    }

    /// Run the Z80 for a given number of T-states.
    /// The Z80 at 4 MHz executes roughly 4M T-states per second.
    ///
    /// Executes instruction-by-instruction so that external NMIs
    /// (triggered by 68000 writes to SOUND_PORT) are serviced
    /// between instructions. Without this, the sound driver never
    /// receives commands and the YM2610 stays silent.
    pub fn run_tstates(&mut self, max_tstates: u32) -> u32 {
        let mut executed = 0u32;

        while executed < max_tstates {
            let ts = self.step();
            executed = executed.saturating_add(ts);
            if ts == 0 {
                // CPU is halted with no pending interrupt — nothing more to do
                break;
            }
        }

        executed
    }

    pub fn pc(&self) -> u16 {
        self.cpu.get_pc()
    }

    pub fn is_halt(&self) -> bool {
        self.cpu.is_halt()
    }

    pub fn interrupt_state(&self) -> (bool, bool, u8) {
        let (iff1, iff2) = self.cpu.get_iffs();
        (iff1, iff2, self.cpu.get_im() as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_test_z80() -> (Z80, Rc<RefCell<Memory>>) {
        let mem = Rc::new(RefCell::new(Memory::new()));
        let ym2610 = Rc::new(RefCell::new(Ym2610::new(mem.clone())));
        let z80 = Z80::new(mem.clone(), ym2610);
        (z80, mem)
    }

    /// Helper: create a Z80Bus connected to the given memory with a fresh YM2610.
    fn new_test_z80bus(mem: Rc<RefCell<Memory>>) -> Z80Bus {
        let ym2610 = Rc::new(RefCell::new(Ym2610::new(mem.clone())));
        Z80Bus { mem, ym2610 }
    }

    #[test]
    fn z80_reads_static_mrom_region() {
        let (_z80, mem) = new_test_z80();
        mem.borrow_mut().mrom = vec![0x12; 0x10000];

        let bus = new_test_z80bus(mem);
        assert_eq!(bus.read_mem(0x0000, 0), 0x12);
        assert_eq!(bus.read_mem(0x7FFF, 0), 0x12);
    }

    #[test]
    fn z80_reads_sm1_when_bios_audio_is_selected() {
        let (_z80, mem) = new_test_z80();
        {
            let mut memory = mem.borrow_mut();
            memory.mrom = vec![0x12; 0x10000];
            memory.sm1 = vec![0xA9; 0x10000];
            memory.use_cart_audio = false;
        }

        let bus = new_test_z80bus(mem);
        assert_eq!(bus.read_mem(0x0000, 0), 0xA9);
        assert_eq!(bus.read_mem(0x7FFF, 0), 0xA9);
    }

    #[test]
    fn z80_reads_from_work_ram() {
        let (_z80, mem) = new_test_z80();
        mem.borrow_mut().z80_ram[0] = 0x42;
        mem.borrow_mut().z80_ram[Z80_RAM_SIZE - 1] = 0xAB;

        let bus = new_test_z80bus(mem);
        assert_eq!(bus.read_mem(Z80_RAM_ADDR, 0), 0x42);
        // Z80_RAM_ADDR + Z80_RAM_SIZE - 1 would overflow u16, so compute via usize
        let last_ram_addr = Z80_RAM_ADDR
            .wrapping_add(Z80_RAM_SIZE as u16)
            .wrapping_sub(1);
        assert_eq!(bus.read_mem(last_ram_addr, 0), 0xAB);
    }

    #[test]
    fn z80_writes_to_work_ram() {
        let (_z80, mem) = new_test_z80();

        let mut bus = new_test_z80bus(mem.clone());
        bus.write_mem(Z80_RAM_ADDR, 0x55, 0);
        bus.write_mem(Z80_RAM_ADDR + 1, 0xAA, 0);

        let mem = mem.borrow();
        assert_eq!(mem.z80_ram[0], 0x55);
        assert_eq!(mem.z80_ram[1], 0xAA);
    }

    #[test]
    fn z80_mrom_write_is_ignored() {
        let (_z80, mem) = new_test_z80();
        mem.borrow_mut().mrom = vec![0x11; 0x100];

        let mut bus = new_test_z80bus(mem.clone());
        bus.write_mem(0x0000, 0xFF, 0);

        assert_eq!(mem.borrow().mrom[0], 0x11);
    }

    #[test]
    fn z80_reads_sound_latch_on_port_00() {
        let (_z80, mem) = new_test_z80();
        mem.borrow_mut().sound_latch = 0x7A;

        let mut bus = new_test_z80bus(mem);
        let (val, ws) = bus.read_io(PORT_SOUND_LATCH, 0);
        assert_eq!(val, 0x7A);
        assert!(ws.is_none());
    }

    #[test]
    fn z80_write_sets_sound_reply_on_port_0c() {
        let (_z80, mem) = new_test_z80();

        let mut bus = new_test_z80bus(mem.clone());
        bus.write_io(PORT_SOUND_REPLY, 0xC3, 0);

        assert_eq!(mem.borrow().z80_sound_reply, 0xC3);
    }

    #[test]
    fn z80_write_to_port_00_or_c0_clears_sound_latch() {
        let (_z80, mem) = new_test_z80();
        mem.borrow_mut().sound_latch = 0x7A;

        let mut bus = new_test_z80bus(mem.clone());
        bus.write_io(PORT_SOUND_LATCH_CLEAR, 0x00, 0);
        assert_eq!(mem.borrow().sound_latch, 0);

        mem.borrow_mut().sound_latch = 0x56;
        bus.write_io(PORT_SOUND_LATCH, 0x00, 0);
        assert_eq!(mem.borrow().sound_latch, 0);
    }

    #[test]
    fn z80_bank_reads_update_memory_offsets_from_port_high_byte() {
        let (_z80, mem) = new_test_z80();

        let mut bus = new_test_z80bus(mem.clone());
        bus.read_io(0x2A00 | PORT_BANK_W0, 0);
        bus.read_io(0x0700 | PORT_BANK_W3, 0);

        let mem = mem.borrow();
        assert_eq!(mem.z80_bank[0], 0x2A * Z80_BANK_WINDOW_0_SIZE);
        assert_eq!(mem.z80_bank[3], 0x07 * Z80_BANK_WINDOW_3_SIZE);
    }

    #[test]
    fn z80_bank_writes_enable_and_disable_nmi() {
        let (_z80, mem) = new_test_z80();

        let mut bus = new_test_z80bus(mem.clone());
        assert!(!mem.borrow().z80_nmi_enabled.get());

        bus.write_io(PORT_BANK_W0, 0x00, 0);
        assert!(mem.borrow().z80_nmi_enabled.get());

        bus.write_io(PORT_NMI_DISABLE, 0x00, 0);
        assert!(!mem.borrow().z80_nmi_enabled.get());
    }

    #[test]
    fn z80_other_bank_ports_do_not_enable_nmi() {
        let mem = Rc::new(RefCell::new(Memory::new()));
        let ym = Rc::new(RefCell::new(Ym2610::new(mem.clone())));
        let mut bus = Z80Bus {
            mem: mem.clone(),
            ym2610: ym,
        };

        for port in [PORT_BANK_W1, PORT_BANK_W2, PORT_BANK_W3] {
            mem.borrow().z80_nmi_enabled.set(false);
            bus.write_io(port, 0x00, 0);
            assert!(
                !mem.borrow().z80_nmi_enabled.get(),
                "write to port 0x{port:02X} must not enable NMI"
            );
        }
    }

    #[test]
    fn z80_io_trace_records_reads_and_writes() {
        let (_z80, mem) = new_test_z80();

        let mut bus = new_test_z80bus(mem.clone());
        bus.write_io(PORT_YM2610_A0, 0x40, 0);
        bus.read_io(PORT_SOUND_LATCH, 0);
        bus.read_io(0x0300 | PORT_BANK_W3, 0);

        let trace = mem.borrow().take_z80_io_trace();
        assert_eq!(trace.len(), 3);
        assert_eq!(trace[0].kind, BusAccessKind::Write);
        assert_eq!(trace[0].port, PORT_YM2610_A0);
        assert_eq!(trace[0].value, 0x40);
        assert_eq!(trace[1].kind, BusAccessKind::Read);
        assert_eq!(trace[1].port, PORT_SOUND_LATCH);
        assert_eq!(trace[2].kind, BusAccessKind::Read);
        assert_eq!(trace[2].port, 0x0300 | PORT_BANK_W3);
        assert_eq!(trace[2].value, 0x00);
    }

    #[test]
    fn z80_banked_mrom_reads_from_correct_window() {
        let (_z80, mem) = new_test_z80();
        let mrom_size = Z80_DEFAULT_BANK_OFFSETS[0] + Z80_BANK_WINDOW_0_SIZE;
        let mut mrom = vec![0x00; mrom_size];
        mrom[Z80_DEFAULT_BANK_OFFSETS[0]] = 0xAA;
        mrom[mrom_size - 1] = 0xBB; // last byte of the 2 KiB window
        mem.borrow_mut().mrom = mrom;

        let bus = new_test_z80bus(mem);

        // Bank window 0 at 0xF000 starts at Geolith's default 0xF000 source offset.
        assert_eq!(bus.read_mem(Z80_BANK_WINDOW_0_ADDR, 0), 0xAA);
        // Last byte of bank window 0
        let last = Z80_BANK_WINDOW_0_ADDR
            .wrapping_add(Z80_BANK_WINDOW_0_SIZE as u16)
            .wrapping_sub(1);
        assert_eq!(bus.read_mem(last, 0), 0xBB);
    }

    #[test]
    fn z80_banked_mrom_reads_selected_port_bank() {
        let (_z80, mem) = new_test_z80();
        let mut mrom = vec![0x00; 0x9000];
        mrom[Z80_DEFAULT_BANK_OFFSETS[3]] = 0x11;
        mrom[0x4000] = 0x44;
        mem.borrow_mut().mrom = mrom;

        let mut bus = new_test_z80bus(mem);
        assert_eq!(bus.read_mem(Z80_BANK_WINDOW_3_ADDR, 0), 0x11);

        bus.read_io(0x0100 | PORT_BANK_W3, 0);
        assert_eq!(bus.read_mem(Z80_BANK_WINDOW_3_ADDR, 0), 0x44);
    }

    #[test]
    fn z80_step_executes_without_crashing() {
        let (mut z80, mem) = new_test_z80();
        // Put a minimal Z80 program: JP $0000 (infinite loop)
        // JP nn = 0xC3, nn nn = 0x00, 0x00
        mem.borrow_mut().mrom = vec![0xC3, 0x00, 0x00];

        let ts = z80.step();
        assert!(ts > 0);
        assert_eq!(z80.pc(), 0x0000);
    }

    #[test]
    fn z80_nmi_is_serviced_when_pending() {
        let (mut z80, mem) = new_test_z80();
        // RST 38H at NMI vector (0x0066): we need some code there.
        // Put JP $0000 at 0x0000 and a JP $0066 loop at 0x0066.
        let mut mrom = vec![0x00; 0x100];
        mrom[0x0000] = 0xC3; // JP $0000
        mrom[0x0001] = 0x00;
        mrom[0x0002] = 0x00;
        mrom[0x0066] = 0xC3; // JP $0066 (NMI handler loop)
        mrom[0x0067] = 0x66;
        mrom[0x0068] = 0x00;
        mem.borrow_mut().mrom = mrom;

        // Trigger NMI
        mem.borrow_mut().z80_nmi_pending.set(true);

        // Step — should jump to NMI handler at 0x0066
        let ts = z80.step();
        assert!(ts > 0);
        assert_eq!(z80.pc(), 0x0066);
        assert!(!mem.borrow().z80_nmi_pending.get());

        // Step again — NMI handler jumps to itself
        z80.step();
        assert_eq!(z80.pc(), 0x0066);
    }

    #[test]
    fn z80_run_tstates_executes_for_requested_duration() {
        let (mut z80, mem) = new_test_z80();
        // JP $0000 (infinite loop) — 10 T-states per JP
        mem.borrow_mut().mrom = vec![0xC3, 0x00, 0x00];

        let executed = z80.run_tstates(100);
        assert!(executed >= 100);
        assert!(executed < 120); // Should be close to 100 (each JP is 10 T-states)
    }

    #[test]
    fn z80_halt_keeps_consuming_idle_tstates_for_ym_timers() {
        let (mut z80, mem) = new_test_z80();
        mem.borrow_mut().mrom = vec![0x76]; // HALT

        let first = z80.step();
        assert!(first > 0, "HALT instruction itself must consume cycles");
        assert!(z80.is_halt());

        let second = z80.step();
        assert_eq!(
            second, HALT_IDLE_TSTATES,
            "HALTed Z80 must keep consuming idle cycles so YM2610 timers advance"
        );

        let executed = z80.run_tstates(100);
        assert!(executed >= 100);
        assert!(
            executed <= 100 + HALT_IDLE_TSTATES,
            "HALT idle catch-up should stay close to the requested slice"
        );
    }

    #[test]
    fn z80_reset_restarts_cpu() {
        let (mut z80, mem) = new_test_z80();
        mem.borrow_mut().mrom = vec![0xC3, 0x00, 0x00];

        // Execute a few steps
        z80.step();
        z80.step();

        // Reset
        z80.reset();
        // PC should be 0 after reset (Z80 starts executing from 0x0000)
        assert_eq!(z80.pc(), 0x0000);
    }

    #[test]
    fn z80_sound_reply_accepts_zero_writes_like_geolith() {
        let (_z80, mem) = new_test_z80();
        mem.borrow_mut().z80_sound_reply = 0xC3;

        let mut bus = new_test_z80bus(mem.clone());
        bus.write_io(PORT_SOUND_REPLY, 0x00, 0);

        assert_eq!(mem.borrow().z80_sound_reply, 0x00);

        // Non-zero replies still pass through normally
        bus.write_io(PORT_SOUND_REPLY, 0x01, 0);
        assert_eq!(mem.borrow().z80_sound_reply, 0x01);
    }

    #[test]
    fn z80_unhandled_aux_ports_read_open_zero_like_geolith() {
        let (_z80, mem) = new_test_z80();
        let mut bus = new_test_z80bus(mem);

        let (aux, _) = bus.read_io(0x000E, 0);
        assert_eq!(aux, 0x00);

        let (unknown, _) = bus.read_io(0x1234, 0);
        assert_eq!(unknown, 0x00);
    }

    #[test]
    fn z80_ym2610_address_ports_are_status() {
        let mem = Rc::new(RefCell::new(Memory::new()));
        let mut bus = new_test_z80bus(mem.clone());

        // YM_A0 read returns status, not the latched address.
        // Initially status should be 0 (no busy, no timers, no flags).
        let (status, _) = bus.read_io(PORT_YM2610_A0, 0);
        assert_eq!(status, 0x00, "YM2610 status should be clear after reset");

        // YM_A1 read returns ADPCM port status
        let (status1, _) = bus.read_io(PORT_YM2610_A1, 0);
        assert_eq!(
            status1, 0x00,
            "YM2610 port 1 status should be clear after reset"
        );

        // Timer flags appear in status when triggered via register write.
        // Write timer A load, then enable with reset flag set (= 0x15: enable A + load + reset flags)
        bus.write_io(PORT_YM2610_A0, 0x24, 0); // address
        bus.write_io(PORT_YM2610_D0, 0x12, 0); // timer A high
        bus.write_io(PORT_YM2610_A0, 0x25, 0); // address
        bus.write_io(PORT_YM2610_D0, 0x34, 0); // timer A low
        bus.write_io(PORT_YM2610_A0, 0x27, 0); // address
        bus.write_io(PORT_YM2610_D0, 0x15, 0); // enable TA + load TA + reset flags

        // Status should still be 0 since timers haven't elapsed yet
        let (status, _) = bus.read_io(PORT_YM2610_A0, 0);
        assert_eq!(status, 0x00, "Timer flags should be clear after reset");
    }

    #[test]
    fn z80_ym2610_ports_are_connected() {
        let mem = Rc::new(RefCell::new(Memory::new()));
        let mut bus = new_test_z80bus(mem.clone());

        // SSG data registers on port 0 are readable.
        bus.write_io(PORT_YM2610_A0, 0x08, 0);
        bus.write_io(PORT_YM2610_D0, 0x0F, 0);

        // Read back by re-latching address
        mem.borrow_mut().ym2610_addr[0] = 0x08;
        let (val0, _) = bus.read_io(PORT_YM2610_D0, 0);
        assert_eq!(val0, 0x0F, "YM2610 SSG readback failed");

        // Port 1 data reads are open zero on YM2610; status is on A1.
        bus.write_io(PORT_YM2610_A1, 0xB0, 0);
        bus.write_io(PORT_YM2610_D1, 0x07, 0);
        mem.borrow_mut().ym2610_addr[1] = 0xB0;
        let (val1, _) = bus.read_io(PORT_YM2610_D1, 0);
        assert_eq!(val1, 0x00, "YM2610 port 1 data reads should be open zero");
    }

    #[test]
    fn z80_handles_empty_mrom_gracefully() {
        let mem = Rc::new(RefCell::new(Memory::new()));
        // M-ROM is empty by default
        let bus = new_test_z80bus(mem);

        // All reads should return open bus value
        assert_eq!(bus.read_mem(0x0000, 0), 0xFF);
        assert_eq!(bus.read_mem(0x7FFF, 0), 0xFF);
        assert_eq!(bus.read_mem(Z80_BANK_WINDOW_3_ADDR, 0), 0xFF);
        assert_eq!(bus.read_mem(Z80_RAM_ADDR, 0), 0x00);
    }

    #[test]
    fn z80_bus_read_debug_matches_read_mem() {
        let (_z80, mem) = new_test_z80();
        mem.borrow_mut().mrom = vec![0xA5; 0x100];

        let bus = new_test_z80bus(mem);
        assert_eq!(bus.read_debug(0x0042), 0xA5);
    }

    #[test]
    fn z80_bus_read_mem16_is_little_endian() {
        let (_z80, mem) = new_test_z80();
        mem.borrow_mut().mrom = vec![0x12, 0x34];

        let bus = new_test_z80bus(mem);
        assert_eq!(bus.read_mem16(0x0000, 0), 0x3412);
    }

    #[test]
    fn z80_bus_read_opcode_works_like_read_mem() {
        let (_z80, mem) = new_test_z80();
        mem.borrow_mut().mrom = vec![0xC3, 0x00, 0x00];

        let mut bus = new_test_z80bus(mem);
        assert_eq!(bus.read_opcode(0x0000, 0x0000, 0), 0xC3);
    }
}

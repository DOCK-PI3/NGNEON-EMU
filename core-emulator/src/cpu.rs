//! 68000 CPU emulation via Musashi (C, compiled as static library).
//!
//! Musashi is linked via FFI declarations in `musashi_ffi`.  Memory
//! callbacks are routed through `musashi_ffi::set_active_memory()`
//! before every `m68k_execute()` call.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(test)]
use std::sync::Mutex;

use crate::musashi_ffi;

/// Guards access to Musashi's global `m68ki_cpu` struct.
/// Musashi (the C emulator) stores ALL CPU state in a single global
/// struct. When multiple tests run in parallel, they corrupt each other's
/// state, causing flaky test failures. This Mutex serializes all access
/// to Musashi functions. In production (single-threaded) it's a no-op.
#[cfg(test)]
pub(crate) static MUSASHI_LOCK: Mutex<()> = Mutex::new(());

// ─── CPU state snapshot ─────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuSnapshot {
    pub pc: u32,
    pub sr: u16,
    pub ssp: u32,
    pub usp: u32,
}

// ─── CPU wrapper ────────────────────────────────────────────────────────

pub struct Cpu {
    mem: Rc<RefCell<crate::memory::Memory>>,
    /// Cache: Musashi register context for save/restore.
    /// Allocated once from m68k_context_size().
    ctx: Vec<u8>,
}

// Musashi uses global C state; init must happen only once.
static MUSASHI_INITIALIZED: AtomicBool = AtomicBool::new(false);

impl Cpu {
    pub fn new(mem: Rc<RefCell<crate::memory::Memory>>) -> Self {
        // Initialise Musashi core — only once.
        if !MUSASHI_INITIALIZED.swap(true, Ordering::SeqCst) {
            unsafe {
                musashi_ffi::m68k_init();
                // Explicitly set plain 68000 mode (value 1 = M68K_CPU_TYPE_68000 from m68k.h)
                musashi_ffi::m68k_set_cpu_type(1);
                // Wire up interrupt acknowledge callback so that virq state
                // is properly cleared when an interrupt is serviced.
                musashi_ffi::m68k_set_int_ack_callback(Some(musashi_ffi::m68k_int_ack_callback));
            }
        }

        // Allocate buffer for register serialization (24 regs × 4 bytes)
        let ctx = vec![0u8; Self::REGS.len() * 4];

        Self { mem, ctx }
    }

    /// Reset the CPU — Musashi reads the initial SSP/PC from memory
    /// (addresses 0 and 4, which the NeoGeo routes to BIOS vectors).
    /// Must set the active memory pointer BEFORE calling m68k_pulse_reset().
    pub fn reset(&mut self) {
        #[cfg(test)]
        let _guard = MUSASHI_LOCK.lock().unwrap();
        let mem_ptr: *mut crate::memory::Memory = &mut *self.mem.borrow_mut();
        musashi_ffi::set_active_memory(mem_ptr);

        unsafe {
            musashi_ffi::m68k_pulse_reset();
        }
    }

    /// Run the CPU for `cycles` instructions / bus cycles.
    /// Returns the actual number of cycles executed (or an error if halted).
    pub fn run_cycles(&mut self, cycles: i32) -> Result<i32, String> {
        #[cfg(test)]
        let _guard = MUSASHI_LOCK.lock().unwrap();
        // Point Musashi's memory callbacks at our Memory
        let mem_ptr: *mut crate::memory::Memory = &mut *self.mem.borrow_mut();
        musashi_ffi::set_active_memory(mem_ptr);

        let executed = unsafe { musashi_ffi::m68k_execute(cycles) };

        // Musashi does not expose an explicit "is halted" check.
        // If the CPU executed 0 cycles, treat it as a halt (double fault).
        if executed == 0 {
            let pc = unsafe {
                musashi_ffi::m68k_get_reg(std::ptr::null_mut(), musashi_ffi::M68K_REG_PC)
            };
            return Err(format!("CPU halted (double fault?) at PC=0x{pc:08X}"));
        }

        Ok(executed)
    }

    /// Request an interrupt at the given level (1–7).
    /// Uses Musashi's virtual IRQ system so that multiple concurrent
    /// interrupt sources are tracked independently; the highest-priority
    /// active source is automatically asserted on the CPU.
    pub fn request_interrupt(&mut self, level: u8) -> Result<(), String> {
        #[cfg(test)]
        let _guard = MUSASHI_LOCK.lock().unwrap();
        if !(1..=7).contains(&level) {
            return Err(format!("Nivel de interrupción inválido: {level}"));
        }
        unsafe {
            musashi_ffi::m68k_set_virq(level as u32, 1);
        }
        Ok(())
    }

    /// Single-step (1 cycle).
    pub fn step(&mut self) -> Result<i32, String> {
        self.run_cycles(1)
    }

    /// Take a snapshot of the current CPU state.
    pub fn snapshot(&self) -> CpuSnapshot {
        #[cfg(test)]
        let _guard = MUSASHI_LOCK.lock().unwrap();
        // Musashi context is saved into self.ctx
        // We can read registers directly from the live core
        // using m68k_get_reg with null context for the current state.
        unsafe {
            let pc = musashi_ffi::m68k_get_reg(std::ptr::null_mut(), musashi_ffi::M68K_REG_PC);
            let sr =
                musashi_ffi::m68k_get_reg(std::ptr::null_mut(), musashi_ffi::M68K_REG_SR) as u16;
            let ssp = musashi_ffi::m68k_get_reg(std::ptr::null_mut(), musashi_ffi::M68K_REG_ISP);
            let usp = musashi_ffi::m68k_get_reg(std::ptr::null_mut(), musashi_ffi::M68K_REG_USP);
            CpuSnapshot { pc, sr, ssp, usp }
        }
    }

    /// Save all Musashi registers into an opaque byte buffer.
    ///
    /// Uses m68k_get_reg() for each register instead of the raw context
    /// pointer, because m68k_set_context() has proven crash-prone across
    /// sequential Cpu instances in the same process.
    pub fn save_context(&mut self) -> &[u8] {
        #[cfg(test)]
        let _guard = MUSASHI_LOCK.lock().unwrap();
        let count = Self::REGS.len();
        let needed = count * 4;
        self.ctx.clear();
        self.ctx.reserve(needed);
        for reg in &Self::REGS {
            let val = unsafe { musashi_ffi::m68k_get_reg(std::ptr::null_mut(), *reg) };
            self.ctx.extend_from_slice(&val.to_le_bytes());
        }
        &self.ctx
    }

    /// Restore all Musashi registers from a previously saved byte buffer.
    pub fn restore_context(&mut self, data: &[u8]) {
        #[cfg(test)]
        let _guard = MUSASHI_LOCK.lock().unwrap();
        self.deserialize_regs_from(data);
    }

    /// Read a register directly (for debugging / state inspection).
    pub fn get_reg(&self, reg: musashi_ffi::M68kReg) -> u32 {
        #[cfg(test)]
        let _guard = MUSASHI_LOCK.lock().unwrap();
        unsafe { musashi_ffi::m68k_get_reg(std::ptr::null_mut(), reg) }
    }

    /// Set a register directly (for debugging / state restoration).
    pub fn set_reg(&mut self, reg: musashi_ffi::M68kReg, value: u32) {
        #[cfg(test)]
        let _guard = MUSASHI_LOCK.lock().unwrap();
        unsafe {
            musashi_ffi::m68k_set_reg(reg, value);
        }
    }

    // ─── Serialization helpers ────────────────────────────────────────

    /// All Musashi registers that we track for save/restore.
    /// Order must stay in sync between serialize and deserialize.
    /// Includes D0-D7, A0-A7, PC, SR, USP, ISP, SFC, DFC, VBR, and prefetch address.
    const REGS: [musashi_ffi::M68kReg; 24] = [
        musashi_ffi::M68K_REG_D0,
        musashi_ffi::M68K_REG_D1,
        musashi_ffi::M68K_REG_D2,
        musashi_ffi::M68K_REG_D3,
        musashi_ffi::M68K_REG_D4,
        musashi_ffi::M68K_REG_D5,
        musashi_ffi::M68K_REG_D6,
        musashi_ffi::M68K_REG_D7,
        musashi_ffi::M68K_REG_A0,
        musashi_ffi::M68K_REG_A1,
        musashi_ffi::M68K_REG_A2,
        musashi_ffi::M68K_REG_A3,
        musashi_ffi::M68K_REG_A4,
        musashi_ffi::M68K_REG_A5,
        musashi_ffi::M68K_REG_A6,
        musashi_ffi::M68K_REG_A7,
        musashi_ffi::M68K_REG_PC,
        musashi_ffi::M68K_REG_SR,
        musashi_ffi::M68K_REG_USP,
        musashi_ffi::M68K_REG_ISP,
        musashi_ffi::M68K_REG_SFC,
        musashi_ffi::M68K_REG_DFC,
        musashi_ffi::M68K_REG_VBR,
        musashi_ffi::M68K_REG_PREF_ADDR,
    ];

    fn deserialize_regs_from(&mut self, data: &[u8]) {
        let count = Self::REGS.len();
        if data.len() < count * 4 {
            return;
        }
        for (i, reg) in Self::REGS.iter().enumerate() {
            let off = i * 4;
            let val = u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]]);
            unsafe {
                musashi_ffi::m68k_set_reg(*reg, val);
            }
        }
    }
}

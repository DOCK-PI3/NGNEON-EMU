//! FFI bindings to the Musashi 68000 C emulator.
//!
//! Musashi is compiled as a static library via `build.rs`.  The C functions
//! are declared here as `extern "C"` and linked at the crate level.
//!
//! Memory-access callbacks (m68k_read_memory_8, m68k_write_memory_8, etc.)
//! are implemented as `extern "C"` functions in this module.  They use a
//! `thread_local!` cell to access the `Memory` instance.

use std::cell::RefCell;

// ─── Thread-local pointer to the active Memory ───────────────────────────

// Musashi uses C callbacks (no context parameter) so we store the pointer
// in a thread-local cell.  The pointer is set before every `run_cycles()`
// call and must not be used concurrently (the emulator is single-threaded).
thread_local! {
    static ACTIVE_MEMORY: RefCell<*mut crate::memory::Memory> = const {
        RefCell::new(std::ptr::null_mut())
    };
}

/// Set the active Memory pointer that the C callbacks will read from/write to.
/// **Must be called before every `m68k_execute()` invocation.**
pub(crate) fn set_active_memory(mem: *mut crate::memory::Memory) {
    ACTIVE_MEMORY.with(|cell| *cell.borrow_mut() = mem);
}

// ─── Musashi public API (from m68k.h) ───────────────────────────────────

extern "C" {
    pub fn m68k_init();
    pub fn m68k_set_cpu_type(cputype: u32);
    pub fn m68k_pulse_reset();
    pub fn m68k_execute(cycles: i32) -> i32;
    pub fn m68k_set_irq(level: u32);
    pub fn m68k_set_virq(level: u32, active: u32);
    pub fn m68k_get_virq(level: u32) -> u32;
    pub fn m68k_set_int_ack_callback(callback: Option<unsafe extern "C" fn(i32) -> i32>);
    pub fn m68k_get_reg(ctx: *mut u8, reg: M68kReg) -> u32;
    pub fn m68k_set_reg(reg: M68kReg, value: u32);
}

// M68K_INT_ACK_AUTOVECTOR = -1
pub const M68K_INT_ACK_AUTOVECTOR: i32 = -1;

/// Interrupt acknowledge callback: clears the virq bit for the serviced
/// level so that the next-highest pending interrupt can be asserted.
///
/// # Safety
///
/// Called only by Musashi while the emulator has installed this callback.
/// `int_level` must be a valid Musashi interrupt level.
pub unsafe extern "C" fn m68k_int_ack_callback(int_level: i32) -> i32 {
    // Clear the virq line for this level; m68k_set_virq will
    // re-assert the next-highest active level if any remain.
    m68k_set_virq(int_level as u32, 0);
    M68K_INT_ACK_AUTOVECTOR
}

// ─── Register enum from m68k.h ──────────────────────────────────────────

// Register identifier type. Matches the `m68k_register_t` typedef from m68k.h.
pub type M68kReg = u32;

pub const M68K_REG_D0: M68kReg = 0;
pub const M68K_REG_D1: M68kReg = 1;
pub const M68K_REG_D2: M68kReg = 2;
pub const M68K_REG_D3: M68kReg = 3;
pub const M68K_REG_D4: M68kReg = 4;
pub const M68K_REG_D5: M68kReg = 5;
pub const M68K_REG_D6: M68kReg = 6;
pub const M68K_REG_D7: M68kReg = 7;
pub const M68K_REG_A0: M68kReg = 8;
pub const M68K_REG_A1: M68kReg = 9;
pub const M68K_REG_A2: M68kReg = 10;
pub const M68K_REG_A3: M68kReg = 11;
pub const M68K_REG_A4: M68kReg = 12;
pub const M68K_REG_A5: M68kReg = 13;
pub const M68K_REG_A6: M68kReg = 14;
pub const M68K_REG_A7: M68kReg = 15;
pub const M68K_REG_PC: M68kReg = 16;
pub const M68K_REG_SR: M68kReg = 17;
pub const M68K_REG_SP: M68kReg = 18;
pub const M68K_REG_USP: M68kReg = 19;
pub const M68K_REG_ISP: M68kReg = 20;
pub const M68K_REG_MSP: M68kReg = 21;
pub const M68K_REG_SFC: M68kReg = 22;
pub const M68K_REG_DFC: M68kReg = 23;
pub const M68K_REG_VBR: M68kReg = 24;
pub const M68K_REG_CACR: M68kReg = 25;
pub const M68K_REG_CAAR: M68kReg = 26;
pub const M68K_REG_PREF_ADDR: M68kReg = 27;

// ─── Memory callbacks (implemented in Rust, called from C) ──────────────

/// Get the active Memory pointer, or null.
fn active_mem_ptr() -> *mut crate::memory::Memory {
    ACTIVE_MEMORY.with(|cell| *cell.borrow())
}

// These functions have C linkage so Musashi can call them.
//
// IMPORTANT: Each callback accesses the thread-local Memory pointer ONCE,
// then reuses it for all byte accesses.  This avoids the overhead of
// a thread-local lookup + null check per byte.

#[no_mangle]
/// Read one byte from the active 68K bus.
///
/// # Safety
///
/// Called by Musashi's C core. The active memory pointer must have been set
/// with `set_active_memory` for the current emulation step.
pub unsafe extern "C" fn m68k_read_memory_8(address: u32) -> u32 {
    let ptr = active_mem_ptr();
    if ptr.is_null() {
        return 0xFF;
    }
    (*ptr).read8(address) as u32
}

#[no_mangle]
/// Read one word from the active 68K bus.
///
/// # Safety
///
/// Called by Musashi's C core. The active memory pointer must have been set
/// with `set_active_memory` for the current emulation step.
pub unsafe extern "C" fn m68k_read_memory_16(address: u32) -> u32 {
    let ptr = active_mem_ptr();
    if ptr.is_null() {
        return 0xFFFF;
    }
    (*ptr).read16(address) as u32
}

#[no_mangle]
/// Read one longword from the active 68K bus.
///
/// # Safety
///
/// Called by Musashi's C core. The active memory pointer must have been set
/// with `set_active_memory` for the current emulation step.
pub unsafe extern "C" fn m68k_read_memory_32(address: u32) -> u32 {
    let ptr = active_mem_ptr();
    if ptr.is_null() {
        return 0xFFFFFFFF;
    }
    (*ptr).read32(address)
}

#[no_mangle]
/// Write one byte to the active 68K bus.
///
/// # Safety
///
/// Called by Musashi's C core. The active memory pointer must have been set
/// with `set_active_memory` for the current emulation step.
pub unsafe extern "C" fn m68k_write_memory_8(address: u32, value: u32) {
    let ptr = active_mem_ptr();
    if ptr.is_null() {
        return;
    }
    (*ptr).write8(address, value as u8);
}

#[no_mangle]
/// Write one word to the active 68K bus.
///
/// # Safety
///
/// Called by Musashi's C core. The active memory pointer must have been set
/// with `set_active_memory` for the current emulation step.
pub unsafe extern "C" fn m68k_write_memory_16(address: u32, value: u32) {
    let ptr = active_mem_ptr();
    if ptr.is_null() {
        return;
    }
    (*ptr).write16(address, value as u16);
}

#[no_mangle]
/// Write one longword to the active 68K bus.
///
/// # Safety
///
/// Called by Musashi's C core. The active memory pointer must have been set
/// with `set_active_memory` for the current emulation step.
pub unsafe extern "C" fn m68k_write_memory_32(address: u32, value: u32) {
    let ptr = active_mem_ptr();
    if ptr.is_null() {
        return;
    }
    (*ptr).write32(address, value);
}

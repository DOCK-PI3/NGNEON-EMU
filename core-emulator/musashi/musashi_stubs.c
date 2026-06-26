/* musashi_stubs.c
 *
 * Stub implementations for FPU/MMU symbols that the generated m68kops.c
 * references even when FPU/MMU emulation is disabled (M68K_EMULATE_FPU=0,
 * M68K_EMULATE_MMU=0).
 *
 * These functions should never actually be called at runtime because the
 * opcode dispatch table only dispatches to them when the CPU type supports
 * FPU/MMU (68040/68060).  We provide them only to satisfy the linker.
 */

/* The m68kops.c dispatch table references these arrays when it is generated
 * with m68kmake for the 68040 code path, even if the runtime config disables
 * FPU.  We define them as empty (zero-length) so the linker is happy. */

const int m68040_fpu_op0[1] = {0};
const int m68040_fpu_op1[1] = {0};
const int m68881_mmu_ops[1] = {0};

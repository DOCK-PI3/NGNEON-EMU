/* ======================================================================== */
/* ========================= LICENSING & COPYRIGHT ======================== */
/* ======================================================================== */
/*
* MUSASHI
* Version 3.32
*
* A portable Motorola M680x0 processor emulation engine.
* Copyright Karl Stenerud. All rights reserved.
*
* Permission is hereby granted, free of charge, to any person obtaining a copy
* of this software and associated documentation files (the "Software"), to deal
* in the Software without restriction, including without limitation the rights
* to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
* copies of the Software, and to permit persons to whom the Software is
* furnished to do so, subject to the following conditions:
*
* The above copyright notice and this permission notice shall be included in
* all copies or substantial portions of the Software.
* THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
* IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
* FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
* AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
* LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
* OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
* THE SOFTWARE.
*/

/* ======================================================================== */
/* ========================= NGNEON-EMU CONFIGURATION ===================== */
/* ======================================================================== */
/*
* Tailored configuration for NeoGeo emulation:
*   - Plain 68000 only (no 010/020/030/040)
*   - No FPU, no MMU
*   - Separate memory handlers for 8/16/32-bit access
*   - No prefetch queue emulation (simplified)
*   - Address error exceptions ON (NeoGeo uses these)
*/

#ifndef M68KCONF__HEADER
#define M68KCONF__HEADER

/* ---- Option constants ------------------------------------------------ */
#define M68K_OPT_OFF              0
#define M68K_OPT_ON               1
#define M68K_OPT_SPECIFY_HANDLER  2

/* ---- CPU variants (all OFF except plain 68000) ----------------------- */

/* M68K Emulation variants: disable everything beyond plain 68000. */
#ifndef M68K_EMULATE_010
#define M68K_EMULATE_010          M68K_OPT_OFF
#endif
#ifndef M68K_EMULATE_020
#define M68K_EMULATE_020          M68K_OPT_OFF
#endif
#ifndef M68K_EMULATE_030
#define M68K_EMULATE_030          M68K_OPT_OFF
#endif
#ifndef M68K_EMULATE_040
#define M68K_EMULATE_040          M68K_OPT_OFF
#endif
#ifndef M68K_EMULATE_EC020
#define M68K_EMULATE_EC020        M68K_OPT_OFF
#endif

/* ---- Memory and addressing ------------------------------------------- */

/* Separate reads (immediate/PC-relative): off — route everything
 * through the standard read callbacks. */
#ifndef M68K_SEPARATE_READS
#define M68K_SEPARATE_READS       M68K_OPT_OFF
#endif

/* Simulate predecrement write ordering: off — write_32_pd = write_32. */
#ifndef M68K_SIMULATE_PD_WRITES
#define M68K_SIMULATE_PD_WRITES   M68K_OPT_OFF
#endif

/* ---- Interrupts ------------------------------------------------------ */

/* Interrupt acknowledge: on — custom callback clears virq state
 * and returns autovector (NeoGeo uses autovectors). */
#ifndef M68K_EMULATE_INT_ACK
#define M68K_EMULATE_INT_ACK      M68K_OPT_ON
#endif

/* Breakpoint acknowledge: off. */
#ifndef M68K_EMULATE_BKPT_ACK
#define M68K_EMULATE_BKPT_ACK     M68K_OPT_OFF
#endif

/* ---- Exceptions & tracing -------------------------------------------- */

/* Trace: off. */
#ifndef M68K_EMULATE_TRACE
#define M68K_EMULATE_TRACE        M68K_OPT_OFF
#endif

/* Address error: ON — NeoGeo catches word/long access at odd addresses. */
#ifndef M68K_EMULATE_ADDRESS_ERROR
#define M68K_EMULATE_ADDRESS_ERROR M68K_OPT_ON
#endif

/* ---- Prefetch -------------------------------------------------------- */

/* Prefetch queue: off — simplified. */
#ifndef M68K_EMULATE_PREFETCH
#define M68K_EMULATE_PREFETCH     M68K_OPT_OFF
#endif

/* ---- Callbacks (all off — no custom handler hooks) ------------------- */

#ifndef M68K_EMULATE_RESET
#define M68K_EMULATE_RESET        M68K_OPT_OFF
#endif

#ifndef M68K_CMPILD_HAS_CALLBACK
#define M68K_CMPILD_HAS_CALLBACK  M68K_OPT_OFF
#endif

#ifndef M68K_RTE_HAS_CALLBACK
#define M68K_RTE_HAS_CALLBACK     M68K_OPT_OFF
#endif

#ifndef M68K_TAS_HAS_CALLBACK
#define M68K_TAS_HAS_CALLBACK     M68K_OPT_OFF
#endif

#ifndef M68K_ILLG_HAS_CALLBACK
#define M68K_ILLG_HAS_CALLBACK    M68K_OPT_OFF
#endif

#ifndef M68K_TRAP_HAS_CALLBACK
#define M68K_TRAP_HAS_CALLBACK    M68K_OPT_OFF
#endif

/* ---- Function code & PC monitoring ----------------------------------- */

#ifndef M68K_EMULATE_FC
#define M68K_EMULATE_FC           M68K_OPT_OFF
#endif

#ifndef M68K_MONITOR_PC
#define M68K_MONITOR_PC           M68K_OPT_OFF
#endif

#ifndef M68K_INSTRUCTION_HOOK
#define M68K_INSTRUCTION_HOOK     M68K_OPT_OFF
#endif

/* ---- PMMU ------------------------------------------------------------ */

#ifndef M68K_EMULATE_PMMU
#define M68K_EMULATE_PMMU         M68K_OPT_OFF
#endif

/* ---- Logging --------------------------------------------------------- */

#ifndef M68K_LOG_ENABLE
#define M68K_LOG_ENABLE           M68K_OPT_OFF
#define M68K_LOG_1010_1111        M68K_OPT_OFF
#define M68K_LOG_TRAP             M68K_OPT_OFF
#endif

/* ---- Performance ----------------------------------------------------- */

/* Use 64-bit integers where helpful. */
#ifndef M68K_USE_64_BIT
#define M68K_USE_64_BIT           M68K_OPT_ON
#endif

/* ---- CPU type (explicit for m68kops.c dispatch) ---------------------- */

/* The CPU type is set at runtime via m68k_set_cpu_type().
 * We do NOT define M68K_CPU_TYPE here — Musashi handles it through
 * the enum in m68k.h and the m68k_set_cpu_type() call. */

/* ---- Include the opcode function table ------------------------------- */
#include "m68kops.h"

#endif /* M68KCONF__HEADER */

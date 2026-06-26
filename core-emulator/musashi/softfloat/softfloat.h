/* softfloat.h — stub for standalone Musashi builds.
 *
 * Only the floatx80 type is required to compile the m68ki_cpu struct.
 * FPU emulation is disabled (M68K_EMULATE_FPU=0), so no actual FPU
 * operations are compiled in.
 */
#ifndef _SOFTFLOAT_H_
#define _SOFTFLOAT_H_

#include "milieu.h"

/* 80-bit extended-precision float (used for 68000-series FPU regs) */
typedef struct {
    uint64_t low;    /* mantissa */
    uint16_t high;   /* sign + exponent */
} floatx80;

#endif /* _SOFTFLOAT_H_ */

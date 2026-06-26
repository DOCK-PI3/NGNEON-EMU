/* milieu.h — stub for standalone Musashi builds.
 *
 * The softfloat library is only needed when FPU emulation is enabled
 * (M68K_EMULATE_FPU).  Since we configure M68K_EMULATE_FPU=0, the
 * floatx80 type declaration is enough to satisfy the struct layout.
 */
#ifndef _MILIEU_H_
#define _MILIEU_H_

/* Integer types (from stdint) */
#include <stdint.h>
typedef int8_t          int8;
typedef int16_t         int16;
typedef int32_t         int32;
typedef int64_t         int64;
typedef uint8_t         uint8;
typedef uint16_t        uint16;
typedef uint32_t        uint32;
typedef uint64_t        uint64;

/* Softfloat flag */
typedef uint8_t         flag;

#endif /* _MILIEU_H_ */

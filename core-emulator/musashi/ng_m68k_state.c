#include "m68kcpu.h"

unsigned ng_m68k_get_int_level(void)
{
    return m68ki_cpu.int_level;
}

unsigned ng_m68k_get_virq_state(void)
{
    return m68ki_cpu.virq_state;
}

unsigned ng_m68k_get_nmi_pending(void)
{
    return m68ki_cpu.nmi_pending;
}

void ng_m68k_set_runtime_state(unsigned int_level,
                               unsigned virq_state,
                               unsigned nmi_pending)
{
    m68ki_cpu.int_level = int_level;
    m68ki_cpu.virq_state = virq_state;
    m68ki_cpu.nmi_pending = nmi_pending;
}

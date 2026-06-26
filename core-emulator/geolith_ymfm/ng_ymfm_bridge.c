/*
 * Thin NGNEON bridge for Geolith's ymfm YM2610 implementation.
 *
 * The bundled ymfm sources are BSD-3-Clause licensed by their original
 * authors. This file only provides the host callbacks required by ymfm and a
 * small C ABI for the Rust core.
 */

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "ymfm_adpcm.h"
#include "ymfm_opn.h"
#include "ymfm_ssg.h"

#define NG_YMFM_TIMER_DIVISOR 144

static const uint8_t *g_v1;
static const uint8_t *g_v2;
static size_t g_v1_size;
static size_t g_v2_size;
static int32_t g_busy_timer;
static int32_t g_timer[2] = { -1, -1 };
static bool g_irq_asserted;

static inline int16_t ng_mix_clip(int32_t a, int32_t b)
{
	int32_t mixed = a + b;
	if (mixed > 32767)
		return 32767;
	if (mixed < -32768)
		return -32768;
	return (int16_t)mixed;
}

static inline void ng_timer_tick(void)
{
	if (g_busy_timer > 0)
	{
		g_busy_timer -= NG_YMFM_TIMER_DIVISOR;
		if (g_busy_timer < 0)
			g_busy_timer = 0;
	}

	for (int i = 0; i < 2; ++i)
	{
		if (g_timer[i] < 0)
			continue;
		g_timer[i] -= NG_YMFM_TIMER_DIVISOR;
		if (g_timer[i] <= 0)
			fm_engine_timer_expired((uint32_t)i);
	}
}

void ng_ymfm_init(void)
{
	ym2610_init();
	ym2610_set_fidelity(OPN_FIDELITY_MED);
	fm_engine_init();
	g_busy_timer = 0;
	g_timer[0] = g_timer[1] = -1;
	g_irq_asserted = false;
}

void ng_ymfm_reset(void)
{
	ym2610_reset();
	g_busy_timer = 0;
	g_timer[0] = g_timer[1] = -1;
	g_irq_asserted = false;
}

void ng_ymfm_set_roms(const uint8_t *v1, size_t v1_size, const uint8_t *v2, size_t v2_size)
{
	g_v1 = v1;
	g_v2 = v2;
	g_v1_size = v1_size;
	g_v2_size = v2_size;
}

uint8_t ng_ymfm_read(uint32_t offset)
{
	return ym2610_read(offset);
}

void ng_ymfm_write(uint32_t offset, uint8_t data)
{
	ym2610_write(offset, data);
}

void ng_ymfm_generate(int16_t *dst, size_t sample_pairs)
{
	for (size_t i = 0; i < sample_pairs; ++i)
	{
		int32_t output[3] = { 0, 0, 0 };
		ng_timer_tick();
		ym2610_generate(output);
		dst[i * 2] = ng_mix_clip(output[0], output[2]);
		dst[i * 2 + 1] = ng_mix_clip(output[1], output[2]);
	}
}

uint8_t ng_ymfm_irq_asserted(void)
{
	return g_irq_asserted ? 1 : 0;
}

int32_t ng_ymfm_timer_remaining(uint32_t tnum)
{
	return tnum < 2 ? g_timer[tnum] : -1;
}

int32_t ng_ymfm_busy_remaining(void)
{
	return g_busy_timer;
}

void ng_ymfm_adpcm_wrap(int wrap)
{
	adpcm_a_set_accum_wrap(wrap != 0);
}

size_t ng_ymfm_state_save(uint8_t *dst, size_t capacity)
{
	(void)capacity;
	if (!dst)
		return 0;

	geo_serial_begin();
	geo_serial_push32(dst, (uint32_t)g_busy_timer);
	geo_serial_push32(dst, (uint32_t)g_timer[0]);
	geo_serial_push32(dst, (uint32_t)g_timer[1]);
	opn_state_save(dst);
	adpcm_state_save(dst);
	ssg_state_save(dst);
	return geo_serial_size();
}

void ng_ymfm_state_load(const uint8_t *src, size_t size)
{
	if (!src || size == 0)
		return;

	geo_serial_begin();
	uint8_t *state = (uint8_t *)src;
	g_busy_timer = (int32_t)geo_serial_pop32(state);
	g_timer[0] = (int32_t)geo_serial_pop32(state);
	g_timer[1] = (int32_t)geo_serial_pop32(state);
	opn_state_load(state);
	adpcm_state_load(state);
	ssg_state_load(state);
	fm_engine_check_interrupts();
}

uint8_t ymfm_external_read(uint32_t type, uint32_t address)
{
	switch (type)
	{
		case ACCESS_ADPCM_A:
			return (g_v1 && address < g_v1_size) ? g_v1[address] : 0;
		case ACCESS_ADPCM_B:
			return (g_v2 && address < g_v2_size) ? g_v2[address] : 0;
		default:
			return 0;
	}
}

void ymfm_external_write(uint32_t type, uint32_t address, uint8_t data)
{
	(void)type;
	(void)address;
	(void)data;
}

void ymfm_sync_mode_write(uint8_t data)
{
	fm_engine_mode_write(data);
}

void ymfm_sync_check_interrupts(void)
{
	fm_engine_check_interrupts();
}

void ymfm_set_timer(uint32_t tnum, int32_t duration_in_clocks)
{
	if (tnum < 2)
		g_timer[tnum] = duration_in_clocks;
}

void ymfm_set_busy_end(uint32_t clocks)
{
	/*
	 * ymfm asks the host to set a new busy-clear deadline relative to the
	 * current chip time.  Repeated register writes restart this short period;
	 * accumulating them can leave the Z80 polling BUSY for thousands of
	 * frames after a driver's initialization burst.
	 */
	g_busy_timer = (int32_t)clocks;
}

bool ymfm_is_busy(void)
{
	return g_busy_timer > 0;
}

void ymfm_update_irq(bool asserted)
{
	g_irq_asserted = asserted;
}

static size_t g_serial_pos;

void geo_serial_begin(void)
{
	g_serial_pos = 0;
}

void geo_serial_pushblk(uint8_t *dst, uint8_t *src, size_t size)
{
	for (size_t i = 0; i < size; ++i)
		dst[g_serial_pos + i] = src[i];
	g_serial_pos += size;
}

void geo_serial_popblk(uint8_t *dst, uint8_t *src, size_t size)
{
	for (size_t i = 0; i < size; ++i)
		dst[i] = src[g_serial_pos + i];
	g_serial_pos += size;
}

void geo_serial_push8(uint8_t *st, uint8_t v)
{
	st[g_serial_pos++] = v;
}

void geo_serial_push16(uint8_t *st, uint16_t v)
{
	st[g_serial_pos++] = (uint8_t)(v >> 8);
	st[g_serial_pos++] = (uint8_t)(v & 0xff);
}

void geo_serial_push32(uint8_t *st, uint32_t v)
{
	st[g_serial_pos++] = (uint8_t)(v >> 24);
	st[g_serial_pos++] = (uint8_t)((v >> 16) & 0xff);
	st[g_serial_pos++] = (uint8_t)((v >> 8) & 0xff);
	st[g_serial_pos++] = (uint8_t)(v & 0xff);
}

void geo_serial_push64(uint8_t *st, uint64_t v)
{
	st[g_serial_pos++] = (uint8_t)(v >> 56);
	st[g_serial_pos++] = (uint8_t)((v >> 48) & 0xff);
	st[g_serial_pos++] = (uint8_t)((v >> 40) & 0xff);
	st[g_serial_pos++] = (uint8_t)((v >> 32) & 0xff);
	st[g_serial_pos++] = (uint8_t)((v >> 24) & 0xff);
	st[g_serial_pos++] = (uint8_t)((v >> 16) & 0xff);
	st[g_serial_pos++] = (uint8_t)((v >> 8) & 0xff);
	st[g_serial_pos++] = (uint8_t)(v & 0xff);
}

uint8_t geo_serial_pop8(uint8_t *st)
{
	return st[g_serial_pos++];
}

uint16_t geo_serial_pop16(uint8_t *st)
{
	uint16_t value = (uint16_t)st[g_serial_pos++] << 8;
	value |= st[g_serial_pos++];
	return value;
}

uint32_t geo_serial_pop32(uint8_t *st)
{
	uint32_t value = (uint32_t)st[g_serial_pos++] << 24;
	value |= (uint32_t)st[g_serial_pos++] << 16;
	value |= (uint32_t)st[g_serial_pos++] << 8;
	value |= st[g_serial_pos++];
	return value;
}

uint64_t geo_serial_pop64(uint8_t *st)
{
	uint64_t value = (uint64_t)st[g_serial_pos++] << 56;
	value |= (uint64_t)st[g_serial_pos++] << 48;
	value |= (uint64_t)st[g_serial_pos++] << 40;
	value |= (uint64_t)st[g_serial_pos++] << 32;
	value |= (uint64_t)st[g_serial_pos++] << 24;
	value |= (uint64_t)st[g_serial_pos++] << 16;
	value |= (uint64_t)st[g_serial_pos++] << 8;
	value |= st[g_serial_pos++];
	return value;
}

uint32_t geo_serial_peek32(uint8_t *st)
{
	uint32_t value = (uint32_t)st[g_serial_pos] << 24;
	value |= (uint32_t)st[g_serial_pos + 1] << 16;
	value |= (uint32_t)st[g_serial_pos + 2] << 8;
	value |= st[g_serial_pos + 3];
	return value;
}

size_t geo_serial_size(void)
{
	return g_serial_pos;
}

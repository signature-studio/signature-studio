/* MMX (mm register form) helpers for fe-lift output (mmx.rs). Header only, included at
 * the end of fle_sse.h. Semantics: Intel SDM Vol. 2 (325383-093US), the "With 64-bit
 * Operands" pseudo-code of each instruction; x87 side effects: SDM Vol. 3A 15.2 and
 * Vol. 1 9.6.2. See docs/decisions/0028-mmx.md.
 *
 * State: MM[n] is MMn, an independent uint64_t. It is not aliased to ST0-7: no x87
 * arithmetic is lifted, so there is no 80-bit register contents to alias, and bits
 * 64-79 of the physical x87 registers (set to ones by an MMX write) are not modeled.
 * What is modeled is the effect on the x87 status and tag word in fle_x87
 * (fle_x87ctl.h, defined in fle_x87ctl.c, which must be linked): every MMX instruction
 * sets TOP (status bits 11-13) to 0 and the tag word to 0 (all valid); EMMS sets TOP
 * to 0 and the tag word to FFFFH (all empty). Flags are never changed. */
#ifndef FLE_MMX_H
#define FLE_MMX_H

#include <stdint.h>

#include "fle_runtime.h"
#include "fle_x87ctl.h"

/* Weak definition: every translation unit that includes this header carries the same
 * symbol, and the linker keeps one. */
uint64_t MM[8] __attribute__((weak)) = { 0 };

/* 1 while MMX state is live (set by every MMX instruction, cleared by EMMS). x87
 * instructions (fle_x87.c, decision 0033) stop when they run in that state, because
 * MM is not aliased to ST. */
uint8_t fle_x87_mmx_active __attribute__((weak)) = 0;

/* After any MMX instruction other than EMMS (SDM Vol. 3A, Table 15-2 and 15-3). */
static inline void fle_mmx_touch(void) {
    fle_x87.sw = (uint16_t)(fle_x87.sw & ~0x3800u);
    fle_x87.tw = 0x0000;
    fle_x87_mmx_active = 1;
}

/* EMMS: all tags empty (SDM Vol. 2 Operation: x87FPUTagWord := FFFFH). TOP is also set to
 * 0 (Vol. 3A Table 15-2; measured on the host, decision 0028). */
static inline void fle_mmx_emms(void) {
    fle_x87.sw = (uint16_t)(fle_x87.sw & ~0x3800u);
    fle_x87.tw = 0xFFFF;
    fle_x87_mmx_active = 0;
}

typedef union {
    uint64_t q;
    uint8_t u8[8];
    int8_t s8[8];
    uint16_t u16[4];
    int16_t s16[4];
    uint32_t u32[2];
} fle_mm_lanes;

static inline uint64_t fle_mmx_pxor(uint64_t a, uint64_t b) { return a ^ b; }

static inline uint64_t fle_mmx_paddw(uint64_t a, uint64_t b) {
    fle_mm_lanes x, y;
    int k;
    x.q = a;
    y.q = b;
    for (k = 0; k < 4; k++) x.u16[k] = (uint16_t)(x.u16[k] + y.u16[k]);
    return x.q;
}

/* PSUBSW: SaturateToSignedWord (DEST - SRC). */
static inline uint64_t fle_mmx_psubsw(uint64_t a, uint64_t b) {
    fle_mm_lanes x, y;
    int k;
    x.q = a;
    y.q = b;
    for (k = 0; k < 4; k++) {
        int32_t v = (int32_t)x.s16[k] - (int32_t)y.s16[k];
        x.s16[k] = (int16_t)(v < -32768 ? -32768 : v > 32767 ? 32767 : v);
    }
    return x.q;
}

/* PMULHW: signed 16 x 16 -> 32, bits 31:16. */
static inline uint64_t fle_mmx_pmulhw(uint64_t a, uint64_t b) {
    fle_mm_lanes x, y;
    int k;
    x.q = a;
    y.q = b;
    for (k = 0; k < 4; k++) x.u16[k] = (uint16_t)(((uint32_t)((int32_t)x.s16[k] * (int32_t)y.s16[k])) >> 16);
    return x.q;
}

/* PMULHUW: unsigned 16 x 16 -> 32, bits 31:16. */
static inline uint64_t fle_mmx_pmulhuw(uint64_t a, uint64_t b) {
    fle_mm_lanes x, y;
    int k;
    x.q = a;
    y.q = b;
    for (k = 0; k < 4; k++) x.u16[k] = (uint16_t)(((uint32_t)x.u16[k] * (uint32_t)y.u16[k]) >> 16);
    return x.q;
}

/* PACKUSWB: the signed words of DEST become bytes 0-3 and those of SRC bytes 4-7, with
 * unsigned saturation (SaturateSignedWordToUnsignedByte). */
static inline uint64_t fle_mmx_packuswb(uint64_t a, uint64_t b) {
    fle_mm_lanes x, y, z;
    int k;
    x.q = a;
    y.q = b;
    z.q = 0;
    for (k = 0; k < 4; k++) {
        int16_t p = x.s16[k], q = y.s16[k];
        z.u8[k] = (uint8_t)(p < 0 ? 0 : p > 255 ? 255 : p);
        z.u8[4 + k] = (uint8_t)(q < 0 ? 0 : q > 255 ? 255 : q);
    }
    return z.q;
}

/* PUNPCKLBW: DEST[7:0] := DEST[7:0]; DEST[15:8] := SRC[7:0]; DEST[23:16] := DEST[15:8]; ...
 * DEST[63:56] := SRC[31:24]. Only SRC[31:0] is used (the memory form is m32). */
static inline uint64_t fle_mmx_punpcklbw(uint64_t a, uint64_t b) {
    fle_mm_lanes x, y, z;
    int k;
    x.q = a;
    y.q = b;
    z.q = 0;
    for (k = 0; k < 4; k++) {
        z.u8[2 * k] = x.u8[k];
        z.u8[2 * k + 1] = y.u8[k];
    }
    return z.q;
}

/* PSHUFW: DEST word k := (SRC >> (ORDER[2k+1:2k] * 16))[15:0]. */
static inline uint64_t fle_mmx_pshufw(uint64_t s, unsigned imm) {
    fle_mm_lanes x, z;
    int k;
    x.q = s;
    z.q = 0;
    for (k = 0; k < 4; k++) z.u16[k] = x.u16[(imm >> (2 * k)) & 3];
    return z.q;
}

/* PSLLW: the count is the whole 64-bit count operand (mm, m64) or imm8; COUNT > 15 gives 0. */
static inline uint64_t fle_mmx_psllw(uint64_t a, uint64_t cnt) {
    fle_mm_lanes x;
    int k;
    x.q = a;
    for (k = 0; k < 4; k++) x.u16[k] = cnt > 15 ? 0 : (uint16_t)(x.u16[k] << (unsigned)cnt);
    return x.q;
}

#endif

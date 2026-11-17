/* XMM registers and scalar SSE helpers for fe-lift output.
 *
 * X[n] is XMMn. Each lifted instruction writes only the lanes it writes (SDM
 * Vol. 1 §11.6, per instruction). Scalar arithmetic is one IEEE float or
 * double operation with MXCSR at its default (RN, all exceptions masked).
 * NaN results follow SSE: when either operand is NaN, the result is the first
 * source's NaN if it is a NaN, else the second's, quieted. Build with
 * -ffp-contract=off so that no FMA is formed.
 */
#ifndef FLE_SSE_H
#define FLE_SSE_H

#include <stdint.h>
#include <string.h>

#include "fle_ai.h"
#include "fle_runtime.h"

typedef union {
    uint64_t u64[2];
    uint32_t u32[4];
    float f32[4];
    double f64[2];
} fle_xmm;

#ifdef FLE_LOWREGS_X
#include "fle_lowregs.h"   /* decision 0234 (mt3, B-prime stage 1): X is +0x200 of the thread-local block fle_ctx */
#define X (*(fle_xmm (*)[16])(fle_ctx + FLE_CTX_X))
#else
extern fle_xmm X[16];
#endif

_Static_assert(sizeof(fle_xmm) == 16, "fle_xmm is 128 bits");

/* anim1 (decision 0225): with -msimd128 the packed integer helpers below use WebAssembly SIMD (one instruction per x86 instruction) instead of a
 * lane loop. The result bits are the same (differential test lift/harness/anim1/); */
#if defined(__wasm_simd128__)
#define FLE_WASM_SIMD 1
#include <wasm_simd128.h>
static inline v128_t fle_v(fle_xmm a) { v128_t v; __builtin_memcpy(&v, &a, 16); return v; }
static inline fle_xmm fle_xv(v128_t v) { fle_xmm a; __builtin_memcpy(&a, &v, 16); return a; }
#endif

static inline FLE_AI int fle_nan32(uint32_t u) {
    return (u & 0x7fffffffu) > 0x7f800000u;
}

static inline FLE_AI int fle_nan64(uint64_t u) {
    return (u & UINT64_C(0x7fffffffffffffff)) > UINT64_C(0x7ff0000000000000);
}

static inline FLE_AI float fle_f32(uint32_t u) {
    float f;
    memcpy(&f, &u, sizeof f);
    return f;
}

static inline FLE_AI double fle_f64(uint64_t u) {
    double d;
    memcpy(&d, &u, sizeof d);
    return d;
}

static inline FLE_AI uint32_t fle_b32(float f) {
    uint32_t u;
    memcpy(&u, &f, sizeof u);
    return u;
}

static inline FLE_AI uint64_t fle_b64(double d) {
    uint64_t u;
    memcpy(&u, &d, sizeof u);
    return u;
}

#define FLE_ARITH32(name, op) \
    static inline uint32_t name(uint32_t a, uint32_t b) \
    { \
        if (fle_nan32(a)) return a | 0x00400000u; \
        if (fle_nan32(b)) return b | 0x00400000u; \
        return fle_b32(fle_f32(a) op fle_f32(b)); \
    }

#define FLE_ARITH64(name, op) \
    static inline uint64_t name(uint64_t a, uint64_t b) \
    { \
        if (fle_nan64(a)) return a | UINT64_C(0x0008000000000000); \
        if (fle_nan64(b)) return b | UINT64_C(0x0008000000000000); \
        return fle_b64(fle_f64(a) op fle_f64(b)); \
    }

FLE_ARITH32(fle_addss, +)
FLE_ARITH32(fle_subss, -)
FLE_ARITH32(fle_mulss, *)
FLE_ARITH32(fle_divss, /)
FLE_ARITH64(fle_addsd, +)
FLE_ARITH64(fle_subsd, -)
FLE_ARITH64(fle_mulsd, *)
FLE_ARITH64(fle_divsd, /)

/* CVTSS2SD: a NaN keeps its sign and payload, and the quiet bit is set. */
static inline FLE_AI uint64_t fle_cvtss2sd(uint32_t b) {
    if (fle_nan32(b)) {
        return ((uint64_t)(b >> 31) << 63) | UINT64_C(0x7ff0000000000000) |
               ((uint64_t)(b & 0x7fffffu) << 29) | UINT64_C(0x0008000000000000);
    }
    return fle_b64((double)fle_f32(b));
}

/* CVTSD2SS: a NaN keeps its sign and the top payload bits, and the quiet bit is set. */
static inline FLE_AI uint32_t fle_cvtsd2ss(uint64_t b) {
    if (fle_nan64(b)) {
        return (uint32_t)(b >> 63) << 31 | 0x7f800000u | (uint32_t)((b >> 29) & 0x7fffffu) | 0x00400000u;
    }
    return fle_b32((float)fle_f64(b));
}

/* MINSS, MAXSS: the second source is returned when either operand is NaN or
 * when both are zero (the comparison is false). */
static inline FLE_AI uint32_t fle_minss(uint32_t a, uint32_t b) {
    if (fle_nan32(a) || fle_nan32(b)) return b;
    return fle_f32(a) < fle_f32(b) ? a : b;
}

static inline FLE_AI uint32_t fle_maxss(uint32_t a, uint32_t b) {
    if (fle_nan32(a) || fle_nan32(b)) return b;
    return fle_f32(a) > fle_f32(b) ? a : b;
}

static inline FLE_AI uint64_t fle_minsd(uint64_t a, uint64_t b) {
    if (fle_nan64(a) || fle_nan64(b)) return b;
    return fle_f64(a) < fle_f64(b) ? a : b;
}

static inline FLE_AI uint64_t fle_maxsd(uint64_t a, uint64_t b) {
    if (fle_nan64(a) || fle_nan64(b)) return b;
    return fle_f64(a) > fle_f64(b) ? a : b;
}

/* fle_sqrtss and fle_sqrtsd (SQRTSS, SQRTSD at the default MXCSR) are defined after the include of
 * fle_mxcsr_flags.h at the end of this file: they use its integer square root. */

/* Round to nearest, ties to even. Adding and subtracting 2^52 rounds
 * |x| < 2^52 to an integer. Larger values, infinities and NaN are returned
 * unchanged. Only the integer value is used, so the sign of zero does not
 * matter. */
static inline FLE_AI double fle_rne(double x) {
    double a = x < 0 ? -x : x;
    double r;
    if (!(a < 0x1p52)) {
        return x;
    }
    r = (a + 0x1p52) - 0x1p52;
    return x < 0 ? -r : r;
}

/* CVTTSD2SI and CVTTSS2SI (truncation). Out of range and NaN give the integer
 * indefinite value. The 32-bit result is zero-extended. */
static inline FLE_AI uint64_t fle_cvtt32(double x) {
    if (!(x > -2147483649.0 && x < 2147483648.0)) {
        return 0x80000000u;
    }
    return (uint32_t)(int32_t)x;
}

static inline FLE_AI uint64_t fle_cvtt64(double x) {
    if (!(x >= -9223372036854775808.0 && x < 9223372036854775808.0)) {
        return UINT64_C(0x8000000000000000);
    }
    return (uint64_t)(int64_t)x;
}

/* CVTSD2SI and CVTSS2SI (round by fle_rne). */
static inline FLE_AI uint64_t fle_cvtr32(double x) {
    double r = fle_rne(x);
    if (!(r >= -2147483648.0 && r < 2147483648.0)) {
        return 0x80000000u;
    }
    return (uint32_t)(int32_t)r;
}

static inline FLE_AI uint64_t fle_cvtr64(double x) {
    double r = fle_rne(x);
    if (!(r >= -9223372036854775808.0 && r < 9223372036854775808.0)) {
        return UINT64_C(0x8000000000000000);
    }
    return (uint64_t)(int64_t)r;
}

/* 128-bit loads and stores. The aligned forms abort on an address that is not
 * a multiple of 16. The unaligned forms do not check. */
static inline FLE_AI void fle_misaligned(uint64_t a) {
    fprintf(stderr, "fle: misaligned 16-byte access at %#llx\n", (unsigned long long)a);
    abort();
}

static inline FLE_AI fle_xmm fle_ldx(uint64_t a) {
    fle_xmm v;
    memcpy(&v, (const void *)fle_addr(a, 16, 0), sizeof v);
    return v;
}

static inline FLE_AI fle_xmm fle_ldxa(uint64_t a) {
    if (a & 15) fle_misaligned(a);
    return fle_ldx(a);
}

static inline FLE_AI void fle_stx(uint64_t a, fle_xmm v) {
    memcpy((void *)fle_addr(a, 16, 1), &v, sizeof v);
}

static inline FLE_AI void fle_stxa(uint64_t a, fle_xmm v) {
    if (a & 15) fle_misaligned(a);
    fle_stx(a, v);
}

/* 128-bit logic. */
static inline FLE_AI fle_xmm fle_pand(fle_xmm a, fle_xmm b) {
    a.u64[0] &= b.u64[0];
    a.u64[1] &= b.u64[1];
    return a;
}

/* PANDN: (NOT dst) AND src. */
static inline FLE_AI fle_xmm fle_pandn(fle_xmm a, fle_xmm b) {
    a.u64[0] = ~a.u64[0] & b.u64[0];
    a.u64[1] = ~a.u64[1] & b.u64[1];
    return a;
}

static inline FLE_AI fle_xmm fle_por(fle_xmm a, fle_xmm b) {
    a.u64[0] |= b.u64[0];
    a.u64[1] |= b.u64[1];
    return a;
}

static inline FLE_AI fle_xmm fle_pxor(fle_xmm a, fle_xmm b) {
    a.u64[0] ^= b.u64[0];
    a.u64[1] ^= b.u64[1];
    return a;
}

/* Packed integer. Lanes wrap. */
static inline FLE_AI fle_xmm fle_paddd(fle_xmm a, fle_xmm b) {
    int k;
#ifdef FLE_WASM_SIMD
    return fle_xv(wasm_i32x4_add(fle_v(a), fle_v(b)));
#endif
    for (k = 0; k < 4; k++) a.u32[k] += b.u32[k];
    return a;
}

static inline FLE_AI fle_xmm fle_psubd(fle_xmm a, fle_xmm b) {
    int k;
#ifdef FLE_WASM_SIMD
    return fle_xv(wasm_i32x4_sub(fle_v(a), fle_v(b)));
#endif
    for (k = 0; k < 4; k++) a.u32[k] -= b.u32[k];
    return a;
}

static inline FLE_AI fle_xmm fle_pcmpeqd(fle_xmm a, fle_xmm b) {
    int k;
#ifdef FLE_WASM_SIMD
    return fle_xv(wasm_i32x4_eq(fle_v(a), fle_v(b)));
#endif
    for (k = 0; k < 4; k++) a.u32[k] = a.u32[k] == b.u32[k] ? 0xffffffffu : 0u;
    return a;
}

/* PMULHUW: the high 16 bits of each unsigned 16 x 16 product. */
static inline FLE_AI fle_xmm fle_pmulhuw(fle_xmm a, fle_xmm b) {
    fle_xmm r;
#ifdef FLE_WASM_SIMD
    {
        v128_t x = fle_v(a), y = fle_v(b);
        return fle_xv(wasm_i8x16_shuffle(wasm_u32x4_extmul_low_u16x8(x, y), wasm_u32x4_extmul_high_u16x8(x, y), 2, 3, 6, 7, 10, 11, 14, 15, 18, 19, 22, 23, 26, 27, 30, 31));
    }
#endif
    uint16_t x[8], y[8], z[8];
    int k;
    memcpy(x, &a, sizeof x);
    memcpy(y, &b, sizeof y);
    for (k = 0; k < 8; k++) z[k] = (uint16_t)(((uint32_t)x[k] * (uint32_t)y[k]) >> 16);
    memcpy(&r, z, sizeof r);
    return r;
}

/* PSHUFD: result lane k is source lane (imm >> 2k) & 3. */
static inline FLE_AI fle_xmm fle_pshufd(fle_xmm s, unsigned imm) {
    fle_xmm r;
    int k;
    for (k = 0; k < 4; k++) r.u32[k] = s.u32[(imm >> (2 * k)) & 3];
    return r;
}

/* PUNPCKL* (hi = 0) and PUNPCKH* (hi = 1). esz is the element size in bytes.
 * The result alternates elements of a and b, starting with a. */
static inline FLE_AI fle_xmm fle_unpack(fle_xmm a, fle_xmm b, int esz, int hi) {
    fle_xmm r;
#ifdef FLE_WASM_SIMD
    {
        v128_t x = fle_v(a), y = fle_v(b);
        if (esz == 1) return fle_xv(hi ? wasm_i8x16_shuffle(x, y, 8, 24, 9, 25, 10, 26, 11, 27, 12, 28, 13, 29, 14, 30, 15, 31) : wasm_i8x16_shuffle(x, y, 0, 16, 1, 17, 2, 18, 3, 19, 4, 20, 5, 21, 6, 22, 7, 23));
        if (esz == 2) return fle_xv(hi ? wasm_i8x16_shuffle(x, y, 8, 9, 24, 25, 10, 11, 26, 27, 12, 13, 28, 29, 14, 15, 30, 31) : wasm_i8x16_shuffle(x, y, 0, 1, 16, 17, 2, 3, 18, 19, 4, 5, 20, 21, 6, 7, 22, 23));
        if (esz == 4) return fle_xv(hi ? wasm_i8x16_shuffle(x, y, 8, 9, 10, 11, 24, 25, 26, 27, 12, 13, 14, 15, 28, 29, 30, 31) : wasm_i8x16_shuffle(x, y, 0, 1, 2, 3, 16, 17, 18, 19, 4, 5, 6, 7, 20, 21, 22, 23));
        if (esz == 8) return fle_xv(hi ? wasm_i8x16_shuffle(x, y, 8, 9, 10, 11, 12, 13, 14, 15, 24, 25, 26, 27, 28, 29, 30, 31) : wasm_i8x16_shuffle(x, y, 0, 1, 2, 3, 4, 5, 6, 7, 16, 17, 18, 19, 20, 21, 22, 23));
    }
#endif
    uint8_t x[16], y[16], z[16];
    int j, base = hi ? 8 : 0;
    memcpy(x, &a, sizeof x);
    memcpy(y, &b, sizeof y);
    for (j = 0; j < 8 / esz; j++) {
        memcpy(&z[2 * j * esz], &x[base + j * esz], (size_t)esz);
        memcpy(&z[(2 * j + 1) * esz], &y[base + j * esz], (size_t)esz);
    }
    memcpy(&r, z, sizeof r);
    return r;
}

/* CVTPS2DQ (by fle_cvtr32, round by MXCSR RN) and CVTTPS2DQ (truncation).
 * Out of range and NaN give the integer indefinite value 0x80000000. */
static inline FLE_AI fle_xmm fle_cvtps2dq(fle_xmm s) {
    fle_xmm r;
    int k;
    for (k = 0; k < 4; k++) r.u32[k] = (uint32_t)fle_cvtr32((double)s.f32[k]);
    return r;
}

static inline FLE_AI fle_xmm fle_cvttps2dq(fle_xmm s) {
    fle_xmm r;
    int k;
    for (k = 0; k < 4; k++) r.u32[k] = (uint32_t)fle_cvtt32((double)s.f32[k]);
    return r;
}

/* Packed SSE2 integer helpers (header only): docs/decisions/0026-sse-int2.md. */
#include "fle_sse_int2.h"
#include "fle_sse_shuf.h"
#include "fle_ssse3.h" /* decision 0190: PALIGNR, PABSB/W/D, PSHUFB */

/* MMX (mm register form) helpers: docs/decisions/0028-mmx.md. */
#include "fle_mmx.h"

/* VEX moves, VZEROUPPER and the GS (TEB) model: docs/decisions/0032-vex-fsgs.md. */
#include "fle_vex.h"

/* SSE floating point with the MXCSR exception flags, RC, FTZ and DAZ: docs/decisions/0038-mxcsr-flags.md. */
#include "fle_mxcsr_flags.h"

/* SQRTSS and SQRTSD with MXCSR at its default (RN, all exceptions masked, FTZ and DAZ off): the correctly rounded
 * (nearest even) root computed with integers by fle_mx_sqrt, so the result bits (the QNaN indefinite for a negative
 * input, a NaN operand quieted, -0 stays -0) are the hardware's and do not depend on the host FP unit. The status
 * flags are dropped and fle_mxcsr is not read or written. fe-lift does not emit these any more: sqrtss and sqrtsd
 * are lifted to fle_mx_sqrtss and fle_mx_sqrtsd (sse_fp_flags.rs), which use the guest's MXCSR. */
static inline FLE_AI uint32_t fle_sqrtss(uint32_t b) {
    uint32_t fl = 0;
    return (uint32_t)fle_mx_sqrt(0, b, 0x1F80u, &fl);
}

static inline FLE_AI uint64_t fle_sqrtsd(uint64_t b) {
    uint32_t fl = 0;
    return fle_mx_sqrt(1, b, 0x1F80u, &fl);
}

#endif

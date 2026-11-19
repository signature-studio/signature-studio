/* SSE/SSE2/SSE4.1 floating-point operations with the MXCSR exception flags, in portable C
 * (docs/decisions/0044-mxcsr-flags-portable.md; revises 0038, which ran each instruction on the host CPU).
 *
 * Header only, C99 + <stdint.h> + unsigned __int128. No asm, no <emmintrin.h>, no <fenv.h>, no float or double
 * arithmetic: every operation is done on the bit patterns with integer arithmetic, so the result does not depend on
 * the host FP unit, the host MXCSR or the width of long (nothing here uses long or uintptr_t).
 *
 * What is modelled, per instruction (SDM Vol. 1 4.8-4.9, 10.2.3; Vol. 2 per instruction):
 *   - result bits: IEEE 754 binary32/binary64 with x86 NaN rules (the first operand's NaN wins over the second's,
 *     an SNaN becomes a QNaN, an invalid operation without a NaN operand gives the default QNaN "real indefinite");
 *   - rounding control RC (MXCSR bits 14:13) for add sub mul div sqrt, the conversions, and ROUND with imm8 bit 2;
 *   - DAZ (bit 6): denormal source operands are zero (same sign) and do not raise DE;
 *   - FTZ (bit 15): with UM (bit 11) set, a tiny result is replaced by a zero of the same sign and UE and PE are
 *     raised; tininess is detected after rounding (as the hardware does);
 *   - the status flags IE DE ZE OE UE PE (bits 5:0), OR-ed into fle_mxcsr (sticky);
 *   - if a raised flag is unmasked in the guest's MXCSR (bit 7+n clear), real hardware would deliver #XM (or #UD):
 *     the run stops with a named abort, before the destination is written;
 *   - COMISS COMISD UCOMISS UCOMISD set ZF PF CF and clear OF SF AF.
 * Packed forms compute each lane and OR the flags of all lanes (one instruction, one flag update).
 *
 * fle_mxcsr has a weak definition here so that a program built from a header-only runtime links; the strong one
 * is in runtime/fle_mxcsr.c.
 */
#ifndef FLE_MXCSR_FLAGS_H
#define FLE_MXCSR_FLAGS_H

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define FLE_MX_CTL 0xE040u   /* RC, FTZ, DAZ */
#define FLE_MX_MASKS 0x1F80u /* IM DM ZM OM UM PM */
#define FLE_MX_FLAGS 0x003Fu /* IE DE ZE OE UE PE */

#define FLE_MX_IE 0x01u
#define FLE_MX_DE 0x02u
#define FLE_MX_ZE 0x04u
#define FLE_MX_OE 0x08u
#define FLE_MX_UE 0x10u
#define FLE_MX_PE 0x20u

__attribute__((weak)) uint32_t fle_mxcsr = 0x1F80;

typedef unsigned __int128 fle_mxu128;

/* class of an unpacked operand */
#define FLE_MXC_ZERO 0
#define FLE_MXC_FIN 1
#define FLE_MXC_INF 2
#define FLE_MXC_QNAN 3
#define FLE_MXC_SNAN 4

/* operation codes of fle_mx_arith */
#define FLE_MXO_ADD 0
#define FLE_MXO_SUB 1
#define FLE_MXO_MUL 2
#define FLE_MXO_DIV 3
#define FLE_MXO_MIN 4
#define FLE_MXO_MAX 5
#define FLE_MXO_SQRT 6

typedef struct {
    uint64_t raw; /* the operand as given */
    uint64_t m;   /* FIN: normalized significand, bit 63 set */
    int32_t e;    /* FIN: exponent of bit 63 (value = m * 2^(e-63)) */
    uint8_t cls;
    uint8_t sign;
    uint8_t den; /* a denormal operand that DAZ did not zero */
} fle_mxv;

static inline void fle_mx_unmasked(uint32_t raised) {
    fprintf(stderr, "fle: SSE floating-point exception unmasked in guest MXCSR (raised flags %#x, MXCSR %#x); #XM is not modelled\n",
            (unsigned)raised, (unsigned)fle_mxcsr);
    abort();
}

/* Called once per instruction with the flags it raised. Aborts before anything is stored if one is unmasked. */
static inline void fle_mx_fin(uint32_t fl) {
    if (fl & ~((fle_mxcsr >> 7) & FLE_MX_FLAGS)) {
        fle_mx_unmasked(fl);
    }
    fle_mxcsr |= fl;
}

static inline int fle_mx_clz64(uint64_t x) { /* x != 0 */
#if defined(__GNUC__) && !defined(FLE_MX_PORTABLE_CLZ)
    return __builtin_clzll(x);
#else
    int n = 0;
    if (!(x >> 32)) { n += 32; x <<= 32; }
    if (!(x >> 48)) { n += 16; x <<= 16; }
    if (!(x >> 56)) { n += 8; x <<= 8; }
    if (!(x >> 60)) { n += 4; x <<= 4; }
    if (!(x >> 62)) { n += 2; x <<= 2; }
    if (!(x >> 63)) { n += 1; }
    return n;
#endif
}

static inline int fle_mx_clz128(fle_mxu128 x) { /* x != 0 */
    uint64_t hi = (uint64_t)(x >> 64);
    return hi ? fle_mx_clz64(hi) : 64 + fle_mx_clz64((uint64_t)x);
}

/* ---------------------------------------------------------------------------------------------------------- */
/* operands */

static inline void fle_mx_unpack(fle_mxv *v, uint64_t x, int f64, uint32_t mx) {
    int mb = f64 ? 52 : 23, eb = f64 ? 11 : 8, bias = f64 ? 1023 : 127;
    uint64_t mant = x & ((UINT64_C(1) << mb) - 1);
    uint32_t ex = (uint32_t)(x >> mb) & ((1u << eb) - 1u);
    v->raw = x;
    v->sign = (uint8_t)((x >> (mb + eb)) & 1);
    v->den = 0;
    v->m = 0;
    v->e = 0;
    if (ex == (1u << eb) - 1u) {
        v->cls = mant == 0 ? FLE_MXC_INF : ((mant >> (mb - 1)) & 1 ? FLE_MXC_QNAN : FLE_MXC_SNAN);
    } else if (ex == 0) {
        if (mant == 0 || (mx & 0x40u)) {
            v->cls = FLE_MXC_ZERO;
        } else {
            int lz = fle_mx_clz64(mant);
            v->cls = FLE_MXC_FIN;
            v->den = 1;
            v->m = mant << lz;
            v->e = (1 - bias) - mb + 63 - lz;
        }
    } else {
        v->cls = FLE_MXC_FIN;
        v->m = (mant | (UINT64_C(1) << mb)) << (63 - mb);
        v->e = (int32_t)ex - bias;
    }
}

static inline uint64_t fle_mx_zero(int f64, int sign) {
    return (uint64_t)sign << (f64 ? 63 : 31);
}

static inline uint64_t fle_mx_inf(int f64, int sign) {
    return f64 ? ((uint64_t)sign << 63) | UINT64_C(0x7ff0000000000000) : ((uint64_t)sign << 31) | UINT64_C(0x7f800000);
}

/* the default QNaN, "real indefinite" */
static inline uint64_t fle_mx_indef(int f64) {
    return f64 ? UINT64_C(0xfff8000000000000) : UINT64_C(0xffc00000);
}

static inline uint64_t fle_mx_quiet(uint64_t raw, int f64) {
    return raw | (UINT64_C(1) << (f64 ? 51 : 22));
}

/* Largest finite value of the format, with the sign. */
static inline uint64_t fle_mx_maxfin(int f64, int sign) {
    return f64 ? ((uint64_t)sign << 63) | UINT64_C(0x7fefffffffffffff) : ((uint64_t)sign << 31) | UINT64_C(0x7f7fffff);
}

/* Round and pack a nonzero value: sig has bit 127 set, value = (sig / 2^127) * 2^e, sticky bits are in sig already
 * (below the guard bit). Raises PE, OE, UE; applies FTZ. */
static inline uint64_t fle_mx_pack(int f64, int sign, int32_t e, fle_mxu128 sig, uint32_t mx, uint32_t *fl) {
    int mb = f64 ? 52 : 23, eb = f64 ? 11 : 8, bias = f64 ? 1023 : 127;
    int p = mb + 1, emin = 1 - bias, rc = (int)((mx >> 13) & 3u);
    int sh = 128 - p;
    int tiny = e < emin, tiny_after = 0;
    uint64_t kept, mant_mask = (UINT64_C(1) << mb) - 1;
    int guard, sticky, inexact, inc;
    uint64_t bits;
    if (tiny) {
        sh += emin - e;
    }
    if (sh >= 129) {
        kept = 0;
        guard = 0;
        sticky = 1;
    } else if (sh == 128) {
        kept = 0;
        guard = (int)(sig >> 127);
        sticky = (sig << 1) != 0;
    } else {
        kept = (uint64_t)(sig >> sh);
        guard = (int)((sig >> (sh - 1)) & 1);
        sticky = (sig & (((fle_mxu128)1 << (sh - 1)) - 1)) != 0;
    }
    inexact = guard | sticky;
    inc = rc == 0 ? (guard && (sticky || (kept & 1))) : rc == 1 ? (sign && inexact) : rc == 2 ? (!sign && inexact) : 0;
    kept += (uint64_t)inc;
    if (tiny) {
        tiny_after = 1;
        if (e == emin - 1) {
            /* tininess after rounding: round to p bits with an unbounded exponent; reaching 2^emin is not tiny */
            uint64_t ku = (uint64_t)(sig >> (128 - p));
            int gu = (int)((sig >> (127 - p)) & 1);
            int su = (sig & (((fle_mxu128)1 << (127 - p)) - 1)) != 0;
            int iu = rc == 0 ? (gu && (su || (ku & 1))) : rc == 1 ? (sign && (gu | su)) : rc == 2 ? (!sign && (gu | su)) : 0;
            if ((ku + (uint64_t)iu) >> p) {
                tiny_after = 0;
            }
        }
        if (tiny_after && (mx & 0x8000u) && (mx & 0x800u)) {
            *fl |= FLE_MX_UE | FLE_MX_PE;
            return fle_mx_zero(f64, sign);
        }
        if (inexact) {
            *fl |= FLE_MX_PE;
        }
        /* UE: tiny and inexact; with UM clear (unmasked) every tiny result signals it (SDM Vol. 1 4.9.1.5) */
        if (tiny_after && (inexact || !(mx & 0x800u))) {
            *fl |= FLE_MX_UE;
        }
        bits = kept;
    } else {
        int32_t ef = e + bias - 1 + (int32_t)(kept >> mb);
        if (ef >= (1 << eb) - 1) {
            *fl |= FLE_MX_OE | FLE_MX_PE;
            if (rc == 0 || (rc == 1 && sign) || (rc == 2 && !sign)) {
                return fle_mx_inf(f64, sign);
            }
            return fle_mx_maxfin(f64, sign);
        }
        if (inexact) {
            *fl |= FLE_MX_PE;
        }
        bits = ((uint64_t)ef << mb) | (kept & mant_mask);
    }
    return bits | ((uint64_t)sign << (mb + eb));
}

/* ---------------------------------------------------------------------------------------------------------- */
/* add sub mul div sqrt min max */

static inline uint64_t fle_mx_add(int f64, const fle_mxv *a, const fle_mxv *b, uint32_t mx, uint32_t *fl) {
    int rc = (int)((mx >> 13) & 3u);
    fle_mxu128 sa, sb, s;
    int32_t ea, eb;
    int sign, d, lz;
    if (a->cls == FLE_MXC_INF || b->cls == FLE_MXC_INF) {
        if (a->cls == FLE_MXC_INF && b->cls == FLE_MXC_INF && a->sign != b->sign) {
            *fl |= FLE_MX_IE;
            return fle_mx_indef(f64);
        }
        if (a->den | b->den) {
            *fl |= FLE_MX_DE;
        }
        return fle_mx_inf(f64, a->cls == FLE_MXC_INF ? a->sign : b->sign);
    }
    if (a->den | b->den) {
        *fl |= FLE_MX_DE;
    }
    if (a->cls == FLE_MXC_ZERO && b->cls == FLE_MXC_ZERO) {
        return fle_mx_zero(f64, a->sign == b->sign ? a->sign : rc == 1);
    }
    if (a->cls == FLE_MXC_ZERO) {
        return fle_mx_pack(f64, b->sign, b->e, (fle_mxu128)b->m << 64, mx, fl);
    }
    if (b->cls == FLE_MXC_ZERO) {
        return fle_mx_pack(f64, a->sign, a->e, (fle_mxu128)a->m << 64, mx, fl);
    }
    sa = (fle_mxu128)a->m << 63;
    sb = (fle_mxu128)b->m << 63;
    ea = a->e;
    eb = b->e;
    sign = a->sign;
    if (ea < eb || (ea == eb && a->m < b->m)) {
        fle_mxu128 t = sa;
        sa = sb;
        sb = t;
        d = ea;
        ea = eb;
        eb = d;
        sign = b->sign;
    }
    d = ea - eb;
    if (d > 0) {
        sb = d >= 127 ? 1 : ((sb >> d) | ((sb << (128 - d)) != 0));
    }
    if (a->sign == b->sign) {
        s = sa + sb;
    } else {
        s = sa - sb;
        if (s == 0) {
            return fle_mx_zero(f64, rc == 1);
        }
    }
    lz = fle_mx_clz128(s);
    s <<= lz;
    return fle_mx_pack(f64, sign, ea + 1 - lz, s, mx, fl);
}

static inline uint64_t fle_mx_mul(int f64, const fle_mxv *a, const fle_mxv *b, uint32_t mx, uint32_t *fl) {
    int sign = a->sign ^ b->sign;
    fle_mxu128 p;
    int32_t e;
    if (a->cls == FLE_MXC_INF || b->cls == FLE_MXC_INF) {
        if (a->cls == FLE_MXC_ZERO || b->cls == FLE_MXC_ZERO) {
            *fl |= FLE_MX_IE;
            return fle_mx_indef(f64);
        }
        if (a->den | b->den) {
            *fl |= FLE_MX_DE;
        }
        return fle_mx_inf(f64, sign);
    }
    if (a->den | b->den) {
        *fl |= FLE_MX_DE;
    }
    if (a->cls == FLE_MXC_ZERO || b->cls == FLE_MXC_ZERO) {
        return fle_mx_zero(f64, sign);
    }
    p = (fle_mxu128)a->m * b->m;
    e = a->e + b->e;
    if (p >> 127) {
        e += 1;
    } else {
        p <<= 1;
    }
    return fle_mx_pack(f64, sign, e, p, mx, fl);
}

static inline uint64_t fle_mx_div(int f64, const fle_mxv *a, const fle_mxv *b, uint32_t mx, uint32_t *fl) {
    int sign = a->sign ^ b->sign;
    fle_mxu128 n, q;
    uint64_t r;
    int32_t e;
    if (a->cls == FLE_MXC_INF) {
        if (b->cls == FLE_MXC_INF) {
            *fl |= FLE_MX_IE;
            return fle_mx_indef(f64);
        }
        if (b->den) {
            *fl |= FLE_MX_DE;
        }
        return fle_mx_inf(f64, sign);
    }
    if (b->cls == FLE_MXC_INF) {
        if (a->den) {
            *fl |= FLE_MX_DE;
        }
        return fle_mx_zero(f64, sign);
    }
    if (b->cls == FLE_MXC_ZERO) {
        if (a->cls == FLE_MXC_ZERO) {
            *fl |= FLE_MX_IE;
            return fle_mx_indef(f64);
        }
        *fl |= FLE_MX_ZE;
        return fle_mx_inf(f64, sign);
    }
    if (a->den | b->den) {
        *fl |= FLE_MX_DE;
    }
    if (a->cls == FLE_MXC_ZERO) {
        return fle_mx_zero(f64, sign);
    }
    n = (fle_mxu128)a->m << 64;
    q = n / b->m;
    r = (uint64_t)(n - q * b->m);
    e = a->e - b->e;
    if (q >> 64) {
        q <<= 63;
    } else {
        q <<= 64;
        e -= 1;
    }
    if (r) {
        q |= 1;
    }
    return fle_mx_pack(f64, sign, e, q, mx, fl);
}

/* floor(sqrt(x)) for x < 2^128; *rem_nonzero tells whether x was not a perfect square */
static inline uint64_t fle_mx_isqrt(fle_mxu128 x, int *rem_nonzero) {
    fle_mxu128 res = 0, bit = (fle_mxu128)1 << 126;
    while (bit > x) {
        bit >>= 2;
    }
    while (bit) {
        if (x >= res + bit) {
            x -= res + bit;
            res = (res >> 1) + bit;
        } else {
            res >>= 1;
        }
        bit >>= 2;
    }
    *rem_nonzero = x != 0;
    return (uint64_t)res;
}

static inline uint64_t fle_mx_sqrt(int f64, uint64_t bb, uint32_t mx, uint32_t *fl) {
    fle_mxv b;
    fle_mxu128 r;
    uint64_t root;
    int s, nz, top;
    int32_t e;
    fle_mx_unpack(&b, bb, f64, mx);
    if (b.cls >= FLE_MXC_QNAN) {
        if (b.cls == FLE_MXC_SNAN) {
            *fl |= FLE_MX_IE;
        }
        return fle_mx_quiet(bb, f64);
    }
    if (b.cls == FLE_MXC_ZERO) {
        return fle_mx_zero(f64, b.sign);
    }
    if (b.sign) {
        *fl |= FLE_MX_IE;
        return fle_mx_indef(f64);
    }
    if (b.cls == FLE_MXC_INF) {
        return fle_mx_inf(f64, 0);
    }
    if (b.den) {
        *fl |= FLE_MX_DE;
    }
    s = (b.e & 1) ? 62 : 63;
    r = (fle_mxu128)b.m << s;
    root = fle_mx_isqrt(r, &nz);
    top = 63 - fle_mx_clz64(root);
    e = (b.e - 63 - s) / 2 + top;
    {
        fle_mxu128 sig = (fle_mxu128)root << (127 - top);
        if (nz) {
            sig |= 1;
        }
        return fle_mx_pack(f64, 0, e, sig, mx, fl);
    }
}

/* -1, 0, 1: a < b, a == b, a > b (zeros are equal); 2: unordered */
static inline int fle_mx_order(const fle_mxv *a, const fle_mxv *b) {
    int mag;
    if (a->cls >= FLE_MXC_QNAN || b->cls >= FLE_MXC_QNAN) {
        return 2;
    }
    if (a->cls == FLE_MXC_ZERO && b->cls == FLE_MXC_ZERO) {
        return 0;
    }
    if (a->cls == FLE_MXC_ZERO) {
        return b->sign ? 1 : -1;
    }
    if (b->cls == FLE_MXC_ZERO) {
        return a->sign ? -1 : 1;
    }
    if (a->sign != b->sign) {
        return a->sign ? -1 : 1;
    }
    if (a->cls == FLE_MXC_INF || b->cls == FLE_MXC_INF) {
        mag = (a->cls == FLE_MXC_INF) - (b->cls == FLE_MXC_INF);
    } else if (a->e != b->e) {
        mag = a->e > b->e ? 1 : -1;
    } else {
        mag = a->m > b->m ? 1 : a->m < b->m ? -1 : 0;
    }
    return a->sign ? -mag : mag;
}

static inline uint64_t fle_mx_minmax(int is_max, int f64, uint64_t ab, uint64_t bb, uint32_t mx, uint32_t *fl) {
    fle_mxv a, b;
    int c;
    fle_mx_unpack(&a, ab, f64, mx);
    fle_mx_unpack(&b, bb, f64, mx);
    c = fle_mx_order(&a, &b);
    /* a denormal that DAZ turned into zero is returned as a zero of the same sign */
    if (a.cls == FLE_MXC_ZERO) {
        ab = fle_mx_zero(f64, a.sign);
    }
    if (b.cls == FLE_MXC_ZERO) {
        bb = fle_mx_zero(f64, b.sign);
    }
    if (c == 2) {
        *fl |= FLE_MX_IE;
        return bb;
    }
    if (a.den | b.den) {
        *fl |= FLE_MX_DE;
    }
    if (is_max) {
        return c > 0 ? ab : bb;
    }
    return c < 0 ? ab : bb;
}

/* dst = op(a, b) for one lane; f64 selects binary64 (else binary32, in the low 32 bits). SQRT uses b only. */
static inline uint64_t fle_mx_arith(int op, int f64, uint64_t ab, uint64_t bb, uint32_t mx, uint32_t *fl) {
    fle_mxv a, b;
    if (op == FLE_MXO_SQRT) {
        return fle_mx_sqrt(f64, bb, mx, fl);
    }
    if (op >= FLE_MXO_MIN) {
        return fle_mx_minmax(op == FLE_MXO_MAX, f64, ab, bb, mx, fl);
    }
    fle_mx_unpack(&a, ab, f64, mx);
    fle_mx_unpack(&b, bb, f64, mx);
    if (a.cls >= FLE_MXC_QNAN || b.cls >= FLE_MXC_QNAN) {
        if (a.cls == FLE_MXC_SNAN || b.cls == FLE_MXC_SNAN) {
            *fl |= FLE_MX_IE;
        }
        return fle_mx_quiet(a.cls >= FLE_MXC_QNAN ? ab : bb, f64);
    }
    if (op == FLE_MXO_SUB) {
        b.sign ^= 1;
        return fle_mx_add(f64, &a, &b, mx, fl);
    }
    if (op == FLE_MXO_ADD) {
        return fle_mx_add(f64, &a, &b, mx, fl);
    }
    if (op == FLE_MXO_MUL) {
        return fle_mx_mul(f64, &a, &b, mx, fl);
    }
    return fle_mx_div(f64, &a, &b, mx, fl);
}

/* ---------------------------------------------------------------------------------------------------------- */
/* conversions */

/* integer rounding of |m * 2^(e-63)| for e < 64: returns the integer, sets *g (guard) and *st (sticky) */
static inline uint64_t fle_mx_intpart(uint64_t m, int32_t e, int *g, int *st) {
    int sh = 63 - e;
    if (sh <= 0) {
        *g = 0;
        *st = 0;
        return m;
    }
    if (sh >= 65) {
        *g = 0;
        *st = 1;
        return 0;
    }
    if (sh == 64) {
        *g = (int)(m >> 63);
        *st = (m << 1) != 0;
        return 0;
    }
    *g = (int)((m >> (sh - 1)) & 1);
    *st = (m & ((UINT64_C(1) << (sh - 1)) - 1)) != 0;
    return m >> sh;
}

static inline uint64_t fle_mx_roundint(uint64_t i, int g, int st, int sign, int rc) {
    int inexact = g | st;
    int inc = rc == 0 ? (g && (st || (i & 1))) : rc == 1 ? (sign && inexact) : rc == 2 ? (!sign && inexact) : 0;
    return i + (uint64_t)inc;
}

/* CVTSS2SI CVTSD2SI CVTTSS2SI CVTTSD2SI CVTPS2DQ CVTTPS2DQ CVTPD2DQ CVTTPD2DQ, one lane. out64 selects a 64-bit result
 * (else 32-bit, returned zero-extended). Out of range, NaN and infinity give the integer indefinite and IE. */
static inline uint64_t fle_mx_f2i(int f64, uint64_t bits, int out64, int trunc, uint32_t mx, uint32_t *fl) {
    fle_mxv v;
    uint64_t indef = out64 ? UINT64_C(0x8000000000000000) : UINT64_C(0x80000000);
    uint64_t i, lim = out64 ? UINT64_C(0x8000000000000000) : UINT64_C(0x80000000);
    int g, st, rc = trunc ? 3 : (int)((mx >> 13) & 3u);
    fle_mx_unpack(&v, bits, f64, mx);
    if (v.cls >= FLE_MXC_QNAN || v.cls == FLE_MXC_INF) {
        *fl |= FLE_MX_IE;
        return indef;
    }
    if (v.cls == FLE_MXC_ZERO) {
        return 0;
    }
    if (v.e >= 64) {
        *fl |= FLE_MX_IE;
        return indef;
    }
    i = fle_mx_intpart(v.m, v.e, &g, &st);
    i = fle_mx_roundint(i, g, st, v.sign, rc);
    if (v.sign ? i > lim : i >= lim) {
        *fl |= FLE_MX_IE;
        return indef;
    }
    if (g | st) {
        *fl |= FLE_MX_PE;
    }
    i = v.sign ? (uint64_t)0 - i : i;
    return out64 ? i : (i & UINT64_C(0xffffffff));
}

/* CVTSI2SS CVTSI2SD CVTDQ2PS CVTDQ2PD, one value */
static inline uint64_t fle_mx_i2f(int f64, int in64, uint64_t v, uint32_t mx, uint32_t *fl) {
    int64_t s = in64 ? (int64_t)v : (int64_t)(int32_t)(uint32_t)v;
    uint64_t mag;
    int sign, lz;
    if (s == 0) {
        return 0;
    }
    sign = s < 0;
    mag = sign ? (uint64_t)0 - (uint64_t)s : (uint64_t)s;
    lz = fle_mx_clz64(mag);
    return fle_mx_pack(f64, sign, 63 - lz, (fle_mxu128)(mag << lz) << 64, mx, fl);
}

/* CVTSS2SD (src_f64 = 0) and CVTSD2SS (src_f64 = 1), one value */
static inline uint64_t fle_mx_f2f(int src_f64, uint64_t bits, uint32_t mx, uint32_t *fl) {
    fle_mxv v;
    int dst_f64 = !src_f64;
    fle_mx_unpack(&v, bits, src_f64, mx);
    if (v.cls >= FLE_MXC_QNAN) {
        uint64_t mant, sign = (uint64_t)v.sign;
        if (v.cls == FLE_MXC_SNAN) {
            *fl |= FLE_MX_IE;
        }
        if (src_f64) {
            mant = (bits & UINT64_C(0x000fffffffffffff)) >> 29;
            return (sign << 31) | UINT64_C(0x7f800000) | mant | UINT64_C(0x400000);
        }
        mant = (bits & UINT64_C(0x7fffff)) << 29;
        return (sign << 63) | UINT64_C(0x7ff0000000000000) | mant | UINT64_C(0x8000000000000);
    }
    if (v.cls == FLE_MXC_INF) {
        return fle_mx_inf(dst_f64, v.sign);
    }
    if (v.den) {
        *fl |= FLE_MX_DE;
    }
    if (v.cls == FLE_MXC_ZERO) {
        return fle_mx_zero(dst_f64, v.sign);
    }
    return fle_mx_pack(dst_f64, v.sign, v.e, (fle_mxu128)v.m << 64, mx, fl);
}

/* ROUNDSS ROUNDSD ROUNDPS ROUNDPD, one lane */
static inline uint64_t fle_mx_roundk(int f64, uint64_t bits, unsigned imm, uint32_t mx, uint32_t *fl) {
    fle_mxv v;
    int mb = f64 ? 52 : 23;
    int rc = (imm & 4u) ? (int)((mx >> 13) & 3u) : (int)(imm & 3u);
    uint64_t i;
    int g, st, lz;
    uint32_t dummy = 0;
    fle_mx_unpack(&v, bits, f64, mx);
    if (v.cls >= FLE_MXC_QNAN) {
        if (v.cls == FLE_MXC_SNAN) {
            *fl |= FLE_MX_IE;
        }
        return fle_mx_quiet(bits, f64);
    }
    if (v.cls == FLE_MXC_INF) {
        return bits;
    }
    if (v.cls == FLE_MXC_ZERO) {
        return fle_mx_zero(f64, v.sign);
    }
    if (v.e >= mb) {
        return bits;
    }
    i = fle_mx_intpart(v.m, v.e, &g, &st);
    i = fle_mx_roundint(i, g, st, v.sign, rc);
    if ((g | st) && !(imm & 8u)) {
        *fl |= FLE_MX_PE;
    }
    if (i == 0) {
        return fle_mx_zero(f64, v.sign);
    }
    lz = fle_mx_clz64(i);
    return fle_mx_pack(f64, v.sign, 63 - lz, (fle_mxu128)(i << lz) << 64, mx, &dummy);
}

/* ---------------------------------------------------------------------------------------------------------- */
/* instruction wrappers (the names and signatures generated C uses) */

/* Memory operands narrower than 128 bits, as the low part of an XMM value. The rest is zero and unused. */
static inline fle_xmm fle_mx_m32(uint64_t a) {
    fle_xmm v;
    v.u64[0] = 0;
    v.u64[1] = 0;
    v.u32[0] = (uint32_t)fle_ld(a, 4);
    return v;
}

static inline fle_xmm fle_mx_m64(uint64_t a) {
    fle_xmm v;
    v.u64[0] = fle_ld(a, 8);
    v.u64[1] = 0;
    return v;
}

/* dst = op(dst, src). Scalar forms keep the upper lanes of dst. */
static inline fle_xmm fle_mx_do2(int op, int f64, int scalar, fle_xmm d, fle_xmm s) {
    uint32_t fl = 0, mx = fle_mxcsr;
    fle_xmm r = d;
    if (f64) {
        r.u64[0] = fle_mx_arith(op, 1, d.u64[0], s.u64[0], mx, &fl);
        if (!scalar) {
            r.u64[1] = fle_mx_arith(op, 1, d.u64[1], s.u64[1], mx, &fl);
        }
    } else {
        int i, n = scalar ? 1 : 4;
        for (i = 0; i < n; i++) {
            r.u32[i] = (uint32_t)fle_mx_arith(op, 0, d.u32[i], s.u32[i], mx, &fl);
        }
    }
    fle_mx_fin(fl);
    return r;
}

#define FLE_MX_OP2(name, op, f64, scalar) \
    static inline fle_xmm name(fle_xmm d, fle_xmm s) { return fle_mx_do2(op, f64, scalar, d, s); }

FLE_MX_OP2(fle_mx_addss, FLE_MXO_ADD, 0, 1)
FLE_MX_OP2(fle_mx_addsd, FLE_MXO_ADD, 1, 1)
FLE_MX_OP2(fle_mx_addps, FLE_MXO_ADD, 0, 0)
FLE_MX_OP2(fle_mx_addpd, FLE_MXO_ADD, 1, 0)

/* HADDPD (SSE3, decision 0106). Intel SDM, HADDPD, 128-bit Legacy SSE version: DEST[63:0] := DEST[63:0] + DEST[127:64];
 * DEST[127:64] := SRC[63:0] + SRC[127:64]. Each sum is the ADDPD lane sum (same NaN, rounding, DAZ and flag rules), with the first operand
 * first. The status flags of both sums are ORed into MXCSR once. */
static inline fle_xmm fle_mx_haddpd(fle_xmm d, fle_xmm s) {
    uint32_t fl = 0, mx = fle_mxcsr;
    fle_xmm r;
    r.u64[0] = fle_mx_arith(FLE_MXO_ADD, 1, d.u64[0], d.u64[1], mx, &fl);
    r.u64[1] = fle_mx_arith(FLE_MXO_ADD, 1, s.u64[0], s.u64[1], mx, &fl);
    fle_mx_fin(fl);
    return r;
}
FLE_MX_OP2(fle_mx_subss, FLE_MXO_SUB, 0, 1)
FLE_MX_OP2(fle_mx_subsd, FLE_MXO_SUB, 1, 1)
FLE_MX_OP2(fle_mx_subps, FLE_MXO_SUB, 0, 0)
FLE_MX_OP2(fle_mx_subpd, FLE_MXO_SUB, 1, 0)
FLE_MX_OP2(fle_mx_mulss, FLE_MXO_MUL, 0, 1)
FLE_MX_OP2(fle_mx_mulsd, FLE_MXO_MUL, 1, 1)
FLE_MX_OP2(fle_mx_mulps, FLE_MXO_MUL, 0, 0)
FLE_MX_OP2(fle_mx_mulpd, FLE_MXO_MUL, 1, 0)
FLE_MX_OP2(fle_mx_divss, FLE_MXO_DIV, 0, 1)
FLE_MX_OP2(fle_mx_divsd, FLE_MXO_DIV, 1, 1)
FLE_MX_OP2(fle_mx_divps, FLE_MXO_DIV, 0, 0)
FLE_MX_OP2(fle_mx_divpd, FLE_MXO_DIV, 1, 0)
FLE_MX_OP2(fle_mx_minss, FLE_MXO_MIN, 0, 1)
FLE_MX_OP2(fle_mx_minsd, FLE_MXO_MIN, 1, 1)
FLE_MX_OP2(fle_mx_minps, FLE_MXO_MIN, 0, 0)
FLE_MX_OP2(fle_mx_minpd, FLE_MXO_MIN, 1, 0)
FLE_MX_OP2(fle_mx_maxss, FLE_MXO_MAX, 0, 1)
FLE_MX_OP2(fle_mx_maxsd, FLE_MXO_MAX, 1, 1)
FLE_MX_OP2(fle_mx_maxps, FLE_MXO_MAX, 0, 0)
FLE_MX_OP2(fle_mx_maxpd, FLE_MXO_MAX, 1, 0)
FLE_MX_OP2(fle_mx_sqrtss, FLE_MXO_SQRT, 0, 1)
FLE_MX_OP2(fle_mx_sqrtsd, FLE_MXO_SQRT, 1, 1)
FLE_MX_OP2(fle_mx_sqrtps, FLE_MXO_SQRT, 0, 0)
FLE_MX_OP2(fle_mx_sqrtpd, FLE_MXO_SQRT, 1, 0)

/* floating-point to floating-point conversions */
static inline fle_xmm fle_mx_cvtss2sd(fle_xmm d, fle_xmm s) {
    uint32_t fl = 0;
    fle_xmm r = d;
    r.u64[0] = fle_mx_f2f(0, s.u32[0], fle_mxcsr, &fl);
    fle_mx_fin(fl);
    return r;
}

static inline fle_xmm fle_mx_cvtsd2ss(fle_xmm d, fle_xmm s) {
    uint32_t fl = 0;
    fle_xmm r = d;
    r.u32[0] = (uint32_t)fle_mx_f2f(1, s.u64[0], fle_mxcsr, &fl);
    fle_mx_fin(fl);
    return r;
}

static inline fle_xmm fle_mx_cvtps2pd(fle_xmm d, fle_xmm s) {
    uint32_t fl = 0;
    fle_xmm r = d;
    r.u64[0] = fle_mx_f2f(0, s.u32[0], fle_mxcsr, &fl);
    r.u64[1] = fle_mx_f2f(0, s.u32[1], fle_mxcsr, &fl);
    fle_mx_fin(fl);
    return r;
}

static inline fle_xmm fle_mx_cvtpd2ps(fle_xmm d, fle_xmm s) {
    uint32_t fl = 0;
    fle_xmm r = d;
    r.u32[0] = (uint32_t)fle_mx_f2f(1, s.u64[0], fle_mxcsr, &fl);
    r.u32[1] = (uint32_t)fle_mx_f2f(1, s.u64[1], fle_mxcsr, &fl);
    r.u64[1] = 0;
    fle_mx_fin(fl);
    return r;
}

static inline fle_xmm fle_mx_cvtdq2ps(fle_xmm d, fle_xmm s) {
    uint32_t fl = 0;
    fle_xmm r = d;
    int i;
    for (i = 0; i < 4; i++) {
        r.u32[i] = (uint32_t)fle_mx_i2f(0, 0, s.u32[i], fle_mxcsr, &fl);
    }
    fle_mx_fin(fl);
    return r;
}

static inline fle_xmm fle_mx_cvtdq2pd(fle_xmm d, fle_xmm s) {
    uint32_t fl = 0;
    fle_xmm r = d;
    r.u64[0] = fle_mx_i2f(1, 0, s.u32[0], fle_mxcsr, &fl);
    r.u64[1] = fle_mx_i2f(1, 0, s.u32[1], fle_mxcsr, &fl);
    fle_mx_fin(fl);
    return r;
}

static inline fle_xmm fle_mx_cvtps2dq_k(fle_xmm d, fle_xmm s, int trunc) {
    uint32_t fl = 0;
    fle_xmm r = d;
    int i;
    for (i = 0; i < 4; i++) {
        r.u32[i] = (uint32_t)fle_mx_f2i(0, s.u32[i], 0, trunc, fle_mxcsr, &fl);
    }
    fle_mx_fin(fl);
    return r;
}

static inline fle_xmm fle_mx_cvtps2dq(fle_xmm d, fle_xmm s) {
    return fle_mx_cvtps2dq_k(d, s, 0);
}

static inline fle_xmm fle_mx_cvttps2dq(fle_xmm d, fle_xmm s) {
    return fle_mx_cvtps2dq_k(d, s, 1);
}

static inline fle_xmm fle_mx_cvtpd2dq_k(fle_xmm d, fle_xmm s, int trunc) {
    uint32_t fl = 0;
    fle_xmm r = d;
    r.u32[0] = (uint32_t)fle_mx_f2i(1, s.u64[0], 0, trunc, fle_mxcsr, &fl);
    r.u32[1] = (uint32_t)fle_mx_f2i(1, s.u64[1], 0, trunc, fle_mxcsr, &fl);
    r.u64[1] = 0;
    fle_mx_fin(fl);
    return r;
}

static inline fle_xmm fle_mx_cvtpd2dq(fle_xmm d, fle_xmm s) {
    return fle_mx_cvtpd2dq_k(d, s, 0);
}

static inline fle_xmm fle_mx_cvttpd2dq(fle_xmm d, fle_xmm s) {
    return fle_mx_cvtpd2dq_k(d, s, 1);
}

/* CMPSS, CMPSD, CMPPS, CMPPD (imm8 predicate 0..7). Scalar forms keep the upper lanes of dst. */
static inline uint64_t fle_mx_cmpk(int f64, uint64_t ab, uint64_t bb, unsigned pred, uint32_t mx, uint32_t *fl) {
    fle_mxv a, b;
    int c, r = 0, signaling = pred == 1 || pred == 2 || pred == 5 || pred == 6;
    fle_mx_unpack(&a, ab, f64, mx);
    fle_mx_unpack(&b, bb, f64, mx);
    c = fle_mx_order(&a, &b);
    if (c == 2) {
        if (a.cls == FLE_MXC_SNAN || b.cls == FLE_MXC_SNAN || signaling) {
            *fl |= FLE_MX_IE;
        }
    } else if (a.den | b.den) {
        *fl |= FLE_MX_DE;
    }
    switch (pred & 7u) {
    case 0: r = c == 0; break;
    case 1: r = c == -1; break;
    case 2: r = c == -1 || c == 0; break;
    case 3: r = c == 2; break;
    case 4: r = c != 0; break;
    case 5: r = c != -1; break;
    case 6: r = !(c == -1 || c == 0); break;
    default: r = c != 2; break;
    }
    return r ? (f64 ? UINT64_MAX : UINT64_C(0xffffffff)) : 0;
}

static inline fle_xmm fle_mx_cmp(fle_xmm d, fle_xmm s, unsigned imm, int f64, int scalar) {
    uint32_t fl = 0, mx = fle_mxcsr;
    fle_xmm r = d;
    if (f64) {
        r.u64[0] = fle_mx_cmpk(1, d.u64[0], s.u64[0], imm, mx, &fl);
        if (!scalar) {
            r.u64[1] = fle_mx_cmpk(1, d.u64[1], s.u64[1], imm, mx, &fl);
        }
    } else {
        int i, n = scalar ? 1 : 4;
        for (i = 0; i < n; i++) {
            r.u32[i] = (uint32_t)fle_mx_cmpk(0, d.u32[i], s.u32[i], imm, mx, &fl);
        }
    }
    fle_mx_fin(fl);
    return r;
}

#define FLE_MX_CMPSS(d, s, imm) fle_mx_cmp((d), (s), (imm), 0, 1)
#define FLE_MX_CMPSD(d, s, imm) fle_mx_cmp((d), (s), (imm), 1, 1)
#define FLE_MX_CMPPS(d, s, imm) fle_mx_cmp((d), (s), (imm), 0, 0)
#define FLE_MX_CMPPD(d, s, imm) fle_mx_cmp((d), (s), (imm), 1, 0)

/* ROUNDSS, ROUNDSD, ROUNDPS, ROUNDPD (imm8). Scalar forms keep the upper lanes of dst. */
static inline fle_xmm fle_mx_round(fle_xmm d, fle_xmm s, unsigned imm, int f64, int scalar) {
    uint32_t fl = 0, mx = fle_mxcsr;
    fle_xmm r = d;
    if (f64) {
        r.u64[0] = fle_mx_roundk(1, s.u64[0], imm, mx, &fl);
        if (!scalar) {
            r.u64[1] = fle_mx_roundk(1, s.u64[1], imm, mx, &fl);
        }
    } else {
        int i, n = scalar ? 1 : 4;
        for (i = 0; i < n; i++) {
            r.u32[i] = (uint32_t)fle_mx_roundk(0, s.u32[i], imm, mx, &fl);
        }
    }
    fle_mx_fin(fl);
    return r;
}

#define FLE_MX_ROUNDSS(d, s, imm) fle_mx_round((d), (s), (imm), 0, 1)
#define FLE_MX_ROUNDSD(d, s, imm) fle_mx_round((d), (s), (imm), 1, 1)
#define FLE_MX_ROUNDPS(d, s, imm) fle_mx_round((d), (s), (imm), 0, 0)
#define FLE_MX_ROUNDPD(d, s, imm) fle_mx_round((d), (s), (imm), 1, 0)

/* CVTSI2SS, CVTSI2SD: the upper lanes of dst are kept. */
#define FLE_MX_CVTSI2(name, f64, in64, T) \
    static inline fle_xmm name(fle_xmm d, T v) \
    { \
        uint32_t fl = 0; \
        fle_xmm r = d; \
        if (f64) { \
            r.u64[0] = fle_mx_i2f(1, in64, (uint64_t)v, fle_mxcsr, &fl); \
        } else { \
            r.u32[0] = (uint32_t)fle_mx_i2f(0, in64, (uint64_t)v, fle_mxcsr, &fl); \
        } \
        fle_mx_fin(fl); \
        return r; \
    }

FLE_MX_CVTSI2(fle_mx_cvtsi2ss32, 0, 0, int32_t)
FLE_MX_CVTSI2(fle_mx_cvtsi2ss64, 0, 1, int64_t)
FLE_MX_CVTSI2(fle_mx_cvtsi2sd32, 1, 0, int32_t)
FLE_MX_CVTSI2(fle_mx_cvtsi2sd64, 1, 1, int64_t)

/* CVTSS2SI, CVTSD2SI, CVTTSS2SI, CVTTSD2SI to a 32-bit (zero-extended) or 64-bit GPR value. */
#define FLE_MX_CVT2SI(name, f64, out64, trunc) \
    static inline uint64_t name(fle_xmm s) \
    { \
        uint32_t fl = 0; \
        uint64_t r = fle_mx_f2i(f64, (f64) ? s.u64[0] : (uint64_t)s.u32[0], out64, trunc, fle_mxcsr, &fl); \
        fle_mx_fin(fl); \
        return r; \
    }

FLE_MX_CVT2SI(fle_mx_cvtss2si32, 0, 0, 0)
FLE_MX_CVT2SI(fle_mx_cvtss2si64, 0, 1, 0)
FLE_MX_CVT2SI(fle_mx_cvtsd2si32, 1, 0, 0)
FLE_MX_CVT2SI(fle_mx_cvtsd2si64, 1, 1, 0)
FLE_MX_CVT2SI(fle_mx_cvttss2si32, 0, 0, 1)
FLE_MX_CVT2SI(fle_mx_cvttss2si64, 0, 1, 1)
FLE_MX_CVT2SI(fle_mx_cvttsd2si32, 1, 0, 1)
FLE_MX_CVT2SI(fle_mx_cvttsd2si64, 1, 1, 1)

/* UCOMISS, UCOMISD, COMISS, COMISD: ZF, PF, CF from the instruction; OF, SF, AF cleared (SDM). */
static inline void fle_mx_comi(fle_xmm x, fle_xmm y, int f64, int signaling) {
    uint32_t fl = 0;
    fle_mxv a, b;
    int c;
    fle_mx_unpack(&a, f64 ? x.u64[0] : (uint64_t)x.u32[0], f64, fle_mxcsr);
    fle_mx_unpack(&b, f64 ? y.u64[0] : (uint64_t)y.u32[0], f64, fle_mxcsr);
    c = fle_mx_order(&a, &b);
    if (c == 2) {
        if (a.cls == FLE_MXC_SNAN || b.cls == FLE_MXC_SNAN || signaling) {
            fl |= FLE_MX_IE;
        }
    } else if (a.den | b.den) {
        fl |= FLE_MX_DE;
    }
    fle_mx_fin(fl);
    FL.cf = c == 2 || c == -1;
    FL.pf = c == 2;
    FL.zf = c == 2 || c == 0;
    FL.of = 0;
    FL.sf = 0;
    FL.af = 0;
}

#define FLE_MX_COMI(name, f64, signaling) \
    static inline void name(fle_xmm a, fle_xmm b) { fle_mx_comi(a, b, f64, signaling); }

FLE_MX_COMI(fle_mx_ucomiss, 0, 0)
FLE_MX_COMI(fle_mx_ucomisd, 1, 0)
FLE_MX_COMI(fle_mx_comiss, 0, 1)
FLE_MX_COMI(fle_mx_comisd, 1, 1)

#endif

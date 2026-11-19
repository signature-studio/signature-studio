/* SSE/SSE2/SSE4.1 floating-point operations with the MXCSR exception flags, in portable C
 * (docs/decisions/0044-mxcsr-flags-portable.md; revises 0038, which ran each instruction on the host CPU).
 *
 * Header only, C99 + <stdint.h> + unsigned __int128. No asm, no <emmintrin.h>, no <fenv.h>. The integer path does
 * no float or double arithmetic: every operation is done on the bit patterns with integer arithmetic, so the result
 * does not depend on the host FP unit, the host MXCSR or the width of long (nothing here uses long or uintptr_t).
 * The fast path (decision 0143, below) does use float and double arithmetic; it relies on the host rounding to
 * nearest even (the default of C, of x86-64 SSE and of WebAssembly) and never reads or sets the host MXCSR.
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
 * Fast path (decision 0143): with the guest rounding control RC = 0 (nearest even; FLE_MX_FAST_CTL), a lane whose
 * operands are zero or ordinary normal numbers, and whose result is an ordinary normal number (or an exact zero), is
 * computed with the host's float and double operations, and only PE is decided, exactly, by an error-free
 * transformation (TwoSum, Dekker's TwoProduct, a remainder, or a comparison of the rounded and the exact value).
 * Every other lane takes the integer path above, unchanged. The host MXCSR, fenv and rounding mode are not read or
 * set: the host only has to run float and double operations in IEEE 754 round-to-nearest-even, without
 * extended precision, fast-math or contraction (checked below).
 *
 * fle_mxcsr has a weak definition here so that a program built from a header-only runtime links; the strong one
 * is in runtime/fle_mxcsr.c.
 */
#ifndef FLE_MXCSR_FLAGS_H
#define FLE_MXCSR_FLAGS_H

#include <float.h>
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

/* The integer path is entered rarely once the fast path exists (decision 0143): it is kept out of line, one copy per unit. */
#define FLE_MX_SLOW static inline __attribute__((noinline))

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
#if defined(__GNUC__)
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
FLE_MX_SLOW uint64_t fle_mx_arith(int op, int f64, uint64_t ab, uint64_t bb, uint32_t mx, uint32_t *fl) {
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
FLE_MX_SLOW uint64_t fle_mx_f2i(int f64, uint64_t bits, int out64, int trunc, uint32_t mx, uint32_t *fl) {
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
FLE_MX_SLOW uint64_t fle_mx_i2f(int f64, int in64, uint64_t v, uint32_t mx, uint32_t *fl) {
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
FLE_MX_SLOW uint64_t fle_mx_f2f(int src_f64, uint64_t bits, uint32_t mx, uint32_t *fl) {
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
/* fast path for ordinary values (decision 0143)
 *
 * A lane takes it when the guest control bits (mx & FLE_MX_FAST_CTL) are 0 (RC = 0) and the operands and the result are
 * "ordinary": zero or normal, inside the exponent windows below, and the result is a normal number above the smallest
 * normal (or an exact zero). No NaN, infinity, denormal, overflow, underflow, or tiny result is handled here, so no
 * flag but PE can be raised: IE needs a NaN, an infinity or an invalid operation (0/0, sqrt of a negative, 0 * inf);
 * DE a denormal operand; ZE a zero divisor; OE an overflowing result; UE a tiny result. DAZ and FTZ only change
 * denormal operands and tiny results, which never get here. PE is exact (see each function). A function returns 0
 * without touching *r or *fl when the lane must take the integer path.
 *
 * What the host must do (the C standard does not promise it, the targets do): round float and double operations to
 * nearest even, evaluate float as float and double as double (FLT_EVAL_METHOD 0), not contract a * b + c into one
 * fused operation, and not reassociate. fle_mxcsr, the host MXCSR and fenv are not read or written.
 */
#if !defined(FLE_MX_NOFAST) && defined(FLT_EVAL_METHOD) && FLT_EVAL_METHOD == 0 && !defined(__FAST_MATH__) && \
    (defined(__clang__) || !defined(__FP_FAST_FMA))
#define FLE_MX_FAST 1
#endif

/* The control bits of the guest MXCSR that keep a lane off the fast path when set: RC (14:13). FTZ (15) and DAZ (6)
 * are deliberately not in it: they change the result only for a denormal operand (DAZ) or a tiny result (FTZ), and
 * the fast path takes neither, so a lane that it accepts has the same bits and flags with them set or clear. The
 * engine runs with MXCSR 0x9fe0 (FTZ and DAZ set) almost all the time (measured, decision 0143), so a guard that
 * included them would never open. */
#define FLE_MX_FAST_CTL 0x6000u

/* The lane functions are called with a constant operation: inlined, the switch goes away and each instruction keeps only
 * its own fast path. */
#define FLE_MXF_AI static inline __attribute__((always_inline))

/* statistics (-DFLE_MX_STATS: hits and misses per instruction class, defined in runtime/fle_mxcsr.c) for measurement only (decision 0243: the operand trace is gone). Ids: arithmetic = op + 8 * f64 (0..14), CVTSS2SD 16, CVTSD2SS 17,
 * int to float 18 + 2 * f64 + in64, float to int 24 + 4 * f64 + 2 * out64 + trunc, COMISS 32, COMISD 34 (+1 for
 * the signaling forms). */
#ifdef FLE_MX_STATS
extern uint64_t fle_mx_stat[64][2];
#define FLE_MXS(id, hit) (fle_mx_stat[(id)][(hit) != 0]++)
#else
#define FLE_MXS(id, hit) ((void)0)
#endif

#ifdef FLE_MX_FAST

#if defined(__clang__)
#define FLE_MXF_NOCONTRACT _Pragma("clang fp contract(off)")
#else
#define FLE_MXF_NOCONTRACT
#endif

#if defined(__has_builtin)
#if __has_builtin(__builtin_elementwise_sqrt)
#define FLE_MXF_SQRT(x) __builtin_elementwise_sqrt(x)
#endif
#endif
#ifndef FLE_MXF_SQRT
#define FLE_MXF_SQRT(x) __builtin_sqrt(x)
#endif

/* Bit casts. __builtin_memcpy, not memcpy: the generated C is compiled with -fno-builtin, under which memcpy is a call
 * into libc (the profile of the first fast build showed 30% of the time there), and __builtin_memcpy is still a move. */
static inline double fle_mxf_d(uint64_t u) {
    double d;
    __builtin_memcpy(&d, &u, sizeof d);
    return d;
}

static inline uint64_t fle_mxf_u(double d) {
    uint64_t u;
    __builtin_memcpy(&u, &d, sizeof u);
    return u;
}

static inline float fle_mxf_f(uint32_t u) {
    float f;
    __builtin_memcpy(&f, &u, sizeof f);
    return f;
}

static inline uint32_t fle_mxf_w(float f) {
    uint32_t u;
    __builtin_memcpy(&u, &f, sizeof u);
    return u;
}

#define FLE_MXF_MINN64 UINT64_C(0x0010000000000000)
#define FLE_MXF_INF64 UINT64_C(0x7ff0000000000000)
#define FLE_MXF_MINN32 0x00800000u
#define FLE_MXF_INF32 0x7f800000u

/* a normal binary64 whose biased exponent field is in [lo, hi] (the sign is ignored) */
static inline int fle_mxf_n64(uint64_t x, uint32_t lo, uint32_t hi) {
    return (((uint32_t)(x >> 52) & 0x7ffu) - lo) <= (hi - lo);
}

/* a zero or a normal binary64 whose biased exponent field is in [lo, hi] */
static inline int fle_mxf_zn64(uint64_t x, uint32_t lo, uint32_t hi) {
    return fle_mxf_n64(x, lo, hi) || (x << 1) == 0;
}

static inline int fle_mxf_zn32(uint32_t x, uint32_t lo, uint32_t hi) {
    return (((x >> 23) & 0xffu) - lo) <= (hi - lo) || (x << 1) == 0;
}

/* zero, normal or infinite (not NaN, not denormal) */
static inline int fle_mxf_ord64(uint64_t x) {
    uint64_t a = x & UINT64_C(0x7fffffffffffffff);
    return a == 0 || (a - FLE_MXF_MINN64) <= (FLE_MXF_INF64 - FLE_MXF_MINN64);
}

static inline int fle_mxf_ord32(uint32_t x) {
    uint32_t a = x & 0x7fffffffu;
    return a == 0 || (a - FLE_MXF_MINN32) <= (FLE_MXF_INF32 - FLE_MXF_MINN32);
}

/* a nonzero result: finite, normal, and not the smallest normal itself (so it is not tiny before or after rounding) */
static inline int fle_mxf_res64(uint64_t x) {
    return ((x & UINT64_C(0x7fffffffffffffff)) - (FLE_MXF_MINN64 + 1)) < (FLE_MXF_INF64 - (FLE_MXF_MINN64 + 1));
}

static inline int fle_mxf_res32(uint32_t x) {
    return ((x & 0x7fffffffu) - (FLE_MXF_MINN32 + 1)) < (FLE_MXF_INF32 - (FLE_MXF_MINN32 + 1));
}

/* Knuth's TwoSum tail: s = fl(a + b) in round to nearest, no overflow: a + b == s + err exactly */
static inline double fle_mxf_twosum_err(double a, double b, double s) {
    FLE_MXF_NOCONTRACT
    double v;
    v = s - a;
    return (a - (s - v)) + (b - v);
}

#define FLE_MXF_PE(cond) (*fl |= (cond) ? FLE_MX_PE : 0u)

/* PE only matters when raising it changes something: PE is sticky, so when the guest MXCSR already has PE set and the
 * precision exception masked (the engine runs with 0x9fe0), the flag need not be decided; with PM clear a raised PE
 * stops the run, so then it must be. np = 1 when PE has to be decided. The result bits do not depend on it. */
#define FLE_MXF_NP(mx) (((mx) & (FLE_MX_PE | 0x1000u)) != (FLE_MX_PE | 0x1000u))

/* ADD (sub = 0), SUB (sub = 1). Operands: zero, or normal with |x| < 2^1023 (so that no TwoSum step overflows).
 * Result: an exact zero (a == -b, or two zeros, with the IEEE sign), or a normal number above the smallest normal
 * (a denormal or smallest-normal result, an overflow or a tiny result take the integer path). With RC = 0 the
 * sum a + b is the correctly rounded value; PE is "the exact sum is not s", that is TwoSum's err != 0. A sum of
 * normal numbers is exact when it is a denormal or zero, so no flag is lost by sending those to the integer path. */
static inline int fle_mxf_add64(uint64_t ab, uint64_t bb, int sub, uint64_t *r, uint32_t *fl, unsigned np) {
    FLE_MXF_NOCONTRACT
    double a, b, s;
    uint64_t sb;
    if (!(fle_mxf_zn64(ab, 1u, 0x7fdu) && fle_mxf_zn64(bb, 1u, 0x7fdu))) {
        return 0;
    }
    a = fle_mxf_d(ab);
    b = fle_mxf_d(bb);
    if (sub) {
        b = -b;
    }
    s = a + b;
    sb = fle_mxf_u(s);
    if ((sb << 1) != 0 && !fle_mxf_res64(sb)) {
        return 0;
    }
    if (np) {
        FLE_MXF_PE(fle_mxf_twosum_err(a, b, s) != 0.0);
    }
    *r = sb;
    return 1;
}

#define FLE_MXF_M52 UINT64_C(0x000fffffffffffff)
#define FLE_MXF_H52 UINT64_C(0x0010000000000000)

/* MUL. Operands: zero or normal. A zero operand gives an exact zero (the sign is the xor). Otherwise p = a * b is
 * accepted when it is a normal number above the smallest normal. With the significands ma, mb (53 bits, the hidden
 * bit included) the exact product is the 106-bit integer P = ma * mb, and p keeps its top 53 bits rounded: the
 * low 52 bits are dropped when P < 2^105 and the low 53 bits when P >= 2^105. PE is "the dropped bits are not
 * all zero", and they all lie in the low 64 bits of P, which one 64-bit multiplication gives. Which of the two
 * cases it is follows from the exponent field of p: it is the sum of the operands' fields minus the bias, plus c,
 * c = 0 when P < 2^105 and no carry, c = 1 when P >= 2^105 or when rounding carried P up to 2^105. In the carry
 * case P < 2^105 is not a multiple of 2^52, so its low 53 bits are not zero either: the 53-bit test gives PE
 * there as well, and no result is ambiguous. c is never anything else for a normal p, and then the integer path runs. */
static inline int fle_mxf_mul64(uint64_t ab, uint64_t bb, uint64_t *r, uint32_t *fl, unsigned np) {
    FLE_MXF_NOCONTRACT
    double p;
    uint64_t pb, lo;
    uint32_t c;
    if (!(fle_mxf_zn64(ab, 1u, 0x7feu) && fle_mxf_zn64(bb, 1u, 0x7feu))) {
        return 0;
    }
    p = fle_mxf_d(ab) * fle_mxf_d(bb);
    pb = fle_mxf_u(p);
    if ((ab << 1) == 0 || (bb << 1) == 0) {
        *r = pb;
        return 1;
    }
    if (!fle_mxf_res64(pb)) {
        return 0;
    }
    if (np) {
        c = ((uint32_t)(pb >> 52) & 0x7ffu) + 1023u - (((uint32_t)(ab >> 52) & 0x7ffu) + ((uint32_t)(bb >> 52) & 0x7ffu));
        lo = ((ab & FLE_MXF_M52) | FLE_MXF_H52) * ((bb & FLE_MXF_M52) | FLE_MXF_H52);
        if (c == 0) {
            FLE_MXF_PE((lo & ((UINT64_C(1) << 52) - 1)) != 0);
        } else if (c == 1) {
            FLE_MXF_PE((lo & ((UINT64_C(1) << 53) - 1)) != 0);
        } else {
            return 0;
        }
    }
    *r = pb;
    return 1;
}

/* DIV. a zero or normal, b normal (a zero divisor takes the integer path). A zero dividend gives an exact zero.
 * Otherwise q = a / b is accepted when it is a normal number above the smallest normal. PE is "q * b != a". With
 * a = ma * 2^(Ea - 1075) and likewise b and q (E = the exponent fields), q * b == a means mq * mb == ma * 2^s with
 * s = 1075 + Ea - Eb - Eq. The correctly rounded q has a relative error at most 2^-53, so the integer
 * D = mq * mb - ma * 2^s is at most ma * 2^s * 2^-53 < 2^s in magnitude; for s <= 62 that is below 2^63, so D == 0
 * exactly when D mod 2^64 == 0, and the low 64 bits of the two products decide it. */
static inline int fle_mxf_div64(uint64_t ab, uint64_t bb, uint64_t *r, uint32_t *fl, unsigned np) {
    FLE_MXF_NOCONTRACT
    double q;
    uint64_t qb;
    uint32_t s;
    if (!(fle_mxf_zn64(ab, 1u, 0x7feu) && fle_mxf_n64(bb, 1u, 0x7feu))) {
        return 0;
    }
    q = fle_mxf_d(ab) / fle_mxf_d(bb);
    qb = fle_mxf_u(q);
    if ((ab << 1) == 0) {
        *r = qb;
        return 1;
    }
    if (!fle_mxf_res64(qb)) {
        return 0;
    }
    if (np) {
        s = 1075u + ((uint32_t)(ab >> 52) & 0x7ffu) - ((uint32_t)(bb >> 52) & 0x7ffu) - ((uint32_t)(qb >> 52) & 0x7ffu);
        if (s > 62u) {
            return 0;
        }
        FLE_MXF_PE((((qb & FLE_MXF_M52) | FLE_MXF_H52) * ((bb & FLE_MXF_M52) | FLE_MXF_H52)) !=
                   (((ab & FLE_MXF_M52) | FLE_MXF_H52) << s));
    }
    *r = qb;
    return 1;
}

/* SQRT of the second operand: +-0 gives itself; a negative number takes the integer path (IE); a positive normal
 * number has a normal root. PE is "q * q != a", decided as for the quotient with s = 1075 + Ea - 2 * Eq; the
 * relative error of q * q is at most 2^-52, so D is below 2^(s+1) in magnitude and s <= 61 is enough. */
static inline int fle_mxf_sqrt64(uint64_t bb, uint64_t *r, uint32_t *fl, unsigned np) {
    FLE_MXF_NOCONTRACT
    double q;
    uint64_t qb, mq;
    uint32_t s;
    if (!fle_mxf_zn64(bb, 1u, 0x7feu)) {
        return 0;
    }
    if ((bb << 1) == 0) {
        *r = bb;
        return 1;
    }
    if (bb >> 63) {
        return 0;
    }
    q = FLE_MXF_SQRT(fle_mxf_d(bb));
    qb = fle_mxf_u(q);
    if (np) {
        s = 1075u + ((uint32_t)(bb >> 52) & 0x7ffu) - 2u * ((uint32_t)(qb >> 52) & 0x7ffu);
        if (s > 61u) {
            return 0;
        }
        mq = (qb & FLE_MXF_M52) | FLE_MXF_H52;
        FLE_MXF_PE(mq * mq != (((bb & FLE_MXF_M52) | FLE_MXF_H52) << s));
    }
    *r = qb;
    return 1;
}

/* MIN, MAX of zeros, normals and infinities: no flag; equal values (and the two zeros) give the second operand. */
static inline int fle_mxf_minmax64(int is_max, uint64_t ab, uint64_t bb, uint64_t *r) {
    double a, b;
    if (!(fle_mxf_ord64(ab) && fle_mxf_ord64(bb))) {
        return 0;
    }
    a = fle_mxf_d(ab);
    b = fle_mxf_d(bb);
    *r = (is_max ? a > b : a < b) ? ab : bb;
    return 1;
}

static inline int fle_mxf_minmax32(int is_max, uint32_t ab, uint32_t bb, uint64_t *r) {
    float a, b;
    if (!(fle_mxf_ord32(ab) && fle_mxf_ord32(bb))) {
        return 0;
    }
    a = fle_mxf_f(ab);
    b = fle_mxf_f(bb);
    *r = (is_max ? a > b : a < b) ? ab : bb;
    return 1;
}

FLE_MXF_AI int fle_mxf_arith64(int op, uint64_t ab, uint64_t bb, uint64_t *r, uint32_t *fl, unsigned np) {
    switch (op) {
    case FLE_MXO_ADD: return fle_mxf_add64(ab, bb, 0, r, fl, np);
    case FLE_MXO_SUB: return fle_mxf_add64(ab, bb, 1, r, fl, np);
    case FLE_MXO_MUL: return fle_mxf_mul64(ab, bb, r, fl, np);
    case FLE_MXO_DIV: return fle_mxf_div64(ab, bb, r, fl, np);
    case FLE_MXO_SQRT: return fle_mxf_sqrt64(bb, r, fl, np);
    default: return fle_mxf_minmax64(op == FLE_MXO_MAX, ab, bb, r);
    }
}

/* binary32 operations are done in double. Operands: zero or normal (binary32 exponent field 1..254). The double
 * product of two binary32 values is exact (48 bits), the double quotient and root are correctly rounded, and the
 * double sum is correctly rounded with TwoSum giving its error. Rounding that double result to binary32 gives the
 * correctly rounded binary32 result: a double has 53 >= 2 * 24 + 2 bits, so the double rounding is innocuous for
 * + - * / and sqrt (Figueroa 1995; Boldo and Daumas 2003). A result is accepted when it is a binary32 normal above
 * the smallest normal or an exact zero. PE: the sum is inexact if the double sum is (TwoSum err != 0) or if the
 * rounding to binary32 changed it; the product and the quotient and the root are inexact iff the binary32 result
 * times the exact factor differs from the operand, computed exactly in double (24 + 24 bits). */
FLE_MXF_AI int fle_mxf_arith32(int op, uint32_t ab, uint32_t bb, uint64_t *r, uint32_t *fl, unsigned np) {
    FLE_MXF_NOCONTRACT
    double x, y, d, t;
    float f;
    uint32_t fb;
    int pe;
    if (op == FLE_MXO_MIN || op == FLE_MXO_MAX) {
        return fle_mxf_minmax32(op == FLE_MXO_MAX, ab, bb, r);
    }
    if (op == FLE_MXO_SQRT) {
        if (!fle_mxf_zn32(bb, 1u, 0xfeu)) {
            return 0;
        }
        if ((bb << 1) == 0) {
            *r = bb;
            return 1;
        }
        if (bb >> 31) {
            return 0;
        }
        x = (double)fle_mxf_f(bb);
        d = FLE_MXF_SQRT(x);
        f = (float)d;
        t = (double)f;
        if (np) {
            FLE_MXF_PE(t * t != x);
        }
        *r = fle_mxf_w(f);
        return 1;
    }
    if (!(fle_mxf_zn32(ab, 1u, 0xfeu) && fle_mxf_zn32(bb, 1u, 0xfeu))) {
        return 0;
    }
    x = (double)fle_mxf_f(ab);
    y = (double)fle_mxf_f(bb);
    if (op == FLE_MXO_MUL) {
        d = x * y;
        f = (float)d;
        fb = fle_mxf_w(f);
        pe = np && (double)f != d;
    } else if (op == FLE_MXO_DIV) {
        if ((bb << 1) == 0) {
            return 0;
        }
        d = x / y;
        f = (float)d;
        fb = fle_mxf_w(f);
        pe = np && (double)f * y != x;
    } else {
        if (op == FLE_MXO_SUB) {
            y = -y;
        }
        d = x + y;
        f = (float)d;
        fb = fle_mxf_w(f);
        pe = np && (fle_mxf_twosum_err(x, y, d) != 0.0 || (double)f != d);
    }
    if (!(fle_mxf_res32(fb) || d == 0.0)) {
        return 0;
    }
    FLE_MXF_PE(pe);
    *r = fb;
    return 1;
}

/* CVTSS2SD (src_f64 = 0): a binary32 zero or normal converts exactly. CVTSD2SS (src_f64 = 1): a zero, or a normal
 * with |x| in [2^-126, 2^128), rounds to nearest even; accepted when the binary32 result is a normal above the
 * smallest normal; PE is "the rounded value differs from x". */
static inline int fle_mxf_f2f(int src_f64, uint64_t bits, uint64_t *r, uint32_t *fl, unsigned np) {
    FLE_MXF_NOCONTRACT
    if (!src_f64) {
        if (!fle_mxf_zn32((uint32_t)bits, 1u, 0xfeu)) {
            return 0;
        }
        *r = fle_mxf_u((double)fle_mxf_f((uint32_t)bits));
        return 1;
    } else {
        double d;
        float f;
        uint32_t fb;
        if (!fle_mxf_zn64(bits, 897u, 1150u)) {
            return 0;
        }
        d = fle_mxf_d(bits);
        f = (float)d;
        fb = fle_mxf_w(f);
        if (!(fle_mxf_res32(fb) || d == 0.0)) {
            return 0;
        }
        if (np) {
            FLE_MXF_PE((double)f != d);
        }
        *r = fb;
        return 1;
    }
}

/* CVTSI2SS CVTSI2SD CVTDQ2PS CVTDQ2PD, one value: int32 to double is exact; the other three round to nearest even
 * (a conversion of an integer never overflows or underflows). PE is "the converted value differs from the integer",
 * compared in integers (for 64 bits the converted value may be 2^63, which does not fit; then it differs). */
static inline int fle_mxf_i2f(int f64, int in64, uint64_t v, uint64_t *r, uint32_t *fl, unsigned np) {
    FLE_MXF_NOCONTRACT
    int64_t s = in64 ? (int64_t)v : (int64_t)(int32_t)(uint32_t)v;
    double t;
    if (f64) {
        t = (double)s;
        *r = fle_mxf_u(t);
        if (in64 && np) {
            FLE_MXF_PE(t >= 9223372036854775808.0 || (int64_t)t != s);
        }
    } else {
        float f = (float)s;
        t = (double)f;
        *r = fle_mxf_w(f);
        if (in64) {
            if (np) {
                FLE_MXF_PE(t >= 9223372036854775808.0 || (int64_t)t != s);
            }
        } else if (np) {
            FLE_MXF_PE(t != (double)s);
        }
    }
    return 1;
}

/* CVTSS2SI CVTSD2SI CVTTSS2SI CVTTSD2SI CVTPS2DQ CVTTPS2DQ CVTPD2DQ CVTTPD2DQ, one lane. Operand: zero or normal
 * (a binary32 converts to double exactly). Truncation: |x| < 2^31 (32-bit result) or < 2^51 (64-bit result).
 * Rounding to nearest even: |x| < 2^31 - 1 or < 2^51, rounded by adding and subtracting 2^52 (exact for |x| < 2^52).
 * A larger value, and every value the integer path would turn into the integer indefinite, takes the integer path.
 * PE is "the integer differs from x". */
static inline int fle_mxf_f2i(int f64, uint64_t bits, int out64, int trunc, uint64_t *r, uint32_t *fl, unsigned np) {
    FLE_MXF_NOCONTRACT
    double x, t;
    int64_t i;
    if (f64 ? !fle_mxf_zn64(bits, 1u, 0x7feu) : !fle_mxf_zn32((uint32_t)bits, 1u, 0xfeu)) {
        return 0;
    }
    x = f64 ? fle_mxf_d(bits) : (double)fle_mxf_f((uint32_t)bits);
    if (out64) {
        if (!(x > -2251799813685248.0 && x < 2251799813685248.0)) {
            return 0;
        }
    } else if (trunc) {
        if (!(x > -2147483648.0 && x < 2147483648.0)) {
            return 0;
        }
    } else if (!(x > -2147483647.0 && x < 2147483647.0)) {
        return 0;
    }
    if (trunc) {
        i = (int64_t)x;
        t = (double)i;
    } else {
        double a = x < 0 ? -x : x;
        t = (a + 4503599627370496.0) - 4503599627370496.0;
        t = x < 0 ? -t : t;
        i = (int64_t)t;
    }
    if (np) {
        FLE_MXF_PE(t != x);
    }
    *r = out64 ? (uint64_t)i : (uint64_t)(uint32_t)(int32_t)i;
    return 1;
}

#endif /* FLE_MX_FAST */

/* One lane of ADD SUB MUL DIV MIN MAX SQRT: the fast path when it applies, else the integer path. */
FLE_MXF_AI uint64_t fle_mx_lane(int op, int f64, uint64_t a, uint64_t b, uint32_t mx, uint32_t *fl) {
#ifdef FLE_MX_FAST
    if (!(mx & FLE_MX_FAST_CTL)) {
        uint64_t r;
        if (f64 ? fle_mxf_arith64(op, a, b, &r, fl, FLE_MXF_NP(mx)) : fle_mxf_arith32(op, (uint32_t)a, (uint32_t)b, &r, fl, FLE_MXF_NP(mx))) {
            FLE_MXS(op + 8 * f64, 1);
            return r;
        }
    }
#endif
    FLE_MXS(op + 8 * f64, 0);
    return fle_mx_arith(op, f64, a, b, mx, fl);
}

FLE_MXF_AI uint64_t fle_mx_f2f_lane(int src_f64, uint64_t bits, uint32_t mx, uint32_t *fl) {
#ifdef FLE_MX_FAST
    if (!(mx & FLE_MX_FAST_CTL)) {
        uint64_t r;
        if (fle_mxf_f2f(src_f64, bits, &r, fl, FLE_MXF_NP(mx))) {
            FLE_MXS(16 + src_f64, 1);
            return r;
        }
    }
#endif
    FLE_MXS(16 + src_f64, 0);
    return fle_mx_f2f(src_f64, bits, mx, fl);
}

FLE_MXF_AI uint64_t fle_mx_i2f_lane(int f64, int in64, uint64_t v, uint32_t mx, uint32_t *fl) {
#ifdef FLE_MX_FAST
    if (!(mx & FLE_MX_FAST_CTL)) {
        uint64_t r;
        if (fle_mxf_i2f(f64, in64, v, &r, fl, FLE_MXF_NP(mx))) {
            FLE_MXS(18 + 2 * f64 + in64, 1);
            return r;
        }
    }
#endif
    FLE_MXS(18 + 2 * f64 + in64, 0);
    return fle_mx_i2f(f64, in64, v, mx, fl);
}

FLE_MXF_AI uint64_t fle_mx_f2i_lane(int f64, uint64_t bits, int out64, int trunc, uint32_t mx, uint32_t *fl) {
#ifdef FLE_MX_FAST
    if (!(mx & FLE_MX_FAST_CTL)) {
        uint64_t r;
        if (fle_mxf_f2i(f64, bits, out64, trunc, &r, fl, FLE_MXF_NP(mx))) {
            FLE_MXS(24 + 4 * f64 + 2 * out64 + trunc, 1);
            return r;
        }
    }
#endif
    FLE_MXS(24 + 4 * f64 + 2 * out64 + trunc, 0);
    return fle_mx_f2i(f64, bits, out64, trunc, mx, fl);
}

#include "fle_mxcsr_vec.h"

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
FLE_MXF_AI fle_xmm fle_mx_do2(int op, int f64, int scalar, fle_xmm d, fle_xmm s) {
    uint32_t fl = 0, mx = fle_mxcsr;
    fle_xmm r = d;
    if (f64) {
        r.u64[0] = fle_mx_lane(op, 1, d.u64[0], s.u64[0], mx, &fl);
        if (!scalar) {
            r.u64[1] = fle_mx_lane(op, 1, d.u64[1], s.u64[1], mx, &fl);
        }
    } else {
        int i, n = scalar ? 1 : 4;
        for (i = 0; i < n; i++) {
            r.u32[i] = (uint32_t)fle_mx_lane(op, 0, d.u32[i], s.u32[i], mx, &fl);
        }
    }
    if (fl) {
        fle_mx_fin(fl);
    }
    return r;
}

/* The one-instruction wrappers. With the guarded fast path of decision 0154 (fle_mxcsr_vec.h) an instruction whose
 * lanes are in the narrow windows is a host vector or scalar operation; every other instruction is fle_mx_do2, the
 * code of decision 0143 (per-lane fast path, then the integer path), out of line as fle_mx_do2_slow. */
#ifdef FLE_MX_VEC
#ifdef FLE_MX_RC
FLE_MX_SLOW void fle_mx_do2_slow_h(int op, int f64, int scalar, const fle_xmm *d, const fle_xmm *s, fle_xmm *r) {
    *r = fle_mx_do2(op, f64, scalar, *d, *s);
}
/* the id of an instruction: op + 8 * f64 + 16 * packed */
FLE_MX_SLOW void fle_mx_do2_slow_id_h(int id, const fle_xmm *d, const fle_xmm *s, fle_xmm *r) {
    *r = fle_mx_do2(id & 7, (id >> 3) & 1, !(id >> 4), *d, *s);
}
/* RC != 0 with PE set and PM masked: one indirect call into the directed-rounding code (fle_mxcsr_rc.h), which calls the integer implementation
 * above (fle_mx_do2_slow_h) itself when a lane is outside its windows. This function is a few instructions so that the common call does not pay
 * the frame of the integer implementation. */
FLE_MX_SLOW void fle_mx_do2_slow_p(int id, const fle_xmm *d, const fle_xmm *s, fle_xmm *r) {
    uint32_t mx = fle_mxcsr;
    fle_mxr_fn_t f = fle_mxrc_tab[FLE_MXR_RC(mx)][id]; /* row 0 (RC = 0) is empty */
    if (f && FLE_MXV_NP0X(mx)) {
        f(d, s, r, fle_mx_do2_slow_id_h);
        return;
    }
    fle_mx_do2_slow_id_h(id, d, s, r);
}
FLE_MX_SLOW fle_xmm fle_mx_do2_slow(int op, int f64, int scalar, fle_xmm d, fle_xmm s) {
    fle_xmm r;
    fle_mx_do2_slow_p(op + 8 * f64 + 16 * !scalar, &d, &s, &r);
    return r;
}
#else
FLE_MX_SLOW fle_xmm fle_mx_do2_slow(int op, int f64, int scalar, fle_xmm d, fle_xmm s) {
    return fle_mx_do2(op, f64, scalar, d, s);
}
FLE_MX_SLOW void fle_mx_do2_slow_p(int id, const fle_xmm *d, const fle_xmm *s, fle_xmm *r) {
    *r = fle_mx_do2(id & 7, (id >> 3) & 1, !(id >> 4), *d, *s);
}
#endif
#define FLE_MX_OP2(name, op, f64, scalar) \
    FLE_MXV_WAI fle_xmm name(fle_xmm d, fle_xmm s) { return fle_mxv_op2(op, f64, scalar, d, s); }
#else
#define FLE_MX_OP2(name, op, f64, scalar) \
    static inline fle_xmm name(fle_xmm d, fle_xmm s) { return fle_mx_do2(op, f64, scalar, d, s); }
#endif

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
    r.u64[0] = fle_mx_lane(FLE_MXO_ADD, 1, d.u64[0], d.u64[1], mx, &fl);
    r.u64[1] = fle_mx_lane(FLE_MXO_ADD, 1, s.u64[0], s.u64[1], mx, &fl);
    if (fl) {
        fle_mx_fin(fl);
    }
    return r;
}
/* Decision 0191 (PROVISIONAL): HADDPS, ADDSUBPS and ADDSUBPD (SSE3; HSUBPS, HSUBPD and LDDQU have no site in the census), with the pattern of HADDPD above (each result lane is the ADDPD / SUBPD lane operation: NaN, rounding, DAZ and flag rules,
 * first operand first; the flags of all lanes are ORed into MXCSR once). Intel SDM, 128-bit Legacy SSE versions:
 *   HADDPS : DEST[31:0] := DEST[31:0] + DEST[63:32]; DEST[63:32] := DEST[95:64] + DEST[127:96]; DEST[95:64] := SRC[31:0] + SRC[63:32]; DEST[127:96] := SRC[95:64] + SRC[127:96]
 *   ADDSUBPS: DEST[31:0] := DEST[31:0] - SRC[31:0]; DEST[63:32] := DEST[63:32] + SRC[63:32]; DEST[95:64] := DEST[95:64] - SRC[95:64]; DEST[127:96] := DEST[127:96] + SRC[127:96]
 *   ADDSUBPD: DEST[63:0] := DEST[63:0] - SRC[63:0]; DEST[127:64] := DEST[127:64] + SRC[127:64] */
static inline fle_xmm fle_mx_haddps(fle_xmm d, fle_xmm s) {
    uint32_t fl = 0, mx = fle_mxcsr;
    fle_xmm r;
    r.u32[0] = (uint32_t)fle_mx_lane(FLE_MXO_ADD, 0, d.u32[0], d.u32[1], mx, &fl);
    r.u32[1] = (uint32_t)fle_mx_lane(FLE_MXO_ADD, 0, d.u32[2], d.u32[3], mx, &fl);
    r.u32[2] = (uint32_t)fle_mx_lane(FLE_MXO_ADD, 0, s.u32[0], s.u32[1], mx, &fl);
    r.u32[3] = (uint32_t)fle_mx_lane(FLE_MXO_ADD, 0, s.u32[2], s.u32[3], mx, &fl);
    if (fl) {
        fle_mx_fin(fl);
    }
    return r;
}
static inline fle_xmm fle_mx_addsubps(fle_xmm d, fle_xmm s) {
    uint32_t fl = 0, mx = fle_mxcsr;
    fle_xmm r;
    r.u32[0] = (uint32_t)fle_mx_lane(FLE_MXO_SUB, 0, d.u32[0], s.u32[0], mx, &fl);
    r.u32[1] = (uint32_t)fle_mx_lane(FLE_MXO_ADD, 0, d.u32[1], s.u32[1], mx, &fl);
    r.u32[2] = (uint32_t)fle_mx_lane(FLE_MXO_SUB, 0, d.u32[2], s.u32[2], mx, &fl);
    r.u32[3] = (uint32_t)fle_mx_lane(FLE_MXO_ADD, 0, d.u32[3], s.u32[3], mx, &fl);
    if (fl) {
        fle_mx_fin(fl);
    }
    return r;
}
static inline fle_xmm fle_mx_addsubpd(fle_xmm d, fle_xmm s) {
    uint32_t fl = 0, mx = fle_mxcsr;
    fle_xmm r;
    r.u64[0] = fle_mx_lane(FLE_MXO_SUB, 1, d.u64[0], s.u64[0], mx, &fl);
    r.u64[1] = fle_mx_lane(FLE_MXO_ADD, 1, d.u64[1], s.u64[1], mx, &fl);
    if (fl) {
        fle_mx_fin(fl);
    }
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

/* floating-point to floating-point conversions. The integer implementation of each conversion is out of line when the directed-rounding code
 * (fle_mxcsr_rc.h) is in the build: one copy per translation unit instead of one per call site (decision 0172). */
/* Measured on a Serum 2 unit (decision 0172 §9): out of line the native -O2 text is 5.8% smaller than before, but the wasm -Oz lifted functions are 1.0% larger
 * (the wrappers, no longer big, are inlined into the callers); so out of line everywhere except wasm. */
#if defined(FLE_MX_RC) && !defined(__wasm__)
#define FLE_MXC_SLOW FLE_MX_SLOW
#else
#define FLE_MXC_SLOW static inline
#endif
FLE_MXC_SLOW fle_xmm fle_mx_cvtss2sd_slow(fle_xmm d, fle_xmm s) {
    uint32_t fl = 0;
    fle_xmm r = d;
    r.u64[0] = fle_mx_f2f_lane(0, s.u32[0], fle_mxcsr, &fl);
    if (fl) {
        fle_mx_fin(fl);
    }
    return r;
}

FLE_MXC_SLOW fle_xmm fle_mx_cvtsd2ss_slow(fle_xmm d, fle_xmm s) {
#ifdef FLE_MX_RC
    {
        fle_xmm rr;
        if (FLE_MXR_ON(fle_mxcsr) && fle_mxrc_cvtsd2ss(&d, &s, &rr)) {
            return rr;
        }
    }
#endif
    uint32_t fl = 0;
    fle_xmm r = d;
    r.u32[0] = (uint32_t)fle_mx_f2f_lane(1, s.u64[0], fle_mxcsr, &fl);
    if (fl) {
        fle_mx_fin(fl);
    }
    return r;
}

FLE_MXC_SLOW fle_xmm fle_mx_cvtps2pd_slow(fle_xmm d, fle_xmm s) {
    uint32_t fl = 0;
    fle_xmm r = d;
    r.u64[0] = fle_mx_f2f_lane(0, s.u32[0], fle_mxcsr, &fl);
    r.u64[1] = fle_mx_f2f_lane(0, s.u32[1], fle_mxcsr, &fl);
    if (fl) {
        fle_mx_fin(fl);
    }
    return r;
}

FLE_MXC_SLOW fle_xmm fle_mx_cvtpd2ps_slow(fle_xmm d, fle_xmm s) {
#ifdef FLE_MX_RC
    {
        fle_xmm rr;
        if (FLE_MXR_ON(fle_mxcsr) && fle_mxrc_cvtpd2ps(&d, &s, &rr)) {
            return rr;
        }
    }
#endif
    uint32_t fl = 0;
    fle_xmm r = d;
    r.u32[0] = (uint32_t)fle_mx_f2f_lane(1, s.u64[0], fle_mxcsr, &fl);
    r.u32[1] = (uint32_t)fle_mx_f2f_lane(1, s.u64[1], fle_mxcsr, &fl);
    r.u64[1] = 0;
    if (fl) {
        fle_mx_fin(fl);
    }
    return r;
}

FLE_MXC_SLOW fle_xmm fle_mx_cvtdq2ps_slow(fle_xmm d, fle_xmm s) {
#ifdef FLE_MX_RC
    {
        fle_xmm rr;
        if (FLE_MXR_ON(fle_mxcsr) && fle_mxrc_cvtdq2ps(&d, &s, &rr)) {
            return rr;
        }
    }
#endif
    uint32_t fl = 0;
    fle_xmm r = d;
    int i;
    for (i = 0; i < 4; i++) {
        r.u32[i] = (uint32_t)fle_mx_i2f_lane(0, 0, s.u32[i], fle_mxcsr, &fl);
    }
    if (fl) {
        fle_mx_fin(fl);
    }
    return r;
}

FLE_MXC_SLOW fle_xmm fle_mx_cvtdq2pd_slow(fle_xmm d, fle_xmm s) {
    uint32_t fl = 0;
    fle_xmm r = d;
    r.u64[0] = fle_mx_i2f_lane(1, 0, s.u32[0], fle_mxcsr, &fl);
    r.u64[1] = fle_mx_i2f_lane(1, 0, s.u32[1], fle_mxcsr, &fl);
    if (fl) {
        fle_mx_fin(fl);
    }
    return r;
}

FLE_MXC_SLOW fle_xmm fle_mx_cvtps2dq_k(fle_xmm d, fle_xmm s, int trunc) {
#ifdef FLE_MX_RC
    {
        fle_xmm rr;
        if (!trunc && FLE_MXR_ON(fle_mxcsr) && fle_mxrc_cvtps2dq(&d, &s, &rr)) {
            return rr;
        }
    }
#endif
    uint32_t fl = 0;
    fle_xmm r = d;
    int i;
    for (i = 0; i < 4; i++) {
        r.u32[i] = (uint32_t)fle_mx_f2i_lane(0, s.u32[i], 0, trunc, fle_mxcsr, &fl);
    }
    if (fl) {
        fle_mx_fin(fl);
    }
    return r;
}

static inline fle_xmm fle_mx_cvtps2dq_slow(fle_xmm d, fle_xmm s) {
    return fle_mx_cvtps2dq_k(d, s, 0);
}

static inline fle_xmm fle_mx_cvttps2dq_slow(fle_xmm d, fle_xmm s) {
    return fle_mx_cvtps2dq_k(d, s, 1);
}

FLE_MXC_SLOW fle_xmm fle_mx_cvtpd2dq_k(fle_xmm d, fle_xmm s, int trunc) {
#ifdef FLE_MX_RC
    {
        fle_xmm rr;
        if (!trunc && FLE_MXR_ON(fle_mxcsr) && fle_mxrc_cvtpd2dq(&d, &s, &rr)) {
            return rr;
        }
    }
#endif
    uint32_t fl = 0;
    fle_xmm r = d;
    r.u32[0] = (uint32_t)fle_mx_f2i_lane(1, s.u64[0], 0, trunc, fle_mxcsr, &fl);
    r.u32[1] = (uint32_t)fle_mx_f2i_lane(1, s.u64[1], 0, trunc, fle_mxcsr, &fl);
    r.u64[1] = 0;
    if (fl) {
        fle_mx_fin(fl);
    }
    return r;
}

static inline fle_xmm fle_mx_cvtpd2dq_slow(fle_xmm d, fle_xmm s) {
    return fle_mx_cvtpd2dq_k(d, s, 0);
}

static inline fle_xmm fle_mx_cvttpd2dq_slow(fle_xmm d, fle_xmm s) {
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

#ifdef FLE_MX_VEC
FLE_MX_SLOW
#else
static inline
#endif
fle_xmm fle_mx_cmp_slow(fle_xmm d, fle_xmm s, unsigned imm, int f64, int scalar) {
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

/* CMP*: operands that are not NaN and not denormal raise no flag and the host comparison is exact (fle_mxcsr_vec.h);
 * every other instruction takes the integer path. */
#ifdef FLE_MX_VEC
FLE_MXV_AI fle_xmm fle_mx_cmp(fle_xmm d, fle_xmm s, unsigned imm, int f64, int scalar) {
    fle_xmm r;
    if (FLE_MXV_CTL_OK(fle_mxcsr) && (f64 ? fle_mxv_cmp2(d, s, imm, scalar, &r) : fle_mxv_cmp4(d, s, imm, scalar, &r))) {
        FLE_MXVS(0, 32 + 4 * f64 + 2 * scalar);
        return r;
    }
    FLE_MXVS(2, 32 + 4 * f64 + 2 * scalar);
    return fle_mx_cmp_slow(d, s, imm, f64, scalar);
}
#else
static inline fle_xmm fle_mx_cmp(fle_xmm d, fle_xmm s, unsigned imm, int f64, int scalar) {
    return fle_mx_cmp_slow(d, s, imm, f64, scalar);
}
#endif

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
#ifdef FLE_MX_RC
#define FLE_MXR_CVTSI2_TRY(f64, in64, v) \
    { \
        uint64_t rb; \
        if (!((f64) && !(in64)) && FLE_MXR_ON(fle_mxcsr) && fle_mxrc_cvtsi2f((f64), (in64), (int64_t)(v), &rb)) { \
            if (f64) { \
                r.u64[0] = rb; \
            } else { \
                r.u32[0] = (uint32_t)rb; \
            } \
            return r; \
        } \
    }
#else
#define FLE_MXR_CVTSI2_TRY(f64, in64, v)
#endif
#define FLE_MX_CVTSI2(name, f64, in64, T) \
    FLE_MXC_SLOW fle_xmm name##_slow(fle_xmm d, T v) \
    { \
        uint32_t fl = 0; \
        fle_xmm r = d; \
        FLE_MXR_CVTSI2_TRY(f64, in64, v) \
        if (f64) { \
            r.u64[0] = fle_mx_i2f_lane(1, in64, (uint64_t)v, fle_mxcsr, &fl); \
        } else { \
            r.u32[0] = (uint32_t)fle_mx_i2f_lane(0, in64, (uint64_t)v, fle_mxcsr, &fl); \
        } \
        if (fl) { \
            fle_mx_fin(fl); \
        } \
        return r; \
    }

FLE_MX_CVTSI2(fle_mx_cvtsi2ss32, 0, 0, int32_t)
FLE_MX_CVTSI2(fle_mx_cvtsi2ss64, 0, 1, int64_t)
FLE_MX_CVTSI2(fle_mx_cvtsi2sd32, 1, 0, int32_t)
FLE_MX_CVTSI2(fle_mx_cvtsi2sd64, 1, 1, int64_t)

/* CVTSS2SI, CVTSD2SI, CVTTSS2SI, CVTTSD2SI to a 32-bit (zero-extended) or 64-bit GPR value. */
#ifdef FLE_MX_RC
#define FLE_MXR_CVT2SI_TRY(f64, out64, trunc, s) \
    { \
        uint64_t rb; \
        if (!(trunc) && FLE_MXR_ON(fle_mxcsr) && fle_mxrc_cvt2si((f64), (out64), (f64) ? (s).u64[0] : (uint64_t)(s).u32[0], &rb)) { \
            return rb; \
        } \
    }
#else
#define FLE_MXR_CVT2SI_TRY(f64, out64, trunc, s)
#endif
#define FLE_MX_CVT2SI(name, f64, out64, trunc) \
    FLE_MXC_SLOW uint64_t name##_slow(fle_xmm s) \
    { \
        uint32_t fl = 0; \
        FLE_MXR_CVT2SI_TRY(f64, out64, trunc, s) \
        uint64_t r = fle_mx_f2i_lane(f64, (f64) ? s.u64[0] : (uint64_t)s.u32[0], out64, trunc, fle_mxcsr, &fl); \
        if (fl) { \
            fle_mx_fin(fl); \
        } \
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
/* COMISS COMISD UCOMISS UCOMISD, the integer path */
FLE_MX_SLOW void fle_mx_comi_slow(fle_xmm x, fle_xmm y, int f64, int signaling) {
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

FLE_MXF_AI void fle_mx_comi(fle_xmm x, fle_xmm y, int f64, int signaling) {
#ifdef FLE_MX_FAST
    /* zeros, normals and infinities compare without a flag: ordered, no denormal operand (IE needs a NaN, DE a
     * denormal); the host's comparison of two such values is exact */
#define FLE_MXF_COMI_CTL (FLE_MX_FAST_CTL & ~0x6000u) /* the flags of a comparison do not depend on RC (decision 0172) */
    if (!(fle_mxcsr & FLE_MXF_COMI_CTL)) {
        uint64_t xb = f64 ? x.u64[0] : (uint64_t)x.u32[0], yb = f64 ? y.u64[0] : (uint64_t)y.u32[0];
        if (f64 ? (fle_mxf_ord64(xb) && fle_mxf_ord64(yb)) : (fle_mxf_ord32((uint32_t)xb) && fle_mxf_ord32((uint32_t)yb))) {
            int lt, eq;
            if (f64) {
                double p = fle_mxf_d(xb), q = fle_mxf_d(yb);
                lt = p < q;
                eq = p == q;
            } else {
                float p = fle_mxf_f((uint32_t)xb), q = fle_mxf_f((uint32_t)yb);
                lt = p < q;
                eq = p == q;
            }
            FLE_MXS(32 + 2 * f64 + signaling, 1);
            FL.cf = (uint8_t)lt;
            FL.pf = 0;
            FL.zf = (uint8_t)eq;
            FL.of = 0;
            FL.sf = 0;
            FL.af = 0;
            return;
        }
    }
#endif
    FLE_MXS(32 + 2 * f64 + signaling, 0);
    fle_mx_comi_slow(x, y, f64, signaling);
}

#define FLE_MX_COMI(name, f64, signaling) \
    static inline void name(fle_xmm a, fle_xmm b) { fle_mx_comi(a, b, f64, signaling); }

FLE_MX_COMI(fle_mx_ucomiss, 0, 0)
FLE_MX_COMI(fle_mx_ucomisd, 1, 0)
FLE_MX_COMI(fle_mx_comiss, 0, 1)
FLE_MX_COMI(fle_mx_comisd, 1, 1)

/* The conversion wrappers of generated C: the guarded fast path of decision 0154, else the code above (the *_slow names). */
#include "fle_mxcsr_cvt.h"

#endif

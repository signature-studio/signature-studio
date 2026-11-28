/* x87 arithmetic for fe-lift output: software extended precision. See fle_x87.h and
 * docs/decisions/0033-x87.md. Semantics: Intel SDM Vol. 1 ch. 4 and 8, Vol. 2 (each
 * instruction); where the SDM leaves a result open, the host CPU (AMD Ryzen 9 9950X)
 * is the reference and the choice is recorded in the decision file. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_x87ctl.h"
#include "fle_x87.h"

typedef unsigned __int128 u128;

fle_f80 fle_x87_r[8];
uint8_t fle_x87_mmx_active __attribute__((weak)) = 0;

#define EX_IE 0x01u
#define EX_DE 0x02u
#define EX_ZE 0x04u
#define EX_OE 0x08u
#define EX_UE 0x10u
#define EX_PE 0x20u

#define SW_C0 0x0100u
#define SW_C1 0x0200u
#define SW_C2 0x0400u
#define SW_C3 0x4000u

enum { K_ZERO, K_DEN, K_NORM, K_INF, K_QNAN, K_SNAN, K_UNSUP };

typedef struct {
    int k;
    int sign;
    int e;      /* unbiased exponent of the leading bit (ZERO, DEN, NORM) */
    uint64_t m; /* normalized significand, bit 63 set (ZERO, DEN, NORM); raw for NaN */
} fp;

typedef struct {
    int kind; /* 0 zero, 1 denormal, 2 normal, 3 infinity */
    int sign;
    int64_t e;
    uint64_t m;
} rr;

static unsigned X; /* exceptions raised by the current instruction */
static int keep_snan; /* arithmetic memory operands: an SNaN stays signaling for the NaN rules */

static const fle_f80 INDEF = { UINT64_C(0xC000000000000000), 0xFFFF };

static void fail(const char *m) {
    fprintf(stderr, "fle: x87: %s\n", m);
    abort();
}

static int clz64(uint64_t x) { return __builtin_clzll(x); }

static inline __attribute__((always_inline)) void mul64x64_128(uint64_t a, uint64_t b, uint64_t *hi_out, uint64_t *lo_out) {
    uint64_t a0 = (uint32_t)a, a1 = a >> 32;
    uint64_t b0 = (uint32_t)b, b1 = b >> 32;
    uint64_t p00 = a0 * b0;
    uint64_t p01 = a0 * b1;
    uint64_t p10 = a1 * b0;
    uint64_t p11 = a1 * b1;
    uint64_t mid = (p00 >> 32) + (uint32_t)p01 + (uint32_t)p10;
    *lo_out = (p00 & UINT64_C(0xffffffff)) | (mid << 32);
    *hi_out = p11 + (p01 >> 32) + (p10 >> 32) + (mid >> 32);
}

static int clz128(u128 x) {
    uint64_t h = (uint64_t)(x >> 64);
    return h ? __builtin_clzll(h) : 64 + __builtin_clzll((uint64_t)x);
}

static fle_f80 mk(int sign, unsigned e, uint64_t m) {
    fle_f80 r;
    r.m = m;
    r.se = (uint16_t)((sign ? 0x8000u : 0u) | e);
    return r;
}

/* ---- control, stack ---- */

static inline unsigned top(void) { return (fle_x87.sw >> 11) & 7u; }

static inline void set_top(unsigned t) { fle_x87.sw = (uint16_t)((fle_x87.sw & ~0x3800u) | ((t & 7u) << 11)); }

static inline unsigned phys(unsigned i) { return (top() + i) & 7u; }

static inline unsigned tag_of(unsigned p) { return (fle_x87.tw >> (2 * p)) & 3u; }

static inline void set_tag(unsigned p, unsigned t) {
    fle_x87.tw = (uint16_t)((fle_x87.tw & ~(3u << (2 * p))) | (t << (2 * p)));
}

static inline int rc_mode(void) { return (fle_x87.cw >> 10) & 3; }

static inline int pc_bits(void) {
    switch ((fle_x87.cw >> 8) & 3) {
    case 0: return 24;
    case 2: return 53;
    default: return 64; /* 01 is reserved; the host computes at 64 bits */
    }
}

static inline unsigned tag_for(fle_f80 v) {
    unsigned e = v.se & 0x7fffu;
    if (__builtin_expect(((e - 1u) < 0x7ffeu) & ((v.m >> 63) != 0u), 1)) return 0u;
    if (e == 0 && v.m == 0) return 1u;
    return 2u;
}

static inline void begin(void) {
    X = 0;
    if (__builtin_expect(fle_x87_mmx_active != 0, 0)) {
        if (fle_x87.tw == 0xFFFF) fle_x87_mmx_active = 0;
        else fail("x87 instruction while MMX state is live (no EMMS); MM and ST are not aliased");
    }
    if (__builtin_expect((fle_x87.sw & ~fle_x87.cw & 0x3Fu) != 0, 0)) fail("pending exception flag is unmasked (#MF not modelled)");
}

static inline __attribute__((always_inline)) void begin_fast(void) {
    if (__builtin_expect((fle_x87_mmx_active != 0) | ((fle_x87.sw & ~fle_x87.cw & 0x3Fu) != 0), 0)) {
        begin();
    }
}

static inline __attribute__((always_inline)) void begin_fast_cw(uint16_t sw, uint16_t cw) {
    if (__builtin_expect((fle_x87_mmx_active != 0) | ((sw & ~cw & 0x3Fu) != 0), 0)) {
        begin();
    }
}

/* Records the exceptions and C1, then stops when one of them is unmasked. */
static inline void commit(unsigned c1) {
    if (__builtin_expect((X & (EX_IE | EX_ZE)) != 0, 0)) X &= ~EX_DE; /* IE and ZE take precedence over DE (host, decision 0033) */
    fle_x87.sw = (uint16_t)((fle_x87.sw & ~SW_C1) | (c1 ? SW_C1 : 0u));
    fle_x87.sw |= (uint16_t)(X & 0x3Fu);
    if (__builtin_expect((X & ~fle_x87.cw & 0x3Fu) != 0, 0)) fail("unmasked exception (#MF not modelled)");
}

static inline void need_nonempty(unsigned i) {
    if (__builtin_expect(tag_of(phys(i)) == 3, 0)) fail("stack underflow (empty ST(i)); stack faults are not modelled");
}

static inline void need_room(void) {
    if (__builtin_expect(tag_of((top() + 7u) & 7u) != 3, 0)) fail("stack overflow (ST(7) not empty); stack faults are not modelled");
}

static inline void put(unsigned i, fle_f80 v) {
    unsigned p = phys(i);
    fle_x87_r[p] = v;
    set_tag(p, tag_for(v));
}

static inline void push(fle_f80 v) {
    set_top(top() - 1u);
    put(0, v);
}

static inline void pop(void) {
    unsigned t = top();
    set_tag(t, 3);
    set_top(t + 1u);
}

/* ---- unpack, round, pack ---- */

static fp unpack(fle_f80 x) {
    fp r;
    unsigned e = x.se & 0x7fffu;
    uint64_t m = x.m;
    r.sign = x.se >> 15;
    r.e = 0;
    r.m = m;
    if (e == 0x7fff) {
        if (!(m >> 63)) r.k = K_UNSUP;
        else if (!(m << 1)) r.k = K_INF;
        else r.k = ((m >> 62) & 1) ? K_QNAN : K_SNAN;
    } else if (e == 0) {
        if (!m) {
            r.k = K_ZERO;
        } else {
            r.k = K_DEN;
            if (m >> 63) {
                r.e = -16382;
            } else {
                int s = clz64(m);
                r.m = m << s;
                r.e = -16382 - s;
            }
        }
    } else if (!(m >> 63)) {
        r.k = K_UNSUP;
    } else {
        r.k = K_NORM;
        r.e = (int)e - 16383;
    }
    return r;
}

static int is_nan(fp a) { return a.k == K_QNAN || a.k == K_SNAN; }

/* M has bit 127 set: value = M * 2^(e-127). Rounds to pbits (quantum of denormals:
 * qbits), exponent range emin..emax. Sets PE, UE, OE in X. *up: the rounding went up. */
static rr round_pack(int sign, int64_t e, u128 M, int sticky, int pbits, int qbits, int64_t emin, int64_t emax, int rc, int *up) {
    rr r;
    int tiny = e < emin;
    int64_t u = e - pbits + 1;
    if (tiny) {
        /* tininess after rounding to pbits bits with an unbounded exponent (host, decision 0033) */
        unsigned sh0 = (unsigned)(128 - pbits);
        u128 rem0 = M & (((u128)1 << sh0) - 1);
        u128 k0 = M >> sh0;
        int rb0 = (int)((rem0 >> (sh0 - 1)) & 1);
        int st0 = (rem0 & (((u128)1 << (sh0 - 1)) - 1)) != 0 || sticky;
        int inc0 = 0;
        switch (rc) {
        case 0: inc0 = rb0 && (st0 || (k0 & 1)); break;
        case 1: inc0 = (rb0 | st0) && sign; break;
        case 2: inc0 = (rb0 | st0) && !sign; break;
        default: break;
        }
        if (e == emin - 1 && (k0 + (u128)inc0) >> pbits) tiny = 0;
    }
    int64_t uq = emin - (qbits - 1);
    int64_t nb;
    u128 keep;
    int rb, st, inc = 0, inexact;
    if (u < uq) u = uq;
    nb = e - u + 1;
    if (nb >= 1) {
        unsigned sh = (unsigned)(128 - nb);
        u128 rem = M & (((u128)1 << sh) - 1);
        keep = M >> sh;
        rb = (int)((rem >> (sh - 1)) & 1);
        st = (rem & (((u128)1 << (sh - 1)) - 1)) != 0 || sticky;
    } else if (nb == 0) {
        keep = 0;
        rb = 1;
        st = (M << 1) != 0 || sticky;
    } else {
        keep = 0;
        rb = 0;
        st = 1;
    }
    inexact = rb | st;
    switch (rc) {
    case 0: inc = rb && (st || (keep & 1)); break;
    case 1: inc = inexact && sign; break;
    case 2: inc = inexact && !sign; break;
    default: inc = 0; break;
    }
    keep += (u128)inc;
    *up = inc;
    if (inexact) X |= EX_PE;
    if (tiny && (inexact || !(fle_x87.cw & EX_UE))) X |= EX_UE;
    r.sign = sign;
    r.e = 0;
    r.m = 0;
    if (keep == 0) {
        r.kind = 0;
        return r;
    }
    {
        int L = 128 - clz128(keep);
        int64_t e2 = u + L - 1;
        uint64_t m = L <= 64 ? (uint64_t)keep << (64 - L) : (uint64_t)(keep >> (L - 64));
        if (e2 >= emin) {
            if (e2 > emax) {
                int toinf = rc == 0 || (rc == 1 && sign) || (rc == 2 && !sign);
                X |= EX_OE | EX_PE;
                if (toinf) {
                    r.kind = 3;
                    *up = 1;
                } else {
                    r.kind = 2;
                    r.e = emax;
                    r.m = ~UINT64_C(0) << (64 - pbits);
                    *up = 0;
                }
                return r;
            }
            r.kind = 2;
            r.e = e2;
            r.m = m;
            return r;
        }
        r.kind = 1;
        r.m = (uint64_t)(keep << (u - emin + 63));
        return r;
    }
}

static fle_f80 pack80(rr r) {
    switch (r.kind) {
    case 0: return mk(r.sign, 0, 0);
    case 1: return mk(r.sign, 0, r.m);
    case 2: return mk(r.sign, (unsigned)(r.e + 16383), r.m);
    default: return mk(r.sign, 0x7fff, UINT64_C(0x8000000000000000));
    }
}


/* f80 results (PC 24, 53 or 64): one rounding to pbits significant bits. A tiny result is
 * rounded on the grid of the pbits-bit denormals, 2^(emin - pbits + 1) (host measurement,
 * decision 0033). Tininess is detected before rounding. */
static fle_f80 round80(int sign, int64_t e, u128 M, int sticky, int pbits, int *up) {
    return pack80(round_pack(sign, e, M, sticky, pbits, pbits, -16382, 16383, rc_mode(), up));
}

/* Fast normal path for round80 when e is strictly inside [-16382, 16382] and bit 63 of hi is set. */
static fle_f80 round80_fast(int sign, int64_t e, uint64_t hi, uint64_t lo, int sticky, int pbits, int rc, int *up, unsigned *x_flags) {
    if (__builtin_expect(e < -16382 || e >= 16383, 0)) {
        X = *x_flags;
        {
            fle_f80 res = round80(sign, e, ((u128)hi << 64) | (u128)lo, sticky, pbits, up);
            *x_flags = X;
            return res;
        }
    }
    if (__builtin_expect(pbits == 64 && rc == 0, 1)) {
        int rb = (int)(lo >> 63);
        int st = ((lo << 1) != 0) | sticky;
        int inexact = rb | st;
        int inc = rb && (st || (hi & 1u));
        uint64_t m = hi + (uint64_t)inc;
        *up = inc;
        if (inexact) *x_flags |= EX_PE;
        if (__builtin_expect(m == 0 && inc, 0)) {
            e++;
            m = UINT64_C(0x8000000000000000);
        }
        return mk(sign, (unsigned)(e + 16383), m);
    }
    uint64_t keep, m;
    int rb, st, inc = 0;
    if (pbits == 64) {
        keep = hi;
        rb = (int)(lo >> 63);
        st = ((lo << 1) != 0) | sticky;
    } else if (pbits == 53) {
        keep = hi >> 11;
        rb = (int)((hi >> 10) & 1u);
        st = (((hi & 0x3ffu) | lo) != 0) | sticky;
    } else {
        keep = hi >> 40;
        rb = (int)((hi >> 39) & 1u);
        st = (((hi & ((UINT64_C(1) << 39) - 1u)) | lo) != 0) | sticky;
    }
    int inexact = rb | st;
    if (__builtin_expect(rc == 0, 1)) {
        inc = rb && (st || (keep & 1u));
    } else if (rc == 1) {
        inc = inexact && sign;
    } else if (rc == 2) {
        inc = inexact && !sign;
    }
    *up = inc;
    if (inexact) *x_flags |= EX_PE;
    keep += (uint64_t)inc;
    if (pbits == 64) {
        if (__builtin_expect(keep == 0 && inc, 0)) {
            e++;
            m = UINT64_C(0x8000000000000000);
        } else {
            m = keep;
        }
    } else if (pbits == 53) {
        if (__builtin_expect((keep >> 53) != 0, 0)) {
            e++;
            m = UINT64_C(0x8000000000000000);
        } else {
            m = keep << 11;
        }
    } else {
        if (__builtin_expect((keep >> 24) != 0, 0)) {
            e++;
            m = UINT64_C(0x8000000000000000);
        } else {
            m = keep << 40;
        }
    }
    return mk(sign, (unsigned)(e + 16383), m);
}

/* ---- conversions to and from f80 ---- */

static fle_f80 from_f32(uint32_t b) {
    int sign = (int)(b >> 31);
    unsigned e = (b >> 23) & 0xffu;
    uint32_t f = b & 0x7fffffu;
    if (e == 0xff) {
        if (!f) return mk(sign, 0x7fff, UINT64_C(0x8000000000000000));
        if (!(f >> 22)) X |= EX_IE;
        return mk(sign, 0x7fff, UINT64_C(0x8000000000000000) | ((!(f >> 22) && keep_snan) ? 0 : UINT64_C(0x4000000000000000)) | ((uint64_t)f << 40));
    }
    if (e == 0) {
        int s;
        if (!f) return mk(sign, 0, 0);
        X |= EX_DE;
        s = clz64(f);
        return mk(sign, (unsigned)(63 - s - 149 + 16383), (uint64_t)f << s);
    }
    return mk(sign, e - 127 + 16383, UINT64_C(0x8000000000000000) | ((uint64_t)f << 40));
}

static fle_f80 from_f64(uint64_t b) {
    int sign = (int)(b >> 63);
    unsigned e = (unsigned)((b >> 52) & 0x7ffu);
    uint64_t f = b & UINT64_C(0xfffffffffffff);
    if (e == 0x7ff) {
        if (!f) return mk(sign, 0x7fff, UINT64_C(0x8000000000000000));
        if (!(f >> 51)) X |= EX_IE;
        return mk(sign, 0x7fff, UINT64_C(0x8000000000000000) | ((!(f >> 51) && keep_snan) ? 0 : UINT64_C(0x4000000000000000)) | (f << 11));
    }
    if (e == 0) {
        int s;
        if (!f) return mk(sign, 0, 0);
        X |= EX_DE;
        s = clz64(f);
        return mk(sign, (unsigned)(63 - s - 1074 + 16383), f << s);
    }
    return mk(sign, e - 1023 + 16383, UINT64_C(0x8000000000000000) | (f << 11));
}

static fle_f80 from_int(int64_t v) {
    uint64_t mag;
    int s;
    if (v == 0) return mk(0, 0, 0);
    mag = v < 0 ? (uint64_t)0 - (uint64_t)v : (uint64_t)v;
    s = clz64(mag);
    return mk(v < 0, (unsigned)(63 - s + 16383), mag << s);
}

/* f80 to f32 (dbl = 0) or f64 (dbl = 1) bits. */
static uint64_t to_float(fle_f80 x, int dbl, int *up) {
    fp a = unpack(x);
    int fb = dbl ? 52 : 23;
    int eb = dbl ? 11 : 8;
    uint64_t sg = (uint64_t)a.sign << (fb + eb);
    uint64_t emax_f = ((uint64_t)1 << eb) - 1;
    *up = 0;
    switch (a.k) {
    case K_UNSUP:
        X |= EX_IE;
        return dbl ? UINT64_C(0xFFF8000000000000) : UINT64_C(0xFFC00000);
    case K_SNAN:
    case K_QNAN: {
        uint64_t f;
        if (a.k == K_SNAN) X |= EX_IE;
        f = (x.m >> (63 - fb)) & (((uint64_t)1 << fb) - 1);
        f |= (uint64_t)1 << (fb - 1);
        return sg | (emax_f << fb) | f;
    }
    case K_INF: return sg | (emax_f << fb);
    case K_ZERO: return sg;
    default: break;
    }
    {
        int64_t emin = dbl ? -1022 : -126, emax = dbl ? 1023 : 127;
        int bias = dbl ? 1023 : 127;
        if (__builtin_expect(a.k == K_NORM && a.e >= emin && a.e < emax, 1)) {
            int rc = rc_mode();
            unsigned sh = (unsigned)(63 - fb);
            uint64_t keep = a.m >> sh;
            uint64_t rem = a.m & ((UINT64_C(1) << sh) - 1u);
            int rb = (int)((rem >> (sh - 1u)) & 1u);
            int st = (rem & ((UINT64_C(1) << (sh - 1u)) - 1u)) != 0;
            int inexact = rb | st, inc = 0;
            if (__builtin_expect(rc == 0, 1)) {
                inc = rb && (st || (keep & 1u));
            } else if (rc == 1) {
                inc = inexact && a.sign;
            } else if (rc == 2) {
                inc = inexact && !a.sign;
            }
            *up = inc;
            if (inexact) X |= EX_PE;
            keep += (uint64_t)inc;
            int64_t e2 = a.e;
            if (__builtin_expect((keep >> (fb + 1)) != 0, 0)) {
                keep >>= 1;
                e2++;
            }
            return sg | ((uint64_t)(e2 + bias) << fb) | (keep & (((uint64_t)1 << fb) - 1u));
        }
        rr r = round_pack(a.sign, a.e, (u128)a.m << 64, 0, fb + 1, fb + 1, emin, emax, rc_mode(), up);
        switch (r.kind) {
        case 0: return sg;
        case 1: return sg | (r.m >> (63 - fb));
        case 2: return sg | ((uint64_t)(r.e + bias) << fb) | ((r.m >> (63 - fb)) & (((uint64_t)1 << fb) - 1));
        default: return sg | (emax_f << fb);
        }
    }
}

/* ---- loads and stores ---- */

static fle_f80 load80(uint64_t a) {
    fle_f80 v;
    v.m = fle_ld(a, 8);
    v.se = (uint16_t)fle_ld(a + 8, 2);
    return v;
}

static fle_f80 load_mem(uint64_t a, unsigned size) {
    if (size == 4) return from_f32((uint32_t)fle_ld(a, 4));
    if (size == 8) return from_f64(fle_ld(a, 8));
    if (size == 10) return load80(a);
    fail("bad operand size");
    return INDEF;
}

static int64_t load_int(uint64_t a, unsigned size) {
    if (size == 2) return (int16_t)fle_ld(a, 2);
    if (size == 4) return (int32_t)fle_ld(a, 4);
    if (size == 8) return (int64_t)fle_ld(a, 8);
    fail("bad operand size");
    return 0;
}

static __attribute__((noinline, cold)) void fld_m_slow(uint64_t a, unsigned size) {
    fle_f80 v;
    X = 0;
    v = load_mem(a, size);
    commit(0);
    push(v);
}

void fle_x87_fld_m(uint64_t a, unsigned size) {
    fle_f80 v;
    unsigned t = top(), p = (t - 1u) & 7u;
    unsigned tg = fle_x87.tw;
    begin_fast();
    if (__builtin_expect(((tg >> (2u * p)) & 3u) != 3u, 0)) {
        fail("stack overflow (ST(7) not empty); stack faults are not modelled");
    }
    if (__builtin_expect(size == 8, 1)) {
        uint64_t bits;
        if (__builtin_expect(((a | (a + 7u)) >> 32) == 0u, 1)) memcpy(&bits, (const void *)(uintptr_t)a, 8);
        else bits = fle_ld(a, 8);
        unsigned e = (unsigned)((bits >> 52) & 0x7ffu);
        if (__builtin_expect((e - 1u) < 0x7feu, 1)) {
            v.m = UINT64_C(0x8000000000000000) | ((bits & UINT64_C(0xfffffffffffff)) << 11);
            v.se = (uint16_t)(((bits >> 48) & 0x8000u) | (e + (16383u - 1023u)));
            fle_x87.sw = (uint16_t)((fle_x87.sw & ~(SW_C1 | 0x3800u)) | (p << 11));
            fle_x87_r[p] = v;
            fle_x87.tw = (uint16_t)(tg & ~(3u << (2u * p)));
            return;
        }
        if ((bits & UINT64_C(0x7fffffffffffffff)) == 0u) {
            fle_x87.sw = (uint16_t)((fle_x87.sw & ~(SW_C1 | 0x3800u)) | (p << 11));
            fle_x87_r[p] = mk((int)(bits >> 63), 0, 0);
            fle_x87.tw = (uint16_t)((tg & ~(3u << (2u * p))) | (1u << (2u * p)));
            return;
        }
    } else if (__builtin_expect(size == 4, 1)) {
        uint32_t bits;
        if (__builtin_expect(((a | (a + 3u)) >> 32) == 0u, 1)) memcpy(&bits, (const void *)(uintptr_t)a, 4);
        else bits = (uint32_t)fle_ld(a, 4);
        unsigned e = (bits >> 23) & 0xffu;
        if (__builtin_expect((e - 1u) < 0xfeu, 1)) {
            v.m = UINT64_C(0x8000000000000000) | ((uint64_t)(bits & 0x7fffffu) << 40);
            v.se = (uint16_t)(((bits >> 16) & 0x8000u) | (e + (16383u - 127u)));
            fle_x87.sw = (uint16_t)((fle_x87.sw & ~(SW_C1 | 0x3800u)) | (p << 11));
            fle_x87_r[p] = v;
            fle_x87.tw = (uint16_t)(tg & ~(3u << (2u * p)));
            return;
        }
        if ((bits & 0x7fffffffu) == 0u) {
            fle_x87.sw = (uint16_t)((fle_x87.sw & ~(SW_C1 | 0x3800u)) | (p << 11));
            fle_x87_r[p] = mk((int)(bits >> 31), 0, 0);
            fle_x87.tw = (uint16_t)((tg & ~(3u << (2u * p))) | (1u << (2u * p)));
            return;
        }
    }
    fld_m_slow(a, size);
}

void fle_x87_fild(uint64_t a, unsigned size) {
    fle_f80 v;
    begin();
    need_room();
    v = from_int(load_int(a, size));
    commit(0);
    push(v);
}

void fle_x87_fld_st(unsigned i) {
    unsigned t = top(), ps = (t + i) & 7u, p_new = (t - 1u) & 7u;
    unsigned tg = fle_x87.tw;
    begin_fast();
    if (__builtin_expect(((tg >> (2u * ps)) & 3u) == 3u, 0)) {
        fail("stack underflow (empty ST(i)); stack faults are not modelled");
    }
    if (__builtin_expect(((tg >> (2u * p_new)) & 3u) != 3u, 0)) {
        fail("stack overflow (ST(7) not empty); stack faults are not modelled");
    }
    {
        unsigned src_tag = (tg >> (2u * ps)) & 3u;
        fle_x87_r[p_new] = fle_x87_r[ps];
        fle_x87.tw = (uint16_t)((tg & ~(3u << (2u * p_new))) | (src_tag << (2u * p_new)));
        fle_x87.sw = (uint16_t)((fle_x87.sw & ~(SW_C1 | 0x3800u)) | (p_new << 11));
    }
}

void fle_x87_fld_const(unsigned k) {
    /* {value rounded to nearest, rounded down, rounded up}: the 66-bit internal constant of
     * the SDM rounded by RC (host measurement, decision 0033). RC 3 gives the rounded-down value. */
    static const fle_f80 C[7][3] = {
        { { UINT64_C(0x8000000000000000), 0x3fff }, { UINT64_C(0x8000000000000000), 0x3fff }, { UINT64_C(0x8000000000000000), 0x3fff } }, /* 1 */
        { { UINT64_C(0xd49a784bcd1b8afe), 0x4000 }, { UINT64_C(0xd49a784bcd1b8afe), 0x4000 }, { UINT64_C(0xd49a784bcd1b8aff), 0x4000 } }, /* log2 10 */
        { { UINT64_C(0xb8aa3b295c17f0bc), 0x3fff }, { UINT64_C(0xb8aa3b295c17f0bb), 0x3fff }, { UINT64_C(0xb8aa3b295c17f0bc), 0x3fff } }, /* log2 e */
        { { UINT64_C(0xc90fdaa22168c235), 0x4000 }, { UINT64_C(0xc90fdaa22168c234), 0x4000 }, { UINT64_C(0xc90fdaa22168c235), 0x4000 } }, /* pi */
        { { UINT64_C(0x9a209a84fbcff799), 0x3ffd }, { UINT64_C(0x9a209a84fbcff798), 0x3ffd }, { UINT64_C(0x9a209a84fbcff799), 0x3ffd } }, /* log10 2 */
        { { UINT64_C(0xb17217f7d1cf79ac), 0x3ffe }, { UINT64_C(0xb17217f7d1cf79ab), 0x3ffe }, { UINT64_C(0xb17217f7d1cf79ac), 0x3ffe } }, /* ln 2 */
        { { 0, 0 }, { 0, 0 }, { 0, 0 } },                                                                                                  /* 0 */
    };
    uint16_t sw = fle_x87.sw, cw = fle_x87.cw;
    unsigned t = (sw >> 11) & 7u, p = (t - 1u) & 7u;
    unsigned tg = fle_x87.tw;
    int rc = (cw >> 10) & 3;
    begin_fast_cw(sw, cw);
    if (__builtin_expect(((tg >> (2u * p)) & 3u) != 3u, 0)) {
        fail("stack overflow (ST(7) not empty); stack faults are not modelled");
    }
    fle_x87.sw = (uint16_t)((sw & ~(SW_C1 | 0x3800u)) | (p << 11));
    fle_x87_r[p] = C[k][rc == 0 ? 0 : rc == 2 ? 2 : 1];
    fle_x87.tw = (uint16_t)((tg & ~(3u << (2u * p))) | ((k == 6u ? 1u : 0u) << (2u * p)));
}

static __attribute__((noinline, cold)) void fst_m_slow(uint64_t a, unsigned size, unsigned popit, fle_f80 v) {
    int up = 0;
    X = 0;
    if (size == 10) {
        commit(0);
        fle_st(a, 8, v.m);
        fle_st(a + 8, 2, v.se);
    } else {
        uint64_t b = to_float(v, size == 8, &up);
        commit(up);
        fle_st(a, size, b);
    }
    if (popit) pop();
}

void fle_x87_fst_m(uint64_t a, unsigned size, unsigned popit) {
    fle_f80 v;
    int fast_inc = 0;
    unsigned fast_pe = 0u;
    uint16_t sw = fle_x87.sw, cw = fle_x87.cw;
    unsigned t = (sw >> 11) & 7u;
    unsigned tg = fle_x87.tw;
    begin_fast_cw(sw, cw);
    if (__builtin_expect(((tg >> (2u * t)) & 3u) == 3u, 0)) {
        fail("stack underflow (empty ST(i)); stack faults are not modelled");
    }
    v = fle_x87_r[t];
    if (__builtin_expect((cw & 0x0C20u) == EX_PE, 1)) {
        if (size == 4) {
            unsigned raw_e = v.se & 0x7fffu;
            uint32_t out32;
            if (__builtin_expect(((raw_e - 16257u) < 253u) & ((v.m >> 63) != 0u), 1)) {
                uint64_t rem = v.m & UINT64_C(0xffffffffff);
                uint64_t keep = v.m >> 40;
                unsigned e_biased = raw_e - (16383u - 127u);
                uint64_t inc = (rem >> 39) & (((rem & UINT64_C(0x7fffffffff)) != 0u) | (keep & 1u));
                keep += inc;
                if (__builtin_expect((keep >> 24) != 0u, 0)) {
                    keep >>= 1;
                    e_biased++;
                }
                out32 = ((uint32_t)(v.se >> 15) << 31) | ((uint32_t)e_biased << 23) | (uint32_t)(keep & 0x7fffffu);
                fast_inc = (int)inc;
                fast_pe = rem ? EX_PE : 0u;
            } else if (raw_e == 0u && v.m == 0u) {
                out32 = (uint32_t)(v.se & 0x8000u) << 16;
            } else {
                fst_m_slow(a, size, popit, v);
                return;
            }
            if (__builtin_expect(((a | (a + 3u)) >> 32) == 0u, 1)) memcpy((void *)(uintptr_t)a, &out32, 4);
            else fle_st(a, 4, out32);
            goto fast_finish;
        } else if (size == 8) {
            unsigned raw_e = v.se & 0x7fffu;
            uint64_t out64;
            if (__builtin_expect(((raw_e - 15361u) < 2045u) & ((v.m >> 63) != 0u), 1)) {
                uint64_t rem = v.m & 0x7ffu;
                uint64_t keep = v.m >> 11;
                unsigned e_biased = raw_e - (16383u - 1023u);
                uint64_t inc = (rem >> 10) & (((rem & 0x3ffu) != 0u) | (keep & 1u));
                keep += inc;
                if (__builtin_expect((keep >> 53) != 0u, 0)) {
                    keep >>= 1;
                    e_biased++;
                }
                out64 = ((uint64_t)(v.se & 0x8000u) << 48) | ((uint64_t)e_biased << 52) | (keep & UINT64_C(0xfffffffffffff));
                fast_inc = (int)inc;
                fast_pe = rem ? EX_PE : 0u;
            } else if (raw_e == 0u && v.m == 0u) {
                out64 = (uint64_t)(v.se & 0x8000u) << 48;
            } else {
                fst_m_slow(a, size, popit, v);
                return;
            }
            if (__builtin_expect(((a | (a + 7u)) >> 32) == 0u, 1)) memcpy((void *)(uintptr_t)a, &out64, 8);
            else fle_st(a, 8, out64);
            goto fast_finish;
        }
    }
    fst_m_slow(a, size, popit, v);
    return;

fast_finish:
    if (popit) {
        fle_x87.tw = (uint16_t)(tg | (3u << (2u * t)));
        fle_x87.sw = (uint16_t)((sw & ~(SW_C1 | 0x3800u)) | (fast_inc ? SW_C1 : 0u) | fast_pe | (((t + 1u) & 7u) << 11));
    } else {
        fle_x87.sw = (uint16_t)((sw & ~SW_C1) | (fast_inc ? SW_C1 : 0u) | fast_pe);
    }
}

void fle_x87_fst_st(unsigned i, unsigned popit) {
    unsigned t = top();
    unsigned tg = fle_x87.tw;
    begin_fast();
    if (__builtin_expect(((tg >> (2u * t)) & 3u) == 3u, 0)) {
        fail("stack underflow (empty ST(i)); stack faults are not modelled");
    }
    if (popit) {
        if (i != 0u) {
            unsigned pd = (t + i) & 7u;
            unsigned st0_tag = (tg >> (2u * t)) & 3u;
            fle_x87_r[pd] = fle_x87_r[t];
            tg = (tg & ~(3u << (2u * pd))) | (st0_tag << (2u * pd));
        }
        fle_x87.tw = (uint16_t)(tg | (3u << (2u * t)));
        fle_x87.sw = (uint16_t)((fle_x87.sw & ~(SW_C1 | 0x3800u)) | (((t + 1u) & 7u) << 11));
    } else {
        fle_x87.sw &= (uint16_t)~SW_C1;
        if (i != 0u) {
            unsigned pd = (t + i) & 7u;
            unsigned st0_tag = (tg >> (2u * t)) & 3u;
            fle_x87_r[pd] = fle_x87_r[t];
            fle_x87.tw = (uint16_t)((tg & ~(3u << (2u * pd))) | (st0_tag << (2u * pd)));
        }
    }
}

/* ---- integer stores ---- */

static unsigned round_int(fp a, int rc, int *up, uint64_t *mag, int *too_big) {
    /* magnitude of a finite nonzero value rounded to an integer; returns inexact */
    uint64_t ip;
    int rb, st, inc;
    *too_big = 0;
    if (a.e >= 63) {
        if (a.e >= 64) {
            *too_big = 1;
            return 0;
        }
        *mag = a.m;
        *up = 0;
        return 0;
    }
    if (a.e < -1) {
        ip = 0;
        rb = 0;
        st = 1;
    } else if (a.e == -1) {
        ip = 0;
        rb = 1;
        st = (a.m << 1) != 0;
    } else {
        unsigned sh = (unsigned)(63 - a.e);
        uint64_t rem = a.m & ((UINT64_C(1) << sh) - 1);
        ip = a.m >> sh;
        rb = (int)((rem >> (sh - 1)) & 1);
        st = (rem & ((UINT64_C(1) << (sh - 1)) - 1)) != 0;
    }
    switch (rc) {
    case 0: inc = rb && (st || (ip & 1)); break;
    case 1: inc = (rb | st) && a.sign; break;
    case 2: inc = (rb | st) && !a.sign; break;
    default: inc = 0; break;
    }
    *up = inc;
    *mag = ip + (uint64_t)inc;
    return (unsigned)(rb | st);
}

void fle_x87_fist(uint64_t a, unsigned size, unsigned popit, unsigned trunc) {
    fp v;
    uint64_t mag = 0;
    int up = 0, too_big = 0, bad = 0;
    unsigned inexact = 0;
    uint64_t out;
    uint64_t indef = (uint64_t)1 << (size * 8 - 1);
    begin();
    need_nonempty(0);
    v = unpack(fle_x87_r[phys(0)]);
    if (v.k == K_UNSUP || v.k == K_QNAN || v.k == K_SNAN || v.k == K_INF) {
        bad = 1;
    } else if (v.k == K_ZERO) {
        mag = 0;
    } else {
        inexact = round_int(v, trunc ? 3 : rc_mode(), &up, &mag, &too_big);
        if (too_big) bad = 1;
        else if (v.sign ? mag > indef : mag >= indef) bad = 1;
    }
    if (bad) {
        X |= EX_IE;
        out = indef;
        up = 0;
    } else {
        if (inexact) X |= EX_PE;
        else up = 0;
        out = v.sign ? (uint64_t)0 - mag : mag;
    }
    commit((unsigned)up);
    fle_st(a, size, out);
    if (popit) pop();
}

/* ---- arithmetic ---- */

/* NaN and unsupported operands of a two-operand instruction. Returns 1 and the result. */
static int nan_operands(fle_f80 ra, fle_f80 rb, fp a, fp b, fle_f80 *out) {
    fp w;
    if (a.k == K_UNSUP || b.k == K_UNSUP) {
        X |= EX_IE;
        *out = INDEF;
        return 1;
    }
    if (!is_nan(a) && !is_nan(b)) return 0;
    if (a.k == K_SNAN || b.k == K_SNAN) X |= EX_IE;
    if (is_nan(a) && is_nan(b)) {
        if (a.k == K_SNAN && b.k == K_QNAN) w = b;
        else if (b.k == K_SNAN && a.k == K_QNAN) w = a;
        else w = (b.m > a.m) ? b : a;
    } else {
        w = is_nan(a) ? a : b;
    }
    (void)ra;
    (void)rb;
    *out = mk(w.sign, 0x7fff, w.m | UINT64_C(0x4000000000000000));
    return 1;
}

static fle_f80 f_inf(int sign) { return mk(sign, 0x7fff, UINT64_C(0x8000000000000000)); }

static inline __attribute__((always_inline)) void commit_known(unsigned t, unsigned pd, unsigned popit, uint64_t rm, uint16_t rse, unsigned tag, int up, unsigned x_flags) {
    fle_x87_r[pd].m = rm;
    fle_x87_r[pd].se = rse;
    {
        uint16_t sw = (uint16_t)((fle_x87.sw & ~SW_C1) | (up ? SW_C1 : 0u) | (x_flags & 0x3Fu));
        if (popit) {
            unsigned tg = (unsigned)fle_x87.tw;
            if (pd == t) {
                tg |= (3u << (2u * t));
            } else {
                tg = (tg & ~(3u << (2u * pd))) | (tag << (2u * pd));
                tg |= (3u << (2u * t));
            }
            fle_x87.tw = (uint16_t)tg;
            fle_x87.sw = (uint16_t)((sw & ~0x3800u) | (((t + 1u) & 7u) << 11));
        } else {
            fle_x87.sw = sw;
            set_tag(pd, tag);
        }
    }
}

static __attribute__((noinline, cold)) void round80_and_commit_slow(int sign, int64_t e, uint64_t hi, uint64_t lo, int sticky, int pbits, int rc, unsigned t, unsigned pd, unsigned popit) {
    unsigned x_flags = 0u;
    int up = 0;
    fle_f80 res = round80_fast(sign, e, hi, lo, sticky, pbits, rc, &up, &x_flags);
    unsigned tag = tag_for(res);
    if (__builtin_expect((x_flags & (EX_IE | EX_ZE)) != 0u, 0)) x_flags &= ~EX_DE;
    commit_known(t, pd, popit, res.m, res.se, tag, up, x_flags);
}

/* Fast path for ADD/SUB/SUBR/MUL when exceptions are masked and both operands are finite normals or zeros; commits directly with zero pointer out-params. */
static inline __attribute__((always_inline)) int binop_fast_commit(unsigned op, uint64_t am, uint16_t ase, uint64_t bm, uint16_t bse, unsigned t, unsigned pd, unsigned popit, uint16_t cw) {
    uint32_t ea = ase & 0x7fffu, eb = bse & 0x7fffu;
    unsigned pc = (cw >> 8) & 3u;
    int pbits = (pc == 3u || pc == 1u) ? 64 : (pc == 2u ? 53 : 24);
    int rc = (cw >> 10) & 3;
    int a_norm, b_norm;
    int rsign;
    int64_t re;
    uint64_t rhi, rlo;
    int rsticky = 0;

    if (__builtin_expect(((cw & EX_PE) == 0u) | (op > FLE_X87_MUL), 0)) {
        return 0;
    }
    a_norm = ((ea - 1u) < 0x7ffeu) && ((am >> 63) != 0u);
    b_norm = ((eb - 1u) < 0x7ffeu) && ((bm >> 63) != 0u);
    if (__builtin_expect(!(a_norm & b_norm), 0)) {
        int a_zero = (ea == 0u && am == 0u);
        int b_zero = (eb == 0u && bm == 0u);
        if (!((a_norm | a_zero) & (b_norm | b_zero))) {
            return 0;
        }
        if (op == FLE_X87_MUL) {
            fle_f80 res = mk((int)((ase ^ bse) >> 15), 0, 0);
            commit_known(t, pd, popit, res.m, res.se, 1u, 0, 0u);
            return 1;
        }
        {
            int sa = (int)(ase >> 15) ^ (op == FLE_X87_SUBR);
            int sb = (int)(bse >> 15) ^ (op == FLE_X87_SUB);
            if (a_zero && b_zero) {
                fle_f80 res = mk(sa == sb ? sa : (rc == 1), 0, 0);
                commit_known(t, pd, popit, res.m, res.se, 1u, 0, 0u);
                return 1;
            }
            {
                int sx = a_zero ? sb : sa;
                uint32_t ex = a_zero ? eb : ea;
                uint64_t xm = a_zero ? bm : am;
                if (__builtin_expect((pbits == 64) || (pbits == 53 && (xm & 0x7ffu) == 0u) || (pbits == 24 && (xm & UINT64_C(0xffffffffff)) == 0u), 1)) {
                    fle_f80 res = mk(sx, ex, xm);
                    commit_known(t, pd, popit, res.m, res.se, 0u, 0, 0u);
                    return 1;
                }
                rsign = sx;
                re = (int64_t)ex - 16383;
                rhi = xm;
                rlo = 0;
                rsticky = 0;
                goto do_round;
            }
        }
    }
    if (op == FLE_X87_MUL) {
        int sign = (int)((ase ^ bse) >> 15);
        uint64_t hi, lo;
        int64_t e = (int64_t)ea + (int64_t)eb - 32765;
        mul64x64_128(am, bm, &hi, &lo);
        if (!(hi >> 63)) {
            hi = (hi << 1) | (lo >> 63);
            lo <<= 1;
            e -= 1;
        }
        rsign = sign;
        re = e;
        rhi = hi;
        rlo = lo;
        rsticky = 0;
        goto do_round;
    }
    {
        int sa = (int)(ase >> 15) ^ (op == FLE_X87_SUBR);
        int sb = (int)(bse >> 15) ^ (op == FLE_X87_SUB);
        int swap = (ea < eb) || (ea == eb && am < bm);
        uint32_t ex = swap ? eb : ea, ey = swap ? ea : eb;
        uint64_t xm = swap ? bm : am, ym = swap ? am : bm;
        int sx = swap ? sb : sa, sy = swap ? sa : sb;
        uint32_t d = ex - ey;
        int64_t e = (int64_t)ex - 16383;
        uint64_t y_hi, y_lo;
        int sticky = 0;
        if (d == 0u) {
            y_hi = ym;
            y_lo = 0;
        } else if (d < 64u) {
            y_hi = ym >> d;
            y_lo = ym << (64u - d);
        } else if (d == 64u) {
            y_hi = 0;
            y_lo = ym;
        } else if (d < 128u) {
            y_hi = 0;
            y_lo = ym >> (d - 64u);
            sticky = (ym << (128u - d)) != 0;
        } else {
            y_hi = 0;
            y_lo = 0;
            sticky = 1;
        }
        if (sx == sy) {
            uint64_t lo = y_lo;
            uint64_t hi = xm + y_hi;
            if (hi < xm) {
                sticky |= (int)(lo & 1u);
                lo = (lo >> 1) | (hi << 63);
                hi = (hi >> 1) | UINT64_C(0x8000000000000000);
                e += 1;
            }
            rsign = sx;
            re = e;
            rhi = hi;
            rlo = lo;
            rsticky = sticky;
            goto do_round;
        } else {
            uint64_t lo = sticky ? ~y_lo : (UINT64_C(0) - y_lo);
            uint64_t borrow = (sticky || y_lo != 0) ? 1u : 0u;
            uint64_t hi = xm - y_hi - borrow;
            if (hi == 0 && lo == 0) {
                fle_f80 res = mk(rc == 1, 0, 0);
                commit_known(t, pd, popit, res.m, res.se, 1u, 0, 0u);
                return 1;
            }
            if (hi) {
                int s = clz64(hi);
                if (s) {
                    hi = (hi << s) | (lo >> (64 - s));
                    lo <<= s;
                    e -= s;
                }
            } else {
                int s = clz64(lo);
                hi = lo << s;
                lo = 0;
                e -= 64 + s;
            }
            rsign = sx;
            re = e;
            rhi = hi;
            rlo = lo;
            rsticky = sticky;
        }
    }

do_round:
    if (__builtin_expect(re < -16382 || re >= 16383 || rc != 0, 0)) {
        if (__builtin_expect((cw & 0x3Fu) != 0x3Fu, 0)) return 0;
        round80_and_commit_slow(rsign, re, rhi, rlo, rsticky, pbits, rc, t, pd, popit);
        return 1;
    }
    if (__builtin_expect(pbits == 64, 1)) {
        uint64_t rb = rlo >> 63;
        uint64_t st = ((rlo << 1) != 0u) | (uint64_t)rsticky;
        uint64_t inc = rb & (st | (rhi & 1u));
        uint64_t m = rhi + inc;
        if (__builtin_expect(m == 0u && inc != 0u, 0)) {
            re++;
            m = UINT64_C(0x8000000000000000);
        }
        commit_known(t, pd, popit, m, (uint16_t)((rsign ? 0x8000u : 0u) | (unsigned)(re + 16383)), 0u, (int)inc, (rb | st) ? EX_PE : 0u);
        return 1;
    } else if (__builtin_expect(pbits == 53, 1)) {
        uint64_t keep = rhi >> 11;
        uint64_t rb = (rhi >> 10) & 1u;
        uint64_t st = (((rhi & 0x3ffu) | rlo) != 0u) | (uint64_t)rsticky;
        uint64_t inc = rb & (st | (keep & 1u));
        keep += inc;
        if (__builtin_expect((keep >> 53) != 0u, 0)) {
            keep >>= 1;
            re++;
        }
        commit_known(t, pd, popit, keep << 11, (uint16_t)((rsign ? 0x8000u : 0u) | (unsigned)(re + 16383)), 0u, (int)inc, (rb | st) ? EX_PE : 0u);
        return 1;
    } else {
        uint64_t keep = rhi >> 40;
        uint64_t rb = (rhi >> 39) & 1u;
        uint64_t st = (((rhi & UINT64_C(0x7fffffffff)) | rlo) != 0u) | (uint64_t)rsticky;
        uint64_t inc = rb & (st | (keep & 1u));
        keep += inc;
        if (__builtin_expect((keep >> 24) != 0u, 0)) {
            keep >>= 1;
            re++;
        }
        commit_known(t, pd, popit, keep << 40, (uint16_t)((rsign ? 0x8000u : 0u) | (unsigned)(re + 16383)), 0u, (int)inc, (rb | st) ? EX_PE : 0u);
        return 1;
    }
}

/* Result of dst op src. a is the destination operand, b the source. Sets *up. */
static fle_f80 binop(unsigned op, fle_f80 ra, fle_f80 rb, int *up) {
    fp a = unpack(ra), b = unpack(rb), t;
    fle_f80 nr;
    int pbits = pc_bits();
    int rc = rc_mode();
    *up = 0;
    if (nan_operands(ra, rb, a, b, &nr)) {
        X &= ~EX_DE; /* a NaN result reports no denormal operand, also one converted at the load */
        return nr;
    }
    if (a.k == K_DEN || b.k == K_DEN) X |= EX_DE;
    if (op == FLE_X87_SUBR || op == FLE_X87_DIVR) {
        t = a;
        a = b;
        b = t;
        op = op == FLE_X87_SUBR ? FLE_X87_SUB : FLE_X87_DIV;
    }
    if (op == FLE_X87_ADD || op == FLE_X87_SUB) {
        int sa = a.sign, sb = b.sign ^ (op == FLE_X87_SUB);
        int eff_sub, sign, sticky = 0;
        int64_t e;
        if (a.k == K_INF || b.k == K_INF) {
            if (a.k == K_INF && b.k == K_INF && sa != sb) {
                X |= EX_IE;
                return INDEF;
            }
            return f_inf(a.k == K_INF ? sa : sb);
        }
        if (a.k == K_ZERO && b.k == K_ZERO) return mk(sa == sb ? sa : (rc == 1), 0, 0);
        if (a.k == K_ZERO || b.k == K_ZERO) {
            fp n = a.k == K_ZERO ? b : a;
            return round80_fast(a.k == K_ZERO ? sb : sa, n.e, n.m, 0, 0, pbits, rc, up, &X);
        }
        /* order by magnitude: x is the larger */
        int swap = a.e < b.e || (a.e == b.e && a.m < b.m);
        fp x = swap ? b : a, y = swap ? a : b;
        int sx = swap ? sb : sa, sy = swap ? sa : sb;
        unsigned d = (unsigned)(x.e - y.e);
        uint64_t y_hi, y_lo;
        eff_sub = sx != sy;
        sign = sx;
        e = x.e;
        if (d >= 128) {
            y_hi = 0;
            y_lo = 0;
            sticky = 1;
        } else if (d > 64) {
            y_hi = 0;
            y_lo = y.m >> (d - 64);
            sticky = (y.m << (128 - d)) != 0;
        } else if (d == 64) {
            y_hi = 0;
            y_lo = y.m;
        } else if (d > 0) {
            y_hi = y.m >> d;
            y_lo = y.m << (64 - d);
        } else {
            y_hi = y.m;
            y_lo = 0;
        }
        if (!eff_sub) {
            uint64_t lo = y_lo;
            uint64_t hi = x.m + y_hi;
            if (hi < x.m) {
                sticky |= (int)(lo & 1u);
                lo = (lo >> 1) | (hi << 63);
                hi = (hi >> 1) | UINT64_C(0x8000000000000000);
                e += 1;
            }
            return round80_fast(sign, e, hi, lo, sticky, pbits, rc, up, &X);
        } else {
            uint64_t lo = sticky ? ~y_lo : (UINT64_C(0) - y_lo);
            uint64_t borrow = (sticky || y_lo != 0) ? 1u : 0u;
            uint64_t hi = x.m - y_hi - borrow;
            if (hi == 0 && lo == 0) return mk(rc == 1, 0, 0);
            if (hi) {
                int s = clz64(hi);
                if (s) {
                    hi = (hi << s) | (lo >> (64 - s));
                    lo <<= s;
                    e -= s;
                }
            } else {
                int s = clz64(lo);
                hi = lo << s;
                lo = 0;
                e -= 64 + s;
            }
            return round80_fast(sign, e, hi, lo, sticky, pbits, rc, up, &X);
        }
    }
    if (op == FLE_X87_MUL) {
        int sign = a.sign ^ b.sign;
        uint64_t hi, lo;
        int64_t e;
        if ((a.k == K_INF && b.k == K_ZERO) || (a.k == K_ZERO && b.k == K_INF)) {
            X |= EX_IE;
            return INDEF;
        }
        if (a.k == K_INF || b.k == K_INF) return f_inf(sign);
        if (a.k == K_ZERO || b.k == K_ZERO) return mk(sign, 0, 0);
        mul64x64_128(a.m, b.m, &hi, &lo);
        e = a.e + b.e + 1;
        if (!(hi >> 63)) {
            hi = (hi << 1) | (lo >> 63);
            lo <<= 1;
            e -= 1;
        }
        return round80_fast(sign, e, hi, lo, 0, pbits, rc, up, &X);
    }
    {
        int sign = a.sign ^ b.sign;
        u128 q, r, Q;
        int bit, s;
        int64_t e;
        if (a.k == K_ZERO && b.k == K_ZERO) {
            X |= EX_IE;
            return INDEF;
        }
        if (a.k == K_INF && b.k == K_INF) {
            X |= EX_IE;
            return INDEF;
        }
        if (a.k == K_INF) return f_inf(sign);
        if (b.k == K_ZERO) {
            X |= EX_ZE;
            return f_inf(sign);
        }
        if (b.k == K_INF || a.k == K_ZERO) return mk(sign, 0, 0);
        q = ((u128)a.m << 64) / b.m;
        r = ((u128)a.m << 64) % b.m;
        r <<= 1;
        bit = r >= b.m;
        if (bit) r -= b.m;
        Q = (q << 1) | (u128)bit;
        s = clz128(Q);
        e = a.e - b.e + (127 - s) - 65;
        return round80(sign, e, Q << s, r != 0, pbits, up);
    }
}

static fle_f80 src_value(unsigned i) {
    need_nonempty(i);
    return fle_x87_r[phys(i)];
}

static inline void finish_op(unsigned pd, fle_f80 r, int up, unsigned popit) {
    unsigned t = top();
    fle_x87_r[pd] = r;
    commit((unsigned)up);
    if (popit) {
        unsigned tg = (unsigned)fle_x87.tw;
        if (pd == t) {
            tg |= (3u << (2u * t));
        } else {
            tg = (tg & ~(3u << (2u * pd))) | (tag_for(r) << (2u * pd));
            tg |= (3u << (2u * t));
        }
        fle_x87.tw = (uint16_t)tg;
        set_top(t + 1u);
    } else {
        set_tag(pd, tag_for(r));
    }
}

static __attribute__((noinline, cold)) void arith_st_slow(unsigned op, fle_f80 a, fle_f80 b, unsigned pd, unsigned popit) {
    int up;
    fle_f80 r;
    X = 0;
    r = binop(op, a, b, &up);
    finish_op(pd, r, up, popit);
}

static __attribute__((noinline, cold)) void arith_m_slow(unsigned op, fle_f80 a, uint64_t addr, unsigned size, unsigned t, int have_b, fle_f80 b) {
    int up;
    fle_f80 r;
    if (!have_b) {
        X = 0;
        keep_snan = 1;
        b = load_mem(addr, size);
        keep_snan = 0;
    } else {
        X = 0;
    }
    r = binop(op, a, b, &up);
    finish_op(t, r, up, 0);
}

void fle_x87_arith_st(unsigned op, unsigned dst, unsigned src, unsigned popit) {
    uint16_t sw = fle_x87.sw, cw = fle_x87.cw;
    unsigned t = (sw >> 11) & 7u, pd = (t + dst) & 7u, ps = (t + src) & 7u;
    unsigned tg = fle_x87.tw;
    fle_f80 a, b;
    begin_fast_cw(sw, cw);
    if (__builtin_expect(((tg >> (2u * pd)) & 3u) == 3u || ((tg >> (2u * ps)) & 3u) == 3u, 0)) {
        fail("stack underflow (empty ST(i)); stack faults are not modelled");
    }
    a = fle_x87_r[pd];
    b = fle_x87_r[ps];
    if (__builtin_expect(binop_fast_commit(op, a.m, a.se, b.m, b.se, t, pd, popit, cw), 1)) {
        return;
    }
    arith_st_slow(op, a, b, pd, popit);
}

void fle_x87_arith_m(unsigned op, uint64_t addr, unsigned size) {
    fle_f80 a, b;
    uint16_t sw = fle_x87.sw, cw = fle_x87.cw;
    unsigned t = (sw >> 11) & 7u;
    begin_fast_cw(sw, cw);
    if (__builtin_expect(((fle_x87.tw >> (2u * t)) & 3u) == 3u, 0)) {
        fail("stack underflow (empty ST(i)); stack faults are not modelled");
    }
    a = fle_x87_r[t];
    if (__builtin_expect(size == 8, 1)) {
        uint64_t bits;
        if (__builtin_expect(((addr | (addr + 7u)) >> 32) == 0u, 1)) memcpy(&bits, (const void *)(uintptr_t)addr, 8);
        else bits = fle_ld(addr, 8);
        unsigned e = (unsigned)((bits >> 52) & 0x7ffu);
        if (__builtin_expect((e - 1u) < 0x7feu, 1)) {
            b.m = UINT64_C(0x8000000000000000) | ((bits & UINT64_C(0xfffffffffffff)) << 11);
            b.se = (uint16_t)(((bits >> 48) & 0x8000u) | (e + (16383u - 1023u)));
            goto have_b;
        }
        if ((bits & UINT64_C(0x7fffffffffffffff)) == 0u) {
            b.m = 0u;
            b.se = (uint16_t)((bits >> 48) & 0x8000u);
            goto have_b;
        }
    } else if (__builtin_expect(size == 4, 1)) {
        uint32_t bits;
        if (__builtin_expect(((addr | (addr + 3u)) >> 32) == 0u, 1)) memcpy(&bits, (const void *)(uintptr_t)addr, 4);
        else bits = (uint32_t)fle_ld(addr, 4);
        unsigned e = (bits >> 23) & 0xffu;
        if (__builtin_expect((e - 1u) < 0xfeu, 1)) {
            b.m = UINT64_C(0x8000000000000000) | ((uint64_t)(bits & 0x7fffffu) << 40);
            b.se = (uint16_t)(((bits >> 16) & 0x8000u) | (e + (16383u - 127u)));
            goto have_b;
        }
        if ((bits & 0x7fffffffu) == 0u) {
            b.m = 0u;
            b.se = (uint16_t)((bits >> 16) & 0x8000u);
            goto have_b;
        }
    }
    arith_m_slow(op, a, addr, size, t, 0, a);
    return;

have_b:
    if (__builtin_expect(binop_fast_commit(op, a.m, a.se, b.m, b.se, t, t, 0u, cw), 1)) return;
    arith_m_slow(op, a, addr, size, t, 1, b);
}

void fle_x87_arith_i(unsigned op, uint64_t addr, unsigned size) {
    fle_f80 a, b, r;
    int up;
    begin();
    a = src_value(0);
    b = from_int(load_int(addr, size));
    r = binop(op, a, b, &up);
    commit((unsigned)up);
    put(0, r);
}

/* ---- comparison ---- */

/* -1, 0, 1 for finite or infinite non-NaN operands. */
static int order(fp a, fp b) {
    int ka, kb;
    if (a.k == K_ZERO && b.k == K_ZERO) return 0;
    if (a.k == K_ZERO) return b.sign ? 1 : -1;
    if (b.k == K_ZERO) return a.sign ? -1 : 1;
    if (a.sign != b.sign) return a.sign ? -1 : 1;
    ka = a.k == K_INF;
    kb = b.k == K_INF;
    {
        int mag;
        if (ka || kb) mag = ka == kb ? 0 : (ka ? 1 : -1);
        else if (a.e != b.e) mag = a.e > b.e ? 1 : -1;
        else mag = a.m == b.m ? 0 : (a.m > b.m ? 1 : -1);
        return a.sign ? -mag : mag;
    }
}

/* Returns the condition bits C3, C2, C0 as status word bits. */
static unsigned compare(fle_f80 ra, fle_f80 rb, int unordered_mode) {
    fp a = unpack(ra), b = unpack(rb);
    int o;
    if (a.k == K_UNSUP || b.k == K_UNSUP || a.k == K_SNAN || b.k == K_SNAN) {
        X |= EX_IE;
        return SW_C3 | SW_C2 | SW_C0;
    }
    if (a.k == K_QNAN || b.k == K_QNAN) {
        if (!unordered_mode) X |= EX_IE;
        return SW_C3 | SW_C2 | SW_C0;
    }
    if (a.k == K_DEN || b.k == K_DEN) X |= EX_DE;
    o = order(a, b);
    return o > 0 ? 0u : o < 0 ? SW_C0 : SW_C3;
}

static void set_cc(unsigned cc) {
    fle_x87.sw = (uint16_t)((fle_x87.sw & ~(SW_C0 | SW_C2 | SW_C3)) | cc);
}

void fle_x87_com_st(unsigned i, unsigned pops, unsigned unordered) {
    unsigned t = top(), pi = (t + i) & 7u;
    unsigned tg = fle_x87.tw;
    fle_f80 ra, rb;
    uint32_t ea, eb;
    int a_ok, b_ok;
    begin_fast();
    if (__builtin_expect(((tg >> (2u * t)) & 3u) == 3u || ((tg >> (2u * pi)) & 3u) == 3u, 0)) {
        fail("stack underflow (empty ST(i)); stack faults are not modelled");
    }
    ra = fle_x87_r[t];
    rb = fle_x87_r[pi];
    ea = ra.se & 0x7fffu;
    eb = rb.se & 0x7fffu;
    a_ok = (((ea - 1u) < 0x7ffeu) && ((ra.m >> 63) != 0u)) || (ea == 0u && ra.m == 0u);
    b_ok = (((eb - 1u) < 0x7ffeu) && ((rb.m >> 63) != 0u)) || (eb == 0u && rb.m == 0u);
    if (__builtin_expect(a_ok & b_ok, 1)) {
        unsigned cc;
        int a_zero = (ea == 0u), b_zero = (eb == 0u);
        if (a_zero && b_zero) {
            cc = SW_C3;
        } else if (a_zero) {
            cc = (rb.se >> 15) ? 0u : SW_C0;
        } else if (b_zero) {
            cc = (ra.se >> 15) ? SW_C0 : 0u;
        } else {
            unsigned sa = ra.se >> 15, sb = rb.se >> 15;
            if (sa != sb) {
                cc = sa ? SW_C0 : 0u;
            } else if (ea != eb) {
                cc = ((ea > eb) ^ sa) ? 0u : SW_C0;
            } else if (ra.m != rb.m) {
                cc = ((ra.m > rb.m) ^ sa) ? 0u : SW_C0;
            } else {
                cc = SW_C3;
            }
        }
        fle_x87.sw = (uint16_t)((fle_x87.sw & ~(SW_C0 | SW_C1 | SW_C2 | SW_C3 | (pops ? 0x3800u : 0u))) | cc | (pops ? (((t + pops) & 7u) << 11) : 0u));
        if (pops) {
            tg |= (3u << (2u * t));
            if (pops >= 2u) tg |= (3u << (2u * ((t + 1u) & 7u)));
            fle_x87.tw = (uint16_t)tg;
        }
        return;
    }
    X = 0;
    {
        unsigned cc = compare(ra, rb, (int)unordered);
        commit(0);
        set_cc(cc);
    }
    if (pops >= 1u) pop();
    if (pops >= 2u) pop();
}

void fle_x87_com_m(uint64_t addr, unsigned size, unsigned popit) {
    unsigned cc;
    fle_f80 b;
    begin();
    need_nonempty(0);
    keep_snan = 1;
    b = load_mem(addr, size);
    keep_snan = 0;
    cc = compare(fle_x87_r[phys(0)], b, 0);
    commit(0);
    set_cc(cc);
    if (popit) pop();
}

void fle_x87_com_i(uint64_t addr, unsigned size, unsigned popit) {
    unsigned cc;
    fle_f80 b;
    begin();
    need_nonempty(0);
    b = from_int(load_int(addr, size));
    cc = compare(fle_x87_r[phys(0)], b, 0);
    commit(0);
    set_cc(cc);
    if (popit) pop();
}

unsigned fle_x87_comi(unsigned i, unsigned popit, unsigned unordered) {
    unsigned cc, r;
    begin();
    cc = compare(src_value(0), src_value(i), (int)unordered);
    commit(0);
    r = cc == (SW_C3 | SW_C2 | SW_C0) ? 7u : cc == SW_C3 ? 4u : cc == SW_C0 ? 1u : 0u;
    if (popit) pop();
    return r;
}

void fle_x87_ftst(void) {
    unsigned cc;
    fle_f80 z = { 0, 0 };
    begin();
    cc = compare(src_value(0), z, 0);
    commit(0);
    set_cc(cc);
}

void fle_x87_fxam(void) {
    unsigned p = phys(0), cc;
    fle_f80 v = fle_x87_r[p];
    fp a = unpack(v);
    begin();
    if (tag_of(p) == 3) cc = SW_C3 | SW_C0;
    else
        switch (a.k) {
        case K_UNSUP: cc = 0; break;
        case K_QNAN:
        case K_SNAN: cc = SW_C0; break;
        case K_NORM: cc = SW_C2; break;
        case K_INF: cc = SW_C2 | SW_C0; break;
        case K_ZERO: cc = SW_C3; break;
        default: cc = SW_C3 | SW_C2; break;
        }
    fle_x87.sw = (uint16_t)((fle_x87.sw & ~(SW_C0 | SW_C1 | SW_C2 | SW_C3)) | cc | (a.sign ? SW_C1 : 0u));
}

/* ---- register moves and sign ---- */

void fle_x87_fxch(unsigned i) {
    unsigned t = top(), pi = (t + i) & 7u;
    unsigned tg = fle_x87.tw;
    begin_fast();
    if (__builtin_expect(((tg >> (2u * t)) & 3u) == 3u || ((tg >> (2u * pi)) & 3u) == 3u, 0)) {
        fail("stack underflow (empty ST(i)); stack faults are not modelled");
    }
    fle_x87.sw &= (uint16_t)~SW_C1;
    if (pi != t) {
        fle_f80 a = fle_x87_r[t], b = fle_x87_r[pi];
        unsigned ta = (tg >> (2u * t)) & 3u, tb = (tg >> (2u * pi)) & 3u;
        fle_x87_r[t] = b;
        fle_x87_r[pi] = a;
        tg = (tg & ~((3u << (2u * t)) | (3u << (2u * pi)))) | (tb << (2u * t)) | (ta << (2u * pi));
        fle_x87.tw = (uint16_t)tg;
    }
}

void fle_x87_fcmov(unsigned i, unsigned cond) {
    fle_f80 b;
    begin();
    need_nonempty(0);
    b = src_value(i);
    commit(0);
    if (cond) put(0, b);
}

void fle_x87_fchs(void) {
    uint16_t sw = fle_x87.sw, cw = fle_x87.cw;
    unsigned t = (sw >> 11) & 7u;
    begin_fast_cw(sw, cw);
    if (__builtin_expect(((fle_x87.tw >> (2u * t)) & 3u) == 3u, 0)) {
        fail("stack underflow (empty ST(i)); stack faults are not modelled");
    }
    fle_x87.sw = (uint16_t)(sw & ~SW_C1);
    fle_x87_r[t].se ^= 0x8000u;
}

void fle_x87_fabs(void) {
    uint16_t sw = fle_x87.sw, cw = fle_x87.cw;
    unsigned t = (sw >> 11) & 7u;
    begin_fast_cw(sw, cw);
    if (__builtin_expect(((fle_x87.tw >> (2u * t)) & 3u) == 3u, 0)) {
        fail("stack underflow (empty ST(i)); stack faults are not modelled");
    }
    fle_x87.sw = (uint16_t)(sw & ~SW_C1);
    fle_x87_r[t].se &= 0x7fffu;
}

/* ---- unary arithmetic ---- */

/* A NaN or unsupported operand of a one-operand instruction. */
static int unary_special(fle_f80 v, fp a, fle_f80 *out) {
    if (a.k == K_UNSUP) {
        X |= EX_IE;
        *out = INDEF;
        return 1;
    }
    if (is_nan(a)) {
        if (a.k == K_SNAN) X |= EX_IE;
        *out = mk(a.sign, 0x7fff, v.m | UINT64_C(0x4000000000000000));
        return 1;
    }
    return 0;
}

void fle_x87_fsqrt(void) {
    fle_f80 v, r;
    fp a;
    int up = 0;
    begin();
    v = src_value(0);
    a = unpack(v);
    if (a.k == K_DEN) X |= EX_DE;
    if (unary_special(v, a, &r)) {
        /* r set */
    } else if (a.k == K_ZERO) {
        r = v;
    } else if (a.sign) {
        X |= EX_IE;
        r = INDEF;
    } else if (a.k == K_INF) {
        r = v;
    } else {
        /* value = N * 2^E, E even, N < 2^65. Digit-by-digit root of N * 4^34. */
        u128 rem = 0, root = 0, N;
        int64_t E;
        int j, p;
        if ((a.e - 63) & 1) {
            N = (u128)a.m << 1;
            E = a.e - 64;
        } else {
            N = a.m;
            E = a.e - 63;
        }
        for (j = 0; j < 67; j++) {
            unsigned pair = j < 33 ? (unsigned)((N >> (64 - 2 * j)) & 3u) : 0u;
            u128 t;
            rem = (rem << 2) | pair;
            t = (root << 2) | 1u;
            if (rem >= t) {
                rem -= t;
                root = (root << 1) | 1u;
            } else {
                root <<= 1;
            }
        }
        p = 127 - clz128(root);
        r = round80(0, (int64_t)p + E / 2 - 34, root << (127 - p), rem != 0, pc_bits(), &up);
    }
    commit((unsigned)up);
    put(0, r);
}

void fle_x87_frndint(void) {
    fle_f80 v, r;
    fp a;
    uint64_t mag = 0;
    int up = 0, big = 0;
    begin();
    v = src_value(0);
    a = unpack(v);
    if (a.k == K_DEN) X |= EX_DE;
    if (unary_special(v, a, &r)) {
        /* r set */
    } else if (a.k == K_ZERO || a.k == K_INF) {
        r = v;
    } else {
        unsigned inexact = round_int(a, rc_mode(), &up, &mag, &big);
        if (big) {
            r = v;
        } else {
            if (inexact) X |= EX_PE;
            else up = 0;
            if (mag == 0) {
                r = mk(a.sign, 0, 0);
            } else {
                int s = clz64(mag);
                r = mk(a.sign, (unsigned)(63 - s + 16383), mag << s);
            }
        }
    }
    commit((unsigned)up);
    put(0, r);
}

void fle_x87_fscale(void) {
    fle_f80 v0, v1, r;
    fp a, b;
    int up = 0;
    begin();
    v0 = src_value(0);
    v1 = src_value(1);
    a = unpack(v0);
    b = unpack(v1);
    if (nan_operands(v0, v1, a, b, &r)) {
        /* r set */
    } else if ((a.k == K_DEN || b.k == K_DEN) && (X |= EX_DE, 0)) {
        /* DE is raised before the special cases */
    } else if (b.k == K_INF) {
        if (b.sign) {
            if (a.k == K_INF && !a.sign) {
                X |= EX_IE;
                r = INDEF;
            } else if (a.k == K_INF) {
                X |= EX_IE;
                r = INDEF;
            } else {
                r = mk(a.sign, 0, 0);
            }
            if (a.k == K_ZERO) r = v0;
        } else if (a.k == K_ZERO) {
            X |= EX_IE;
            r = INDEF;
        } else {
            r = f_inf(a.sign);
        }
    } else if (a.k == K_ZERO || a.k == K_INF) {
        r = v0;
    } else {
        int64_t n = 0;
        if (b.k != K_ZERO && b.e >= 0) {
            n = b.e >= 40 ? ((int64_t)1 << 40) : (int64_t)(b.m >> (63 - b.e));
            if (b.sign) n = -n;
        }
        r = pack80(round_pack(a.sign, a.e + n, (u128)a.m << 64, 0, 64, 64, -16382, 16383, rc_mode(), &up));
    }
    commit((unsigned)up);
    put(0, r);
}

/* ---- F2XM1 and FSINCOS: correctly rounded, no host floating point (decision 0043) ----
 *
 * The SDM gives no algorithm and no exact result for these (Vol. 1 8.3.10: error below 1 ulp
 * for round-to-nearest, below 1.5 ulp otherwise, monotonic), so the result is the one rounded
 * correctly by RC from a 256-bit evaluation. Only integer arithmetic on 32-bit limbs is used
 * (plus u128 in round_pack), so the code builds the same on x86 and on wasm32.
 *
 * bf is a 256-bit significand with its own exponent. A result is rounded from the interval
 * [v - 2^16 units, v + 2^16 units] (a margin far above the error of the series); when the two
 * ends do not round to the same value with the same flags the run stops with a named abort
 * (the true value would have to lie within 2^-239 of a rounding boundary).
 *
 * FSINCOS reduces with the SDM's 66-bit Pi (Vol. 1 8.3.8): x = k * Pi/2 + r with k the nearest
 * integer, |r| <= Pi/2 / 2, computed exactly; sin and cos of r are taken exactly (series) and
 * rotated by k mod 4. */

enum { BL = 8 };

typedef struct {
    int sign;
    int z; /* zero */
    int64_t e; /* exponent of the leading bit: value = m * 2^(e - 255) */
    uint32_t m[BL];
} bf;

typedef struct {
    bf v;
    int kind; /* 0: the value v; 1: just below |v| (v exact, at most 128 bits); 2: just below 1 */
} tv;

typedef struct {
    rr r;
    int up;
    unsigned x;
} rnd1;

static const bf BF_LN2 = { 0, 0, -1, { 0x8baafa2bu, 0x8a0d175bu, 0x7298b62du, 0x40f34326u, 0x03f2f6afu, 0xc9e3b398u, 0xd1cf79abu, 0xb17217f7u } };

static void bf_zero(bf *a) {
    memset(a, 0, sizeof *a);
    a->z = 1;
}

static uint32_t limb_at(const uint32_t *w, int n, int64_t i) { return (i >= 0 && i < n) ? w[i] : 0u; }

/* The 32 bits p .. p+31 of the n-limb integer w (bits outside it are 0). */
static uint32_t bits32(const uint32_t *w, int n, int64_t p) {
    int64_t li = p >> 5;
    unsigned sh = (unsigned)(p & 31);
    uint32_t lo = limb_at(w, n, li);
    if (!sh) return lo;
    uint32_t hi = limb_at(w, n, li + 1);
    return (lo >> sh) | (hi << (32u - sh));
}

/* a = W * 2^s with W of n limbs, normalized to 256 bits (truncated). */
static void bf_norm(bf *a, int sign, const uint32_t *w, int n, int64_t s) {
    int64_t hb = -1, i;
    for (i = n - 1; i >= 0; i--) {
        if (w[i]) {
            hb = 32 * i + 31 - __builtin_clz((unsigned)w[i]);
            break;
        }
    }
    if (hb < 0) {
        bf_zero(a);
        return;
    }
    a->z = 0;
    a->sign = sign;
    a->e = hb + s;
    for (i = 0; i < BL; i++) a->m[i] = bits32(w, n, hb - 255 + 32 * i);
}

static void bf_from_u128(bf *a, int sign, u128 v, int64_t s) {
    uint32_t w[4];
    w[0] = (uint32_t)v;
    w[1] = (uint32_t)(v >> 32);
    w[2] = (uint32_t)(v >> 64);
    w[3] = (uint32_t)(v >> 96);
    bf_norm(a, sign, w, 4, s);
}

static void bf_mul(bf *r, const bf *a, const bf *b) {
    uint32_t P[2 * BL];
    int i, j;
    bf t;
    if (a->z || b->z) {
        bf_zero(r);
        return;
    }
    memset(P, 0, sizeof P);
    for (i = 0; i < BL; i++) {
        uint64_t c = 0;
        for (j = 0; j < BL; j++) {
            uint64_t v = (uint64_t)a->m[i] * b->m[j] + P[i + j] + c;
            P[i + j] = (uint32_t)v;
            c = v >> 32;
        }
        P[i + BL] = (uint32_t)c;
    }
    bf_norm(&t, a->sign ^ b->sign, P, 2 * BL, (a->e - 255) + (b->e - 255));
    *r = t;
}

static int bf_cmp_mag(const bf *a, const bf *b) {
    int i;
    if (a->e != b->e) return a->e > b->e ? 1 : -1;
    for (i = BL - 1; i >= 0; i--) {
        if (a->m[i] != b->m[i]) return a->m[i] > b->m[i] ? 1 : -1;
    }
    return 0;
}

static void bf_add(bf *r, const bf *pa, const bf *pb) {
    const bf *x = pa, *y = pb;
    uint32_t A[BL + 1], B[BL + 1], S[BL + 2];
    int64_t d;
    int i, sub;
    uint64_t c = 0;
    bf t;
    if (pa->z) {
        *r = *pb;
        return;
    }
    if (pb->z) {
        *r = *pa;
        return;
    }
    if (bf_cmp_mag(x, y) < 0) {
        x = pb;
        y = pa;
    }
    d = x->e - y->e;
    sub = x->sign != y->sign;
    for (i = 0; i <= BL; i++) {
        A[i] = i >= 1 ? x->m[i - 1] : 0u;
        B[i] = bits32(y->m, BL, (int64_t)32 * i + d - 32);
    }
    if (!sub) {
        for (i = 0; i <= BL; i++) {
            uint64_t v = (uint64_t)A[i] + B[i] + c;
            S[i] = (uint32_t)v;
            c = v >> 32;
        }
        S[BL + 1] = (uint32_t)c;
    } else {
        uint64_t bw = 0;
        for (i = 0; i <= BL; i++) {
            uint64_t v = (uint64_t)A[i] - B[i] - bw;
            S[i] = (uint32_t)v;
            bw = (v >> 63) & 1u;
        }
        S[BL + 1] = 0;
    }
    bf_norm(&t, x->sign, S, BL + 2, x->e - 255 - 32);
    *r = t;
}

/* r = a / d for 2 <= d < 2^32 (a 288-bit quotient, then truncated to 256 bits). */
static void bf_div_small(bf *r, const bf *a, uint32_t d) {
    uint32_t q[BL + 1];
    uint64_t rem = 0, cur;
    int i;
    bf t;
    if (a->z) {
        *r = *a;
        return;
    }
    for (i = BL - 1; i >= 0; i--) {
        cur = (rem << 32) | a->m[i];
        q[i + 1] = (uint32_t)(cur / d);
        rem = cur % d;
    }
    q[0] = (uint32_t)((rem << 32) / d);
    bf_norm(&t, a->sign, q, BL + 1, a->e - 255 - 32);
    *r = t;
}

/* exp(y) - 1 for |y| < 1: sum of y^n / n! until the term is below 2^-270 of the sum. */
static void bf_expm1(bf *out, const bf *y) {
    bf t = *y, s = *y, u;
    uint32_t n;
    for (n = 2; n < 300; n++) {
        bf_mul(&u, &t, y);
        bf_div_small(&t, &u, n);
        if (t.z) break;
        bf_add(&s, &s, &t);
        if (t.e < s.e - 270) break;
    }
    if (n >= 300) fail("F2XM1: series did not converge");
    *out = s;
}

/* sin(r) and cos(r) for |r| < 1. */
static void bf_sincos(bf *sn, bf *cs, const bf *r) {
    bf r2, t, s, u;
    uint32_t n;
    bf_mul(&r2, r, r);
    t = *r;
    s = *r;
    for (n = 1; n < 200; n++) {
        bf_mul(&u, &t, &r2);
        bf_div_small(&t, &u, (2u * n) * (2u * n + 1u));
        if (t.z) break;
        t.sign ^= 1;
        bf_add(&s, &s, &t);
        if (t.e < s.e - 270) break;
    }
    if (n >= 200) fail("FSINCOS: sine series did not converge");
    *sn = s;
    bf_from_u128(&t, 0, 1, 0);
    s = t;
    for (n = 1; n < 200; n++) {
        bf_mul(&u, &t, &r2);
        bf_div_small(&t, &u, (2u * n - 1u) * (2u * n));
        if (t.z) break;
        t.sign ^= 1;
        bf_add(&s, &s, &t);
        if (t.e < s.e - 270) break;
    }
    if (n >= 200) fail("FSINCOS: cosine series did not converge");
    *cs = s;
}

static u128 top128(const bf *a) {
    return ((u128)a->m[7] << 96) | ((u128)a->m[6] << 64) | ((u128)a->m[5] << 32) | (u128)a->m[4];
}

static int low_nonzero(const bf *a) { return (a->m[0] | a->m[1] | a->m[2] | a->m[3]) != 0; }

/* a +/- 2^16 units of the significand, as an exact normalized value. */
static void bf_bump(bf *r, const bf *a, int up) {
    uint32_t w[BL + 1];
    int i;
    for (i = 0; i < BL; i++) w[i] = a->m[i];
    w[BL] = 0;
    if (up) {
        uint64_t v = (uint64_t)w[0] + 0x10000u;
        w[0] = (uint32_t)v;
        for (i = 1; i <= BL && (v >> 32); i++) {
            v = (uint64_t)w[i] + 1u;
            w[i] = (uint32_t)v;
        }
    } else {
        uint64_t v = (uint64_t)w[0] - 0x10000u;
        w[0] = (uint32_t)v;
        for (i = 1; i < BL && (v >> 63); i++) {
            v = (uint64_t)w[i] - 1u;
            w[i] = (uint32_t)v;
        }
    }
    bf_norm(r, a->sign, w, BL + 1, a->e - 255);
}

/* One rounding to 64 bits by RC, on its own exception bits. */
static rnd1 rnd_window(int sign, int64_t e, u128 M, int sticky) {
    rnd1 o;
    unsigned sx = X;
    X = 0;
    o.r = round_pack(sign, e, M, sticky, 64, 64, -16382, 16383, rc_mode(), &o.up);
    o.x = X;
    X = sx;
    return o;
}

static int same_round(const rnd1 *a, const rnd1 *b) {
    return a->r.kind == b->r.kind && a->r.sign == b->r.sign && a->r.e == b->r.e && a->r.m == b->r.m && a->up == b->up && a->x == b->x;
}

/* Rounds the value to f80 by RC. Adds PE, UE (and OE) to X, sets *up. */
static fle_f80 tv_round(const tv *t, int *up, const char *who) {
    rnd1 a;
    if (t->kind) {
        u128 M;
        int64_t e;
        if (t->kind == 2) {
            M = ~(u128)0;
            e = -1;
        } else {
            M = top128(&t->v);
            e = t->v.e;
            if (M == ((u128)1 << 127)) {
                M = ~(u128)0;
                e -= 1;
            } else {
                M -= 1;
            }
        }
        a = rnd_window(t->v.sign, e, M, 1);
    } else {
        bf lo, hi;
        rnd1 b;
        bf_bump(&lo, &t->v, 0);
        bf_bump(&hi, &t->v, 1);
        a = rnd_window(t->v.sign, lo.e, top128(&lo), low_nonzero(&lo));
        b = rnd_window(t->v.sign, hi.e, top128(&hi), low_nonzero(&hi));
        if (!same_round(&a, &b)) fail(who);
    }
    X |= a.x;
    *up = a.up;
    return pack80(a.r);
}

/* Fast 128-bit interval arithmetic for F2XM1 / FSINCOS with automatic fallback to 256-bit bf_*. */
typedef struct {
    int sign;
    int z;
    int64_t e;
    u128 m;
} bf2;

static const bf2 BF2_LN2 = { 0, 0, -1, (((u128)0xb17217f7d1cf79abULL) << 64) | (u128)0xc9e3b39803f2f6afULL };

static inline void bf2_norm(bf2 *a, int sign, u128 m, int64_t s) {
    if (!m) {
        a->z = 1;
        a->sign = 0;
        a->e = 0;
        a->m = 0;
        return;
    }
    int l = clz128(m);
    a->z = 0;
    a->sign = sign;
    a->m = m << l;
    a->e = (127 - l) + s;
}

static inline void bf2_mul(bf2 *r, const bf2 *a, const bf2 *b) {
    if (a->z || b->z) {
        r->z = 1;
        r->sign = 0;
        r->e = 0;
        r->m = 0;
        return;
    }
    uint64_t ah = (uint64_t)(a->m >> 64), al = (uint64_t)a->m;
    uint64_t bh = (uint64_t)(b->m >> 64), bl = (uint64_t)b->m;
    u128 hh = (u128)ah * bh;
    u128 hl = (u128)ah * bl;
    u128 lh = (u128)al * bh;
    u128 ll = (u128)al * bl;
    u128 mid = (ll >> 64) + (uint64_t)hl + (uint64_t)lh;
    u128 phi = hh + (hl >> 64) + (lh >> 64) + (mid >> 64);
    int64_t e = a->e + b->e + 1;
    if (!(phi >> 127)) {
        phi = (phi << 1) | ((mid >> 63) & 1u);
        e--;
    }
    r->z = 0;
    r->sign = a->sign ^ b->sign;
    r->e = e;
    r->m = phi;
}

static void bf2_add(bf2 *r, const bf2 *pa, const bf2 *pb) {
    if (pa->z) { *r = *pb; return; }
    if (pb->z) { *r = *pa; return; }
    const bf2 *x = pa, *y = pb;
    if (x->e < y->e || (x->e == y->e && x->m < y->m)) {
        x = pb;
        y = pa;
    }
    int64_t d = x->e - y->e;
    if (d >= 128) {
        *r = *x;
        return;
    }
    u128 yb = d ? (y->m >> d) : y->m;
    if (x->sign == y->sign) {
        u128 S = x->m + yb;
        int64_t e = x->e;
        if (S < x->m) {
            S = (S >> 1) | ((u128)1 << 127);
            e++;
        }
        r->z = 0;
        r->sign = x->sign;
        r->e = e;
        r->m = S;
    } else {
        bf2_norm(r, x->sign, x->m - yb, x->e - 127);
    }
}

static void bf2_div_small(bf2 *r, const bf2 *a, uint32_t d) {
    if (a->z) { *r = *a; return; }
    uint32_t m0 = (uint32_t)a->m, m1 = (uint32_t)(a->m >> 32), m2 = (uint32_t)(a->m >> 64), m3 = (uint32_t)(a->m >> 96);
    uint64_t rem = 0, cur;
    uint32_t q4, q3, q2, q1, q0;
    cur = m3;               q4 = (uint32_t)(cur / d); rem = cur % d;
    cur = (rem << 32) | m2; q3 = (uint32_t)(cur / d); rem = cur % d;
    cur = (rem << 32) | m1; q2 = (uint32_t)(cur / d); rem = cur % d;
    cur = (rem << 32) | m0; q1 = (uint32_t)(cur / d); rem = cur % d;
    cur = (rem << 32);      q0 = (uint32_t)(cur / d);
    if (__builtin_expect(q4 != 0, 1)) {
        u128 top = ((u128)q4 << 96) | ((u128)q3 << 64) | ((u128)q2 << 32) | (u128)q1;
        int l = __builtin_clz(q4);
        r->z = 0;
        r->sign = a->sign;
        r->e = a->e - l;
        r->m = l ? ((top << l) | (u128)(q0 >> (32 - l))) : top;
    } else {
        u128 full = ((u128)q3 << 96) | ((u128)q2 << 64) | ((u128)q1 << 32) | (u128)q0;
        bf2_norm(r, a->sign, full, a->e - 159);
    }
}

static int bf2_try_round(const bf2 *v, int *up, fle_f80 *out) {
    if (v->z || v->m <= (u128)0x200000ULL) return 0;
    bf2 lo, hi;
    const u128 eps = (u128)0x100000ULL;
    u128 lm = v->m - eps;
    int ls = clz128(lm);
    lo.e = v->e - ls;
    lo.m = lm << ls;
    u128 hm = v->m + eps;
    if (hm < v->m) {
        hi.e = v->e + 1;
        hi.m = (hm >> 1) | ((u128)1 << 127);
    } else {
        hi.e = v->e;
        hi.m = hm;
    }
    rnd1 a = rnd_window(v->sign, lo.e, lo.m, 1);
    rnd1 b = rnd_window(v->sign, hi.e, hi.m, 1);
    if (!same_round(&a, &b)) return 0;
    X |= a.x;
    *up = a.up;
    *out = pack80(a.r);
    return 1;
}

static int bf2_expm1_round(int sign, uint64_t m, int64_t e, int *up, fle_f80 *out) {
    bf2 y, t, s, u;
    uint32_t n;
    bf2_norm(&y, sign, (u128)m, e - 63);
    bf2_mul(&y, &y, &BF2_LN2);
    t = y;
    s = y;
    for (n = 2; n < 80; n++) {
        bf2_mul(&u, &t, &y);
        bf2_div_small(&t, &u, n);
        if (t.z) break;
        bf2_add(&s, &s, &t);
        if (t.e < s.e - 115) break;
    }
    if (n >= 80) return 0;
    return bf2_try_round(&s, up, out);
}

static int bf2_sincos_round(int sign_a, int quad, int neg_r, u128 Rrem, int *up, fle_f80 *rs, fle_f80 *rc) {
    bf2 r, r2, t, s, c, u;
    bf2_norm(&r, neg_r, Rrem, -65);
    if (r.z) return 0;
    if (r.e < -70) return 0;
    bf2_mul(&r2, &r, &r);
    t = r;
    s = r;
    for (uint32_t n = 1; n < 40; n++) {
        bf2_mul(&u, &t, &r2);
        bf2_div_small(&t, &u, (2u * n) * (2u * n + 1u));
        if (t.z) break;
        t.sign ^= 1;
        bf2_add(&s, &s, &t);
        if (t.e < s.e - 115) break;
    }
    bf2_norm(&t, 0, (u128)1, 0);
    c = t;
    for (uint32_t n = 1; n < 40; n++) {
        bf2_mul(&u, &t, &r2);
        bf2_div_small(&t, &u, (2u * n - 1u) * (2u * n));
        if (t.z) break;
        t.sign ^= 1;
        bf2_add(&c, &c, &t);
        if (t.e < c.e - 115) break;
    }
    bf2 sn, cs;
    switch (quad) {
    case 0: sn = s; cs = c; break;
    case 1: sn = c; cs = s; cs.sign ^= 1; break;
    case 2: sn = s; cs = c; sn.sign ^= 1; cs.sign ^= 1; break;
    default: sn = c; cs = s; sn.sign ^= 1; break;
    }
    if (sign_a) sn.sign ^= 1;
    int upc = 0;
    unsigned sx = X;
    if (!bf2_try_round(&sn, up, rs)) { X = sx; return 0; }
    if (!bf2_try_round(&cs, &upc, rc)) { X = sx; return 0; }
    return 1;
}

void fle_x87_f2xm1(void) {
    fle_f80 v, r;
    fp a;
    int up = 0;
    begin();
    need_nonempty(0);
    v = fle_x87_r[phys(0)];
    a = unpack(v);
    if (a.k == K_DEN) X |= EX_DE;
    if (unary_special(v, a, &r)) {
        /* r set */
    } else if (a.k == K_ZERO) {
        r = v;
    } else if (a.k == K_INF || a.e >= 1 || (a.e == 0 && a.m != UINT64_C(0x8000000000000000))) {
        fail("F2XM1: operand outside -1.0 .. +1.0 (the SDM leaves the result undefined); not modelled");
    } else if (a.e == 0) {
        r = a.sign ? mk(1, 0x3ffe, UINT64_C(0x8000000000000000)) : mk(0, 0x3fff, UINT64_C(0x8000000000000000));
    } else if (!bf2_expm1_round(a.sign, a.m, a.e, &up, &r)) {
        bf y, s;
        tv t;
        bf_from_u128(&y, a.sign, a.m, a.e - 63);
        bf_mul(&y, &y, &BF_LN2);
        bf_expm1(&s, &y);
        t.v = s;
        t.kind = 0;
        r = tv_round(&t, &up, "F2XM1: rounding is not decided at 256 bits");
    }
    commit((unsigned)up);
    put(0, r);
}

/* sin and cos of a finite nonzero x with |x| < 2^63, as f80 rounded by RC. *up: the sine was rounded up. */
static void sincos_round(fp a, fle_f80 *rs, fle_f80 *rc, int *up) {
    /* Pi/2 of the SDM (66 significant bits 0x3243F6A8885A308D3 * 2^-64, halved): D * 2^-65 */
    const u128 D = ((u128)3 << 64) | UINT64_C(0x243f6a8885a308d3);
    bf r, s, c;
    tv ts, tc, sn, cs;
    int quad = 0, upc = 0;
    if (a.e < -2) {
        /* Note: when a.e is negative (e.g. < -2), compute with quad=0 directly. */
        if (a.e >= -70) {
            bf2 r2, t, sb, cb, u;
            bf2_norm(&r2, 0, (u128)a.m, a.e - 63);
            bf2_mul(&r2, &r2, &r2);
            bf2_norm(&sb, 0, (u128)a.m, a.e - 63);
            t = sb;
            for (uint32_t n = 1; n < 40; n++) {
                bf2_mul(&u, &t, &r2);
                bf2_div_small(&t, &u, (2u * n) * (2u * n + 1u));
                if (t.z) break;
                t.sign ^= 1;
                bf2_add(&sb, &sb, &t);
                if (t.e < sb.e - 115) break;
            }
            bf2_norm(&t, 0, (u128)1, 0);
            cb = t;
            for (uint32_t n = 1; n < 40; n++) {
                bf2_mul(&u, &t, &r2);
                bf2_div_small(&t, &u, (2u * n - 1u) * (2u * n));
                if (t.z) break;
                t.sign ^= 1;
                bf2_add(&cb, &cb, &t);
                if (t.e < cb.e - 115) break;
            }
            sb.sign ^= a.sign;
            unsigned sx = X;
            if (bf2_try_round(&sb, up, rs) && bf2_try_round(&cb, &upc, rc)) return;
            X = sx;
        }
        bf_from_u128(&r, 0, a.m, a.e - 63);
    } else {
        u128 A = (u128)a.m << (a.e + 2);
        u128 q = A / D, rem = A % D;
        u128 Rrem = rem;
        int neg = 0;
        if (rem * 2 > D) {
            q += 1;
            Rrem = D - rem;
            neg = 1;
        }
        quad = (int)(q & 3);
        if (bf2_sincos_round(a.sign, quad, neg, Rrem, up, rs, rc)) return;
        bf_from_u128(&r, neg, Rrem, -65);
    }
    if (r.z) fail("FSINCOS: reduced argument is zero");
    bf_sincos(&s, &c, &r);
    ts.v = s;
    ts.kind = 0;
    tc.v = c;
    tc.kind = 0;
    if (r.e < -70) {
        /* sin r = r (1 - d), cos r = 1 - d' with d, d' < 2^-141: the value lies just below |r| and just below 1 */
        ts.v = r;
        ts.kind = 1;
        bf_from_u128(&tc.v, 0, 1, 0);
        tc.kind = 2;
    }
    switch (quad) {
    case 0:
        sn = ts;
        cs = tc;
        break;
    case 1:
        sn = tc;
        cs = ts;
        cs.v.sign ^= 1;
        break;
    case 2:
        sn = ts;
        cs = tc;
        sn.v.sign ^= 1;
        cs.v.sign ^= 1;
        break;
    default:
        sn = tc;
        cs = ts;
        sn.v.sign ^= 1;
        break;
    }
    if (a.sign) sn.v.sign ^= 1;
    *rs = tv_round(&sn, up, "FSINCOS: sine rounding is not decided at 256 bits");
    *rc = tv_round(&cs, &upc, "FSINCOS: cosine rounding is not decided at 256 bits");
}

void fle_x87_fsincos(void) {
    fle_f80 v, rs, rc;
    fp a;
    int up = 0;
    begin();
    need_nonempty(0);
    need_room();
    v = fle_x87_r[phys(0)];
    a = unpack(v);
    if (a.k == K_DEN) X |= EX_DE;
    if (unary_special(v, a, &rs)) {
        rc = rs;
    } else if (a.k == K_INF) {
        X |= EX_IE;
        rs = INDEF;
        rc = INDEF;
    } else if (a.k == K_ZERO) {
        rs = v;
        rc = mk(0, 0x3fff, UINT64_C(0x8000000000000000));
    } else if (a.e >= 63) {
        /* |x| >= 2^63: C2 = 1, ST(0) unchanged, nothing pushed */
        commit(0);
        fle_x87.sw = (uint16_t)(fle_x87.sw | SW_C2);
        return;
    } else {
        sincos_round(a, &rs, &rc, &up);
    }
    commit((unsigned)up);
    fle_x87.sw = (uint16_t)(fle_x87.sw & ~SW_C2);
    put(0, rs);
    push(rc);
}

/* Differential test of the MXCSR fast path (decision 0143).
 *
 * Four implementations of every SSE floating-point instruction the runtime models are linked into one program:
 *   new     runtime/fle_mxcsr_flags.h with the fast path           (impl_new.c, -DFLE_MX_STATS)
 *   nofast  the same header with -DFLE_MX_NOFAST                    (impl_nofast.c)
 *   old     the header of HEAD ac5a790 before the change, verbatim  (impl_old.c, ref/fle_mxcsr_flags_old.h)  = the reference
 *   host    the instruction itself on the host CPU (x86-64 only)    (impl_host.c)
 * Each case gives the same inputs (destination XMM, source XMM, a general register, the guest MXCSR with the
 * exception masks set, and the sticky flags it holds) to the implementations and compares the destination XMM, the
 * general register, the whole MXCSR after the instruction, and for COMIS* the flags ZF PF CF OF SF AF.
 * new is compared with old (the criterion), nofast with old (the refactoring did not change the integer path), and
 * new with host when host=1. For new the statistics of the fast path are read before and after: a case is "fast" when
 * every lane took the fast path. A case whose control bits are not all in the fast path's guard (FLE_MX_FAST_CTL)
 * must have taken it in no lane (guard violations are counted and must be 0): MXFP_EXPECT_CTL, 0x6000 = RC by default.
 *
 * usage: mxfp_diff rand   OPFILTER CASES SEED HOST      random and structured operands
 *        mxfp_diff grid32 OPFILTER                     the grid of binary32 exponent and mantissa classes
 *        mxfp_diff replay FILE                          operands recorded from the engine (FLE_MX_TRACE)
 *        mxfp_diff unmasked CASES SEED                  an unmasked flag aborts the same way
 *        mxfp_diff list
 * OPFILTER is a name, a prefix ending in '*' or '*'. Exit status 0 only if nothing mismatched. */
#include <setjmp.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_runtime.h"
#include "mxfp_common.h"

/* the control bits that must keep a lane off the fast path: independent of the header's own macro (so that a change of
 * the header's guard is seen); -DMXFP_EXPECT_CTL=0xE040u for a build of the stricter guard */
#ifndef MXFP_EXPECT_CTL
#define MXFP_EXPECT_CTL 0x6000u
#endif

uint64_t R[16];
fle_eflags FL;
xmm_t X[16];
uint32_t fle_mxcsr = 0x1F80;
uint64_t fle_mx_stat[64][2];
uint64_t fle_mx_vstat[3][64];
uint64_t fle_mx_rcstat[3][64];

/* the host implementation reads fle_mxcsr too; the impl files share this one variable */

/* ------------------------------------------------------------------------------------------------------------- */
/* random numbers */

static uint64_t rs = 0x9e3779b97f4a7c15ull;

static uint64_t rnd(void) {
    rs ^= rs >> 12;
    rs ^= rs << 25;
    rs ^= rs >> 27;
    return rs * 0x2545f4914f6cdd1dull;
}

static unsigned rb(unsigned n) {
    return (unsigned)((rnd() >> 11) % n);
}

static void seed_for(const char *name, uint64_t seed) {
    rs = 0x9e3779b97f4a7c15ull ^ (seed * 0xd1342543de82ef95ull);
    while (*name) {
        rs = (rs ^ (unsigned char)*name++) * 0x100000001b3ull;
    }
    if (rs == 0) rs = 1;
    for (int i = 0; i < 8; i++) rnd();
}

/* ------------------------------------------------------------------------------------------------------------- */
/* operand generators. f64 selects binary64 (else binary32 in the low 32 bits). */

static uint64_t gm(int mb) { /* a mantissa field */
    uint64_t full = (UINT64_C(1) << mb) - 1, q = UINT64_C(1) << (mb - 1);
    unsigned k;
    switch (rb(16)) {
    case 0: return 0;
    case 1: return full;
    case 2: return 1;
    case 3: return q;
    case 4: return q - 1;
    case 5: return q | 1;
    case 6: return rnd() & full & (rnd() | rnd());
    case 7: k = rb(mb); return ((rnd() & full) >> k << k | (UINT64_C(1) << k >> 1)) & full; /* a tie pattern */
    case 8: return (rnd() & full) >> rb(mb);
    case 9: k = rb(mb + 1); return (rnd() & full) >> k << k;                                 /* few significant bits */
    case 10: k = rb(mb + 1); return ((rnd() & full) >> k << k) ^ (UINT64_C(1) << rb(mb));
    case 11: return full ^ (UINT64_C(1) << rb(mb));
    case 12: k = 1 + rb(26); return ((rnd() & ((UINT64_C(1) << (k < (unsigned)mb ? k : (unsigned)mb)) - 1)) << (mb - (k < (unsigned)mb ? k : (unsigned)mb))) & full;
    default: return rnd() & full;
    }
}

static const uint32_t EDGE64[] = {0, 1, 2, 3, 4, 512, 513, 514, 1532, 1533, 1534, 622, 623, 624, 625, 896, 897, 898, 1021, 1022, 1023, 1024, 1025, 1148, 1149, 1150, 1151,
                                  1420, 1421, 1422, 1423, 1424, 2040, 2042, 2043, 2044, 2045, 2046, 2047};
static const uint32_t EDGE32[] = {0, 1, 2, 3, 4, 5, 64, 65, 66, 188, 189, 190, 60, 61, 62, 63, 64, 126, 127, 128, 129, 190, 191, 192, 250, 251, 252, 253, 254, 255};

static uint64_t mk(int f64, uint64_t sign, uint64_t ex, uint64_t mant) {
    int mb = f64 ? 52 : 23, eb = f64 ? 11 : 8;
    return (sign << (mb + eb)) | (ex << mb) | mant;
}

/* a value inside or next to what the fast path accepts */
static uint64_t gen_friendly(int f64) {
    int mb = f64 ? 52 : 23, bias = f64 ? 1023 : 127;
    uint64_t emax = f64 ? 2047 : 255, ex;
    switch (rb(12)) {
    case 0: ex = 1 + rnd() % (emax - 1); break;
    case 1: ex = (uint64_t)bias - 20 + rb(41); break;
    case 2: ex = (uint64_t)bias - 4 + rb(9); break;
    case 10: ex = (uint64_t)(bias + (rb(2) ? 1 : -1) * (f64 ? 510 : 62)) - 2 + rb(5); break; /* the edges of the windows of decision 0154 */
    case 11: ex = (uint64_t)(bias + (rb(2) ? 1 : -1) * (f64 ? 510 : 62)) - 40 + rb(81); break;
    default: ex = (uint64_t)bias - (f64 ? 60 : 40) + rb(f64 ? 121 : 81); break;
    }
    return mk(f64, rnd() & 1, ex, gm(mb));
}

/* a value with at most k significant bits and a modest exponent: sums and products of such values are often exact */
static uint64_t gen_few(int f64) {
    int mb = f64 ? 52 : 23, bias = f64 ? 1023 : 127, k = 1 + (int)rb(f64 ? 26 : 12);
    uint64_t m = (rnd() & ((UINT64_C(1) << k) - 1)) << (mb - k);
    if (k > mb) m = rnd() & ((UINT64_C(1) << mb) - 1);
    return mk(f64, rnd() & 1, (uint64_t)bias - 12 + rb(25), m);
}

static uint64_t gen_special(int f64) {
    int mb = f64 ? 52 : 23, eb = f64 ? 11 : 8;
    uint64_t emax = (UINT64_C(1) << eb) - 1, sign = rnd() & 1, full = (UINT64_C(1) << mb) - 1;
    switch (rb(10)) {
    case 0: return mk(f64, sign, 0, 0);
    case 1: return mk(f64, sign, 0, 1 + (rnd() & full) % full);
    case 2: return mk(f64, sign, 0, full);
    case 3: return mk(f64, sign, emax, 0);
    case 4: return mk(f64, sign, emax, UINT64_C(1) << (mb - 1)); /* default QNaN payload 0 */
    case 5: return mk(f64, sign, emax, 1 + (rnd() & (full >> 1)) % (full >> 1)); /* SNaN */
    case 6: return mk(f64, sign, emax, (UINT64_C(1) << (mb - 1)) | (rnd() & full)); /* QNaN with payload */
    case 7: return mk(f64, sign, 1, 0);
    case 8: return mk(f64, sign, 1, full);
    default: return mk(f64, sign, 0, 1);
    }
}

static uint64_t gen_edge(int f64) {
    int mb = f64 ? 52 : 23;
    uint32_t e = f64 ? EDGE64[rb(sizeof EDGE64 / sizeof EDGE64[0])] : EDGE32[rb(sizeof EDGE32 / sizeof EDGE32[0])];
    return mk(f64, rnd() & 1, e, gm(mb));
}

/* a single operand of any kind */
static uint64_t gen_any(int f64) {
    switch (rb(20)) {
    case 0: case 1: case 2: case 3: case 4: case 5: case 6: case 7: case 8: case 9: case 10: case 11: return gen_friendly(f64);
    case 12: case 13: return gen_few(f64);
    case 14: case 15: return gen_edge(f64);
    case 16: case 17: return gen_special(f64);
    default: { uint64_t b = rnd(); return f64 ? b : (b & 0xffffffffull); }
    }
}

/* second operand related to the first (a few ulps away, the negative, alignment distance, boundary exponents) */
static uint64_t gen_rel(int f64, uint64_t a) {
    int mb = f64 ? 52 : 23, eb = f64 ? 11 : 8, bias = f64 ? 1023 : 127;
    uint64_t emax = (UINT64_C(1) << eb) - 1, full = f64 ? UINT64_MAX : 0xffffffffull;
    int64_t ea = (int64_t)((a >> mb) & emax), t;
    uint64_t sign = (uint64_t)(rnd() & 1);
    switch (rb(16)) {
    case 0: return (a + rb(5) - 2) & full;
    case 1: return (a ^ (UINT64_C(1) << (mb + eb))) & full;
    case 2: return ((a ^ (UINT64_C(1) << (mb + eb))) + rb(5) - 2) & full;
    case 3: t = ea - (int64_t)rb(60); break;
    case 4: t = ea + (int64_t)rb(60); break;
    case 5: t = 1 + bias - ea + (int64_t)rb(5) - 2; break;                       /* product at the underflow boundary */
    case 6: t = (int64_t)emax - 1 + bias - ea + (int64_t)rb(5) - 2; break;       /* product at the overflow boundary */
    case 7: t = ea + bias - 1 + (int64_t)rb(5) - 2; break;                       /* quotient at the underflow boundary */
    case 8: t = ea + bias - ((int64_t)emax - 1) + (int64_t)rb(5) - 2; break;     /* quotient at the overflow boundary */
    case 9: t = ea + (int64_t)rb(5) - 2; break;
    case 10: t = ea + (int64_t)(f64 ? 52 : 23) + (int64_t)rb(5) - 2; break;      /* sum at the edge of absorption */
    case 11: t = ea - (int64_t)(f64 ? 53 : 24) - (int64_t)rb(3) + 1; break;
    case 12: t = bias + (int64_t)(f64 ? 400 : 100) - ea + (int64_t)rb(5) - 2; break; /* near the 2^400 window */
    case 13: t = bias - (int64_t)(f64 ? 400 : 100) - ea + (int64_t)rb(5) - 2; break;
    default: return gen_any(f64);
    }
    if (t < 0) t = 0;
    if (t > (int64_t)emax) t = (int64_t)emax;
    return mk(f64, sign, (uint64_t)t, gm(mb)) & full;
}

static uint32_t geni32(void) {
    switch (rb(10)) {
    case 0: return (uint32_t)rnd();
    case 1: return rb(40) - 20;
    case 2: return (uint32_t)((int32_t)(rb(2) ? 1 : -1) * (int32_t)((1u << 24) + rb(9) - 4));
    case 3: return rb(2) ? 0x7fffffffu - rb(4) : 0x80000000u + rb(4);
    case 4: return (uint32_t)rnd() >> rb(32);
    case 5: return (uint32_t)((int32_t)((uint32_t)rnd() >> rb(32)) * (rb(2) ? -1 : 1));
    case 6: return rb(256);
    case 7: { uint32_t k = 1 + rb(31); return (uint32_t)((int32_t)((1u << k) + rb(5) - 2) * (rb(2) ? -1 : 1)); }
    default: return (uint32_t)rnd() >> rb(31) << rb(8);
    }
}

static uint64_t geni64(void) {
    switch (rb(11)) {
    case 0: return rnd();
    case 1: return rb(40) - 20;
    case 2: return (uint64_t)((int64_t)(rb(2) ? 1 : -1) * ((INT64_C(1) << 53) + (int64_t)rb(9) - 4));
    case 3: return rb(2) ? UINT64_C(0x7fffffffffffffff) - rb(4) : UINT64_C(0x8000000000000000) + rb(4);
    case 4: return rnd() >> rb(64);
    case 5: return (uint64_t)((int64_t)(rnd() >> rb(64)) * (rb(2) ? -1 : 1));
    case 6: return (uint64_t)(int64_t)(int32_t)geni32();
    case 7: { uint32_t k = 1 + rb(63); return (uint64_t)((int64_t)((INT64_C(1) << k) + (int64_t)rb(9) - 4) * (rb(2) ? -1 : 1)); }
    case 8: { uint32_t k = 24 + rb(40); return (uint64_t)((int64_t)(((rnd() >> 1) & ((INT64_C(1) << k) - 1)) | (INT64_C(1) << k)) * (rb(2) ? -1 : 1)); }
    default: return rnd() >> rb(63) << rb(12);
    }
}

/* doubles that exercise conversion to integers and to binary32 */
static uint64_t gen_conv(int f64) {
    int64_t n;
    double v;
    switch (rb(14)) {
    case 0: n = (int64_t)geni32(); v = (double)(int32_t)n; break;                       /* an integer */
    case 1: n = (int64_t)(int32_t)geni32(); v = (double)n + 0.5; break;                  /* a tie */
    case 2: n = (int64_t)(int32_t)geni32(); v = (double)n + (rb(2) ? 0.25 : 0.75); break;
    case 3: n = (int64_t)(int32_t)geni32(); v = (double)n - 0.5; break;
    case 4: v = (double)(int64_t)geni64(); break;
    case 5: n = (int64_t)geni64() >> 11; v = (double)n + 0.5; break;
    case 6: v = ((double)(int32_t)geni32()) / 3.0; break;
    case 7: v = (double)(rb(2) ? 2147483647.0 : -2147483648.0) + (double)(int)rb(5) - 2.0 + (rb(2) ? 0.5 : 0.0); break;
    case 8: v = 2251799813685248.0 + (double)(int)rb(5) - 2.0 + (rb(2) ? 0.5 : 0.0); break;
    case 9: return f64 ? gen_any(1) : gen_any(0);
    case 10: v = (double)(int32_t)geni32() * 1e-3; break;
    case 11: v = rb(2) ? 0.5 : -0.5; v += (double)(int)rb(3) - 1.0; break;
    default: return f64 ? gen_friendly(1) : gen_friendly(0);
    }
    if (!f64) {
        float f = (float)v;
        uint32_t u;
        memcpy(&u, &f, 4);
        return u;
    }
    uint64_t u;
    memcpy(&u, &v, 8);
    return u;
}

/* doubles near binary32 values: ties between binary32 neighbours, the binary32 range edges */
static uint64_t gen_d2s(void) {
    uint32_t w;
    float f;
    double d, e;
    uint64_t u;
    switch (rb(6)) {
    case 0: w = (uint32_t)rnd(); break;
    case 1: w = (uint32_t)gen_friendly(0); break;
    case 2: w = (uint32_t)gen_edge(0); break;
    case 3: w = (uint32_t)gen_few(0); break;
    default: w = (uint32_t)gen_any(0); break;
    }
    memcpy(&f, &w, 4);
    d = (double)f;
    e = d;
    switch (rb(5)) {
    case 0: break;
    case 1: { uint32_t w2 = w + 1; float f2; memcpy(&f2, &w2, 4); e = (d + (double)f2) / 2.0; break; } /* the tie */
    case 2: { uint64_t b; memcpy(&b, &d, 8); b += rb(5) - 2; memcpy(&e, &b, 8); break; }
    case 3: { uint32_t w2 = w + 1; float f2; memcpy(&f2, &w2, 4); e = d + ((double)f2 - d) * (rb(2) ? 0.5000000001 : 0.4999999999); break; }
    default: e = d * (1.0 + (double)(int)rb(3) * 1e-12); break;
    }
    memcpy(&u, &e, 8);
    return u;
}

/* ------------------------------------------------------------------------------------------------------------- */
/* test case construction */

static double bits2d(uint64_t u) { double d; memcpy(&d, &u, 8); return d; }
static uint64_t d2bits(double d) { uint64_t u; memcpy(&u, &d, 8); return u; }


enum { OC_ADD, OC_MUL, OC_DIV, OC_SQRT, OC_OTHER };

static int op_class(const char *n) {
    if (!strncmp(n, "add", 3) || !strncmp(n, "sub", 3) || !strncmp(n, "hadd", 4)) return OC_ADD;
    if (!strncmp(n, "mul", 3)) return OC_MUL;
    if (!strncmp(n, "div", 3)) return OC_DIV;
    if (!strncmp(n, "sqrt", 4)) return OC_SQRT;
    return OC_OTHER;
}


/* 2^e / a (e small): a product a * b of such a pair is within a few ulps of a power of two, where the carry of the
 * rounding (c = 1 with a zero fraction) is decided */
static uint64_t gen_recip(int f64, uint64_t a) {
    int mb = f64 ? 52 : 23;
    int e = (int)rb(7) - 3;
    int64_t t;
    uint64_t r;
    if (f64) {
        double x = bits2d(a);
        r = d2bits(1.0 / x);
    } else {
        float x, y;
        uint32_t w = (uint32_t)a, wy;
        memcpy(&x, &w, 4);
        y = 1.0f / x;
        memcpy(&wy, &y, 4);
        r = wy;
    }
    t = (int64_t)(((r >> mb) & (f64 ? 0x7ffu : 0xffu))) + e;
    if (t < 1) t = 1;
    if (t > (f64 ? 2046 : 254)) t = f64 ? 2046 : 254;
    r = (r & ~(((f64 ? UINT64_C(0x7ff) : UINT64_C(0xff)) << mb))) | ((uint64_t)t << mb);
    return f64 ? r + (rb(5) - 2) : (r + (rb(5) - 2)) & 0xffffffffull;
}

/* a pair of operands for a binary operation of class oc */
static void gen_pair(int f64, int oc, uint64_t *a, uint64_t *b) {
    unsigned k = rb(100);
    if (oc == OC_DIV && k < 18) { /* an exact quotient: a = q * y */
        uint64_t q = gen_few(f64), y = gen_few(f64);
        double dq = f64 ? bits2d(q) : (double)(float)bits2d(0), dy;
        if (f64) {
            dy = bits2d(y);
            *a = d2bits(dq * dy);
            *b = y;
        } else {
            float fq, fy, fa;
            uint32_t uq = (uint32_t)q, uy = (uint32_t)y, ua;
            memcpy(&fq, &uq, 4);
            memcpy(&fy, &uy, 4);
            fa = fq * fy;
            memcpy(&ua, &fa, 4);
            *a = ua;
            *b = uy;
        }
        return;
    }
    if (oc == OC_DIV && k < 30) { /* a quotient a hair away from exact */
        uint64_t q = gen_few(f64), y = gen_few(f64);
        if (f64) {
            *a = d2bits(bits2d(q) * bits2d(y)) + (rb(3) - 1);
            *b = y;
        } else {
            float fq, fy, fa;
            uint32_t uq = (uint32_t)q, uy = (uint32_t)y, ua;
            memcpy(&fq, &uq, 4);
            memcpy(&fy, &uy, 4);
            fa = fq * fy;
            memcpy(&ua, &fa, 4);
            *a = ua + (rb(3) - 1);
            *b = uy;
        }
        return;
    }
    if (oc == OC_MUL && k < 12) { *a = gen_friendly(f64); *b = gen_recip(f64, *a); return; }
    if (oc == OC_DIV && k < 40) { *b = gen_friendly(f64); *a = gen_recip(f64, *b); return; } /* a / b = 2^e / b / b ... near powers of two */
    if (k < 38) { *a = gen_friendly(f64); *b = gen_friendly(f64); return; }
    if (k < 58) { *a = gen_any(f64); *b = gen_rel(f64, *a); return; }
    if (k < 72) { *a = gen_few(f64); *b = gen_few(f64); return; }
    if (k < 78) { *a = gen_edge(f64); *b = rb(2) ? gen_edge(f64) : gen_friendly(f64); return; }
    if (k < 84) { *a = gen_friendly(f64); *b = gen_rel(f64, *a); return; }
    if (k < 92) { *a = gen_any(f64); *b = gen_any(f64); return; }
    { uint64_t x = rnd(), y = rnd(); *a = f64 ? x : (x & 0xffffffffull); *b = f64 ? y : (y & 0xffffffffull); }
}

/* an operand for a unary operation (SQRT, conversions) of kind: 0 sqrt, 1 float to int, 2 double to float, 3 float to double */
static uint64_t gen_unary(int f64, int kind) {
    switch (kind) {
    case 0: { /* sqrt: positive mostly, squares */
        uint64_t u;
        if (rb(100) < 25) {
            uint64_t r = gen_few(f64);
            if (f64) {
                double x = bits2d(r);
                u = d2bits(x * x);
            } else {
                float x, y;
                uint32_t w = (uint32_t)r, wy;
                memcpy(&x, &w, 4);
                y = x * x;
                memcpy(&wy, &y, 4);
                u = wy;
            }
            if (rb(4) == 0) u += rb(3) - 1;
        } else {
            u = gen_any(f64);
        }
        if (rb(100) < 90) u &= ~(UINT64_C(1) << (f64 ? 63 : 31));
        return u;
    }
    case 1: return gen_conv(f64);
    case 2: return gen_d2s();
    default: return rb(4) == 0 ? gen_any(0) : gen_friendly(0);
    }
}

static uint32_t gen_mxcsr(void) {
    uint32_t mx = 0x1f80u;
    unsigned k = rb(100);
    if (k < 35) {
        /* default controls */
    } else if (k < 55) {
        mx |= 0x8040u; /* FTZ and DAZ, as the engine runs (0x9fc0 and 0x9fe0 are what it loads) */
    } else if (k < 68) {
        mx |= (uint32_t)rb(2) << 15 | (uint32_t)rb(2) << 6; /* FTZ, DAZ only */
    } else if (k < 84) {
        mx |= (uint32_t)(1 + rb(3)) << 13; /* RC != 0 */
        if (rb(3) == 0) mx |= (uint32_t)rb(2) << 15 | (uint32_t)rb(2) << 6;
    } else {
        mx |= (uint32_t)rb(4) << 13 | (uint32_t)rb(2) << 15 | (uint32_t)rb(2) << 6;
    }
    if (rb(4) == 0) mx |= (uint32_t)rb(64); /* sticky flags already set */
    if (rb(3) == 0) mx |= 0x20u; /* PE already set (the engine runs with 0x9fe0): the guard of decision 0154 opens */
    return mx;
}

struct Case {
    xmm_t d, s;
    uint64_t g;
    uint32_t mx;
};

static void gen_case(const struct MxfpOp *op, struct Case *c) {
    int oc = op_class(op->name), lanes, i;
    c->mx = gen_mxcsr();
    c->d.u64[0] = rnd();
    c->d.u64[1] = rnd();
    c->s.u64[0] = rnd();
    c->s.u64[1] = rnd();
    c->g = 0;
    if (op->sk == 3) {
        c->g = rb(2) ? geni64() : (uint64_t)(int64_t)(int32_t)geni32();
        return;
    }
    if (op->sk == 2) {
        for (i = 0; i < 4; i++) c->s.u32[i] = rb(3) ? geni32() : (uint32_t)rnd();
        return;
    }
    {
        int f64 = op->sk == 1;
        int unary = !strncmp(op->name, "sqrt", 4) || !strncmp(op->name, "cvt", 3) || !strncmp(op->name, "round", 5);
        int packed_friendly = rb(2);
        lanes = f64 ? 2 : 4;
        if (op->comi || !unary) {
            for (i = 0; i < lanes; i++) {
                uint64_t a, b;
                gen_pair(f64, oc, &a, &b);
                if (packed_friendly && lanes > 1 && i > 0 && rb(10) < 8) {
                    uint64_t a2 = gen_friendly(f64), b2 = gen_friendly(f64);
                    a = a2;
                    b = b2;
                }
                if (f64) { c->d.u64[i] = a; c->s.u64[i] = b; } else { c->d.u32[i] = (uint32_t)a; c->s.u32[i] = (uint32_t)b; }
            }
        } else {
            int kind = 1;
            if (!strncmp(op->name, "sqrt", 4)) kind = 0;
            else if (!strcmp(op->name, "cvtsd2ss") || !strcmp(op->name, "cvtpd2ps")) kind = 2;
            else if (!strcmp(op->name, "cvtss2sd") || !strcmp(op->name, "cvtps2pd")) kind = 3;
            int tame = rb(100) < 50; /* all lanes friendly: the vector windows of decision 0154 open */
            for (i = 0; i < lanes; i++) {
                uint64_t a = tame && kind == 1 ? gen_friendly(f64) : gen_unary(f64, kind);
                if (tame && kind == 1 && rb(3) == 0) { /* a small value with a fraction: a tie, near an integer */
                    double v = ((double)(int32_t)rb(4000001) - 2000000.0) / (rb(2) ? 2.0 : 1.0 + (double)rb(7));
                    if (f64) a = d2bits(v); else { float f = (float)v; uint32_t u; memcpy(&u, &f, 4); a = u; }
                }
                if (f64) c->s.u64[i] = a; else c->s.u32[i] = (uint32_t)a;
            }
        }
    }
}

/* ------------------------------------------------------------------------------------------------------------- */
/* running */

struct Res {
    xmm_t d;
    uint64_t g;
    uint32_t mx;
    uint64_t fl;
};

static uint64_t fl_bits(void) {
    return ((uint64_t)FL.cf << 0) | ((uint64_t)FL.pf << 1) | ((uint64_t)FL.af << 2) | ((uint64_t)FL.zf << 3) |
           ((uint64_t)FL.sf << 4) | ((uint64_t)FL.of << 5);
}

static void run(const struct MxfpOp *op, const struct Case *c, struct Res *r) {
    r->d = c->d;
    r->g = c->g;
    FL.cf = FL.pf = FL.af = FL.zf = FL.sf = FL.of = FLE_UNDEF;
    fle_mxcsr = c->mx;
    op->fn(&r->d, &c->s, &r->g);
    r->mx = fle_mxcsr;
    r->fl = op->comi ? fl_bits() : fl_bits();
}

static int same(const struct Res *a, const struct Res *b) {
    return a->d.u64[0] == b->d.u64[0] && a->d.u64[1] == b->d.u64[1] && a->g == b->g && a->mx == b->mx && a->fl == b->fl;
}

static void show(const char *what, const struct MxfpOp *op, const struct Case *c, const struct Res *x, const struct Res *y,
                 const char *xn, const char *yn) {
    printf("MISMATCH %s %s: mx_in %#x\n  d  %016llx %016llx\n  s  %016llx %016llx g %016llx\n"
           "  %-6s %016llx %016llx g %016llx mx %#x fl %#llx\n  %-6s %016llx %016llx g %016llx mx %#x fl %#llx\n",
           what, op->name, c->mx, (unsigned long long)c->d.u64[1], (unsigned long long)c->d.u64[0],
           (unsigned long long)c->s.u64[1], (unsigned long long)c->s.u64[0], (unsigned long long)c->g, xn,
           (unsigned long long)x->d.u64[1], (unsigned long long)x->d.u64[0], (unsigned long long)x->g, x->mx,
           (unsigned long long)x->fl, yn, (unsigned long long)y->d.u64[1], (unsigned long long)y->d.u64[0],
           (unsigned long long)y->g, y->mx, (unsigned long long)y->fl);
}

/* the id of the vector counters of fle_mxcsr_vec.h / fle_mxcsr_cvt.h for an operation name, and whether the vector path
 * needs the guard (RC = 0, PE set, PM set) to apply: need 2 = windows only, no guard needed at all (exact operations) */
static int vid_of(const char *n, int *need_np0) {
    static const char *const cv[] = {"cvtss2sd", "cvtsd2ss", "cvtps2pd", "cvtpd2ps", "cvtdq2ps", "cvtdq2pd", "cvtps2dq", "cvttps2dq",
                                     "cvtpd2dq", "cvttpd2dq", "cvtsi2ss32", "cvtsi2ss64", "cvtsi2sd32", "cvtsi2sd64", "cvtss2si32",
                                     "cvtss2si64", "cvtsd2si32", "cvtsd2si64", "cvttss2si32", "cvttss2si64", "cvttsd2si32", "cvttsd2si64"};
    static const int cvneed[] = {0, 1, 0, 1, 2, 0, 1, 3, 1, 3, 2, 2, 0, 2, 1, 1, 1, 1, 3, 3, 3, 3}; /* 0 none, 1 guard (RC = 0, PE, PM), 2 guard or exact, 3 PE and PM only (truncation does not depend on RC, decision 0172) */
    static const char *const ar[] = {"add", "sub", "mul", "div", "min", "max", "sqrt"};
    size_t l = strlen(n);
    *need_np0 = 0;
    for (unsigned i = 0; i < sizeof cv / sizeof cv[0]; i++) if (!strcmp(n, cv[i])) { *need_np0 = cvneed[i]; return 40 + (int)i; }
    if (!strncmp(n, "cmp", 3) && (n[3] == 's' || n[3] == 'p')) {
        int f64 = n[4] == 'd', scalar = n[3] == 's';
        return 32 + 4 * f64 + 2 * scalar;
    }
    for (unsigned i = 0; i < 7; i++) {
        size_t k = strlen(ar[i]);
        if (!strncmp(n, ar[i], k) && l == k + 2 && n[k] != '2') {
            int f64 = n[k + 1] == 'd', packed = n[k] == 'p';
            if (n[k] != 's' && n[k] != 'p') continue;
            *need_np0 = (i == 4 || i == 5) ? 0 : 1;
            return (int)i + 8 * f64 + 16 * packed;
        }
    }
    return -1;
}

struct Stat {
    unsigned long long vec_hit, vec_viol, vec_cases;
    unsigned long long cases, bad_old, bad_nofast, bad_host, host_cases;
    unsigned long long fast_all, fast_any, lane_hit, lane_miss, guard_viol, guard_cases;
    unsigned long long fast_all_pe, fast_all_nope, fast_all_zero_res;
};

static const struct MxfpOp *find(const struct MxfpOp *tab, unsigned n, const char *name) {
    for (unsigned i = 0; i < n; i++) if (!strcmp(tab[i].name, name)) return &tab[i];
    return NULL;
}

static int match(const char *filter, const char *name) {
    size_t n = strlen(filter);
    if (!strcmp(filter, "*")) return 1;
    if (n && filter[n - 1] == '*') return !strncmp(filter, name, n - 1);
    return !strcmp(filter, name);
}

/* compare one case across the implementations */
static void one_case(const struct MxfpOp *on, const struct MxfpOp *oo, const struct MxfpOp *of, const struct MxfpOp *oh,
                     const struct Case *c, struct Stat *st, int verbose_max) {
    struct Res rn, ro, rf, rh;
    unsigned long long h0 = 0, m0 = 0, v0 = 0;
    int sid = on->sid, need = 0, vid = vid_of(on->name, &need), vec = 0;
    if (sid >= 0) { h0 = fle_mx_stat[sid][1]; m0 = fle_mx_stat[sid][0]; }
    if (vid >= 0) v0 = fle_mx_vstat[0][vid] + fle_mx_rcstat[0][vid];
    run(on, c, &rn);
    if (vid >= 0) {
        st->vec_cases++;
        if (fle_mx_vstat[0][vid] + fle_mx_rcstat[0][vid] != v0) {
            vec = 1;
            st->vec_hit++;
            /* the guard of decision 0154: operations that can be inexact need RC = 0 and PE and PM set */
            /* decision 0172: the path is taken with RC != 0 too (the directed-rounding code), so the guard that is checked is PE set and PM masked */
            if ((need == 1 || need == 3) && ((c->mx & 0x1020u) != 0x1020u)) st->vec_viol++;
            if (need == 1 && ((c->mx & 0x6000u) == 0) && ((c->mx & 0x1020u) != 0x1020u)) st->vec_viol++;
        }
    }
    if (sid >= 0) {
        unsigned long long dh = fle_mx_stat[sid][1] - h0, dm = fle_mx_stat[sid][0] - m0;
        st->lane_hit += dh;
        st->lane_miss += dm;
        if (dh) st->fast_any++;
        if (vec || (dh && dh == (unsigned long long)on->lanes && !dm)) {
            st->fast_all++;
            if (rn.mx & ~c->mx & 0x20) st->fast_all_pe++; else st->fast_all_nope++;
        }
        if ((c->mx & MXFP_EXPECT_CTL) && !on->comi) { /* the flags of a comparison do not depend on RC (decision 0172) */
            st->guard_cases++;
            if (dh) st->guard_viol++;
        }
    }
    run(oo, c, &ro);
    st->cases++;
    if (!same(&rn, &ro)) {
        if (++st->bad_old <= (unsigned long long)verbose_max) show("new/old", on, c, &rn, &ro, "new", "old");
    }
    if (of) {
        run(of, c, &rf);
        if (!same(&rf, &ro)) {
            if (++st->bad_nofast <= (unsigned long long)verbose_max) show("nofast/old", on, c, &rf, &ro, "nofast", "old");
        }
    }
    if (oh) {
        run(oh, c, &rh);
        st->host_cases++;
        if (!same(&rn, &rh)) {
            if (++st->bad_host <= (unsigned long long)verbose_max) show("new/host", on, c, &rn, &rh, "new", "host");
        }
    }
}

static void report(const char *mode, const char *name, const struct Stat *st, int host) {
    printf("OP %s %s cases %llu mism_old %llu mism_nofast %llu mism_host %llu/%llu fast_all %llu (%.1f%%) fast_any %llu lane_hit %llu lane_miss %llu"
           " fast_all_pe %llu fast_all_exact %llu guard_cases %llu guard_viol %llu\n",
           mode, name, st->cases, st->bad_old, st->bad_nofast, st->bad_host, st->host_cases, st->fast_all,
           st->cases ? 100.0 * (double)st->fast_all / (double)st->cases : 0.0, st->fast_any, st->lane_hit, st->lane_miss,
           st->fast_all_pe, st->fast_all_nope, st->guard_cases, st->guard_viol);
    if (st->vec_cases) printf("  VEC %s %s cases %llu vec_hit %llu (%.1f%%) vec_guard_viol %llu\n", mode, name, st->vec_cases, st->vec_hit,
                               100.0 * (double)st->vec_hit / (double)st->vec_cases, st->vec_viol);
    (void)host;
}

static int cmd_rand(const char *filter, unsigned long long cases, uint64_t seed, int host) {
    unsigned long long bad = 0, viol = 0, total = 0;
    for (unsigned i = 0; i < MXFP_N_new; i++) {
        const struct MxfpOp *on = &MXFP_OPS_new[i];
        const struct MxfpOp *oo, *of, *oh;
        struct Stat st;
        if (!match(filter, on->name)) continue;
        oo = find(MXFP_OPS_old, MXFP_N_old, on->name);
        of = find(MXFP_OPS_nofast, MXFP_N_nofast, on->name);
        oh = host ? find(MXFP_OPS_host, MXFP_N_host, on->name) : NULL;
        if (!oo) { printf("no old for %s\n", on->name); return 2; }
        memset(&st, 0, sizeof st);
        seed_for(on->name, seed);
        /* operations the fast path does not touch run a tenth of the cases */
        int need_unused;
        unsigned long long n = (on->sid >= 0 || vid_of(on->name, &need_unused) >= 0) ? cases : (cases / 10 ? cases / 10 : 1);
        for (unsigned long long k = 0; k < n; k++) {
            struct Case c;
            gen_case(on, &c);
            one_case(on, oo, of, oh, &c, &st, 4);
        }
        report("rand", on->name, &st, host);
        if (on->sid >= 0 && on->sid != 12 && on->sid != 13) {
            /* the fast path has to be exercised: a mutation that makes it never apply must not survive */
            int pe_possible = !(on->comi || !strncmp(on->name, "min", 3) || !strncmp(on->name, "max", 3) || !strncmp(on->name, "cvtss2sd", 8) ||
                                !strncmp(on->name, "cvtps2pd", 8) || !strncmp(on->name, "cvtdq2pd", 8) || !strncmp(on->name, "cvtsi2sd32", 10));
            if (st.fast_all * 100 < 10 * st.cases || (pe_possible && (st.fast_all_pe == 0 || st.fast_all_nope == 0))) {
                printf("LOW COVERAGE %s: fast_all %llu of %llu, with PE %llu, exact %llu\n", on->name, st.fast_all, st.cases, st.fast_all_pe, st.fast_all_nope);
                bad++;
            }
        }
        if (st.vec_cases && st.vec_hit * 100 < 3 * st.vec_cases) {
            printf("LOW VECTOR COVERAGE %s: vec_hit %llu of %llu\n", on->name, st.vec_hit, st.vec_cases);
            bad++;
        }
        bad += st.bad_old + st.bad_nofast + st.bad_host;
        viol += st.guard_viol + st.vec_viol;
        total += st.cases;
    }
    printf("TOTAL rand cases %llu mismatches %llu guard_violations %llu\n", total, bad, viol);
    return bad || viol ? 1 : 0;
}

/* ------------------------------------------------------------------------------------------------------------- */
/* directed rounding (decision 0172): new against the instructions on the host CPU, for every RC != 0 and FTZ, DAZ */

static int cmd_rc(const char *filter, unsigned long long cases, uint64_t seed, int rcfilter) {
    unsigned long long bad = 0, total = 0, lowcov = 0;
    for (unsigned i = 0; i < MXFP_N_new; i++) {
        const struct MxfpOp *on = &MXFP_OPS_new[i];
        const struct MxfpOp *oh;
        int need, vid;
        if (!match(filter, on->name)) continue;
        vid = vid_of(on->name, &need);
        oh = find(MXFP_OPS_host, MXFP_N_host, on->name);
        if (!oh) oh = find(MXFP_OPS_old, MXFP_N_old, on->name); /* no host CPU reference (wasm): the integer implementation, which the host test checks */
        if (!oh || (vid < 0 && !on->comi) || !strncmp(on->name, "min", 3) || !strncmp(on->name, "max", 3) || !strncmp(on->name, "cmp", 3)) continue;
        for (unsigned rc = 1; rc < 4; rc++) {
            if (rcfilter && (int)rc != rcfilter) continue;
            for (unsigned fd = 0; fd < 4; fd++) {
                unsigned long long hit = 0, mism = 0, n = 0, hit_pe = 0, nohit_pe = 0;
                uint64_t before = 0;
                seed_for(on->name, seed + rc * 16 + fd);
                for (unsigned long long k = 0; k < cases; k++) {
                    struct Case c;
                    struct Res rn, rh;
                    gen_case(on, &c);
                    /* the control bits of this combination; the masks all set mostly, PE set in 90% (the guard), flags otherwise random */
                    c.mx = 0x1f80u | rc << 13 | (fd & 1u) << 15 | (fd >> 1) << 6;
                    if (rb(10) != 0) c.mx |= 0x20u;
                    if (rb(8) == 0) c.mx |= (uint32_t)rb(64);
                    before = vid >= 0 ? fle_mx_rcstat[0][vid] + fle_mx_vstat[0][vid] : fle_mx_stat[on->sid][1];
                    run(on, &c, &rn);
                    int h = (vid >= 0 ? fle_mx_rcstat[0][vid] + fle_mx_vstat[0][vid] : fle_mx_stat[on->sid][1]) != before;
                    run(oh, &c, &rh);
                    n++;
                    hit += h;
                    if (h) hit_pe++; else nohit_pe += 0;
                    if (!same(&rn, &rh)) {
                        if (++mism <= 4) show("rc new/host", on, &c, &rn, &rh, "new", "host");
                    }
                }
                printf("RC %s rc %u ftz %u daz %u cases %llu rc_hit %llu (%.1f%%) mism_ref %llu\n", on->name, rc, fd & 1u, fd >> 1, n, hit,
                       n ? 100.0 * (double)hit / (double)n : 0.0, mism);
                (void)hit_pe; (void)nohit_pe;
                if (hit * 100 < 5 * n && vid >= 0 && (need == 1 || need == 2) && strncmp(on->name, "cvtt", 4) && strncmp(on->name, "cvtss2sd", 8) &&
                    strncmp(on->name, "cvtps2pd", 8) && strncmp(on->name, "cvtdq2pd", 8) && strncmp(on->name, "cvtsi2sd32", 10)) {
                    printf("LOW RC COVERAGE %s rc %u: %llu of %llu\n", on->name, rc, hit, n);
                    lowcov++;
                }
                bad += mism;
                total += n;
            }
        }
    }
    printf("TOTAL rc cases %llu mismatches %llu low_coverage %llu\n", total, bad, lowcov);
    return bad || lowcov ? 1 : 0;
}

/* ------------------------------------------------------------------------------------------------------------- */
/* every control and mask state (decision 0172): RC (4) x FTZ x DAZ x the five masks IM DM ZM OM UM x PM x PE already set = 1024 values of MXCSR (flags
 * 4:0 random). For each, `cases` cases of each operation are run by the new code and by the instruction on the host CPU; an unmasked flag makes both stop
 * (the host header aborts, so does the runtime), and the two must stop on the same cases. Counts how many cases took the directed-rounding path. */

static sigjmp_buf jb2;
static void on_abort2(int sg) { (void)sg; siglongjmp(jb2, 1); }

static int cmd_mxall(const char *filter, unsigned long long cases, uint64_t seed) {
    unsigned long long bad = 0, total = 0, rchit = 0, aborts = 0, hostcases = 0;
    struct sigaction sa;
    FILE *devnull = freopen("/dev/null", "w", stderr);
    (void)devnull;
    memset(&sa, 0, sizeof sa);
    sa.sa_handler = on_abort2;
    sigaction(SIGABRT, &sa, NULL);
    for (unsigned i = 0; i < MXFP_N_new; i++) {
        const struct MxfpOp *on = &MXFP_OPS_new[i];
        const struct MxfpOp *oh, *oo;
        int need, vid;
        unsigned long long opbad = 0, ophit = 0, opn = 0, opab = 0;
        if (!match(filter, on->name)) continue;
        vid = vid_of(on->name, &need);
        oh = find(MXFP_OPS_host, MXFP_N_host, on->name);
        oo = find(MXFP_OPS_old, MXFP_N_old, on->name);
        if (!oo || vid < 0 || !strncmp(on->name, "min", 3) || !strncmp(on->name, "max", 3) || !strncmp(on->name, "cmp", 3)) continue;
        seed_for(on->name, seed);
        for (unsigned combo = 0; combo < 1024; combo++) {
            unsigned rc = combo & 3, ftz = (combo >> 2) & 1, daz = (combo >> 3) & 1, masks = (combo >> 4) & 31, pm = (combo >> 9) & 1;
            for (unsigned long long k = 0; k < cases; k++) {
                struct Case c;
                struct Res rn, ro, rh;
                int an = 0, ao = 0, ah = 0, usehost = oh && masks == 31;
                unsigned long long h0;
                gen_case(on, &c);
                c.mx = rc << 13 | ftz << 15 | daz << 6 | masks << 7 | pm << 12 | (uint32_t)rb(32) | (rb(2) ? 0x20u : 0u);
                h0 = fle_mx_rcstat[0][vid] + fle_mx_vstat[0][vid];
                if (sigsetjmp(jb2, 1) == 0) run(on, &c, &rn); else an = 1;
                if (sigsetjmp(jb2, 1) == 0) run(oo, &c, &ro); else ao = 1;
                opn++;
                ophit += (fle_mx_rcstat[0][vid] + fle_mx_vstat[0][vid]) != h0;
                opab += ao;
                if (an != ao || (!an && !same(&rn, &ro))) {
                    if (++opbad <= 3) printf("MISMATCH mxall new/old %s mx %#x abort new %d old %d d %016llx %016llx s %016llx %016llx\n", on->name, c.mx, an, ao, (unsigned long long)c.d.u64[1], (unsigned long long)c.d.u64[0], (unsigned long long)c.s.u64[1], (unsigned long long)c.s.u64[0]);
                }
                if (usehost) { /* the host reference runs with every mask set, so it is a reference only where the guest masks everything but possibly PM */
                    if (sigsetjmp(jb2, 1) == 0) run(oh, &c, &rh); else ah = 1;
                    hostcases++;
                    if (an != ah || (!an && !same(&rn, &rh))) {
                        if (++opbad <= 3) printf("MISMATCH mxall new/host %s mx %#x abort new %d host %d\n", on->name, c.mx, an, ah);
                    }
                }
            }
        }
        printf("OP mxall %s mx_values 1024 cases %llu rc_or_vec_path %llu aborts %llu mismatches %llu\n", on->name, opn, ophit, opab, opbad);
        bad += opbad;
        total += opn;
        rchit += ophit;
        aborts += opab;
    }
    printf("TOTAL mxall cases %llu host_cases %llu path_hits %llu aborts %llu mismatches %llu\n", total, hostcases, rchit, aborts, bad);
    return bad ? 1 : 0;
}

/* ------------------------------------------------------------------------------------------------------------- */
/* operands recorded from a Serum 2 process (tools/serum_ops.py): 48-byte records {u32 kind, u32 aux, u32 mxcsr, u32 pad, u8 d[16], u8 s[16]}; new against old and the host */

static int cmd_replay128(const char *file, int host) {
    FILE *f = fopen(file, "rb");
    struct { uint32_t kind, aux, mx, pad; uint8_t d[16], s[16]; } rec;
    static const char *const an[7] = {"add", "sub", "mul", "div", "min", "max", "sqrt"};
    static const char *const cn[5] = {0, "cvtsd2ss", "cvttsd2si64", "cvttss2si32", "cvtsd2si32"};
    struct Stat st[64];
    char names[64][24];
    unsigned long long bad = 0, n = 0;
    unsigned nn = 0;
    if (!f) { perror(file); return 2; }
    memset(st, 0, sizeof st);
    memset(names, 0, sizeof names);
    while (fread(&rec, sizeof rec, 1, f) == 1) {
        char nm[24];
        const struct MxfpOp *on, *oo, *of, *oh;
        unsigned k;
        if (rec.kind == 0) {
            unsigned op = rec.aux & 255, f64 = (rec.aux >> 8) & 1, sc = (rec.aux >> 16) & 1;
            if (op > 6) continue;
            snprintf(nm, sizeof nm, "%s%c%c", an[op], sc ? 's' : 'p', f64 ? 'd' : 's');
        } else if (rec.kind >= 1 && rec.kind <= 4) {
            snprintf(nm, sizeof nm, "%s", cn[rec.kind]);
        } else continue;
        for (k = 0; k < nn; k++) if (!strcmp(names[k], nm)) break;
        if (k == nn) { if (nn == 64) continue; snprintf(names[nn++], 24, "%s", nm); }
        on = find(MXFP_OPS_new, MXFP_N_new, nm);
        oo = find(MXFP_OPS_old, MXFP_N_old, nm);
        of = find(MXFP_OPS_nofast, MXFP_N_nofast, nm);
        oh = host ? find(MXFP_OPS_host, MXFP_N_host, nm) : NULL;
        if (!on || !oo) continue;
        for (int pe = 0; pe < 2; pe++) { /* the recorded state (flags cleared), and with PE already set */
            struct Case c;
            memcpy(&c.d, rec.d, 16);
            memcpy(&c.s, rec.s, 16);
            c.g = 0;
            c.mx = (rec.mx & 0xFFC0u) | (pe ? 0x20u : 0u);
            one_case(on, oo, of, oh, &c, &st[k], 8);
            n++;
        }
    }
    fclose(f);
    for (unsigned k = 0; k < nn; k++) {
        report("serum", names[k], &st[k], host);
        bad += st[k].bad_old + st[k].bad_nofast + st[k].bad_host + st[k].vec_viol;
    }
    printf("TOTAL serum cases %llu mismatches %llu\n", n, bad);
    return bad ? 1 : 0;
}

/* ------------------------------------------------------------------------------------------------------------- */
/* grid: binary32 values by (sign, exponent, mantissa class) for both operands */

static const uint32_t GM[] = {0x000000, 0x000001, 0x000002, 0x000003, 0x400000, 0x400001, 0x3fffff, 0x7ffffe, 0x7fffff,
                              0x200000, 0x100000, 0x555555, 0x2aaaaa, 0x7fffff ^ 0x40, 0x600000, 0x7f0000, 0x000100, 0x010101,
                              0x123456, 0x654321, 0x1ffffff & 0x7fffff, 0x3c0000, 0x7c0000, 0x00ffff};

static int cmd_grid32(const char *filter, unsigned stride) {
    unsigned long long bad = 0, total = 0;
    static const char *names[] = {"addss", "subss", "mulss", "divss", "minss", "maxss", "ucomiss", "comiss", "addps", "mulps", "divps",
                                  "sqrtss", "cvtss2sd", "cvtss2si32", "cvttss2si32", "cvtss2si64", "cvttss2si64", "cvtps2dq",
                                  "cvttps2dq", "cvtdq2ps", "cvtsi2ss32"};
    uint32_t ctlset[] = {0x1f80, 0x1f80 | 0x8000, 0x1f80 | 0x40, 0x1f80 | (1u << 13), 0x1f80 | 0x8040};
    for (unsigned ni = 0; ni < sizeof names / sizeof names[0]; ni++) {
        const struct MxfpOp *on = find(MXFP_OPS_new, MXFP_N_new, names[ni]);
        const struct MxfpOp *oo, *of;
        struct Stat st;
        int unary;
        if (!on || !match(filter, on->name)) continue;
        oo = find(MXFP_OPS_old, MXFP_N_old, on->name);
        of = find(MXFP_OPS_nofast, MXFP_N_nofast, on->name);
        memset(&st, 0, sizeof st);
        unary = !strncmp(on->name, "sqrt", 4) || !strncmp(on->name, "cvt", 3);
        uint64_t n = 0;
        for (unsigned sa = 0; sa < 2; sa++)
            for (unsigned ea = 0; ea < 256; ea++)
                for (unsigned ma = 0; ma < sizeof GM / sizeof GM[0]; ma++) {
                    uint32_t a = (sa << 31) | (ea << 23) | GM[ma];
                    if (unary || on->sk == 2 || on->sk == 3) {
                        struct Case c;
                        memset(&c, 0, sizeof c);
                        c.s.u32[0] = c.s.u32[1] = c.s.u32[2] = c.s.u32[3] = a;
                        c.d.u64[0] = 0x0123456789abcdefull;
                        c.d.u64[1] = 0xfedcba9876543210ull;
                        c.g = (uint64_t)(int64_t)(int32_t)a;
                        c.mx = ctlset[n++ % 5];
                        one_case(on, oo, of, NULL, &c, &st, 4);
                        continue;
                    }
                    for (unsigned sb = 0; sb < 2; sb++)
                        for (unsigned eb = 0; eb < 256; eb += stride)
                            for (unsigned mb = 0; mb < sizeof GM / sizeof GM[0]; mb++) {
                                struct Case c;
                                uint32_t b = (sb << 31) | (eb << 23) | GM[mb];
                                c.d.u32[0] = a;
                                c.d.u32[1] = a ^ 0x00400000u;
                                c.d.u32[2] = (a & 0x80000000u) | ((a + 0x00800000u) & 0x7fffffffu);
                                c.d.u32[3] = a;
                                c.s.u32[0] = b;
                                c.s.u32[1] = b;
                                c.s.u32[2] = b ^ 0x80000000u;
                                c.s.u32[3] = (b & 0x80000000u) | ((b + 0x00800000u) & 0x7fffffffu);
                                c.g = 0;
                                c.mx = ctlset[n++ % 5];
                                one_case(on, oo, of, NULL, &c, &st, 4);
                            }
                }
        report("grid32", on->name, &st, 0);
        bad += st.bad_old + st.bad_nofast;
        total += st.cases;
        if (st.guard_viol) bad += st.guard_viol;
    }
    printf("TOTAL grid32 cases %llu mismatches %llu\n", total, bad);
    return bad ? 1 : 0;
}

/* ------------------------------------------------------------------------------------------------------------- */
/* operands recorded from the engine: records of {u32 id; u32 mx; u64 a; u64 b} */

static const char *ID_NAMES[40] = {
    "addss", "subss", "mulss", "divss", "minss", "maxss", "sqrtss", 0, "addsd", "subsd", "mulsd", "divsd", "minsd", "maxsd", "sqrtsd", 0,
    "cvtss2sd", "cvtsd2ss", "cvtsi2ss32", "cvtsi2ss64", "cvtsi2sd32", "cvtsi2sd64", 0, 0,
    "cvtss2si32", "cvttss2si32", "cvtss2si64", "cvttss2si64", "cvtsd2si32", "cvttsd2si32", "cvtsd2si64", "cvttsd2si64",
    "ucomiss", "comiss", "ucomisd", "comisd", 0, 0, 0, 0};

static int cmd_replay(const char *file, int host) {
    FILE *f = fopen(file, "rb");
    struct Rec { uint32_t id, mx; uint64_t a, b; } rec;
    struct Stat st[40];
    unsigned long long bad = 0, n = 0;
    if (!f) { perror(file); return 2; }
    memset(st, 0, sizeof st);
    seed_for("replay", 1);
    while (fread(&rec, sizeof rec, 1, f) == 1) {
        const struct MxfpOp *on, *oo, *of, *oh;
        struct Case c;
        if (rec.id >= 40 || !ID_NAMES[rec.id]) continue;
        on = find(MXFP_OPS_new, MXFP_N_new, ID_NAMES[rec.id]);
        oo = find(MXFP_OPS_old, MXFP_N_old, ID_NAMES[rec.id]);
        of = find(MXFP_OPS_nofast, MXFP_N_nofast, ID_NAMES[rec.id]);
        oh = host ? find(MXFP_OPS_host, MXFP_N_host, ID_NAMES[rec.id]) : NULL;
        c.d.u64[0] = rnd(); c.d.u64[1] = rnd(); c.s.u64[0] = rnd(); c.s.u64[1] = rnd();
        c.g = 0;
        c.mx = rec.mx & 0xFFC0u; /* the status flags are cleared: every flag the instruction raises is seen */
        for (int pe = 0; pe < 2; pe++) { /* second pass: PE already set (the engine's state, 0x9fe0), the guard of decision 0154 can open */
        c.mx = (rec.mx & 0xFFC0u) | (pe ? 0x20u : 0u);
        if (on->sk == 3) {
            c.g = rec.a;
        } else if (on->comi || rec.id < 16) {
            if (on->sk == 1) { c.d.u64[0] = rec.a; c.s.u64[0] = rec.b; } else { c.d.u32[0] = (uint32_t)rec.a; c.s.u32[0] = (uint32_t)rec.b; }
            if (rec.id == 6 || rec.id == 14) { if (on->sk == 1) c.s.u64[0] = rec.b; else c.s.u32[0] = (uint32_t)rec.b; }
        } else {
            if (on->sk == 1) c.s.u64[0] = rec.a; else c.s.u32[0] = (uint32_t)rec.a;
        }
        one_case(on, oo, of, oh, &c, &st[rec.id], 8);
        n++;
        }
    }
    fclose(f);
    for (unsigned i = 0; i < 40; i++) {
        if (!st[i].cases) continue;
        report("replay", ID_NAMES[i], &st[i], host);
        bad += st[i].bad_old + st[i].bad_nofast + st[i].bad_host;
    }
    printf("TOTAL replay cases %llu mismatches %llu\n", n, bad);
    return bad ? 1 : 0;
}

/* ------------------------------------------------------------------------------------------------------------- */
/* unmasked exceptions: the same abort, or the same result */

static sigjmp_buf jb;
static void on_abort(int s) { (void)s; siglongjmp(jb, 1); }

static int cmd_unmasked(unsigned long long cases, uint64_t seed) {
    unsigned long long bad = 0, aborts = 0, total = 0;
    struct sigaction sa;
    FILE *devnull = freopen("/dev/null", "w", stderr);
    (void)devnull;
    memset(&sa, 0, sizeof sa);
    sa.sa_handler = on_abort;
    sigaction(SIGABRT, &sa, NULL);
    for (unsigned i = 0; i < MXFP_N_new; i++) {
        const struct MxfpOp *on = &MXFP_OPS_new[i];
        const struct MxfpOp *oo;
        unsigned long long ab_new = 0, ab_old = 0, badop = 0;
        int need_unused;
        if (on->sid < 0 && vid_of(on->name, &need_unused) < 0) continue;
        oo = find(MXFP_OPS_old, MXFP_N_old, on->name);
        seed_for(on->name, seed);
        for (unsigned long long k = 0; k < cases; k++) {
            struct Case c;
            struct Res rn, ro;
            int an = 0, ao = 0;
            gen_case(on, &c);
            c.mx = (c.mx & ~0x1f80u) | ((uint32_t)rb(64) << 7); /* random masks */
            if (rb(3) == 0) c.mx &= ~0x3fu;
            if (rb(2)) c.mx &= ~0xE040u;
            if (sigsetjmp(jb, 1) == 0) run(on, &c, &rn); else { an = 1; }
            if (sigsetjmp(jb, 1) == 0) run(oo, &c, &ro); else { ao = 1; }
            ab_new += an;
            ab_old += ao;
            if (an != ao || (!an && !same(&rn, &ro))) {
                badop++;
                if (badop <= 3) printf("MISMATCH unmasked %s mx %#x abort new %d old %d\n", on->name, c.mx, an, ao);
            }
        }
        total += cases;
        aborts += ab_old;
        bad += badop;
        printf("OP unmasked %s cases %llu aborts_new %llu aborts_old %llu mismatches %llu\n", on->name, cases, ab_new, ab_old, badop);
    }
    printf("TOTAL unmasked cases %llu aborts %llu mismatches %llu\n", total, aborts, bad);
    return bad ? 1 : 0;
}

int main(int argc, char **argv) {
    if (argc >= 2 && !strcmp(argv[1], "list")) {
        for (unsigned i = 0; i < MXFP_N_new; i++) printf("%s sid %d lanes %d\n", MXFP_OPS_new[i].name, MXFP_OPS_new[i].sid, MXFP_OPS_new[i].lanes);
        return 0;
    }
    if (argc >= 6 && !strcmp(argv[1], "rand")) return cmd_rand(argv[2], strtoull(argv[3], 0, 0), strtoull(argv[4], 0, 0), atoi(argv[5]));
    if (argc >= 5 && !strcmp(argv[1], "rc")) return cmd_rc(argv[2], strtoull(argv[3], 0, 0), strtoull(argv[4], 0, 0), argc > 5 ? atoi(argv[5]) : 0);
    if (argc >= 5 && !strcmp(argv[1], "mxall")) return cmd_mxall(argv[2], strtoull(argv[3], 0, 0), strtoull(argv[4], 0, 0));
    if (argc >= 3 && !strcmp(argv[1], "replay128")) return cmd_replay128(argv[2], argc > 3 ? atoi(argv[3]) : 0);
    if (argc >= 3 && !strcmp(argv[1], "grid32")) return cmd_grid32(argv[2], argc > 3 ? (unsigned)atoi(argv[3]) : 1u);
    if (argc >= 3 && !strcmp(argv[1], "replay")) return cmd_replay(argv[2], argc > 3 ? atoi(argv[3]) : 0);
    if (argc >= 4 && !strcmp(argv[1], "unmasked")) return cmd_unmasked(strtoull(argv[2], 0, 0), strtoull(argv[3], 0, 0));
    fprintf(stderr, "usage: see the head of mxfp_diff.c\n");
    return 2;
}

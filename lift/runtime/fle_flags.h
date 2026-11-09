/* Status flags (CF, PF, AF, ZF, SF, OF) for fe-lift output.
 *
 * Eager model: each flag-setting operation writes all six flags. A flag that
 * the SDM leaves undefined for that operation is set to FLE_UNDEF. Reading an
 * undefined flag through fle_rd() or fle_cc() aborts, so a result never
 * depends on an undefined flag without the run stopping.
 */
#ifndef FLE_FLAGS_H
#define FLE_FLAGS_H

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_ai.h"

#define FLE_UNDEF 0xffu

typedef struct {
    uint8_t cf, pf, af, zf, sf, of;
} fle_eflags;

#ifdef FLE_LOWREGS
#include "fle_lowregs.h"
#else
extern fle_eflags FL;
#endif

static inline FLE_AI uint8_t fle_rd(uint8_t f) {
    if (f == FLE_UNDEF) {
        fprintf(stderr, "fle: read of an undefined flag\n");
        abort();
    }
    return f;
}

static inline FLE_AI uint64_t fle_mask(unsigned w) {
    return w >= 64 ? UINT64_MAX : (UINT64_C(1) << w) - 1;
}

static inline FLE_AI uint8_t fle_sb(unsigned w, uint64_t v) {
    return (uint8_t)((v >> (w - 1)) & 1);
}

static inline FLE_AI int64_t fle_sext(unsigned w, uint64_t v) {
    uint64_t m = fle_mask(w);
    v &= m;
    return (int64_t)(fle_sb(w, v) ? (v | ~m) : v);
}

static inline FLE_AI void fle_szp(unsigned w, uint64_t r) {
    uint8_t p;
    r &= fle_mask(w);
    p = (uint8_t)r;
    p ^= p >> 4;
    p ^= p >> 2;
    p ^= p >> 1;
    FL.pf = (uint8_t)((~p) & 1);
    FL.zf = r == 0;
    FL.sf = fle_sb(w, r);
}

static inline FLE_AI void fle_undef_all(void) {
    FL.cf = FL.pf = FL.af = FL.zf = FL.sf = FL.of = FLE_UNDEF;
}

/* Count above the operand width (8- and 16-bit shifts with a masked count of
 * 9..31) gives an undefined result in the SDM. Stop instead of making a value. */
static inline FLE_AI void fle_bad_count(void) {
    fprintf(stderr, "fle: shift count above operand width (undefined result)\n");
    abort();
}

static inline FLE_AI uint64_t fle_op_add(unsigned w, uint64_t a, uint64_t b, uint8_t cin) {
    uint64_t m = fle_mask(w);
    unsigned __int128 full;
    uint64_t r;
    a &= m;
    b &= m;
    full = (unsigned __int128)a + b + cin;
    r = (uint64_t)full & m;
    FL.cf = (uint8_t)((full >> w) & 1);
    FL.af = (uint8_t)(((a ^ b ^ r) >> 4) & 1);
    FL.of = fle_sb(w, (a ^ r) & (b ^ r));
    fle_szp(w, r);
    return r;
}

static inline FLE_AI uint64_t fle_op_sub(unsigned w, uint64_t a, uint64_t b, uint8_t bin) {
    uint64_t m = fle_mask(w);
    uint64_t r;
    a &= m;
    b &= m;
    r = (a - b - bin) & m;
    FL.cf = (unsigned __int128)a < (unsigned __int128)b + bin;
    FL.af = (uint8_t)(((a ^ b ^ r) >> 4) & 1);
    FL.of = fle_sb(w, (a ^ b) & (a ^ r));
    fle_szp(w, r);
    return r;
}

/* AND, OR, XOR, TEST: CF = OF = 0, AF undefined. */
static inline FLE_AI uint64_t fle_op_logic(unsigned w, uint64_t r) {
    r &= fle_mask(w);
    FL.cf = 0;
    FL.of = 0;
    FL.af = FLE_UNDEF;
    fle_szp(w, r);
    return r;
}

/* INC and DEC leave CF unchanged. */
static inline FLE_AI uint64_t fle_op_inc(unsigned w, uint64_t a) {
    uint8_t cf = FL.cf;
    uint64_t r = fle_op_add(w, a, 1, 0);
    FL.cf = cf;
    return r;
}

static inline FLE_AI uint64_t fle_op_dec(unsigned w, uint64_t a) {
    uint8_t cf = FL.cf;
    uint64_t r = fle_op_sub(w, a, 1, 0);
    FL.cf = cf;
    return r;
}

/* NEG: CF = (a != 0), other flags as 0 - a. */
static inline FLE_AI uint64_t fle_op_neg(unsigned w, uint64_t a) {
    return fle_op_sub(w, 0, a, 0);
}

/* NOT changes no flags. */
static inline FLE_AI uint64_t fle_op_not(unsigned w, uint64_t a) {
    return ~a & fle_mask(w);
}

/* The count is masked to 5 bits, or 6 bits for a 64-bit operand. */
static inline FLE_AI unsigned fle_cnt(unsigned w, uint64_t c) {
    return (unsigned)(c & (w == 64 ? 0x3f : 0x1f));
}

/* Count 0 changes no flags. */
static inline FLE_AI uint64_t fle_op_shl(unsigned w, uint64_t a, uint64_t cnt) {
    uint64_t m = fle_mask(w);
    unsigned c = fle_cnt(w, cnt);
    uint64_t r;
    uint8_t cf;
    a &= m;
    if (c == 0) {
        return a;
    }
    if (c > w) {
        fle_bad_count();
    }
    r = (uint64_t)((unsigned __int128)a << c) & m;
    cf = (uint8_t)((a >> (w - c)) & 1);
    FL.cf = cf;
    FL.of = c == 1 ? (uint8_t)(fle_sb(w, r) ^ cf) : FLE_UNDEF;
    FL.af = FLE_UNDEF;
    fle_szp(w, r);
    return r;
}

static inline FLE_AI uint64_t fle_op_shr(unsigned w, uint64_t a, uint64_t cnt) {
    uint64_t m = fle_mask(w);
    unsigned c = fle_cnt(w, cnt);
    uint64_t r;
    a &= m;
    if (c == 0) {
        return a;
    }
    if (c > w) {
        fle_bad_count();
    }
    r = a >> c;
    FL.cf = (uint8_t)((a >> (c - 1)) & 1);
    FL.of = c == 1 ? fle_sb(w, a) : FLE_UNDEF;
    FL.af = FLE_UNDEF;
    fle_szp(w, r);
    return r;
}

static inline FLE_AI uint64_t fle_op_sar(unsigned w, uint64_t a, uint64_t cnt) {
    uint64_t m = fle_mask(w);
    unsigned c = fle_cnt(w, cnt);
    int64_t s;
    uint64_t r;
    a &= m;
    if (c == 0) {
        return a;
    }
    if (c > w) {
        fle_bad_count();
    }
    s = fle_sext(w, a);
    r = (uint64_t)(s >> c) & m;
    FL.cf = (uint8_t)(((uint64_t)s >> (c - 1)) & 1);
    FL.of = c == 1 ? 0 : FLE_UNDEF;
    FL.af = FLE_UNDEF;
    fle_szp(w, r);
    return r;
}

/* IMUL with 2 or 3 operands: CF = OF = the truncated result differs from the
 * full signed product. The other flags are undefined. */
static inline FLE_AI uint64_t fle_op_imul(unsigned w, uint64_t a, uint64_t b) {
    uint64_t m = fle_mask(w);
    __int128 full = (__int128)fle_sext(w, a) * (__int128)fle_sext(w, b);
    uint64_t r = (uint64_t)full & m;
    uint8_t ov = full != (__int128)fle_sext(w, r);
    FL.cf = ov;
    FL.of = ov;
    FL.pf = FL.af = FL.zf = FL.sf = FLE_UNDEF;
    return r;
}

/* MUL (one operand, unsigned). Returns the high half and stores the low half
 * in *lo. CF = OF = (high half != 0). The other flags are undefined. */
static inline FLE_AI uint64_t fle_op_mul(unsigned w, uint64_t a, uint64_t b, uint64_t *lo) {
    uint64_t m = fle_mask(w);
    unsigned __int128 full = (unsigned __int128)(a & m) * (b & m);
    uint64_t hi = (uint64_t)(full >> w) & m;
    uint8_t ov = hi != 0;
    *lo = (uint64_t)full & m;
    FL.cf = ov;
    FL.of = ov;
    FL.pf = FL.af = FL.zf = FL.sf = FLE_UNDEF;
    return hi;
}

/* UCOMISS, UCOMISD, COMISS, COMISD. Unordered (a NaN): ZF = PF = CF = 1.
 * OF, SF, AF are 0. */
static inline FLE_AI void fle_set_comi(int unord, int lt, int eq) {
    FL.of = 0;
    FL.sf = 0;
    FL.af = 0;
    if (unord) {
        FL.zf = FL.pf = FL.cf = 1;
    } else {
        FL.pf = 0;
        FL.zf = eq != 0;
        FL.cf = lt != 0;
    }
}

static inline FLE_AI void fle_op_ucomiss(uint64_t a, uint64_t b) {
    uint32_t ua = (uint32_t)a, ub = (uint32_t)b;
    float fa, fb;
    memcpy(&fa, &ua, sizeof fa);
    memcpy(&fb, &ub, sizeof fb);
    fle_set_comi(fa != fa || fb != fb, fa < fb, fa == fb);
}

static inline FLE_AI void fle_op_ucomisd(uint64_t a, uint64_t b) {
    double fa, fb;
    memcpy(&fa, &a, sizeof fa);
    memcpy(&fb, &b, sizeof fb);
    fle_set_comi(fa != fa || fb != fb, fa < fb, fa == fb);
}

/* COMISD sets the same flags as UCOMISD. Only the exception rules differ. */
static inline FLE_AI void fle_op_comisd(uint64_t a, uint64_t b) {
    fle_op_ucomisd(a, b);
}

/* Jcc, SETcc, CMOVcc. CC is the low nibble of the opcode (tttn). Each case
 * reads every flag that its SDM formula names, so an undefined flag stops the
 * run even when a short-circuit would not need it. */
static inline FLE_AI int fle_cc(unsigned cc) {
    switch (cc) {
    case 0x0: return fle_rd(FL.of);
    case 0x1: return !fle_rd(FL.of);
    case 0x2: return fle_rd(FL.cf);
    case 0x3: return !fle_rd(FL.cf);
    case 0x4: return fle_rd(FL.zf);
    case 0x5: return !fle_rd(FL.zf);
    case 0x6: { int c = fle_rd(FL.cf); int z = fle_rd(FL.zf); return c || z; }
    case 0x7: { int c = fle_rd(FL.cf); int z = fle_rd(FL.zf); return !c && !z; }
    case 0x8: return fle_rd(FL.sf);
    case 0x9: return !fle_rd(FL.sf);
    case 0xa: return fle_rd(FL.pf);
    case 0xb: return !fle_rd(FL.pf);
    case 0xc: { int s = fle_rd(FL.sf); int o = fle_rd(FL.of); return s != o; }
    case 0xd: { int s = fle_rd(FL.sf); int o = fle_rd(FL.of); return s == o; }
    case 0xe: { int z = fle_rd(FL.zf); int s = fle_rd(FL.sf); int o = fle_rd(FL.of); return z || (s != o); }
    case 0xf: { int z = fle_rd(FL.zf); int s = fle_rd(FL.sf); int o = fle_rd(FL.of); return !z && (s == o); }
    default:
        fprintf(stderr, "fle: bad condition code\n");
        abort();
    }
}

#endif

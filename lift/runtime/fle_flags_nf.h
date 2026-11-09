/* Decision 0155 (provisional): the helpers of fle_flags.h that compute the result and leave FL alone. fe-lift --dead-flags uses them for an instruction whose
 * flag writes nothing reads (flag liveness over the closure). The value is the one of the helper of the same name (fle_op_add -> fle_nf_add, ...);
 * the stop on a count above the operand width is kept. */
#ifndef FLE_FLAGS_NF_H
#define FLE_FLAGS_NF_H

#include "fle_ai.h"
#include "fle_flags.h"

static inline FLE_AI uint64_t fle_nf_add(unsigned w, uint64_t a, uint64_t b, uint8_t cin) {
    return (a + b + cin) & fle_mask(w);
}

static inline FLE_AI uint64_t fle_nf_sub(unsigned w, uint64_t a, uint64_t b, uint8_t bin) {
    return (a - b - bin) & fle_mask(w);
}

static inline FLE_AI uint64_t fle_nf_logic(unsigned w, uint64_t r) {
    return r & fle_mask(w);
}

static inline FLE_AI uint64_t fle_nf_inc(unsigned w, uint64_t a) {
    return (a + 1) & fle_mask(w);
}

static inline FLE_AI uint64_t fle_nf_dec(unsigned w, uint64_t a) {
    return (a - 1) & fle_mask(w);
}

static inline FLE_AI uint64_t fle_nf_neg(unsigned w, uint64_t a) {
    return (0 - a) & fle_mask(w);
}

static inline FLE_AI uint64_t fle_nf_shl(unsigned w, uint64_t a, uint64_t cnt) {
    uint64_t m = fle_mask(w);
    unsigned c = fle_cnt(w, cnt);
    a &= m;
    if (c == 0) {
        return a;
    }
    if (c > w) {
        fle_bad_count();
    }
    return (uint64_t)((unsigned __int128)a << c) & m;
}

static inline FLE_AI uint64_t fle_nf_shr(unsigned w, uint64_t a, uint64_t cnt) {
    uint64_t m = fle_mask(w);
    unsigned c = fle_cnt(w, cnt);
    a &= m;
    if (c == 0) {
        return a;
    }
    if (c > w) {
        fle_bad_count();
    }
    return a >> c;
}

static inline FLE_AI uint64_t fle_nf_sar(unsigned w, uint64_t a, uint64_t cnt) {
    uint64_t m = fle_mask(w);
    unsigned c = fle_cnt(w, cnt);
    a &= m;
    if (c == 0) {
        return a;
    }
    if (c > w) {
        fle_bad_count();
    }
    return (uint64_t)(fle_sext(w, a) >> c) & m;
}

static inline FLE_AI uint64_t fle_nf_imul(unsigned w, uint64_t a, uint64_t b) {
    return (uint64_t)((__int128)fle_sext(w, a) * (__int128)fle_sext(w, b)) & fle_mask(w);
}

#endif

/* Signed divide and one-operand IMUL for fe-lift output. See fle_int_ext.h and
 * docs/decisions/0010-int-ext.md. */
#include "fle_runtime.h"
#include "fle_int_ext.h"

/* #DE: zero divisor, or a quotient outside the destination. Stop instead of making a value. */
static void fle_div_fault(void) {
    fprintf(stderr, "fle: divide error (#DE)\n");
    abort();
}

/* IDIV: quotient truncated toward zero, remainder has the sign of the dividend.
 * The flags are undefined. */
uint64_t fle_idiv(unsigned w, unsigned __int128 dvd, uint64_t src, uint64_t *rem) {
    unsigned sh = 128 - 2 * w;
    __int128 n = (__int128)(dvd << sh) >> sh;
    __int128 d = fle_sext(w, src);
    __int128 q, r, lim;

    if (d == 0) {
        fle_div_fault();
    }
    /* Most negative dividend / -1 does not fit the 128-bit type (for w = 64). Its quotient is out of range anyway. */
    if (d == -1 && dvd == ((unsigned __int128)1 << (2 * w - 1))) {
        fle_div_fault();
    }
    q = n / d;
    r = n % d;
    lim = (__int128)1 << (w - 1);
    if (q < -lim || q > lim - 1) {
        fle_div_fault();
    }
    fle_undef_all();
    *rem = (uint64_t)r & fle_mask(w);
    return (uint64_t)q & fle_mask(w);
}

/* One-operand IMUL: the flags follow the SDM (CF = OF = the high half is not the sign of the low half).
 * SF, ZF, AF, PF are undefined. */
uint64_t fle_imul1(unsigned w, uint64_t a, uint64_t b, uint64_t *lo) {
    uint64_t m = fle_mask(w);
    __int128 full = (__int128)fle_sext(w, a) * (__int128)fle_sext(w, b);
    uint64_t l = (uint64_t)full & m;
    uint64_t hi = (uint64_t)(full >> w) & m;
    uint8_t ov = full != (__int128)fle_sext(w, l);
    *lo = l;
    FL.cf = ov;
    FL.of = ov;
    FL.pf = FL.af = FL.zf = FL.sf = FLE_UNDEF;
    return hi;
}

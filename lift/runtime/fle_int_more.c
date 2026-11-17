/* Unsigned DIV and BT for fe-lift output. See fle_int_more.h and
 * docs/decisions/0013-more-insns.md. */
#include "fle_runtime.h"
#include "fle_int_more.h"

/* #DE: zero divisor, or a quotient outside the destination. Stop instead of making a value. */
static void fle_div_fault(void) {
    fprintf(stderr, "fle: divide error (#DE)\n");
    abort();
}

/* DIV: quotient truncated toward zero. The flags are undefined. */
uint64_t fle_div(unsigned w, unsigned __int128 dvd, uint64_t src, uint64_t *rem) {
    uint64_t m = fle_mask(w);
    uint64_t d = src & m;
    unsigned __int128 q, r;

    if (d == 0) {
        fle_div_fault();
    }
    q = dvd / d;
    r = dvd % d;
    if (q > (unsigned __int128)m) {
        fle_div_fault();
    }
    fle_undef_all();
    *rem = (uint64_t)r;
    return (uint64_t)q;
}

/* BT: CF is the selected bit. ZF is kept. The other four flags are undefined. */
void fle_bt_cf(uint8_t cf) {
    FL.cf = cf;
    FL.pf = FL.af = FL.sf = FL.of = FLE_UNDEF;
}

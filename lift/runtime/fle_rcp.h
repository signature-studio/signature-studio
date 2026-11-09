/* RCPPS for fe-lift output (decision 0047): portable integer C, no host FP unit and no asm.
 * The SDM fixes only |relative error| <= 1.5 * 2^-12 and the special cases, so the result is
 * implementation-specific. This reproduces the host (AMD Ryzen 9 9950X, Zen 5) bit for bit: the
 * result mantissa depends only on the top 12 bits of the source mantissa (table
 * fle_rcp_tab, runtime/fle_rcp_table.h), the exponent is 253 - e. Special cases (SDM RCPPS):
 * zero and denormal (treated as 0.0) give an infinity of the source sign; infinity gives 0 of the
 * source sign; a result that would be tiny (exponent field 253, 254) is flushed to 0 with the
 * source sign; an SNaN becomes a QNaN, a QNaN is returned. MXCSR RC, FTZ and DAZ have no effect, and
 * no exception flag is raised (host-measured over all 2^32 inputs; the SDM: SIMD exceptions "None").
 * Intel CPUs are unverified. Header only; included at the end of fle_sse_int2.h. */
#ifndef FLE_RCP_H
#define FLE_RCP_H

#include "fle_rcp_table.h"

static inline uint32_t fle_rcp_f32(uint32_t x) {
    uint32_t s = x & 0x80000000u, e = (x >> 23) & 0xffu, m = x & 0x7fffffu;
    if (e == 0xffu) return m ? (x | 0x400000u) : s;
    if (e == 0) return s | 0x7f800000u;
    if (e >= 253) return s;
    return s | ((253u - e) << 23) | ((uint32_t)fle_rcp_tab[m >> 11] << 11);
}

/* RCPPS xmm1, xmm2/m128: the destination's old value is not used. */
static inline fle_xmm fle_i2_rcpps(fle_xmm unused, fle_xmm b) {
    int k;
    (void)unused;
    for (k = 0; k < 4; k++) b.u32[k] = fle_rcp_f32(b.u32[k]);
    return b;
}

#endif

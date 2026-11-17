/* SSSE3 integer helpers for fe-lift output (sse_ssse3.rs, decision 0190, PROVISIONAL): PALIGNR, PABSB, PABSW, PABSD, PSHUFB (xmm forms). Header only (static inline), included at the end of fle_sse.h.
 * Semantics follow the 128-bit Legacy SSE pseudo-code of each instruction in Intel SDM Vol. 2. No flags are touched. */
#ifndef FLE_SSSE3_H
#define FLE_SSSE3_H

#include "fle_sse.h"

/* PALIGNR: the 32 bytes d:s (d = high half) shifted right by imm bytes; the low 16 bytes are kept; imm > 31 gives zero. */
static inline fle_xmm fle_palignr(fle_xmm d, fle_xmm s, unsigned imm) {
    uint8_t cat[32];
    uint8_t o[16];
    fle_xmm r;
    unsigned k;
    memcpy(cat, &s, 16);
    memcpy(cat + 16, &d, 16);
    for (k = 0; k < 16; k++) o[k] = (k + imm < 32u) ? cat[k + imm] : 0;
    memcpy(&r, o, 16);
    return r;
}

/* PABSB / PABSW / PABSD: |x| per lane; the most negative value is its own absolute value. */
static inline fle_xmm fle_pabsb(fle_xmm s) {
    uint8_t v[16];
    fle_xmm r;
    unsigned k;
    memcpy(v, &s, 16);
    for (k = 0; k < 16; k++) v[k] = (v[k] & 0x80u) ? (uint8_t)(0u - v[k]) : v[k];
    memcpy(&r, v, 16);
    return r;
}
static inline fle_xmm fle_pabsw(fle_xmm s) {
    uint16_t v[8];
    fle_xmm r;
    unsigned k;
    memcpy(v, &s, 16);
    for (k = 0; k < 8; k++) v[k] = (v[k] & 0x8000u) ? (uint16_t)(0u - v[k]) : v[k];
    memcpy(&r, v, 16);
    return r;
}
static inline fle_xmm fle_pabsd(fle_xmm s) {
    fle_xmm r;
    unsigned k;
    for (k = 0; k < 4; k++) r.u32[k] = (s.u32[k] & 0x80000000u) ? (0u - s.u32[k]) : s.u32[k];
    return r;
}

/* PSHUFB: result byte k = 0 if mask byte k has bit 7 set, else the byte of d selected by mask bits 3:0 (d is read before it is written). */
static inline fle_xmm fle_pshufb(fle_xmm d, fle_xmm m) {
    uint8_t db[16], mb[16], o[16];
    fle_xmm r;
    unsigned k;
    memcpy(db, &d, 16);
    memcpy(mb, &m, 16);
    for (k = 0; k < 16; k++) o[k] = (mb[k] & 0x80u) ? 0 : db[mb[k] & 15u];
    memcpy(&r, o, 16);
    return r;
}

#endif

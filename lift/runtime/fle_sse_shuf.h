/* Floating-point shuffle and half-move helpers for fe-lift output (sse_shuf.rs):
 * SHUFPS, SHUFPD, UNPCKLPS, UNPCKHPS, UNPCKLPD, UNPCKHPD, MOVHLPS, MOVLHPS.
 * Header only (static inline), included at the end of fle_sse.h. Semantics follow the
 * 128-bit Legacy SSE pseudo-code in Intel SDM Vol. 2 (Order Number 325383-093US); see
 * docs/decisions/0027-sse-shuf.md. The lanes are copied as bits: no floating-point
 * operation is done, so NaN payloads, signs and zeros are preserved. No flags are touched.
 * MOVLPS, MOVHPS, MOVLPD and MOVHPD are lifted inline (fle_ld / fle_st of 8 bytes). */
#ifndef FLE_SSE_SHUF_H
#define FLE_SSE_SHUF_H

#include "fle_sse.h"

/* SHUFPS: dword k of the result is a dword of a (k = 0, 1) or of b (k = 2, 3),
 * selected by imm[2k+1:2k]. */
static inline fle_xmm fle_shufps(fle_xmm a, fle_xmm b, unsigned imm) {
    fle_xmm r;
    r.u32[0] = a.u32[imm & 3];
    r.u32[1] = a.u32[(imm >> 2) & 3];
    r.u32[2] = b.u32[(imm >> 4) & 3];
    r.u32[3] = b.u32[(imm >> 6) & 3];
    return r;
}

/* SHUFPD: qword 0 is a.u64[imm[0]], qword 1 is b.u64[imm[1]]. imm[7:2] are ignored. */
static inline fle_xmm fle_shufpd(fle_xmm a, fle_xmm b, unsigned imm) {
    fle_xmm r;
    r.u64[0] = a.u64[imm & 1];
    r.u64[1] = b.u64[(imm >> 1) & 1];
    return r;
}

static inline fle_xmm fle_unpcklps(fle_xmm a, fle_xmm b) {
    fle_xmm r;
    r.u32[0] = a.u32[0];
    r.u32[1] = b.u32[0];
    r.u32[2] = a.u32[1];
    r.u32[3] = b.u32[1];
    return r;
}

static inline fle_xmm fle_unpckhps(fle_xmm a, fle_xmm b) {
    fle_xmm r;
    r.u32[0] = a.u32[2];
    r.u32[1] = b.u32[2];
    r.u32[2] = a.u32[3];
    r.u32[3] = b.u32[3];
    return r;
}

static inline fle_xmm fle_unpcklpd(fle_xmm a, fle_xmm b) {
    fle_xmm r;
    r.u64[0] = a.u64[0];
    r.u64[1] = b.u64[0];
    return r;
}

static inline fle_xmm fle_unpckhpd(fle_xmm a, fle_xmm b) {
    fle_xmm r;
    r.u64[0] = a.u64[1];
    r.u64[1] = b.u64[1];
    return r;
}

/* MOVHLPS: DEST[63:0] := SRC[127:64]; the high quadword of DEST is unchanged. */
static inline fle_xmm fle_movhlps(fle_xmm a, fle_xmm b) {
    a.u64[0] = b.u64[1];
    return a;
}

/* MOVLHPS: DEST[127:64] := SRC[63:0]; the low quadword of DEST is unchanged. */
static inline fle_xmm fle_movlhps(fle_xmm a, fle_xmm b) {
    a.u64[1] = b.u64[0];
    return a;
}


/* Decision 0191 (PROVISIONAL): the SSE3 moves MOVDDUP, MOVSLDUP, MOVSHDUP (Intel SDM Vol. 2, 128-bit Legacy SSE versions). No arithmetic, no flags; the lanes are copied as bits.
 *   MOVDDUP  xmm1, xmm2/m64 : DEST[63:0] := SRC[63:0]; DEST[127:64] := SRC[63:0]
 *   MOVSLDUP xmm1, xmm2/m128: DEST[31:0] := SRC[31:0]; DEST[63:32] := SRC[31:0]; DEST[95:64] := SRC[95:64]; DEST[127:96] := SRC[95:64]
 *   MOVSHDUP xmm1, xmm2/m128: DEST[31:0] := SRC[63:32]; DEST[63:32] := SRC[63:32]; DEST[95:64] := SRC[127:96]; DEST[127:96] := SRC[127:96] */
static inline fle_xmm fle_movddup_q(uint64_t q) {
    fle_xmm r;
    r.u64[0] = q;
    r.u64[1] = q;
    return r;
}
static inline fle_xmm fle_movddup(fle_xmm s) {
    return fle_movddup_q(s.u64[0]);
}
static inline fle_xmm fle_movsldup(fle_xmm s) {
    fle_xmm r;
    r.u32[0] = s.u32[0];
    r.u32[1] = s.u32[0];
    r.u32[2] = s.u32[2];
    r.u32[3] = s.u32[2];
    return r;
}
static inline fle_xmm fle_movshdup(fle_xmm s) {
    fle_xmm r;
    r.u32[0] = s.u32[1];
    r.u32[1] = s.u32[1];
    r.u32[2] = s.u32[3];
    r.u32[3] = s.u32[3];
    return r;
}

#endif

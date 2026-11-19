/* Packed SSE helpers for fe-lift output. See fle_sse_more.h and
 * docs/decisions/0013-more-insns.md. Each float lane uses the scalar helper of the same
 * operation in fle_sse.h, so NaN results match the scalar instructions. */
#include "fle_sse.h"
#include "fle_sse_more.h"

static uint8_t fle_satu8(int16_t v) {
    if (v < 0) {
        return 0;
    }
    if (v > 255) {
        return 255;
    }
    return (uint8_t)v;
}

/* PACKUSWB: the eight signed words of a become bytes 0..7 and those of b become
 * bytes 8..15, each saturated to 0..255. */
fle_xmm fle_packuswb(fle_xmm a, fle_xmm b) {
    fle_xmm r;
#ifdef FLE_WASM_SIMD
    return fle_xv(wasm_u8x16_narrow_i16x8(fle_v(a), fle_v(b)));
#endif
    int16_t x[8], y[8];
    uint8_t z[16];
    int k;
    memcpy(x, &a, sizeof x);
    memcpy(y, &b, sizeof y);
    for (k = 0; k < 8; k++) {
        z[k] = fle_satu8(x[k]);
        z[8 + k] = fle_satu8(y[k]);
    }
    memcpy(&r, z, sizeof r);
    return r;
}

/* PADDW: eight 16-bit lanes, wrapping. */
fle_xmm fle_paddw(fle_xmm a, fle_xmm b) {
#ifdef FLE_WASM_SIMD
    return fle_xv(wasm_i16x8_add(fle_v(a), fle_v(b)));
#endif
    uint16_t x[8], y[8];
    int k;
    memcpy(x, &a, sizeof x);
    memcpy(y, &b, sizeof y);
    for (k = 0; k < 8; k++) {
        x[k] = (uint16_t)(x[k] + y[k]);
    }
    memcpy(&a, x, sizeof a);
    return a;
}

fle_xmm fle_addps(fle_xmm a, fle_xmm b) {
    int k;
    for (k = 0; k < 4; k++) {
        a.u32[k] = fle_addss(a.u32[k], b.u32[k]);
    }
    return a;
}

fle_xmm fle_subps(fle_xmm a, fle_xmm b) {
    int k;
    for (k = 0; k < 4; k++) {
        a.u32[k] = fle_subss(a.u32[k], b.u32[k]);
    }
    return a;
}

fle_xmm fle_mulps(fle_xmm a, fle_xmm b) {
    int k;
    for (k = 0; k < 4; k++) {
        a.u32[k] = fle_mulss(a.u32[k], b.u32[k]);
    }
    return a;
}

/* MINPS and MAXPS: the first source is dst, the second is src (SDM pseudo-code MIN and MAX). */
fle_xmm fle_minps(fle_xmm a, fle_xmm b) {
    int k;
    for (k = 0; k < 4; k++) {
        a.u32[k] = fle_minss(a.u32[k], b.u32[k]);
    }
    return a;
}

fle_xmm fle_maxps(fle_xmm a, fle_xmm b) {
    int k;
    for (k = 0; k < 4; k++) {
        a.u32[k] = fle_maxss(a.u32[k], b.u32[k]);
    }
    return a;
}

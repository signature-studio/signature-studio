/* HLE of lifted guest functions (decision 0162, PROVISIONAL; the mechanism is the reference implementation of decision 0151). See fle_hle.h for the contract: the host implementation leaves the registers,
 * the flags and the memory exactly as the lifted body would, or does not handle the call (and then touches nothing).
 *
 * f_33d6e70 of dsp_ippv2 (rebased to 0x2000000): the bilinear affine warp (the inner loop of ippiWarpAffineQuad_8u_C4R). Read from the 229 lifted instructions:
 *   Win64 arguments: rcx = src, rdx = dst, r8d = src pitch (bytes, signed), r9d = dst pitch, [rsp+0x28] = first row y0, [+0x30] = last row y1, [+0x38] = table of (xstart, xend) int32 pairs per row,
 *   [+0x40] = six doubles c0..c5, [+0x48] = src width W, [+0x50] = src height H (S = rsp at entry, the return address at S).
 *   The frame: three pushes (r14 r15 rbp at S-8 .. S-24), sub rsp,0xb0; the callee-saved registers and xmm11..15 are spilled at F+0x20 .. F+0x90 (F = S-200); each row that has pixels also writes its row index (4 bytes),
 *   the dst pitch and the dst row pointer at F+0xa8 / F+0xa0 / F+0x98.
 *   Per row r = 0..y1-y0 (y1 < y0: nothing, a different path; this implementation declines it): sx0 = c1*y0 + c2, sy0 = y0*c4 + c5 (+ c1, + c4 after each row); a row with xs <= xe (signed) has xe-xs+1 pixels from xs:
 *   sx = c0*xs + sx0, sy = xs*c3 + sy0 (+ c0, + c3 after each pixel); ix = trunc(sx + 1e-7), iy = trunc(sy + 1e-7); fx = sx - ix, fy = sy - iy (1.0 and the last index W-1 / H-1 when ix >= W / iy >= H);
 *   p = src + (int32)(ix*4) + (int32)(iy*pitch), q = p + pitch; per channel c: top = p[c] + (p[4+c] - p[c])*fx, bottom = q[c] + (q[4+c] - q[c])*fx, out = trunc((0.5 + top) + (bottom - top)*fy), stored as a byte at dst + 4*xs + 4*k + c.
 *   Afterwards: rax = W-1, rcx = H-1, rdx = dst + nrows*dpitch, r9 = r10 = nrows, r11 = dpitch (sign-extended), xmm0 = c4, xmm1 = c1, xmm2 = c3, xmm3 = c0, xmm4 = sy0 and xmm5 = sx0 after the last row (the upper halves are zero),
 *   everything else as at entry (rbx rsi rdi rbp r12..r15 and xmm11..15 are restored from the frame), the flags those of the last instruction (add rsp,0xb0 with rsp = F).
 * The host computes the same sequence of IEEE double operations, in the same order, so the results are the same bits as long as the guest's MXCSR-tracking primitives would do exactly that: RC = 0, no operand or
 * result a denormal, no overflow, invalid or divide, no flag that is not already set. The plan proves that for the whole call before anything is touched (the numbers do not depend on the pixel data: the operands are
 * bytes converted to double, and the coordinates):
 *   - guest MXCSR: RC = 0, PE set and all exception masks set (nothing is raised that is not there already; FTZ and DAZ do not matter without tiny or denormal values); the host's own MXCSR: RC = 0, no FTZ, no DAZ;
 *   - the six coefficients are 0 or have 1e-30 <= |c| <= 1e30; the three constants at 0x440ef50 / 0x440ef58 / 0x440ef60 are 1.0, 0.5 and 1e-7;
 *   - every pixel's sx + 1e-7 and sy + 1e-7 lie in (-1, 1e9), by a bound of the recurrence with its rounding error: then the truncation is in range and the index is >= 0;
 *   - nothing the function writes (the frame below S, the destination rows) touches anything it reads after writing: the argument slots, the coefficients, the row table, the source image (all of it that the
 *     indices can reach), and the destination rows do not touch the frame;
 *   - 1 <= W, H < 2^20; rows < 2^16 (not the y1 < y0 path); each row at most 2^16 pixels; |pitch| * (H+1) < 2^31 (so the 32-bit product of the guest does not wrap).
 * Anything else: the lifted body runs. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include "fle_test_env.h"
#include <string.h>
#ifndef __EMSCRIPTEN__
#include <signal.h>
#endif
#if defined(__SSE__) && !defined(__EMSCRIPTEN__)
#include <xmmintrin.h>
#endif

#include "fle_hle.h"
#include "fle_realmod.h"
#include "fle_runtime.h"
#include "fle_sse.h"
#include "fle_sse_more.h"
#include "fle_mxcsr.h"

#if defined(__clang__)
#pragma clang fp contract(off)
#endif
#pragma STDC FP_CONTRACT OFF

#define WARP_BASE UINT64_C(0x2000000)
#define WARP_MODULE "dsp_ippv2_x64.dll"
#define WARP_C_ONE UINT64_C(0x440ef50)
#define WARP_C_HALF UINT64_C(0x440ef58)
#define WARP_C_EPS UINT64_C(0x440ef60)

static int g_init;
static int g_base_ok = -1;
static fle_hle_stats g_wst;
static int g_printed;

const fle_hle_stats *fle_hle_warp_bilin_stats(void) { return &g_wst; }

static void stats_line(void) {
    if (g_wst.calls)
        fprintf(stderr, "fle_hle: warp_bilin (f_33d6e70) calls=%llu handled=%llu fallback=%llu rows=%llu pixels=%llu\n",
                (unsigned long long)g_wst.calls, (unsigned long long)g_wst.handled, (unsigned long long)g_wst.fallback, (unsigned long long)g_wst.rows,
                (unsigned long long)g_wst.pixels);
}

static void print_stats(void) {
    if (g_printed) return;
    g_printed = 1;
    stats_line();
}

#ifndef __EMSCRIPTEN__
/* The native harness ends a run with abort() (fle_stop, the end of a G3 script) and has its own SIGABRT handler (installed before our first call): the statistics are
 * printed from a handler that is put in front of it at the first call of the guard, and then passes the signal on. */
static struct sigaction g_old_abort;

static void on_abort(int sig, siginfo_t *si, void *uc) {
    print_stats();
    if (g_old_abort.sa_flags & SA_SIGINFO) {
        if (g_old_abort.sa_sigaction) g_old_abort.sa_sigaction(sig, si, uc);
    } else if (g_old_abort.sa_handler != SIG_DFL && g_old_abort.sa_handler != SIG_IGN) {
        g_old_abort.sa_handler(sig);
    }
    signal(sig, SIG_DFL);
    raise(sig);
}

static void hook_abort(void) {
    struct sigaction sa;
    memset(&sa, 0, sizeof sa);
    sa.sa_sigaction = on_abort;
    sa.sa_flags = SA_SIGINFO;
    sigaction(SIGABRT, &sa, &g_old_abort);
}
#else
#define hook_abort() ((void)0)
#endif

/* The first call of a guard (decision 0243: HLE is on; the off and check modes and the statistics line every N calls were diagnostics of the environment). */
static void hle_init(void) {
    g_init = 1;
    atexit(print_stats);
    hook_abort();
}

/* The function is the one of dsp_ippv2_x64.dll at the base 0x2000000 only (the constants it reads are at that address): the link must say that module is there. */
static int base_ok(void) {
    uint64_t n, i;
    if (g_base_ok >= 0) return g_base_ok;
    g_base_ok = 0;
    if (!fle_realmod_total) return 0;
    n = fle_realmod_total();
    for (i = 0; i < n; i++) {
        const char *nm = fle_realmod_name((int)i);
        if (nm && !strcmp(nm, WARP_MODULE) && fle_realmod_base((int)i) == WARP_BASE) g_base_ok = 1;
    }
    return g_base_ok;
}

static inline uint32_t ld32(uint64_t a) { return (uint32_t)fle_ld(a, 4); }

static int addr_ok(uint64_t a, uint64_t n) {
    if (a + n < a) return 0;
#if defined(FLE_WASM32) || defined(FLE_CHECK_HI32)
    if ((a + n) >> 32) return 0;
#endif
    return 1;
}

static int overlap(uint64_t a, uint64_t an, uint64_t b, uint64_t bn) { return an && bn && a < b + bn && b < a + an; }

/* ---- f_33d6e70 ---- */

typedef struct {
    uint64_t S, src, dst, tbl, coef;
    int32_t sp, dp, y0, y1, W, H;
    uint32_t nrows;
    double c[6];
} warp_plan;

/* Appends the dst ranges of the call (used by the check mode): ranges[i] = {address, bytes}. */
typedef struct { uint64_t a, n; } hrange;

static int ok_coef(double d) {
    double a = d < 0 ? -d : d;
    return d == 0 || (a >= 1e-30 && a <= 1e30);
}

static int host_fp_ok(void) {
#if defined(__SSE__) && !defined(__EMSCRIPTEN__)
    return (_mm_getcsr() & 0xe040u) == 0; /* RC = 0 (bits 13-14), no FTZ (bit 15), no DAZ (bit 6) */
#else
    return 1;
#endif
}

/* Decides whether the host does this call, without touching anything. With `rg` it also lists the destination rows. */
static int warp_plan_make(warp_plan *p, hrange *rg, size_t *nrg, size_t cap) {
    uint64_t S = R[4], bits, F, hull_lo, hull_n;
    uint32_t r;
    double sx0, sy0;
    size_t nr = 0;
    int64_t spa, hull_rows;
    if (nrg) *nrg = 0;
    if (S < 256 || ((S - 200) & 15) || !addr_ok(S - 200, 200 + 0x60)) return 0;
    F = S - 200;
    if ((fle_mxcsr & 0x6000u) || (fle_mxcsr & 0x1fa0u) != 0x1fa0u) return 0;
    if (!host_fp_ok()) return 0;
    p->S = S;
    p->src = R[1];
    p->dst = R[2];
    p->sp = (int32_t)(uint32_t)R[8];
    p->dp = (int32_t)(uint32_t)R[9];
    p->y0 = (int32_t)ld32(S + 0x28);
    p->y1 = (int32_t)ld32(S + 0x30);
    p->tbl = fle_ld(S + 0x38, 8);
    p->coef = fle_ld(S + 0x40, 8);
    p->W = (int32_t)ld32(S + 0x48);
    p->H = (int32_t)ld32(S + 0x50);
    if ((int32_t)((uint32_t)p->y1 - (uint32_t)p->y0) < 0) return 0; /* the js path of the entry: left to the lifted body */
    p->nrows = (uint32_t)p->y1 - (uint32_t)p->y0 + 1u;
    if (p->nrows == 0 || p->nrows > (1u << 16) || p->W < 1 || p->H < 1 || p->W >= (1 << 20) || p->H >= (1 << 20)) return 0;
    if (!addr_ok(p->coef, 0x30) || !addr_ok(p->tbl, 8u * (uint64_t)p->nrows) || !addr_ok(p->src, 1)) return 0;
    if (overlap(p->coef, 0x30, F, 200) || overlap(p->tbl, 8u * (uint64_t)p->nrows, F, 200)) return 0;
    if (fle_ld(WARP_C_ONE, 8) != UINT64_C(0x3ff0000000000000) || fle_ld(WARP_C_HALF, 8) != UINT64_C(0x3fe0000000000000) || fle_ld(WARP_C_EPS, 8) != UINT64_C(0x3e7ad7f29abcaf48)) return 0;
    for (r = 0; r < 6; r++) {
        bits = fle_ld(p->coef + 8u * r, 8);
        memcpy(&p->c[r], &bits, 8);
        if (!ok_coef(p->c[r])) return 0;
    }
    /* the source bytes the indices can reach: rows 0..H (the row below the last index is read), bytes 0 .. 4W+3 of each, at the pitch (no 32-bit wrap of index*pitch) */
    spa = p->sp < 0 ? -(int64_t)p->sp : (int64_t)p->sp;
    hull_rows = (int64_t)p->H;
    if (spa * (hull_rows + 1) >= ((int64_t)1 << 31)) return 0;
    if (p->sp >= 0) {
        hull_lo = p->src;
        hull_n = (uint64_t)(spa * hull_rows) + 4u * (uint64_t)p->W + 4u;
    } else {
        if (p->src < (uint64_t)(spa * hull_rows)) return 0;
        hull_lo = p->src - (uint64_t)(spa * hull_rows);
        hull_n = (uint64_t)(spa * hull_rows) + 4u * (uint64_t)p->W + 4u;
    }
    if (!addr_ok(hull_lo, hull_n) || overlap(hull_lo, hull_n, F, 200)) return 0;
    sx0 = p->c[1] * (double)p->y0 + p->c[2];
    sy0 = (double)p->y0 * p->c[4] + p->c[5];
    for (r = 0; r < p->nrows; r++) {
        int32_t xs = (int32_t)ld32(p->tbl + 8u * r), xe = (int32_t)ld32(p->tbl + 8u * r + 4);
        if (xs <= xe) {
            int64_t n64 = (int64_t)xe - (int64_t)xs + 1;
            double sxs, sys, dn, errx, erry, lox, hix, loy, hiy, ex, ey;
            uint64_t d0;
            if (xs < -(1 << 20) || xe > (1 << 20) || n64 > (1 << 16)) return 0;
            dn = (double)n64;
            sxs = p->c[0] * (double)xs + sx0;
            sys = (double)xs * p->c[3] + sy0;
            ex = sxs + dn * p->c[0];
            ey = sys + dn * p->c[3];
            errx = (dn + 4) * 2.3e-16 * ((sxs < 0 ? -sxs : sxs) + dn * (p->c[0] < 0 ? -p->c[0] : p->c[0]) + 1.0);
            erry = (dn + 4) * 2.3e-16 * ((sys < 0 ? -sys : sys) + dn * (p->c[3] < 0 ? -p->c[3] : p->c[3]) + 1.0);
            lox = (sxs < ex ? sxs : ex) - errx;
            hix = (sxs > ex ? sxs : ex) + errx;
            loy = (sys < ey ? sys : ey) - erry;
            hiy = (sys > ey ? sys : ey) + erry;
            if (!(lox > -0.999 && hix < 1.0e9 && loy > -0.999 && hiy < 1.0e9)) return 0;
            d0 = p->dst + (uint64_t)((int64_t)r * (int64_t)p->dp) + (uint64_t)(int64_t)(int32_t)((uint32_t)xs * 4u);
            if (!addr_ok(d0, 4u * (uint64_t)n64)) return 0;
            if (overlap(d0, 4u * (uint64_t)n64, F, 200 + 0x60) || overlap(d0, 4u * (uint64_t)n64, p->coef, 0x30) || overlap(d0, 4u * (uint64_t)n64, p->tbl, 8u * (uint64_t)p->nrows) ||
                overlap(d0, 4u * (uint64_t)n64, hull_lo, hull_n))
                return 0;
            if (rg && nr < cap) { rg[nr].a = d0; rg[nr].n = 4u * (uint64_t)n64; nr++; }
            else if (rg) return 0;
        }
        sx0 = sx0 + p->c[1];
        sy0 = sy0 + p->c[4];
    }
    if (nrg) *nrg = nr;
    return 1;
}

/* The pixels. Returns the number of the last row that had pixels, or -1; adds the numbers of rows and pixels done to *pix. */
static int64_t warp_pixels(const warp_plan *p, uint64_t *pix) {
    const double c0 = p->c[0], c1 = p->c[1], c3 = p->c[3], c4 = p->c[4];
    const double eps = 1.0000000000000000e-7;
    const int32_t W = p->W, H = p->H;
    const int64_t sp64 = p->sp, dp64 = p->dp;
    const uint8_t *const src = (const uint8_t *)(uintptr_t)p->src;
    double sx0 = c1 * (double)p->y0 + p->c[2];
    double sy0 = (double)p->y0 * c4 + p->c[5];
    int64_t last = -1;
    uint32_t r;
    uint64_t npix = 0;
    for (r = 0; r < p->nrows; r++) {
        int32_t xs = (int32_t)ld32(p->tbl + 8u * r), xe = (int32_t)ld32(p->tbl + 8u * r + 4);
        if (xs <= xe) {
            uint32_t n = (uint32_t)xe - (uint32_t)xs + 1u, k;
            double sx = c0 * (double)xs + sx0, sy = (double)xs * c3 + sy0;
            uint8_t *d = (uint8_t *)(uintptr_t)(p->dst + (uint64_t)((int64_t)r * dp64) + (uint64_t)(int64_t)(int32_t)((uint32_t)xs * 4u));
            last = r;
            npix += n;
            for (k = 0; k < n; k++) {
                int32_t ix = (int32_t)(sx + eps), iy = (int32_t)(sy + eps);
                double fx, fy;
                const uint8_t *pp, *qq;
                int c;
                if (ix < W) fx = sx - (double)ix;
                else { fx = 1.0; ix = W - 1; }
                if (iy < H) fy = sy - (double)iy;
                else { fy = 1.0; iy = H - 1; }
                pp = src + (int64_t)(int32_t)((uint32_t)ix << 2) + (int64_t)(int32_t)((uint32_t)iy * (uint32_t)p->sp);
                qq = pp + sp64;
                for (c = 0; c < 4; c++) {
                    double a = (double)pp[c], b = (double)pp[4 + c], e = (double)qq[c], f = (double)qq[4 + c];
                    double top = a + (b - a) * fx;
                    double bot = e + (f - e) * fx;
                    double v = (0.5 + top) + (bot - top) * fy;
                    d[c] = (uint8_t)(int32_t)v;
                }
                d += 4;
                sx = sx + c0;
                sy = sy + c3;
            }
        }
        sx0 = sx0 + c1;
        sy0 = sy0 + c4;
    }
    *pix += npix;
    return last;
}

static void warp_exec(const warp_plan *p) {
    const uint64_t S = p->S, F = S - 200;
    const int64_t dp64 = p->dp;
    fle_xmm x4, x5;
    double sx0 = p->c[1] * (double)p->y0 + p->c[2], sy0 = (double)p->y0 * p->c[4] + p->c[5];
    int64_t last;
    uint32_t r;
    uint64_t pix = 0;

    fle_st(S - 8, 8, R[14]);
    fle_st(S - 16, 8, R[15]);
    fle_st(S - 24, 8, R[5]);
    fle_stxa(F + 0x20, X[11]);
    fle_stxa(F + 0x30, X[12]);
    fle_stxa(F + 0x40, X[13]);
    fle_stxa(F + 0x50, X[14]);
    fle_stxa(F + 0x60, X[15]);
    fle_st(F + 0x70, 8, R[3]);
    fle_st(F + 0x78, 8, R[6]);
    fle_st(F + 0x80, 8, R[7]);
    fle_st(F + 0x88, 8, R[12]);
    fle_st(F + 0x90, 8, R[13]);

    last = warp_pixels(p, &pix);
    g_wst.rows += p->nrows;
    g_wst.pixels += pix;
    if (last >= 0) {
        fle_st(F + 0xa8, 4, (uint64_t)(uint32_t)last);
        fle_st(F + 0xa0, 8, (uint64_t)dp64);
        fle_st(F + 0x98, 8, p->dst + (uint64_t)((int64_t)last * dp64));
    }

    for (r = 0; r < p->nrows; r++) {
        sx0 = sx0 + p->c[1];
        sy0 = sy0 + p->c[4];
    }
    memset(&x4, 0, sizeof x4);
    memset(&x5, 0, sizeof x5);
    x4.f64[0] = sy0;
    x5.f64[0] = sx0;
    R[0] = (uint32_t)(p->W - 1);
    R[1] = (uint32_t)(p->H - 1);
    R[2] = p->dst + (uint64_t)((int64_t)p->nrows * dp64);
    R[9] = p->nrows;
    R[10] = p->nrows;
    R[11] = (uint64_t)dp64;
    memset(&X[0], 0, sizeof X[0]);
    memset(&X[1], 0, sizeof X[1]);
    memset(&X[2], 0, sizeof X[2]);
    memset(&X[3], 0, sizeof X[3]);
    X[0].f64[0] = p->c[4];
    X[1].f64[0] = p->c[1];
    X[2].f64[0] = p->c[3];
    X[3].f64[0] = p->c[0];
    X[4] = x4;
    X[5] = x5;
    (void)fle_op_add(64, F, 0xb0, 0);
}

int fle_hle_warp_bilin_run(void) {
    warp_plan p;
    if (!base_ok() || !warp_plan_make(&p, NULL, NULL, 0)) return 0;
    warp_exec(&p);
    return 1;
}

/* The guard of the generated code: 1 when the host did everything the function does. */
int fle_hle_warp_bilin(void) {
    if (!g_init) hle_init();
    g_wst.calls++;
    if (fle_hle_warp_bilin_run()) { g_wst.handled++; return 1; }
    g_wst.fallback++;
    return 0;
}

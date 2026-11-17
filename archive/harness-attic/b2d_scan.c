/* X2 scan of the Agg2D entries of the guest's Blend2D (see b2d_scan.h and docs/investigations/blend2d-nojit-slots.md). Harness only, portable
 * (native and wasm: no signal, no alarm, no backtrace). Linked only by a build with B2D_SCAN=1.
 *
 * One test per process: AggCreateInstance, attach(buffer, 64, 64, stride 256), the calls of the test, the "scan:" lines. Every call of a slot first
 * prints "scan: CALL <slot>", so after a stop the last such line names the slot that was running. The arguments of each slot are READINGS of the
 * disassembly of its wrapper (docs/investigations/blend2d-nojit-slots.md section 3); a CHECK says in its text whether the expected value comes from
 * a SPEC (a rule that does not depend on the wrapper: source-over, a rectangle, a Bezier curve, a clip box) or from a READING (what the wrapper
 * seems to do). A test without CHECK lines reports the pixel hash only. */
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifndef __EMSCRIPTEN__
#include <unistd.h>
#endif

#include "a4.h"
#include "boot.h"
#include "fle_callback.h"
#include "fle_dispatch.h"
#include "fle_env.h"
#include "fle_realmod.h"
#include "fle_runtime.h"
#include "fle_sse.h"
#include "b2d_scan.h"

#define ULL(x) ((unsigned long long)(x))
#define W 64
#define H 64
#define STRIDE (W * 4)
#define NBYTES (W * H * 4)
#define B2D_NAME "blend2d.dll"
#define RVA_AGGVT 0x1dbec0u
#define RVA_FIXED_PROVIDER 0x1f30e0u
#define BG_B 0x33
#define BG_G 0x22
#define BG_R 0x11

static const char *cfg_test;
static char cfg_arg[64];
static char cfg_full[64];
static int cfg_list;
static int targ = -1;

static uint8_t *pix;
static uint8_t bg_copy[NBYTES];
static uint64_t obj;
static double dbuf[32] __attribute__((aligned(16)));

int b2d_scan_requested(void) { return cfg_test != NULL; }

static uint64_t d2u(double d) {
    uint64_t u;
    memcpy(&u, &d, sizeof u);
    return u;
}
#define D(x) d2u(x)
static double u2d(uint64_t u) {
    double d;
    memcpy(&d, &u, sizeof d);
    return d;
}
static double ret_d(void) { return u2d(X[0].u64[0]); }

static uint64_t gcall(uint64_t va, uint64_t a1, uint64_t a2, uint64_t a3, uint64_t a4, int nstk, const uint64_t *stk) {
    uint64_t call_sp = (R[4] - FLE_CB_SHADOW_BYTES) & ~UINT64_C(15);
    int i;
    for (i = 0; i < nstk; i++) fle_st(call_sp + FLE_CB_SHADOW_BYTES + 8u * (unsigned)i, 8, stk[i]);
    return fle_call_guest(va, a1, a2, a3, a4);
}
static uint64_t slotfn(unsigned slot) { return fle_ld(fle_ld(obj, 8) + 8u * slot, 8); }

/* One call of a slot. The positions 1..3 after `this` are given once as raw words and put in rdx/r8/r9 and in xmm1/xmm2/xmm3 (the callee reads the
 * ones its signature uses: an integer in the register, a double in the xmm register); the words after them go on the stack in order. */
static uint64_t fnv(const uint8_t *p, size_t n);
static int ret_on = -1;
static uint64_t cp(unsigned slot, uint64_t a, uint64_t b, uint64_t c, int ns, const uint64_t *s) {
    uint64_t rv;
    printf("scan: CALL %u\n", slot);
    fflush(stdout);
    X[1].u64[0] = a;
    X[2].u64[0] = b;
    X[3].u64[0] = c;
    X[1].u64[1] = X[2].u64[1] = X[3].u64[1] = 0;
    rv = gcall(slotfn(slot), obj, a, b, c, ns, s);
    /* Decision 0160: FLE_B2D1_RET=1 prints, after every call, the low byte of the returned word (only slot 126, the hit test: for a void slot rax is whatever the callee left) and a hash of the context's BLContextState as the wrapper sees it (impl+8 -> state, 0x120
     * bytes; the pointer fields targetImage [0,8) and dashArray [0x60,0x70) are left out). The same line in the unswapped build is the DLL's own state. */
    if (ret_on < 0) ret_on = getenv("FLE_B2D1_RET") != NULL;
    if (ret_on) {
        uint64_t impl = fle_ld(obj + 0x18, 8), st = impl ? fle_ld(impl + 8, 8) : 0;
        uint8_t buf[0x120 - 8 - 16];
        if (st) {
            unsigned i, o = 0;
            for (i = 8; i < 0x120; i++) {
                if (i >= 0x60 && i < 0x70) continue;
                buf[o++] = (uint8_t)fle_ld(st + i, 1);
            }
            printf("scan: VALUE %s ret slot%u %#llx state %#llx\n", cfg_full, slot, slot == 126 ? ULL(rv & 0xff) : 0ull, ULL(fnv(buf, o)));
        } else {
            printf("scan: VALUE %s ret slot%u %#llx state none\n", cfg_full, slot, slot == 126 ? ULL(rv & 0xff) : 0ull);
        }
    }
    return rv;
}
static uint64_t c0(unsigned slot) { return cp(slot, 0, 0, 0, 0, NULL); }
static uint64_t c1(unsigned slot, uint64_t a) { return cp(slot, a, 0, 0, 0, NULL); }
static uint64_t c2(unsigned slot, uint64_t a, uint64_t b) { return cp(slot, a, b, 0, 0, NULL); }
static uint64_t c3(unsigned slot, uint64_t a, uint64_t b, uint64_t c) { return cp(slot, a, b, c, 0, NULL); }
static uint64_t c4(unsigned slot, uint64_t a, uint64_t b, uint64_t c, uint64_t d) {
    uint64_t s[1] = {d};
    return cp(slot, a, b, c, 1, s);
}
static uint64_t c5(unsigned slot, uint64_t a, uint64_t b, uint64_t c, uint64_t d, uint64_t e) {
    uint64_t s[2] = {d, e};
    return cp(slot, a, b, c, 2, s);
}
static uint64_t c6(unsigned slot, uint64_t a, uint64_t b, uint64_t c, uint64_t d, uint64_t e, uint64_t f) {
    uint64_t s[3] = {d, e, f};
    return cp(slot, a, b, c, 3, s);
}
static uint64_t c7(unsigned slot, uint64_t a, uint64_t b, uint64_t c, uint64_t d, uint64_t e, uint64_t f, uint64_t g) {
    uint64_t s[4] = {d, e, f, g};
    return cp(slot, a, b, c, 4, s);
}
static uint64_t c8(unsigned slot, uint64_t a, uint64_t b, uint64_t c, uint64_t d, uint64_t e, uint64_t f, uint64_t g, uint64_t h) {
    uint64_t s[5] = {d, e, f, g, h};
    return cp(slot, a, b, c, 5, s);
}
/* colour calls: (r, g, b, a) bytes in rdx, r8, r9 and the 5th argument */
static void colour(unsigned slot, unsigned r, unsigned g, unsigned b, unsigned a) { c4(slot, r, g, b, a); }

/* ---- the pixel buffer ---- */
static uint64_t fnv(const uint8_t *p, size_t n) {
    uint64_t h = UINT64_C(0xcbf29ce484222325);
    size_t i;
    for (i = 0; i < n; i++) h = (h ^ p[i]) * UINT64_C(0x100000001b3);
    return h;
}
static const uint8_t *px(unsigned x, unsigned y) { return pix + (size_t)y * STRIDE + (size_t)x * 4; }
static void fill_bg(void) {
    unsigned i;
    for (i = 0; i < W * H; i++) {
        pix[4 * i + 0] = BG_B;
        pix[4 * i + 1] = BG_G;
        pix[4 * i + 2] = BG_R;
        pix[4 * i + 3] = 0xff;
    }
}
static unsigned changed(void) {
    unsigned n = 0, i;
    for (i = 0; i < W * H; i++)
        if (memcmp(pix + 4u * i, bg_copy + 4u * i, 4) != 0) n++;
    return n;
}
static unsigned count_px(unsigned b, unsigned g, unsigned r, unsigned a) {
    unsigned n = 0, i;
    for (i = 0; i < W * H; i++) n += pix[4 * i] == b && pix[4 * i + 1] == g && pix[4 * i + 2] == r && pix[4 * i + 3] == a;
    return n;
}

static void check(const char *label, int ok, const char *how, const char *fmt, ...) __attribute__((format(printf, 4, 5)));
static void check(const char *label, int ok, const char *how, const char *fmt, ...) {
    char d[240];
    va_list ap;
    va_start(ap, fmt);
    vsnprintf(d, sizeof d, fmt, ap);
    va_end(ap);
    printf("scan: CHECK %s %s %s [%s] %s\n", cfg_full, label, ok ? "PASS" : "FAIL", how, d);
}
static void check_px(const char *label, unsigned x, unsigned y, unsigned b, unsigned g, unsigned r, unsigned a, const char *how) {
    const uint8_t *p = px(x, y);
    int ok = p[0] == b && p[1] == g && p[2] == r && p[3] == a;
    check(label, ok, how, "pixel(%u,%u) is %02x %02x %02x %02x (B G R A), expected %02x %02x %02x %02x", x, y, p[0], p[1], p[2], p[3], b, g, r, a);
}
static void check_bg(const char *label, unsigned x, unsigned y, const char *how) { check_px(label, x, y, BG_B, BG_G, BG_R, 0xff, how); }
static void value(const char *label, long long v) { printf("scan: VALUE %s %s %lld\n", cfg_full, label, v); }
static void dvalue(const char *label, double v) { printf("scan: VALUE %s %s %.9g\n", cfg_full, label, v); }

/* ---- common set-ups ---- */
static void clear_bg(void) { colour(8, BG_R, BG_G, BG_B, 0xff); }
static void fill_noline(unsigned r, unsigned g, unsigned b, unsigned a) {
    colour(27, r, g, b, a);
    c0(31);
}
static void rect(double x1, double y1, double x2, double y2) { c4(63, D(x1), D(y1), D(x2), D(y2)); }
static void tri_path(void) { /* (10,50) (50,50) (30,10) */
    c0(82);
    c2(83, D(10), D(50));
    c2(85, D(50), D(50));
    c2(85, D(30), D(10));
}
static void check_tri(const char *how) {
    check_px("tri-inside", 30, 40, 0xff, 0xff, 0x00, 0xff, how);
    check_bg("tri-outside", 5, 5, how);
}
static int is_px(unsigned x, unsigned y, unsigned b, unsigned g, unsigned r, unsigned a) {
    const uint8_t *p = px(x, y);
    return p[0] == b && p[1] == g && p[2] == r && p[3] == a;
}
/* area of a shape drawn in one colour: the sum over the pixels of the coverage seen in channel `ch` (0 B, 1 G, 2 R) between the background value and the colour value */
static double area(int ch, int from, int to) {
    double s = 0;
    unsigned i;
    for (i = 0; i < W * H; i++) s += ((int)pix[4 * i + ch] - from) / (double)(to - from);
    return s;
}
static int max_in(unsigned x, unsigned y, int ch) { /* the largest value of channel ch in the 3x3 neighbourhood of (x,y) */
    int m = 0;
    unsigned i, j;
    for (j = y - 1; j <= y + 1; j++)
        for (i = x - 1; i <= x + 1; i++)
            if (px(i, j)[ch] > m) m = px(i, j)[ch];
    return m;
}
static void hvalue(const char *label, uint64_t h) { printf("scan: VALUE %s %s %#llx\n", cfg_full, label, ULL(h)); }
#define P(a) ((uint64_t)(uintptr_t)(a))
#define PI 3.14159265358979323846

typedef struct {
    const char *name;
    const char *slots;
    void (*fn)(void);
} test_info;
#define TEST(n) static void t_##n(void)
#include "b2d_scan_tests.h"
#include "b2d_scan_tests2.h"
#include "b2d1/b2d1_stress_test.h" /* decision 0160 */
#include "b2d1/b3_scan_test.h" /* decision 0195 */

/* ---- the table ---- */
static const test_info tests[] = {
#define T(n, s) {#n, s, t_##n},
#include "b2d_scan_table.h"
#undef T
    {NULL, NULL, NULL}};

int b2d_scan_precheck(void) {
    const char *e = getenv("FLE_B2D_SCAN");
    char *c;
    if (!e || !*e) return 0;
    if (strcmp(e, "list") == 0) {
        const test_info *t;
        for (t = tests; t->name; t++) printf("scan: TEST %-12s slots %s\n", t->name, t->slots);
        cfg_list = 1;
        return 1;
    }
    if (strlen(e) >= sizeof cfg_arg) {
        fprintf(stderr, "scan: FLE_B2D_SCAN is too long\n");
        return -1;
    }
    snprintf(cfg_arg, sizeof cfg_arg, "%s", e);
    snprintf(cfg_full, sizeof cfg_full, "%s", e);
    c = strchr(cfg_arg, ':');
    if (c) {
        *c = 0;
        targ = atoi(c + 1);
    }
    cfg_test = cfg_arg;
    return 0;
}

int b2d_scan_run(uint64_t entry_sp) {
    int idx;
    uint64_t base, pe, dd, dir, nn, afunc, anames, aord, i, agg = 0;
    const test_info *ti;
    if (!cfg_test) return 0;
    for (ti = tests; ti->name; ti++)
        if (strcmp(ti->name, cfg_test) == 0) break;
    if (!ti->name) {
        printf("scan: unknown test \"%s\" (FLE_B2D_SCAN=list)\n", cfg_test);
        return 2;
    }
    idx = fle_realmod_find(B2D_NAME);
    if (idx < 0) {
        printf("scan: no real module \"%s\"\n", B2D_NAME);
        return 2;
    }
    base = fle_realmod_base_of(idx);
    pe = base + fle_ld(base + 0x3c, 4);
    dd = pe + 24 + 112;
    dir = base + fle_ld(dd, 4);
    nn = fle_ld(dir + 24, 4);
    afunc = fle_ld(dir + 28, 4);
    anames = fle_ld(dir + 32, 4);
    aord = fle_ld(dir + 36, 4);
    for (i = 0; i < nn; i++) {
        uint64_t nrva = fle_ld(base + anames + 4 * i, 4);
        if (strncmp((const char *)(uintptr_t)(base + nrva), "AggCreateInstance", 18) == 0) agg = base + fle_ld(base + afunc + 4 * fle_ld(base + aord + 2 * i, 2), 4);
    }
    if (!agg) {
        printf("scan: AggCreateInstance not found\n");
        return 2;
    }
    R[4] = entry_sp;
    pix = aligned_alloc(64, NBYTES);
    if (!pix) return 2;
    fill_bg();
    memcpy(bg_copy, pix, NBYTES);
    printf("scan: TESTNAME %s\n", cfg_full);
    obj = gcall(agg, 0, 0, 0, 0, 0, NULL);
    if (!obj || fle_ld(obj, 8) != base + RVA_AGGVT) {
        printf("scan: RESULT %s status=no-object\n", cfg_full);
        return 3;
    }
    printf("scan: CALL 1\n");
    {
        uint64_t s[1] = {STRIDE};
        gcall(slotfn(1), obj, (uint64_t)(uintptr_t)pix, W, H, 1, s);
    }
#ifdef B2D1_SWAP
    printf("scan: attached %dx%d, context swapped (decision 0160): pipeline provider not read\n", W, H);
#else
    printf("scan: attached %dx%d, pipeline provider %s\n", W, H, fle_ld(fle_ld(obj + 0x18, 8) + 0x608, 8) == base + RVA_FIXED_PROVIDER ? "fixed (no JIT)" : "NOT the fixed provider");
#endif
    fflush(stdout);
    ti->fn();
    printf("scan: RESULT %s status=done hash=%#llx changed=%u\n", cfg_full, ULL(fnv(pix, NBYTES)), changed());
    fflush(stdout);
    return 0;
}

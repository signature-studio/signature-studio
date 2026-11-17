/* Decision 0195 (opt-in test tool): a deterministic scene generator for the Agg2D entries of Blend2D.dll, shared by two drivers that execute the SAME call sequence:
 *   - lift/harness/b2d1/b3_wine.c      a Windows program (mingw) that calls the REAL Blend2D.dll through AggCreateInstance (run under Wine);
 *   - lift/harness/b2d1/b3_scan_test.h the Agg2D scan harness (FLE_B2D_SCAN=b3ov), which calls the lifted wrapper (+ the adapter + upstream Blend2D) through the guest.
 * One scene = a random 64x64 PRGB32 target (valid premultiplied pixels), blendMode(op), no line, a solid or gradient fill, one rectangle or one path (fill only). The result is a hash of the
 * 64x64 buffer. Portable C (no platform calls here); the driver supplies call/call_ret/poke16 and the buffer. */
#ifndef B3SCENE_H
#define B3SCENE_H
#include <stdint.h>
#include <string.h>
#define B3W 64
#define B3H 64
typedef struct B3Env {
    void (*call)(struct B3Env *, unsigned slot, int nwords, const uint64_t *w);              /* nwords raw words after `this` (the first three in rdx/r8/r9 and xmm1..3, the rest on the stack) */
    uint64_t (*call_ret)(struct B3Env *, unsigned slot, int nwords, const uint64_t *w);       /* the same, returns rax */
    void (*poke16)(struct B3Env *, uint64_t ptr, unsigned byteoff, uint16_t v);
    uint8_t *pix;                                                                             /* the attached 64x64x4 buffer (host-visible) */
    void *drv;
    void (*mark)(struct B3Env *, const char *tag);   /* optional debug hook (may be NULL) */
} B3Env;
typedef struct { uint64_t s; } B3Rng;
static uint64_t b3_next(B3Rng *r) { uint64_t z = (r->s += 0x9e3779b97f4a7c15ULL); z = (z ^ (z >> 30)) * 0xbf58476d1ce4e5b9ULL; z = (z ^ (z >> 27)) * 0x94d049bb133111ebULL; return z ^ (z >> 31); }
static uint32_t b3_u(B3Rng *r, uint32_t n) { return (uint32_t)(b3_next(r) % n); }
static double b3_f(B3Rng *r, double a, double b) { return a + (b - a) * ((double)(b3_next(r) >> 11) * (1.0 / 9007199254740992.0)); }
static uint64_t b3_D(double d) { uint64_t u; memcpy(&u, &d, 8); return u; }
static uint64_t b3_fnv(const uint8_t *p, size_t n) { uint64_t h = 0xcbf29ce484222325ULL; size_t i; for (i = 0; i < n; i++) h = (h ^ p[i]) * 0x100000001b3ULL; return h; }
static void b3_c(B3Env *e, unsigned slot, int n, uint64_t a, uint64_t b, uint64_t c, uint64_t d, uint64_t f, uint64_t g) { uint64_t w[6]; w[0] = a; w[1] = b; w[2] = c; w[3] = d; w[4] = f; w[5] = g; e->call(e, slot, n, w); }

/* n random doubles in [lo,hi), drawn in order (C leaves the evaluation order of call arguments open, and the two drivers are built by different compilers) */
static void b3_cr(B3Env *e, unsigned slot, int n, B3Rng *r, double lo, double hi) { uint64_t w[8]; int i; for (i = 0; i < n; i++) w[i] = b3_D(b3_f(r, lo, hi)); e->call(e, slot, n, w); }

/* tuple: op = Agg2D blendMode value (3 SRC_OVER, 12 PLUS, 16 OVERLAY, ...), style 0 solid, 1 linear gradient, 2 radial gradient; geom 0 rect, 1 polygon, 2 path with curves (both: fill only, drawPath flag 0),
 * 3 polygon, 4 path with curves (drawPath flag 4 = the path drawn as a line of lineWidth in the FILL colour/gradient: the form of the engine's one OVERLAY call, decision 0159);
 * ramp 0 = smooth ramp from 2..4 random stops, 1 = every entry random (as the fuzz of the scan does); opaque 1 = solid colour / ramp alpha forced to full */
static void b3_scene(B3Env *e, unsigned op, unsigned style, unsigned geom, unsigned ramp_noise, unsigned opaque, uint64_t seed, long idx) {
    B3Rng r; unsigned i, k, mode;
    r.s = seed * 0x100000001b3ULL + (uint64_t)idx * 0x9e3779b97f4a7c15ULL + 0x5bd1e995ULL;
    mode = b3_u(&r, 4);
    for (i = 0; i < B3W * B3H; i++) {   /* B G R A, premultiplied */
        uint32_t a = mode == 0 ? 255 : mode == 1 ? b3_u(&r, 256) : mode == 2 ? (b3_u(&r, 2) ? 0 : b3_u(&r, 256)) : (b3_u(&r, 10) < 3 ? 255 : b3_u(&r, 256));
        uint32_t rr = b3_u(&r, a + 1), g = b3_u(&r, a + 1), b = b3_u(&r, a + 1);
        if (!a) rr = g = b = 0;
        e->pix[4 * i + 0] = (uint8_t)b; e->pix[4 * i + 1] = (uint8_t)g; e->pix[4 * i + 2] = (uint8_t)rr; e->pix[4 * i + 3] = (uint8_t)a;
    }
    if (e->mark) e->mark(e, "pre");
    b3_c(e, 15, 1, op, 0, 0, 0, 0, 0);       /* blendMode(n) */
    b3_c(e, 31, 0, 0, 0, 0, 0, 0, 0);        /* noLine */
    if (style == 0) {
        uint32_t a = opaque ? 255 : (b3_u(&r, 3) == 0 ? 255 : b3_u(&r, 256));
        { uint64_t cr = b3_u(&r, 256), cg = b3_u(&r, 256), cb = b3_u(&r, 256); b3_c(e, 27, 4, cr, cg, cb, a, 0, 0); }   /* fillColor(r,g,b,a) */
    } else {
        uint64_t rp = e->call_ret(e, 120, 1, (uint64_t[]){1});      /* fill colour ramp: 256 x (4 x int16) */
        int ns = 2 + (int)b3_u(&r, 3); double off[4]; uint32_t col[4][4];
        off[0] = 0; off[ns - 1] = 1; for (k = 1; (int)k < ns - 1; k++) off[k] = b3_f(&r, 0.05, 0.95);
        for (k = 1; (int)k < (unsigned)ns - 1; k++) { unsigned j; for (j = k; j > 1 && off[j] < off[j - 1]; j--) { double t = off[j]; off[j] = off[j - 1]; off[j - 1] = t; } }
        for (k = 0; (int)k < ns; k++) { col[k][0] = b3_u(&r, 0x8000); col[k][1] = b3_u(&r, 0x8000); col[k][2] = b3_u(&r, 0x8000); col[k][3] = opaque ? 0x7fff : (b3_u(&r, 3) == 0 ? 0x7fff : 0x2000 + b3_u(&r, 0x6000)); }
        for (i = 0; i < 256; i++) {
            uint16_t v[4];
            if (ramp_noise) { v[0] = (uint16_t)b3_u(&r, 0x8000); v[1] = (uint16_t)b3_u(&r, 0x8000); v[2] = (uint16_t)b3_u(&r, 0x8000); v[3] = (uint16_t)(opaque ? 0x7fff : 0x4000 + b3_u(&r, 0x3fff)); }
            else {
                double t = i / 255.0; int s0 = 0; unsigned c;
                while (s0 + 2 < ns && t > off[s0 + 1]) s0++;
                { double u = off[s0 + 1] > off[s0] ? (t - off[s0]) / (off[s0 + 1] - off[s0]) : 0; if (u < 0) u = 0; if (u > 1) u = 1;
                  for (c = 0; c < 4; c++) v[c] = (uint16_t)(col[s0][c] + (col[s0 + 1][c] - (double)col[s0][c]) * u + 0.5); }
            }
            for (k = 0; k < 4; k++) e->poke16(e, rp, 8 * i + 2 * k, v[k]);
        }
        if (style == 1) { uint64_t w[5]; for (k = 0; k < 4; k++) w[k] = b3_D(b3_f(&r, -8, 72)); w[4] = 1; e->call(e, 121, 5, w); }   /* linearGradient(x1,y1,x2,y2,fill) */
        else { uint64_t w[4]; w[0] = b3_D(b3_f(&r, 0, 64)); w[1] = b3_D(b3_f(&r, 0, 64)); w[2] = b3_D(b3_f(&r, 4, 60)); w[3] = 1; e->call(e, 122, 4, w); }   /* radialGradient(x,y,r,fill) */
    }
    if (geom == 0) {
        double x1 = b3_f(&r, -4, 60), y1 = b3_f(&r, -4, 60), dx, dy;
        if (b3_u(&r, 3) == 0) { x1 = (int)x1; y1 = (int)y1; dx = (int)b3_f(&r, 1, 40); dy = (int)b3_f(&r, 1, 40); }
        else { dx = b3_f(&r, 0.5, 40); dy = b3_f(&r, 0.5, 40); }
        b3_c(e, 63, 4, b3_D(x1), b3_D(y1), b3_D(x1 + dx), b3_D(y1 + dy), 0, 0);   /* rectangle(x1,y1,x2,y2) */
    } else {
        int n = 2 + (int)b3_u(&r, 6);
        b3_c(e, 82, 0, 0, 0, 0, 0, 0, 0);                                                              /* resetPath */
        b3_cr(e, 83, 2, &r, -6, 70);                    /* moveTo */
        for (k = 0; (int)k < n; k++) {
            unsigned t = (geom == 2 || geom == 4) ? b3_u(&r, 3) : 0;
            if (t == 0) b3_cr(e, 85, 2, &r, -6, 70);   /* lineTo */
            else if (t == 1) b3_cr(e, 93, 4, &r, -6, 70);   /* quadTo */
            else b3_cr(e, 97, 6, &r, -6, 70);   /* cubicTo */
        }
        if (b3_u(&r, 4)) b3_c(e, 102, 0, 0, 0, 0, 0, 0, 0);                                            /* closePolygon */
        if (geom >= 3) {   /* lineWidth, lineCap, lineJoin: drawn in order, then drawPath(4) */
            static const double lw[] = {0.7, 1.0, 1.5, 2.5, 4.0, 8.0}; double wv = lw[b3_u(&r, 6)]; unsigned cap = b3_u(&r, 3), jn = b3_u(&r, 5);
            b3_c(e, 42, 1, b3_D(wv), 0, 0, 0, 0, 0); b3_c(e, 44, 1, cap, 0, 0, 0, 0, 0); b3_c(e, 46, 1, jn, 0, 0, 0, 0, 0);
            b3_c(e, 103, 1, 4, 0, 0, 0, 0, 0);                                                         /* drawPath(4) */
        } else
        b3_c(e, 103, 1, 0, 0, 0, 0, 0, 0);                                                             /* drawPath(0): fill only */
    }
    if (e->mark) e->mark(e, "post");
}
#endif

/* Decision 0160 (provisional): the adapter (b2d_swap.cpp) without any lifted code, native and wasm32 alike. The "guest" is plain memory: this file builds in it the objects of the
 * guest layout that the lifted wrapper would hold (an image impl, a path impl with its command/vertex arrays, a gradient impl with stops, array views), puts the arguments into R[]
 * and X[] as the ms x64 caller does, calls the stubs fle_swap_ctx_* and compares the pixels with the same drawing done through the upstream C++ API directly. It runs the code
 * paths that depend on the pointer width (BLArrayView {u64 data, u64 size} -> {ptr, size_t}, the proxy, the mirror), which a native 64-bit run cannot distinguish from the guest layout.
 * usage: test_adapter [rounds] -> "ok" and the hash, or the first difference. */
#include <blend2d.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

extern "C" {
uint64_t R[16];
typedef union { uint64_t u64[2]; uint32_t u32[4]; float f32[4]; double f64[2]; } b2d1_xmm;
b2d1_xmm X[16];
typedef struct { uint64_t va; void (*fn)(void); } b2d1_fn_entry;
static void nop(void) {}
/* the 17 swapped addresses (B2D1_BASE + RVA) so that fill_table finds them */
extern const b2d1_fn_entry fle_dispatch_table[];
const b2d1_fn_entry fle_dispatch_table[] = {
    {0x4700000 + 0x12f170, nop}, {0x4700000 + 0x12e8a0, nop}, {0x4700000 + 0x0f7c80, nop}, {0x4700000 + 0x0f4b10, nop}, {0x4700000 + 0x0f15b0, nop}, {0x4700000 + 0x0f0ba0, nop},
    {0x4700000 + 0x0f0360, nop}, {0x4700000 + 0x0ef980, nop}, {0x4700000 + 0x0ef220, nop}, {0x4700000 + 0x0eeec0, nop}, {0x4700000 + 0x0eca70, nop}, {0x4700000 + 0x0ec670, nop},
    {0x4700000 + 0x0ea670, nop}, {0x4700000 + 0x0e9340, nop}, {0x4700000 + 0x0e8ae0, nop}, {0x4700000 + 0x12cf40, nop}, {0x4700000 + 0x12d3a0, nop}};
extern const uint64_t fle_dispatch_count;
const uint64_t fle_dispatch_count = 17;
uint64_t fle_sys_clock_ticks(void) { return 0; }
void fle_swap_ctx_init_as(void); void fle_swap_ctx_destroy(void); void fle_swap_ctx_set_style(void); void fle_swap_ctx_set_comp_op(void);
void fle_swap_ctx_set_fill_rule(void); void fle_swap_ctx_set_stroke_width(void); void fle_swap_ctx_set_stroke_join(void); void fle_swap_ctx_set_stroke_caps(void);
void fle_swap_ctx_set_global_alpha(void); void fle_swap_ctx_apply_transform_op(void); void fle_swap_ctx_clip_to_rect_d(void); void fle_swap_ctx_restore_clipping(void);
void fle_swap_ctx_fill_all_rgba32(void); void fle_swap_ctx_fill_geometry(void); void fle_swap_ctx_fill_geometry_rgba32(void);
void fle_swap_ctx_stroke_geometry(void); void fle_swap_ctx_stroke_geometry_rgba32(void);
}
static uint64_t P(const void *p) { return (uint64_t)(uintptr_t)p; }
static uint64_t fnv(const uint8_t *p, size_t n) { uint64_t h = 0xcbf29ce484222325ULL; for (size_t i = 0; i < n; i++) h = (h ^ p[i]) * 0x100000001b3ULL; return h; }
static uint32_t S = 12345;
static uint32_t rnd() { S = S * 1664525u + 1013904223u; return S >> 8; }
static double rf(double a, double b) { return a + (b - a) * ((rnd() & 0xFFFF) / 65535.0); }

enum { W = 96, H = 80 };
/* ---- the guest layout objects, built by hand (x86-64 offsets) ---- */
struct GImage { uint64_t impl; uint32_t pad, info; };
static uint8_t g_image_impl[0x40] __attribute__((aligned(16)));
static void make_image(GImage *core, uint8_t *pixels) {
    memset(g_image_impl, 0, sizeof g_image_impl);
    uint64_t px = P(pixels), stride = W * 4; int32_t w = W, h = H;
    memcpy(g_image_impl + 0, &px, 8); memcpy(g_image_impl + 8, &stride, 8); memcpy(g_image_impl + 0x10, &w, 4); memcpy(g_image_impl + 0x14, &h, 4);
    g_image_impl[0x18] = BL_FORMAT_PRGB32;
    core->impl = P(g_image_impl); core->pad = 0; core->info = 0;
}
static void call_init(GImage *img, uint64_t selfmem) {
    R[1] = selfmem; R[2] = P(img); R[8] = 0; fle_swap_ctx_init_as();
}
struct Ctx { uint64_t core[2]; uint64_t impl() const { return core[0]; } };
#define CALL(fn, a1, a2, a3, a4) do { R[1] = (a1); R[2] = (a2); R[8] = (a3); R[9] = (a4); fn(); } while (0)

int main(int argc, char **argv) {
    int rounds = argc > 1 ? atoi(argv[1]) : 40;
    blRuntimeInit();
    static uint8_t gpix[W * H * 4], upix[W * H * 4];
    int bad = 0;
    for (int round = 0; round < rounds; round++) {
        memset(gpix, 0x40, sizeof gpix); memset(upix, 0x40, sizeof upix);
        GImage img; make_image(&img, gpix);
        Ctx c; call_init(&img, P(c.core));
        uint64_t impl = c.impl();
        BLImage uimg; uimg.createFromData(W, H, BL_FORMAT_PRGB32, upix, W * 4);
        BLContextCreateInfo ci{}; ci.flags = BL_CONTEXT_CREATE_FLAG_DISABLE_JIT;
        BLContext u(uimg, ci);
        for (int step = 0; step < 60; step++) {
            unsigned op = rnd() % 9;
            uint32_t col = 0xff000000u | (rnd() & 0xffffff); if (rnd() % 3 == 0) col = (rnd() & 0xff000000u) | (rnd() & 0xffffff);
            switch (op) {
            case 0: { double r[4] = {rf(0, 60), rf(0, 50), rf(5, 60), rf(5, 50)};   /* BLRect fillGeometry RECTD (type 4) */
                CALL(fle_swap_ctx_fill_geometry_rgba32, impl, BL_GEOMETRY_TYPE_RECTD, P(r), col);
                u.fillRect(BLRect(r[0], r[1], r[2], r[3]), BLRgba32(col)); break; }
            case 1: { double e[4] = {rf(20, 70), rf(20, 60), rf(3, 30), rf(3, 30)};  /* ELLIPSE (6) */
                CALL(fle_swap_ctx_fill_geometry_rgba32, impl, BL_GEOMETRY_TYPE_ELLIPSE, P(e), col);
                u.fillEllipse(BLEllipse(e[0], e[1], e[2], e[3]), BLRgba32(col)); break; }
            case 2: { unsigned n = 3 + rnd() % 5; double v[16]; BLPoint pts[8];     /* POLYGOND: {u64 data,u64 size} array view of the guest layout */
                for (unsigned k = 0; k < n; k++) { v[2 * k] = rf(0, W); v[2 * k + 1] = rf(0, H); pts[k] = BLPoint(v[2 * k], v[2 * k + 1]); }
                uint64_t view[2] = {P(v), n};
                CALL(fle_swap_ctx_fill_geometry_rgba32, impl, BL_GEOMETRY_TYPE_POLYGOND, P(view), col);
                u.fillPolygon(pts, n, BLRgba32(col)); break; }
            case 3: { unsigned n = 2 + rnd() % 5; double v[16]; BLPoint pts[8];     /* POLYLINED stroke */
                for (unsigned k = 0; k < n; k++) { v[2 * k] = rf(0, W); v[2 * k + 1] = rf(0, H); pts[k] = BLPoint(v[2 * k], v[2 * k + 1]); }
                uint64_t view[2] = {P(v), n};
                CALL(fle_swap_ctx_stroke_geometry_rgba32, impl, BL_GEOMETRY_TYPE_POLYLINED, P(view), col);
                u.strokePolyline(pts, n, BLRgba32(col)); break; }
            case 4: { double w = rf(0.5, 6); X[1].f64[0] = w; CALL(fle_swap_ctx_set_stroke_width, impl, 0, 0, 0); u.setStrokeWidth(w); break; }
            case 5: { unsigned j = rnd() % 5; CALL(fle_swap_ctx_set_stroke_join, impl, j, 0, 0); u.setStrokeJoin((BLStrokeJoin)j); break; }
            case 6: { unsigned fr = rnd() % 2; CALL(fle_swap_ctx_set_fill_rule, impl, fr, 0, 0); u.setFillRule((BLFillRule)fr); break; }
            case 7: { double t[2] = {rf(-5, 5), rf(-5, 5)}; CALL(fle_swap_ctx_apply_transform_op, impl, BL_TRANSFORM_OP_TRANSLATE, P(t), 0); u.translate(t[0], t[1]); break; }
            default: {  /* a path of the guest layout (command/vertex arrays + impl) filled with the context's colour */
                uint8_t cmds[8]; double vtx[16]; unsigned n = 3 + rnd() % 4; BLPath up;
                for (unsigned k = 0; k < n; k++) { double x = rf(0, W), y = rf(0, H); vtx[2 * k] = x; vtx[2 * k + 1] = y; cmds[k] = k ? BL_PATH_CMD_ON : BL_PATH_CMD_MOVE; if (k) up.lineTo(x, y); else up.moveTo(x, y); }
                cmds[n - 1] = BL_PATH_CMD_ON; up.close(); uint8_t cmds2[9]; memcpy(cmds2, cmds, n); cmds2[n] = BL_PATH_CMD_CLOSE; double vtx2[18]; memcpy(vtx2, vtx, 16 * sizeof(double)); vtx2[2 * n] = 0; vtx2[2 * n + 1] = 0;
                uint8_t pimpl[0x40] __attribute__((aligned(16))); memset(pimpl, 0, sizeof pimpl);
                uint64_t c0 = P(cmds2), v0 = P(vtx2), sz = n + 1; memcpy(pimpl, &c0, 8); memcpy(pimpl + 8, &v0, 8); memcpy(pimpl + 0x10, &sz, 8);
                uint64_t pcore[2] = {P(pimpl), 0};
                CALL(fle_swap_ctx_fill_geometry_rgba32, impl, BL_GEOMETRY_TYPE_PATH, P(pcore), col);
                u.fillPath(up, BLRgba32(col)); break; }
            }
        }
        uint64_t hg = fnv(gpix, sizeof gpix), hu = fnv(upix, sizeof upix);
        if (hg != hu) { printf("round %d: pixels differ: adapter %#llx direct %#llx\n", round, (unsigned long long)hg, (unsigned long long)hu); bad++; }
        CALL(fle_swap_ctx_destroy, impl, 0, 0, 0);
    }
    printf(bad ? "FAILED %d of %d rounds\n" : "ok %d rounds\n", bad ? bad : rounds, rounds);
    return bad ? 1 : 0;
}

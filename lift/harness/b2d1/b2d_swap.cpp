/* Decision 0160 (provisional): host implementation of the BLContext of the guest's Blend2D (Image-Line's Blend2D.dll = upstream Blend2D 0.11.1 + the Agg2D wrapper).
 *
 * Linked only by a build that asks for it (lift/harness/b2d1/build_b2d1.sh). The lifted wrapper keeps its own object model (BLPath, BLGradient, BLImage, the headers of their
 * impls, refcounts), in the guest layout (x86-64: 8-byte pointers and size_t). What the wrapper does with the context goes through two things that the generator
 * (fe-lift --swap-entries) replaces by calls into this file:
 *   - blContextInitAs  (RVA 0x146ff0): builds a PROXY impl in the guest layout and an upstream BLContext on the guest's pixels (the lifted blImageCreateFromData made the
 *     image; its impl is read, never written);
 *   - 17 slots of the context's virtual function table (destroy and the 16 that the wrapper calls): the proxy's own table, in the guest layout, holds the VAs of the
 *     swapped functions (functions of the DLL's static BLContextVirt table at RVA 0x1d6380, whose bodies are not lifted), each of which forwards to the upstream
 *     BLContextVirt function of the real context, converting what is in the guest layout (path, gradient, array views) into upstream objects for the duration of the call.
 * The wrapper also READS the proxy directly (refcount/flags at impl-0x10/-8, virt at +0, state at +8, and the state: transforms, compOp, fillRule, stroke width/caps/join,
 * globalAlpha, ...): the proxy holds a mirror of the upstream BLContextState in the guest layout, refreshed after every call that can change the state. Nothing is guessed:
 * a style that is not a colour or a gradient, a geometry that is not handled, a table slot that was not filled, stop the run by name (fail closed).
 *
 * Calling convention of a swapped entry (ms x64, lifted): arguments R[1] R[2] R[8] R[9] and X[1..3] (double arguments in the xmm of their position), result R[0].
 * Written for the native and the wasm32 build alike: the guest layout is described by explicit offsets and fixed-width reads, never by sizeof of an upstream struct that
 * holds a pointer. */
#include <blend2d.h>
#include "../../runtime/fle_test_env.h"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

extern "C" {
typedef union { uint64_t u64[2]; uint32_t u32[4]; float f32[4]; double f64[2]; } b2d1_xmm;
/* Decision 0167, 0234, 0243: in the wasm build R and X are the first 0x300 bytes of the thread-local block fle_ctx (runtime/fle_lowregs.h), as in the generated C; the native build declares them as data (decision 0160). */
#ifdef __EMSCRIPTEN__
extern thread_local unsigned char fle_ctx[];   /* decision 0234 (mt3, B-prime stage 1): runtime/fle_lowregs.h */
#define R (*(uint64_t (*)[16])(fle_ctx + 0x000))
#define X (*(b2d1_xmm (*)[16])(fle_ctx + 0x200))
#else
extern uint64_t R[16];
extern b2d1_xmm X[16];
#endif
typedef struct { uint64_t va; void (*fn)(void); } b2d1_fn_entry;
extern const b2d1_fn_entry fle_dispatch_table[];
extern const uint64_t fle_dispatch_count;
}

#define B2D1_BASE UINT64_C(0x4700000)   /* the load address of the lifted Blend2D (decision 0160) */

[[noreturn]] static void die(const char *what, uint64_t a = 0, uint64_t b = 0) {
    fprintf(stderr, "fle: b2d1 (decision 0160): %s %#llx %#llx\n", what, (unsigned long long)a, (unsigned long long)b);
    abort();
}

/* Decision 0167 (always on since decision 0243): upstream's no-JIT (fixed) pipeline implements 4 of the 29 composition operators and returns BL_ERROR_NOT_IMPLEMENTED for the others, drawing nothing. The guest
 * ignores the result, so without this check the picture would silently lack the primitive; with it the run stops by name. A native test entry, the environment B2D2_STRICT=0 (FLE_TEST_GETENV: the wasm product reads none), lets the
 * Agg2D scan compare against the lifted DLL, which returns the same error silently. */
static int strict_on(void) {
    static int v = -1;
    if (v < 0) { const char *e = FLE_TEST_GETENV("B2D2_STRICT"); v = e ? (e[0] != '0') : 1; }
    return v;
}
static void b2d3_pre(void *proxy, unsigned slot, uint32_t geom);
static void b2d3_census_draw(void *proxy, unsigned slot, uint32_t geom, uint32_t rc);
static void b2d3_unimpl_note(void *proxy, unsigned slot, uint32_t geom);
#define DRAW_RET(call, slot, geom) do { b2d3_pre(p, (slot), (geom)); uint32_t rc_ = (uint32_t)(call); b2d3_census_draw(p, (slot), (geom), rc_); if (rc_ == BL_ERROR_NOT_IMPLEMENTED && strict_on()) { b2d3_unimpl_note(p, (slot), (geom)); die("upstream no-JIT pipeline returned BL_ERROR_NOT_IMPLEMENTED (a JIT-only composition operator): compOp, BLContextVirt slot", p->gs.compOp, (slot)); } RET(rc_); } while (0)

static inline void *G(uint64_t a) {
#ifdef __EMSCRIPTEN__
    if (a >> 32) die("guest pointer above 4 GiB", a);
#endif
    return (void *)(uintptr_t)a;
}
static inline uint64_t ld64(uint64_t a) { uint64_t v; memcpy(&v, G(a), 8); return v; }
static inline uint32_t ld32(uint64_t a) { uint32_t v; memcpy(&v, G(a), 4); return v; }
static inline void st64(uint64_t a, uint64_t v) { memcpy(G(a), &v, 8); }
static inline void st32(uint64_t a, uint32_t v) { memcpy(G(a), &v, 4); }

/* ---- the guest layout of what the wrapper reads (offsets: tables/wrapper-reads.tsv; the x86-64 layout of upstream 0.11.1, checked against offsetof in a native build below) ---- */
struct GuestState {                    /* BLContextState */
    uint64_t targetImage;              /* 0x00 (the wrapper never reads it; kept 0) */
    double targetW, targetH;           /* 0x08 */
    uint8_t hints[8];                  /* 0x18 */
    uint8_t compOp, fillRule, styleType[2]; uint32_t savedStateCount;   /* 0x20 */
    double globalAlpha;                /* 0x28 */
    double styleAlpha[2];              /* 0x30 */
    uint8_t startCap, endCap, join, transformOrder, reserved[4];   /* 0x40 strokeOptions */
    double width, miterLimit, dashOffset;                          /* 0x48 */
    uint64_t dashArray[2];             /* 0x60 (a BLArrayCore of the guest layout: kept 0) */
    uint8_t approx[32];                /* 0x70 */
    double meta[6], user[6], fin[6];   /* 0x90 0xc0 0xf0 */
};
static_assert(sizeof(GuestState) == 0x120, "guest BLContextState is 0x120 bytes");
static_assert(__builtin_offsetof(GuestState, hints) == 0x18 && __builtin_offsetof(GuestState, compOp) == 0x20 && __builtin_offsetof(GuestState, globalAlpha) == 0x28 &&
              __builtin_offsetof(GuestState, startCap) == 0x40 && __builtin_offsetof(GuestState, width) == 0x48 && __builtin_offsetof(GuestState, approx) == 0x70 &&
              __builtin_offsetof(GuestState, meta) == 0x90 && __builtin_offsetof(GuestState, user) == 0xc0 && __builtin_offsetof(GuestState, fin) == 0xf0, "layout");
#if __SIZEOF_POINTER__ == 8
static_assert(__builtin_offsetof(BLContextState, userTransform) == 0xc0 && __builtin_offsetof(BLContextState, finalTransform) == 0xf0 &&
              __builtin_offsetof(BLContextState, strokeOptions) == 0x40 && __builtin_offsetof(BLContextState, approximationOptions) == 0x70 &&
              __builtin_offsetof(BLContextState, compOp) == 0x20 && sizeof(BLContextState) == 0x120, "native layout = guest layout");
#endif
static_assert(sizeof(BLContextHints) == 8 && sizeof(BLApproximationOptions) == 32 && sizeof(BLMatrix2D) == 48, "pointer-free parts have one size");

#define B2D1_MAGIC UINT64_C(0xb2d1b2d1c0de0160)
#define VIRT_SLOTS 80
struct Proxy {                         /* allocated 16-aligned; the guest sees &impl */
    uint64_t refCount, flags;          /* header, at impl-0x10, impl-0x08 (the DLL's own context: refCount 1, flags 0x21) */
    uint64_t virt;                     /* impl+0x00 */
    uint64_t state;                    /* impl+0x08 */
    uint32_t contextType, pad;         /* impl+0x10 */
    uint64_t magic;
    BLContextCore real;                /* upstream */
    BLImageCore img;                   /* upstream, on the guest's pixels */
    GuestState gs;
    uint64_t table[VIRT_SLOTS];        /* the guest-visible BLContextVirt */
    uint8_t fmt;                       /* decision 0195 (census only): BLFormat of the target image */
    uint8_t gtype[2], gext[2];         /* decision 0195 (census only): type/extend of the last gradient set in slot fill(0)/stroke(1); 0xff = none */
    uint8_t *pixels; intptr_t stride; int tw, th;   /* decision 0195 (census only): the target pixels (the guest's) */
    uint8_t *snap;                     /* decision 0195 (census only): a copy of the target before a draw that is not SRC_OVER */
};
static_assert(__builtin_offsetof(Proxy, virt) == 0x10 && __builtin_offsetof(Proxy, state) == 0x18, "impl is at +0x10");

static Proxy *P_of(uint64_t impl) {
    Proxy *p = (Proxy *)((char *)G(impl) - 0x10);
    if (p->magic != B2D1_MAGIC) die("not a b2d1 context impl", impl);
    return p;
}
static inline BLContextImpl *RI(Proxy *p) { return (BLContextImpl *)p->real._d.impl; }

/* Mirror: the guest-layout BLContextState, from the upstream state. Called after every call that can change the state. */
static void fill_state(GuestState *g, const BLContextState *s) {
    g->targetW = s->targetSize.w; g->targetH = s->targetSize.h;
    /* Decision 0167: only hints[0..2] (rendering, gradient, pattern quality) are defined; upstream leaves hints[3..7] of a fresh state uninitialised. They were zero on native (a fresh heap) and 0x20 in wasm32 (a reused heap block): the
     * scan's state hash of the tests `reattach` and `dtor` differed from the lifted DLL's in that one byte, state offset 0x1c. The guest never reads them; the DLL's own state has zero there. */
    memset(g->hints, 0, 8); memcpy(g->hints, &s->hints, 3);
    g->compOp = s->compOp; g->fillRule = s->fillRule; g->styleType[0] = s->styleType[0]; g->styleType[1] = s->styleType[1];
    g->savedStateCount = s->savedStateCount;
    g->globalAlpha = s->globalAlpha; g->styleAlpha[0] = s->styleAlpha[0]; g->styleAlpha[1] = s->styleAlpha[1];
    g->startCap = s->strokeOptions.startCap; g->endCap = s->strokeOptions.endCap; g->join = s->strokeOptions.join; g->transformOrder = s->strokeOptions.transformOrder;
    g->width = s->strokeOptions.width; g->miterLimit = s->strokeOptions.miterLimit; g->dashOffset = s->strokeOptions.dashOffset;
    memcpy(g->approx, &s->approximationOptions, 32); memset(g->approx + 2, 0, 6);   /* reservedFlags[6]: "must be zero", not written by every upstream path */
    memcpy(g->meta, &s->metaTransform, 48); memcpy(g->user, &s->userTransform, 48); memcpy(g->fin, &s->finalTransform, 48);
}
static void sync_state(Proxy *p) { fill_state(&p->gs, RI(p)->state); }
/* B2D1_CHECK=1: after every swapped call the mirror must be what a fresh copy of the upstream state gives (the invariant: the guest never reads a stale state) */
static int check_on = -1;
static void check_mirror(Proxy *p, const char *where) {
    if (check_on < 0) check_on = FLE_TEST_GETENV("B2D1_CHECK") ? 1 : 0;
    if (!check_on) return;
    GuestState t; memset(&t, 0, sizeof t);
    fill_state(&t, RI(p)->state);
    if (memcmp(&t, &p->gs, sizeof t) != 0) {
        const unsigned char *a = (const unsigned char *)&t, *b = (const unsigned char *)&p->gs; size_t i = 0;
        while (a[i] == b[i]) i++;
        die(where, i, 0);
    }
}

/* The 17 swapped functions of the DLL's table at RVA 0x1d6380 (first of its three tables; their 17 addresses are distinct), by slot of BLContextVirt. */
struct SlotRva { unsigned slot; uint32_t rva; };
static const SlotRva slot_rva[] = {
    {0, 0x12f170}, {3, 0x12e8a0}, {19, 0x0f7c80}, {25, 0x0f4b10}, {32, 0x0f15b0}, {33, 0x0f0ba0}, {34, 0x0f0360}, {35, 0x0ef980}, {38, 0x0ef220},
    {39, 0x0eeec0}, {45, 0x0eca70}, {46, 0x0ec670}, {51, 0x0ea670}, {53, 0x0e9340}, {54, 0x0e8ae0}, {71, 0x12cf40}, {72, 0x12d3a0},
};
static int dispatch_has(uint64_t va) {
    for (uint64_t i = 0; i < fle_dispatch_count; i++) if (fle_dispatch_table[i].va == va) return 1;
    return 0;
}
static void fill_table(Proxy *p) {
    static int checked;   /* the dispatch table has over 100,000 rows: look the 17 addresses up once, not at every attach */
    for (const SlotRva &s : slot_rva) {
        uint64_t va = B2D1_BASE + s.rva;
        if (!checked && !dispatch_has(va)) die("a swapped context function is not in the dispatch table (check the swap file and B2D1_BASE)", va);
        p->table[s.slot] = va;
    }
    checked = 1;
}

/* ---- conversions of what the guest holds in its own layout ---- */
struct TmpPath { BLPathCore p; int live; };
static void path_from_guest(TmpPath *t, uint64_t core) {
    blPathInit(&t->p); t->live = 1;
    uint64_t impl = ld64(core);
    if (!impl) return;
    uint64_t cmd = ld64(impl + 0), vtx = ld64(impl + 8), n = ld64(impl + 0x10);
    if (!n) return;
    uint8_t *cd; BLPoint *vd;
    if (blPathModifyOp(&t->p, BL_MODIFY_OP_ASSIGN_FIT, (size_t)n, &cd, &vd) != BL_SUCCESS) die("path copy failed", n);
    memcpy(cd, G(cmd), (size_t)n);
    memcpy(vd, G(vtx), (size_t)n * 16);
}

struct GeomTmp { TmpPath path; BLArrayView<BLPoint> view; const void *data; };
static void geom_from_guest(GeomTmp *g, uint32_t type, uint64_t data) {
    g->path.live = 0;
    if (type >= BL_GEOMETRY_TYPE_BOXI && type <= BL_GEOMETRY_TYPE_TRIANGLE) { g->data = G(data); return; }
    if (type >= BL_GEOMETRY_TYPE_POLYLINEI && type <= BL_GEOMETRY_TYPE_ARRAY_VIEW_RECTD) {
        /* BLArrayView<T> of the guest layout: {u64 data, u64 size}; the element types hold no pointers */
        g->view.data = (const BLPoint *)G(ld64(data)); g->view.size = (size_t)ld64(data + 8);
        g->data = &g->view; return;
    }
    if (type == BL_GEOMETRY_TYPE_PATH) { path_from_guest(&g->path, data); g->data = &g->path.p; return; }
    die("geometry type not handled", type);
}
static void geom_done(GeomTmp *g) { if (g->path.live) blPathReset(&g->path.p); }

/* a style object: colours (RGBA, RGBA32, RGBA64: no pointer, copied as 16 bytes) and gradients (converted); anything else stops */
struct StyleTmp { BLObjectCore core; BLGradientCore grad; int live_grad; };
static uint8_t last_gtype = 0xff, last_gext = 0xff;   /* decision 0195 (census only): of the gradient that style_from_guest converted last, 0xff = not a gradient */
static const BLObjectCore *style_from_guest(StyleTmp *s, uint64_t style) {
    s->live_grad = 0; last_gtype = last_gext = 0xff;
    uint32_t info = ld32(style + 12);
    unsigned type = (info & 0x80000000u) ? ((info >> 22) & 0x7f) : 0;
    if (type == BL_OBJECT_TYPE_RGBA || type == BL_OBJECT_TYPE_RGBA32 || type == BL_OBJECT_TYPE_RGBA64 || type == BL_OBJECT_TYPE_NULL) {
        memcpy(&s->core, G(style), 16); return &s->core;
    }
    if (type == BL_OBJECT_TYPE_GRADIENT) {
        uint64_t impl = ld64(style);
        if (!impl) die("gradient with no impl", style);
        unsigned gtype = (info >> 16) & 0x3f, extend = (info >> 12) & 0xf;
        last_gtype = (uint8_t)gtype; last_gext = (uint8_t)extend;
        uint64_t stops = ld64(impl + 0), n = ld64(impl + 8);
        if (blGradientInitAs(&s->grad, (BLGradientType)gtype, G(impl + 0x48), (BLExtendMode)extend, (const BLGradientStop *)G(stops), (size_t)n, (const BLMatrix2D *)G(impl + 0x18)) != BL_SUCCESS)
            die("gradient copy failed", gtype);
        s->live_grad = 1; return (const BLObjectCore *)&s->grad;
    }
    die("style object type not handled", type, info);
}
static void style_done(StyleTmp *s) { if (s->live_grad) blGradientReset(&s->grad); }


/* ---- counting (opt-in: B2D1_STATS=<file>): what the guest passes to the context, per virtual-clock millisecond (the g3 snapshot windows are cut from the harness log by the caller) ---- */
extern "C" uint64_t fle_sys_clock_ticks(void);
struct StatRec { uint64_t ms; uint32_t kind, val, aux; uint64_t cnt; };
static StatRec *stat_recs; static size_t stat_n, stat_cap; static int stat_on = -1;
static void stat_dump(void) {
    const char *f = FLE_TEST_GETENV("B2D1_STATS");
    FILE *o = f ? fopen(f, "w") : NULL;
    if (!o) return;
    fprintf(o, "clock_ms\tkind\tvalue\taux\tcount\n");
    for (size_t i = 0; i < stat_n; i++) fprintf(o, "%llu\t%u\t%u\t%u\t%llu\n", (unsigned long long)stat_recs[i].ms, stat_recs[i].kind, stat_recs[i].val, stat_recs[i].aux, (unsigned long long)stat_recs[i].cnt);
    fclose(o);
}
/* kind 1 = setCompOp(val); kind 2 = draw (aux = BLContextVirt slot 51/53/54/71/72) under compOp val; kind 3 = initAs; kind 4 = destroy */
static unsigned long stat_calls;
static void stat_add(uint32_t kind, uint32_t val, uint32_t aux) {
    if (stat_on < 0) { stat_on = FLE_TEST_GETENV("B2D1_STATS") ? 1 : 0; if (stat_on) atexit(stat_dump); }
    if (!stat_on) return;
    if ((++stat_calls & 0x3ff) == 0) stat_dump();   /* the harness may leave through _exit: rewrite the file now and then */
    uint64_t ms = fle_sys_clock_ticks() / 10000u;
    for (size_t i = stat_n; i-- > 0 && i + 32 > stat_n;)
        if (stat_recs[i].ms == ms && stat_recs[i].kind == kind && stat_recs[i].val == val && stat_recs[i].aux == aux) { stat_recs[i].cnt++; return; }
    if (stat_n == stat_cap) { stat_cap = stat_cap ? stat_cap * 2 : 4096; stat_recs = (StatRec *)realloc(stat_recs, stat_cap * sizeof *stat_recs); if (!stat_recs) die("out of memory"); }
    stat_recs[stat_n++] = StatRec{ms, kind, val, aux, 1};
}


/* ---- decision 0195 (opt-in, B2D3_CENSUS=1 at run time; default OFF and then nothing here runs): which (draw slot, composition operator, geometry type, style, gradient type/extend/quality, target format, rc) the guest asks for, and the first
 * guest return addresses that look like the instruction after a `call` inside FLEngine (a scan of the guest stack: stale words can be mixed in, so this is a READING, not a call stack). One line per new tuple when first seen, a line per new caller of
 * a tuple whose operator is not SRC_OVER (or whose result is not success), the whole table again every 1024 draws (a browser tab has no exit hook), all to stderr with the prefix "b2d3-census:". For a draw whose operator is not SRC_OVER the number
 * of target pixels the draw changed and their bounding box are printed (CHANGE; at most 400 lines). Never changes what is drawn; with B2D2_STRICT=0 a NOT_IMPLEMENTED draw is counted (rc) and the run goes on. ---- */
struct CensusKey { uint8_t slot, compOp, style, gtype, gext, gq, fmt, galpha; uint32_t geom, rc; };
struct CensusCaller { uint64_t wr, c1, n; };
struct CensusRow { CensusKey k; uint64_t n; CensusCaller callers[4]; int ncallers; };
static CensusRow *cen_rows; static size_t cen_n, cen_cap; static unsigned long cen_draws; static int cen_on = -1; static int cen_change_lines;
static int census_on(void) { if (cen_on < 0) cen_on = FLE_TEST_GETENV("B2D3_CENSUS") ? 1 : 0; return cen_on; }
static void census_print(const CensusRow *r, const char *tag) {
    const CensusKey &k = r->k;
    fprintf(stderr, "b2d3-census: %s slot=%u compOp=%u geom=%u style=%u gtype=%u gext=%u gq=%u fmt=%u galpha=%u rc=%u n=%llu", tag, k.slot, k.compOp, (unsigned)k.geom, k.style, k.gtype, k.gext, k.gq, k.fmt, k.galpha, (unsigned)k.rc, (unsigned long long)r->n);
    for (int i = 0; i < r->ncallers; i++) fprintf(stderr, " caller%d=%#llx/%#llx:%llu", i, (unsigned long long)r->callers[i].c1, (unsigned long long)r->callers[i].wr, (unsigned long long)r->callers[i].n);
    fprintf(stderr, "\n");
}
/* does the code before guest address w end in a `call` (E8 rel32, or FF /2 with a 2..7 byte encoding)? */
static int census_after_call(uint64_t w) {
    const uint8_t *c = (const uint8_t *)G(w);
    if (c[-5] == 0xE8) return 1;
    for (int len = 2; len <= 7; len++) if (c[-len] == 0xFF && ((c[-len + 1] >> 3) & 7) == 2) return 1;
    return 0;
}
static void census_callers(uint64_t *wr, uint64_t *c1) {
    *wr = *c1 = 0;
    uint64_t sp = R[4];
    if (sp < 0x10000 || (sp & 7)) return;
    *wr = ld64(sp);
    for (int i = 1; i < 1024; i++) {
        uint64_t w = ld64(sp + 8 * (uint64_t)i);
        if (w >= 0x401000 && w < 0x1400000 && census_after_call(w)) { *c1 = w; return; }
    }
}
static void census_key(const Proxy *p, unsigned slot, uint32_t geom, uint32_t rc, CensusKey *k) {
    memset(k, 0, sizeof *k);
    const unsigned fs = (slot == 71 || slot == 72) ? 1u : 0u;   /* the style slot a draw uses: fill (0) or stroke (1) */
    k->slot = (uint8_t)slot; k->compOp = p->gs.compOp; k->geom = geom; k->rc = rc; k->fmt = p->fmt;
    k->style = (slot == 54 || slot == 72 || slot == 51) ? (uint8_t)BL_OBJECT_TYPE_RGBA32 : (uint8_t)p->gs.styleType[fs];   /* style = BLObjectType (RGBA 0, RGBA32 1, RGBA64 2, NULL 3, PATTERN 4, GRADIENT 5); the Rgba32 forms and fillAllRgba32 take a colour */
    if (k->style == BL_OBJECT_TYPE_GRADIENT) { k->gtype = p->gtype[fs]; k->gext = p->gext[fs]; } else { k->gtype = k->gext = 0xff; }
    k->gq = p->gs.hints[1]; k->galpha = p->gs.globalAlpha != 1.0;
}
static void census_dump(const char *tag) { for (size_t i = 0; i < cen_n; i++) census_print(&cen_rows[i], tag); }

static void b2d3_pre(void *proxy, unsigned slot, uint32_t geom) {
    Proxy *p = (Proxy *)proxy;
    if (census_on() && p->gs.compOp != BL_COMP_OP_SRC_OVER && p->pixels && cen_change_lines < 400) {
        size_t n = (size_t)p->tw * (size_t)p->th * 4;
        free(p->snap); p->snap = (uint8_t *)malloc(n);
        if (p->snap) for (int y = 0; y < p->th; y++) memcpy(p->snap + (size_t)y * p->tw * 4, p->pixels + (size_t)y * p->stride, (size_t)p->tw * 4);
    }
}
static void b2d3_census_draw(void *proxy, unsigned slot, uint32_t geom, uint32_t rc) {
    if (!census_on()) return;
    Proxy *p = (Proxy *)proxy;
    CensusKey k; census_key(p, slot, geom, rc, &k);
    uint64_t wr, c1; census_callers(&wr, &c1);
    cen_draws++;
    CensusRow *row = NULL;
    for (size_t i = 0; i < cen_n; i++) if (memcmp(&cen_rows[i].k, &k, sizeof k) == 0) { row = &cen_rows[i]; break; }
    if (!row) {
        if (cen_n == cen_cap) { cen_cap = cen_cap ? cen_cap * 2 : 256; cen_rows = (CensusRow *)realloc(cen_rows, cen_cap * sizeof *cen_rows); if (!cen_rows) die("out of memory"); }
        row = &cen_rows[cen_n++]; memset(row, 0, sizeof *row); row->k = k;
        row->n = 1; census_print(row, "NEW");
    } else row->n++;
    int c; for (c = 0; c < row->ncallers; c++) if (row->callers[c].c1 == c1 && row->callers[c].wr == wr) { row->callers[c].n++; break; }
    if (c == row->ncallers && c < 4) {
        row->callers[c].wr = wr; row->callers[c].c1 = c1; row->callers[c].n = 1; row->ncallers++;
        if (k.compOp != BL_COMP_OP_SRC_OVER || rc != 0) census_print(row, "CALLER");
    }
    if (p->snap && p->gs.compOp != BL_COMP_OP_SRC_OVER) {
        long ch = 0; int x0 = p->tw, y0 = p->th, x1 = -1, y1 = -1;
        for (int y = 0; y < p->th; y++) {
            const uint8_t *a = p->snap + (size_t)y * p->tw * 4, *b = p->pixels + (size_t)y * p->stride;
            for (int x = 0; x < p->tw; x++) if (memcmp(a + 4 * x, b + 4 * x, 4)) { ch++; if (x < x0) x0 = x; if (x > x1) x1 = x; if (y < y0) y0 = y; if (y > y1) y1 = y; }
        }
        cen_change_lines++;
        fprintf(stderr, "b2d3-census: CHANGE slot=%u compOp=%u geom=%u rc=%u target=%dx%d changed_pixels=%ld bbox=%d,%d..%d,%d\n", slot, k.compOp, (unsigned)geom, (unsigned)rc, p->tw, p->th, ch, x0, y0, x1, y1);
    }
    if ((cen_draws & 1023) == 0) { fprintf(stderr, "b2d3-census: DUMP draws=%lu rows=%zu\n", cen_draws, cen_n); census_dump("ROW"); }
}
static void b2d3_unimpl_note(void *proxy, unsigned slot, uint32_t geom) {
    Proxy *p = (Proxy *)proxy; const unsigned fs = (slot == 71 || slot == 72) ? 1u : 0u;
    fprintf(stderr, "fle: b2d3 (decision 0195): the draw that stops: BLContextVirt slot %u, compOp %u, geometry type %u, style type %u (gradient type %u extend %u), target format %u\n", slot, p->gs.compOp, (unsigned)geom,
            (slot == 54 || slot == 72 || slot == 51) ? (unsigned)BL_OBJECT_TYPE_RGBA32 : (unsigned)p->gs.styleType[fs], p->gtype[fs], p->gext[fs], p->fmt);
}

/* ---- libm: the lifted DLL computes sin/cos/tan/atan2/acos/pow/fmod with its own (MSVC) C runtime, which is not always the correctly rounded result that the host's libm gives (measured:
 * sin(pi/4) and cos(1.0) differ in the last bit, runs/rotprobe-*.txt). Upstream Blend2D calls these through <math.h>; the symbols below (decision 0243: always) take precedence over the
 * host's libm and call the DLL's lifted functions (RVAs found by calling the candidates and matching the results, tests libmprobe), so that rotations, arcs, stroke joins and so on
 * are computed by the same code as in the unswapped build. cbrt is not in the DLL's runtime as far as found: left to the host. The call is a guest call: the arguments in X[0], X[1], a
 * return slot pushed on the guest stack, the result in X[0]. Everything the adapter needs from R[] and X[] is read before any such call. ---- */
enum { M_SIN, M_COS, M_TAN, M_ATAN2, M_ACOS, M_POW, M_FMOD, M_N };
static const uint32_t math_rva[M_N] = {0x1b4480, 0x1b24d0, 0x1b4b60, 0x1b1de4, 0x1b1b40, 0x1b2de0, 0x1b2b00};
static void (*math_fn[M_N])(void);
static double call_math(int id, double a, double b) {
    if (!math_fn[id]) {
        for (uint64_t i = 0; i < fle_dispatch_count; i++) if (fle_dispatch_table[i].va == B2D1_BASE + math_rva[id]) math_fn[id] = fle_dispatch_table[i].fn;
        if (!math_fn[id]) die("a lifted C-runtime math function is not in the dispatch table", B2D1_BASE + math_rva[id]);
    }
    memcpy(&X[0].u64[0], &a, 8); X[0].u64[1] = 0; memcpy(&X[1].u64[0], &b, 8); X[1].u64[1] = 0;
    uint64_t saved_sp = R[4];
    R[4] = ((saved_sp - 0x40) & ~UINT64_C(15)) - 8;   /* a call from a 16-byte aligned frame: rsp = 8 mod 16 at the callee's entry (the lifted code uses aligned SSE stores on its stack) */
    st64(R[4], 0);
    math_fn[id]();
    R[4] = saved_sp;
    double r; memcpy(&r, &X[0].u64[0], 8);
    return r;
}
extern "C" double sin(double x) { return call_math(M_SIN, x, 0); }
extern "C" double cos(double x) { return call_math(M_COS, x, 0); }
extern "C" double tan(double x) { return call_math(M_TAN, x, 0); }
extern "C" double atan2(double y, double x) { return call_math(M_ATAN2, y, x); }
extern "C" double acos(double x) { return call_math(M_ACOS, x, 0); }
extern "C" double pow(double x, double y) { return call_math(M_POW, x, y); }
extern "C" double fmod(double x, double y) { return call_math(M_FMOD, x, y); }
extern "C" void sincos(double x, double *s, double *c) { *s = call_math(M_SIN, x, 0); *c = call_math(M_COS, x, 0); }

/* ---- the swapped functions ---- */
#define RET(x) (R[0] = (uint32_t)(x))
#define ARGI(i) (i == 1 ? R[1] : i == 2 ? R[2] : i == 3 ? R[8] : R[9])

static long live_ctx;   /* proxies created and not yet destroyed (B2D1_CHECK=1 stops at 64: the guest's release path must reach destroy) */
extern "C" void fle_swap_ctx_init_as(void) {
    uint64_t self = R[1], image = R[2], cci = R[8];
    static int up_init;
    if (!up_init) { blRuntimeInit(); up_init = 1; }   /* a static library: the first use initialises it (a repeated call only counts) */
    if (!image) die("blContextInitAs with no image");
    Proxy *p = (Proxy *)aligned_alloc(16, (sizeof(Proxy) + 15) & ~(size_t)15);
    if (!p) die("out of memory");
    memset(p, 0, sizeof *p);
    uint64_t im = ld64(image);
    if (!im) die("blContextInitAs: the image has no impl", image);
    void *pixels = G(ld64(im + 0)); intptr_t stride = (intptr_t)(int64_t)ld64(im + 8);
    int w = (int)ld32(im + 0x10), h = (int)ld32(im + 0x14); unsigned fmt = *(uint8_t *)G(im + 0x18);
    BLResult r = blImageInitAsFromData(&p->img, w, h, (BLFormat)fmt, pixels, stride, BL_DATA_ACCESS_RW, NULL, NULL);
    if (r != BL_SUCCESS) { free(p); RET(r); return; }
    BLContextCreateInfo ci; memset(&ci, 0, sizeof ci);
    if (cci) memcpy(&ci, G(cci), sizeof ci);
    r = blContextInitAs(&p->real, &p->img, &ci);
    if (r != BL_SUCCESS) { blImageReset(&p->img); free(p); RET(r); return; }
    p->refCount = 1; p->flags = 0x21; p->magic = B2D1_MAGIC;
    live_ctx++;
    if (live_ctx > 64 && FLE_TEST_GETENV("B2D1_CHECK")) die("context leak: more than 64 live proxies; the guest's release did not reach destroy", live_ctx);
    p->contextType = RI(p)->contextType;
    p->pixels = (uint8_t *)pixels; p->stride = stride; p->tw = w; p->th = h;
    p->fmt = (uint8_t)fmt; p->gtype[0] = p->gtype[1] = p->gext[0] = p->gext[1] = 0xff;
    fill_table(p);
    p->virt = (uint64_t)(uintptr_t)&p->table[0];
    p->state = (uint64_t)(uintptr_t)&p->gs;
    sync_state(p);
    stat_add(3, 0, 0);
    uint64_t impl = (uint64_t)(uintptr_t)&p->virt;
    st64(self, impl); st32(self + 8, 0); st32(self + 12, p->real._d.info.bits);
    RET(BL_SUCCESS);
}

extern "C" void fle_swap_ctx_destroy(void) {
    Proxy *p = P_of(R[1]);
    stat_add(4, 0, 0);
    live_ctx--;
    blContextReset(&p->real);
    blImageReset(&p->img);
    free(p->snap);
    p->magic = 0;
    free(p);
    RET(BL_SUCCESS);
}

extern "C" void fle_swap_ctx_apply_transform_op(void) {
    Proxy *p = P_of(R[1]); BLContextImpl *ri = RI(p);
    RET(ri->virt->applyTransformOp(ri, (BLTransformOp)(uint32_t)R[2], R[8] ? G(R[8]) : NULL));
    sync_state(p);
}
extern "C" void fle_swap_ctx_set_hint(void) {
    Proxy *p = P_of(R[1]); BLContextImpl *ri = RI(p);
    RET(ri->virt->setHint(ri, (BLContextHint)(uint32_t)R[2], (uint32_t)R[8]));
    sync_state(p);
    check_mirror(p, __func__);
}
extern "C" void fle_swap_ctx_set_style(void) {
    Proxy *p = P_of(R[1]); BLContextImpl *ri = RI(p);
    StyleTmp st;
    const BLObjectCore *s = style_from_guest(&st, R[8]);
    { const uint32_t slot_ = (uint32_t)R[2] & 1u; p->gtype[slot_] = last_gtype; p->gext[slot_] = last_gext; }
    RET(ri->virt->setStyle(ri, (BLContextStyleSlot)(uint32_t)R[2], s, (BLContextStyleTransformMode)(uint32_t)R[9]));
    style_done(&st);
    sync_state(p);
    check_mirror(p, __func__);
}
extern "C" void fle_swap_ctx_set_global_alpha(void) {
    Proxy *p = P_of(R[1]); BLContextImpl *ri = RI(p);
    RET(ri->virt->setGlobalAlpha(ri, X[1].f64[0]));
    sync_state(p);
    check_mirror(p, __func__);
}
extern "C" void fle_swap_ctx_set_comp_op(void) {
    Proxy *p = P_of(R[1]); BLContextImpl *ri = RI(p);
    stat_add(1, (uint32_t)R[2], 0);
    RET(ri->virt->setCompOp(ri, (BLCompOp)(uint32_t)R[2]));
    sync_state(p);
    check_mirror(p, __func__);
}
extern "C" void fle_swap_ctx_set_fill_rule(void) {
    Proxy *p = P_of(R[1]); BLContextImpl *ri = RI(p);
    RET(ri->virt->setFillRule(ri, (BLFillRule)(uint32_t)R[2]));
    sync_state(p);
    check_mirror(p, __func__);
}
extern "C" void fle_swap_ctx_set_stroke_width(void) {
    Proxy *p = P_of(R[1]); BLContextImpl *ri = RI(p);
    RET(ri->virt->setStrokeWidth(ri, X[1].f64[0]));
    sync_state(p);
    check_mirror(p, __func__);
}
extern "C" void fle_swap_ctx_set_stroke_caps(void) {
    Proxy *p = P_of(R[1]); BLContextImpl *ri = RI(p);
    RET(ri->virt->setStrokeCaps(ri, (BLStrokeCap)(uint32_t)R[2]));
    sync_state(p);
    check_mirror(p, __func__);
}
extern "C" void fle_swap_ctx_set_stroke_join(void) {
    Proxy *p = P_of(R[1]); BLContextImpl *ri = RI(p);
    RET(ri->virt->setStrokeJoin(ri, (BLStrokeJoin)(uint32_t)R[2]));
    sync_state(p);
    check_mirror(p, __func__);
}
extern "C" void fle_swap_ctx_clip_to_rect_d(void) {
    Proxy *p = P_of(R[1]); BLContextImpl *ri = RI(p);
    { BLRect rc; memcpy(&rc, G(R[2]), sizeof rc); RET(ri->virt->clipToRectD(ri, &rc)); }
    sync_state(p);
    check_mirror(p, __func__);
}
extern "C" void fle_swap_ctx_restore_clipping(void) {
    Proxy *p = P_of(R[1]); BLContextImpl *ri = RI(p);
    RET(ri->virt->restoreClipping(ri));
    sync_state(p);
    check_mirror(p, __func__);
}
extern "C" void fle_swap_ctx_fill_all_rgba32(void) {
    Proxy *p = P_of(R[1]); BLContextImpl *ri = RI(p);
    stat_add(2, p->gs.compOp, 51);
    DRAW_RET(ri->virt->fillAllRgba32(ri, (uint32_t)R[2]), 51, 0xffffffffu);
    check_mirror(p, __func__);
}
extern "C" void fle_swap_ctx_fill_geometry(void) {
    Proxy *p = P_of(R[1]); BLContextImpl *ri = RI(p); GeomTmp g;
    const uint32_t gt = (uint32_t)R[2];
    stat_add(2, p->gs.compOp, 53);
    geom_from_guest(&g, (uint32_t)R[2], R[8]);
    DRAW_RET(ri->virt->fillGeometry(ri, (BLGeometryType)(uint32_t)R[2], g.data), 53, gt);
    geom_done(&g);
    check_mirror(p, __func__);
}
extern "C" void fle_swap_ctx_fill_geometry_rgba32(void) {
    Proxy *p = P_of(R[1]); BLContextImpl *ri = RI(p); GeomTmp g;
    const uint32_t gt = (uint32_t)R[2];
    stat_add(2, p->gs.compOp, 54);
    geom_from_guest(&g, (uint32_t)R[2], R[8]);
    DRAW_RET(ri->virt->fillGeometryRgba32(ri, (BLGeometryType)(uint32_t)R[2], g.data, (uint32_t)R[9]), 54, gt);
    geom_done(&g);
    check_mirror(p, __func__);
}
extern "C" void fle_swap_ctx_stroke_geometry(void) {
    Proxy *p = P_of(R[1]); BLContextImpl *ri = RI(p); GeomTmp g;
    const uint32_t gt = (uint32_t)R[2];
    stat_add(2, p->gs.compOp, 71);
    geom_from_guest(&g, (uint32_t)R[2], R[8]);
    DRAW_RET(ri->virt->strokeGeometry(ri, (BLGeometryType)(uint32_t)R[2], g.data), 71, gt);
    geom_done(&g);
    check_mirror(p, __func__);
}
extern "C" void fle_swap_ctx_stroke_geometry_rgba32(void) {
    Proxy *p = P_of(R[1]); BLContextImpl *ri = RI(p); GeomTmp g;
    const uint32_t gt = (uint32_t)R[2];
    stat_add(2, p->gs.compOp, 72);
    geom_from_guest(&g, (uint32_t)R[2], R[8]);
    DRAW_RET(ri->virt->strokeGeometryRgba32(ri, (BLGeometryType)(uint32_t)R[2], g.data, (uint32_t)R[9]), 72, gt);
    geom_done(&g);
    check_mirror(p, __func__);
}

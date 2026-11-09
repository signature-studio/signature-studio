/* The drawing surfaces of top-level windows, as the browser side reads them (decision 0128, PROVISIONAL; lift/harness/web/present/CONTRACT.md sections 2 and 3). Harness only, wasm only: compiled and linked by
 * build_nmod_wasm.sh FRUITY=1. NOTHING in lift/runtime is changed for it: the runtime's own pixel model (fle_raster.c, decision 0126) is read through its public functions (fle_raster_surface_peek, fle_raster_surface_count,
 * fle_raster_evidence_seen) and the window table through fle_win_get / fle_win_dc_query, and the one runtime call that every window write goes through (fle_raster_wrote, called by target_wrote of fle_gdi.c) is
 * wrapped by the linker (-Wl,--wrap=fle_raster_wrote): __wrap_fle_raster_wrote calls the real function, then records the write in the surface table.
 *
 * What it holds: the table of CONTRACT.md section 2 (header of 8 words, entries of 28 words since version 2 / decision 0145: the 24 words of version 1 and the four LAYER words, FLSF), the four C functions of section 3, and two EVENTS that the stop condition of the harness and the first
 * non-background pixel (decision 0126's evidence) produce: a line "wf1-surface: ..." on stdout (lid, size, pixel address, number of non-zero pixels, SHA-256 of the surface converted to RGBA with A = 255, the byte order
 * of a canvas read-back and of the SHA-256 of decision 0126's PNGs) and a call of Module.fleSurfaceEvent(kind, lid, pixels, width, height, stride) when the JS side has set it (kind 1 = the first non-background pixel,
 * 2 = the stop; the call is synchronous, inside the engine's stack, so the JS side may copy the bytes at that moment). Which surface is "the one" of the evidence is the surface that the evidence rule names (the surface
 * written when fle_raster_evidence_seen first becomes 1); at the stop every live surface is reported. Mapping of a surface to its logical id: the wrapper receives the address of the surface's view, the table is found by
 * scanning fle_raster_surface_peek(lid) for lid = 0 .. 4095 once per new surface (a window with a larger lid is recorded with lid 0xFFFFFFFF; none is seen in the measurements).
 * Write order (CONTRACT section 4): the pixels are already in memory when the wrapper runs (the real function has returned): "write, then publish". Everything runs on the engine's one JSPI/wasm thread at a time
 * (decision 0124: other JS runs only while the engine is suspended), so the table is changed without atomics; the fields keep the layout of the contract so that a reader on another thread (method C) could be added. */
#ifndef __EMSCRIPTEN__
#error "fruity_surface.c is the wasm build of the FRUITY-on harness (build_nmod_wasm.sh FRUITY=1)"
#endif
#include <emscripten.h>
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_layered.h"
#include "fle_raster.h"
#include "fle_win.h"

void __real_fle_raster_wrote(const fle_rsurf *surf, fle_rrect r, const char *api);

#define CAP 32u
#define F_LIVE 1u
#define F_VISIBLE 2u
#define F_LAYERED 4u      /* flags bit 2 (version 2): the window has WS_EX_LAYERED; layer_mode says whether UpdateLayeredWindow / SetLayeredWindowAttributes was called */
#define LF_ALPHA 1u       /* layer_flags bit 0: layer_alpha is the constant alpha to apply */
#define LF_KEY 2u         /* layer_flags bit 1: layer_key is a colour key (COLORREF): pixels of that colour are transparent */
#define LF_PREMUL 4u      /* layer_flags bit 2: the fourth byte of the surface is an alpha and B, G, R are premultiplied by it */
#define FMT_BGRX8888 1u
#define LID_SCAN_MAX 4096u
#define WS_VISIBLE_BIT 0x10000000u

typedef struct {
    uint32_t lseq, lid, hwnd, flags;
    int32_t x, y;
    uint32_t width, height, stride, format, pixels, layout_gen, owner_lid, style, exstyle, form_key;
    uint32_t dlock, gen, taken_gen, dx0, dy0, dx1, dy1, dirty_first_gen;
    uint32_t layer_mode, layer_alpha, layer_key, layer_flags;   /* words 24-27 (version 2). Meaningful only when flags has F_LAYERED; all 0 otherwise. Guarded by lseq like words 1-15. */
} fle_surface_info_t;

typedef struct {
    uint32_t magic, version, entry_size, capacity, live, doorbell, pad[2];
    fle_surface_info_t e[CAP];
} table_t;

static table_t T;
static const fle_rsurf *view_of[CAP];
static uint32_t used;
static int first_event_done;

EM_JS(void, fle_js_surface_event, (int kind, unsigned lid, unsigned pixels, unsigned w, unsigned h, unsigned stride), {
    if (typeof Module !== 'undefined' && typeof Module.fleSurfaceEvent === 'function') Module.fleSurfaceEvent(kind, lid, pixels, w, h, stride);
});

static void init_table(void) {
    if (T.magic) return;
    T.version = 2u;
    T.entry_size = (uint32_t)sizeof(fle_surface_info_t);
    T.capacity = CAP;
    T.magic = 0x46534c46u;
}

/* ---- SHA-256 (FIPS 180-4), used for the event lines; the pixels are streamed row by row ---- */
typedef struct {
    uint32_t h[8];
    uint8_t buf[64];
    uint64_t len;
    size_t fill;
} sha_t;

static const uint32_t K256[64] = {
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2};
#define ROR(x, n) (((x) >> (n)) | ((x) << (32 - (n))))

static void sha_block(sha_t *s, const uint8_t *p) {
    uint32_t w[64], a, b, c, d, e, f, g, h, t1, t2;
    int i;
    for (i = 0; i < 16; i++) w[i] = (uint32_t)p[4 * i] << 24 | (uint32_t)p[4 * i + 1] << 16 | (uint32_t)p[4 * i + 2] << 8 | p[4 * i + 3];
    for (i = 16; i < 64; i++) {
        uint32_t s0 = ROR(w[i - 15], 7) ^ ROR(w[i - 15], 18) ^ (w[i - 15] >> 3), s1 = ROR(w[i - 2], 17) ^ ROR(w[i - 2], 19) ^ (w[i - 2] >> 10);
        w[i] = w[i - 16] + s0 + w[i - 7] + s1;
    }
    a = s->h[0]; b = s->h[1]; c = s->h[2]; d = s->h[3]; e = s->h[4]; f = s->h[5]; g = s->h[6]; h = s->h[7];
    for (i = 0; i < 64; i++) {
        t1 = h + (ROR(e, 6) ^ ROR(e, 11) ^ ROR(e, 25)) + ((e & f) ^ (~e & g)) + K256[i] + w[i];
        t2 = (ROR(a, 2) ^ ROR(a, 13) ^ ROR(a, 22)) + ((a & b) ^ (a & c) ^ (b & c));
        h = g; g = f; f = e; e = d + t1; d = c; c = b; b = a; a = t1 + t2;
    }
    s->h[0] += a; s->h[1] += b; s->h[2] += c; s->h[3] += d; s->h[4] += e; s->h[5] += f; s->h[6] += g; s->h[7] += h;
}

static void sha_init(sha_t *s) {
    static const uint32_t iv[8] = {0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19};
    memcpy(s->h, iv, sizeof iv);
    s->len = 0;
    s->fill = 0;
}

static void sha_update(sha_t *s, const uint8_t *p, size_t n) {
    s->len += n;
    while (n) {
        size_t k = 64 - s->fill;
        if (k > n) k = n;
        memcpy(s->buf + s->fill, p, k);
        s->fill += k;
        p += k;
        n -= k;
        if (s->fill == 64) {
            sha_block(s, s->buf);
            s->fill = 0;
        }
    }
}

static void sha_final(sha_t *s, char hex[65]) {
    uint64_t bits = s->len * 8u;
    uint8_t pad = 0x80, z = 0, lb[8];
    int i;
    sha_update(s, &pad, 1);
    while (s->fill != 56) sha_update(s, &z, 1);
    for (i = 0; i < 8; i++) lb[i] = (uint8_t)(bits >> (56 - 8 * i));
    sha_update(s, lb, 8);
    for (i = 0; i < 8; i++) snprintf(hex + 8 * i, 9, "%08x", s->h[i]);
}

/* SHA-256 of the surface as RGBA (R, G, B from the BGRX bytes, A = 255): what a canvas read-back gives and what decision 0126's PNGs decode to. */
static void rgba_sha(const uint8_t *bits, uint32_t w, uint32_t h, uint32_t stride, char hex[65], uint64_t *nonzero) {
    sha_t s;
    uint8_t *row = (uint8_t *)malloc((size_t)w * 4u);
    uint64_t nz = 0;
    sha_init(&s);
    if (row) {
        for (uint32_t y = 0; y < h; y++) {
            const uint8_t *p = bits + (uint64_t)y * stride;
            for (uint32_t x = 0; x < w; x++) {
                row[4 * x] = p[4 * x + 2];
                row[4 * x + 1] = p[4 * x + 1];
                row[4 * x + 2] = p[4 * x];
                row[4 * x + 3] = 255;
                if (p[4 * x] | p[4 * x + 1] | p[4 * x + 2]) nz++;
            }
            sha_update(&s, row, (size_t)w * 4u);
        }
        free(row);
    }
    sha_final(&s, hex);
    if (nonzero) *nonzero = nz;
}

/* ---- the table ---- */
static int find_entry(const fle_rsurf *view) {
    for (uint32_t k = 0; k < used; k++)
        if (view_of[k] == view) return (int)k;
    return -1;
}

static uint32_t lid_of_view(const fle_rsurf *view) {
    for (uint32_t l = 0; l < LID_SCAN_MAX; l++)
        if (fle_raster_surface_peek(l) == view) return l;
    return 0xFFFFFFFFu;
}

static void bump(uint32_t *p) { *p += 1u; }

/* Decision 0145: the layered-window words of an entry, from fle_layered_query (runtime/fle_layered.h; the 1:1 map of its fields is in docs/decisions/0145-layered-display.md).
 * layer_mode 0 = WS_EX_LAYERED and neither call yet (the page shows nothing), 1 = UpdateLayeredWindow (the fourth byte of the surface is a premultiplied alpha), 2 = SetLayeredWindowAttributes (BGRX). */
typedef struct {
    uint32_t flag, mode, alpha, key, lflags;
} layer_t;

static layer_t layer_of(const fle_surface_info_t *s) {
    layer_t n = {0, 0, 0, 0, 0};
    fle_layered_info li;
    if (s->lid == 0xFFFFFFFFu || !fle_layered_query(FLE_WIN_HANDLE_BASE + 4u * (uint64_t)s->lid, &li)) return n;
    n.flag = F_LAYERED;
    n.mode = (uint32_t)li.mode;
    n.alpha = li.alpha & 0xFFu;
    if (li.mode == FLE_LAYER_ULW || (li.mode == FLE_LAYER_SLWA && (li.lwa_flags & FLE_LWA_ALPHA))) n.lflags |= LF_ALPHA;
    if (li.mode == FLE_LAYER_SLWA && (li.lwa_flags & FLE_LWA_COLORKEY)) {
        n.lflags |= LF_KEY;
        n.key = li.colorkey & 0xFFFFFFu;
    }
    if (li.mode == FLE_LAYER_ULW && li.per_pixel) n.lflags |= LF_PREMUL;
    return n;
}

static int layer_differs(const fle_surface_info_t *s, const layer_t *n) {
    return (s->flags & F_LAYERED) != n->flag || s->layer_mode != n->mode || s->layer_alpha != n->alpha || s->layer_key != n->key || s->layer_flags != n->lflags;
}

static void layer_store(fle_surface_info_t *s, const layer_t *n) {
    s->flags = (s->flags & ~F_LAYERED) | n->flag;
    s->layer_mode = n->mode;
    s->layer_alpha = n->alpha;
    s->layer_key = n->key;
    s->layer_flags = n->lflags;
}

/* the words are written between two increments of lseq (odd while they change), the way words 1-15 are */
static void layer_sync(fle_surface_info_t *s) {
    layer_t n = layer_of(s);
    if (!layer_differs(s, &n)) return;
    bump(&s->lseq);
    layer_store(s, &n);
    bump(&s->lseq);
}

static void window_fields(fle_surface_info_t *s) {
    fle_win_dcq q;
    fle_win_view v;
    uint64_t h = FLE_WIN_HANDLE_BASE + 4u * (uint64_t)s->lid;
    if (s->lid == 0xFFFFFFFFu || !fle_win_dc_query(h, &q) || q.lid != s->lid || !fle_win_get(h, &v)) return;
    s->hwnd = (uint32_t)h;
    s->x = v.x;
    s->y = v.y;
    s->style = v.style;
    s->exstyle = v.exstyle;
    if (q.visible) s->flags |= F_VISIBLE;
    else s->flags &= ~F_VISIBLE;
}

static void refresh(void) {
    for (uint32_t k = 0; k < used; k++) {
        fle_surface_info_t *s = &T.e[k];
        if ((s->flags & F_LIVE) && s->lid != 0xFFFFFFFFu && fle_raster_surface_peek(s->lid) != view_of[k]) {
            bump(&s->lseq);
            s->flags = 0;
            bump(&s->lseq);
            view_of[k] = NULL; /* the slot is free (flags bit0 = 0, CONTRACT.md section 2) and a new view at the freed address must not find it */
            if (T.live) T.live--;
        }
    }
}

static void note_write(const fle_rsurf *view, fle_rrect r) {
    int k = find_entry(view);
    fle_surface_info_t *s;
    int64_t x0, y0, x1, y1;
    init_table();
    if (k < 0) {
        /* a free slot (flags bit0 = 0: its window is gone) is reused before the table grows: a menu or a dialog is a new window every time it opens, and with 32 slots that are never given back the
         * 33rd window ever drawn (about the 29th opening of a menu) got no surface and was never shown. gen / layout_gen / lseq / dlock only ever go up, lid is never reused (CONTRACT.md section 2). */
        uint32_t f = used;
        refresh(); /* the entries of windows that went since the last idle are free now (the idle hook does this too, but a menu can close and another window open within one idle) */
        for (uint32_t j = 0; j < used; j++)
            if (!(T.e[j].flags & F_LIVE)) { f = j; break; }
        if (f == used) {
            if (used == CAP) return;
            used++;
        }
        k = (int)f;
        view_of[k] = view;
        s = &T.e[k];
        bump(&s->lseq);
        s->lid = lid_of_view(view);
        s->hwnd = 0;
        s->flags = F_LIVE;
        s->x = s->y = 0;
        s->width = s->height = s->stride = s->format = s->pixels = 0; /* so that the new-buffer branch below runs: all dirty, layout_gen up */
        s->owner_lid = s->style = s->exstyle = s->form_key = 0;
        s->dx0 = s->dy0 = s->dx1 = s->dy1 = 0;
        s->dirty_first_gen = 0;
        s->layer_mode = s->layer_alpha = s->layer_key = s->layer_flags = 0;
        bump(&s->lseq);
        T.live++;
    }
    s = &T.e[k];
    if (!(s->flags & F_LIVE)) return;
    if (s->width != view->width || s->height != view->height || s->stride != view->stride || s->pixels != (uint32_t)(uintptr_t)view->bits) {
        bump(&s->lseq);
        s->width = view->width;
        s->height = view->height;
        s->stride = view->stride;
        s->format = FMT_BGRX8888;
        s->pixels = (uint32_t)(uintptr_t)view->bits;
        bump(&s->layout_gen);
        window_fields(s);
        {
            layer_t n = layer_of(s);
            layer_store(s, &n);
        }
        bump(&s->lseq);
        r = (fle_rrect){0, 0, (int64_t)view->width, (int64_t)view->height}; /* a new buffer is all dirty */
    } else {
        window_fields(s);
        layer_sync(s);
    }
    x0 = r.x0 < 0 ? 0 : r.x0;
    y0 = r.y0 < 0 ? 0 : r.y0;
    x1 = r.x1 > (int64_t)s->width ? (int64_t)s->width : r.x1;
    y1 = r.y1 > (int64_t)s->height ? (int64_t)s->height : r.y1;
    bump(&s->gen);
    if (x0 >= x1 || y0 >= y1) return;
    if (!(s->dx0 < s->dx1 && s->dy0 < s->dy1)) {
        s->dx0 = (uint32_t)x0;
        s->dy0 = (uint32_t)y0;
        s->dx1 = (uint32_t)x1;
        s->dy1 = (uint32_t)y1;
        s->dirty_first_gen = s->gen;
    } else {
        if ((uint32_t)x0 < s->dx0) s->dx0 = (uint32_t)x0;
        if ((uint32_t)y0 < s->dy0) s->dy0 = (uint32_t)y0;
        if ((uint32_t)x1 > s->dx1) s->dx1 = (uint32_t)x1;
        if ((uint32_t)y1 > s->dy1) s->dy1 = (uint32_t)y1;
    }
}

__attribute__((export_name("fle_surface_table"))) uint32_t fle_surface_table(void) {
    init_table();
    return (uint32_t)(uintptr_t)&T;
}

__attribute__((export_name("fle_surface_count"))) uint32_t fle_surface_count(void) {
    refresh();
    return T.live;
}

__attribute__((export_name("fle_surface_info"))) int fle_surface_info(uint32_t i, fle_surface_info_t *out) {
    uint32_t n = 0;
    refresh();
    for (uint32_t k = 0; k < used; k++) {
        if (!(T.e[k].flags & F_LIVE)) continue;
        if (n++ != i) continue;
        *out = T.e[k];
        return 0;
    }
    return -1;
}

/* a scratch area for a caller that has no allocator (the node probe of two/surface_probe_fruity.cjs): room for one entry (112 bytes since version 2) and one take result (5 words) */
static uint32_t probe_scratch[40];
__attribute__((export_name("fle_surface_scratch"))) uint32_t fle_surface_scratch(void) { return (uint32_t)(uintptr_t)probe_scratch; }

__attribute__((export_name("fle_surface_take_dirty"))) int fle_surface_take_dirty(uint32_t lid, uint32_t *out) {
    refresh();
    for (uint32_t k = 0; k < used; k++) {
        fle_surface_info_t *s = &T.e[k];
        int nonempty;
        if (!(s->flags & F_LIVE) || s->lid != lid) continue;
        out[0] = s->gen;
        out[1] = s->dx0;
        out[2] = s->dy0;
        out[3] = s->dx1;
        out[4] = s->dy1;
        nonempty = s->dx0 < s->dx1 && s->dy0 < s->dy1;
        s->dx0 = s->dy0 = s->dx1 = s->dy1 = 0;
        s->taken_gen = s->gen;
        return nonempty;
    }
    return -1;
}

/* ---- the events ---- */
extern uint64_t trace_call_count(void);

static void report_surface(int kind, uint32_t lid, const fle_rsurf *v) {
    char hex[65];
    uint64_t nz = 0;
    if (!v || !v->bits || v->bpp != 32) return;
    rgba_sha(v->bits, v->width, v->height, v->stride, hex, &nz);
    printf("wf1-surface: event=%s lid=%u size=%ux%u stride=%u pixels=%#x nonzero=%" PRIu64 " rgba_sha256=%s entries_since_program_start=%" PRIu64 "\n", kind == 1 ? "first-non-background-pixel" : kind == 3 ? "idle" : "stop", (unsigned)lid,
           (unsigned)v->width, (unsigned)v->height, (unsigned)v->stride, (unsigned)(uintptr_t)v->bits, nz, hex, trace_call_count());
    fflush(stdout);
    fle_js_surface_event(kind, lid, (unsigned)(uintptr_t)v->bits, v->width, v->height, v->stride);
}

void __wrap_fle_raster_wrote(const fle_rsurf *surf, fle_rrect r, const char *api) {
    __real_fle_raster_wrote(surf, r, api);
    if (!surf) return;
    note_write(surf, r);
    if (!first_event_done && fle_raster_evidence_seen()) {
        int k = find_entry(surf);
        first_event_done = 1;
        report_surface(1, k >= 0 ? T.e[k].lid : 0xFFFFFFFFu, surf);
    }
}

/* fruity_idle.c, in the engine's idle hook before the engine waits for the host (decision 0145): UpdateLayeredWindow / SetLayeredWindowAttributes change the layer state without a pixel write, or just AFTER
 * the one that fle_raster_wrote reports (fle_layered.c updates its state after the write), so the layer words of the table are brought up to date at every idle, which is where the page reads the table. */
void fruity_surface_idle_sync(void) {
    refresh();
    for (uint32_t k = 0; k < used; k++) {
        if (!(T.e[k].flags & F_LIVE)) continue;
        layer_sync(&T.e[k]);
    }
}

/* fruity_idle.c: the engine is idle (G3): every live surface is reported as event kind 3. */
void fruity_surface_idle_event(unsigned idle_no) {
    (void)idle_no;
    refresh();
    for (uint32_t k = 0; k < used; k++) {
        if (!(T.e[k].flags & F_LIVE)) continue;
        layer_sync(&T.e[k]);
        report_surface(3, T.e[k].lid, view_of[k]);
    }
}

/* fruity.c's report() calls this first when the harness stops: every live surface (the main form's is the one that decision 0126's stop PNG is). */
void fruity_surface_stop_event(void) {
    refresh();
    for (uint32_t k = 0; k < used; k++) {
        if (!(T.e[k].flags & F_LIVE)) continue;
        layer_sync(&T.e[k]);
        report_surface(2, T.e[k].lid, view_of[k]);
    }
}

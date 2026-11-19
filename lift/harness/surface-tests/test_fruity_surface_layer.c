/* The layered-window words of the surface table written by lift/harness/fruity_surface.c (decision 0145), checked natively with the runtime replaced by a few controllable fakes. fruity_surface.c is wasm-only
 * (it needs emscripten); here it is included into this file with a stub <emscripten.h> (test-stubs/) and the functions it asks of the runtime are defined below:
 *   fle_raster_surface_peek / fle_raster_evidence_seen / __real_fle_raster_wrote   the pixel model     fle_win_get / fle_win_dc_query   the window table     fle_layered_query   the layer state of fle_layered.c
 * What is checked: the header (version 2, entry_size 112, capacity), the words 24-27 and flags bit 2 for no layered window / a window nothing was called for / ULW / SLWA with alpha and colour key / a destroyed window,
 * the seqlock (lseq is even after every change and moved only when something changed), that the table is brought up to date by the idle event and not before, and that a version 1 layout is not what is written.
 * usage:  clang -std=gnu11 -O1 -Wall -Wextra -D__EMSCRIPTEN__=1 -I test-stubs -I ../../../runtime -o /tmp/t test_fruity_surface_layer.c && /tmp/t        (exit 0 = all passed) */
#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include "../fruity_surface.c"

/* ---- the fakes ---- */
static fle_rsurf fake_view[3];
static uint32_t fake_pixels[3][4];
static fle_layered_info fake_layer[3];
static int fake_layered[3];        /* WS_EX_LAYERED and live */
static int fake_live[3] = {1, 1, 1};

const fle_rsurf *fle_raster_surface_peek(uint32_t lid) { return lid >= 1 && lid <= 3 && fake_live[lid - 1] ? &fake_view[lid - 1] : NULL; }
int fle_raster_evidence_seen(void) { return 0; }
void __real_fle_raster_wrote(const fle_rsurf *surf, fle_rrect r, const char *api) { (void)surf; (void)r; (void)api; }
uint64_t trace_call_count(void) { return 0; }
int fle_win_dc_query(uint64_t hwnd, fle_win_dcq *q) {
    uint32_t lid = (uint32_t)((hwnd - FLE_WIN_HANDLE_BASE) / 4);
    if (lid < 1 || lid > 3) return 0;
    memset(q, 0, sizeof *q);
    q->lid = lid;
    q->visible = 1;
    return 1;
}
int fle_win_get(uint64_t hwnd, fle_win_view *v) {
    uint32_t lid = (uint32_t)((hwnd - FLE_WIN_HANDLE_BASE) / 4);
    if (lid < 1 || lid > 3) return 0;
    memset(v, 0, sizeof *v);
    v->x = 7; v->y = 9; v->style = 0x90000000u; v->exstyle = fake_layered[lid - 1] ? 0x80000u : 0u;
    return 1;
}
int fle_layered_query(uint64_t hwnd, fle_layered_info *out) {
    uint32_t lid = (uint32_t)((hwnd - FLE_WIN_HANDLE_BASE) / 4);
    if (lid < 1 || lid > 3 || !fake_layered[lid - 1]) return 0;
    *out = fake_layer[lid - 1];
    return 1;
}

/* ---- the checks ---- */
/* the contract's numbers, written out (CONTRACT.md section 2): flags bit 2 = layered; layer_flags bit 0 = alpha valid, bit 1 = key valid, bit 2 = premultiplied 4th byte. Not taken from fruity_surface.c's macros. */
enum { C_LAYERED = 4, C_ALPHA = 1, C_KEY = 2, C_PREMUL = 4 };
static int fails, checks;
#define CHECK(c, ...) do { checks++; if (!(c)) { fails++; printf("FAIL line %d: ", __LINE__); printf(__VA_ARGS__); printf("\n"); } } while (0)

static const fle_surface_info_t *entry_of(uint32_t lid) {
    for (uint32_t k = 0; k < used; k++) if (T.e[k].lid == lid) return &T.e[k];
    return NULL;
}
static void write_to(int i) { /* a pixel write into window lid i+1 */
    fle_rrect r = {0, 0, 4, 4};
    __wrap_fle_raster_wrote(&fake_view[i], r, "test");
}

int main(void) {
    for (int i = 0; i < 3; i++) {
        fake_view[i].bits = (uint8_t *)fake_pixels[i]; fake_view[i].width = 4; fake_view[i].height = 4; fake_view[i].stride = 16; fake_view[i].bpp = 32;
    }
    (void)fle_surface_table();   /* returns a wasm (32-bit) address; natively the table is read through T itself */
    const table_t *t = &T;
    CHECK(t->magic == 0x46534c46u, "magic %#x", t->magic);
    CHECK(t->version == 2u, "version %u", t->version);
    CHECK(t->entry_size == 112u && sizeof(fle_surface_info_t) == 112u, "entry_size %u sizeof %zu", t->entry_size, sizeof(fle_surface_info_t));
    CHECK(t->capacity == CAP, "capacity %u", t->capacity);
    CHECK(F_LAYERED == C_LAYERED && LF_ALPHA == C_ALPHA && LF_KEY == C_KEY && LF_PREMUL == C_PREMUL, "bit values %u %u %u %u", F_LAYERED, LF_ALPHA, LF_KEY, LF_PREMUL);
    CHECK(offsetof(fle_surface_info_t, layer_mode) == 96 && offsetof(fle_surface_info_t, layer_alpha) == 100 && offsetof(fle_surface_info_t, layer_key) == 104 && offsetof(fle_surface_info_t, layer_flags) == 108,
          "layer words at %zu %zu %zu %zu (words 24-27 = bytes 96-108)", offsetof(fle_surface_info_t, layer_mode), offsetof(fle_surface_info_t, layer_alpha), offsetof(fle_surface_info_t, layer_key), offsetof(fle_surface_info_t, layer_flags));

    /* lid 1: an ordinary window; lid 2: layered, nothing called; lid 3: layered (ULW) */
    fake_layered[1] = 1; fake_layer[1].mode = FLE_LAYER_NONE; fake_layer[1].alpha = 255;
    write_to(0); write_to(1); write_to(2);
    const fle_surface_info_t *e1 = entry_of(1), *e2 = entry_of(2), *e3 = entry_of(3);
    CHECK(e1 && e2 && e3, "entries %p %p %p", (void *)e1, (void *)e2, (void *)e3);
    CHECK(!(e1->flags & C_LAYERED) && e1->layer_mode == 0 && e1->layer_alpha == 0 && e1->layer_key == 0 && e1->layer_flags == 0, "ordinary window: flags %#x words %u %u %u %u", e1->flags, e1->layer_mode, e1->layer_alpha, e1->layer_key, e1->layer_flags);
    CHECK((e1->flags & (F_LIVE | F_VISIBLE)) == (F_LIVE | F_VISIBLE), "ordinary window keeps its live/visible bits: %#x", e1->flags);
    CHECK((e2->flags & C_LAYERED) && e2->layer_mode == 0 && e2->layer_alpha == 255 && e2->layer_flags == 0, "layered, nothing called: flags %#x mode %u alpha %u lflags %u", e2->flags, e2->layer_mode, e2->layer_alpha, e2->layer_flags);
    CHECK((e2->flags & (F_LIVE | F_VISIBLE)) == (F_LIVE | F_VISIBLE), "layered window keeps its live/visible bits: %#x", e2->flags);
    CHECK(!(e3->flags & C_LAYERED), "lid 3 is not layered yet");

    /* lid 3 becomes a ULW window with SourceConstantAlpha 128 and per-pixel alpha; the table is not touched until the next write or idle */
    fake_layered[2] = 1; fake_layer[2].mode = FLE_LAYER_ULW; fake_layer[2].alpha = 128; fake_layer[2].per_pixel = 1; fake_layer[2].colorkey = 0;
    CHECK(!(e3->flags & C_LAYERED) && e3->layer_mode == 0, "the table does not change by itself");
    uint32_t lseq3 = e3->lseq;
    fruity_surface_idle_sync();   /* what the engine's idle hook calls (fruity_idle.c) */
    CHECK((e3->flags & C_LAYERED) && e3->layer_mode == 1 && e3->layer_alpha == 128 && e3->layer_key == 0 && e3->layer_flags == (C_ALPHA | C_PREMUL), "ULW after idle: flags %#x words %u %u %u %u", e3->flags, e3->layer_mode, e3->layer_alpha, e3->layer_key, e3->layer_flags);
    CHECK((e3->lseq & 1u) == 0 && e3->lseq == lseq3 + 2u, "lseq even and moved by one change: %u -> %u", lseq3, e3->lseq);
    uint32_t lseq3b = e3->lseq, lseq1 = e1->lseq;
    fruity_surface_idle_sync();
    fruity_surface_idle_event(2);
    CHECK(e3->lseq == lseq3b && e1->lseq == lseq1, "an idle with nothing changed does not touch lseq");

    /* SLWA with LWA_ALPHA | LWA_COLORKEY */
    fake_layer[2].mode = FLE_LAYER_SLWA; fake_layer[2].lwa_flags = FLE_LWA_ALPHA | FLE_LWA_COLORKEY; fake_layer[2].alpha = 77; fake_layer[2].colorkey = 0x00FF00FFu; fake_layer[2].per_pixel = 0;
    write_to(2);   /* a pixel write brings the words up to date as well */
    CHECK(e3->layer_mode == 2 && e3->layer_alpha == 77 && e3->layer_key == 0x00FF00FFu && e3->layer_flags == (C_ALPHA | C_KEY), "SLWA after a write: words %u %u %#x %u", e3->layer_mode, e3->layer_alpha, e3->layer_key, e3->layer_flags);
    CHECK((e3->lseq & 1u) == 0, "lseq even after the write");
    /* SLWA with the colour key only: alpha is 255 and not valid */
    fake_layer[2].lwa_flags = FLE_LWA_COLORKEY; fake_layer[2].alpha = 255; fake_layer[2].colorkey = 0x00123456u;
    fruity_surface_idle_sync();
    CHECK(e3->layer_mode == 2 && e3->layer_alpha == 255 && e3->layer_key == 0x00123456u && e3->layer_flags == C_KEY, "SLWA key only: words %u %u %#x %u", e3->layer_mode, e3->layer_alpha, e3->layer_key, e3->layer_flags);
    /* SLWA with LWA_ALPHA only: the key word is 0 and not valid */
    fake_layer[2].lwa_flags = FLE_LWA_ALPHA; fake_layer[2].alpha = 0; fake_layer[2].colorkey = 0;
    fruity_surface_idle_event(4);
    CHECK(e3->layer_mode == 2 && e3->layer_alpha == 0 && e3->layer_key == 0 && e3->layer_flags == C_ALPHA, "SLWA alpha 0: words %u %u %#x %u", e3->layer_mode, e3->layer_alpha, e3->layer_key, e3->layer_flags);
    /* ULW without per-pixel alpha bytes (per_pixel 0): not premultiplied */
    fake_layer[2].mode = FLE_LAYER_ULW; fake_layer[2].lwa_flags = 0; fake_layer[2].alpha = 255; fake_layer[2].per_pixel = 0;
    fruity_surface_idle_event(5);
    CHECK(e3->layer_mode == 1 && e3->layer_alpha == 255 && e3->layer_flags == C_ALPHA, "ULW without per-pixel alpha: words %u %u %u", e3->layer_mode, e3->layer_alpha, e3->layer_flags);
    /* the colour key of a ULW window is baked into the surface: the word is 0 and the key bit is clear, whatever else the layer state holds */
    fake_layer[2].per_pixel = 1; fake_layer[2].colorkey = 0xABCDEFu; fake_layer[2].lwa_flags = FLE_LWA_COLORKEY | FLE_LWA_ALPHA;
    fruity_surface_idle_event(6);
    CHECK(e3->layer_key == 0 && (e3->layer_flags & C_KEY) == 0 && e3->layer_flags == (C_ALPHA | C_PREMUL), "a ULW window's key word is 0 (the key is baked into the pixels): key %#x flags %u", e3->layer_key, e3->layer_flags);
    /* the words keep 24 bits of the key and 8 bits of the alpha even when the state holds more */
    fake_layer[2].mode = FLE_LAYER_SLWA; fake_layer[2].lwa_flags = FLE_LWA_COLORKEY | FLE_LWA_ALPHA; fake_layer[2].colorkey = 0xAB123456u; fake_layer[2].alpha = 0x1FFu; fake_layer[2].per_pixel = 0;
    fruity_surface_idle_event(6);
    CHECK(e3->layer_key == 0x123456u && e3->layer_alpha == 0xFFu && e3->layer_flags == (C_ALPHA | C_KEY), "24-bit key and 8-bit alpha: key %#x alpha %#x flags %u", e3->layer_key, e3->layer_alpha, e3->layer_flags);
    fake_layer[2].mode = FLE_LAYER_ULW; fake_layer[2].lwa_flags = 0; fake_layer[2].per_pixel = 1; fake_layer[2].alpha = 255; fake_layer[2].colorkey = 0;
    fruity_surface_idle_event(6);

    /* the style bit cleared (or the window destroyed): the words go back to 0 */
    fake_layered[2] = 0;
    fruity_surface_stop_event();
    CHECK(!(e3->flags & C_LAYERED) && e3->layer_mode == 0 && e3->layer_alpha == 0 && e3->layer_key == 0 && e3->layer_flags == 0, "no longer layered: flags %#x words %u %u %u %u", e3->flags, e3->layer_mode, e3->layer_alpha, e3->layer_key, e3->layer_flags);
    CHECK((e3->flags & (F_LIVE | F_VISIBLE)) == (F_LIVE | F_VISIBLE), "live/visible survive: %#x", e3->flags);

    /* fle_surface_info copies a whole 112-byte entry, the layer words included; the scratch area holds one entry and a take result */
    fake_layered[2] = 1; fake_layer[2].mode = FLE_LAYER_ULW; fake_layer[2].alpha = 200; fake_layer[2].per_pixel = 1;
    fruity_surface_idle_event(7);
    fle_surface_info_t copy;
    int rc = fle_surface_info(2, &copy);   /* the third live entry = lid 3 */
    CHECK(rc == 0 && copy.lid == 3 && copy.layer_mode == 1 && copy.layer_alpha == 200 && copy.layer_flags == (C_ALPHA | C_PREMUL), "fle_surface_info: rc %d lid %u words %u %u %u", rc, copy.lid, copy.layer_mode, copy.layer_alpha, copy.layer_flags);
    CHECK(sizeof(probe_scratch) >= sizeof(fle_surface_info_t) + 5 * sizeof(uint32_t), "scratch %zu bytes holds an entry (%zu) and a take result (20)", sizeof(probe_scratch), sizeof(fle_surface_info_t));

    printf("%s: %d checks, %d failed\n", fails ? "FAILED" : "ok", checks, fails);
    return fails ? 1 : 0;
}

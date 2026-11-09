//! Tests of decision 0223 (docs/decisions/0223-desktop-compositor.md, PROVISIONAL): the desktop compositor of the runtime. The window table, the surfaces and the layered-window
//! state are STUBS here (the compositor only reads them); what is tested is what it draws and what it reports as changed.
//!
//! Expected values are DERIVED BY HAND from Microsoft Learn, not read back from the C code:
//!   Window Features / Z Order: "A window higher in the z-order is displayed over windows lower in it" (the top-most is drawn last).
//!   Window Features / Layered Windows: "the layered window will not become visible until SetLayeredWindowAttributes or UpdateLayeredWindow has been called".
//!   SetLayeredWindowAttributes: LWA_ALPHA "Dst = Src * bAlpha/255 + Dst * (1 - bAlpha/255)"; LWA_COLORKEY "all pixels painted by the window in this color will be transparent".
//!   BLENDFUNCTION / UpdateLayeredWindow: "Src = Src * SourceConstantAlpha / 255" in all four channels (AC_SRC_ALPHA: premultiplied), then "Dst = Src + (1 - Src.A) * Dst".
//! A rounded division x / 255 takes the nearest integer, halves up (the compositor's div255). The desktop background is (5, 7, 10), the colour of the page behind the windows before this compositor (S-desk-bg).
//! Nothing is compared with a real Windows.

mod etxt_common;

use std::path::PathBuf;
use std::process::Command;

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn rt_dir() -> PathBuf {
    manifest().join("../../runtime")
}
fn src_dir() -> PathBuf {
    if let Ok(d) = std::env::var("FLE_D1_SRC_DIR") {
        return PathBuf::from(d);
    }
    rt_dir()
}
fn work_dir() -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("desk1_compose");
    std::fs::create_dir_all(&d).unwrap();
    d
}

const PROG: &str = r##"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "fle_desk.h"
#include "fle_input.h"
#include "fle_layered.h"
#include "fle_raster.h"
#include "fle_screen.h"
#include "fle_win.h"
#include "fle_class.h"

/* ---- the stubs: the window table, the surfaces, the layered state ---- */
typedef struct { uint32_t lid; int visible; int32_t x, y; uint32_t w, h; fle_rsurf s; fle_layered_info li; int layered; } win_t;
static win_t W[8];
static unsigned order[8];          /* lids, top first */
static unsigned norder;

unsigned fle_win_input_toplevels(uint64_t *h, unsigned max) { unsigned n = 0; for (; n < norder && n < max; n++) h[n] = 0x1000 + order[n]; return n; }
int fle_win_input_view(uint64_t hwnd, fle_win_iview *v) {
    win_t *w = &W[hwnd - 0x1000];
    memset(v, 0, sizeof *v); v->live = 1; v->visible = w->visible; v->ox = w->x; v->oy = w->y; v->cx = (int32_t)w->w; v->cy = (int32_t)w->h; v->lid = w->lid; return 1;
}
const fle_rsurf *fle_raster_surface_peek(uint32_t lid) { return W[lid].w ? &W[lid].s : NULL; }
/* desk2: the class name and the title of a window, as fle_win_get and fle_class_name_of give them (the compositor prints them in fle_desk_describe) */
static const char *CLS_OF[8], *TTL_OF[8];
int fle_win_get(uint64_t hwnd, fle_win_view *v) {
    win_t *w = &W[hwnd - 0x1000]; const char *s = TTL_OF[w->lid];
    memset(v, 0, sizeof *v); v->live = 1; v->atom = (uint16_t)(0x100 + w->lid);
    for (; s && *s; s++) v->text[v->text_len++] = (unsigned char)*s;
    if (w->lid == 3) { v->text_len = 0; v->text[v->text_len++] = 'a'; v->text[v->text_len++] = '"'; v->text[v->text_len++] = '\\'; v->text[v->text_len++] = 0x3042; v->text[v->text_len++] = 'z'; }
    return 1;
}
int fle_class_name_of(uint16_t atom, uint16_t *out, uint16_t *len) {
    const char *c = CLS_OF[atom - 0x100]; uint16_t n = 0;
    if (!c) return 0;
    for (; *c; c++) out[n++] = (unsigned char)*c;
    *len = n; return 1;
}
int fle_layered_query(uint64_t hwnd, fle_layered_info *out) { win_t *w = &W[hwnd - 0x1000]; if (!w->layered) return 0; *out = w->li; return 1; }

static int checks, failed;
#define CHECK(c, ...) do { checks++; if (!(c)) { failed++; printf("FAIL line %d: ", __LINE__); printf(__VA_ARGS__); printf("\n"); } } while (0)

static void mkwin(unsigned lid, int32_t x, int32_t y, uint32_t w, uint32_t h, uint8_t b, uint8_t g, uint8_t r) {
    win_t *p = &W[lid]; memset(p, 0, sizeof *p);
    p->lid = lid; p->visible = 1; p->x = x; p->y = y; p->w = w; p->h = h;
    p->s.bits = calloc(1, (size_t)w * h * 4); p->s.width = w; p->s.height = h; p->s.stride = w * 4; p->s.bpp = 32;
    for (uint32_t i = 0; i < w * h; i++) { p->s.bits[4 * i] = b; p->s.bits[4 * i + 1] = g; p->s.bits[4 * i + 2] = r; p->s.bits[4 * i + 3] = 0; }
}
static void setpx(unsigned lid, uint32_t x, uint32_t y, uint8_t b, uint8_t g, uint8_t r, uint8_t a) { uint8_t *p = W[lid].s.bits + ((size_t)y * W[lid].w + x) * 4; p[0] = b; p[1] = g; p[2] = r; p[3] = a; }
static void fillrect(unsigned lid, uint32_t x0, uint32_t y0, uint32_t x1, uint32_t y1, uint8_t b, uint8_t g, uint8_t r) { for (uint32_t y = y0; y < y1; y++) for (uint32_t x = x0; x < x1; x++) setpx(lid, x, y, b, g, r, 0); }
static const uint8_t *pix(const fle_desk_state *st, int x, int y) { return fle_desk_picture() + ((size_t)y * st->width + (size_t)x) * 4; }
static int px_is(const fle_desk_state *st, int x, int y, int r, int g, int b) { const uint8_t *p = pix(st, x, y); return p[0] == r && p[1] == g && p[2] == b && p[3] == 255; }
static int has_rect(const fle_desk_state *st, int x0, int y0, int x1, int y1) {
    for (uint32_t i = 0; i < st->n_rects; i++) { const fle_desk_rect *r = &st->rects[i]; if (r->x0 <= x0 && r->y0 <= y0 && r->x1 >= x1 && r->y1 >= y1) return 1; }
    return 0;
}
static int rect_count_over(const fle_desk_state *st, int64_t area) { int64_t a = 0; for (uint32_t i = 0; i < st->n_rects; i++) a += (int64_t)(st->rects[i].x1 - st->rects[i].x0) * (st->rects[i].y1 - st->rects[i].y0); return a > area; }

int main(void) {
    const fle_desk_state *st;

    /* 1. an empty desktop: the whole picture, the background colour; then nothing changed */
    st = fle_desk_compose();
    CHECK(st && st->magic == FLE_DESK_MAGIC && st->version == FLE_DESK_VERSION, "header");
    CHECK(st->width == 1920 && st->height == 1080 && st->stride == 7680, "size %u x %u stride %u", st->width, st->height, st->stride);
    CHECK((st->flags & 1u) && st->n_rects == 1 && has_rect(st, 0, 0, 1920, 1080), "first call reports the whole picture (%u rects, flags %u)", st->n_rects, st->flags);
    CHECK(px_is(st, 0, 0, 5, 7, 10) && px_is(st, 1919, 1079, 5, 7, 10) && px_is(st, 900, 500, 5, 7, 10), "background");
    st = fle_desk_compose();
    CHECK(st->n_rects == 0 && !(st->flags & 1u), "nothing changed: %u rects", st->n_rects);

    /* 2. one opaque window (BGRX 0x11 0x22 0x33 -> R 0x33 G 0x22 B 0x11): its rectangle, nothing outside */
    mkwin(1, 10, 20, 100, 50, 0x11, 0x22, 0x33);
    order[0] = 1; norder = 1;
    st = fle_desk_compose();
    CHECK(st->n_rects == 1 && has_rect(st, 10, 20, 110, 70) && !rect_count_over(st, 100 * 50), "a new window reports its own rectangle (%u rects)", st->n_rects);
    CHECK(px_is(st, 10, 20, 0x33, 0x22, 0x11) && px_is(st, 109, 69, 0x33, 0x22, 0x11) && px_is(st, 9, 20, 5, 7, 10) && px_is(st, 110, 69, 5, 7, 10) && px_is(st, 10, 70, 5, 7, 10), "window pixels and its edges");
    CHECK(st->n_windows == 1, "windows drawn %u", st->n_windows);

    /* 2b. desk2: fle_desk_describe names each window by its class and title (printable ASCII; the quote, the backslash and a non-ASCII unit become '?') */
    CLS_OF[1] = "TFruityLoopsMainForm"; TTL_OF[1] = "FL Studio 2026";
    st = fle_desk_compose();
    {
        const char *d = fle_desk_describe();
        CHECK(strstr(d, "lid 1 at (10, 20) 100x50") && strstr(d, "class \"TFruityLoopsMainForm\" title \"FL Studio 2026\""), "describe names the class and the title: %s", d);
    }
    mkwin(3, 300, 300, 40, 30, 1, 2, 3); CLS_OF[3] = "T\"Odd"; order[1] = order[0]; order[0] = 3; norder = 2;
    st = fle_desk_compose();
    {
        const char *d = fle_desk_describe();
        CHECK(strstr(d, "class \"T?Odd\" title \"a???z\""), "describe makes the quote, the backslash and a non-ASCII unit '?': %s", d);
    }
    norder = 1; order[0] = 1; W[3].visible = 0; CLS_OF[3] = NULL;
    st = fle_desk_compose();
    /* 3. a pixel written inside (fle_desk_wrote): only that rectangle is reported, and the picture has the new colour; the 4th byte is not an alpha */
    setpx(1, 5, 6, 1, 2, 3, 99);
    fle_desk_wrote(1, 5, 6, 6, 7);
    st = fle_desk_compose();
    CHECK(st->n_rects == 1 && has_rect(st, 15, 26, 16, 27) && !rect_count_over(st, 1), "dirty rectangle (%u rects)", st->n_rects);
    CHECK(px_is(st, 15, 26, 3, 2, 1) && px_is(st, 14, 26, 0x33, 0x22, 0x11), "the written pixel is R3 G2 B1, its neighbour is not touched, A is 255 whatever the 4th byte");

    /* 4. a rectangle that lies partly outside the window is clipped to the window; one wholly outside is nothing */
    fillrect(1, 90, 40, 100, 50, 7, 8, 9);
    fle_desk_wrote(1, 90, 40, 500, 500);
    st = fle_desk_compose();
    CHECK(st->n_rects == 1 && has_rect(st, 100, 60, 110, 70) && !rect_count_over(st, 100), "clipped to the window (%u rects)", st->n_rects);
    fle_desk_wrote(1, 200, 200, 300, 300);
    st = fle_desk_compose();
    CHECK(st->n_rects == 0, "outside the window: nothing");

    /* 4b. the same pixels written again are not reported (the engine's idle redraw); of a rectangle in which only a corner changed, the bounding rectangle of the change is */
    fle_desk_wrote(1, 0, 0, 100, 50);
    st = fle_desk_compose();
    CHECK(st->n_rects == 0 && st->gen != 0, "the same pixels again: nothing to report (%u rects)", st->n_rects);
    setpx(1, 20, 10, 1, 1, 1, 0);
    setpx(1, 22, 12, 2, 2, 2, 0);
    fle_desk_wrote(1, 0, 0, 100, 50);
    st = fle_desk_compose();
    CHECK(st->n_rects == 1 && has_rect(st, 30, 30, 33, 33) && !rect_count_over(st, 9), "only the changed pixels' bounding rectangle: %u rects", st->n_rects);
    CHECK(px_is(st, 30, 30, 1, 1, 1) && px_is(st, 32, 32, 2, 2, 2), "and the picture has them");

    /* 5. two windows: the one higher in the z-order is drawn over the one below (order[] is top first); restacking redraws the overlap */
    mkwin(2, 50, 40, 100, 50, 0x44, 0x55, 0x66);
    order[0] = 2; order[1] = 1; norder = 2;
    st = fle_desk_compose();
    CHECK(px_is(st, 60, 50, 0x66, 0x55, 0x44) && px_is(st, 49, 50, 0x33, 0x22, 0x11), "window 2 over window 1");
    order[0] = 1; order[1] = 2;
    st = fle_desk_compose();
    CHECK(has_rect(st, 50, 40, 110, 70), "restacking reports at least the overlapped windows' rectangles (%u rects)", st->n_rects);
    CHECK(px_is(st, 60, 50, 0x33, 0x22, 0x11) && px_is(st, 120, 50, 0x66, 0x55, 0x44), "window 1 over window 2 now; window 2 alone elsewhere");

    /* 6. a moved window: the old place shows what was under it, the new place the window */
    W[2].x = 400; W[2].y = 300;
    st = fle_desk_compose();
    CHECK(has_rect(st, 50, 40, 150, 90) && has_rect(st, 400, 300, 500, 350), "old and new rectangles (%u rects)", st->n_rects);
    CHECK(px_is(st, 120, 50, 5, 7, 10) && px_is(st, 60, 50, 0x33, 0x22, 0x11) && px_is(st, 400, 300, 0x66, 0x55, 0x44), "old place: background where window 1 is not, window 1 elsewhere; new place");

    /* 7. a hidden window is not drawn, and the place it had shows what is below; shown again it comes back */
    W[2].visible = 0;
    st = fle_desk_compose();
    CHECK(px_is(st, 400, 300, 5, 7, 10) && has_rect(st, 400, 300, 500, 350) && st->n_windows == 1, "hidden");
    W[2].visible = 1;
    st = fle_desk_compose();
    CHECK(px_is(st, 400, 300, 0x66, 0x55, 0x44) && has_rect(st, 400, 300, 500, 350), "shown again");

    /* 8. a window partly off the screen is clipped (the main window is 1930 x 1092 at (-5, -6)) */
    mkwin(3, -5, -6, 1930, 1092, 0x10, 0x20, 0x30);
    order[0] = 3; order[1] = 1; order[2] = 2; norder = 3;
    st = fle_desk_compose();
    CHECK(has_rect(st, 0, 0, 1920, 1080), "the window covers the screen");
    CHECK(px_is(st, 0, 0, 0x30, 0x20, 0x10) && px_is(st, 1919, 1079, 0x30, 0x20, 0x10), "corners");
    setpx(3, 5, 6, 9, 8, 7, 0);
    fle_desk_wrote(3, 5, 6, 6, 7);
    st = fle_desk_compose();
    CHECK(has_rect(st, 0, 0, 1, 1) && px_is(st, 0, 0, 7, 8, 9), "surface pixel (5, 6) is screen pixel (0, 0)");
    W[3].visible = 0;
    fle_desk_compose();

    /* 9. SetLayeredWindowAttributes + LWA_ALPHA 128 over the background (5, 7, 10): src (200, 100, 50) -> R round(25600+635 = 26235 / 255) = 103, G round(12800+889 = 13689 / 255) = 54, B round(6400+1270 = 7670 / 255) = 30 */
    mkwin(4, 600, 600, 20, 20, 50, 100, 200);
    W[4].layered = 1; W[4].li.mode = FLE_LAYER_SLWA; W[4].li.lwa_flags = FLE_LWA_ALPHA; W[4].li.alpha = 128;
    order[0] = 4; norder = 1;
    st = fle_desk_compose();
    CHECK(px_is(st, 600, 600, 103, 54, 30), "LWA_ALPHA 128: got %d %d %d", pix(st, 600, 600)[0], pix(st, 600, 600)[1], pix(st, 600, 600)[2]);
    /* alpha 255 + no key: opaque copy */
    W[4].li.alpha = 255;
    st = fle_desk_compose();
    CHECK(px_is(st, 600, 600, 200, 100, 50), "LWA_ALPHA 255 is opaque");

    /* 10. a layered window that neither call has been made for is not shown */
    W[4].li.mode = FLE_LAYER_NONE;
    st = fle_desk_compose();
    CHECK(px_is(st, 600, 600, 5, 7, 10) && st->n_windows == 0, "layered, not set up: not shown");

    /* 11. LWA_COLORKEY: pixels of the key (COLORREF 0x00BBGGRR = R 200 G 100 B 50 -> 0x326 4C8) are transparent; the others are opaque */
    W[4].li.mode = FLE_LAYER_SLWA; W[4].li.lwa_flags = FLE_LWA_COLORKEY; W[4].li.alpha = 255; W[4].li.colorkey = (50u << 16) | (100u << 8) | 200u;
    setpx(4, 1, 1, 7, 8, 9, 0);
    fle_desk_wrote(4, 0, 0, 20, 20);
    st = fle_desk_compose();
    CHECK(px_is(st, 600, 600, 5, 7, 10) && px_is(st, 601, 601, 9, 8, 7), "colour key: keyed pixel shows the background, another pixel is opaque");

    /* 12. UpdateLayeredWindow, premultiplied: B 0 G 0 R 128 A 128, SCA 255: Dst = Src + (1 - A/255) * Dst -> R 128 + round(127*5/255 = 2.49) = 130, G round(127*7/255 = 3.49) = 3, B round(127*10/255 = 4.98) = 5 */
    memset(&W[4].li, 0, sizeof W[4].li);
    W[4].li.mode = FLE_LAYER_ULW; W[4].li.alpha = 255; W[4].li.per_pixel = 1;
    for (int y = 0; y < 20; y++) for (int x = 0; x < 20; x++) setpx(4, x, y, 0, 0, 128, 128);
    fle_desk_wrote(4, 0, 0, 20, 20);
    st = fle_desk_compose();
    CHECK(px_is(st, 610, 610, 130, 3, 5), "ULW premultiplied: got %d %d %d", pix(st, 610, 610)[0], pix(st, 610, 610)[1], pix(st, 610, 610)[2]);
    /* with SourceConstantAlpha 128: Src' = Src * 128 / 255: R round(128*128/255 = 64.25) = 64, A' = 64; Dst = Src' + (255 - 64) * Dst / 255: R 64 + round(191*5/255 = 3.75) = 68, G round(191*7/255 = 5.24) = 5, B round(191*10/255 = 7.49) = 7 */
    W[4].li.alpha = 128;
    st = fle_desk_compose();
    CHECK(px_is(st, 610, 610, 68, 5, 7), "ULW premultiplied with SCA 128: got %d %d %d", pix(st, 610, 610)[0], pix(st, 610, 610)[1], pix(st, 610, 610)[2]);
    /* a fully transparent premultiplied pixel leaves the destination alone */
    setpx(4, 3, 3, 0, 0, 0, 0);
    fle_desk_wrote(4, 3, 3, 4, 4);
    st = fle_desk_compose();
    CHECK(px_is(st, 603, 603, 5, 7, 10), "A = 0: the background shows");
    /* an opaque pixel (A 255) with SCA 255 replaces */
    W[4].li.alpha = 255;
    setpx(4, 4, 4, 30, 20, 10, 255);
    fle_desk_wrote(4, 4, 4, 5, 5);
    st = fle_desk_compose();
    CHECK(px_is(st, 604, 604, 10, 20, 30), "A = 255: replaces");

    /* 13. many small writes: the list stays within its capacity and still covers every one of them */
    mkwin(5, 0, 0, 1900, 1000, 1, 2, 3);
    order[0] = 5; norder = 1;
    fle_desk_compose();
    for (int i = 0; i < 100; i++) { setpx(5, (uint32_t)(i * 17 % 1800), (uint32_t)(i * 31 % 900), 4, 5, 6, 0); fle_desk_wrote(5, i * 17 % 1800, i * 31 % 900, i * 17 % 1800 + 1, i * 31 % 900 + 1); }
    st = fle_desk_compose();
    CHECK(st->n_rects >= 1 && st->n_rects <= FLE_DESK_MAX_RECTS, "rects %u", st->n_rects);
    {
        int all = 1;
        for (int i = 0; i < 100; i++) all &= has_rect(st, i * 17 % 1800, i * 31 % 900, i * 17 % 1800 + 1, i * 31 % 900 + 1) && px_is(st, i * 17 % 1800, i * 31 % 900, 6, 5, 4);
        CHECK(all, "all 100 written pixels are covered by the reported rectangles and drawn");
    }

    /* 14. resync reports the whole picture again */
    fle_desk_resync();
    st = fle_desk_compose();
    CHECK((st->flags & 1u) && has_rect(st, 0, 0, 1920, 1080), "resync");

    /* 15. a display change (decision 0212): the picture takes the size of the new screen and is reported whole; windows beyond it are clipped */
    CHECK(fle_screen_set(800, 600) == 1, "fle_screen_set");
    st = fle_desk_compose();
    CHECK(st->width == 800 && st->height == 600 && st->stride == 3200, "new size %u x %u", st->width, st->height);
    CHECK((st->flags & 1u) && st->n_rects == 1 && has_rect(st, 0, 0, 800, 600), "the whole new picture is reported");
    CHECK(px_is(st, 799, 599, 3, 2, 1) && px_is(st, 400, 300, 3, 2, 1), "window 5 is drawn on the smaller screen");
    fillrect(5, 790, 590, 800, 600, 20, 21, 22);
    fle_desk_wrote(5, 790, 590, 1000, 1000);
    st = fle_desk_compose();
    CHECK(st->n_rects == 1 && has_rect(st, 790, 590, 800, 600) && !rect_count_over(st, 100), "a write beyond the screen is clipped to it (%u rects)", st->n_rects);

    /* 16. decision 0215 (sweep1): UpdateLayeredWindow with prcDirty writes only that rectangle of the layered window's surface and reports only that rectangle; the compositor draws and reports exactly it, and the rest of the window is not touched */
    mkwin(6, 100, 100, 60, 40, 0, 0, 0);       /* every surface pixel (0, 0, 0, 0): premultiplied, fully transparent */
    W[6].layered = 1; W[6].li.mode = FLE_LAYER_ULW; W[6].li.alpha = 255; W[6].li.per_pixel = 1;
    for (unsigned k = norder; k > 0; k--) order[k] = order[k - 1];
    order[0] = 6; norder++;
    st = fle_desk_compose();
    st = fle_desk_compose();
    {
        uint8_t keep[4];
        memcpy(keep, pix(st, 101, 101), 4);
        setpx(6, 30, 20, 0, 0, 255, 255);          /* surface pixel (30, 20): premultiplied opaque red */
        fle_desk_wrote(6, 30, 20, 31, 21);
        st = fle_desk_compose();
        CHECK(st->n_rects == 1 && has_rect(st, 130, 120, 131, 121) && !rect_count_over(st, 1), "a one-pixel dirty rectangle of a layered window is one pixel of the picture (%u rects)", st->n_rects);
        CHECK(px_is(st, 130, 120, 255, 0, 0), "and that pixel is the opaque red");
        CHECK(memcmp(keep, pix(st, 101, 101), 4) == 0, "the rest of the layered window is not touched");
        setpx(6, 10, 10, 0, 0, 64, 128);           /* premultiplied: R 64 at A 128 = red at half alpha */
        fle_desk_wrote(6, 10, 10, 11, 11);
        st = fle_desk_compose();
        CHECK(st->n_rects == 1 && has_rect(st, 110, 110, 111, 111) && !rect_count_over(st, 1), "a second dirty rectangle: only that pixel (%u rects)", st->n_rects);
    }

    printf("desk1: %d checks, %d failed\n", checks, failed);
    return failed ? 1 : 0;
}
"##;

#[test]
fn compositor_draws_what_the_window_table_says_and_reports_what_changed() {
    let dir = work_dir();
    let tag = etxt_common::uniq("desk1");
    let src = dir.join(format!("{tag}.c"));
    let exe = dir.join(format!("{tag}.out"));
    std::fs::write(&src, PROG).unwrap();
    let desk = src_dir().join("fle_desk.c");
    let cc = Command::new("clang")
        .args(["-O1", "-std=gnu11", "-Wall", "-Wextra", "-Werror", "-I"])
        .arg(src_dir())
        .arg("-I")
        .arg(rt_dir())
        .arg(&src)
        .arg(&desk)
        .arg("-o")
        .arg(&exe)
        .env("TMPDIR", &dir)
        .output()
        .expect("clang not found");
    assert!(cc.status.success(), "clang failed: {}", String::from_utf8_lossy(&cc.stderr));
    let run = Command::new(&exe).output().expect("run");
    let out = String::from_utf8_lossy(&run.stdout).to_string();
    assert!(run.status.success(), "the compositor test failed:\n{out}");
    assert!(out.contains(", 0 failed"), "{out}");
    let _ = std::fs::remove_file(&src);
    let _ = std::fs::remove_file(&exe);
}

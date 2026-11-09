/* Decision 0223 (desk1, PROVISIONAL): the desktop compositor. See fle_desk.h for what it reads, what it draws and the sentences of Microsoft Learn it follows (docs/decisions/0223-desktop-compositor.md has the quotations and
 * the choices without a source, S-desk-*). It does not decide anything about windows; it draws what the window table says, bottom to top, on the one picture of the virtual screen.
 *
 * Linked by the wasm build (decision 0243: always built; fle_raster.c reaches it through a weak reference, so a native link list without this file works). The build must also compile this file with optimisation (RT_O2_FILES="fle_desk"): the runtime's default -O0 makes the pixel loops about ten times slower. */

#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef __EMSCRIPTEN__
#include <emscripten.h>
#else
#define EMSCRIPTEN_KEEPALIVE
#endif

#include "fle_class.h"
#include "fle_desk.h"
#include "fle_input.h"
#include "fle_layered.h"
#include "fle_monitor.h"
#include "fle_raster.h"
#include "fle_screen.h"
#include "fle_win.h"

/* weak: a unit-test link of this file with a stub window table that has no frame margins and no window regions still links (decision 0244) */
int fle_win_frame_margins(uint64_t hwnd, int32_t m[4]) __attribute__((weak));
int fle_win_window_region_box(uint64_t hwnd, int32_t *l, int32_t *t, int32_t *r, int32_t *b) __attribute__((weak));

/* the size of the picture = the size of the virtual screen now (fle_screen.h, decision 0212: a value of the run, and it can change while the engine runs); the origin of the screen is (FLE_MONITOR_LEFT, FLE_MONITOR_TOP) */
static uint32_t W_, H_;
#define LF_ALPHA 1u            /* the layer words of fle_layered_info as the page-side table gave them (decision 0145): alpha is to be applied */
#define LF_KEY 2u              /* a colour key */
#define LF_PREMUL 4u           /* the fourth byte is a premultiplied alpha */
#define LF_GLASS 8u            /* decision 0244: the frame of the window is extended into its client area (DwmExtendFrameIntoClientArea): outside the inner rectangle the fourth byte is a premultiplied alpha */

typedef struct {
    uint32_t lid;
    int32_t x, y;                       /* the window's origin on the screen */
    uint32_t w, h;                      /* the size of its surface */
    uint32_t layered, lflags, alpha, key;
    int32_t cl, ct, cr, cb;             /* decision 0244: the box of the window region (SetWindowRgn) in the window's own coordinates, inside [0, w] x [0, h]; the whole window when it has none */
    int32_t gx0, gy0, gx1, gy1;         /* decision 0244, LF_GLASS: the inner rectangle of the client area that the extended frame does not reach (the fourth byte is not read there); empty for the sheet of glass */
    const fle_rsurf *surf;
    char cls[48], title[80];            /* diagnostics only (fle_desk_describe): the window's class name and title as printable ASCII; nothing in the compositor reads them */
} dwin;

static uint32_t *pic;                   /* the picture, R G B A bytes */
static dwin cur[FLE_DESK_MAX_WINDOWS], prev[FLE_DESK_MAX_WINDOWS];
static unsigned ncur, nprev;
static fle_desk_state st;
static fle_desk_rect dmg[FLE_DESK_MAX_RECTS];      /* what must be drawn again */
static unsigned ndmg;
static fle_desk_rect out[FLE_DESK_MAX_RECTS];      /* what really changed, to be reported */
static unsigned nout;
static uint32_t *scratch;                          /* a rectangle drawn before it is compared with the picture */
static size_t scratch_px;
static int need_full = 1;
static char desc[FLE_DESK_MAX_WINDOWS * 300u + 64u];

static int64_t imin(int64_t a, int64_t b) { return a < b ? a : b; }
static int64_t imax(int64_t a, int64_t b) { return a > b ? a : b; }

static int rect_touch(const fle_desk_rect *a, const fle_desk_rect *b) { return a->x0 <= b->x1 && b->x0 <= a->x1 && a->y0 <= b->y1 && b->y0 <= a->y1; }
static int rect_in(const fle_desk_rect *a, const fle_desk_rect *b) { return a->x0 >= b->x0 && a->y0 >= b->y0 && a->x1 <= b->x1 && a->y1 <= b->y1; }
static fle_desk_rect rect_union(const fle_desk_rect *a, const fle_desk_rect *b)
{
    fle_desk_rect u = { a->x0 < b->x0 ? a->x0 : b->x0, a->y0 < b->y0 ? a->y0 : b->y0, a->x1 > b->x1 ? a->x1 : b->x1, a->y1 > b->y1 ? a->y1 : b->y1 };

    return u;
}
static int64_t rect_area(const fle_desk_rect *r) { return (int64_t)(r->x1 - r->x0) * (r->y1 - r->y0); }

/* adds a rectangle (in picture coordinates, clipped to the picture) to a list of at most FLE_DESK_MAX_RECTS: one inside another is dropped, rectangles that touch are merged when little is added, and when the list is full the pair that grows least is merged */
static void list_add(fle_desk_rect *list, unsigned *n, int64_t x0, int64_t y0, int64_t x1, int64_t y1)
{
    fle_desk_rect r;
    unsigned i;

    x0 = imax(x0, 0);
    y0 = imax(y0, 0);
    x1 = imin(x1, (int64_t)W_);
    y1 = imin(y1, (int64_t)H_);
    if (x0 >= x1 || y0 >= y1) return;
    r.x0 = (int32_t)x0;
    r.y0 = (int32_t)y0;
    r.x1 = (int32_t)x1;
    r.y1 = (int32_t)y1;
    for (i = 0; i < *n; i++)
        if (rect_in(&r, &list[i])) return;
    for (i = 0; i < *n;) {
        if (rect_in(&list[i], &r)) list[i] = list[--*n];
        else i++;
    }
    for (i = 0; i < *n; i++)
        if (rect_touch(&r, &list[i])) {
            fle_desk_rect u = rect_union(&r, &list[i]);

            /* merge only when little is added: two touching rectangles whose union is much larger than both stay apart */
            if (rect_area(&u) <= (rect_area(&r) + rect_area(&list[i])) * 5 / 4) {
                list[i] = list[--*n];
                list_add(list, n, u.x0, u.y0, u.x1, u.y1);
                return;
            }
        }
    if (*n == FLE_DESK_MAX_RECTS) {
        unsigned bi = 0;
        int64_t best = -1;

        for (i = 0; i < *n; i++) {
            fle_desk_rect u = rect_union(&r, &list[i]);
            int64_t grow = rect_area(&u) - rect_area(&list[i]);

            if (best < 0 || grow < best) {
                best = grow;
                bi = i;
            }
        }
        {
            fle_desk_rect u = rect_union(&r, &list[bi]);

            list[bi] = list[--*n];
            list_add(list, n, u.x0, u.y0, u.x1, u.y1);
        }
        return;
    }
    list[(*n)++] = r;
}

static void damage_add(int64_t x0, int64_t y0, int64_t x1, int64_t y1) { list_add(dmg, &ndmg, x0, y0, x1, y1); }

static void damage_win(const dwin *w) { damage_add(w->x - FLE_MONITOR_LEFT, w->y - FLE_MONITOR_TOP, (int64_t)w->x - FLE_MONITOR_LEFT + w->w, (int64_t)w->y - FLE_MONITOR_TOP + w->h); }

static const dwin *find_in(const dwin *list, unsigned n, uint32_t lid)
{
    for (unsigned i = 0; i < n; i++)
        if (list[i].lid == lid) return &list[i];
    return NULL;
}

static int scene_on;                    /* decision 0240: fle_desk_scene_mode(1) */
static void scene_wrote(uint32_t lid, int64_t x0, int64_t y0, int64_t x1, int64_t y1);

void fle_desk_wrote(uint32_t lid, int64_t x0, int64_t y0, int64_t x1, int64_t y1)
{
    const dwin *w;

    if (scene_on) {
        scene_wrote(lid, x0, y0, x1, y1);
        return;
    }
    w = find_in(prev, nprev, lid);

    if (!w) return;                     /* not drawn at the last compose: the whole window is damaged when it appears */
    x0 = imax(x0, 0);
    y0 = imax(y0, 0);
    x1 = imin(x1, (int64_t)w->w);
    y1 = imin(y1, (int64_t)w->h);
    if (x0 >= x1 || y0 >= y1) return;
    damage_add(x0 + w->x - FLE_MONITOR_LEFT, y0 + w->y - FLE_MONITOR_TOP, x1 + w->x - FLE_MONITOR_LEFT, y1 + w->y - FLE_MONITOR_TOP);
}

EMSCRIPTEN_KEEPALIVE void fle_desk_resync(void) { need_full = 1; }

/* ---- pixels ---- */
static uint32_t div255(uint32_t x) { x += 128u; return (x + (x >> 8)) >> 8; }

/* BGRX (bytes B, G, R, X) -> R, G, B, 255 */
static uint32_t bgrx_to_rgba(uint32_t p) { return 0xFF000000u | ((p >> 16) & 0xFFu) | (p & 0xFF00u) | ((p & 0xFFu) << 16); }

/* One pixel of a premultiplied B, G, R, A source over the destination d: UpdateLayeredWindow / AC_SRC_ALPHA ("Src = Src * SCA / 255" in all four channels, then "Dst = Src + (1 - Src.A) * Dst": BLENDFUNCTION) evaluated with one
 * rounding to the nearest integer over 255 * 255 (the table of docs/decisions/0244-translucent-windows.md): Dst' = (C * SCA * 255 + Dst * (255 * 255 - A * SCA) + 32512) / (255 * 255), each channel clamped at 255 (a source with a colour
 * above its alpha, which the guest must not make, adds light). A pixel with A = 0 and no colour leaves d as it is (the formula gives d exactly). The result is opaque (the picture has A = 255). */
static uint32_t over_premul(uint32_t p, uint32_t d, uint32_t sca)
{
    uint32_t b = p & 0xFFu, g = (p >> 8) & 0xFFu, r = (p >> 16) & 0xFFu, a = p >> 24, k, o, v;

    if (a == 0 && (b | g | r) == 0) return d;
    k = 65025u - a * sca;
    v = (r * sca * 255u + (d & 0xFFu) * k + 32512u) / 65025u;
    o = v > 255u ? 255u : v;
    v = (g * sca * 255u + ((d >> 8) & 0xFFu) * k + 32512u) / 65025u;
    o |= (v > 255u ? 255u : v) << 8;
    v = (b * sca * 255u + ((d >> 16) & 0xFFu) * k + 32512u) / 65025u;
    o |= (v > 255u ? 255u : v) << 16;
    return 0xFF000000u | o;
}

/* the rectangle [x0, x1) x [y0, y1) of the picture, of window w (already clipped to the window, to its window region and to the picture) */
static void draw_rect(uint32_t *base, int32_t stride, int32_t ox, int32_t oy, const dwin *w, int32_t x0, int32_t y0, int32_t x1, int32_t y1)
{
    const fle_rsurf *s = w->surf;
    int32_t sx = x0 - (w->x - FLE_MONITOR_LEFT), sy = y0 - (w->y - FLE_MONITOR_TOP);
    uint32_t alpha = (w->lflags & LF_ALPHA) ? (w->alpha & 0xFFu) : 255u;
    int keyed = w->layered && (w->lflags & LF_KEY) != 0;
    int premul = w->layered && (w->lflags & LF_PREMUL) != 0;
    int glass = !w->layered && (w->lflags & LF_GLASS) != 0;
    uint32_t kr = w->key & 0xFFu, kg = (w->key >> 8) & 0xFFu, kb = (w->key >> 16) & 0xFFu;

    for (int32_t y = y0; y < y1; y++, sy++) {
        const uint32_t *src = (const uint32_t *)(const void *)(s->bits + (uint64_t)sy * s->stride) + sx;
        uint32_t *dst = base + (int64_t)(y - oy) * stride + (x0 - ox);
        int32_t n = x1 - x0;

        if (glass) {
            /* decision 0244: the extended frame (Learn DwmExtendFrameIntoClientArea: "the client area is rendered as a solid surface with no window border"; DWM custom frame: "the regions underlying each of the extended frame's sides must have
             * pixel data with an alpha value of 0" to show the frame): outside the inner rectangle the pixel is a premultiplied ARGB over what is below, inside it the window is opaque as any other window. UNOFFICIAL: how a partial alpha is
             * composed (the pages do not say; the formula of UpdateLayeredWindow, which the page of DwmEnableBlurBehindWindow points to with "The alpha values in the window are honored"). */
            int inner_row = sy >= w->gy0 && sy < w->gy1;

            for (int32_t i = 0; i < n; i++) {
                int32_t wx = sx + i;

                if (inner_row && wx >= w->gx0 && wx < w->gx1) dst[i] = bgrx_to_rgba(src[i]);
                else dst[i] = over_premul(src[i], dst[i], 255u);
            }
        } else if (!w->layered || (alpha == 255u && !keyed && !premul)) {
            for (int32_t i = 0; i < n; i++) dst[i] = bgrx_to_rgba(src[i]);
        } else if (premul) {
            for (int32_t i = 0; i < n; i++) dst[i] = over_premul(src[i], dst[i], alpha);
        } else {
            /* SetLayeredWindowAttributes: LWA_COLORKEY makes the pixels of the key transparent; LWA_ALPHA: Dst = Src * bAlpha / 255 + Dst * (1 - bAlpha / 255) */
            for (int32_t i = 0; i < n; i++) {
                uint32_t p = src[i], b = p & 0xFFu, g = (p >> 8) & 0xFFu, r = (p >> 16) & 0xFFu, d = dst[i];

                if (keyed && r == kr && g == kg && b == kb) continue;
                if (alpha == 255u) {
                    dst[i] = 0xFF000000u | r | (g << 8) | (b << 16);
                } else {
                    uint32_t ia = 255u - alpha;

                    dst[i] = 0xFF000000u | div255(r * alpha + (d & 0xFFu) * ia) | (div255(g * alpha + ((d >> 8) & 0xFFu) * ia) << 8) | (div255(b * alpha + ((d >> 16) & 0xFFu) * ia) << 16);
                }
            }
        }
    }
}

/* The one rule of 'report only what differs' (the trim of decision 0223, which the scene of decision 0240 shares): in one row, the span [*a, *b) narrowed to its first and its last pixel where `old` and `neu` differ; `old` takes the new
 * pixels there. 0: the row is the same (nothing changes, nothing is reported). Used by paint() on the compositor's picture and by sh_trim() on the scene's copy of a window's surface. */
static int trim_row(uint32_t *old, const uint32_t *neu, int32_t *a, int32_t *b)
{
    int32_t x0 = *a, x1 = *b;

    while (x0 < x1 && old[x0] == neu[x0]) x0++;
    if (x0 == x1) return 0;
    while (old[x1 - 1] == neu[x1 - 1]) x1--;
    memcpy(old + x0, neu + x0, (size_t)(x1 - x0) * 4u);
    *a = x0;
    *b = x1;
    return 1;
}

/* Draws the rectangle (the background, then the windows from the bottom) into a scratch rectangle, compares it with the picture and copies what differs. The bounding rectangle of the changed pixels is added to the
 * report; a rectangle in which nothing changed is not reported. `whole`: the report is the rectangle itself (the first call, a new screen size, a browser that lost its copy). The browser is told what it must
 * put on its canvas: an engine that draws the same pixels again (it does, once per frame, an idle redraw) costs the browser nothing. */
static void paint(const fle_desk_rect *r, int whole)
{
    const uint32_t bg = 0xFF000000u | FLE_DESK_BACKGROUND_R | (FLE_DESK_BACKGROUND_G << 8) | (FLE_DESK_BACKGROUND_B << 16);
    int32_t w = r->x1 - r->x0, h = r->y1 - r->y0, cx0 = w, cy0 = h, cx1 = 0, cy1 = 0;
    size_t need = (size_t)w * (size_t)h;

    if (need > scratch_px) {
        free(scratch);
        scratch = (uint32_t *)malloc(need * 4u);
        scratch_px = scratch ? need : 0;
        if (!scratch) return;
    }
    for (size_t i = 0; i < need; i++) scratch[i] = bg;
    for (unsigned k = 0; k < ncur; k++) {
        const dwin *wd = &cur[k];
        int64_t wx0 = (int64_t)wd->x - FLE_MONITOR_LEFT, wy0 = (int64_t)wd->y - FLE_MONITOR_TOP;
        int64_t x0 = imax(r->x0, wx0 + wd->cl), y0 = imax(r->y0, wy0 + wd->ct), x1 = imin(r->x1, wx0 + wd->cr), y1 = imin(r->y1, wy0 + wd->cb);   /* decision 0244: inside the window region only */

        if (x0 >= x1 || y0 >= y1) continue;
        draw_rect(scratch, w, r->x0, r->y0, wd, (int32_t)x0, (int32_t)y0, (int32_t)x1, (int32_t)y1);
    }
    for (int32_t y = 0; y < h; y++) {
        uint32_t *row = pic + (int64_t)(r->y0 + y) * W_ + r->x0;
        const uint32_t *nw = scratch + (size_t)y * (size_t)w;
        int32_t a = 0, b = w;

        if (!trim_row(row, nw, &a, &b)) continue;
        if (a < cx0) cx0 = a;
        if (b > cx1) cx1 = b;
        if (y < cy0) cy0 = y;
        cy1 = y + 1;
    }
    if (whole) list_add(out, &nout, r->x0, r->y0, r->x1, r->y1);
    else if (cx0 < cx1) list_add(out, &nout, r->x0 + cx0, r->y0 + cy0, r->x0 + cx1, r->y0 + cy1);
}

/* ---- the window table, read ---- */
static int same_win(const dwin *a, const dwin *b)
{
    return a->x == b->x && a->y == b->y && a->w == b->w && a->h == b->h && a->layered == b->layered && a->lflags == b->lflags && a->alpha == b->alpha && a->key == b->key && a->surf == b->surf
        && a->cl == b->cl && a->ct == b->ct && a->cr == b->cr && a->cb == b->cb && a->gx0 == b->gx0 && a->gy0 == b->gy0 && a->gx1 == b->gx1 && a->gy1 == b->gy1;
}

/* UTF-16 units -> printable ASCII for the text of fle_desk_describe: a unit outside 0x20..0x7e, and the quote and the backslash, become '?' (a test reads the line back with a regular expression) */
static void ascii_of(char *out, size_t cap, const uint16_t *u, size_t n)
{
    size_t i;

    for (i = 0; i < n && i + 1 < cap; i++) out[i] = (u[i] >= 0x20u && u[i] <= 0x7eu && u[i] != '"' && u[i] != '\\') ? (char)u[i] : '?';
    out[i] = 0;
}

static void names_of(uint64_t hwnd, dwin *w)
{
    fle_win_view wv;
    uint16_t nm[FLE_CLASS_NAME_MAX_UNITS + 1], nl = 0;

    w->cls[0] = 0;
    w->title[0] = 0;
    if (!fle_win_get(hwnd, &wv)) return;
    ascii_of(w->title, sizeof w->title, wv.text, wv.text_len < FLE_WIN_TEXT_MAX_UNITS ? wv.text_len : FLE_WIN_TEXT_MAX_UNITS);
    if (fle_class_name_of(wv.atom, nm, &nl)) ascii_of(w->cls, sizeof w->cls, nm, nl);
}

static void gather(void)
{
    uint64_t hs[FLE_DESK_MAX_WINDOWS];
    unsigned n = fle_win_input_toplevels(hs, FLE_DESK_MAX_WINDOWS);

    ncur = 0;
    for (unsigned k = n; k-- > 0;) {    /* the list is top first: the picture is drawn from the bottom */
        fle_win_iview v;
        fle_layered_info li;
        const fle_rsurf *s;
        dwin *w;

        if (!fle_win_input_view(hs[k], &v) || !v.visible) continue;
        s = fle_raster_surface_peek(v.lid);
        if (!s || !s->bits || s->bpp != 32 || s->width == 0 || s->height == 0) continue;
        w = &cur[ncur];
        memset(w, 0, sizeof *w);
        w->lid = v.lid;
        names_of(hs[k], w);
        w->x = v.ox;
        w->y = v.oy;
        w->w = s->width;
        w->h = s->height;
        w->surf = s;
        if (fle_layered_query(hs[k], &li)) {
            if (li.mode == FLE_LAYER_NONE) continue;    /* "the layered window will not become visible until" a call of UpdateLayeredWindow or SetLayeredWindowAttributes */
            w->layered = 1;
            w->alpha = li.alpha & 0xFFu;
            if (li.mode == FLE_LAYER_ULW || (li.mode == FLE_LAYER_SLWA && (li.lwa_flags & FLE_LWA_ALPHA))) w->lflags |= LF_ALPHA;
            if (li.mode == FLE_LAYER_SLWA && (li.lwa_flags & FLE_LWA_COLORKEY)) {
                w->lflags |= LF_KEY;
                w->key = li.colorkey & 0xFFFFFFu;
            }
            if (li.mode == FLE_LAYER_ULW && li.per_pixel) w->lflags |= LF_PREMUL;
        } else {
            int32_t m[4];

            /* decision 0244: DwmExtendFrameIntoClientArea. A negative margin is the sheet of glass (the page's own example is MARGINS {-1}): the whole window is the extended frame; otherwise the frame is the strips of the margins. */
            if (fle_win_frame_margins && fle_win_frame_margins(hs[k], m)) {
                int64_t iw = (int64_t)w->w, ih = (int64_t)w->h;

                w->lflags |= LF_GLASS;
                if (m[0] < 0 || m[1] < 0 || m[2] < 0 || m[3] < 0) {
                    w->gx0 = w->gy0 = w->gx1 = w->gy1 = 0;
                } else {
                    w->gx0 = (int32_t)imin(m[0], iw);
                    w->gy0 = (int32_t)imin(m[2], ih);
                    w->gx1 = (int32_t)imax(iw - m[1], w->gx0);
                    w->gy1 = (int32_t)imax(ih - m[3], w->gy0);
                }
            }
        }
        w->cl = 0;
        w->ct = 0;
        w->cr = (int32_t)w->w;
        w->cb = (int32_t)w->h;
        {
            int32_t l, t, r, b;

            /* decision 0244: SetWindowRgn, Learn: "The system does not display any portion of a window that lies outside of the window region". The region is in the coordinates of the window, whose upper-left corner is the origin of its surface. */
            if (fle_win_window_region_box && fle_win_window_region_box(hs[k], &l, &t, &r, &b)) {
                w->cl = (int32_t)imax(0, imin(l, (int64_t)w->w));
                w->ct = (int32_t)imax(0, imin(t, (int64_t)w->h));
                w->cr = (int32_t)imax(w->cl, imin(r, (int64_t)w->w));
                w->cb = (int32_t)imax(w->ct, imin(b, (int64_t)w->h));
            }
        }
        ncur++;
    }
}

static void diff_damage(void)
{
    unsigned first = 0;

    for (unsigned i = 0; i < ncur; i++) {
        const dwin *p = find_in(prev, nprev, cur[i].lid);

        if (!p) damage_win(&cur[i]);
        else if (!same_win(p, &cur[i])) {
            damage_win(p);
            damage_win(&cur[i]);
        }
    }
    for (unsigned i = 0; i < nprev; i++)
        if (!find_in(cur, ncur, prev[i].lid)) damage_win(&prev[i]);
    while (first < ncur && first < nprev && cur[first].lid == prev[first].lid) first++;
    if (first < ncur || first < nprev) {
        /* the z-order changed from this place upwards: every window from here on may be drawn over another in a new way */
        for (unsigned i = first; i < ncur; i++) damage_win(&cur[i]);
        for (unsigned i = first; i < nprev; i++) damage_win(&prev[i]);
    }
}

static void build_desc(void)
{
    char *d = desc;
    unsigned i;

    *d = 0;
    for (i = 0; i < ncur; i++) {
        char gl[40] = "", rg[56] = "";

        /* decision 0244: the words of the extended frame (the inner rectangle the frame does not reach) and of the window region, with the other layer words: the line still ends with the class and the title */
        if (cur[i].lflags & LF_GLASS) snprintf(gl, sizeof gl, " glass(%d,%d,%d,%d)", cur[i].gx0, cur[i].gy0, cur[i].gx1, cur[i].gy1);
        if (cur[i].cl != 0 || cur[i].ct != 0 || cur[i].cr != (int32_t)cur[i].w || cur[i].cb != (int32_t)cur[i].h) snprintf(rg, sizeof rg, " region(%d,%d,%d,%d)", cur[i].cl, cur[i].ct, cur[i].cr, cur[i].cb);
        d += snprintf(d, sizeof desc - (size_t)(d - desc), "lid %u at (%d, %d) %ux%u%s%s%s%s%s%s alpha %u key %06x class \"%s\" title \"%s\"\n", cur[i].lid, cur[i].x, cur[i].y, cur[i].w, cur[i].h, cur[i].layered ? " layered" : "", (cur[i].lflags & LF_PREMUL) ? " premultiplied" : "",
                      (cur[i].lflags & LF_ALPHA) ? " alpha" : "", (cur[i].lflags & LF_KEY) ? " key" : "", gl, rg, cur[i].alpha, cur[i].key, cur[i].cls, cur[i].title);
        if ((size_t)(d - desc) >= sizeof desc - 300u) break;
    }
}

EMSCRIPTEN_KEEPALIVE const fle_desk_state *fle_desk_compose(void)
{
    unsigned i;
    int whole = 0;

    if (!pic || W_ != fle_screen_w() || H_ != fle_screen_h()) {
        /* the first call, or the screen changed size: a new picture (the old one is not kept: the whole picture is reported), no window is known to have been drawn on it */
        free(pic);
        W_ = fle_screen_w();
        H_ = fle_screen_h();
        pic = (uint32_t *)calloc((size_t)W_ * H_, 4u);
        if (!pic) return NULL;
        need_full = 1;
        nprev = 0;
        ndmg = 0;
    }
    st.magic = FLE_DESK_MAGIC;
    st.version = FLE_DESK_VERSION;
    st.width = W_;
    st.height = H_;
    st.stride = W_ * 4u;
    st.pixels = (uint32_t)(uintptr_t)pic;
    gather();
    diff_damage();
    if (need_full) {
        ndmg = 0;
        damage_add(0, 0, W_, H_);
        st.flags = 1u;
        whole = 1;
        need_full = 0;
    } else {
        st.flags = 0;
    }
    nout = 0;
    for (i = 0; i < ndmg; i++) paint(&dmg[i], whole);
    st.n_rects = nout;
    memcpy(st.rects, out, nout * sizeof out[0]);
    if (nout) st.gen++;
    ndmg = 0;
    st.n_windows = ncur;
    memcpy(prev, cur, ncur * sizeof cur[0]);
    nprev = ncur;
    build_desc();
    return &st;
}

EMSCRIPTEN_KEEPALIVE const char *fle_desk_describe(void) { return desc; }

const uint8_t *fle_desk_picture(void) { return (const uint8_t *)(const void *)pic; }

/* ---- Decision 0240 (gpu1): the scene (fle_desk.h) ---- */
static fle_scene sc;
static uint32_t su_lid[FLE_SCENE_MAX_WINDOWS];      /* the rectangles collected by fle_desk_wrote: one bounding rectangle per window, in its surface */
static fle_desk_rect su_r[FLE_SCENE_MAX_WINDOWS];
static unsigned n_su;
static int scene_whole = 1;
/* What the browser holds of each window's surface, byte for byte (B, G, R, X as the surface has them): a written rectangle whose pixels are the ones the browser already has is not reported (the idle redraw of the main form writes
 * the same pixels again once per frame; the CPU compositor drops those the same way, by comparing with its picture). One copy per window of the scene, made when the window is new or has another size. */
static uint32_t sh_lid[FLE_SCENE_MAX_WINDOWS];
static uint32_t *sh_px[FLE_SCENE_MAX_WINDOWS];
static uint32_t sh_w[FLE_SCENE_MAX_WINDOWS], sh_h[FLE_SCENE_MAX_WINDOWS];
static unsigned n_sh;

static unsigned sh_find(uint32_t lid)
{
    unsigned i;

    for (i = 0; i < n_sh; i++)
        if (sh_lid[i] == lid) return i;
    return n_sh;
}
static void sh_drop(unsigned i)
{
    free(sh_px[i]);
    n_sh--;
    sh_lid[i] = sh_lid[n_sh];
    sh_px[i] = sh_px[n_sh];
    sh_w[i] = sh_w[n_sh];
    sh_h[i] = sh_h[n_sh];
}
/* the shadow of a window: made (and filled from the surface) when it does not exist or has another size; 1 when it was made now (the whole surface is to be sent), 0 when it was there, -1 when there is no memory */
static int sh_get(const dwin *c, uint32_t **out)
{
    unsigned i = sh_find(c->lid);
    uint32_t y;

    if (i < n_sh && sh_w[i] == c->w && sh_h[i] == c->h) {
        *out = sh_px[i];
        return 0;
    }
    if (i < n_sh) sh_drop(i);
    if (n_sh == FLE_SCENE_MAX_WINDOWS) return -1;
    sh_px[n_sh] = (uint32_t *)malloc((size_t)c->w * c->h * 4u);
    if (!sh_px[n_sh]) return -1;
    sh_lid[n_sh] = c->lid;
    sh_w[n_sh] = c->w;
    sh_h[n_sh] = c->h;
    for (y = 0; y < c->h; y++) memcpy(sh_px[n_sh] + (size_t)y * c->w, c->surf->bits + (size_t)y * c->surf->stride, (size_t)c->w * 4u);
    *out = sh_px[n_sh];
    n_sh++;
    return 1;
}
/* compares a written rectangle of the surface with the shadow, brings the shadow up to date and returns the bounding rectangle of the pixels that differ (empty: none) */
static int sh_trim(const dwin *c, uint32_t *sh, fle_desk_rect *r)
{
    int32_t cx0 = r->x1, cy0 = r->y1, cx1 = r->x0, cy1 = r->y0;

    for (int32_t y = r->y0; y < r->y1; y++) {
        const uint32_t *s = (const uint32_t *)(const void *)(c->surf->bits + (size_t)y * c->surf->stride);
        uint32_t *d = sh + (size_t)y * c->w;
        int32_t a = r->x0, b = r->x1;

        if (!trim_row(d, s, &a, &b)) continue;
        if (a < cx0) cx0 = a;
        if (b > cx1) cx1 = b;
        if (y < cy0) cy0 = y;
        cy1 = y + 1;
    }
    if (cx0 >= cx1) return 0;
    r->x0 = cx0;
    r->x1 = cx1;
    r->y0 = cy0;
    r->y1 = cy1;
    return 1;
}

static void scene_wrote(uint32_t lid, int64_t x0, int64_t y0, int64_t x1, int64_t y1)
{
    unsigned i;

    if (x0 >= x1 || y0 >= y1) return;
    x0 = imax(x0, 0);
    y0 = imax(y0, 0);
    if (x0 >= x1 || y0 >= y1) return;
    if (x1 > 0x7fffffff) x1 = 0x7fffffff;
    if (y1 > 0x7fffffff) y1 = 0x7fffffff;
    for (i = 0; i < n_su; i++)
        if (su_lid[i] == lid) {
            if (x0 < su_r[i].x0) su_r[i].x0 = (int32_t)x0;
            if (y0 < su_r[i].y0) su_r[i].y0 = (int32_t)y0;
            if (x1 > su_r[i].x1) su_r[i].x1 = (int32_t)x1;
            if (y1 > su_r[i].y1) su_r[i].y1 = (int32_t)y1;
            return;
        }
    if (n_su == FLE_SCENE_MAX_WINDOWS) return;      /* more windows than the compositor draws: the lid is not in the scene either */
    su_lid[n_su] = lid;
    su_r[n_su].x0 = (int32_t)x0;
    su_r[n_su].y0 = (int32_t)y0;
    su_r[n_su].x1 = (int32_t)x1;
    su_r[n_su].y1 = (int32_t)y1;
    n_su++;
}

EMSCRIPTEN_KEEPALIVE void fle_desk_scene_mode(int on)
{
    scene_on = on != 0;
    scene_whole = 1;
    n_su = 0;
    nprev = 0;
    need_full = 1;
}

EMSCRIPTEN_KEEPALIVE const fle_scene *fle_desk_scene(void)
{
    unsigned i, j, nu = 0;
    int changed = 0, whole = 0;

    if (W_ != fle_screen_w() || H_ != fle_screen_h()) {
        W_ = fle_screen_w();
        H_ = fle_screen_h();
        scene_whole = 1;
    }
    if (need_full) {                    /* fle_desk_resync(), or the switch to the scene */
        scene_whole = 1;
        need_full = 0;
    }
    if (scene_whole) {
        whole = 1;
        scene_whole = 0;
        nprev = 0;
        n_su = 0;
        while (n_sh) sh_drop(n_sh - 1);     /* the browser starts again: every window is sent whole */
    }
    gather();
    for (i = 0; i < n_sh;) {
        unsigned q;

        for (q = 0; q < ncur && cur[q].lid != sh_lid[i]; q++) {}
        if (q == ncur) sh_drop(i);
        else i++;
    }
    if (ncur != nprev) changed = 1;
    sc.magic = FLE_SCENE_MAGIC;
    sc.version = FLE_SCENE_VERSION;
    sc.width = W_;
    sc.height = H_;
    sc.background = FLE_DESK_BACKGROUND_R | (FLE_DESK_BACKGROUND_G << 8) | (FLE_DESK_BACKGROUND_B << 16);
    for (i = 0; i < ncur; i++) {
        const dwin *c = &cur[i];
        const dwin *p = find_in(prev, nprev, c->lid);
        fle_scene_win *o = &sc.win[i];
        int fresh = !p || p->w != c->w || p->h != c->h || p->surf != c->surf;
        uint32_t *sh = NULL;
        int made;

        if (!p || p->surf != c->surf) {         /* another surface: what the browser holds is not what this one has */
            unsigned k = sh_find(c->lid);

            if (k < n_sh) sh_drop(k);
        }
        made = sh_get(c, &sh);

        if (made != 0) fresh = 1;       /* the shadow was just made from the surface (or could not be made): the whole surface is sent */

        if (!p || !same_win(p, c) || (i < nprev && prev[i].lid != c->lid)) changed = 1;
        o->lid = c->lid;
        o->x = (uint32_t)(int32_t)(c->x - FLE_MONITOR_LEFT);
        o->y = (uint32_t)(int32_t)(c->y - FLE_MONITOR_TOP);
        o->w = c->w;
        o->h = c->h;
        o->bits = (uint32_t)(uintptr_t)c->surf->bits;
        o->stride = (uint32_t)c->surf->stride;
        o->layer = (c->layered ? FLE_SCENE_LAYERED : 0u) | (c->lflags & (LF_ALPHA | LF_KEY | LF_PREMUL));
        o->alpha = c->alpha;
        o->key = c->key;
        o->fresh = fresh;
        o->reserved = 0;
        if (!c->layered && (c->lflags & LF_GLASS)) o->layer |= FLE_SCENE_GLASS;
        o->gx0 = c->gx0;
        o->gy0 = c->gy0;
        o->gx1 = c->gx1;
        o->gy1 = c->gy1;
        o->cl = c->cl;
        o->ct = c->ct;
        o->cr = c->cr;
        o->cb = c->cb;
        if (fresh) {                    /* a new texture: the whole surface */
            sc.upd[nu].lid = c->lid;
            sc.upd[nu].x0 = 0;
            sc.upd[nu].y0 = 0;
            sc.upd[nu].x1 = (int32_t)c->w;
            sc.upd[nu].y1 = (int32_t)c->h;
            nu++;
        } else {
            for (j = 0; j < n_su; j++)
                if (su_lid[j] == c->lid) {
                    fle_desk_rect r = su_r[j];

                    if (r.x1 > (int32_t)c->w) r.x1 = (int32_t)c->w;
                    if (r.y1 > (int32_t)c->h) r.y1 = (int32_t)c->h;
                    if (r.x0 < r.x1 && r.y0 < r.y1 && sh_trim(c, sh, &r)) {
                        sc.upd[nu].lid = c->lid;
                        sc.upd[nu].x0 = r.x0;
                        sc.upd[nu].y0 = r.y0;
                        sc.upd[nu].x1 = r.x1;
                        sc.upd[nu].y1 = r.y1;
                        nu++;
                    }
                    break;
                }
        }
    }
    n_su = 0;
    sc.n_windows = ncur;
    sc.n_updates = nu;
    sc.flags = (whole ? FLE_SCENE_FLAG_WHOLE : 0u) | (changed ? FLE_SCENE_FLAG_CHANGED : 0u);
    if (nu || changed || whole) sc.gen++;
    memcpy(prev, cur, ncur * sizeof cur[0]);
    nprev = ncur;
    build_desc();
    return &sc;
}


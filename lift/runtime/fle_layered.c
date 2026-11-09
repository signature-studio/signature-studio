/* Layered windows (decision 0140, PROVISIONAL; docs/decisions/0140-layered-windows.md). See fle_layered.h for the pixel model.
 *
 * Pages read (Microsoft Learn, 2026-11-02, copies in docs/investigations/lay1/learn/): UpdateLayeredWindow, UPDATELAYEREDWINDOWINFO, SetLayeredWindowAttributes, GetLayeredWindowAttributes, BLENDFUNCTION,
 * AlphaBlend, Window Features (Layered Windows), Extended Window Styles (WS_EX_LAYERED). The page of UpdateLayeredWindowIndirect itself could not be fetched (404): the structure page and the ULW page carry its rules.
 * What the pages say and this file does:
 *  - hWnd is "a layered window", one with WS_EX_LAYERED. After CreateWindowEx "the layered window will not become visible until the SetLayeredWindowAttributes or UpdateLayeredWindow function has been called".
 *  - pptDst: "the new screen position ... If the current position is not changing, pptDst can be NULL". psize: "the new size ... If the size of the window is not changing, psize can be NULL. If hdcSrc is NULL, psize must be
 *    NULL". hdcSrc: "A handle to a DC for the surface that defines the layered window ... CreateCompatibleDC ... If the shape and visual context of the window are not changing, hdcSrc can be NULL"; "If hdcSrc is NULL,
 *    hdcDst must be NULL"; "If hdcSrc is NULL, pptSrc should be NULL"; "If hdcSrc is NULL, dwFlags should be zero". pptSrc: "the location of the layer in the device context".
 *  - dwFlags ULW_ALPHA 2 ("Use pblend as the blend function"), ULW_COLORKEY 1 ("Use crKey as the transparency color"), ULW_OPAQUE 4 ("Draw an opaque layered window"), ULW_EX_NORESIZE 8 ("Force the
 *    UpdateLayeredWindowIndirect function to fail if the current window size does not match the size specified in the psize").
 *  - "UpdateLayeredWindow always updates the entire window"; UpdateLayeredWindowIndirect's prcDirty: "If it is non-NULL, only the area in this rectangle is updated from the source DC".
 *  - BLENDFUNCTION: BlendFlags "Must be zero"; BlendOp only AC_SRC_OVER; SourceConstantAlpha is "combined with any per-pixel alpha values", 0 = transparent, 255 = use only the per-pixel alpha; AC_SRC_ALPHA: "the bitmap
 *    has an Alpha channel ... the APIs use premultiplied alpha"; "When the AlphaFormat member is AC_SRC_ALPHA, the source bitmap must be 32 bpp".
 *  - "once SetLayeredWindowAttributes has been called for a layered window, subsequent UpdateLayeredWindow calls will fail until the layering style bit is cleared and set again"; GetLayeredWindowAttributes "can be called
 *    only if the application has previously called SetLayeredWindowAttributes ... The function will fail if the layered window was setup with UpdateLayeredWindow".
 *  The pages name NO error code for any failure, and say nothing about what the calls send to the window: the choices without a source are named S-ulw-* below and in the decision. A form the pages do not define stops
 *  the run by name (fle_stop), as everywhere in this runtime. */
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_env.h"
#include "fle_gdi.h"
#include "fle_imports.h"
#include "fle_layered.h"
#include "fle_raster.h"
#include "fle_runtime.h"
#include "fle_win.h"
#include "fle_thread.h"

/* weak, like the hooks of fle_win.c: a unit-test link without fle_gdi.c still links (a call then stops by name) */
int fle_gdi_memdc_surface(uint64_t hdc, fle_rsurf *out) __attribute__((weak));
int fle_win_layered_rect(uint64_t hwnd, int set_pos, int32_t x, int32_t y, int set_size, int32_t cx, int32_t cy) __attribute__((weak));

#define LAY_MAX 64u

typedef struct {
    int used;
    uint64_t hwnd;
    fle_layered_info in;
} lay_t;

static lay_t lays[LAY_MAX];
static lay_t *lay_find(uint64_t hwnd, int create)
{
    unsigned k;

    for (k = 0; k < LAY_MAX; k++)
        if (lays[k].used && lays[k].hwnd == hwnd) return &lays[k];
    if (!create) return NULL;
    for (k = 0; k < LAY_MAX; k++) {
        if (!lays[k].used) {
            memset(&lays[k], 0, sizeof lays[k]);
            lays[k].used = 1;
            lays[k].hwnd = hwnd;
            lays[k].in.mode = FLE_LAYER_NONE;
            lays[k].in.alpha = 255;
            return &lays[k];
        }
    }
    return NULL;
}

static void fail(uint32_t err)
{
    fle_set_last_error(err);
    R[0] = 0;
}

/* S-ulw-error-invalid-hwnd: a hWnd that is not a live window of the table: 0 and ERROR_INVALID_WINDOW_HANDLE (1400), the choice of SendMessageW (fle_win.h, decision 0119). S-ulw-error-not-layered: a live window
 * without WS_EX_LAYERED: 0 and ERROR_INVALID_PARAMETER (87). Both are choices without a source: the pages name no code. */
static int layered_window(const char *api, uint64_t hwnd, fle_win_view *v)
{
    (void)api;
    if (!fle_win_get(hwnd, v) || !v->live) {
        fail(FLE_ERROR_INVALID_WINDOW_HANDLE);
        return 0;
    }
    if (!(v->exstyle & FLE_WS_EX_LAYERED)) {
        fail(FLE_W32_ERR_INVALID_PARAMETER);
        return 0;
    }
    return 1;
}

static void stopf(const char *api, const char *why, uint64_t a) __attribute__((noreturn));
static void stopf(const char *api, const char *why, uint64_t a)
{
    char buf[300];

    snprintf(buf, sizeof buf, "%s (0x%" PRIx64 "; decision 0140)", why, a);
    fle_stop(api, buf);
}

/* One pixel of a source surface as a COLORREF and, for a 32-bit BI_RGB surface, the fourth byte. */
static inline uint32_t src_colorref(const fle_rsurf *s, uint32_t x, uint32_t y) { return fle_r_get(s, x, y); }

/* The common body of UpdateLayeredWindow and UpdateLayeredWindowIndirect. All pointers are guest addresses. */
static void ulw(const char *api, int indirect, uint64_t hwnd, uint64_t hdc_dst, uint64_t ppt_dst, uint64_t psize, uint64_t hdc_src, uint64_t ppt_src, uint32_t crkey, uint64_t pblend, uint32_t flags,
                uint64_t prc_dirty)
{
    fle_win_view v;
    fle_win_dcq q;
    lay_t *l;
    fle_rsurf src;
    fle_rsurf *dst;
    int32_t dx = 0, dy = 0, cx, cy, sx = 0, sy = 0, ux0, uy0, ux1, uy1;
    int have_pos = ppt_dst != 0, have_size = psize != 0;
    uint32_t kind = flags & (FLE_ULW_COLORKEY | FLE_ULW_ALPHA | FLE_ULW_OPAQUE);
    uint32_t sca = 255, afmt = 0;
    unsigned nk;
    int per_pixel_src = 0;

    if (!layered_window(api, hwnd, &v)) return;
    if (flags & ~(FLE_ULW_COLORKEY | FLE_ULW_ALPHA | FLE_ULW_OPAQUE | FLE_ULW_EX_NORESIZE)) stopf(api, "dwFlags has a bit that the page does not list", flags);
    if ((flags & FLE_ULW_EX_NORESIZE) && !indirect) stopf(api, "ULW_EX_NORESIZE for UpdateLayeredWindow: the page ties it to UpdateLayeredWindowIndirect", flags);
    nk = (kind & 1u) + ((kind >> 1) & 1u) + ((kind >> 2) & 1u);
    if (nk > 1) stopf(api, "more than one of ULW_COLORKEY, ULW_ALPHA and ULW_OPAQUE: the page says \"one of the following values\" and does not define a combination", flags);
    if (!fle_win_dc_query(hwnd, &q)) stopf(api, "the window has no drawing-surface record", hwnd);
    if (q.root != hwnd) stopf(api, "a child window with WS_EX_LAYERED (Windows 8): not modelled, the surface of a child is part of its top-level window's", hwnd);
    l = lay_find(hwnd, 1);
    if (!l) stopf(api, "more layered windows than the table holds", LAY_MAX);
    /* S-ulw-after-slwa: "subsequent UpdateLayeredWindow calls will fail until the layering style bit is cleared and set again": 0 and ERROR_INVALID_PARAMETER (87, a choice: the page names no code). */
    if (l->in.mode == FLE_LAYER_SLWA) {
        fail(FLE_W32_ERR_INVALID_PARAMETER);
        return;
    }
    if (hdc_src == 0) {
        /* "If hdcSrc is NULL, hdcDst must be NULL", "psize must be NULL": the call fails (S-ulw-error-null-src, 87). "pptSrc should be NULL" and "dwFlags should be zero": a form the page does not define: stop. */
        if (hdc_dst != 0 || psize != 0) {
            fail(FLE_W32_ERR_INVALID_PARAMETER);
            return;
        }
        if (ppt_src != 0) stopf(api, "hdcSrc is NULL and pptSrc is not: the page says pptSrc \"should be NULL\" and does not say what the call does otherwise", ppt_src);
        if (kind != 0) stopf(api, "hdcSrc is NULL and dwFlags is not zero: the page says dwFlags \"should be zero\" and does not say what the call does otherwise", flags);
        if (pblend != 0) stopf(api, "hdcSrc is NULL and pblend is given: the page does not say whether the blend function alone is applied", pblend);
    } else {
        if (!fle_gdi_memdc_surface) stopf(api, "no GDI model is linked: hdcSrc cannot be read", hdc_src);
        if (hdc_dst != 0) stopf(api, "hdcDst is not NULL: the page says it is a DC for the screen used for palette matching, and the palette of the screen is not modelled", hdc_dst);
        if (!fle_gdi_memdc_surface(hdc_src, &src)) stopf(api, "hdcSrc is not a memory DC with a bitmap selected (the page says a DC from CreateCompatibleDC): not modelled", hdc_src);
        if (kind == 0) stopf(api, "hdcSrc is given and none of ULW_COLORKEY, ULW_ALPHA, ULW_OPAQUE is: the page does not define it", flags);
        if (kind == FLE_ULW_ALPHA) {
            if (pblend == 0) {
                fail(FLE_W32_ERR_INVALID_PARAMETER);   /* S-ulw-error-null-blend: ULW_ALPHA says "Use pblend as the blend function" */
                return;
            }
            if ((uint32_t)fle_ld(pblend, 1) != FLE_AC_SRC_OVER || (uint32_t)fle_ld(pblend + 1, 1) != 0) {
                fail(FLE_W32_ERR_INVALID_PARAMETER);   /* S-ulw-error-blend: BlendOp is only AC_SRC_OVER, BlendFlags "must be zero" */
                return;
            }
            sca = (uint32_t)fle_ld(pblend + 2, 1);
            afmt = (uint32_t)fle_ld(pblend + 3, 1);
            if (afmt & ~FLE_AC_SRC_ALPHA) stopf(api, "AlphaFormat has a bit other than AC_SRC_ALPHA", afmt);
            if (afmt == FLE_AC_SRC_ALPHA) {
                if (src.bpp != 32) {
                    fail(FLE_W32_ERR_INVALID_PARAMETER);   /* "must be 32 bpp" */
                    return;
                }
                if (src.comp != 0) stopf(api, "AC_SRC_ALPHA with a BI_BITFIELDS source: the position of the alpha byte is not defined by the pages", src.comp);
                per_pixel_src = 1;
            }
        }
        if (ppt_src != 0) {
            sx = (int32_t)fle_ld(ppt_src, 4);
            sy = (int32_t)fle_ld(ppt_src + 4, 4);
        }
        if (!fle_r_readable(&src)) stopf(api, "the bitmap in hdcSrc has a format that the pixel model cannot read", src.bpp);
    }
    cx = have_size ? (int32_t)fle_ld(psize, 4) : v.cx;
    cy = have_size ? (int32_t)fle_ld(psize + 4, 4) : v.cy;
    if (have_pos) {
        dx = (int32_t)fle_ld(ppt_dst, 4);
        dy = (int32_t)fle_ld(ppt_dst + 4, 4);
    }
    if (have_size && (cx <= 0 || cy <= 0)) {
        fail(FLE_W32_ERR_INVALID_PARAMETER);   /* S-ulw-error-size: a window of no area */
        return;
    }
    if ((flags & FLE_ULW_EX_NORESIZE) && have_size && (cx != v.cx || cy != v.cy)) {
        fail(FLE_W32_ERR_INVALID_PARAMETER);   /* S-ulw-error-noresize: "Force the ... function to fail if the current window size does not match the size specified in the psize" */
        return;
    }
    /* Decision 0215 (sweep1): UPDATELAYEREDWINDOWINFO.prcDirty, Learn: "The area to be updated. This parameter can be NULL. If it is non-NULL, only the area in this rectangle is updated from the source DC." The page gives no
     * coordinate space; the rectangle is read in the coordinates of the layered window itself (origin = the window's top-left, the space of its drawing surface; S-sweep1-ulw-1, UNOFFICIAL: Wine 10.0's driver intersects the
     * window rectangle {0, 0, cx, cy} with prcDirty). The part of the rectangle outside the window, and an empty or inverted rectangle, update nothing. The source pixel of window pixel (x, y) is (sx + x, sy + y) as for the
     * whole update. The earlier decision 0140 stopped here by name because the space was not on the page; 21 clicks on the sub-menu windows of the engine (Help > Tutorials, View > Toolbars) met that stop. */
    if (prc_dirty != 0) {
        int32_t dl = (int32_t)fle_ld(prc_dirty, 4), dt = (int32_t)fle_ld(prc_dirty + 4, 4), dr = (int32_t)fle_ld(prc_dirty + 8, 4), db = (int32_t)fle_ld(prc_dirty + 12, 4);

        ux0 = dl < 0 ? 0 : dl;
        uy0 = dt < 0 ? 0 : dt;
        ux1 = dr > cx ? cx : dr;
        uy1 = db > cy ? cy : db;
        if (ux1 < ux0) ux1 = ux0;
        if (uy1 < uy0) uy1 = uy0;
    } else {
        ux0 = 0; uy0 = 0; ux1 = cx; uy1 = cy;
    }
    if (hdc_src != 0) {
        if (sx < 0 || sy < 0 || (int64_t)sx + cx > (int64_t)src.width || (int64_t)sy + cy > (int64_t)src.height) stopf(api, "the source rectangle (pptSrc, size) is not inside the bitmap of hdcSrc: the page does not say", (uint64_t)(uint32_t)sx);
    }

    /* the call is valid: apply it (S-ulw-no-messages: the window moves and changes size without WM_WINDOWPOSCHANGING, WM_WINDOWPOSCHANGED, WM_MOVE or WM_SIZE: the pages say nothing about messages) */
    if ((have_pos || have_size) && fle_win_layered_rect) fle_win_layered_rect(hwnd, have_pos, dx, dy, have_size, cx, cy);
    if (hdc_src != 0) {
        uint32_t x, y;
        fle_rrect all;

        dst = fle_raster_window_surface(q.root_lid, (uint32_t)cx, (uint32_t)cy);
        if (!dst) stopf(api, "the drawing surface of the layered window cannot be made (a size above 16384 or no memory)", (uint64_t)(uint32_t)cx);
        if (dst->width != (uint32_t)cx || dst->height != (uint32_t)cy) stopf(api, "the drawing surface of the layered window has another size than the window", dst->width);
        for (y = (uint32_t)uy0; y < (uint32_t)uy1; y++) {
            uint8_t *d = fle_r_row(dst, y);
            const uint8_t *s = fle_r_row(&src, (uint32_t)sy + y);

            for (x = (uint32_t)ux0; x < (uint32_t)ux1; x++) {
                uint8_t *p = d + 4u * x;

                if (per_pixel_src) {
                    const uint8_t *sp = s + 4u * ((uint32_t)sx + x);   /* B, G, R, A of a 32-bit BI_RGB pixel, premultiplied by the guest (AC_SRC_ALPHA) */
                    p[0] = sp[0]; p[1] = sp[1]; p[2] = sp[2]; p[3] = sp[3];
                } else {
                    uint32_t c = src_colorref(&src, (uint32_t)sx + x, (uint32_t)sy + y);

                    if (kind == FLE_ULW_COLORKEY && ((c ^ crkey) & 0xFFFFFFu) == 0) {
                        p[0] = p[1] = p[2] = p[3] = 0;     /* S-ulw-colorkey-baked: the key colour is transparent */
                    } else {
                        p[0] = (uint8_t)(c >> 16); p[1] = (uint8_t)(c >> 8); p[2] = (uint8_t)c; p[3] = 255;
                    }
                }
            }
        }
        all.x0 = ux0; all.y0 = uy0; all.x1 = ux1; all.y1 = uy1;
        if (ux1 > ux0 && uy1 > uy0) fle_raster_wrote(dst, all, api);
        l->in.per_pixel = 1;
        l->in.width = (uint32_t)cx;
        l->in.height = (uint32_t)cy;
        l->in.updates++;
        l->in.alpha = kind == FLE_ULW_ALPHA ? sca : 255u;
        l->in.colorkey = 0;
    }
    l->in.mode = FLE_LAYER_ULW;
    R[0] = 1;
}

/* user32!UpdateLayeredWindow: BOOL f(HWND hWnd rcx, HDC hdcDst rdx, POINT *pptDst r8, SIZE *psize r9, HDC hdcSrc [5], POINT *pptSrc [6], COLORREF crKey [7], BLENDFUNCTION *pblend [8], DWORD dwFlags [9]). */
void fle_imp_UpdateLayeredWindow(void)
{
    ulw("UpdateLayeredWindow", 0, R[1], R[2], R[8], R[9], fle_arg(5), fle_arg(6), (uint32_t)fle_arg(7), fle_arg(8), (uint32_t)fle_arg(9), 0);
}

/* user32!UpdateLayeredWindowIndirect: BOOL f(HWND hWnd rcx, const UPDATELAYEREDWINDOWINFO *pULWInfo rdx). cbSize must be the size of the structure (80 on x64): another value fails with 87 (S-ulw-error-cbsize). */
void fle_imp_UpdateLayeredWindowIndirect(void)
{
    uint64_t p = R[2];

    if (p == 0 || (uint32_t)fle_ld(p, 4) != FLE_ULWINFO_SIZE) {
        fail(FLE_W32_ERR_INVALID_PARAMETER);
        return;
    }
    ulw("UpdateLayeredWindowIndirect", 1, R[1], fle_ld(p + 8, 8), fle_ld(p + 16, 8), fle_ld(p + 24, 8), fle_ld(p + 32, 8), fle_ld(p + 40, 8), (uint32_t)fle_ld(p + 48, 4), fle_ld(p + 56, 8),
        (uint32_t)fle_ld(p + 64, 4), fle_ld(p + 72, 8));
}

/* user32!SetLayeredWindowAttributes: BOOL f(HWND hwnd rcx, COLORREF crKey edx, BYTE bAlpha r8b, DWORD dwFlags r9d). The window keeps its GDI surface (the "redirection": Window Features, "the system redirects the painting
 * of windows that called SetLayeredWindowAttributes into off-screen memory and recomposes it"): the attributes are kept for the Presenter, nothing is changed in the pixels. dwFlags: LWA_COLORKEY | LWA_ALPHA, at least
 * one; any other value stops (the page says "one or more of the following values"). A window set up with UpdateLayeredWindow: the page does not say: stop. */
void fle_imp_SetLayeredWindowAttributes(void)
{
    static const char api[] = "SetLayeredWindowAttributes";
    fle_win_view v;
    lay_t *l;
    uint32_t key = (uint32_t)R[2], alpha = (uint32_t)(R[8] & 0xFFu), flags = (uint32_t)R[9];

    if (!layered_window(api, R[1], &v)) return;
    if (flags == 0 || (flags & ~(FLE_LWA_COLORKEY | FLE_LWA_ALPHA))) stopf(api, "dwFlags is not LWA_COLORKEY and/or LWA_ALPHA: the page does not define it", flags);
    l = lay_find(R[1], 1);
    if (!l) stopf(api, "more layered windows than the table holds", LAY_MAX);
    if (l->in.mode == FLE_LAYER_ULW) stopf(api, "the window was set up with UpdateLayeredWindow: the page does not say what SetLayeredWindowAttributes does", R[1]);
    l->in.mode = FLE_LAYER_SLWA;
    l->in.lwa_flags = flags;
    l->in.alpha = (flags & FLE_LWA_ALPHA) ? alpha : 255u;
    l->in.colorkey = (flags & FLE_LWA_COLORKEY) ? (key & 0xFFFFFFu) : 0u;
    l->in.per_pixel = 0;
    R[0] = 1;
}

/* user32!GetLayeredWindowAttributes: BOOL f(HWND hwnd rcx, COLORREF *pcrKey rdx, BYTE *pbAlpha r8, DWORD *pdwFlags r9); each pointer may be NULL. Fails (0, 87: S-glwa-error) when SetLayeredWindowAttributes was not called
 * for the window, which includes a window set up with UpdateLayeredWindow ("The function will fail if the layered window was setup with UpdateLayeredWindow"). */
void fle_imp_GetLayeredWindowAttributes(void)
{
    static const char api[] = "GetLayeredWindowAttributes";
    fle_win_view v;
    lay_t *l;

    if (!layered_window(api, R[1], &v)) return;
    l = lay_find(R[1], 0);
    if (!l || l->in.mode != FLE_LAYER_SLWA) {
        fail(FLE_W32_ERR_INVALID_PARAMETER);
        return;
    }
    if (R[2]) fle_st(R[2], 4, l->in.colorkey);
    if (R[8]) fle_st(R[8], 1, l->in.alpha);
    if (R[9]) fle_st(R[9], 4, l->in.lwa_flags);
    R[0] = 1;
}

int fle_layered_query(uint64_t hwnd, fle_layered_info *out)
{
    fle_win_view v;
    lay_t *l;

    if (!fle_win_get(hwnd, &v) || !v.live || !(v.exstyle & FLE_WS_EX_LAYERED)) return 0;
    l = lay_find(hwnd, 0);
    if (l) {
        *out = l->in;
    } else {
        memset(out, 0, sizeof *out);
        out->mode = FLE_LAYER_NONE;
        out->alpha = 255;
    }
    return 1;
}

void fle_layered_exstyle_changed(uint64_t hwnd, uint32_t old_ex, uint32_t new_ex)
{
    lay_t *l;

    if (!((old_ex ^ new_ex) & FLE_WS_EX_LAYERED)) return;
    l = lay_find(hwnd, 0);
    if (l) l->used = 0;
}

void fle_layered_window_destroyed(uint64_t hwnd)
{
    lay_t *l = lay_find(hwnd, 0);

    if (l) l->used = 0;
}

void fle_layered_report(FILE *f)
{
    unsigned k;

    for (k = 0; k < LAY_MAX; k++) {
        if (!lays[k].used) continue;
        fprintf(f, "layered: hwnd=%#" PRIx64 " mode=%s alpha=%u key=%#x per_pixel=%u size=%ux%u updates=%" PRIu64 "\n", lays[k].hwnd,
                lays[k].in.mode == FLE_LAYER_ULW ? "ULW" : lays[k].in.mode == FLE_LAYER_SLWA ? "SLWA" : "NONE", (unsigned)lays[k].in.alpha, (unsigned)lays[k].in.colorkey, (unsigned)lays[k].in.per_pixel,
                (unsigned)lays[k].in.width, (unsigned)lays[k].in.height, lays[k].in.updates);
    }
}

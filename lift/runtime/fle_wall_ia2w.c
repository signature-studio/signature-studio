/* Walls of the second wave of user operations that need the window table and the pixel model (decision 0191, PROVISIONAL): user32!ScrollWindowEx (wall ia2-W23). See fle_wall_ia2.h.
 * Separate from fle_wall_ia2.c, which needs nothing of the window table (its tests link it alone). */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_win.h"
#include "fle_gdi.h"
#include "fle_raster.h"
#include "fle_wall_ia2.h"

/* decision 0214: weak, so that a link without the raster side (tests/wall_ia2.rs) is unchanged */
void fle_raster_wrote(const fle_rsurf *s, fle_rrect r, const char *api) __attribute__((weak));

/* Argument n (1-based, n >= 5) of the Windows x64 call (the guest return address is at R[4], the 32-byte shadow space follows it). */
static uint64_t stack_arg(unsigned n)
{
    return fle_ld(R[4] + 8u + 8u * (n - 1u), 8);
}

/* ---- user32!ScrollWindowEx (wall ia2-W23: the playlist's wheel and scroll bars) ----
 * int f(HWND hWnd rcx, int dx rdx, int dy r8d, const RECT *prcScroll r9, const RECT *prcClip, HRGN hrgnUpdate, LPRECT prcUpdate, UINT flags).
 * Learn (winuser.h, "ScrollWindowEx function"): scrolls "the contents of the specified window's client area"; prcScroll "the portion of the client area to be scrolled. If this parameter is NULL, the entire client area is
 * scrolled"; prcClip "Only device bits within the clipping rectangle are affected. Bits scrolled from the scrolling rectangle that lie outside the clipping rectangle are not drawn. If this parameter is NULL, the clipping
 * rectangle is the same as the scrolling rectangle"; hrgnUpdate "A handle to the region that is modified to hold the region invalidated by scrolling"; prcUpdate "receives the boundaries of the rectangle invalidated by
 * scrolling"; flags SW_SCROLLCHILDREN 0x1, SW_INVALIDATE 0x2 ("Invalidates the region identified by the hrgnUpdate parameter after scrolling"), SW_ERASE 0x4 ("When specified with SW_INVALIDATE, erases the newly invalidated
 * region by sending a WM_ERASEBKGND message to the window"), SW_SMOOTHSCROLL 0x10; the return value is SIMPLEREGION (2, "rectangular invalidated region"), COMPLEXREGION (3, "nonrectangular invalidated region; the overlap of
 * two rectangles") or NULLREGION (1, "no invalidated region"), ERROR (0) on failure; "If the SW_INVALIDATE and SW_ERASE flags are not specified, ScrollWindowEx does not invalidate the area that is scrolled from. If either of
 * these flags is set, ScrollWindowEx invalidates this area."
 * Model: the pixels of the scroll rectangle (client coordinates, clipped to the client area and to the window's drawing surface) that fall inside the clip rectangle after the move by (dx, dy) are copied inside the drawing
 * surface of the top-level window (decision 0126's pixel model: one surface per top-level window, the window's origin in it from fle_win_dc_query); the area the scroll leaves uncovered (the scroll rectangle inside the clip
 * rectangle minus the moved bits) is returned in prcUpdate as its bounding rectangle and, with SW_INVALIDATE or SW_ERASE, added to the window's update region (fle_imp_InvalidateRect: ONE bounding rectangle, S-updregion).
 * The return value is NULLREGION when nothing is uncovered, SIMPLEREGION when the uncovered area is one rectangle (a pure vertical or horizontal scroll), COMPLEXREGION for the L-shape of a diagonal scroll.
 * Choices without a source (S-ia2-scroll-*): (1) SW_SMOOTHSCROLL scrolls at once (no animation; the time in the high word of flags is ignored); (2) the surface pixels are the pixels "on the screen" that Windows would move
 * (the window's own drawing, no other window over it: the visible region is not modelled, S-clip-rects of decision 0126); (3) SW_SCROLLCHILDREN, a negative-size rectangle and a surface that
 * cannot be written stop by name: the child windows' move and WM_MOVE, and a region to modify, are not modelled. */
typedef struct { int32_t l, t, r, b; } irect;
static int rect_empty(const irect *a) { return a->r <= a->l || a->b <= a->t; }
static irect rect_and(irect a, irect b)
{
    irect o = { a.l > b.l ? a.l : b.l, a.t > b.t ? a.t : b.t, a.r < b.r ? a.r : b.r, a.b < b.b ? a.b : b.b };
    if (o.r < o.l) o.r = o.l;
    if (o.b < o.t) o.b = o.t;
    return o;
}
static irect rect_ld(uint64_t p)
{
    irect o = { (int32_t)fle_ld(p, 4), (int32_t)fle_ld(p + 4, 4), (int32_t)fle_ld(p + 8, 4), (int32_t)fle_ld(p + 12, 4) };
    return o;
}

static void scroll_window_ex_simple(void)
{
    static const char api[] = "ScrollWindowEx";
    uint64_t hwnd = R[1], prc_scroll = R[9], prc_clip = stack_arg(5), hrgn = stack_arg(6), prc_update = stack_arg(7);
    int32_t dx = (int32_t)R[2], dy = (int32_t)R[8];
    uint32_t flags = (uint32_t)stack_arg(8);
    fle_win_dcq q;
    irect client, src, clip, dst, unc;
    const fle_rsurf *surf;
    int rows = 0, type, parts;

    if (flags & 0x1u) fle_stop(api, "SW_SCROLLCHILDREN: moving the child windows (and the WM_MOVE they get) is not modelled");
    if (!fle_win_dc_query(hwnd, &q)) {
        fle_set_last_error(1400);       /* ERROR_INVALID_WINDOW_HANDLE: the page names no code (S-ia2-scroll-err, as the other window functions) */
        R[0] = 0;
        return;
    }
    client.l = 0; client.t = 0; client.r = q.cx; client.b = q.cy;
    src = client;
    if (prc_scroll != 0) { irect s = rect_ld(prc_scroll); if (s.r < s.l || s.b < s.t) fle_stop(api, "prcScroll has a negative size: the page gives no behaviour"); src = rect_and(s, client); }
    clip = src;
    if (prc_clip != 0) { irect c = rect_ld(prc_clip); if (c.r < c.l || c.b < c.t) fle_stop(api, "prcClip has a negative size: the page gives no behaviour"); clip = rect_and(c, client); }
    dst = src;
    dst.l += dx; dst.r += dx; dst.t += dy; dst.b += dy;
    dst = rect_and(dst, clip);
    /* the bits */
    if (!rect_empty(&dst) && (dx != 0 || dy != 0)) {
        irect vis = { q.clip_l - q.org_x, q.clip_t - q.org_y, q.clip_r - q.org_x, q.clip_b - q.org_y };
        irect d2 = rect_and(dst, vis);
        irect s2;
        surf = fle_raster_surface_peek(q.root_lid);
        if (surf == NULL || !fle_r_direct(surf)) fle_stop(api, "the drawing surface of the window cannot be written (no surface yet, or a format that is not 24 or 32 bit)");
        s2.l = d2.l - dx; s2.r = d2.r - dx; s2.t = d2.t - dy; s2.b = d2.b - dy;
        s2 = rect_and(s2, vis);
        d2.l = s2.l + dx; d2.r = s2.r + dx; d2.t = s2.t + dy; d2.b = s2.b + dy;
        if (!rect_empty(&d2)) {
            unsigned bpp = surf->bpp / 8u;
            int32_t y, w = d2.r - d2.l;
            int32_t sy0 = d2.t - dy + q.org_y, dy0 = d2.t + q.org_y;
            int up = dy > 0;        /* moving down: copy from the bottom row up */
            for (y = 0; y < d2.b - d2.t; y++) {
                int32_t yy = up ? (d2.b - d2.t - 1 - y) : y;
                int32_t srow = sy0 + yy, drow = dy0 + yy, sx = d2.l - dx + q.org_x, dxp = d2.l + q.org_x;
                if (srow < 0 || drow < 0 || (uint32_t)srow >= surf->height || (uint32_t)drow >= surf->height || sx < 0 || dxp < 0 || (uint32_t)(sx + w) > surf->width || (uint32_t)(dxp + w) > surf->width) continue;
                memmove(fle_r_row(surf, (uint32_t)drow) + (size_t)dxp * bpp, fle_r_row(surf, (uint32_t)srow) + (size_t)sx * bpp, (size_t)w * bpp);
                rows++;
            }
            /* decision 0214: the moved pixels are a write into the window's surface; the dirty rectangle (what the page is sent) has to know (before, a scroll reached the page only through the repaint of the uncovered strip) */
            if (rows > 0) {
                fle_rrect wr = { (int64_t)d2.l + q.org_x, (int64_t)d2.t + q.org_y, (int64_t)d2.r + q.org_x, (int64_t)d2.b + q.org_y };

                if (fle_raster_wrote) fle_raster_wrote(surf, wr, api);
            }
        }
    }
    /* the uncovered area: (src inside clip) minus dst, as up to four rectangles; its bounding rectangle is what is reported and invalidated */
    {
        irect sc = rect_and(src, clip);
        irect a[4];
        int i;
        parts = 0;
        if (rect_empty(&sc)) {
            memset(&unc, 0, sizeof unc);
        } else if (rect_empty(&dst)) {
            a[parts++] = sc;
        } else {
            irect top = { sc.l, sc.t, sc.r, dst.t }, bot = { sc.l, dst.b, sc.r, sc.b }, lef = { sc.l, dst.t, dst.l, dst.b }, rig = { dst.r, dst.t, sc.r, dst.b };
            if (!rect_empty(&top)) a[parts++] = top;
            if (!rect_empty(&bot)) a[parts++] = bot;
            if (!rect_empty(&lef)) a[parts++] = lef;
            if (!rect_empty(&rig)) a[parts++] = rig;
        }
        memset(&unc, 0, sizeof unc);
        for (i = 0; i < parts; i++) {
            if (i == 0) unc = a[0];
            else {
                if (a[i].l < unc.l) unc.l = a[i].l;
                if (a[i].t < unc.t) unc.t = a[i].t;
                if (a[i].r > unc.r) unc.r = a[i].r;
                if (a[i].b > unc.b) unc.b = a[i].b;
            }
        }
    }
    type = parts == 0 ? 1 : parts == 1 ? 2 : 3;
    if (hrgn != 0) {   /* the region that holds the uncovered area: a rectangular region (decision 0123's only kind) = its bounding rectangle (S-updregion) */
        uint64_t saved2[16], st2[8] = { 0 };
        memcpy(saved2, R, sizeof saved2);
        R[1] = hrgn; R[2] = (uint64_t)(uint32_t)unc.l; R[8] = (uint64_t)(uint32_t)unc.t; R[9] = (uint64_t)(uint32_t)unc.r; st2[5] = (uint64_t)(uint32_t)unc.b; R[4] = (uint64_t)(uintptr_t)st2;
        fle_imp_SetRectRgn();
        memcpy(R, saved2, sizeof saved2);
    }
    if (prc_update != 0) {
        fle_st(prc_update, 4, (uint32_t)unc.l); fle_st(prc_update + 4, 4, (uint32_t)unc.t); fle_st(prc_update + 8, 4, (uint32_t)unc.r); fle_st(prc_update + 12, 4, (uint32_t)unc.b);
    }
    if ((flags & 0x6u) != 0 && parts > 0) {
        int32_t rc[4] = { unc.l, unc.t, unc.r, unc.b };
        uint64_t saved[16];
        memcpy(saved, R, sizeof saved);
        R[1] = hwnd; R[2] = (uint64_t)(uintptr_t)rc; R[8] = (flags & 0x4u) ? 1u : 0u;
        fle_imp_InvalidateRect();
        memcpy(R, saved, sizeof saved);
    }
    R[0] = (uint64_t)type;
}

/* ---- Decision 0319 (PROVISIONAL; PM 07:2x; O9): ScrollWindowEx and ScrollWindow on the window model of decision 0214 (fle_win.c: fle_win_scroll; the contract, the Wine 10 probes and the model are described there) ----
 * ScrollWindowEx: the bits that exist (the visible region with the window's clip style) move into the clip rectangle (NULL = the whole client area), the uncovered area is reported (prcUpdate: its bounding rectangle;
 * hrgnUpdate: a rectangular region = the bounding rectangle of the uncovered area and the moved update area; the return value is NULLREGION / SIMPLEREGION / COMPLEXREGION by the number of rectangles of the uncovered
 * area) and, with SW_INVALIDATE or SW_ERASE, invalidated; SW_SCROLLCHILDREN moves the child windows (WM_MOVE). ScrollWindow(hWnd, X, Y, lpRect, lpClipRect) = ScrollWindowEx(hWnd, X, Y, lpRect, lpClipRect, NULL, NULL,
 * SW_INVALIDATE | SW_ERASE | (lpRect == NULL ? SW_SCROLLCHILDREN : 0)) (Learn ScrollWindow: "the area uncovered ... is combined into the window's update region"; with lpRect NULL the child windows and the invalid areas
 * are offset, with lpRect not NULL they are not); BOOL, nonzero on success. A handle that is not a live window: ERROR / 0 and ERROR_INVALID_WINDOW_HANDLE (1400, as before). When the model cannot answer (fle_win_scroll
 * returns 0: a build without the region functions, an iconic window) the former simple path answers. */
static int scroll_run(const char *api, uint64_t hwnd, int32_t dx, int32_t dy, uint64_t prc_scroll, uint64_t prc_clip, uint32_t flags, fle_win_scroll_result *res)
{
    int32_t rs[4], rc[4];

    if (prc_scroll != 0) {
        irect s = rect_ld(prc_scroll);
        if (s.r < s.l || s.b < s.t) fle_stop(api, "prcScroll has a negative size: the page gives no behaviour");
        rs[0] = s.l; rs[1] = s.t; rs[2] = s.r; rs[3] = s.b;
    }
    if (prc_clip != 0) {
        irect c = rect_ld(prc_clip);
        if (c.r < c.l || c.b < c.t) fle_stop(api, "prcClip has a negative size: the page gives no behaviour");
        rc[0] = c.l; rc[1] = c.t; rc[2] = c.r; rc[3] = c.b;
    }
    return fle_win_scroll(api, hwnd, dx, dy, prc_scroll != 0 ? rs : NULL, prc_clip != 0 ? rc : NULL, flags, res);
}

void fle_imp_ScrollWindowEx(void)
{
    static const char api[] = "ScrollWindowEx";
    uint64_t hwnd = R[1], prc_scroll = R[9], prc_clip = stack_arg(5), hrgn = stack_arg(6), prc_update = stack_arg(7);
    int32_t dx = (int32_t)R[2], dy = (int32_t)R[8];
    uint32_t flags = (uint32_t)stack_arg(8);
    fle_win_scroll_result sr;
    int type;

    if (!scroll_run(api, hwnd, dx, dy, prc_scroll, prc_clip, flags, &sr)) { scroll_window_ex_simple(); return; }
    if (sr.invalid) {
        fle_set_last_error(1400);       /* ERROR_INVALID_WINDOW_HANDLE */
        R[0] = 0;
        return;
    }
    type = (flags & 0x6u) == 0 || sr.parts == 0 ? 1 : sr.parts == 1 ? 2 : 3;   /* Learn: the type of the INVALIDATED region; without SW_INVALIDATE / SW_ERASE nothing is invalidated (Wine probe U3: NULLREGION, prcUpdate still gives the strip) */
    if (hrgn != 0) {   /* the region that holds the uncovered area and the moved update area: rectangular (decision 0123's only kind) */
        uint64_t saved2[16], st2[8] = { 0 };
        int has = sr.parts > 0 || sr.upd[2] > sr.upd[0];

        memcpy(saved2, R, sizeof saved2);
        R[1] = hrgn; R[2] = (uint64_t)(uint32_t)(has ? sr.upd[0] : 0); R[8] = (uint64_t)(uint32_t)(has ? sr.upd[1] : 0); R[9] = (uint64_t)(uint32_t)(has ? sr.upd[2] : 0);
        st2[5] = (uint64_t)(uint32_t)(has ? sr.upd[3] : 0); R[4] = (uint64_t)(uintptr_t)st2;
        fle_imp_SetRectRgn();
        memcpy(R, saved2, sizeof saved2);
    }
    if (prc_update != 0) {
        int32_t z[4] = { 0, 0, 0, 0 };
        const int32_t *u = sr.parts > 0 ? sr.unc : z;

        fle_st(prc_update, 4, (uint32_t)u[0]); fle_st(prc_update + 4, 4, (uint32_t)u[1]); fle_st(prc_update + 8, 4, (uint32_t)u[2]); fle_st(prc_update + 12, 4, (uint32_t)u[3]);
    }
    R[0] = (uint64_t)type;
}

/* user32!ScrollWindow: BOOL f(HWND hWnd rcx, int XAmount edx, int YAmount r8d, const RECT *lpRect r9, const RECT *lpClipRect [rsp+0x28]) */
void fle_imp_ScrollWindow(void)
{
    static const char api[] = "ScrollWindow";
    uint64_t hwnd = R[1], prc_scroll = R[9], prc_clip = stack_arg(5);
    int32_t dx = (int32_t)R[2], dy = (int32_t)R[8];
    fle_win_scroll_result sr;
    uint32_t flags = 0x2u | 0x4u | (prc_scroll == 0 ? 0x1u : 0x80000000u);   /* SW_INVALIDATE | SW_ERASE | SW_SCROLLCHILDREN when the whole client area is scrolled; with an lpRect the invalid areas are not offset (internal bit) */

    if (!scroll_run(api, hwnd, dx, dy, prc_scroll, prc_clip, flags, &sr)) {
        /* no model: the same as ScrollWindowEx with a NULL hrgnUpdate and prcUpdate, through the former simple path (its arguments are on the stack: build the call) */
        fle_stop(api, "the window model for scrolling is not available in this build (fle_win_scroll answered 0)");
    }
    if (sr.invalid) {
        fle_set_last_error(1400);       /* ERROR_INVALID_WINDOW_HANDLE */
        R[0] = 0;
        return;
    }
    R[0] = 1;
}

/* ---- user32!InvalidateRgn (wall ia2-W31: the engine's scroll helper f_825d20 calls it after ScrollWindowEx with the region ScrollWindowEx filled; met by Alt+F7) ----
 * BOOL f(HWND hWnd rcx, HRGN hRgn rdx, BOOL bErase r8d). Learn (winuser.h, "InvalidateRgn function"): "invalidates the client area within the specified region by adding it to the current update region of a window";
 * hRgn "The region is assumed to have client coordinates. If this parameter is NULL, the entire client area is added to the update region"; bErase as InvalidateRect; "The return value is always nonzero";
 * "The specified region must have been created by using one of the region functions". Model: the update region is ONE bounding rectangle (S-updregion, fle_imp_InvalidateRect), so the region's bounding box
 * (GetRgnBox) is added; an empty region adds nothing; a handle that is not a live region stops by name (the page defines nothing for it). */
void fle_imp_InvalidateRgn(void)
{
    static const char api[] = "InvalidateRgn";
    uint64_t hwnd = R[1], hrgn = R[2], erase = R[8] & 0xFFFFFFFFu, saved[16];
    int32_t rc[4] = { 0, 0, 0, 0 };
    uint64_t kind;

    memcpy(saved, R, sizeof saved);
    if (hrgn == 0) {
        R[1] = hwnd; R[2] = 0; R[8] = erase;
        fle_imp_InvalidateRect();
        memcpy(R, saved, sizeof saved);
        R[0] = 1;
        return;
    }
    R[1] = hrgn; R[2] = (uint64_t)(uintptr_t)rc;
    fle_imp_GetRgnBox();
    kind = R[0];
    memcpy(R, saved, sizeof saved);
    if (kind == 0) fle_stop(api, "hRgn is not a live region: the page defines nothing for it");
    if (kind != 1 && rc[2] > rc[0] && rc[3] > rc[1]) {
        R[1] = hwnd; R[2] = (uint64_t)(uintptr_t)rc; R[8] = erase;
        fle_imp_InvalidateRect();
        memcpy(R, saved, sizeof saved);
    }
    R[0] = 1;
}

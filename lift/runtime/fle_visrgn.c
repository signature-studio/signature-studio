/* Decision 0142 (PROVISIONAL; docs/decisions/0142-window-visible-region.md; fle_visrgn.h). */
#define _GNU_SOURCE
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_sys.h"
#include "fle_thread.h"
#include "fle_visrgn.h"

/* ---- the region ---- */

static int rect_empty(fle_rrect r) { return r.x1 <= r.x0 || r.y1 <= r.y0; }

void fle_visrgn_set_rect(fle_visrgn *v, fle_rrect r)
{
    v->n = 0;
    if (!rect_empty(r)) v->r[v->n++] = r;
}

/* Two rectangles that share a whole side and touch there are one rectangle (keeps the number of pieces small; the region does not change). */
static void coalesce(fle_visrgn *v)
{
    int again = 1;

    while (again) {
        again = 0;
        for (uint32_t i = 0; i < v->n && !again; i++)
            for (uint32_t j = i + 1; j < v->n && !again; j++) {
                fle_rrect *a = &v->r[i], *b = &v->r[j];

                if (a->y0 == b->y0 && a->y1 == b->y1 && (a->x1 == b->x0 || b->x1 == a->x0)) {
                    if (b->x0 < a->x0) a->x0 = b->x0;
                    if (b->x1 > a->x1) a->x1 = b->x1;
                } else if (a->x0 == b->x0 && a->x1 == b->x1 && (a->y1 == b->y0 || b->y1 == a->y0)) {
                    if (b->y0 < a->y0) a->y0 = b->y0;
                    if (b->y1 > a->y1) a->y1 = b->y1;
                } else {
                    continue;
                }
                v->r[j] = v->r[--v->n];
                again = 1;
            }
    }
}

int fle_visrgn_subtract(fle_visrgn *v, fle_rrect s)
{
    fle_visrgn t;

    if (rect_empty(s)) return 1;
    t.n = 0;
    for (uint32_t i = 0; i < v->n; i++) {
        fle_rrect r = v->r[i], q;
        fle_rrect parts[4];
        uint32_t np = 0;

        if (s.x1 <= r.x0 || s.x0 >= r.x1 || s.y1 <= r.y0 || s.y0 >= r.y1) {
            parts[np++] = r;
        } else {
            if (s.y0 > r.y0) { q = r; q.y1 = s.y0; parts[np++] = q; }                       /* above the hole */
            if (s.y1 < r.y1) { q = r; q.y0 = s.y1; parts[np++] = q; }                       /* below it */
            q.y0 = s.y0 > r.y0 ? s.y0 : r.y0;
            q.y1 = s.y1 < r.y1 ? s.y1 : r.y1;
            if (s.x0 > r.x0) { q.x0 = r.x0; q.x1 = s.x0; parts[np++] = q; }                 /* left of it, in the rows of the hole */
            if (s.x1 < r.x1) { q.x0 = s.x1; q.x1 = r.x1; parts[np++] = q; }                 /* right of it */
        }
        for (uint32_t k = 0; k < np; k++) {
            if (t.n == FLE_VISRGN_MAX) return 0;
            t.r[t.n++] = parts[k];
        }
    }
    coalesce(&t);
    *v = t;
    return 1;
}

void fle_visrgn_intersect_rect(fle_visrgn *v, fle_rrect c)
{
    uint32_t k = 0;

    for (uint32_t i = 0; i < v->n; i++) {
        fle_rrect r = v->r[i];

        if (r.x0 < c.x0) r.x0 = c.x0;
        if (r.y0 < c.y0) r.y0 = c.y0;
        if (r.x1 > c.x1) r.x1 = c.x1;
        if (r.y1 > c.y1) r.y1 = c.y1;
        if (!rect_empty(r)) v->r[k++] = r;
    }
    v->n = k;
}

int fle_visrgn_intersect(fle_visrgn *a, const fle_visrgn *b)
{
    fle_visrgn t;

    t.n = 0;
    for (uint32_t i = 0; i < a->n; i++)
        for (uint32_t j = 0; j < b->n; j++) {
            fle_rrect r = a->r[i];

            if (r.x0 < b->r[j].x0) r.x0 = b->r[j].x0;
            if (r.y0 < b->r[j].y0) r.y0 = b->r[j].y0;
            if (r.x1 > b->r[j].x1) r.x1 = b->r[j].x1;
            if (r.y1 > b->r[j].y1) r.y1 = b->r[j].y1;
            if (rect_empty(r)) continue;
            if (t.n == FLE_VISRGN_MAX) return 0;
            t.r[t.n++] = r;
        }
    coalesce(&t);
    *a = t;
    return 1;
}

fle_rrect fle_visrgn_bounds(const fle_visrgn *v)
{
    fle_rrect b = { 0, 0, 0, 0 };

    for (uint32_t i = 0; i < v->n; i++) {
        if (i == 0) {
            b = v->r[0];
            continue;
        }
        if (v->r[i].x0 < b.x0) b.x0 = v->r[i].x0;
        if (v->r[i].y0 < b.y0) b.y0 = v->r[i].y0;
        if (v->r[i].x1 > b.x1) b.x1 = v->r[i].x1;
        if (v->r[i].y1 > b.y1) b.y1 = v->r[i].y1;
    }
    return b;
}

int fle_visrgn_contains(const fle_visrgn *v, int64_t x, int64_t y)
{
    for (uint32_t i = 0; i < v->n; i++)
        if (x >= v->r[i].x0 && x < v->r[i].x1 && y >= v->r[i].y0 && y < v->r[i].y1) return 1;
    return 0;
}

int fle_visrgn_equal_rect(const fle_visrgn *v, fle_rrect r)
{
    return v->n == 1 && v->r[0].x0 == r.x0 && v->r[0].y0 == r.y0 && v->r[0].x1 == r.x1 && v->r[0].y1 == r.y1;
}

/* ---- the clip flags of a DC ----
 * Learn (Window Styles): WS_CLIPCHILDREN "Excludes the area occupied by child windows when drawing occurs within the parent window"; WS_CLIPSIBLINGS "Clips child windows relative to each other; that is, when a particular child window
 * receives a WM_PAINT message, the WS_CLIPSIBLINGS style clips all other overlapping child windows out of the region of the child window to be updated. If WS_CLIPSIBLINGS is not specified and child windows overlap, it is possible,
 * when drawing within the client area of a child window, to draw within the client area of a neighboring child window". Learn (GetDCEx): DCX_CLIPCHILDREN "Excludes the visible regions of all child windows below the window",
 * DCX_CLIPSIBLINGS "Excludes the visible regions of all sibling windows above the window", DCX_PARENTCLIP "Uses the visible region of the parent window. The parent's WS_CLIPCHILDREN and CS_PARENTDC style bits are ignored".
 * The DC that GetDC, GetWindowDC and BeginPaint give follows the two style bits of the window (S-style-clip: the pages for the styles say what the style does to drawing in the window and give GetDC no flags; the DC of
 * GetDCEx follows them only with DCX_USESTYLE, 0x10000, UNOFFICIAL: windows-sys on docs.rs, S-dcx-usestyle) and the flags of GetDCEx. A window of a class with CS_PARENTDC gets the visible region of its parent
 * (Learn Window Class Styles: "Sets the clipping rectangle of the child window to that of the parent window so that the child can draw on the parent"; S-parentdc-region). */
#define VR_DCX_CLIPCHILDREN  0x00000008u
#define VR_DCX_CLIPSIBLINGS  0x00000010u
#define VR_DCX_PARENTCLIP    0x00000020u
#define VR_DCX_USESTYLE      0x00010000u
#define VR_CS_PARENTDC       0x00000080u
#define VR_WS_CLIPSIBLINGS   0x04000000u
#define VR_WS_CLIPCHILDREN   0x02000000u

uint32_t fle_visrgn_dc_flags(int from_getdcex, uint32_t dcx, uint32_t class_style, uint32_t wstyle)
{
    uint32_t f = 0;

    if (from_getdcex) {
        if (dcx & VR_DCX_CLIPCHILDREN) f |= FLE_VR_CLIPCHILDREN;
        if (dcx & VR_DCX_CLIPSIBLINGS) f |= FLE_VR_CLIPSIBLINGS;
        if (dcx & VR_DCX_PARENTCLIP) f |= FLE_VR_PARENTCLIP;
        if (dcx & VR_DCX_USESTYLE) {
            if (wstyle & VR_WS_CLIPCHILDREN) f |= FLE_VR_CLIPCHILDREN;
            if (wstyle & VR_WS_CLIPSIBLINGS) f |= FLE_VR_CLIPSIBLINGS;
        }
        return f;
    }
    if (wstyle & VR_WS_CLIPCHILDREN) f |= FLE_VR_CLIPCHILDREN;
    if (wstyle & VR_WS_CLIPSIBLINGS) f |= FLE_VR_CLIPSIBLINGS;
    if (class_style & VR_CS_PARENTDC) f |= FLE_VR_PARENTCLIP;
    return f;
}

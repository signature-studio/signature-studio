/* Decision 0142 (PROVISIONAL; docs/decisions/0142-window-visible-region.md): the visible region of a window and the diagnostics of the pixel calls that write into it.
 * fle_visrgn.c is pure C over the rectangles of fle_raster.h (fle_rrect: [x0, x1) x [y0, y1)); the window table (fle_win.c) hands it the windows, it never reads the table itself.
 * Every function that other runtime files call is declared weak there, so a test link that leaves this file out behaves as before decision 0142. */
#ifndef FLE_VISRGN_H
#define FLE_VISRGN_H

#include <stdint.h>
#include <stdio.h>

#include "fle_raster.h"

/* ---- the region: a set of disjoint rectangles (decision 0123: a region is a set of rectangles; nothing else is modelled) ----
 * [x0, x1) x [y0, y1) in the surface coordinates of fle_raster.h. The union of the rectangles is the region; an empty rectangle is never held. A function that can run out of room returns 0 and leaves the region as it was. */
#define FLE_VISRGN_MAX 128u
typedef struct {
    uint32_t n;
    fle_rrect r[FLE_VISRGN_MAX];
} fle_visrgn;
void fle_visrgn_set_rect(fle_visrgn *v, fle_rrect r);                         /* v = r (empty: n = 0) */
int fle_visrgn_subtract(fle_visrgn *v, fle_rrect s);                          /* v = v minus s */
void fle_visrgn_intersect_rect(fle_visrgn *v, fle_rrect c);                   /* v = v intersect c */
int fle_visrgn_intersect(fle_visrgn *a, const fle_visrgn *b);                 /* a = a intersect b */
fle_rrect fle_visrgn_bounds(const fle_visrgn *v);                             /* the tightest rectangle around v (empty when n = 0) */
int fle_visrgn_contains(const fle_visrgn *v, int64_t x, int64_t y);
int fle_visrgn_equal_rect(const fle_visrgn *v, fle_rrect r);                  /* v is exactly the one rectangle r */

/* The window table's side (fle_win.c): the visible region of the DC of window `hwnd` for the clip flags below, in the surface coordinates of the top-level window. 1 and the region; 0 when the window is not in the
 * table; -1 when the region needs more than FLE_VISRGN_MAX rectangles. */
int fle_win_dc_region(uint64_t hwnd, uint32_t clip_flags, fle_visrgn *out);
/* The same, but only the part of the region that lies inside `within` (surface coordinates; NULL: all of it). (A - B) intersect R = (A intersect R) - B, so the answer is the same set as fle_win_dc_region's cut to R; but the children and
 * siblings that do not touch R subtract nothing, so the region needs only the rectangles that lie inside R. A DC made by BeginPaint draws only inside the update rectangle (decision 0240). */
int fle_win_dc_region_within(uint64_t hwnd, uint32_t clip_flags, const fle_rrect *within, fle_visrgn *out);

/* What the clipping of a DC excludes. clip_children: the visible child windows of the window; clip_siblings: the visible sibling windows above it in the z-order; parent_clip: the DC takes the visible region of the
 * PARENT window (the window's own clip_siblings is not used; the parent's clip_children is ignored). */
#define FLE_VR_CLIPCHILDREN 1u
#define FLE_VR_CLIPSIBLINGS 2u
#define FLE_VR_PARENTCLIP   4u
/* The clip flags of a DC: from_getdcex = the DC came from GetDCEx with the flags `dcx` (else GetDC, GetWindowDC or BeginPaint: `dcx` is 0), the class style and the window style. See the decision for each rule and its source. */
uint32_t fle_visrgn_dc_flags(int from_getdcex, uint32_t dcx, uint32_t class_style, uint32_t wstyle);


#endif

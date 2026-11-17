/* comctl32 image lists for the drag of an image (decision 0319 section 14, alpha1, PROVISIONAL). See fle_imagelist.h for what is and is not implemented.
 *
 * Pages: Learn ImageList_Create ("creates a new image list": cx, cy "the width / height, in pixels, of each image", flags ILC_MASK 0x1 and ILC_COLOR* (4, 8, 16, 24, 32 = 0x20, ILC_COLORDDB 0xFE), cInitial, cGrow; NULL on failure),
 * ImageList_Destroy, GetImageCount, GetIconSize ("retrieves the dimensions of images in an image list. All images in an image list have the same dimensions"), SetIconSize ("sets the dimensions of images in an image list and
 * removes all images from the list"), GetBkColor / SetBkColor ("the background color of an image list. This function only works if you add an icon or use ImageList_AddMasked with a black and white bitmap. Without a
 * mask, the entire image draws, so the background color is not visible"; CLR_NONE draws transparently; SetBkColor returns the previous color, CLR_NONE on failure), ImageList_Add ("the number of images is inferred from the
 * width of the bitmap"; "the bitmaps are copied into the image list's internal structure; the caller deletes them"; returns the index of the first new image, -1 on failure; the mask is ignored for a list without ILC_MASK),
 * ImageList_Replace, ImageList_BeginDrag ("creates a temporary image list that the system uses for dragging an image"; himlTrack, iTrack, dxHotspot "the x-position of the hot spot within the new image that
 * defines the hotspot"), DragEnter ("locks updates to the specified window during a drag operation and displays the drag image at the specified position within the window"; x, y "relative to the upper-left corner of the
 * window, not the client area"), DragMove ("moves the image that is being dragged"), DragLeave ("unlocks the specified window and hides the drag image, allowing the window to be updated"), DragShowNolock ("shows or hides the
 * image being dragged"; "does not lock the window"), EndDrag ("ends a drag operation"), GetDragImage ("retrieves the temporary image list that is used for the drag image. The function also retrieves the current drag position
 * and the offset of the drag image relative to the drag position").
 * Where the pages are silent the reference is Wine 10.0 (dlls/comctl32/imagelist.c; UNOFFICIAL): BeginDrag fails when himlTrack or iTrack is not valid or a drag is already begun; the drag position is the CURSOR position
 * and the image is drawn at (x - dxHotspot, y - dyHotspot); the drag image is laid over the window with alpha 192/255 (ILS_ALPHA, Frame 192) and the pixels it covered are saved and put back when it moves or hides;
 * DragShowNolock returns FALSE when the image is already shown / hidden; DragEnter fails without a drag; the window of a NULL hwndLock is the desktop; GetDragImage gives the position (x, y) and the hotspot;
 * the list's background color starts as CLR_NONE. Drawing goes through a window DC made with GetDCEx(DCX_WINDOW | DCX_CACHE | DCX_LOCKWINDOWUPDATE) like Wine's, which draws while a LockWindowUpdate is in effect
 * (decision 0319 section 13: the exception of Learn GetDCEx). DragEnter takes LockWindowUpdate on the window (Learn); a failure to lock (another window is locked) does not stop the drag.
 * A list stores its images as 32-bit B, G, R, A bytes (straight alpha): A = 255 where the image is opaque, 0 where the mask is set (a 1 bit of the mask = transparent: Learn Bitmap Masks), the per-pixel alpha of a 32-bit DIB
 * section when the list is ILC_COLOR32 and the source has any alpha, else 255. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_env.h"
#include "fle_gdi.h"
#include "fle_gdc.h"
#include "fle_imagelist.h"
#include "fle_imports.h"
#include "fle_raster.h"
#include "fle_runtime.h"
#include "fle_win.h"

#define IL_MAX 64u
#define CLR_NONE_ 0xFFFFFFFFu
#define ILC_MASK_ 0x1u
#define ILC_COLOR_MASK_ 0xFEu
#define ILC_COLOR32_ 0x20u
#define DRAG_ALPHA 192u

void fle_imp_LockWindowUpdate(void) __attribute__((weak));
void fle_imp_GetDCEx(void) __attribute__((weak));
void fle_imp_ReleaseDC(void) __attribute__((weak));

typedef struct {
    int used;
    int32_t cx, cy, count, cap;
    uint32_t flags, bk;
    uint32_t *px;                 /* count images of cx * cy pixels, image k at px + k * cx * cy */
} il_t;

static il_t ils[IL_MAX];

static int il_index(uint64_t h)
{
    uint64_t off;

    if (h < FLE_IL_HANDLE_BASE + 4u || h >= FLE_IL_HANDLE_BASE + FLE_IL_REGION_SIZE || ((h - FLE_IL_HANDLE_BASE) & 3u) != 0) return -1;
    off = (h - FLE_IL_HANDLE_BASE) / 4u - 1u;
    if (off >= IL_MAX || !ils[off].used) return -1;
    return (int)off;
}
static uint64_t il_handle(int k) { return FLE_IL_HANDLE_BASE + 4u * ((uint64_t)k + 1u); }

static int il_new(int32_t cx, int32_t cy, uint32_t flags, int32_t cinitial)
{
    int k;

    for (k = 0; k < (int)IL_MAX; k++) {
        if (!ils[k].used) {
            memset(&ils[k], 0, sizeof ils[k]);
            ils[k].used = 1;
            ils[k].cx = cx; ils[k].cy = cy;
            ils[k].flags = flags;
            ils[k].bk = CLR_NONE_;
            ils[k].cap = cinitial > 0 ? cinitial : 4;
            ils[k].px = (uint32_t *)calloc((size_t)ils[k].cap * (size_t)cx * (size_t)cy, 4);
            if (!ils[k].px) { ils[k].used = 0; return -1; }
            return k;
        }
    }
    return -1;
}

static void il_free(int k)
{
    free(ils[k].px);
    memset(&ils[k], 0, sizeof ils[k]);
}

static int il_grow(il_t *l, int32_t want)
{
    uint32_t *p;
    int32_t ncap = l->cap;

    if (want <= l->cap) return 1;
    while (ncap < want) ncap *= 2;
    p = (uint32_t *)realloc(l->px, (size_t)ncap * (size_t)l->cx * (size_t)l->cy * 4u);
    if (!p) return 0;
    l->px = p;
    l->cap = ncap;
    return 1;
}

int fle_il_query(uint64_t himl, fle_il_info *out)
{
    int k = il_index(himl);

    if (k < 0) return 0;
    out->cx = ils[k].cx; out->cy = ils[k].cy; out->count = ils[k].count; out->flags = ils[k].flags; out->bk = ils[k].bk;
    return 1;
}

/* the pixels of image `idx` of list `l` out of a bitmap (and its mask) at column `col` of the bitmap: 1 on success */
static int il_load(il_t *l, int32_t idx, const fle_rsurf *img, const fle_rsurf *mask, int32_t col)
{
    int32_t x, y;
    int per_pixel_alpha = 0;
    uint32_t *dst = l->px + (size_t)idx * (size_t)l->cx * (size_t)l->cy;

    if (!fle_r_readable(img)) return 0;
    if ((l->flags & ILC_COLOR_MASK_) == ILC_COLOR32_ && img->bpp == 32 && img->comp == 0) {
        for (y = 0; y < l->cy && !per_pixel_alpha; y++) {
            const uint8_t *row = fle_r_row(img, (uint32_t)y);

            for (x = 0; x < l->cx; x++) if (row[4 * ((size_t)col * (size_t)l->cx + (size_t)x) + 3] != 0) { per_pixel_alpha = 1; break; }
        }
    }
    for (y = 0; y < l->cy; y++) {
        for (x = 0; x < l->cx; x++) {
            uint32_t sx = (uint32_t)((int64_t)col * l->cx + x), a = 255u, c;

            if (per_pixel_alpha) {
                const uint8_t *p = fle_r_row(img, (uint32_t)y) + 4u * (size_t)sx;

                dst[(size_t)y * (size_t)l->cx + (size_t)x] = (uint32_t)p[0] | ((uint32_t)p[1] << 8) | ((uint32_t)p[2] << 16) | ((uint32_t)p[3] << 24);
                continue;
            }
            c = fle_r_get(img, sx, (uint32_t)y);               /* COLORREF 0x00BBGGRR */
            if ((l->flags & ILC_MASK_) && mask && fle_r_readable(mask) && sx < mask->width && (uint32_t)y < mask->height && (fle_r_get(mask, sx, (uint32_t)y) & 0xFFFFFFu) != 0) a = 0;
            dst[(size_t)y * (size_t)l->cx + (size_t)x] = ((c >> 16) & 0xFFu) | (c & 0xFF00u) | ((c & 0xFFu) << 16) | (a << 24);
        }
    }
    return 1;
}

static void il_stop(const char *api, const char *why) __attribute__((noreturn));
static void il_stop(const char *api, const char *why)
{
    char buf[300];

    snprintf(buf, sizeof buf, "%s (decision 0319 section 14: only the drag of an image is implemented for image lists; nothing observed calls this one yet)", why);
    fle_stop(api, buf);
}

/* ---- layer A ---- */
void fle_imp_ImageList_Create(void)
{
    int32_t cx = (int32_t)(uint32_t)R[1], cy = (int32_t)(uint32_t)R[2], cinit = (int32_t)(uint32_t)R[9];
    uint32_t flags = (uint32_t)R[8];
    int k;

    if (cx <= 0 || cy <= 0 || cinit < 0 || (int64_t)cx * cy > 4096 * 4096) { R[0] = 0; return; }
    k = il_new(cx, cy, flags, cinit);
    R[0] = k < 0 ? 0 : il_handle(k);
}

void fle_imp_ImageList_Destroy(void)
{
    int k = il_index(R[1]);

    if (k < 0) { R[0] = 0; return; }
    il_free(k);
    R[0] = 1;
}

void fle_imp_ImageList_GetImageCount(void)
{
    int k = il_index(R[1]);

    R[0] = k < 0 ? 0 : (uint32_t)ils[k].count;
}

void fle_imp_ImageList_GetIconSize(void)
{
    int k = il_index(R[1]);
    uint64_t pcx = R[2], pcy = R[8];

    if (k < 0 || (pcx == 0 && pcy == 0)) { R[0] = 0; return; }
    if (pcx) fle_st(pcx, 4, (uint32_t)ils[k].cx);
    if (pcy) fle_st(pcy, 4, (uint32_t)ils[k].cy);
    R[0] = 1;
}

void fle_imp_ImageList_SetIconSize(void)
{
    int k = il_index(R[1]);
    int32_t cx = (int32_t)(uint32_t)R[2], cy = (int32_t)(uint32_t)R[8];
    uint32_t *p;

    if (k < 0 || cx <= 0 || cy <= 0 || (int64_t)cx * cy > 4096 * 4096) { R[0] = 0; return; }
    p = (uint32_t *)calloc((size_t)(ils[k].cap > 0 ? ils[k].cap : 4) * (size_t)cx * (size_t)cy, 4);
    if (!p) { R[0] = 0; return; }
    free(ils[k].px);
    ils[k].px = p;
    ils[k].cx = cx; ils[k].cy = cy; ils[k].count = 0;       /* Learn: "removes all images from the list" */
    R[0] = 1;
}

void fle_imp_ImageList_GetBkColor(void)
{
    int k = il_index(R[1]);

    R[0] = k < 0 ? CLR_NONE_ : ils[k].bk;
}

void fle_imp_ImageList_SetBkColor(void)
{
    int k = il_index(R[1]);
    uint32_t old;

    if (k < 0) { R[0] = CLR_NONE_; return; }
    old = ils[k].bk;
    ils[k].bk = (uint32_t)R[2];
    R[0] = old;
}

/* ---- layer B: Add / Replace ---- */
void fle_imp_ImageList_Add(void)
{
    static const char api[] = "ImageList_Add";
    int k = il_index(R[1]);
    fle_rsurf img, mask;
    int32_t n, first, j;
    int have_mask = 0;

    if (k < 0 || R[2] == 0 || !fle_gdi_bitmap_view(R[2], &img) || img.width == 0 || img.height < (uint32_t)ils[k].cy) { R[0] = (uint32_t)-1; return; }
    if (R[8] != 0 && (ils[k].flags & ILC_MASK_)) have_mask = fle_gdi_bitmap_view(R[8], &mask);
    n = (int32_t)(img.width / (uint32_t)ils[k].cx);                  /* Learn: the number of images is inferred from the width of the bitmap */
    if (n <= 0) { R[0] = (uint32_t)-1; return; }
    first = ils[k].count;
    if (!il_grow(&ils[k], first + n)) { R[0] = (uint32_t)-1; return; }
    for (j = 0; j < n; j++) {
        if (!il_load(&ils[k], first + j, &img, have_mask ? &mask : NULL, j)) il_stop(api, "the bitmap has a pixel format that the image list cannot read");
    }
    ils[k].count = first + n;
    R[0] = (uint32_t)first;
}

void fle_imp_ImageList_Replace(void)
{
    static const char api[] = "ImageList_Replace";
    int k = il_index(R[1]);
    int32_t i = (int32_t)(uint32_t)R[2];
    fle_rsurf img, mask;
    int have_mask = 0;

    if (k < 0 || i < 0 || i >= ils[k].count || R[8] == 0 || !fle_gdi_bitmap_view(R[8], &img) || img.width < (uint32_t)ils[k].cx || img.height < (uint32_t)ils[k].cy) { R[0] = 0; return; }
    if (R[9] != 0 && (ils[k].flags & ILC_MASK_)) have_mask = fle_gdi_bitmap_view(R[9], &mask);
    if (!il_load(&ils[k], i, &img, have_mask ? &mask : NULL, 0)) il_stop(api, "the bitmap has a pixel format that the image list cannot read");
    R[0] = 1;
}

/* ---- layer E: the drag ---- */
static struct {
    int active;                   /* BeginDrag without EndDrag */
    int slot;                     /* the temporary list (one image) */
    uint64_t hwnd;                /* the window of DragEnter (the desktop for NULL) */
    int locked;                   /* DragEnter took LockWindowUpdate */
    int32_t x, y, dx, dy;
    int shown;
    uint32_t *saved;              /* the pixels under the image while it is shown */
    uint8_t *valid;
} drag;

/* a window DC like Wine's: GetDCEx(hwnd, NULL, DCX_WINDOW | DCX_CACHE | DCX_LOCKWINDOWUPDATE); 0 when it cannot be made */
static uint64_t drag_dc(void)
{
    uint64_t sv[16], dc;

    if (!fle_imp_GetDCEx) fle_stop("ImageList_DragEnter", "GetDCEx is not linked");
    memcpy(sv, R, sizeof sv);
    R[1] = drag.hwnd == FLE_DESKTOP_HANDLE ? 0 : drag.hwnd; R[2] = 0; R[8] = 0x403u;
    fle_imp_GetDCEx();
    dc = R[0];
    memcpy(R, sv, sizeof sv);
    return dc;
}

static void drag_release(uint64_t dc)
{
    uint64_t sv[16];

    if (!fle_imp_ReleaseDC) return;
    memcpy(sv, R, sizeof sv);
    R[1] = drag.hwnd == FLE_DESKTOP_HANDLE ? 0 : drag.hwnd; R[2] = dc;
    fle_imp_ReleaseDC();
    memcpy(R, sv, sizeof sv);
}

static void drag_lock(uint64_t hwnd)
{
    uint64_t sv[16];

    if (!fle_imp_LockWindowUpdate) return;
    memcpy(sv, R, sizeof sv);
    R[1] = hwnd;
    fle_imp_LockWindowUpdate();
    if (hwnd != 0) drag.locked = (R[0] & 0xFFFFFFFFu) != 0;
    memcpy(R, sv, sizeof sv);
}

/* show (1) or hide (0) the drag image: 1 on success, 0 when it already is so or no drag */
static int drag_show(const char *api, int show)
{
    il_t *l;
    uint64_t dc;
    int32_t x, y;

    if (!drag.active) return 0;
    if ((drag.shown && show) || (!drag.shown && !show)) return 0;
    l = &ils[drag.slot];
    x = drag.x - drag.dx;
    y = drag.y - drag.dy;
    dc = drag_dc();
    if (dc == 0) return 0;
    if (show) {
        size_t n = (size_t)l->cx * (size_t)l->cy;

        free(drag.saved); free(drag.valid);
        drag.saved = (uint32_t *)calloc(n, 4);
        drag.valid = (uint8_t *)calloc(n, 1);
        if (!drag.saved || !drag.valid) fle_stop(api, "out of memory");
        if (!fle_gdi_dc_blend32(api, dc, x, y, l->cx, l->cy, l->px, DRAG_ALPHA, drag.saved, drag.valid)) fle_stop(api, "the window's drawing surface is not a 32-bit surface that can be written");
    } else if (drag.saved) {
        if (!fle_gdi_dc_put32(api, dc, x, y, l->cx, l->cy, drag.saved, drag.valid)) fle_stop(api, "the window's drawing surface is not a 32-bit surface that can be written");
    }
    drag.shown = show;
    drag_release(dc);
    return 1;
}

void fle_imp_ImageList_BeginDrag(void)
{
    int k = il_index(R[1]), t;
    int32_t i = (int32_t)(uint32_t)R[2];

    if (k < 0 || i < 0 || i >= ils[k].count || drag.active) { R[0] = 0; return; }
    t = il_new(ils[k].cx, ils[k].cy, ils[k].flags, 1);
    if (t < 0) { R[0] = 0; return; }
    memcpy(ils[t].px, ils[k].px + (size_t)i * (size_t)ils[k].cx * (size_t)ils[k].cy, (size_t)ils[k].cx * (size_t)ils[k].cy * 4u);
    ils[t].count = 1;
    ils[t].bk = ils[k].bk;
    memset(&drag, 0, sizeof drag);
    drag.active = 1;
    drag.slot = t;
    drag.dx = (int32_t)(uint32_t)R[8];
    drag.dy = (int32_t)(uint32_t)R[9];
    R[0] = 1;
}

void fle_imp_ImageList_DragEnter(void)
{
    uint64_t hwnd = R[1];

    if (!drag.active) { R[0] = 0; return; }
    drag.hwnd = hwnd ? hwnd : FLE_DESKTOP_HANDLE;
    drag.x = (int32_t)(uint32_t)R[2];
    drag.y = (int32_t)(uint32_t)R[8];
    if (hwnd) drag_lock(hwnd);                                    /* Learn: "locks updates to the specified window during a drag operation" */
    R[0] = drag_show("ImageList_DragEnter", 1) ? 1u : 0u;
}

void fle_imp_ImageList_DragMove(void)
{
    int32_t x = (int32_t)(uint32_t)R[1], y = (int32_t)(uint32_t)R[2];

    if (!drag.active) { R[0] = 0; return; }
    if (drag.shown) {
        (void)drag_show("ImageList_DragMove", 0);               /* put the pixels back, then save and draw at the new place */
        drag.x = x; drag.y = y;
        (void)drag_show("ImageList_DragMove", 1);
    } else {
        drag.x = x; drag.y = y;
    }
    R[0] = 1;
}

void fle_imp_ImageList_DragLeave(void)
{
    if (drag.active) (void)drag_show("ImageList_DragLeave", 0);
    if (drag.locked) {
        uint64_t sv[16];

        memcpy(sv, R, sizeof sv);
        R[1] = 0;
        if (fle_imp_LockWindowUpdate) fle_imp_LockWindowUpdate();   /* "unlocks the specified window": the one that DragEnter locked */
        memcpy(R, sv, sizeof sv);
        drag.locked = 0;
    }
    R[0] = 1;
}

void fle_imp_ImageList_DragShowNolock(void)
{
    R[0] = drag_show("ImageList_DragShowNolock", (R[1] & 0xFFFFFFFFu) != 0) ? 1u : 0u;
}

void fle_imp_ImageList_EndDrag(void)
{
    if (drag.active) il_free(drag.slot);
    free(drag.saved); free(drag.valid);
    memset(&drag, 0, sizeof drag);
}

void fle_imp_ImageList_GetDragImage(void)
{
    uint64_t ppt = R[1], phs = R[2];

    if (!drag.active) { R[0] = 0; return; }
    if (ppt) { fle_st(ppt, 4, (uint32_t)drag.x); fle_st(ppt + 4u, 4, (uint32_t)drag.y); }
    if (phs) { fle_st(phs, 4, (uint32_t)drag.dx); fle_st(phs + 4u, 4, (uint32_t)drag.dy); }
    R[0] = il_handle(drag.slot);
}

int fle_il_drag_active(int *shown)
{
    if (shown) *shown = drag.shown;
    return drag.active;
}

/* ---- named stops ---- */
void fle_imp_ImageList_Remove(void) { il_stop("ImageList_Remove", "removing images"); }
void fle_imp_ImageList_SetImageCount(void) { il_stop("ImageList_SetImageCount", "setting the image count"); }
void fle_imp_ImageList_GetIcon(void) { il_stop("ImageList_GetIcon", "making an icon of an image"); }
void fle_imp_ImageList_Draw(void) { il_stop("ImageList_Draw", "drawing an image"); }
void fle_imp_ImageList_DrawEx(void) { il_stop("ImageList_DrawEx", "drawing an image"); }
void fle_imp_ImageList_Read(void) { il_stop("ImageList_Read", "reading a list from a stream"); }
void fle_imp_ImageList_Write(void) { il_stop("ImageList_Write", "writing a list to a stream"); }

/* The virtual GDI object table (docs/decisions/0054-virtual-gdi-objects.md; the pages read are listed there and in
 * docs/investigations/gdi-objects-caller.md). Single guest thread. A slot is {never issued, live, deleted}; a deleted slot is
 * never reused. A case the Microsoft pages leave open, an object kind this table does not hold, and every argument the guest
 * does not use, stops the run through fle_stop (fle_env.h). Nothing here reads the host: no display, no resource file, no
 * clock, no thread other than the id of the calling one. */
#define _GNU_SOURCE
#include <inttypes.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

#include "fle_runtime.h"
#include "fle_host.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_gdi.h"
#include "fle_gdc.h"
#include "fle_raster.h"
#include "fle_visrgn.h"
#include "fle_win.h"
#include "fle_sysmetrics.h"
#include "fle_callback.h"
#include "fle_spi.h"

static int64_t fle_gdi_owner_tid;

static void fle_gdi_check_thread(const char *api)
{
    int64_t tid = (int64_t)fle_host_tid();

    if (fle_gdi_owner_tid == 0) fle_gdi_owner_tid = tid;
    if (tid != fle_gdi_owner_tid) fle_stop(api, "called from a second thread, not supported");
}

#define ST_NEVER   0u
#define ST_LIVE    1u
#define ST_DELETED 2u

/* Decision 0108: the state of a memory DC (the handles of what is selected into it and the two colours) and of a DIB section (the
 * block of pixels that the guest gets a pointer to, the format and the colour table). Both hang off gdi_obj.ext. */
/* Decision 0126 (PROVISIONAL): the application-defined clipping region of a DC (Learn GetClipRgn: "an application-defined clipping region is a clipping region identified by the SelectClipRgn function") as a
 * set of rectangles in DEVICE units ("The SelectClipRgn function assumes that the coordinates for a region are specified in device units"), the normal form of decision 0123 (at most 64). has = 0: there is none. */
/* Decision 0229 (ref1, PROVISIONAL): the clip and the regions are no longer limited to 64 rectangles (decision 0123 and 0126 had a fixed array of 64 per clip and per region, and 192 cells per combine; the reference project's
 * playlist reaches a CombineRgn(DIFF) of a 52-rectangle region with one rectangle whose result has more than 192 cells). They are heap arrays that grow; the only limit is memory. */
typedef struct { int32_t l, t, r, b; } gdi_crect;
typedef struct {
    int has;
    uint32_t n, cap;
    gdi_crect *r;                           /* cap entries (NULL while cap is 0); the first n are the region */
} gdi_clip;

static void clip_reserve(gdi_clip *c, uint32_t n)
{
    if (n > c->cap) {
        uint32_t nc = c->cap ? c->cap : 4u;
        gdi_crect *p;

        while (nc < n) nc *= 2u;
        p = (gdi_crect *)realloc(c->r, (size_t)nc * sizeof *p);
        if (!p) fle_stop("clip", "out of memory");
        c->r = p;
        c->cap = nc;
    }
}

static void clip_copy(gdi_clip *d, const gdi_clip *s)
{
    clip_reserve(d, s->n);
    if (s->n) memcpy(d->r, s->r, (size_t)s->n * sizeof *d->r);
    d->has = s->has;
    d->n = s->n;
}

static void clip_free(gdi_clip *c)
{
    free(c->r);
    c->r = NULL;
    c->n = c->cap = 0;
    c->has = 0;
}

typedef struct {
    uint64_t bmp, brush, pen, font, pal;    /* decision 0111: one state of the DC stack (SaveDC): the objects selected, the graphic modes ... */
    uint32_t bk_mode, rop2, map_mode, stretch, text, bk;
    int32_t pos[2], worg[2], vorg[2], borg[2];
    gdi_clip clip;                          /* decision 0126: the SaveDC page lists the region among the saved items */
} gdi_dcsave;

typedef struct {
    uint64_t bmp, brush, pen, font, pal;    /* handles of the objects selected into the DC */
    uint64_t placeholder;                   /* the 1-by-1 monochrome bitmap the DC was created with */
    uint32_t text, bk;                      /* COLORREF */
    uint32_t bk_mode, rop2, map_mode, stretch;       /* decision 0111: the graphic modes (fle_gdi.h: the defaults of a new DC) */
    int32_t pos[2], worg[2], vorg[2], borg[2];       /* current position, window origin, viewport origin, brush origin (x, y) */
    gdi_dcsave *stack;                      /* SaveDC: stack[0 .. depth - 1], the state numbers are 1 .. depth */
    uint32_t depth, cap;
    gdi_clip clip;                          /* decision 0126: the application-defined clipping region (device units) */
} gdi_dc;

typedef struct {
    uint8_t *bits;                          /* zero-filled block, stride * |height| bytes; the guest writes it, this runtime never does */
    uint32_t width, bpp, comp, stride, ncolors;
    int32_t height;                         /* as in the header: negative = top-down */
    uint32_t xppm, yppm, clrused, clrimp;   /* decision 0113: biXPelsPerMeter, biYPelsPerMeter, biClrUsed and biClrImportant as the guest passed them (GetObject echoes them) */
    uint32_t masks[3];                      /* BI_BITFIELDS only */
    uint32_t clr[256];                      /* RGBQUAD of the colour table as blue | green << 8 | red << 16 | reserved << 24 (bpp <= 8 only) */
} gdi_dib;

/* Decision 0126 (PROVISIONAL): a device-dependent bitmap keeps pixels. A colour one (CreateCompatibleBitmap of the screen or of a DC with a colour bitmap) is a 32-bit BGRX block (the format of the virtual display,
 * fle_gdc.h BITSPIXEL 32 and PLANES 1: the pages say a DDB has "the color organization of that device context"); a monochrome one (CreateBitmap with 1 plane and 1 bit, and the 1-by-1 bitmap of a new memory DC) a
 * 1-bit block with the colour table black and white (the pages: "a bit value of zero represents a black pixel and a bit value of 1 represents a white pixel"). The block is made when the bitmap is first drawn into
 * or read (S-ddb-lazy: no memory for a bitmap the guest only measures) and starts as zero bytes (S-ddb-init: black; the CreateBitmap page says "the contents of the new bitmap are undefined"). */
typedef struct {
    uint8_t *bits;
    uint32_t stride;
} gdi_ddb;

typedef struct {
    uint8_t state;
    uint8_t kind;
    uint32_t pal[FLE_GDI_PALETTE_ENTRIES];  /* palettes only: peRed | peGreen << 8 | peBlue << 16 | peFlags << 24 */
    uint8_t lf[FLE_GDI_LOGFONTW_SIZE];      /* created fonts only (decision 0066): the copy of the guest's LOGFONTW */
    uint32_t bm[4];                         /* created bitmaps only (decision 0106): width, height, planes, bit count */
    uint8_t fmt;                            /* bitmaps: FLE_GDI_BMP_* (0 = monochrome, the CreateBitmap form); default objects: which one */
    uint8_t flags;                          /* GF_* */
    uint32_t sel;                           /* number of DCs the object is selected into at this moment (decision 0108) */
    uint32_t u[6];                          /* brush: style, COLORREF; icon or cursor: fIcon, xHotspot, yHotspot, width, height of the mask, has colour bitmap */
    void *ext;                              /* memory DC: gdi_dc; DIB section: gdi_dib; device-dependent bitmap: gdi_ddb (decision 0126, made on first use) */
    uint64_t key;                           /* decision 0118: an icon of LoadIconW(hInstance, name) (GF_MODICON): the VA of its RT_GROUP_ICON data, the identity of the group in its image */
} gdi_obj;

#define GF_PLACEHOLDER 1u                   /* the 1-by-1 monochrome bitmap of a memory DC */
#define GF_OPAQUE_PAL  2u                   /* a halftone palette: the entries are an implementation detail (the page), none are kept */
#define GF_MODICON     4u                   /* decision 0118: a shared icon loaded from a module's RT_GROUP_ICON by LoadIconW(hInstance, name) (kind FLE_GDI_KIND_ICON) */

static gdi_obj *objs;
static uint64_t objs_n, objs_cap, objs_live, pal_live, font_live, bmp_live, dc_live, brush_live, pen_live;
static uint64_t cur_cursor;                 /* decision 0111: the handle SetCursor set last (0: none) */
static int32_t show_count;                  /* decision 0111: the display count of ShowCursor (0: Learn's initial count when a mouse is installed) */
static uint64_t dc_default[4];              /* decision 0108: handles of the default brush, pen, font and palette (0 until the first memory DC) */

static gdi_obj *slot_get(const char *api, uint64_t slot)
{
    if (slot >= FLE_GDI_REGION_SIZE / 4u - 2u) fle_stop(api, "GDI handle region exhausted");
    if (slot >= objs_cap) {
        uint64_t nc = objs_cap ? objs_cap * 2 : 64;
        gdi_obj *n;

        while (nc <= slot) nc *= 2;
        n = (gdi_obj *)realloc(objs, nc * sizeof *objs);
        if (!n) fle_stop(api, "out of memory");
        objs = n;
        for (uint64_t i = objs_cap; i < nc; i++) memset(&objs[i], 0, sizeof objs[i]);   /* ST_NEVER, FLE_GDI_KIND_NONE, no selection */
        objs_cap = nc;
    }
    if (slot >= objs_n) objs_n = slot + 1;
    return &objs[slot];
}

static uint64_t slot_handle(uint64_t slot) { return FLE_GDI_HANDLE_BASE + 4u * (slot + 1u); }

/* The slot of a value that this table issued at some time (live or deleted), or -1. */
static int64_t slot_of(uint64_t h)
{
    uint64_t off, i;

    if (h <= FLE_GDI_HANDLE_BASE || h >= FLE_GDI_HANDLE_BASE + FLE_GDI_REGION_SIZE) return -1;
    off = h - FLE_GDI_HANDLE_BASE;
    if (off % 4u) return -1;
    i = off / 4u - 1u;
    if (i >= objs_n || objs[i].state == ST_NEVER) return -1;
    return (int64_t)i;
}

static void issue(gdi_obj *o, uint8_t kind)
{
    o->kind = kind;
    o->state = ST_LIVE;
    objs_live++;
}

int fle_gdi_kind(uint64_t h)
{
    int64_t i = slot_of(h);

    return i >= 0 && objs[i].state == ST_LIVE ? objs[i].kind : FLE_GDI_KIND_NONE;
}

uint64_t fle_gdi_live_count(void) { return objs_live; }
uint64_t fle_gdi_palette_live_count(void) { return pal_live; }
uint64_t fle_gdi_font_live_count(void) { return font_live; }
uint64_t fle_gdi_bitmap_live_count(void) { return bmp_live; }

int fle_gdi_bitmap_info(uint64_t h, uint32_t out[4])
{
    int64_t s = slot_of(h);

    if (fle_gdi_kind(h) != FLE_GDI_KIND_BITMAP) return 0;
    for (uint32_t i = 0; i < 4; i++) out[i] = objs[s].bm[i];
    return 1;
}

int fle_gdi_font_logfont(uint64_t h, uint8_t out[FLE_GDI_LOGFONTW_SIZE])
{
    int64_t s = slot_of(h);

    if (fle_gdi_kind(h) != FLE_GDI_KIND_FONT) return 0;
    for (uint32_t i = 0; i < FLE_GDI_LOGFONTW_SIZE; i++) out[i] = objs[s].lf[i];
    return 1;
}

uint32_t fle_gdi_palette_count(uint64_t h)
{
    int64_t s = slot_of(h);

    return fle_gdi_kind(h) == FLE_GDI_KIND_PALETTE && !(objs[s].flags & GF_OPAQUE_PAL) ? FLE_GDI_PALETTE_ENTRIES : 0;
}

uint32_t fle_gdi_palette_entry(uint64_t h, uint32_t i)
{
    int64_t s = slot_of(h);

    if (fle_gdi_kind(h) != FLE_GDI_KIND_PALETTE || (objs[s].flags & GF_OPAQUE_PAL) || i >= FLE_GDI_PALETTE_ENTRIES) return 0;
    return objs[s].pal[i];
}

/* ---- decision 0108 (PROVISIONAL): memory DCs, DIB sections, brushes, selection ---------------------------------------------------------
 * A memory DC is an object of this table (kind FLE_GDI_KIND_DC; the CreateCompatibleDC page: "you can also call DeleteObject with the HDC to
 * delete the DC"). It holds the handles of what is selected into it, two colours, and a 1-by-1 monochrome placeholder bitmap (the page: "its
 * display surface is exactly one monochrome pixel wide and one monochrome pixel high"). Every selected object counts the DCs it is selected
 * into (DeleteObject: zero "if the specified handle is ... currently selected into a DC").
 * The pixels: only a DIB section has any, in a block the guest gets a pointer to (CreateDIBSection). A device-dependent bitmap keeps its size and
 * format and no pixels (decision 0106). Every operation that would write or read pixels through a DC stops by name (BitBlt, PatBlt, FillRect,
 * GetDIBits): the bytes GDI writes are not on the pages. */

static gdi_obj *live_obj(uint64_t h)
{
    int64_t i = slot_of(h);

    return i >= 0 && objs[i].state == ST_LIVE ? &objs[i] : NULL;
}

/* Decision 0174 (PROVISIONAL): the slot of a deleted object is reused, after a delay. Before, every created object took a slot for good (224 bytes in objs[] and, from a region's slot up, 1,028 bytes in the region
 * table) and an engine that creates regions while it plays filled the table: about 1.25 KB per object, to FLE_GDI_REGION_SIZE / 4 slots and then "GDI handle region exhausted". Learn "GDI Objects": at most 65,536 GDI handles
 * per session and a per-process quota (GDIProcessHandleQuota, 256 to 65,536), a destroyer function "invalidates the object handle", so a Windows process cannot have issued more handles than that and must reuse the
 * entries of deleted objects; the page gives no reuse rule (when a value comes back: choice S-1 of the decision).
 * The handle VALUE format is not changed (FLE_GDI_HANDLE_BASE + 4 * (slot + 1)); the slot is reused first-in first-out and only once FLE_GDI_REUSE_AFTER deleted slots are waiting, so a stale handle of the last
 * FLE_GDI_REUSE_AFTER deleted objects is still told from a live one (DeleteObject 0, the stops on a deleted handle), and a run that deletes fewer objects is identical to the earlier behaviour. */
#define FLE_GDI_REUSE_AFTER 32768u
static uint64_t *dead_q;                    /* ring of the slots of deleted objects, oldest first */
static uint64_t dead_head, dead_n, dead_cap;

static void rgn_forget(uint64_t slot);

static void slot_release(uint64_t slot)
{
    rgn_forget(slot);
    if (slot < FLE_GDI_FIRST_CREATED_SLOT) return;
    if (dead_n == dead_cap) {
        uint64_t nc = dead_cap ? dead_cap * 2 : 1024, *n;

        n = (uint64_t *)malloc(nc * sizeof *n);
        if (!n) return;                     /* out of memory: the slot is simply not reused */
        for (uint64_t k = 0; k < dead_n; k++) n[k] = dead_q[(dead_head + k) % dead_cap];
        free(dead_q);
        dead_q = n;
        dead_head = 0;
        dead_cap = nc;
    }
    dead_q[(dead_head + dead_n) % dead_cap] = slot;
    dead_n++;
}

static uint64_t new_slot(const char *api)
{
    uint64_t slot;

    if (dead_n > FLE_GDI_REUSE_AFTER) {
        slot = dead_q[dead_head];
        dead_head = (dead_head + 1) % dead_cap;
        dead_n--;
        memset(&objs[slot], 0, sizeof objs[slot]);   /* ST_NEVER, FLE_GDI_KIND_NONE, no selection: as a slot that was never used */
        return slot;
    }
    slot = objs_n < FLE_GDI_FIRST_CREATED_SLOT ? FLE_GDI_FIRST_CREATED_SLOT : objs_n;
    slot_get(api, slot);
    return slot;
}

enum { DCK_NONE, DCK_SCREEN, DCK_MEM, DCK_DELETED };

/* What a DC handle is: a DC of GetDC(NULL), a live memory DC (*out set when out is not NULL), a deleted memory DC, or none of these. */
static int dc_class(uint64_t h, gdi_dc **out)
{
    int64_t i;

    if (fle_gdc_is_live(h)) return DCK_SCREEN;
    i = slot_of(h);
    if (i < 0 || objs[i].kind != FLE_GDI_KIND_DC) return DCK_NONE;
    if (objs[i].state == ST_DELETED) return DCK_DELETED;
    if (out) *out = (gdi_dc *)objs[i].ext;
    return DCK_MEM;
}

static gdi_dc *need_mem_dc(const char *api, uint64_t h, const char *screen_why);
static gdi_dc *screen_state(const char *api, uint64_t h);
static void default_objects(const char *api);
static void sel_add(uint64_t h);
static void sel_drop(uint64_t h);

static gdi_dc *need_mem_dc(const char *api, uint64_t h, const char *screen_why)
{
    gdi_dc *dc = NULL;

    switch (dc_class(h, &dc)) {
    case DCK_MEM: return dc;
    case DCK_SCREEN: fle_stop(api, screen_why);
    case DCK_DELETED: fle_stop(api, "hdc was deleted by DeleteDC: the page does not say what the call returns for it");
    default: fle_stop(api, "hdc is not a DC of this runtime (NULL, a number never issued or another kind of handle): the page does not define it");
    }
}

/* Decision 0126: the state of a DC that is drawn on: a memory DC's, or the state of a live window / screen DC (fle_gdi.c keeps one per GetDC handle). */
static gdi_dc *drawn_dc(const char *api, uint64_t h, const char *why)
{
    if (fle_gdc_is_live(h)) return screen_state(api, h);
    return need_mem_dc(api, h, why);
}


/* The four default objects of a memory DC. The pages say that a DC starts with "a set of default objects" ("there is no default bitmap") and do
 * not say which ones (decision 0108, a choice): four hidden objects that every memory DC starts with, so that a guest that saves the previous
 * handle of SelectObject or SelectPalette can select it back. They are created with the first memory DC. */
static void default_objects(const char *api)
{
    uint64_t s[4];

    if (dc_default[0]) return;
    for (uint32_t i = 0; i < 4; i++) s[i] = new_slot(api);
    for (uint32_t i = 0; i < 4; i++) {
        gdi_obj *o = &objs[s[i]];

        o->fmt = (uint8_t)i;
        issue(o, FLE_GDI_KIND_DEFAULT);
        dc_default[i] = slot_handle(s[i]);
    }
}

static void sel_drop(uint64_t h)
{
    gdi_obj *o = live_obj(h);

    if (o && o->sel) o->sel--;
}

static void sel_add(uint64_t h)
{
    gdi_obj *o = live_obj(h);

    if (o) o->sel++;
}

static void obj_kill(gdi_obj *o)
{
    o->state = ST_DELETED;
    o->kind = FLE_GDI_KIND_NONE;
    objs_live--;
    slot_release((uint64_t)(o - objs));
}

static void dib_release(gdi_obj *o)
{
    gdi_dib *d = (gdi_dib *)o->ext;

    if (d) free(d->bits);
    free(d);
    o->ext = NULL;
}

static void ddb_release(gdi_obj *o)
{
    gdi_ddb *d = (gdi_ddb *)o->ext;

    if (d) free(d->bits);
    free(d);
    o->ext = NULL;
}

/* Deletes the memory DC in objs[i]: the objects selected into it are no longer selected (they are not deleted: the SelectObject page asks the
 * application to select the original objects back and nothing in the pages says that the DC deletes what is selected into it), the placeholder
 * goes with the DC. The slot keeps its kind so that a second DeleteDC is told from a handle that was never a DC. */
static void dc_destroy(int64_t i)
{
    gdi_dc *dc = (gdi_dc *)objs[i].ext;
    int64_t ph;

    sel_drop(dc->bmp);
    sel_drop(dc->brush);
    sel_drop(dc->pen);
    sel_drop(dc->font);
    sel_drop(dc->pal);
    ph = slot_of(dc->placeholder);
    if (ph >= 0 && objs[ph].state == ST_LIVE) {
        ddb_release(&objs[ph]);
        obj_kill(&objs[ph]);
        bmp_live--;
    }
    for (uint32_t k = 0; k < dc->depth; k++) clip_free(&dc->stack[k].clip);
    clip_free(&dc->clip);
    free(dc->stack);
    free(dc);
    objs[i].ext = NULL;
    objs[i].state = ST_DELETED;
    objs_live--;
    dc_live--;
    slot_release((uint64_t)i);
}

uint64_t fle_gdi_dc_live_count(void) { return dc_live; }
uint64_t fle_gdi_brush_live_count(void) { return brush_live; }

int fle_gdi_dc_state(uint64_t h, uint64_t sel[5], uint32_t col[2])
{
    gdi_obj *o = live_obj(h);
    gdi_dc *dc;

    if (!o || o->kind != FLE_GDI_KIND_DC) return 0;
    dc = (gdi_dc *)o->ext;
    sel[0] = dc->bmp;
    sel[1] = dc->brush;
    sel[2] = dc->pen;
    sel[3] = dc->font;
    sel[4] = dc->pal;
    col[0] = dc->text;
    col[1] = dc->bk;
    return 1;
}

uint64_t fle_gdi_dc_placeholder(uint64_t h)
{
    gdi_obj *o = live_obj(h);

    return o && o->kind == FLE_GDI_KIND_DC ? ((gdi_dc *)o->ext)->placeholder : 0;
}

uint64_t fle_gdi_default_object(uint32_t which) { return which < 4 ? dc_default[which] : 0; }

uint32_t fle_gdi_selected_count(uint64_t h)
{
    gdi_obj *o = live_obj(h);

    return o ? o->sel : 0;
}

int fle_gdi_bitmap_format(uint64_t h)
{
    gdi_obj *o = live_obj(h);

    return o && o->kind == FLE_GDI_KIND_BITMAP ? (int)o->fmt : -1;
}

/* A live DIB section: 1 and the pointer to its bits in *bits, out = width, |height|, bit count, compression, stride, number of colour table entries. */
int fle_gdi_dib_info(uint64_t h, uint64_t *bits, uint32_t out[6])
{
    gdi_obj *o = live_obj(h);
    gdi_dib *d;

    if (!o || o->kind != FLE_GDI_KIND_BITMAP || o->fmt != FLE_GDI_BMP_DIB) return 0;
    d = (gdi_dib *)o->ext;
    *bits = (uint64_t)(size_t)d->bits;
    out[0] = d->width;
    out[1] = d->height < 0 ? (uint32_t)(-(int64_t)d->height) : (uint32_t)d->height;
    out[2] = d->bpp;
    out[3] = d->comp;
    out[4] = d->stride;
    out[5] = d->ncolors;
    return 1;
}

uint32_t fle_gdi_dib_color(uint64_t h, uint32_t i)
{
    gdi_obj *o = live_obj(h);
    gdi_dib *d;

    if (!o || o->kind != FLE_GDI_KIND_BITMAP || o->fmt != FLE_GDI_BMP_DIB) return 0;
    d = (gdi_dib *)o->ext;
    return i < d->ncolors ? d->clr[i] : 0;
}

int fle_gdi_brush_info(uint64_t h, uint32_t out[2])
{
    gdi_obj *o = live_obj(h);

    if (!o || o->kind != FLE_GDI_KIND_BRUSH) return 0;
    out[0] = o->u[0];
    out[1] = o->u[1];
    return 1;
}

int fle_gdi_iconcur_info(uint64_t h, uint32_t out[6])
{
    gdi_obj *o = live_obj(h);

    if (!o || o->kind != FLE_GDI_KIND_ICONCUR) return 0;
    for (uint32_t i = 0; i < 6; i++) out[i] = o->u[i];
    return 1;
}

/* gdi32!CreatePalette: HPALETTE f(const LOGPALETTE *plpal rcx). Learn (CreatePalette): the return value is a handle to a logical
 * palette, NULL on failure; LOGPALETTE is {WORD palVersion; WORD palNumEntries; PALETTEENTRY palPalEntry[1]} with each entry
 * four bytes {peRed, peGreen, peBlue, peFlags}. The pages say nothing about a NULL pointer, a palVersion, or a count of zero, so
 * only the guest's form is accepted: palVersion 0x300 and 16 entries; anything else stops. The entries are copied (the guest's
 * buffer is its stack). Every call returns a new handle; the last error is left alone. */
void fle_imp_CreatePalette(void)
{
    static const char api[] = "CreatePalette";
    uint64_t p = R[1], slot;
    uint32_t ver, n, e[FLE_GDI_PALETTE_ENTRIES];
    gdi_obj *o;

    fle_gdi_check_thread(api);
    if (p == 0) fle_stop(api, "plpal is NULL: the page does not define this call");
    ver = (uint32_t)fle_ld(p, 2);
    n = (uint32_t)fle_ld(p + 2u, 2);
    if (ver != FLE_GDI_PALETTE_VERSION) fle_stop(api, "palVersion is not 0x300 (the only value the guest passes; the page gives none)");
    if (n != FLE_GDI_PALETTE_ENTRIES) fle_stop(api, "palNumEntries is not 16 (the only count the guest passes)");
    for (uint32_t i = 0; i < n; i++) e[i] = (uint32_t)fle_ld(p + 4u + 4u * i, 4);
    slot = objs_n < FLE_GDI_FIRST_CREATED_SLOT ? FLE_GDI_FIRST_CREATED_SLOT : objs_n;
    o = slot_get(api, slot);
    for (uint32_t i = 0; i < n; i++) o->pal[i] = e[i];
    issue(o, FLE_GDI_KIND_PALETTE);
    pal_live++;
    R[0] = slot_handle(slot);
}

/* gdi32!CreateFontIndirectW: HFONT f(const LOGFONTW *lplf rcx) (decision 0066). Learn (CreateFontIndirectW): "creates a logical font
 * that has the specified characteristics"; the return value is a handle to a logical font, NULL on failure; "When you no longer
 * need the font, call the DeleteObject function". The page gives no GetLastError for the failure and says nothing about a NULL
 * lplf or about field values it would reject, so NULL stops the run and every other content is accepted as is (the font mapper
 * runs when a font is selected, which this runtime does not do). The 92 bytes are copied (the guest's buffer is its stack). Every
 * call returns a new handle; the last error is left alone. */
void fle_imp_CreateFontIndirectW(void)
{
    static const char api[] = "CreateFontIndirectW";
    uint64_t p = R[1], slot;
    uint32_t w[FLE_GDI_LOGFONTW_SIZE / 4];
    gdi_obj *o;

    fle_gdi_check_thread(api);
    if (p == 0) fle_stop(api, "lplf is NULL: the page does not define this call");
    for (uint32_t i = 0; i < FLE_GDI_LOGFONTW_SIZE / 4; i++) w[i] = (uint32_t)fle_ld(p + 4u * i, 4);
    slot = objs_n < FLE_GDI_FIRST_CREATED_SLOT ? FLE_GDI_FIRST_CREATED_SLOT : objs_n;
    o = slot_get(api, slot);
    for (uint32_t i = 0; i < FLE_GDI_LOGFONTW_SIZE / 4; i++) {
        o->lf[4 * i] = (uint8_t)w[i];
        o->lf[4 * i + 1] = (uint8_t)(w[i] >> 8);
        o->lf[4 * i + 2] = (uint8_t)(w[i] >> 16);
        o->lf[4 * i + 3] = (uint8_t)(w[i] >> 24);
    }
    issue(o, FLE_GDI_KIND_FONT);
    font_live++;
    R[0] = slot_handle(slot);
}

/* gdi32!CreateBitmap: HBITMAP f(int nWidth ecx, int nHeight edx, UINT nPlanes r8d, UINT nBitCount r9d, const VOID *lpBits [rsp+0x28])
 * (decision 0106, PROVISIONAL). Learn (CreateBitmap): "creates a bitmap with the specified width, height, and color format (color planes and
 * bits-per-pixel)"; lpBits "If this parameter is NULL, then the contents of the new bitmap are undefined"; the expected buffer is
 * cj = (((nWidth * nPlanes * nBitCount + 15) >> 4) << 1) * nHeight; the return value is a handle to a bitmap, NULL on failure; the one error
 * the page names is ERROR_INVALID_BITMAP ("The calculated size of the bitmap is less than zero"); "If an application sets the nWidth or nHeight
 * parameters to zero, CreateBitmap returns the handle to a 1-by-1 pixel, monochrome bitmap"; "When you no longer need the bitmap, call the
 * DeleteObject function". Answered: the guest's form (a cursor mask: nPlanes 1, nBitCount 1, lpBits NULL) with a nWidth and nHeight that are not
 * negative and whose cj is below 2^31; a zero nWidth or nHeight gives the 1-by-1 monochrome bitmap the page names (the guest's first call passes 0
 * and 0: decision 0106). The table keeps width, height, planes and bit count and no pixels (undefined contents cannot be read back, and a function
 * that would read them, such as SelectObject with BitBlt or GetBitmapBits, is not answered). Every other form stops by name: a negative size,
 * another format, a non-NULL lpBits. Each call returns a new handle; the last error is left alone. Only the low DWORD of rcx, rdx, r8 and r9 is an argument (int, int, UINT, UINT). */
/* Decision 0132 (PROVISIONAL), wall 18: CreateBitmap's lpBits (after bitmap_bits below). */
static void ddb_init_bits(const char *api, uint64_t h, uint64_t bits);

void fle_imp_CreateBitmap(void)
{
    static const char api[] = "CreateBitmap";
    int32_t w = (int32_t)(uint32_t)R[1], h = (int32_t)(uint32_t)R[2];
    uint32_t planes = (uint32_t)R[8], bpp = (uint32_t)R[9];
    uint64_t bits = fle_arg(5), slot, cj;
    gdi_obj *o;
    int zero_dim = 0;

    fle_gdi_check_thread(api);
    if (w < 0 || h < 0) fle_stop(api, "nWidth or nHeight is negative: the page does not define it (its ERROR_INVALID_BITMAP is for a calculated size below zero, not implemented)");
    if (planes != 1 || bpp != 1) fle_stop(api, "nPlanes and nBitCount are not 1 and 1: only the monochrome mask of the guest's cursor is answered");
    cj = (((uint64_t)(uint32_t)w * planes * bpp + 15u) >> 4 << 1) * (uint64_t)(uint32_t)h;
    if (cj >= UINT64_C(0x80000000)) fle_stop(api, "the calculated size of the bitmap is 2 GiB or more: the page's ERROR_INVALID_BITMAP rule is not implemented");
    if (w == 0 || h == 0) {
        w = h = 1;   /* the page: a zero width or height gives "the handle to a 1-by-1 pixel, monochrome bitmap" */
        zero_dim = 1;
    }
    slot = objs_n < FLE_GDI_FIRST_CREATED_SLOT ? FLE_GDI_FIRST_CREATED_SLOT : objs_n;
    o = slot_get(api, slot);
    o->bm[0] = (uint32_t)w;
    o->bm[1] = (uint32_t)h;
    o->bm[2] = planes;
    o->bm[3] = bpp;
    issue(o, FLE_GDI_KIND_BITMAP);
    bmp_live++;
    R[0] = slot_handle(slot);
    if (bits != 0 && !zero_dim) ddb_init_bits(api, slot_handle(slot), bits);
}

/* GetObjectW on a bitmap (decision 0108, PROVISIONAL; decision 0106 stopped on every bitmap). Learn (GetObjectW): a bitmap gives a BITMAP; "a bitmap
 * created by calling CreateDIBSection" gives "DIBSECTION, if cbBuffer is set to sizeof(DIBSECTION), or BITMAP, if cbBuffer is set to sizeof(BITMAP)", and
 * "the bmBits member ... will contain a pointer to the bitmap's bit values"; "if hgdiobj is a handle to a bitmap created by any other means, GetObject returns
 * only the width, height, and color format information"; the return value is "the number of bytes stored into the buffer"; "The address of lpvObject must be
 * on a 4-byte boundary; otherwise, GetObject fails" (zero). Learn (BITMAP): bmType "must be zero", bmWidthBytes "must be divisible by 2". Answered: a
 * device-dependent bitmap with c >= sizeof(BITMAP) = 32 (32 bytes are stored, 32 is returned: bmWidthBytes is the CreateBitmap page's ((w * planes * bpp + 15) >> 4)
 * << 1, bmBits is 0 because the page gives no bit pointer for it: a choice), and a DIB section with c == 32 (bmWidthBytes is the stride of the BITMAPINFOHEADER page,
 * bmBits the pointer the guest got). Stops: pv NULL (the byte count differs for the two kinds and the page names it for neither), c below 32, a DIB section with
 * c >= sizeof(DIBSECTION) = 104 (biSizeImage, biClrUsed, biClrImportant, dsBitfields and dsOffset are not on the pages), a DIB section with another c.
 * Nothing is stored before a stop. A pv that is not 4-byte aligned returns 0.
 * Decision 0113 wall 5 (PROVISIONAL; PM decision 2 of 0108 kept this a stop "until reached", and the guest's Delphi CopyBitmap reached it with c = 104): a DIB
 * section with c == 104 stores the 104 bytes of a DIBSECTION and returns 104 (c above 104 and the other values still stop). dsBm is the BITMAP above. What the pages
 * give: dsBmih is "information about the color format of the DIB" (BITMAPINFOHEADER: biSize 40, biPlanes 1); dshSection is NULL because hSection was NULL (CreateDIBSection and
 * DIBSECTION pages); dsOffset "has no meaning" then; dsBitfields is "only valid when the BitCount member ... has a value greater than 8" and the three masks of a BI_BITFIELDS
 * DIB are the ones the guest passed (BITMAPINFOHEADER: "three DWORD color masks (red, green, and blue)"). What no page gives is what GetObject stores, so these are choices
 * without a source: dsBmih is the header the guest passed (biHeight with its sign, biCompression, biXPelsPerMeter, biYPelsPerMeter, biClrUsed, biClrImportant as given) with
 * biSizeImage = |biHeight| * stride (the BITMAPINFOHEADER page's formula; CreateDIBSection here accepts only 0 or that value); dsBitfields of a BI_RGB DIB of 16 bits is
 * RGB 555 (the BITMAPINFOHEADER page: "always RGB 555"; the masks 0x7C00, 0x03E0, 0x001F are the 5-5-5 layout, red first) and of 32 bits is 0x00FF0000, 0x0000FF00, 0x000000FF
 * (no page gives them: the 8-8-8 layout, a choice); 8 bits and less: zeros; a BI_RGB DIB of 24 bits stops (no page gives masks for it);
 * dsOffset, dshSection and the 8 padding bytes of the structure are 0. */
static void get_bitmap(const char *api, const gdi_obj *o, uint32_t c, uint64_t pv)
{
    uint32_t w = o->bm[0], h = o->bm[1], planes = o->bm[2], bpp = o->bm[3], wb;
    uint64_t bits = 0;
    char why[200];

    if (pv == 0) fle_stop(api, "handle is a bitmap and pv is NULL: the page gives the byte count for it but not for a DIB section (decision 0108)");
    if (c < FLE_GDI_BITMAP_SIZE) {
        snprintf(why, sizeof why, "handle is a bitmap: c is 0x%x, below sizeof(BITMAP) = 0x20: the page does not say what is stored for a smaller c", (unsigned)c);
        fle_stop(api, why);
    }
    if (o->fmt == FLE_GDI_BMP_DIB) {
        const gdi_dib *d = (const gdi_dib *)o->ext;

        if (c > FLE_GDI_DIBSECTION_SIZE)
            fle_stop(api, "handle is a DIB section and c is above sizeof(DIBSECTION) = 0x68: the page names sizeof(BITMAP) and sizeof(DIBSECTION) only");
        if (c == FLE_GDI_DIBSECTION_SIZE) {
            uint32_t mk[3] = { d->masks[0], d->masks[1], d->masks[2] };

            if (d->comp == FLE_GDI_BI_RGB && d->bpp == 24)
                fle_stop(api, "handle is a 24-bit BI_RGB DIB section and c asks for a DIBSECTION: no page gives dsBitfields for it (decision 0113)");
            if (d->comp == FLE_GDI_BI_RGB && d->bpp == 16) { mk[0] = 0x7C00u; mk[1] = 0x03E0u; mk[2] = 0x001Fu; }
            if (d->comp == FLE_GDI_BI_RGB && d->bpp == 32) { mk[0] = 0x00FF0000u; mk[1] = 0x0000FF00u; mk[2] = 0x000000FFu; }
            if (pv & 3u) {
                R[0] = 0;
                return;
            }
            fle_st(pv, 4, 0);
            fle_st(pv + 4u, 4, d->width);
            fle_st(pv + 8u, 4, o->bm[1]);
            fle_st(pv + 12u, 4, d->stride);
            fle_st(pv + 16u, 2, 1);
            fle_st(pv + 18u, 2, d->bpp);
            fle_st(pv + 20u, 4, 0);
            fle_st(pv + 24u, 8, (uint64_t)(size_t)d->bits);
            fle_st(pv + 32u, 4, FLE_GDI_BMIH_SIZE);
            fle_st(pv + 36u, 4, d->width);
            fle_st(pv + 40u, 4, (uint32_t)d->height);
            fle_st(pv + 44u, 2, 1);
            fle_st(pv + 46u, 2, d->bpp);
            fle_st(pv + 48u, 4, d->comp);
            fle_st(pv + 52u, 4, d->stride * o->bm[1]);
            fle_st(pv + 56u, 4, d->xppm);
            fle_st(pv + 60u, 4, d->yppm);
            fle_st(pv + 64u, 4, d->clrused);
            fle_st(pv + 68u, 4, d->clrimp);
            fle_st(pv + 72u, 4, mk[0]);
            fle_st(pv + 76u, 4, mk[1]);
            fle_st(pv + 80u, 4, mk[2]);
            fle_st(pv + 84u, 4, 0);
            fle_st(pv + 88u, 8, 0);
            fle_st(pv + 96u, 4, 0);
            fle_st(pv + 100u, 4, 0);
            R[0] = FLE_GDI_DIBSECTION_SIZE;
            return;
        }
        if (c != FLE_GDI_BITMAP_SIZE) {
            snprintf(why, sizeof why, "handle is a DIB section and c is 0x%x, neither sizeof(BITMAP) = 0x20 nor sizeof(DIBSECTION) = 0x68: the page names those two", (unsigned)c);
            fle_stop(api, why);
        }
        wb = d->stride;
        bits = (uint64_t)(size_t)d->bits;
    } else {
        wb = (uint32_t)((((uint64_t)w * planes * bpp + 15u) >> 4) << 1);
    }
    if (pv & 3u) {
        R[0] = 0;
        return;
    }
    fle_st(pv, 4, 0);
    fle_st(pv + 4u, 4, w);
    fle_st(pv + 8u, 4, h);
    fle_st(pv + 12u, 4, wb);
    fle_st(pv + 16u, 2, planes);
    fle_st(pv + 18u, 2, bpp);
    fle_st(pv + 20u, 4, 0);
    fle_st(pv + 24u, 8, bits);
    R[0] = FLE_GDI_BITMAP_SIZE;
}

/* gdi32!GetObjectW: int f(HANDLE h rcx, int c edx, LPVOID pv r8) (decision 0068). Learn (GetObjectW): "If hgdiobj is a handle to a
 * font, the LOGFONT that is returned is the LOGFONT used to create the font"; with a valid pv the return value is "the number of
 * bytes stored into the buffer", with pv NULL "the number of bytes required", zero on failure. Answered: a live created font
 * (CreateFontIndirectW), c == 0x5c (sizeof(LOGFONTW), the guest's call at 0x5c0c0f) and pv not NULL: the 92 bytes of the copy are
 * stored at pv and 92 is returned. Only the low DWORD of rdx is c (an int). The last error and every register but rax are left
 * alone. Everything else stops, and nothing is stored before a stop: a handle this table never issued or deleted, the stock objects
 * (the stock font has no decided LOGFONT), a palette, an icon, a c that is not 0x5c (the page does not say what is stored for a
 * smaller or a larger c), a NULL pv (the page gives the byte count for it, the guest never asks). The page does not say which of
 * these fail, so none returns zero. */
void fle_imp_GetObjectW(void)
{
    static const char api[] = "GetObjectW";
    uint64_t h = R[1], pv = R[8];
    uint32_t c = (uint32_t)R[2];
    int64_t s;
    gdi_obj *o;
    char why[160];

    fle_gdi_check_thread(api);
    s = slot_of(h);
    if (s < 0) fle_stop(api, "handle was not issued by this runtime's GDI table (NULL, a DC, another object kind or a number never issued): the page says only that failure returns zero");
    o = &objs[s];
    if (o->state == ST_DELETED) fle_stop(api, "handle was deleted by DeleteObject: the page does not say what GetObject returns for it");
    switch (o->kind) {
    case FLE_GDI_KIND_FONT: break;
    case FLE_GDI_KIND_STOCK_FONT: fle_stop(api, "handle is the stock SYSTEM_FONT: its LOGFONT is not decided (decision 0066 gives a value only to the fonts CreateFontIndirectW made)");
    case FLE_GDI_KIND_PALETTE: fle_stop(api, "handle is a palette: the page says GetObject stores a 2-byte entry count, the guest's calls (c = 4) are not decided");
    case FLE_GDI_KIND_STOCK_BRUSH: fle_stop(api, "handle is a stock brush: its LOGBRUSH is not decided");
    case FLE_GDI_KIND_STOCK_PEN: fle_stop(api, "handle is a stock pen: its LOGPEN is not decided");
    case FLE_GDI_KIND_ICON: fle_stop(api, "handle is an icon: it is not an object kind the GetObject page describes");
    case FLE_GDI_KIND_BITMAP: get_bitmap(api, o, c, pv); return;
    default: fle_stop(api, "handle is of a kind this table does not describe");
    }
    if (c != FLE_GDI_LOGFONTW_SIZE) {
        snprintf(why, sizeof why, "c is 0x%x, not sizeof(LOGFONTW) = 0x5c: the page does not say what is stored for another c", (unsigned)c);
        fle_stop(api, why);
    }
    if (pv == 0) fle_stop(api, "pv is NULL: the page returns the byte count for it, the guest never passes NULL for a font");
    for (uint32_t i = 0; i < FLE_GDI_LOGFONTW_SIZE / 4; i++)
        fle_st(pv + 4u * i, 4, (uint64_t)o->lf[4 * i] | (uint64_t)o->lf[4 * i + 1] << 8 | (uint64_t)o->lf[4 * i + 2] << 16 | (uint64_t)o->lf[4 * i + 3] << 24);
    R[0] = FLE_GDI_LOGFONTW_SIZE;
}

/* gdi32!DeleteObject: BOOL f(HGDIOBJ ho rcx). Learn (DeleteObject): the object is deleted and "the specified handle is no longer
 * valid"; the return value is nonzero on success and zero "if the specified handle is not valid or is currently selected into a
 * DC". A live palette, a live created font (decision 0066; the page lists fonts), a live bitmap (decisions 0106 and 0108; the page lists
 * bitmaps), a live brush made by CreateBrushIndirect or CreateSolidBrush (decisions 0108, 0111; the page lists brushes) and a live pen made by CreatePenIndirect (decision 0111; the page lists pens) is deleted and the result is 1, unless it is
 * selected into a memory DC (decision 0108: SelectObject, SelectPalette): then it stays and the result is 0. The memory of a DIB section goes with
 * it (the CreateDIBSection page: "The system closes the handle to that memory when you later delete the DIB by calling DeleteObject"). A memory DC is
 * deleted like DeleteDC does (the CreateCompatibleDC page: "you can also call DeleteObject with the HDC to delete the DC"). NULL and an object already
 * deleted are not valid handles: 0. The GetStockObject page says deleting a stock object is "not necessary (but it is not harmful)" and gives no
 * return value, and the pages give no rule for deleting an icon (DestroyIcon is the call for that) or the placeholder bitmap of a memory DC, so those
 * stop. A non-NULL value this table never issued (another kind of object, a number in the region that was never issued) is not described and stops. */
void fle_imp_DeleteObject(void)
{
    static const char api[] = "DeleteObject";
    uint64_t h = R[1];
    int64_t i;
    gdi_obj *o;

    fle_gdi_check_thread(api);
    if (h == 0) {
        R[0] = 0;
        return;
    }
    i = slot_of(h);
    if (i < 0) fle_stop(api, "handle was not issued by this runtime's GDI table (other object kinds are not defined here)");
    o = &objs[i];
    if (o->state == ST_DELETED) {
        R[0] = 0;
        return;
    }
    if (o->kind == FLE_GDI_KIND_DC) {
        dc_destroy(i);
        R[0] = 1;
        return;
    }
    if (o->kind != FLE_GDI_KIND_PALETTE && o->kind != FLE_GDI_KIND_FONT && o->kind != FLE_GDI_KIND_BITMAP && o->kind != FLE_GDI_KIND_BRUSH && o->kind != FLE_GDI_KIND_PEN && o->kind != FLE_GDI_KIND_REGION) {
        if (o->kind == FLE_GDI_KIND_ICON) fle_stop(api, "handle is an icon: the pages give no rule for DeleteObject on an icon");
        if (o->kind == FLE_GDI_KIND_ICONCUR) fle_stop(api, "handle is an icon or a cursor made by CreateIconIndirect: the pages give no rule for DeleteObject on one (DestroyIcon is the call)");
        if (o->kind == FLE_GDI_KIND_DEFAULT) fle_stop(api, "handle is a default object of the memory DCs, like a stock object: the page gives no return value for deleting one");
        fle_stop(api, "handle is a stock object: the page gives no return value for deleting one");
    }
    if (o->kind == FLE_GDI_KIND_BITMAP && (o->flags & GF_PLACEHOLDER)) fle_stop(api, "handle is the placeholder bitmap of a memory DC: the pages give no rule for deleting it");
    if (o->sel) {
        R[0] = 0;
        return;
    }
    if (o->kind == FLE_GDI_KIND_PALETTE) pal_live--;
    else if (o->kind == FLE_GDI_KIND_BITMAP) {
        bmp_live--;
        if (o->fmt == FLE_GDI_BMP_DIB) dib_release(o);
        else ddb_release(o);
    } else if (o->kind == FLE_GDI_KIND_BRUSH) brush_live--;
    else if (o->kind == FLE_GDI_KIND_PEN) pen_live--;
    else font_live--;
    o->state = ST_DELETED;
    o->kind = FLE_GDI_KIND_NONE;
    objs_live--;
    slot_release((uint64_t)i);
    R[0] = 1;
}

/* gdi32!GetStockObject: HGDIOBJ f(int i ecx). Learn (GetStockObject): the return value is a handle to the requested logical
 * object, NULL on failure. Answered: NULL_BRUSH 5, BLACK_PEN 7, SYSTEM_FONT 13 (the guest's call sites; the numbers are
 * unofficial, fle_gdi.h) and, decision 0126, the other stock brushes (WHITE 0, LTGRAY 1, GRAY 2, DKGRAY 3, BLACK 4) and pens (WHITE 6, NULL 8): the SDK's numbers, UNOFFICIAL. A stock object has one handle for every call (choice). Every other index stops: the pages say nothing
 * about an index outside the list (the NULL return is for "fails", and no index is documented to fail). Only the low DWORD of
 * rcx is the index (an int). */
void fle_imp_GetStockObject(void)
{
    static const char api[] = "GetStockObject";
    uint32_t idx = (uint32_t)R[1];
    uint8_t kind;
    gdi_obj *o;
    char why[160];

    fle_gdi_check_thread(api);
    switch (idx) {
    case 0: case 1: case 2: case 3: case 4:
    case FLE_GDI_STOCK_NULL_BRUSH: kind = FLE_GDI_KIND_STOCK_BRUSH; break;
    case 6: case FLE_GDI_STOCK_BLACK_PEN: case 8: kind = FLE_GDI_KIND_STOCK_PEN; break;
    case FLE_GDI_STOCK_SYSTEM_FONT: kind = FLE_GDI_KIND_STOCK_FONT; break;
    default:
        snprintf(why, sizeof why, "index %u is not answered (answered: the brushes 0 to 5, the pens 6 to 8 and the system font 13)", (unsigned)idx);
        fle_stop(api, why);
    }
    o = slot_get(api, idx);
    if (o->state == ST_NEVER) {
        issue(o, kind);
        o->fmt = (uint8_t)idx;      /* decision 0126: the index says which stock brush or pen it is (the colour of a drawing call) */
    }
    R[0] = slot_handle(idx);
}

/* user32!LoadIconW: HICON f(HINSTANCE hInstance rcx, LPCWSTR lpIconName rdx). Learn (LoadIconW): with hInstance NULL, lpIconName
 * is the IDI_ identifier of a predefined system icon; the return value is a handle to the icon, NULL on failure; "LoadIcon loads
 * the icon resource only if it has not been loaded; otherwise, it retrieves a handle to the existing resource" (one handle for
 * every call). Learn (About Icons): IDI_APPLICATION is MAKEINTRESOURCE(32512). Only that one is answered. A non-NULL hInstance
 * needs the resources of a module, another identifier needs another icon: both stop. The whole 64 bits of rdx are compared (a
 * value that is not 32512 as a pointer is a name, not an identifier). The last error is left alone.
 * Decision 0069 (provisional): hInstance NULL and lpIconName a string (not IS_INTRESOURCE: a bit above the low 16 is set) that is
 * exactly L"MAINICON" (read through fle_ld, at most FLE_GDI_ICON_NAME_MAX_UNITS units) returns NULL; Learn gives NULL for a
 * failure and does not say what this form returns. Any other string, or a string without a NUL in the first 16 units, stops. */
/* Decision 0118 wall 8 (PROVISIONAL; decision 0069 stopped on it): LoadIconW(hInstance, name) with hInstance the lifted DLL or a real module loads an icon from that image's
 * RT_GROUP_ICON of that name (integer id or string; fle_icon_group_pick in the resource file, a weak reference like the ones above: a build without it keeps the old stop).
 * Learn (LoadIconW): "If hInstance is non-NULL, lpIconName specifies the icon resource either by name or ordinal"; "LoadIcon loads the icon resource only if it has not been
 * loaded; otherwise, it retrieves a handle to the existing resource" (the same group of the same image gives the same handle); "searches the icon resource for the icon most
 * appropriate for the current display"; "can only load an icon whose size conforms to the SM_CXICON and SM_CYICON system metric values" (32 x 32, fle_sysmetrics.h); the display
 * depth is FLE_GDC_BITSPIXEL (fle_gdc.h). The handle is a shared icon (kind FLE_GDI_KIND_ICON, flag GF_MODICON): it keeps the group's identity, the size and depth of the chosen
 * entry and the hot spot GetIconInfo reports; no pixels. A name the module has no group for stops (the page names no error code for a failed load, and none is made up).
 * The handle values are the table's (decision 0054: one region, 4 * (slot + 1)). */
extern int fle_icon_group_pick(const char *api, uint64_t base, uint64_t name, uint32_t cx, uint32_t cy, uint32_t depth, uint64_t out[5]) __attribute__((weak));

static void load_icon_of_module(const char *api)
{
    uint64_t out[5], slot, i;
    gdi_obj *o;
    uint32_t w, h, d;

    if (fle_icon_group_pick == NULL) fle_stop(api, "hInstance is not NULL: icon resources of a module are not implemented (only IDI_APPLICATION is)");
    if (!fle_icon_group_pick(api, R[1], R[2], FLE_SM_CXICON, FLE_SM_CYICON, FLE_GDC_BITSPIXEL, out))
        fle_stop(api, "the module has no RT_GROUP_ICON of that name: the page names no error code for a failed load (decision 0118)");
    for (i = 0; i < objs_n; i++)
        if (objs[i].state == ST_LIVE && objs[i].kind == FLE_GDI_KIND_ICON && (objs[i].flags & GF_MODICON) && objs[i].key == out[0]) {
            R[0] = slot_handle(i);
            return;
        }
    w = (uint32_t)out[1];
    h = (uint32_t)out[2];
    d = (uint32_t)out[3];
    slot = new_slot(api);
    o = &objs[slot];
    o->flags = GF_MODICON;
    o->key = out[0];
    o->u[0] = 1;                            /* fIcon */
    o->u[1] = w / 2u;                       /* the page: the hot spot of an icon "is always in the center of the icon" (a reading: pixel (w / 2, h / 2)) */
    o->u[2] = h / 2u;
    o->u[3] = w;
    o->u[4] = d > 1u ? h : 2u * h;          /* ICONINFO: a colour icon's mask is the size of the icon; a monochrome icon's mask is twice the height and hbmColor is NULL */
    o->u[5] = d > 1u;
    issue(o, FLE_GDI_KIND_ICON);
    R[0] = slot_handle(slot);
}

void fle_imp_LoadIconW(void)
{
    static const char api[] = "LoadIconW";
    static const char mainicon[] = "MAINICON";
    gdi_obj *o;

    fle_gdi_check_thread(api);
    if (R[1] != 0) {
        load_icon_of_module(api);
        return;
    }
    if ((R[2] >> 16) != 0) {
        uint64_t p = R[2];
        unsigned n = 0;
        int same = 1;

        for (;;) {
            uint16_t c;

            if (n >= FLE_GDI_ICON_NAME_MAX_UNITS) fle_stop(api, "lpIconName is a string longer than 16 UTF-16 units: not answered");
            c = (uint16_t)fle_ld(p + 2u * n, 2);
            if (c == 0) break;
            if (n < sizeof mainicon - 1 && c != (uint16_t)(uint8_t)mainicon[n]) same = 0;
            n++;
        }
        if (n != sizeof mainicon - 1) same = 0;
        if (!same) fle_stop(api, "lpIconName is a string other than L\"MAINICON\": not answered");
        R[0] = 0;
        return;
    }
    if (R[2] != FLE_GDI_IDI_APPLICATION) fle_stop(api, "lpIconName is not IDI_APPLICATION (MAKEINTRESOURCE(32512)): not answered");
    o = slot_get(api, FLE_GDI_ICON_SLOT);
    if (o->state == ST_NEVER) issue(o, FLE_GDI_KIND_ICON);
    R[0] = slot_handle(FLE_GDI_ICON_SLOT);
}

/* gdi32!EnumFontFamiliesExW (decision 0098, PROVISIONAL). Learn (EnumFontFamiliesExW): "enumerates all uniquely-named fonts in the system that match
 * the font characteristics specified by the LOGFONT structure"; lfCharSet "If set to DEFAULT_CHARSET, the function enumerates all uniquely-named fonts
 * in all character sets"; lfFaceName "If set to an empty string, the function enumerates one font in each available typeface name. If set to a valid
 * typeface name, the function enumerates all fonts with the specified name"; lfPitchAndFamily "Must be set to zero"; dwFlags "is not used and must
 * be zero"; "The return value is the last value returned by the callback function." Learn (EnumFontFamExProc): "The return value must be a nonzero
 * value to continue enumeration; to stop enumeration, it must return zero." Learn (ENUMLOGFONTEXW): { LOGFONTW elfLogFont; WCHAR elfFullName[64];
 * WCHAR elfStyle[32]; WCHAR elfScript[32] }.
 * The only font of this environment is the system font of decision 0066 (the face "Segoe UI", weight 400). The call for all families, or for that
 * family by name (ASCII case folding, as everywhere in this runtime), calls the callback once with: elfLogFont lfWeight 400 and lfFaceName the face,
 * elfFullName the face, FontType TRUETYPE_FONTTYPE; elfStyle, elfScript, lfCharSet and the other members are undecided and written as 0 (the guest's
 * callback reads lfFaceName only). The NEWTEXTMETRICEXW is all 0. One call, so the enumeration cannot be stopped early.
 * The device context must be one that GetDC(NULL) handed out and that is not released (decision 0053). The last error is left alone.
 * Stops: hdc not a live virtual DC, lpLogfont or lpProc NULL, dwFlags not 0, lfPitchAndFamily not 0, lfCharSet other than DEFAULT_CHARSET, a face
 * name other than the empty string and the face. */
extern uint64_t fle_call_guest(uint64_t va, uint64_t rcx, uint64_t rdx, uint64_t r8, uint64_t r9) __attribute__((weak));

void fle_imp_EnumFontFamiliesExW(void)
{
    static const char api[] = "EnumFontFamiliesExW";
    static const char face[] = FLE_SPI_FONT_FACE;
    uint64_t hdc = R[1], lf = R[2], proc = R[8], param = R[9];
    uint64_t sp = R[4];
    uint64_t elfe, ntme, cb;
    uint32_t flags = (uint32_t)fle_arg(5);
    unsigned i, n;
    int match = 1;

    fle_gdi_check_thread(api);
    if (!fle_gdc_is_live(hdc)) fle_stop(api, "hdc is not a live virtual screen DC (GetDC(NULL), decision 0053)");
    if (lf == 0) fle_stop(api, "lpLogfont is NULL: the page does not define it (decision 0098)");
    if (proc == 0) fle_stop(api, "lpProc is NULL: the page does not define it (decision 0098)");
    if (flags != 0) fle_stop(api, "dwFlags is not zero: the page says it must be zero (decision 0098)");
    if (fle_ld(lf + FLE_GDI_LF_PITCHANDFAMILY, 1) != 0) fle_stop(api, "lfPitchAndFamily is not zero: the page says it must be zero (decision 0098)");
    if (fle_ld(lf + FLE_GDI_LF_CHARSET, 1) != FLE_GDI_DEFAULT_CHARSET) fle_stop(api, "lfCharSet is not DEFAULT_CHARSET: only the call for all character sets is answered (decision 0098)");
    n = (unsigned)sizeof face - 1u;
    if ((uint16_t)fle_ld(lf + FLE_GDI_LF_FACENAME, 2) != 0) {       /* a name: it must be the face, then the terminating zero */
        for (i = 0; i <= n && match; i++) {
            uint16_t c = (uint16_t)fle_ld(lf + FLE_GDI_LF_FACENAME + 2u * i, 2);

            if (i < n) match = fle_w_fold(c) == fle_w_fold((uint16_t)(uint8_t)face[i]);
            else match = c == 0;
        }
    }
    if (!match) fle_stop(api, "lfFaceName is neither empty nor the face of the system font: no other font is defined (decision 0098)");
    if (fle_call_guest == NULL) fle_stop(api, "the guest callback mechanism is not linked");

    elfe = (sp - 0x200) & ~UINT64_C(15);
    ntme = elfe + 0x160;
    for (i = 0; i < FLE_GDI_ENUMLOGFONTEXW_SIZE; i += 4) fle_st(elfe + i, 4, 0);
    for (i = 0; i < FLE_GDI_NEWTEXTMETRICEXW_SIZE; i += 4) fle_st(ntme + i, 4, 0);
    fle_st(elfe + FLE_GDI_LF_WEIGHT, 4, FLE_SPI_FONT_WEIGHT);
    for (i = 0; i < n; i++) {
        fle_st(elfe + FLE_GDI_LF_FACENAME + 2u * i, 2, (uint8_t)face[i]);
        fle_st(elfe + FLE_GDI_ELF_FULLNAME + 2u * i, 2, (uint8_t)face[i]);
    }
    R[4] = elfe;
    cb = fle_call_guest(proc, elfe, ntme, FLE_GDI_TRUETYPE_FONTTYPE, param);
    R[4] = sp;
    R[0] = (uint32_t)cb;
}

/* ---- decision 0108 (PROVISIONAL): the memory DC, DIB section, brush and selection answers (the model is described above, before CreatePalette) ---- */

/* gdi32!CreateCompatibleDC: HDC f(HDC hdc rcx). Learn (CreateCompatibleDC): "creates a memory device context (DC) compatible with the specified device"; "If this handle is
 * NULL, the function creates a memory DC compatible with the application's current screen"; "When the memory DC is created, its display surface is exactly one monochrome
 * pixel wide and one monochrome pixel high"; "all attributes are set to normal default values"; the return value is the handle to a memory DC, NULL on failure. Answered:
 * hdc NULL or a DC that GetDC(NULL) handed out (this environment has one virtual display, decision 0053): a new memory DC with its own placeholder bitmap selected, the four
 * default objects (fle_gdi.h; the pages do not say which objects a new DC holds), text colour black and background colour white (choices: no page gives the initial colours).
 * Every call returns a new handle, never reused. A memory DC as hdc, a deleted DC and every other value stop: the page defines the failure value and no case that fails. */
void fle_imp_CreateCompatibleDC(void)
{
    static const char api[] = "CreateCompatibleDC";
    uint64_t h = R[1], s_dc, s_bm;
    gdi_dc *dc;
    gdi_obj *o, *p;

    fle_gdi_check_thread(api);
    if (h != 0) {
        switch (dc_class(h, NULL)) {
        case DCK_SCREEN: break;
        case DCK_MEM: fle_stop(api, "hdc is a memory DC: a DC compatible with a memory DC (the format of the bitmap selected into it) is not modelled, only NULL and a DC of GetDC(NULL) are");
        case DCK_DELETED: fle_stop(api, "hdc was deleted by DeleteDC: the page does not say what the call returns for it");
        default: fle_stop(api, "hdc is neither NULL nor a DC of this runtime: the page does not define it");
        }
    }
    default_objects(api);
    s_dc = new_slot(api);
    s_bm = new_slot(api);
    dc = (gdi_dc *)calloc(1, sizeof *dc);
    if (!dc) fle_stop(api, "out of memory");
    o = &objs[s_dc];
    p = &objs[s_bm];
    p->bm[0] = p->bm[1] = p->bm[2] = p->bm[3] = 1;
    p->fmt = FLE_GDI_BMP_MONO;
    p->flags = GF_PLACEHOLDER;
    p->sel = 1;
    issue(p, FLE_GDI_KIND_BITMAP);
    bmp_live++;
    dc->bmp = dc->placeholder = slot_handle(s_bm);
    dc->brush = dc_default[FLE_GDI_DEFAULT_BRUSH];
    dc->pen = dc_default[FLE_GDI_DEFAULT_PEN];
    dc->font = dc_default[FLE_GDI_DEFAULT_FONT];
    dc->pal = dc_default[FLE_GDI_DEFAULT_PALETTE];
    sel_add(dc->brush);
    sel_add(dc->pen);
    sel_add(dc->font);
    sel_add(dc->pal);
    dc->text = FLE_GDI_DC_TEXT_COLOR;
    dc->bk = FLE_GDI_DC_BK_COLOR;
    dc->bk_mode = FLE_GDI_OPAQUE;           /* decision 0111: the Display Device Context Defaults page; position and origins are (0, 0) from calloc */
    dc->rop2 = FLE_GDI_R2_COPYPEN;
    dc->map_mode = FLE_GDI_MM_TEXT;
    dc->stretch = FLE_GDI_BLACKONWHITE;
    o->ext = dc;
    issue(o, FLE_GDI_KIND_DC);
    dc_live++;
    R[0] = slot_handle(s_dc);
}

/* gdi32!DeleteDC: BOOL f(HDC hdc rcx). Learn (DeleteDC): "deletes the specified device context (DC)"; nonzero on success, zero on failure; "An application must not delete a
 * DC whose handle was obtained by calling the GetDC function. Instead, it must call the ReleaseDC function". Answered: a live memory DC is deleted (what is selected into it
 * is not deleted, only unselected; the placeholder goes with the DC) and the result is 1; NULL and a memory DC that is already deleted are not valid handles: 0. A DC of
 * GetDC(NULL) stops (the page says not to, and not what happens); every other value stops. */
void fle_imp_DeleteDC(void)
{
    static const char api[] = "DeleteDC";
    uint64_t h = R[1];

    fle_gdi_check_thread(api);
    if (h == 0) {
        R[0] = 0;
        return;
    }
    switch (dc_class(h, NULL)) {
    case DCK_MEM: break;
    case DCK_DELETED: R[0] = 0; return;
    case DCK_SCREEN: fle_stop(api, "hdc is a DC of GetDC(NULL): the page says an application must not delete one (ReleaseDC is the call) and not what the call returns if it does");
    default: fle_stop(api, "hdc is not a DC of this runtime (a number never issued or another kind of handle): the page does not define it");
    }
    dc_destroy(slot_of(h));
    R[0] = 1;
}

/* gdi32!SelectObject: HGDIOBJ f(HDC hdc rcx, HGDIOBJ h rdx). Learn (SelectObject): "selects an object into the specified device context (DC). The new object replaces the
 * previous object of the same type"; the objects are a bitmap, a brush, a font, a pen or a region; "Bitmaps can only be selected into memory DC's. A single bitmap cannot be
 * selected into more than one DC at the same time"; "If the selected object is not a region and the function succeeds, the return value is a handle to the object being
 * replaced"; "If an error occurs and the selected object is not a region, the return value is NULL". Answered: a bitmap (also a DIB section), a brush (stock, default or made by
 * CreateBrushIndirect), a pen (stock or default) or a font (stock, default or made by CreateFontIndirectW) into a memory DC: the object is selected, the previous object of that
 * type is returned (the first call of a new DC returns the placeholder or the default object). A bitmap that is selected into another DC: NULL. A bitmap into a DC of
 * GetDC(NULL): NULL (the page lists memory DCs). Stops: any other object into a DC of GetDC(NULL) (its state is not modelled), a palette (SelectPalette is the call; the page's
 * list has none), a DC, an icon, a handle that was never issued or is deleted (the page names NULL for "an error" and not which handles are errors). The last error is left alone. */
void fle_imp_SelectObject(void)
{
    static const char api[] = "SelectObject";
    uint64_t hdc = R[1], h = R[2], prev, *at;
    gdi_dc *dc = NULL;
    gdi_obj *o;
    int64_t i;

    fle_gdi_check_thread(api);
    i = slot_of(h);
    if (i < 0) fle_stop(api, "h was not issued by this runtime's GDI table (NULL, a DC, a cursor or a number never issued): the page names NULL for an error and not which handles are errors");
    o = &objs[i];
    if (o->state == ST_DELETED) fle_stop(api, "h was deleted by DeleteObject: the page does not say what the call returns for it");
    switch (dc_class(hdc, &dc)) {
    case DCK_MEM: break;
    case DCK_SCREEN:
        if (o->kind == FLE_GDI_KIND_BITMAP) {
            R[0] = 0;
            return;
        }
        dc = screen_state(api, hdc);      /* decision 0126: a window or screen DC holds a brush, a pen and a font: the objects are selected into the state of this GetDC handle */
        break;
    case DCK_DELETED: fle_stop(api, "hdc was deleted by DeleteDC: the page does not say what the call returns for it");
    default: fle_stop(api, "hdc is not a DC of this runtime (NULL, a number never issued or another kind of handle): the page does not define it");
    }
    switch (o->kind) {
    case FLE_GDI_KIND_BITMAP:
        if (o->sel && dc->bmp != h) {
            R[0] = 0;
            return;
        }
        at = &dc->bmp;
        break;
    case FLE_GDI_KIND_STOCK_BRUSH:
    case FLE_GDI_KIND_BRUSH: at = &dc->brush; break;
    case FLE_GDI_KIND_STOCK_PEN:
    case FLE_GDI_KIND_PEN: at = &dc->pen; break;
    case FLE_GDI_KIND_FONT:
    case FLE_GDI_KIND_STOCK_FONT: at = &dc->font; break;
    case FLE_GDI_KIND_DEFAULT:
        if (o->fmt == FLE_GDI_DEFAULT_BRUSH) at = &dc->brush;
        else if (o->fmt == FLE_GDI_DEFAULT_PEN) at = &dc->pen;
        else if (o->fmt == FLE_GDI_DEFAULT_FONT) at = &dc->font;
        else fle_stop(api, "h is a palette: the page's list of objects has none, SelectPalette is the call");
        break;
    case FLE_GDI_KIND_PALETTE: fle_stop(api, "h is a palette: the page's list of objects has none, SelectPalette is the call");
    case FLE_GDI_KIND_DC: fle_stop(api, "h is a DC: the page's list of objects has none");
    case FLE_GDI_KIND_ICON:
    case FLE_GDI_KIND_ICONCUR: fle_stop(api, "h is an icon or a cursor: the page's list of objects has none");
    default: fle_stop(api, "h is of a kind this table does not describe");
    }
    prev = *at;
    if (prev != h) {
        sel_drop(prev);
        sel_add(h);
        *at = h;
    }
    R[0] = prev;
}

/* gdi32!SelectPalette: HPALETTE f(HDC hdc rcx, HPALETTE hPal rdx, BOOL bForceBkgd r8). Learn (SelectPalette): "selects the specified logical palette into a device context";
 * "If the function succeeds, the return value is a handle to the device context's previous logical palette. If the function fails, the return value is NULL"; "If the hdc
 * parameter is a memory device context, this parameter [bForceBkgd] is ignored"; a palette can be selected into more than one DC "only if device contexts are compatible"
 * (all the DCs of this environment are). Answered: a live palette (CreatePalette, CreateHalftonePalette, or the default palette of a memory DC) into a memory DC: the previous
 * palette is returned (the default palette on the first call). Stops: a DC of GetDC(NULL) (its palette is not modelled), a hPal that is not a live palette of this table.
 * Decision 0113 wall 7 (PROVISIONAL; decision 0108 stopped on it): hPal NULL on a memory DC returns NULL, the page's failure value, and changes nothing. The page does not say
 * what a NULL hPal does ("a handle to the logical palette to be selected"; it names NULL only as the failure value, and a successful call returns the previous palette, which
 * is never NULL here): a choice without a source. The guest (ILTools' Delphi TBitmap draw, after StretchDIBits) restores a saved palette that is 0 and ignores the result.
 * The last error is left alone (the page names none). */
void fle_imp_SelectPalette(void)
{
    static const char api[] = "SelectPalette";
    uint64_t h = R[2], prev;
    gdi_dc *dc;
    gdi_obj *o;

    fle_gdi_check_thread(api);
    dc = drawn_dc(api, R[1], "hdc is a DC of GetDC(NULL): its logical palette is not modelled (only memory DCs are)");
    if (h == 0) {
        R[0] = 0;
        return;
    }
    o = live_obj(h);
    if (!o || (o->kind != FLE_GDI_KIND_PALETTE && !(o->kind == FLE_GDI_KIND_DEFAULT && o->fmt == FLE_GDI_DEFAULT_PALETTE)))
        fle_stop(api, "hPal is not a live palette of this runtime (NULL, a number never issued, deleted, or another kind of object): the page names NULL for failure and not which handles fail");
    prev = dc->pal;
    if (prev != h) {
        sel_drop(prev);
        sel_add(h);
        dc->pal = h;
    }
    R[0] = prev;
}

/* gdi32!CreateCompatibleBitmap: HBITMAP f(HDC hdc rcx, int cx edx, int cy r8d). Learn (CreateCompatibleBitmap): "The color format of the bitmap ... matches the color format of
 * the device identified by the hdc parameter"; for a memory DC "the format of the bitmap returned ... differs": "When a memory device context is created, it initially has a 1-by-1
 * monochrome bitmap selected into it. If this memory device context is used in CreateCompatibleBitmap, the bitmap that is created is a monochrome bitmap"; "If an application sets
 * the nWidth or nHeight parameters to zero, CreateCompatibleBitmap returns the handle to a 1-by-1 pixel, monochrome bitmap"; "If a DIB section ... is selected into the device
 * context identified by the hdc parameter, CreateCompatibleBitmap creates a DIB section"; the return value is a handle to the compatible bitmap, NULL on failure. Answered: hdc a DC of
 * GetDC(NULL): a colour bitmap of the virtual display (fle_gdc.h: 32 bits, 1 plane); hdc a memory DC with a monochrome or a colour bitmap selected: a bitmap of the same format; a zero
 * cx or cy: the 1-by-1 monochrome bitmap. The table keeps size and format and no pixels (decision 0106). Stops: a DIB section selected into hdc, a negative size, a size of 2 GiB
 * or more (no page rule), hdc NULL or not a live DC. The last error is left alone. */
void fle_imp_CreateCompatibleBitmap(void)
{
    static const char api[] = "CreateCompatibleBitmap";
    int32_t cx = (int32_t)(uint32_t)R[2], cy = (int32_t)(uint32_t)R[8];
    uint64_t slot, bytes;
    uint32_t bpp;
    uint8_t fmt;
    gdi_dc *dc = NULL;
    gdi_obj *o;

    fle_gdi_check_thread(api);
    switch (dc_class(R[1], &dc)) {
    case DCK_SCREEN: fmt = FLE_GDI_BMP_COLOR; break;
    case DCK_MEM: {
        gdi_obj *b = live_obj(dc->bmp);

        if (!b) fle_stop(api, "the bitmap selected into hdc is gone");
        if (b->fmt == FLE_GDI_BMP_DIB) fle_stop(api, "a DIB section is selected into hdc: the page says the result is then a DIB section, which CreateCompatibleBitmap does not make here (decision 0108)");
        fmt = b->fmt;
        break;
    }
    case DCK_DELETED: fle_stop(api, "hdc was deleted by DeleteDC: the page does not say what the call returns for it");
    default: fle_stop(api, "hdc is not a DC of this runtime (NULL, a number never issued or another kind of handle): the page does not define it");
    }
    if (cx < 0 || cy < 0) fle_stop(api, "cx or cy is negative: the page does not define it");
    if (cx == 0 || cy == 0) {
        cx = cy = 1;
        fmt = FLE_GDI_BMP_MONO;
    }
    bpp = fmt == FLE_GDI_BMP_COLOR ? FLE_GDC_BITSPIXEL : 1;
    bytes = ((((uint64_t)(uint32_t)cx * bpp + 15u) >> 4) << 1) * (uint64_t)(uint32_t)cy;
    if (bytes >= UINT64_C(0x80000000)) fle_stop(api, "the size of the bitmap is 2 GiB or more: the page gives no rule for it");
    slot = new_slot(api);
    o = &objs[slot];
    o->bm[0] = (uint32_t)cx;
    o->bm[1] = (uint32_t)cy;
    o->bm[2] = FLE_GDC_PLANES;
    o->bm[3] = bpp;
    o->fmt = fmt;
    issue(o, FLE_GDI_KIND_BITMAP);
    bmp_live++;
    R[0] = slot_handle(slot);
}

static int std_masks(uint32_t bpp, const uint32_t m[3])
{
    if (bpp == 16) return (m[0] == 0x7C00u && m[1] == 0x03E0u && m[2] == 0x001Fu) || (m[0] == 0xF800u && m[1] == 0x07E0u && m[2] == 0x001Fu);
    return bpp == 32 && m[0] == 0x00FF0000u && m[1] == 0x0000FF00u && m[2] == 0x000000FFu;
}

/* gdi32!CreateDIBSection: HBITMAP f(HDC hdc rcx, const BITMAPINFO *pbmi rdx, UINT usage r8d, VOID **ppvBits r9, HANDLE hSection [rsp+0x28], DWORD offset [rsp+0x30]).
 * Learn (CreateDIBSection): "creates a DIB that applications can write to directly. The function gives you a pointer to the location of the bitmap bit values"; "If hSection is NULL,
 * the system allocates memory for the DIB"; the return value is a handle to the new DIB with *ppvBits pointing to the bit values, NULL with *ppvBits NULL on failure (ERROR_INVALID_PARAMETER
 * for "one or more of the input parameters is invalid"); "The system closes the handle to that memory when you later delete the DIB by calling DeleteObject". Learn (BITMAPINFOHEADER):
 * stride = ((((biWidth * biBitCount) + 31) & ~31) >> 3), biSizeImage = abs(biHeight) * stride, "can be set to 0 for uncompressed RGB bitmaps"; a positive biHeight is bottom-up and a
 * negative one top-down; "If biCompression equals BI_RGB and the bitmap uses 8 bpp or less, the bitmap has a color table immediately following the BITMAPINFOHEADER ... biClrUsed ... If
 * biClrUsed is zero, the array contains the maximum number of colors for the given bitdepth; that is, 2^biBitCount colors"; BI_BITFIELDS is "Valid for 16-bpp and 32-bpp bitmaps" and three
 * DWORD masks follow the header; "For 16-bpp bitmaps, if biCompression equals BI_RGB, the format is always RGB 555".
 * Answered: hdc a live DC (the page uses it only for DIB_PAL_COLORS), usage DIB_RGB_COLORS, hSection NULL, ppvBits not NULL, a BITMAPINFOHEADER of 40 bytes with biPlanes 1, biWidth above 0,
 * biHeight not 0, biBitCount 1, 4, 8, 16, 24 or 32, biCompression BI_RGB (or BI_BITFIELDS for 16 and 32 bits with the masks of RGB 555, RGB 565 or the 8-8-8 layout), biSizeImage 0 or the
 * computed size, and a block of stride * |biHeight| bytes below 2 GiB. The block is calloc'd (ZERO-FILLED: a choice, no page gives the initial bytes), its address is stored at *ppvBits, the
 * colour table of 8 bpp and less is copied from pbmi. Everything else stops (the pages give a failure value and name no case that fails). The last error is left alone. */
void fle_imp_CreateDIBSection(void)
{
    static const char api[] = "CreateDIBSection";
    uint64_t pbmi = R[2], ppv = R[9], slot, stride, nabs, bytes;
    uint32_t usage = (uint32_t)R[8], size, planes, bpp, comp, simg, clrused, ncol = 0, masks[3] = { 0, 0, 0 }, maxc;
    int32_t w, h;
    int k;
    gdi_dib *d;
    gdi_obj *o;
    uint8_t *bits;
    char why[200];

    fle_gdi_check_thread(api);
    k = dc_class(R[1], NULL);
    if (k != DCK_SCREEN && k != DCK_MEM) fle_stop(api, "hdc is not a live DC of this runtime: the page uses it for DIB_PAL_COLORS only and does not say whether a DIB_RGB_COLORS call needs a valid one");
    if (usage != FLE_GDI_DIB_RGB_COLORS) {
        snprintf(why, sizeof why, "usage is %u, not DIB_RGB_COLORS (0): DIB_PAL_COLORS needs the logical palette of the DC (decision 0108)", (unsigned)usage);
        fle_stop(api, why);
    }
    if (pbmi == 0) fle_stop(api, "pbmi is NULL: the page does not define it");
    if (ppv == 0) fle_stop(api, "ppvBits is NULL: the page does not define it");
    if (fle_arg(5) != 0) fle_stop(api, "hSection is not NULL: a DIB over a file-mapping object is not modelled (decision 0108)");
    size = (uint32_t)fle_ld(pbmi, 4);
    w = (int32_t)(uint32_t)fle_ld(pbmi + 4u, 4);
    h = (int32_t)(uint32_t)fle_ld(pbmi + 8u, 4);
    planes = (uint32_t)fle_ld(pbmi + 12u, 2);
    bpp = (uint32_t)fle_ld(pbmi + 14u, 2);
    comp = (uint32_t)fle_ld(pbmi + 16u, 4);
    simg = (uint32_t)fle_ld(pbmi + 20u, 4);
    clrused = (uint32_t)fle_ld(pbmi + 32u, 4);
    if (size != FLE_GDI_BMIH_SIZE) {
        snprintf(why, sizeof why, "biSize is %u, not 40 (BITMAPINFOHEADER): the core, V4 and V5 headers are not modelled", (unsigned)size);
        fle_stop(api, why);
    }
    if (planes != 1) fle_stop(api, "biPlanes is not 1: the page says it must be 1");
    if (w <= 0 || h == 0) fle_stop(api, "biWidth is not above 0 or biHeight is 0: the page does not define it");
    if (bpp != 1 && bpp != 4 && bpp != 8 && bpp != 16 && bpp != 24 && bpp != 32) {
        snprintf(why, sizeof why, "biBitCount is %u: only 1, 4, 8, 16, 24 and 32 are modelled", (unsigned)bpp);
        fle_stop(api, why);
    }
    if (comp != FLE_GDI_BI_RGB && !(comp == FLE_GDI_BI_BITFIELDS && (bpp == 16 || bpp == 32))) {
        snprintf(why, sizeof why, "biCompression is %u: only BI_RGB, and BI_BITFIELDS for 16 and 32 bits, are modelled", (unsigned)comp);
        fle_stop(api, why);
    }
    stride = (((uint64_t)(uint32_t)w * bpp + 31u) & ~UINT64_C(31)) >> 3;
    nabs = h < 0 ? (uint64_t)(-(int64_t)h) : (uint64_t)h;
    bytes = stride * nabs;
    if (bytes >= UINT64_C(0x80000000)) fle_stop(api, "the size of the bits is 2 GiB or more: the page gives no rule for it");
    if (simg != 0 && simg != bytes) fle_stop(api, "biSizeImage is neither 0 nor stride * |biHeight|: the page does not say what the call does with another value");
    if (comp == FLE_GDI_BI_BITFIELDS) {
        for (uint32_t i = 0; i < 3; i++) masks[i] = (uint32_t)fle_ld(pbmi + FLE_GDI_BMIH_SIZE + 4u * i, 4);
        if (!std_masks(bpp, masks)) fle_stop(api, "the BI_BITFIELDS masks are not RGB 555, RGB 565 (16 bits) or 8-8-8 (32 bits): the page does not say which masks are valid");
    } else if (bpp <= 8) {
        maxc = 1u << bpp;
        ncol = clrused ? clrused : maxc;
        if (ncol > maxc) fle_stop(api, "biClrUsed is above 2^biBitCount: the page does not define it");
    }
    if (!fle_host_size_fits(bytes)) fle_stop(api, "out of memory");
    bits = (uint8_t *)calloc(1, (size_t)bytes);
    d = (gdi_dib *)calloc(1, sizeof *d);
    if (!bits || !d) fle_stop(api, "out of memory");
    d->bits = bits;
    d->width = (uint32_t)w;
    d->height = h;
    d->bpp = bpp;
    d->comp = comp;
    d->stride = (uint32_t)stride;
    d->ncolors = ncol;
    d->xppm = (uint32_t)fle_ld(pbmi + 24u, 4);
    d->yppm = (uint32_t)fle_ld(pbmi + 28u, 4);
    d->clrused = clrused;
    d->clrimp = (uint32_t)fle_ld(pbmi + 36u, 4);
    for (uint32_t i = 0; i < 3; i++) d->masks[i] = masks[i];
    for (uint32_t i = 0; i < ncol; i++) d->clr[i] = (uint32_t)fle_ld(pbmi + FLE_GDI_BMIH_SIZE + 4u * i, 4);
    slot = new_slot(api);
    o = &objs[slot];
    o->bm[0] = (uint32_t)w;
    o->bm[1] = (uint32_t)nabs;
    o->bm[2] = 1;
    o->bm[3] = bpp;
    o->fmt = FLE_GDI_BMP_DIB;
    o->ext = d;
    issue(o, FLE_GDI_KIND_BITMAP);
    bmp_live++;
    fle_st(ppv, 8, (uint64_t)(size_t)bits);
    R[0] = slot_handle(slot);
}

/* gdi32!GetDIBColorTable / gdi32!SetDIBColorTable: UINT f(HDC hdc rcx, UINT iStart edx, UINT cEntries r8d, RGBQUAD *prgbq r9). Learn (GetDIBColorTable): "retrieves RGB (red, green,
 * blue) color values from a range of entries in the color table of the DIB section bitmap that is currently selected into a specified device context"; (SetDIBColorTable) "sets RGB
 * color values in a range of entries in the color table of the DIB that is currently selected into a specified device context"; the return value is "the number of color table entries
 * that the function retrieves" ("sets"), zero if the function fails; "DIB section bitmaps with a biBitCount value greater than eight do not have a color table". Answered: a memory DC
 * with a DIB section of 8 bits or less selected: min(cEntries, entries - iStart) entries are copied (0 when iStart is not below the number of entries) and that number is returned (the
 * clamp is a reading of "the number of ... entries that the function retrieves"). No DIB section selected (a DC of GetDC(NULL), a memory DC holding a device-dependent bitmap or the
 * placeholder, NULL) or a DIB section of more than 8 bits: 0, the failure value (the pages name "must be selected" and no colour table). Stops: prgbq NULL, a deleted DC, any other handle. */
static void dib_color_table(const char *api, int set)
{
    uint64_t hdc = R[1], prgbq = R[9];
    uint32_t start = (uint32_t)R[2], count = (uint32_t)R[8], n;
    gdi_dc *dc = NULL;
    gdi_obj *b;
    gdi_dib *d;

    fle_gdi_check_thread(api);
    if (hdc == 0) {
        R[0] = 0;
        return;
    }
    switch (dc_class(hdc, &dc)) {
    case DCK_SCREEN: R[0] = 0; return;
    case DCK_MEM: break;
    case DCK_DELETED: fle_stop(api, "hdc was deleted by DeleteDC: the page does not say what the call returns for it");
    default: fle_stop(api, "hdc is not a DC of this runtime (a number never issued or another kind of handle): the page does not define it");
    }
    b = live_obj(dc->bmp);
    if (!b || b->fmt != FLE_GDI_BMP_DIB) {
        R[0] = 0;
        return;
    }
    d = (gdi_dib *)b->ext;
    if (d->bpp > 8) {
        R[0] = 0;
        return;
    }
    if (prgbq == 0) fle_stop(api, "prgbq is NULL: the page does not define it");
    n = start >= d->ncolors ? 0 : (count < d->ncolors - start ? count : d->ncolors - start);
    for (uint32_t i = 0; i < n; i++) {
        if (set) d->clr[start + i] = (uint32_t)fle_ld(prgbq + 4u * i, 4);
        else fle_st(prgbq + 4u * i, 4, d->clr[start + i]);
    }
    R[0] = n;
}

void fle_imp_GetDIBColorTable(void) { dib_color_table("GetDIBColorTable", 0); }
void fle_imp_SetDIBColorTable(void) { dib_color_table("SetDIBColorTable", 1); }

/* Decision 0126 (PROVISIONAL), wall 56: a COLORREF with the high-order byte 2 is a PALETTERGB specifier. Learn (PALETTERGB): "returns a palette-relative red, green, blue (RGB) specifier consisting of 2 in the high-order byte and
 * an RGB value in the three low-order bytes"; "If an output device does not support a system palette, the system uses the palette-relative RGB as though it were a conventional RGB value returned by the RGB macro". This
 * environment's display is not a palette device (fle_gdc.h: SIZEPALETTE 0, "valid only if the device driver sets the RC_PALETTE bit"), so the specifier is the RGB value in its three low bytes (the guest passes 0x02FF00FF for
 * a transparent colour key, wall 56). The high byte 1 (PALETTEINDEX: "a logical-palette index specifier") and every other high byte still stop. The number 2 is the macro's own on the page (the mingw-w64 header gives the same). */
#define FLE_COLORREF_PALETTERGB_HI 2u

static int color_plain(uint32_t c, uint32_t *rgb)
{
    if ((c >> 24) == 0) {
        *rgb = c;
        return 1;
    }
    if ((c >> 24) == FLE_COLORREF_PALETTERGB_HI) {
        *rgb = c & 0x00FFFFFFu;
        return 1;
    }
    return 0;
}

/* gdi32!SetTextColor / gdi32!SetBkColor: COLORREF f(HDC hdc rcx, COLORREF color edx). Learn (SetTextColor): "sets the text color for the specified device context"; the return value is "a color
 * reference for the previous text color as a COLORREF value", CLR_INVALID on failure; (SetBkColor) "sets the current background color to the specified color value, or to the nearest
 * physical color if the device cannot represent the specified color value"; the return value "specifies the previous background color". Answered: a memory DC and a colour whose high byte is 0
 * (a plain RGB value 0x00bbggrr): the colour is stored as it is (every 24-bit colour is representable on the virtual 32-bit display, the memory DC's 1-by-1 monochrome placeholder is a
 * choice not argued here) and the previous one is returned (the first call returns the initial colour, a choice: fle_gdi.h). Stops: a DC of GetDC(NULL) (its colours are not modelled), a colour
 * with a nonzero high byte (PALETTEINDEX and PALETTERGB need a palette), a handle that is not a live memory DC. */
static void set_dc_color(const char *api, int bk)
{
    uint32_t c = (uint32_t)R[2], *at, rgb;
    gdi_dc *dc;

    fle_gdi_check_thread(api);
    dc = drawn_dc(api, R[1], "hdc is a DC of GetDC(NULL): its colours are not modelled (only memory DCs are)");
    if (!color_plain(c, &rgb)) fle_stop(api, "the colour has a high-order byte other than 0 and 2 (PALETTEINDEX needs a palette: the page does not describe it for this DC)");
    c = rgb;
    at = bk ? &dc->bk : &dc->text;
    R[0] = *at;
    *at = c;
}

void fle_imp_SetTextColor(void) { set_dc_color("SetTextColor", 0); }
void fle_imp_SetBkColor(void) { set_dc_color("SetBkColor", 1); }

/* gdi32!CreateBrushIndirect: HBRUSH f(const LOGBRUSH *plbrush rcx). Learn (CreateBrushIndirect): "creates a logical brush that has the specified style, color, and pattern"; the return value
 * "identifies a logical brush", NULL on failure; "When you no longer need the brush, call the DeleteObject function". Learn (LOGBRUSH): { UINT lbStyle; COLORREF lbColor; ULONG_PTR lbHatch }; "If
 * lbStyle is the BS_HOLLOW or BS_PATTERN style, lbColor is ignored"; "If lbStyle is BS_HATCHED or BS_SOLID, lbColor is a COLORREF color value". Answered: BS_SOLID (0) with a colour whose high byte is
 * 0, and BS_HOLLOW (1, also named BS_NULL): a new brush handle; the table keeps the style and (for BS_SOLID) the colour, lbHatch is not read. Every other style (hatched and the pattern styles, which need
 * hatch bitmaps and pixels), a NULL plbrush and a BS_SOLID colour with a nonzero high byte (palette forms) stop. The last error is left alone. */
void fle_imp_CreateBrushIndirect(void)
{
    static const char api[] = "CreateBrushIndirect";
    uint64_t p = R[1], slot;
    uint32_t style, color;
    gdi_obj *o;
    char why[200];

    fle_gdi_check_thread(api);
    if (p == 0) fle_stop(api, "plbrush is NULL: the page does not define it");
    style = (uint32_t)fle_ld(p, 4);
    color = (uint32_t)fle_ld(p + 4u, 4);
    if (style != FLE_GDI_BS_SOLID && style != FLE_GDI_BS_HOLLOW) {
        snprintf(why, sizeof why, "lbStyle is %u: only BS_SOLID (0) and BS_HOLLOW (1) are answered, the hatched and pattern styles need patterns and pixels (decision 0108)", (unsigned)style);
        fle_stop(api, why);
    }
    if (style == FLE_GDI_BS_SOLID && !color_plain(color, &color)) fle_stop(api, "lbColor has a high-order byte other than 0 and 2 (PALETTEINDEX needs a palette): the page does not describe it for a solid brush");
    slot = new_slot(api);
    o = &objs[slot];
    o->u[0] = style;
    o->u[1] = style == FLE_GDI_BS_SOLID ? color : 0;
    issue(o, FLE_GDI_KIND_BRUSH);
    brush_live++;
    R[0] = slot_handle(slot);
}

/* gdi32!CreateHalftonePalette: HPALETTE f(HDC hdc rcx). Learn (CreateHalftonePalette): "creates a halftone palette for the specified device context (DC)"; the return value is "a handle to a logical halftone
 * palette", zero on failure; "The exact color entries are an implementation detail and are not guaranteed to be identical across devices or Windows versions"; "When you no longer need the palette, call the
 * DeleteObject function". Answered: hdc a live DC (a DC of GetDC(NULL) or a memory DC): a new palette handle. The table keeps NO entries and no count (the page gives none): the palette can be selected
 * (SelectPalette) and deleted, and asking for its count or entries (GetObjectW, GetPaletteEntries) stops. Every other hdc stops. */
void fle_imp_CreateHalftonePalette(void)
{
    static const char api[] = "CreateHalftonePalette";
    uint64_t slot;
    int k;
    gdi_obj *o;

    fle_gdi_check_thread(api);
    k = dc_class(R[1], NULL);
    if (k == DCK_DELETED) fle_stop(api, "hdc was deleted by DeleteDC: the page does not say what the call returns for it");
    if (k != DCK_SCREEN && k != DCK_MEM) fle_stop(api, "hdc is not a DC of this runtime (NULL, a number never issued or another kind of handle): the page does not define it");
    slot = new_slot(api);
    o = &objs[slot];
    o->flags = GF_OPAQUE_PAL;
    issue(o, FLE_GDI_KIND_PALETTE);
    pal_live++;
    R[0] = slot_handle(slot);
}

/* user32!GetSysColor: DWORD f(int nIndex ecx). Learn (GetSysColor): "retrieves the current color of the specified display element"; the table lists the elements and their numbers and gives NO colour value
 * for any (the page's colour table is for the Aquatic contrast theme); "If the nIndex parameter is out of range, the return value is zero". Decision 0123 item 2 (provisional): the indices 0..30 answer the
 * default colours of Windows 10/11 with the light theme, in the COLORREF form 0x00BBGGRR; any other index (below 0 or above 30, taken as a signed 32-bit int) answers 0 as the page says. EVERY VALUE OF THE
 * TABLE IS A CHOICE WITHOUT A SOURCE (decision 0123, list S-colour): taken from the default HKCU\Control Panel\Colors values that a web forum page publishes for Windows 11 (not a Microsoft page), except COLOR_HIGHLIGHT
 * (13): 0 120 215 (a search-result snippet of a Windows 10 page; the forum page says 51 153 255, which is used for COLOR_MENUHILIGHT 29). Index 25 has no element in the page and answers 0. COLOR_WINDOWTEXT (8) stays 0
 * (black, decision 0108). Withdrawn when a measurement on a real Windows gives other values. Stateless: no thread rule. */
static const uint32_t sys_colors[FLE_GDI_SYS_COLOR_COUNT] = {
    /*  0 COLOR_SCROLLBAR               */ 0x00C8C8C8u, /*  1 COLOR_BACKGROUND (DESKTOP)    */ 0x00000000u, /*  2 COLOR_ACTIVECAPTION           */ 0x00D1B499u,
    /*  3 COLOR_INACTIVECAPTION         */ 0x00DBCDBFu, /*  4 COLOR_MENU                    */ 0x00F0F0F0u, /*  5 COLOR_WINDOW                  */ 0x00FFFFFFu,
    /*  6 COLOR_WINDOWFRAME             */ 0x00646464u, /*  7 COLOR_MENUTEXT                */ 0x00000000u, /*  8 COLOR_WINDOWTEXT              */ 0x00000000u,
    /*  9 COLOR_CAPTIONTEXT             */ 0x00000000u, /* 10 COLOR_ACTIVEBORDER            */ 0x00B4B4B4u, /* 11 COLOR_INACTIVEBORDER          */ 0x00FCF7F4u,
    /* 12 COLOR_APPWORKSPACE            */ 0x00ABABABu, /* 13 COLOR_HIGHLIGHT               */ 0x00D77800u, /* 14 COLOR_HIGHLIGHTTEXT           */ 0x00FFFFFFu,
    /* 15 COLOR_BTNFACE (3DFACE)        */ 0x00F0F0F0u, /* 16 COLOR_BTNSHADOW (3DSHADOW)    */ 0x00A0A0A0u, /* 17 COLOR_GRAYTEXT                */ 0x006D6D6Du,
    /* 18 COLOR_BTNTEXT                 */ 0x00000000u, /* 19 COLOR_INACTIVECAPTIONTEXT     */ 0x00000000u, /* 20 COLOR_BTNHIGHLIGHT (3DHILIGHT)*/ 0x00FFFFFFu,
    /* 21 COLOR_3DDKSHADOW              */ 0x00696969u, /* 22 COLOR_3DLIGHT                 */ 0x00E3E3E3u, /* 23 COLOR_INFOTEXT                */ 0x00000000u,
    /* 24 COLOR_INFOBK                  */ 0x00E1FFFFu, /* 25 (no element in Learn)         */ 0x00000000u, /* 26 COLOR_HOTLIGHT                */ 0x00CC6600u,
    /* 27 COLOR_GRADIENTACTIVECAPTION   */ 0x00EAD1B9u, /* 28 COLOR_GRADIENTINACTIVECAPTION */ 0x00F2E4D7u, /* 29 COLOR_MENUHILIGHT             */ 0x00FF9933u,
    /* 30 COLOR_MENUBAR                 */ 0x00F0F0F0u,
};

void fle_imp_GetSysColor(void)
{
    int32_t idx = (int32_t)(uint32_t)R[1];

    R[0] = (idx >= 0 && idx < FLE_GDI_SYS_COLOR_COUNT) ? sys_colors[idx] : 0u;
}

/* user32!CreateIconIndirect: HICON f(PICONINFO piconinfo rcx). Learn (CreateIconIndirect): "creates an icon or cursor from an ICONINFO structure"; "The system copies the bitmaps in the ICONINFO structure
 * before creating the icon or cursor. Because the system may temporarily select the bitmaps in a device context, the hbmMask and hbmColor members of the ICONINFO structure should not already be selected into
 * a device context. The application must continue to manage the original bitmaps and delete them"; Learn (ICONINFO): fIcon TRUE is an icon and FALSE a cursor; "If this structure defines an icon, the hot spot
 * is always in the center of the icon, and this member is ignored"; hbmMask is "the icon monochrome mask bitmap"; "For monochrome icons, the hbmMask is twice the height of the icon ..., and hbmColor is NULL.
 * Also, in this case the height should be an even multiple of two. For color icons, the hbmMask and hbmColor bitmaps are the same size". Answered: a live monochrome mask bitmap (1 plane, 1 bit) and either
 * hbmColor NULL with an even mask height or a live hbmColor of the same size as the mask, neither selected into a DC: a new icon or cursor handle. The table keeps fIcon, the hot spot, the mask size and whether
 * a colour bitmap was given; it keeps no pixels (so the icon cannot be drawn: no drawing API is answered). The original bitmaps stay and are deleted by the guest. Stops: NULL, a mask that is not a monochrome
 * bitmap, a colour bitmap of another size, an odd mask height with hbmColor NULL, a bitmap selected into a DC (the page says "should not", not what happens), a handle that is not a live bitmap. */
void fle_imp_CreateIconIndirect(void)
{
    static const char api[] = "CreateIconIndirect";
    uint64_t p = R[1], hmask, hcolor, slot;
    uint32_t ficon, hx, hy;
    int has_color;
    gdi_obj *m, *c, *o;

    fle_gdi_check_thread(api);
    if (p == 0) fle_stop(api, "piconinfo is NULL: the page does not define it");
    ficon = (uint32_t)fle_ld(p, 4);
    hx = (uint32_t)fle_ld(p + 4u, 4);
    hy = (uint32_t)fle_ld(p + 8u, 4);
    hmask = fle_ld(p + 16u, 8);
    hcolor = fle_ld(p + 24u, 8);
    m = live_obj(hmask);
    if (!m || m->kind != FLE_GDI_KIND_BITMAP) fle_stop(api, "hbmMask is not a live bitmap of this runtime: the page does not define it");
    if (m->bm[2] != 1 || m->bm[3] != 1) fle_stop(api, "hbmMask is not a monochrome bitmap (1 plane, 1 bit): the page calls it the icon monochrome mask");
    if (m->sel) fle_stop(api, "hbmMask is selected into a DC: the page says it should not be and not what the call does then");
    has_color = hcolor != 0;
    if (has_color) {
        c = live_obj(hcolor);
        if (!c || c->kind != FLE_GDI_KIND_BITMAP) fle_stop(api, "hbmColor is not NULL and not a live bitmap of this runtime: the page does not define it");
        if (c->sel) fle_stop(api, "hbmColor is selected into a DC: the page says it should not be and not what the call does then");
        if (c->bm[0] != m->bm[0] || c->bm[1] != m->bm[1]) fle_stop(api, "hbmColor and hbmMask differ in size: the page says that for a colour icon they are the same size");
    } else if (m->bm[1] & 1u) {
        fle_stop(api, "hbmColor is NULL and the height of hbmMask is odd: the page says the mask of a monochrome icon is twice the height of the icon");
    }
    slot = new_slot(api);
    o = &objs[slot];
    m = live_obj(hmask);
    o->u[0] = ficon != 0;
    o->u[1] = hx;
    o->u[2] = hy;
    o->u[3] = m->bm[0];
    o->u[4] = m->bm[1];
    o->u[5] = (uint32_t)has_color;
    issue(o, FLE_GDI_KIND_ICONCUR);
    R[0] = slot_handle(slot);
}

/* ---- Decision 0126 (PROVISIONAL): the pixel model of the GDI calls (docs/decisions/0126-pixel-model-and-walls.md). Supersedes the pixel operations of decision 0111 (which wrote DIB sections only, and stopped for a
 * device-dependent bitmap, a window DC and a screen DC). A drawing call has a DESTINATION that is
 *   - a memory DC: the bitmap selected into it (a DIB section: its own block; a device-dependent bitmap: its block, made on first use; the 1-by-1 placeholder too), clipped to the bitmap;
 *   - a DC of a window or of the screen (GetDC(NULL), GetDC(hWnd), GetWindowDC, GetDCEx, BeginPaint): the DRAWING SURFACE OF THE TOP-LEVEL WINDOW (R10 of docs/investigations/detach-window.md; fle_raster.c), at
 *     the origin of the window in it, clipped to the rectangle of the window's client area (or of the whole window for GetWindowDC) intersected with the client areas of its parents; the screen DC draws on the
 *     surface of the desktop (1920 x 1080), a window that is not visible (it or a parent lacks WS_VISIBLE) has an empty visible region: the call is counted and discards its output (S-draw-hidden).
 * The clip is one rectangle (decision 0123: regions are sets of rectangles; the clip regions of a DC are still named stops, so no region reaches a drawing call).
 * Learn pages (fetched 2026-10-31, learn/index.tsv): FillRect, PatBlt, BitBlt, StretchBlt, StretchDIBits, SetDIBitsToDevice, GetPixel, SetPixel, SetPixelV, GetBitmapBits, SetBitmapBits, GetDIBits, SetDIBits,
 * LineTo, MoveToEx, Rectangle, FrameRect, InvertRect, DrawEdge, DrawFrameControl, CreatePen, SetROP2, SetStretchBltMode, Ternary and Binary Raster Operations, Lines, Device-Dependent Bitmaps. */

/* System Error Codes: 87 ERROR_INVALID_PARAMETER "The parameter is incorrect." (the pages of SetPixel, SetDIBits and StretchBlt name it for a failure; PatBlt and BitBlt give no code: S-err-param of decision 0126) */
#define FLE_GDI_ERROR_INVALID_PARAMETER_ 87u

/* weak: a link without fle_win.c (the unit tests of this file) has no window table */
void fle_win_lock_attempted(uint64_t hwnd) __attribute__((weak));
/* weak too (decision 0126): a unit-test link of this file without fle_win.c has no window table, so a window / screen DC stops by name */
int fle_win_dc_query(uint64_t hwnd, fle_win_dcq *out) __attribute__((weak));
int fle_win_frame_margins(uint64_t hwnd, int32_t m[4]) __attribute__((weak));   /* decision 0244 */
/* weak too (decision 0142): without fle_visrgn.c / the window table's region function a window DC is clipped to its client rectangle and the client rectangles of its parents only, as before */
int fle_win_dc_region(uint64_t hwnd, uint32_t clip_flags, fle_visrgn *out) __attribute__((weak));
int fle_win_dc_region_within(uint64_t hwnd, uint32_t clip_flags, const fle_rrect *within, fle_visrgn *out) __attribute__((weak));
uint32_t fle_visrgn_dc_flags(int from_getdcex, uint32_t dcx, uint32_t class_style, uint32_t wstyle) __attribute__((weak));
void fle_visrgn_intersect_rect(fle_visrgn *v, fle_rrect c) __attribute__((weak));
int fle_visrgn_intersect(fle_visrgn *a, const fle_visrgn *b) __attribute__((weak));   /* decision 0247 */
fle_rrect fle_visrgn_bounds(const fle_visrgn *v) __attribute__((weak));
int fle_visrgn_equal_rect(const fle_visrgn *v, fle_rrect r) __attribute__((weak));

typedef struct {
    fle_rsurf s;                            /* the pixels */
    fle_rrect clip;                         /* in the coordinates of s */
    int is_window;                          /* a window or screen DC: the surface of a top-level window */
    int hidden;                             /* a window that is not visible: nothing is written */
    uint32_t lid;                           /* is_window: the logical id of the top-level window (0 = the desktop) */
    uint64_t hwnd;                          /* is_window: the window of the DC (0 = the screen) */
    int32_t org[2];                         /* is_window: the origin of the DC in the surface (the window's client origin, or 0 for a whole-window DC) */
    int whole_window;
    int has_app_clip;                       /* the pieces in `pieces` are the clip (surface coordinates, each inside `clip`): an application-defined clipping region (SelectClipRgn), the visible region of the window when it is not one rectangle, or both */
    uint32_t npieces;
    int has_vis;                            /* decision 0142: the visible region of a window DC is not the one rectangle `clip`: it is `vis` (the pieces of apply_app_clip come from it) */
    fle_visrgn vis;                         /* kept at the end with `pieces`: get_target0 clears only what comes before */
    fle_rrect pieces[FLE_VISRGN_MAX];
} target;

static void pixel_stop(const char *api, const char *pre, const char *reason) __attribute__((noreturn));

static void pixel_stop(const char *api, const char *pre, const char *reason)
{
    char why[420];

    snprintf(why, sizeof why, "%s: %s (decision 0126)", pre, reason);
    fle_stop(api, why);
}

static uint32_t dib_rows(const gdi_dib *d) { return d->height < 0 ? (uint32_t)(-(int64_t)d->height) : (uint32_t)d->height; }

static void dib_view(const gdi_dib *d, fle_rsurf *v)
{
    memset(v, 0, sizeof *v);
    v->bits = d->bits;
    v->width = d->width;
    v->height = dib_rows(d);
    v->stride = d->stride;
    v->bpp = d->bpp;
    v->comp = d->comp;
    for (int i = 0; i < 3; i++) v->masks[i] = d->masks[i];
    v->ncolors = d->ncolors;
    v->clr = d->clr;
    v->bottom_up = d->height > 0;
    v->keep_x = 1;
}

static const uint32_t ddb_mono_table[2] = { 0x00000000u, 0x00FFFFFFu };      /* RGBQUAD of black and white: blue | green << 8 | red << 16 */

/* The pixel block of a device-dependent bitmap (S-ddb-lazy): made on the first call that draws into it or reads it. A colour bitmap is 32-bit BGRX top-down; a monochrome one is a 1-bit block with a black and a white
 * entry. Each row is padded to a multiple of 4 bytes (S-ddb-stride: the CreateBitmap page's WORD alignment of the guest's buffer is not the layout of the block, which the guest cannot see). */
static int ddb_view(gdi_obj *o, fle_rsurf *v)
{
    gdi_ddb *d = (gdi_ddb *)o->ext;
    uint32_t w = o->bm[0], h = o->bm[1], bpp = o->bm[3];

    if (!d) {
        uint64_t stride = (((uint64_t)w * bpp + 31u) & ~UINT64_C(31)) >> 3, bytes = stride * h;

        if (bytes >= UINT64_C(0x80000000) || !fle_host_size_fits(bytes)) return 0;
        d = (gdi_ddb *)calloc(1, sizeof *d);
        if (!d) return 0;
        d->bits = (uint8_t *)calloc(1, (size_t)bytes);
        if (!d->bits) {
            free(d);
            return 0;
        }
        d->stride = (uint32_t)stride;
        o->ext = d;
    }
    memset(v, 0, sizeof *v);
    v->bits = d->bits;
    v->width = w;
    v->height = h;
    v->stride = d->stride;
    v->bpp = bpp;
    if (bpp == 1) {
        v->ncolors = 2;
        v->clr = ddb_mono_table;
    }
    return 1;
}

/* Decision 0140: the pixels of the bitmap selected into the memory DC `h` (UpdateLayeredWindow reads its source through this; fle_layered.c). 1 and the view in *out; 0 when h is not a live memory DC with a bitmap. */
int fle_gdi_memdc_surface(uint64_t h, fle_rsurf *out)
{
    gdi_dc *dc = NULL;
    gdi_obj *b;

    if (dc_class(h, &dc) != DCK_MEM) return 0;
    b = live_obj(dc->bmp);
    if (!b || b->kind != FLE_GDI_KIND_BITMAP) return 0;
    if (b->fmt == FLE_GDI_BMP_DIB) dib_view((const gdi_dib *)b->ext, out);
    else if (!ddb_view(b, out)) return 0;
    return 1;
}

/* The target of a drawing call: the DC `h` (a memory DC or a window / screen DC). `pre` names the arguments in every stop. */
static void apply_app_clip(uint64_t h, target *t);
static void get_target0(const char *api, const char *pre, uint64_t h, const char *role, target *t)
{
    gdi_dc *dc = NULL;
    char why[320];

    memset(t, 0, offsetof(target, vis));
    t->vis.n = 0;
    switch (dc_class(h, &dc)) {
    case DCK_MEM: {
        gdi_obj *b = live_obj(dc->bmp);

        if (!b || b->kind != FLE_GDI_KIND_BITMAP) pixel_stop(api, pre, "the bitmap selected into the DC is gone");
        if (b->fmt == FLE_GDI_BMP_DIB) dib_view((const gdi_dib *)b->ext, &t->s);
        else if (!ddb_view(b, &t->s)) pixel_stop(api, pre, "out of memory for the pixels of a device-dependent bitmap");
        t->clip = fle_r_bounds(&t->s);
        return;
    }
    case DCK_SCREEN: {
        fle_gdc_info gi;
        fle_win_dcq q;
        fle_rsurf *surf;
        int32_t w, ht;

        if (!fle_gdc_dc_info(h, &gi)) pixel_stop(api, pre, "the DC is not live");
        if (gi.kind == FLE_GDC_K_SCREEN) {
            if (!fle_win_dc_query) pixel_stop(api, pre, "no window table is linked: the screen DC has no surface");
            fle_win_dc_query(FLE_DESKTOP_HANDLE, &q);
            t->lid = 0;
            t->clip.x0 = 0;
            t->clip.y0 = 0;
            t->clip.x1 = q.root_cx;
            t->clip.y1 = q.root_cy;
            t->org[0] = t->org[1] = 0;
            w = q.root_cx;
            ht = q.root_cy;
        } else {
            if (!fle_win_dc_query) pixel_stop(api, pre, "no window table is linked: a window DC has no surface");
            if (!fle_win_dc_query(gi.hwnd, &q)) pixel_stop(api, pre, "the window of the DC is gone");
            t->hwnd = gi.hwnd;
            t->lid = q.root_lid;
            w = q.root_cx;
            ht = q.root_cy;
            t->whole_window = gi.kind == FLE_GDC_K_WINDOW;
            if (t->whole_window) {
                t->org[0] = q.org_x;
                t->org[1] = q.org_y;
                t->clip.x0 = q.org_x;
                t->clip.y0 = q.org_y;
                t->clip.x1 = q.org_x + q.cx;
                t->clip.y1 = q.org_y + q.cy;
            } else {
                t->org[0] = q.org_x;
                t->org[1] = q.org_y;
                t->clip.x0 = q.clip_l;
                t->clip.y0 = q.clip_t;
                t->clip.x1 = q.clip_r;
                t->clip.y1 = q.clip_b;
            }
            t->hidden = !q.visible;
            /* Decision 0142: the visible region of the window's DC: its rectangle minus the visible children (WS_CLIPCHILDREN) and the visible siblings above it (WS_CLIPSIBLINGS), kept inside the parents; BeginPaint clips to
             * the update region. When the window is not visible the old rule (the call is discarded and counted) stays. */
            if (q.visible && fle_win_dc_region && fle_visrgn_dc_flags) {
                uint32_t vflags = fle_visrgn_dc_flags((int)gi.from_ex, gi.from_ex ? gi.flags : 0u, gi.class_style, q.style);
                /* a DC made by BeginPaint draws only inside the update rectangle (the intersection below): the region is made for that rectangle alone, so that the children and siblings elsewhere do not count against the room of a region (decision 0240) */
                fle_rrect pr = { (int64_t)gi.paint[0] + q.org_x, (int64_t)gi.paint[1] + q.org_y, (int64_t)gi.paint[2] + q.org_x, (int64_t)gi.paint[3] + q.org_y };
                int rr = gi.paint_valid && fle_win_dc_region_within ? fle_win_dc_region_within(gi.hwnd, vflags, &pr, &t->vis) : fle_win_dc_region(gi.hwnd, vflags, &t->vis);

                if (rr < 0) pixel_stop(api, pre, "the visible region of the window has more rectangles than this runtime keeps (decision 0142)");
                if (rr > 0) {
                    if (gi.paint_valid) {
                        fle_visrgn_intersect_rect(&t->vis, (fle_rrect){ (int64_t)gi.paint[0] + q.org_x, (int64_t)gi.paint[1] + q.org_y, (int64_t)gi.paint[2] + q.org_x, (int64_t)gi.paint[3] + q.org_y });
                        if (gi.paint_np > 1) {
                            /* decision 0247 (alpha3): the update region of the PAINTSTRUCT is more than one rectangle (Learn BeginPaint: the DC is clipped "to exclude any area outside the update region"): the visible region is cut by the pieces */
                            fle_visrgn pr_;

                            pr_.n = gi.paint_np;
                            for (uint32_t k = 0; k < gi.paint_np; k++)
                                pr_.r[k] = (fle_rrect){ (int64_t)gi.pp[k][0] + q.org_x, (int64_t)gi.pp[k][1] + q.org_y, (int64_t)gi.pp[k][2] + q.org_x, (int64_t)gi.pp[k][3] + q.org_y };
                            if (fle_visrgn_intersect && !fle_visrgn_intersect(&t->vis, &pr_)) pixel_stop(api, pre, "the visible region of the window cut by the update region has more rectangles than this runtime keeps (decision 0247)");
                        }
                    }
                    t->clip = fle_visrgn_bounds(&t->vis);
                    t->has_vis = !fle_visrgn_equal_rect(&t->vis, t->clip);
                }
            }
        }
        t->is_window = 1;
        surf = fle_raster_window_surface(t->lid, (uint32_t)(w > 0 ? w : 0), (uint32_t)(ht > 0 ? ht : 0));
        if (!surf) {
            if (w <= 0 || ht <= 0) {
                /* a window with no area has an empty surface and an empty visible region: nothing can be drawn */
                memset(&t->s, 0, sizeof t->s);
                t->hidden = 1;
                return;
            }
            pixel_stop(api, pre, "the drawing surface of the top-level window cannot be made (a size above 16384 or no memory)");
        }
        t->s = *surf;
        {
            /* decision 0244: a window whose frame is extended into the client area (DwmExtendFrameIntoClientArea) has a surface whose fourth byte is an alpha that the DWM composes (fle_desk.c), so a plain SRCCOPY from a 32-bit
             * DIB copies it like it copies the unused byte between two DIB sections (keep_x, decision 0111). Another window's surface has no such byte. */
            int32_t fm[4];

            if (gi.kind != FLE_GDC_K_SCREEN && fle_win_frame_margins && fle_win_frame_margins(q.root, fm)) t->s.keep_x = 1;
        }
        fle_r_clip(&t->clip, &(fle_rrect){ 0, 0, (int64_t)t->s.width, (int64_t)t->s.height });
        return;
    }
    case DCK_DELETED: snprintf(why, sizeof why, "%s was deleted by DeleteDC: the page does not say what the call returns for it", role); break;
    default: snprintf(why, sizeof why, "%s is not a DC of this runtime (NULL, a number never issued or another kind of handle): the page does not define it", role); break;
    }
    pixel_stop(api, pre, why);
}

/* Decision 0126: the application-defined clip region of the DC `h` (device units of the DC: the origin of the DC in its surface is added) intersected with the visible rectangle of the target: `has_app_clip` and the
 * pieces. A region with no piece (an empty one) leaves nothing to draw. */
static void apply_app_clip(uint64_t h, target *t)
{
    gdi_dc *dc = NULL;
    uint32_t k;

    t->has_app_clip = 0;
    t->npieces = 0;
    if (dc_class(h, &dc) != DCK_MEM) dc = screen_state("pixel", h);
    if (!dc || !dc->clip.has) {
        if (t->has_vis) {
            /* decision 0142: the visible region alone is the clip */
            t->has_app_clip = 1;
            for (k = 0; k < t->vis.n; k++) t->pieces[t->npieces++] = t->vis.r[k];
        }
        return;
    }
    t->has_app_clip = 1;
    for (k = 0; k < dc->clip.n; k++) {
        fle_rrect r = { (int64_t)dc->clip.r[k].l + t->org[0], (int64_t)dc->clip.r[k].t + t->org[1], (int64_t)dc->clip.r[k].r + t->org[0], (int64_t)dc->clip.r[k].b + t->org[1] };

        fle_r_clip(&r, &t->clip);
        if (fle_r_empty(&r)) continue;
        if (!t->has_vis) {
            t->pieces[t->npieces++] = r;
            continue;
        }
        /* decision 0142: the application-defined piece cut by each piece of the visible region (both are disjoint sets, so the cuts are disjoint) */
        for (uint32_t v = 0; v < t->vis.n; v++) {
            fle_rrect q = r;

            fle_r_clip(&q, &t->vis.r[v]);
            if (fle_r_empty(&q)) continue;
            if (t->npieces == FLE_VISRGN_MAX) pixel_stop("clip", "the clip of a window DC", "the application-defined clipping region cut by the visible region of the window has more pieces than this runtime keeps (decision 0142)");
            t->pieces[t->npieces++] = q;
        }
    }
}

static void get_target(const char *api, const char *pre, uint64_t h, const char *role, target *t)
{
    get_target0(api, pre, h, role, t);
    apply_app_clip(h, t);
}

/* The attribute state of the DC of a target: a memory DC's, or the screen state of a window / screen DC (fle_gdi.c keeps one for every live GetDC handle: the attribute calls made it, or the defaults). */
static gdi_dc *target_attrs(uint64_t h)
{
    gdi_dc *dc = NULL;

    if (dc_class(h, &dc) == DCK_MEM) return dc;
    return screen_state("pixel", h);
}

static void mode_check(const char *api, const char *pre, const gdi_dc *dc, const char *role)
{
    char why[200];

    if (dc->map_mode != FLE_GDI_MM_TEXT) {
        snprintf(why, sizeof why, "%s is in mapping mode %u, not MM_TEXT: the extents of the other modes are not on the pages", role, (unsigned)dc->map_mode);
        pixel_stop(api, pre, why);
    }
}

/* The device point of the logical point (x, y) of the DC in surface coordinates: Dx = Lx - WOx + VOx (MM_TEXT, decision 0111) plus the origin of the DC in its surface. */
static fle_rrect dev_rect(const gdi_dc *dc, const target *t, int64_t x, int64_t y, int64_t w, int64_t h)
{
    fle_rrect r;

    r.x0 = x - dc->worg[0] + dc->vorg[0] + t->org[0];
    r.y0 = y - dc->worg[1] + dc->vorg[1] + t->org[1];
    r.x1 = r.x0 + w;
    r.y1 = r.y0 + h;
    return r;
}

/* The clip pieces of a target: without an application-defined clipping region the one visible rectangle, with one the pieces of the region inside it (disjoint, the normal form of decision 0123: a pixel is drawn at most
 * once, which a XOR or an invert needs). EACH_CLIP runs a statement once per piece with CL_ naming the piece. A copy inside ONE surface that spans several pieces (the source read after the destination was written) is
 * not exact when the pieces are visited one after the other; the pieces are visited in order and the case is stated in the decision (S-clip-overlap). */
static uint32_t clip_pieces(const target *t, const fle_rrect **list)
{
    if (!t->has_app_clip) {
        *list = &t->clip;
        return 1;
    }
    *list = t->pieces;
    return t->npieces;
}

/* a point of the target's clip: inside the rectangle and, when there are pieces (an application-defined region or the visible region of a window that is not one rectangle), inside one of them (decision 0142: GetPixel and
 * SetPixel used the rectangle only) */
static int target_has_point(const target *t, int64_t x, int64_t y)
{
    if (x < t->clip.x0 || x >= t->clip.x1 || y < t->clip.y0 || y >= t->clip.y1) return 0;
    if (!t->has_app_clip) return 1;
    for (uint32_t i = 0; i < t->npieces; i++)
        if (x >= t->pieces[i].x0 && x < t->pieces[i].x1 && y >= t->pieces[i].y0 && y < t->pieces[i].y1) return 1;
    return 0;
}

#define EACH_CLIP(T, ...) do { const fle_rrect *cl_; uint32_t nc_ = clip_pieces(&(T), &cl_); for (uint32_t ci_ = 0; ci_ < nc_; ci_++) { const fle_rrect *CL_ = &cl_[ci_]; __VA_ARGS__; } } while (0)

/* The pixels were written by `api`: a window surface records it (and the first non-background pixel is the evidence). The first write of a window that is not visible is discarded and counted instead. */
static int target_live(const char *api, const target *t)
{
    if (t->is_window && t->hidden) {
        fle_raster_discarded(api, t->lid);
        if (t->hwnd && fle_win_lock_attempted) fle_win_lock_attempted(t->hwnd);       /* LockWindowUpdate: the extent of the attempt is recorded, the unlock invalidates it */
        return 0;
    }
    return 1;
}


static void target_wrote(const char *api, const target *t, fle_rrect r, uint64_t n)
{
    if (t->is_window && n) {
        fle_rrect c = t->clip;

        fle_r_clip(&r, &c);
        fle_raster_wrote(fle_raster_surface_peek(t->lid), r, api);
    }
}

/* Decision 0245 (alpha2): the plain copies into a WINDOW surface compare what they write (fle_raster.c, fle_r_changed): blit_begin before the fle_r_blit calls, blit_end before target_wrote. A copy that left the surface as it was
 * (the guest draws the same picture again) reports 0 pixels, so the compositor is not told; one that changed it reports the rows that differed. A memory DC is not compared (nothing is told to anyone about it). */
static int blit_begin(const target *t)
{
    if (!t->is_window) return 0;
    fle_r_changed_begin();
    return 1;
}
static void blit_end(int begun, fle_rrect *dr, uint64_t *n)
{
    fle_rrect w = *dr;

    if (!begun) return;
    if (!fle_r_changed(&w)) *n = 0;
    else *dr = w;
}

/* The colour of a brush: a live solid brush made by CreateBrushIndirect or CreateSolidBrush; the stock brushes WHITE_BRUSH, LTGRAY_BRUSH, GRAY_BRUSH, DKGRAY_BRUSH, BLACK_BRUSH are solid (S-stock-brush-colour: the
 * pages give no colours; the Win32 names give white and black, the grays are 0xC0C0C0, 0x808080, 0x404040 as the values of the SDK headers' documentation of the 1990s: a choice); NULL_BRUSH / HOLLOW_BRUSH, a hatched or
 * pattern brush stop (they are in the stop condition (d') of decision 0126). The DEFAULT brush of a DC is WHITE_BRUSH (Display Device Context Defaults: "Brush WHITE_BRUSH"). `*hollow` = 1: the brush paints nothing
 * (NULL_BRUSH: the page "Null brush (equivalent to HOLLOW_BRUSH)"); a hollow brush is valid for PatBlt and FillRect and leaves the destination unchanged (S-hollow-fill). */
static uint32_t brush_color(const char *api, const char *pre, uint64_t hbr, int hbr_is_arg, int *hollow)
{
    gdi_obj *o = live_obj(hbr);

    *hollow = 0;
    if (hbr_is_arg && hbr >= 1u && hbr <= FLE_GDI_SYS_COLOR_COUNT) return sys_colors[hbr - 1u];       /* a system colour index plus 1 (FillRect page; the table of decision 0123) */
    if (!o) pixel_stop(api, pre, hbr_is_arg ? "hbr is not a live brush of this runtime nor a system colour index plus 1 (0 to 30)" : "the brush selected into the DC is gone");
    switch (o->kind) {
    case FLE_GDI_KIND_BRUSH:
        if (o->u[0] == FLE_GDI_BS_PATTERN) pixel_stop(api, pre, "the brush is a pattern brush (CreatePatternBrush): the pixels of a pattern fill are not implemented (decision 0132, S-patternbrush)");
        if (o->u[0] != FLE_GDI_BS_SOLID) {
            *hollow = 1;
            return 0;
        }
        return o->u[1];
    case FLE_GDI_KIND_STOCK_BRUSH:
        if (o->fmt == FLE_GDI_STOCK_NULL_BRUSH) {
            *hollow = 1;
            return 0;
        }
        switch (o->fmt) {
        case 0: return 0xFFFFFFu;
        case 1: return 0xC0C0C0u;
        case 2: return 0x808080u;
        case 3: return 0x404040u;
        default: return 0x000000u;
        }
    case FLE_GDI_KIND_DEFAULT:
        if (o->fmt == FLE_GDI_DEFAULT_BRUSH) return 0xFFFFFFu;
        pixel_stop(api, pre, "the handle is not a brush");
    default: pixel_stop(api, pre, "the handle is not a brush");
    }
}

/* ---- the raster-operation argument ---- */

static uint32_t ternary(const char *api, const char *pre, uint32_t rop, int needs_src)
{
    uint32_t idx;

    if (!fle_r_rop3_valid(rop & ~UINT32_C(0x40000000), &idx)) pixel_stop(api, pre, "rop is not one of the 256 ternary raster-operation codes of the page (a CAPTUREBLT or NOMIRRORBITMAP flag is accepted and ignored)");
    if (!needs_src && fle_r_idx_uses_source(idx)) pixel_stop(api, pre, "the raster operation uses a source and this call has none");
    return idx;
}

/* The pattern colour of a ternary operation that uses the brush, taken from the brush selected into the DC. A hollow brush with a pattern operation: nothing is painted (S-hollow-fill). */
static int pattern_of(const char *api, const char *pre, uint32_t idx, const gdi_dc *dc, uint32_t *pat)
{
    int hollow;

    *pat = 0;
    if (!fle_r_idx_uses_pattern(idx)) return 1;
    *pat = brush_color(api, pre, dc->brush, 0, &hollow);
    return !hollow;
}

static void fill_check(const char *api, const char *pre, const target *t, const char *role)
{
    char why[200];

    if (t->hidden) return;
    if (!fle_r_writable(&t->s)) {
        snprintf(why, sizeof why, "%s: the pixel format is not a 24- or 32-bit RGB form or a monochrome one: the colour-to-pixel mapping of the other forms (palette index, 5-5-5 / 5-6-5) is not on the pages", role);
        pixel_stop(api, pre, why);
    }
}

static int source_ok(const char *api, const char *pre, const target *t, const char *role)
{
    char why[200];

    if (!fle_r_readable(&t->s)) {
        snprintf(why, sizeof why, "%s has a pixel format that cannot be read (16 bits per pixel: the 5-5-5 and 5-6-5 layouts have no page for the conversion to a colour)", role);
        pixel_stop(api, pre, why);
    }
    return 1;
}

/* gdi32!BitBlt: BOOL f(HDC hdc rcx, int x edx, int y r8d, int cx r9d, int cy [5], HDC hdcSrc [6], int x1 [7], int y1 [8], DWORD rop [9]). Learn (BitBlt): "performs a bit-block transfer of the color data corresponding
 * to a rectangle of pixels from the specified source device context into a destination device context"; rop is "a raster-operation code" (the 256 of the Ternary Raster Operations page; CAPTUREBLT "Includes any windows
 * that are layered on top of your window", NOMIRRORBITMAP); "BitBlt only does clipping on the destination DC"; "If the color formats of the source and destination device contexts do not match, the BitBlt function
 * converts the source color format to match the destination format"; the return value is nonzero on success. Answered (decision 0126): any of the 256 codes; the destination is a memory DC or a window / screen DC (above);
 * the source is a memory DC or a window / screen DC (it reads the surface of its top-level window); both in MM_TEXT. A source rectangle (cx, cy) that is not inside the source bitmap or surface: the part outside is not
 * read: the destination pixels that would take it are left unchanged (S-source-outside: the page says only that clipping is done on the destination; the colour of a pixel outside the source is not on the pages). A negative
 * cx or cy: failure, 0 (S-negative-extent: the page says nothing; the guest never does it). The unused byte of a 32-bit pixel follows decision 0111. A pattern operation uses the brush selected into the destination DC
 * (solid brushes only). */
static void blit_call(const char *api, const char *pre, uint64_t hdst, int64_t x, int64_t y, int64_t cx, int64_t cy, uint64_t hsrc, int64_t x1, int64_t y1, uint32_t rop, int stretch, int64_t sw, int64_t sh)
{
    target td, ts;
    gdi_dc *dd, *sd;
    fle_rrect dr, sr;
    int chk = 0;   /* decision 0245: fle_r_changed_begin was called */
    uint32_t idx, pat = 0;
    uint64_t n = 0;
    int us;

    idx = ternary(api, pre, rop, 1);
    us = fle_r_idx_uses_source(idx);
    get_target(api, pre, hdst, "hdc", &td);
    dd = target_attrs(hdst);
    mode_check(api, pre, dd, "hdc");
    fill_check(api, pre, &td, "hdc");
    if (us) {
        get_target(api, pre, hsrc, "hdcSrc", &ts);
        sd = target_attrs(hsrc);
        mode_check(api, pre, sd, "hdcSrc");
        source_ok(api, pre, &ts, "hdcSrc");
    } else {
        memset(&ts, 0, sizeof ts);
        sd = dd;
    }
    if (!pattern_of(api, pre, idx, dd, &pat)) {
        R[0] = 1;
        return;
    }
    /* decision 0126 (S-mono-convert): the colours of the conversion between a monochrome and a colour surface are the text colour (zero bits: "foreground") and the background colour (one bits) of the DC with colour. The DC with
     * colour is the destination for a monochrome source and the source for a monochrome destination (the page StretchBlt: "The foreground and background colors of the device context with color are used"). */
    if (us && ts.s.bpp == 1 && td.s.bpp != 1) {
        ts.s.conv_set = 1;
        ts.s.conv_fg = dd->text;
        ts.s.conv_bg = dd->bk;
    }
    if (us && td.s.bpp == 1 && ts.s.bpp != 1) {
        td.s.mono_key_set = 1;
        td.s.mono_key = sd->bk;
    }
    if (cx < 0 || cy < 0 || (stretch && (sw == 0 || sh == 0) && us)) {
        if (!stretch) {
            fle_set_last_error(FLE_GDI_ERROR_INVALID_PARAMETER_);
            R[0] = 0;
            return;
        }
    }
    if (!target_live(api, &td)) {
        R[0] = 1;
        return;
    }
    dr = dev_rect(dd, &td, x, y, cx, cy);
    if (!stretch) {
        sr = us ? dev_rect(sd, &ts, x1, y1, cx, cy) : dr;
        if (cx == 0 || cy == 0) {
            R[0] = 1;
            return;
        }
        if (us) {
            /* the part of the source rectangle outside the source (or outside its visible part: the window case) is not read: the destination keeps its pixels there (S-source-outside) */
            fle_rrect srcbounds = ts.is_window ? ts.clip : fle_r_bounds(&ts.s), in = sr, dest_in;

            fle_r_clip(&in, &srcbounds);
            if (fle_r_empty(&in)) {
                R[0] = 1;
                return;
            }
            dest_in.x0 = dr.x0 + (in.x0 - sr.x0);
            dest_in.y0 = dr.y0 + (in.y0 - sr.y0);
            dest_in.x1 = dest_in.x0 + (in.x1 - in.x0);
            dest_in.y1 = dest_in.y0 + (in.y1 - in.y0);
            n = 0;
            chk = blit_begin(&td);
            EACH_CLIP(td, n += fle_r_blit(&td.s, CL_, dest_in.x0, dest_in.y0, in.x1 - in.x0, in.y1 - in.y0, &ts.s, in.x0, in.y0, idx, pat));
            dr = dest_in;
        } else {
            n = 0;
            EACH_CLIP(td, n += fle_r_rop_rect(&td.s, CL_, dr, idx, pat));
        }
    } else {
        fle_rrect sr2;
        int64_t ssx = x1, ssy = y1;

        sr2 = us ? dev_rect(sd, &ts, ssx, ssy, sw, sh) : dr;
        if (cx == 0 || cy == 0 || (us && (sw == 0 || sh == 0))) {
            R[0] = 1;
            return;
        }
        if (us) {
            int64_t lx = sw < 0 ? sr2.x0 + sw : sr2.x0, ly = sh < 0 ? sr2.y0 + sh : sr2.y0, ex = sw < 0 ? sr2.x0 : sr2.x0 + sw, ey = sh < 0 ? sr2.y0 : sr2.y0 + sh;
            fle_rrect srcbounds = ts.is_window ? ts.clip : fle_r_bounds(&ts.s);

            if (lx < srcbounds.x0 || ly < srcbounds.y0 || ex > srcbounds.x1 || ey > srcbounds.y1)
                pixel_stop(api, pre, "the source rectangle is not inside the source: a stretched copy of a partly missing source has no rule on the pages (BitBlt clips the destination only)");
            n = 0;
            /* decision 0126: the same size and the same direction is a plain copy (BitBlt's path: a SRCCOPY between two surfaces of one format is a byte copy, the unused byte of a 32-bit pixel included, decision 0111);
             * a different size or a mirror is stretched (S-stretch-groups) */
            if ((cx < 0 ? -cx : cx) == (sw < 0 ? -sw : sw) && (cy < 0 ? -cy : cy) == (sh < 0 ? -sh : sh) && (cx < 0) == (sw < 0) && (cy < 0) == (sh < 0)) {
                chk = blit_begin(&td);
                EACH_CLIP(td, n += fle_r_blit(&td.s, CL_, cx < 0 ? dr.x0 + cx : dr.x0, cy < 0 ? dr.y0 + cy : dr.y0, cx < 0 ? -cx : cx, cy < 0 ? -cy : cy, &ts.s, sw < 0 ? sr2.x0 + sw : sr2.x0, sh < 0 ? sr2.y0 + sh : sr2.y0, idx, pat));
            } else {
                EACH_CLIP(td, n += fle_r_stretch(&td.s, CL_, dr.x0, dr.y0, cx, cy, &ts.s, sr2.x0, sr2.y0, sw, sh, idx, pat, dd->stretch));
            }
        } else {
            n = 0;
            EACH_CLIP(td, n += fle_r_rop_rect(&td.s, CL_, dr, idx, pat));
        }
    }
    blit_end(chk, &dr, &n);
    target_wrote(api, &td, dr, n);
    R[0] = 1;
}

void fle_imp_BitBlt(void)
{
    static const char api[] = "BitBlt";
    int32_t x = (int32_t)(uint32_t)R[2], y = (int32_t)(uint32_t)R[8], cx = (int32_t)(uint32_t)R[9], cy = (int32_t)(uint32_t)fle_arg(5);
    int32_t x1 = (int32_t)(uint32_t)fle_arg(7), y1 = (int32_t)(uint32_t)fle_arg(8);
    uint32_t rop = (uint32_t)fle_arg(9);
    char pre[200];

    fle_gdi_check_thread(api);
    snprintf(pre, sizeof pre, "pixel transfer between DCs (rop 0x%x, %d x %d to (%d, %d) from (%d, %d))", (unsigned)rop, (int)cx, (int)cy, (int)x, (int)y, (int)x1, (int)y1);
    blit_call(api, pre, R[1], x, y, cx, cy, fle_arg(6), x1, y1, rop, 0, 0, 0);
}

/* gdi32!PatBlt: BOOL f(HDC hdc rcx, int x edx, int y r8d, int w r9d, int h [5], DWORD rop [6]). Learn (PatBlt): "paints the specified rectangle using the brush that is currently selected into the specified device
 * context. The brush color and the surface color or colors are combined by using the specified raster operation"; PATCOPY, PATINVERT, DSTINVERT, BLACKNESS, WHITENESS; "an operation code that refers to a source
 * rectangle cannot be used" (failure, 0, with ERROR_INVALID_PARAMETER: S-patblt-source); the return value is nonzero on success. Answered: every ternary code that uses no source (the sixteen of the page's table that
 * do not mention S), in MM_TEXT, on any target above. A negative w or h is failure, 0 (S-negative-extent); a zero one paints nothing and returns 1. */
void fle_imp_PatBlt(void)
{
    static const char api[] = "PatBlt";
    int32_t x = (int32_t)(uint32_t)R[2], y = (int32_t)(uint32_t)R[8], w = (int32_t)(uint32_t)R[9], h = (int32_t)(uint32_t)fle_arg(5);
    uint32_t rop = (uint32_t)fle_arg(6), idx, pat = 0;
    target t;
    gdi_dc *dc;
    fle_rrect r;
    uint64_t n;
    char pre[200];

    fle_gdi_check_thread(api);
    snprintf(pre, sizeof pre, "painting a rectangle with the selected brush (rop 0x%x, %d x %d at (%d, %d))", (unsigned)rop, (int)w, (int)h, (int)x, (int)y);
    if (!fle_r_rop3_valid(rop & ~UINT32_C(0x40000000), &idx)) pixel_stop(api, pre, "rop is not one of the 256 ternary raster-operation codes of the page");
    get_target(api, pre, R[1], "hdc", &t);
    dc = target_attrs(R[1]);
    mode_check(api, pre, dc, "hdc");
    fill_check(api, pre, &t, "hdc");
    if (fle_r_idx_uses_source(idx)) {
        fle_set_last_error(FLE_GDI_ERROR_INVALID_PARAMETER_);
        R[0] = 0;
        return;
    }
    if (!pattern_of(api, pre, idx, dc, &pat)) {
        R[0] = 1;
        return;
    }
    if (w < 0 || h < 0) {
        fle_set_last_error(FLE_GDI_ERROR_INVALID_PARAMETER_);
        R[0] = 0;
        return;
    }
    R[0] = 1;
    if (w == 0 || h == 0 || !target_live(api, &t)) return;
    r = dev_rect(dc, &t, x, y, w, h);
    n = 0;
    EACH_CLIP(t, n += fle_r_rop_rect(&t.s, CL_, r, idx, pat));
    target_wrote(api, &t, r, n);
}

/* user32!FillRect: int f(HDC hDC rcx, const RECT *lprc rdx, HBRUSH hbr r8). Learn (FillRect): "fills a rectangle by using the specified brush. This function includes the left and top borders, but excludes the right
 * and bottom borders"; the rectangle is in logical coordinates; hbr "may be either a handle to a logical brush or a color value" (a system colour index plus 1: "FillRect(hdc, &rect, (HBRUSH) (COLOR_WINDOW+1))"); the
 * return value is nonzero on success. Answered (decision 0126): a solid or stock brush, or COLOR_xxx + 1 for the 31 system colours of decision 0123; any target above; an empty or inverted rectangle paints nothing and
 * returns 1 (a READING of the half-open set). A hollow brush paints nothing (S-hollow-fill). The fill is PATCOPY: the raster operation of the DC (SetROP2) is not used by FillRect (S-fillrect-copy). */
void fle_imp_FillRect(void)
{
    static const char api[] = "FillRect";
    uint64_t prc = R[2], hbr = R[8];
    int32_t l, t_, rt, b;
    uint32_t cref;
    int hollow;
    target t;
    gdi_dc *dc;
    fle_rrect r;
    uint64_t n;
    char pre[120];

    fle_gdi_check_thread(api);
    snprintf(pre, sizeof pre, "filling a rectangle with a brush (hbr 0x%" PRIx64 ")", hbr);
    get_target(api, pre, R[1], "hDC", &t);
    dc = target_attrs(R[1]);
    mode_check(api, pre, dc, "hDC");
    fill_check(api, pre, &t, "hDC");
    if (prc == 0) pixel_stop(api, pre, "lprc is NULL: the page does not define it");
    cref = brush_color(api, pre, hbr, 1, &hollow);
    l = (int32_t)(uint32_t)fle_ld(prc, 4);
    t_ = (int32_t)(uint32_t)fle_ld(prc + 4u, 4);
    rt = (int32_t)(uint32_t)fle_ld(prc + 8u, 4);
    b = (int32_t)(uint32_t)fle_ld(prc + 12u, 4);
    R[0] = 1;
    if (hollow || !target_live(api, &t)) return;
    r = dev_rect(dc, &t, l, t_, 0, 0);
    r.x1 = r.x0 + ((int64_t)rt - l);
    r.y1 = r.y0 + ((int64_t)b - t_);
    n = 0;
    EACH_CLIP(t, n += fle_r_rop_rect(&t.s, CL_, r, FLE_R_PATCOPY_IDX, cref));
    target_wrote(api, &t, r, n);
}

/* user32!InvertRect: BOOL f(HDC hDC rcx, const RECT *lprc rdx). Learn (InvertRect): "inverts a rectangle in a window by performing a logical NOT operation on the color values for each pixel in the rectangle's
 * interior"; "Calling InvertRect twice for the same rectangle restores the display to its previous colors"; the rectangle is in logical coordinates; nonzero on success. Answered: DSTINVERT of the rectangle [left, right) x
 * [top, bottom) (the half-open rule of the other rectangle calls is a READING: the page does not say which edges "the interior" includes: S-invertrect-edges). */
void fle_imp_InvertRect(void)
{
    static const char api[] = "InvertRect";
    uint64_t prc = R[2];
    int32_t l, t_, rt, b;
    target t;
    gdi_dc *dc;
    fle_rrect r;
    uint64_t n;
    char pre[80];

    fle_gdi_check_thread(api);
    snprintf(pre, sizeof pre, "inverting a rectangle (hDC 0x%" PRIx64 ")", R[1]);
    get_target(api, pre, R[1], "hDC", &t);
    dc = target_attrs(R[1]);
    mode_check(api, pre, dc, "hDC");
    fill_check(api, pre, &t, "hDC");
    if (prc == 0) pixel_stop(api, pre, "lprc is NULL: the page does not define it");
    l = (int32_t)(uint32_t)fle_ld(prc, 4);
    t_ = (int32_t)(uint32_t)fle_ld(prc + 4u, 4);
    rt = (int32_t)(uint32_t)fle_ld(prc + 8u, 4);
    b = (int32_t)(uint32_t)fle_ld(prc + 12u, 4);
    R[0] = 1;
    if (!target_live(api, &t)) return;
    r = dev_rect(dc, &t, l, t_, 0, 0);
    r.x1 = r.x0 + ((int64_t)rt - l);
    r.y1 = r.y0 + ((int64_t)b - t_);
    n = 0;
    EACH_CLIP(t, n += fle_r_rop_rect(&t.s, CL_, r, FLE_R_DSTINVERT_IDX, 0u));
    target_wrote(api, &t, r, n);
}

/* gdi32!RealizePalette: UINT f(HDC hdc rcx). Learn (RealizePalette): "maps palette entries from the current logical palette to the system palette"; the return value is "the number of entries in the logical palette
 * mapped to the system palette"; "If the device context is a memory DC, the color table for the bitmap selected into the DC is modified". The count and the modified table depend on the system palette and the
 * selected bitmap, which are not modelled. */
void fle_imp_RealizePalette(void)
{
    static const char api[] = "RealizePalette";

    fle_gdi_check_thread(api);
    fle_stop(api, "the number of entries mapped to the system palette, and the colour table of the selected bitmap that a memory DC modifies, are not given by the page (decision 0108)");
}

/* ---- decision 0111 (PROVISIONAL): DC attributes, the DC stack, pens, solid brushes, palette entries, icons and cursors, the named stops (docs/decisions/0111-gdi-bulk.md) ----
 * The initial value of every attribute below is the one of the Learn page "Display Device Context Defaults", read as the "normal default values" of the CreateCompatibleDC page. Every call
 * checks the thread like the older ones and leaves the last error alone. A screen DC (GetDC(NULL)) has no attribute state here: the calls stop by name, like SetTextColor does. */

static const char screen_dc_why[] = "hdc is a DC of GetDC(NULL): its attributes are not modelled (only memory DCs are)";

/* ---- decision 0121 (PROVISIONAL, PM's decision 1): the attributes of a screen DC (GetDC(NULL)) ----
 * A screen DC of GetDC(NULL) now has the attribute state of the memory DC (gdi_dc), with the same initial values: Learn (Display Device Context Defaults): "Upon first creating a display device
 * context, the system assigns default values for the attributes ... Background mode OPAQUE, Brush origin (0,0), Current pen position (0,0), Drawing mode R2_COPYPEN, Mapping mode MM_TEXT, Stretch mode
 * BLACKONWHITE, Viewport extent (1,1), Viewport origin (0,0), Window extent (1,1), Window origin (0,0)"; "Changes to the attribute values of a common, parent, or window device context are not permanent. When an
 * application releases these device contexts, the current selections, such as mapping mode and clipping region, are lost as the context is returned to the cache": the state belongs to ONE GetDC handle, a
 * new GetDC starts again from the defaults, and ReleaseDC discards it (fle_gdi_screen_released, called by fle_gdc.c through a weak reference). Only the attribute calls of this section use it (the 14
 * functions that stopped with screen_dc_why): setting and querying. Nothing is drawn: the pixel operations, SelectObject, SelectPalette, SetTextColor, SetBkColor and GetClipBox still stop for a screen DC
 * (the objects, the two colours and the visible area are not modelled for it: the page gives their defaults as "WHITE_BRUSH", "Text color setting from Control Panel (typically, black)" and so on). The
 * selected-object and colour fields of the state stay 0 and are never read for a screen DC. */
static gdi_dc **scr_dc;                     /* by the index of the handle (fle_gdc.h: FLE_GDC_HANDLE_BASE + 4 * (i + 1)); lazily made */
static uint64_t scr_cap;

static gdi_dc *screen_state(const char *api, uint64_t h)
{
    uint64_t i = (h - FLE_GDC_HANDLE_BASE) / 4u - 1u;

    if (i >= scr_cap) {
        uint64_t nc = scr_cap ? scr_cap * 2 : 16;
        gdi_dc **n;

        while (nc <= i) nc *= 2;
        n = (gdi_dc **)realloc(scr_dc, nc * sizeof *n);
        if (!n) fle_stop(api, "out of memory");
        for (uint64_t k = scr_cap; k < nc; k++) n[k] = NULL;
        scr_dc = n;
        scr_cap = nc;
    }
    if (!scr_dc[i]) {
        gdi_dc *dc = (gdi_dc *)calloc(1, sizeof *dc);

        if (!dc) fle_stop(api, "out of memory");
        dc->bk_mode = FLE_GDI_OPAQUE;
        dc->rop2 = FLE_GDI_R2_COPYPEN;
        dc->map_mode = FLE_GDI_MM_TEXT;
        dc->stretch = FLE_GDI_BLACKONWHITE;
        /* decision 0126: a window or screen DC is drawn on, so it holds the default objects (WHITE_BRUSH, BLACK_PEN, SYSTEM_FONT, DEFAULT_PALETTE: Display Device Context Defaults) and the two colours of a new DC */
        default_objects(api);
        dc->brush = dc_default[FLE_GDI_DEFAULT_BRUSH];
        dc->pen = dc_default[FLE_GDI_DEFAULT_PEN];
        dc->font = dc_default[FLE_GDI_DEFAULT_FONT];
        dc->pal = dc_default[FLE_GDI_DEFAULT_PALETTE];
        sel_add(dc->brush);
        sel_add(dc->pen);
        sel_add(dc->font);
        sel_add(dc->pal);
        dc->text = FLE_GDI_DC_TEXT_COLOR;
        dc->bk = FLE_GDI_DC_BK_COLOR;
        scr_dc[i] = dc;
    }
    return scr_dc[i];
}

void fle_gdi_screen_released(uint64_t h)
{
    uint64_t i = (h - FLE_GDC_HANDLE_BASE) / 4u - 1u;

    if (i < scr_cap && scr_dc[i]) {
        sel_drop(scr_dc[i]->brush);
        sel_drop(scr_dc[i]->pen);
        sel_drop(scr_dc[i]->font);
        sel_drop(scr_dc[i]->pal);
        free(scr_dc[i]->stack);
        free(scr_dc[i]);
        scr_dc[i] = NULL;
    }
}

/* The DC of an attribute call: a live screen DC (its own state) or a live memory DC; every other handle stops as before. */
static gdi_dc *need_attr_dc(const char *api, uint64_t h)
{
    if (fle_gdc_is_live(h)) return screen_state(api, h);
    return need_mem_dc(api, h, screen_dc_why);
}

/* Decision 0125 (PROVISIONAL), wall 52: an attribute call with hdc NULL FAILS: the return value is the page's failure value (zero for every call that uses attr_dc: the pages of SetWindowOrgEx, SetViewportOrgEx,
 * MoveToEx, GetCurrentPositionEx, SetBkMode, SetROP2, SetMapMode, SetStretchBltMode, GetStretchBltMode, LPtoDP, SetBrushOrgEx, GetBrushOrgEx, SaveDC and RestoreDC say "if the function fails, the return value is
 * zero") and nothing is written through the POINT pointer. The pages do not say that a NULL hdc is a failure and give no GetLastError for it: READING (a NULL DC cannot be the DC of anything), and the last error
 * ERROR_INVALID_HANDLE (6, System Error Codes: "The handle is invalid.") is a CHOICE WITHOUT A SOURCE (S-hdc-null). Why the guest passes NULL: the Delphi TBitmap image (f_6b9120) that is made without a DIB section
 * and without a DC (its pixels are a heap block) has the DC 0 at [image + 0x424], and the canvas code of f_820440 sets the window origin of that DC and does not read the result (measured by gdb and read from the
 * instructions, docs/decisions/0125-window-dc-and-walls.md). Every other handle that is not a live DC still stops as before. Returns 1 when `*out` is set, 0 when the call has failed (R[0] is set). */
#define FLE_GDI_ERROR_INVALID_HANDLE 6u

/* Decision 0129 (PROVISIONAL), wall 3 (S-hdc-deleted): an attribute call with the handle of a memory DC that DeleteDC has deleted FAILS in the same way (0, last error 6) instead of stopping. The guest (f_5c3db0, a TCanvas
 * whose Handle field still holds a DC that the bitmap canvas released: DeleteDC at f_6aff60) calls GetCurrentPositionEx on it and reads the POINT only. The pages say "if the function fails, the return value is zero" and
 * do not say what a deleted handle gives; the choice reads a deleted DC as no DC, like the NULL one. Withdrawn when a measurement on a real Windows gives another value. The drawing calls (drawn_dc) still stop. */
static int attr_dc(const char *api, uint64_t h, gdi_dc **out)
{
    if (h == 0 || dc_class(h, NULL) == DCK_DELETED) {
        fle_set_last_error(FLE_GDI_ERROR_INVALID_HANDLE);
        R[0] = 0;
        return 0;
    }
    *out = need_attr_dc(api, h);
    return 1;
}

static void write_point(uint64_t p, int32_t x, int32_t y)
{
    fle_st(p, 4, (uint32_t)x);
    fle_st(p + 4u, 4, (uint32_t)y);
}

/* gdi32!MoveToEx: BOOL f(HDC hdc rcx, int x edx, int y r8d, LPPOINT lppt r9). Learn (MoveToEx): "updates the current position to the specified point and optionally returns the previous
 * position"; x and y are logical units; lppt "receives the previous current position. If this parameter is a NULL pointer, the previous position is not returned"; the return value is nonzero on
 * success. The initial position is (0, 0) (Display Device Context Defaults: "Current pen position (0,0)"). The position is kept in logical units, as given. */
void fle_imp_MoveToEx(void)
{
    static const char api[] = "MoveToEx";
    int32_t x = (int32_t)(uint32_t)R[2], y = (int32_t)(uint32_t)R[8];
    uint64_t lppt = R[9];
    gdi_dc *dc;

    fle_gdi_check_thread(api);
    if (!attr_dc(api, R[1], &dc)) return;
    if (lppt) write_point(lppt, dc->pos[0], dc->pos[1]);
    dc->pos[0] = x;
    dc->pos[1] = y;
    R[0] = 1;
}

/* gdi32!GetCurrentPositionEx: BOOL f(HDC hdc rcx, LPPOINT lppt rdx). Learn: "retrieves the current position in logical coordinates"; nonzero on success. A NULL lppt has no result on the page: stop. */
void fle_imp_GetCurrentPositionEx(void)
{
    static const char api[] = "GetCurrentPositionEx";
    uint64_t lppt = R[2];
    gdi_dc *dc;

    fle_gdi_check_thread(api);
    if (!attr_dc(api, R[1], &dc)) return;
    if (lppt == 0) fle_stop(api, "lppt is NULL: the page does not define it (decision 0111)");
    write_point(lppt, dc->pos[0], dc->pos[1]);
    R[0] = 1;
}

/* gdi32!SetBkMode: int f(HDC hdc rcx, int mode edx). Learn (SetBkMode): mode is OPAQUE or TRANSPARENT; "the return value specifies the previous background mode"; zero on failure. The initial mode is
 * OPAQUE (the example on the page: "Reset the background mode to its default. SetBkMode(hdc, OPAQUE);"; Display Device Context Defaults). Another value stops (the page lists two). */
void fle_imp_SetBkMode(void)
{
    static const char api[] = "SetBkMode";
    uint32_t mode = (uint32_t)R[2];
    gdi_dc *dc;
    char why[120];

    fle_gdi_check_thread(api);
    if (!attr_dc(api, R[1], &dc)) return;
    if (mode != FLE_GDI_OPAQUE && mode != FLE_GDI_TRANSPARENT) {
        snprintf(why, sizeof why, "mode is %u, neither OPAQUE (2) nor TRANSPARENT (1): the page lists these two", (unsigned)mode);
        fle_stop(api, why);
    }
    R[0] = dc->bk_mode;
    dc->bk_mode = mode;
}

/* gdi32!SetROP2: int f(HDC hdc rcx, int rop2 edx). Learn (SetROP2): the mix mode is one of the 16 R2_ names (R2_BLACK ... R2_XORPEN); "the return value specifies the previous mix mode"; zero on failure.
 * The initial mode is R2_COPYPEN (Display Device Context Defaults: "Drawing mode R2_COPYPEN"). The numbers 1 to 16 are from the SDK header (UNOFFICIAL); another value stops. */
void fle_imp_SetROP2(void)
{
    static const char api[] = "SetROP2";
    uint32_t rop2 = (uint32_t)R[2];
    gdi_dc *dc;
    char why[120];

    fle_gdi_check_thread(api);
    if (!attr_dc(api, R[1], &dc)) return;
    if (rop2 < FLE_GDI_R2_BLACK || rop2 > FLE_GDI_R2_WHITE) {
        snprintf(why, sizeof why, "rop2 is %u, not one of the 16 mix modes (1 to 16)", (unsigned)rop2);
        fle_stop(api, why);
    }
    R[0] = dc->rop2;
    dc->rop2 = rop2;
}

/* gdi32!SetMapMode: int f(HDC hdc rcx, int iMode edx). Learn (SetMapMode): iMode is one of MM_ANISOTROPIC, MM_HIENGLISH, MM_HIMETRIC, MM_ISOTROPIC, MM_LOENGLISH, MM_LOMETRIC, MM_TEXT, MM_TWIPS (the numbers 1 to 8
 * are from the SDK header: UNOFFICIAL); "the return value identifies the previous mapping mode"; zero on failure. The initial mode is MM_TEXT (the page Default Transformations: "MM_TEXT (the default
 * page-space to device-space transformation)"). The mode is kept; the extents that the page says the system sets for the six fixed modes are NOT (they depend on the device size, which the pages give
 * no value for): every call that needs the transformation (LPtoDP, GetClipBox, the pixel operations) stops unless the mode is MM_TEXT. Another value stops. */
void fle_imp_SetMapMode(void)
{
    static const char api[] = "SetMapMode";
    uint32_t mode = (uint32_t)R[2];
    gdi_dc *dc;
    char why[120];

    fle_gdi_check_thread(api);
    if (!attr_dc(api, R[1], &dc)) return;
    if (mode < FLE_GDI_MM_TEXT || mode > FLE_GDI_MM_ANISOTROPIC) {
        snprintf(why, sizeof why, "iMode is %u, not one of the 8 mapping modes (1 to 8)", (unsigned)mode);
        fle_stop(api, why);
    }
    R[0] = dc->map_mode;
    dc->map_mode = mode;
}

/* gdi32!SetStretchBltMode / GetStretchBltMode: int f(HDC hdc rcx, int mode edx) / int f(HDC hdc rcx). Learn (SetStretchBltMode): mode is BLACKONWHITE, COLORONCOLOR, HALFTONE, STRETCH_ANDSCANS (same as
 * BLACKONWHITE), STRETCH_DELETESCANS (same as COLORONCOLOR), STRETCH_HALFTONE (same as HALFTONE), STRETCH_ORSCANS (same as WHITEONBLACK) or WHITEONBLACK; "the return value is the previous stretching
 * mode"; zero on failure. "Additional stretching modes might also be available depending on the capabilities of the device driver": a value outside 1 to 4 stops, it is not called invalid. Learn
 * (GetStretchBltMode): "the return value is the current stretching mode". The initial mode is BLACKONWHITE (Display Device Context Defaults: "Stretch mode BLACKONWHITE"). */
void fle_imp_SetStretchBltMode(void)
{
    static const char api[] = "SetStretchBltMode";
    uint32_t mode = (uint32_t)R[2];
    gdi_dc *dc;
    char why[160];

    fle_gdi_check_thread(api);
    if (!attr_dc(api, R[1], &dc)) return;
    if (mode < FLE_GDI_BLACKONWHITE || mode > FLE_GDI_HALFTONE) {
        snprintf(why, sizeof why, "mode is %u, not one of the 4 modes of the page (1 to 4): the page allows driver-specific modes and says nothing of them", (unsigned)mode);
        fle_stop(api, why);
    }
    R[0] = dc->stretch;
    dc->stretch = mode;
}

void fle_imp_GetStretchBltMode(void)
{
    static const char api[] = "GetStretchBltMode";
    gdi_dc *dc;

    fle_gdi_check_thread(api);
    if (!attr_dc(api, R[1], &dc)) return;
    R[0] = dc->stretch;
}

/* gdi32!SetWindowOrgEx / SetViewportOrgEx: BOOL f(HDC hdc rcx, int x edx, int y r8d, LPPOINT lppt r9); GetWindowOrgEx: BOOL f(HDC hdc rcx, LPPOINT lppt rdx). Learn (SetWindowOrgEx): "specifies which window point
 * maps to the viewport origin (0,0)"; lppt "receives the previous origin of the window, in logical units. If lpPoint is NULL, this parameter is not used"; nonzero on success. (SetViewportOrgEx): "specifies
 * which device point maps to the window origin (0,0)"; the previous origin is returned "in device coordinates". (GetWindowOrgEx): "retrieves the x-coordinates and y-coordinates of the window origin".
 * Both origins start at (0, 0) (Mapping Modes and Translations: "When no translation is in effect (this is the default)"; Display Device Context Defaults). They are independent of the mapping mode ("Origins
 * are specified in absolute units that the current mapping mode does not affect"). */
static void set_org(const char *api, int viewport)
{
    int32_t x = (int32_t)(uint32_t)R[2], y = (int32_t)(uint32_t)R[8];
    uint64_t lppt = R[9];
    gdi_dc *dc;
    int32_t *at;

    fle_gdi_check_thread(api);
    if (!attr_dc(api, R[1], &dc)) return;
    at = viewport ? dc->vorg : dc->worg;
    if (lppt) write_point(lppt, at[0], at[1]);
    at[0] = x;
    at[1] = y;
    R[0] = 1;
}

void fle_imp_SetWindowOrgEx(void) { set_org("SetWindowOrgEx", 0); }
void fle_imp_SetViewportOrgEx(void) { set_org("SetViewportOrgEx", 1); }

void fle_imp_GetWindowOrgEx(void)
{
    static const char api[] = "GetWindowOrgEx";
    uint64_t lppt = R[2];
    gdi_dc *dc;

    fle_gdi_check_thread(api);
    if (!attr_dc(api, R[1], &dc)) return;
    if (lppt == 0) fle_stop(api, "lppoint is NULL: the page does not define it (decision 0111)");
    write_point(lppt, dc->worg[0], dc->worg[1]);
    R[0] = 1;
}

/* gdi32!LPtoDP: BOOL f(HDC hdc rcx, LPPOINT lppt rdx, int c r8d). Learn (LPtoDP): "converts logical coordinates into device coordinates. The conversion depends on the mapping mode of the device context, the
 * settings of the origins and extents for the window and viewport, and the world transformation"; it fails "if the logical coordinates exceed 32 bits, or if the converted device coordinates exceed 27
 * bits" (then "the results for all the points are undefined"). Answered: MM_TEXT, where the extents are 1 and 1, so the page's formula Dx = (Lx - WOx) * VEx / WEx + VOx is Lx - WOx + VOx. All c points are
 * converted in place; c must be positive and every converted coordinate must fit 27 bits plus sign (-2^27 .. 2^27 - 1: a READING of "exceed 27 bits"), otherwise the call stops (it does not return 0:
 * the page calls the results undefined). Another mapping mode stops (the extents of the fixed modes are not on the pages), except MM_LOMETRIC and MM_HIMETRIC of a screen DC (decision 0121, below).
 * A NULL lppt stops. */
void fle_imp_LPtoDP(void)
{
    static const char api[] = "LPtoDP";
    uint64_t p = R[2];
    int32_t c = (int32_t)(uint32_t)R[8];
    gdi_dc *dc;
    int64_t lim = INT64_C(1) << 27;
    char why[160];
    int screen;
    int64_t we[2] = { 1, 1 }, ve[2] = { 1, 1 };         /* the extents of the page's formula (MM_TEXT: 1 and 1) */

    fle_gdi_check_thread(api);
    screen = fle_gdc_is_live(R[1]);
    if (!attr_dc(api, R[1], &dc)) return;
    if (dc->map_mode != FLE_GDI_MM_TEXT) {
        if (screen && (dc->map_mode == FLE_GDI_MM_LOMETRIC || dc->map_mode == FLE_GDI_MM_HIMETRIC)) {
            /* decision 0121 (PROVISIONAL): the extents of the two metric modes of a screen DC, from the physical size of the virtual display (fle_gdc.h): one logical unit is 0.1 mm
             * (MM_LOMETRIC, 2) or 0.01 mm (MM_HIMETRIC, 3) of the physical screen, so the window extent is HORZSIZE * 10 (or * 100) units, likewise VERTSIZE, for the viewport extent
             * HORZRES pixels (VERTRES), and the y axis points up (the viewport extent is negative). READING: the page "Mapping Modes and Translations" says what a unit is and that
             * "the extents are set by the system when SetMapMode is called", and gives no number; that the extents come from HORZSIZE/HORZRES (and not from 96 / 254 directly, which
             * differs for y: 1080 / 2860 against 96 / 254) is a choice without a source (unofficial corroboration: Wine's win32u set_map_mode does the same with the HORZSIZE and
             * HORZRES of the device). The guest asks MM_HIMETRIC (edx = 3 at 0x6f642d; the numbers are those of [MS-WMF] "MapMode Enumeration"); MM_LOMETRIC has the same rule with the
             * factor 10 and is answered for the same reason (the PM's decision 1 names it) without a guest call that needs it. The other fixed modes of the page stop by name. */
            int64_t k = dc->map_mode == FLE_GDI_MM_LOMETRIC ? 10 : 100;

            we[0] = (int64_t)FLE_GDC_HORZSIZE * k;
            we[1] = (int64_t)FLE_GDC_VERTSIZE * k;
            ve[0] = FLE_GDC_HORZRES;
            ve[1] = -(int64_t)FLE_GDC_VERTRES;
        } else {
            snprintf(why, sizeof why, "the mapping mode is %u, not MM_TEXT%s: the extents of the other modes are not on the pages", (unsigned)dc->map_mode,
                     screen ? " (a screen DC answers MM_LOMETRIC and MM_HIMETRIC too)" : "");
            fle_stop(api, why);
        }
    }
    if (p == 0) fle_stop(api, "lppt is NULL: the page does not define it (decision 0111)");
    if (c <= 0) fle_stop(api, "c is not positive: the page does not define it (decision 0111)");
    for (int pass = 0; pass < 2; pass++) {
        for (int32_t i = 0; i < c; i++) {
            int64_t d[2];

            for (int a = 0; a < 2; a++) {
                int64_t l = (int64_t)(int32_t)(uint32_t)fle_ld(p + 8u * (uint64_t)i + 4u * (uint64_t)a, 4);
                int64_t n = (l - (a ? dc->worg[1] : dc->worg[0])) * ve[a], q = n / we[a], r = n % we[a];

                /* rounding of the quotient: to the nearest, a half away from zero (a CHOICE WITHOUT A SOURCE, decision 0121: the page says "complex floating-point arithmetic" and gives no rule;
                 * MulDiv's page states this rule for its own result). For MM_TEXT the extents are 1 and the quotient is exact. */
                if (2 * (r < 0 ? -r : r) >= we[a]) q += n < 0 ? -1 : 1;
                d[a] = q + (a ? dc->vorg[1] : dc->vorg[0]);
            }
            if (d[0] < -lim || d[0] >= lim || d[1] < -lim || d[1] >= lim) fle_stop(api, "a converted coordinate exceeds 27 bits: the page calls the results undefined");
            if (pass) write_point(p + 8u * (uint64_t)i, (int32_t)d[0], (int32_t)d[1]);
        }
    }
    R[0] = 1;
}

/* gdi32!SetBrushOrgEx: BOOL f(HDC hdc rcx, int x edx, int y r8d, LPPOINT lppt r9); GetBrushOrgEx: BOOL f(HDC hdc rcx, LPPOINT lppt rdx). Learn (SetBrushOrgEx): "sets the brush origin that GDI assigns to the next
 * brush an application selects into the specified device context"; "If this value is greater than the brush width, its value is reduced using the modulus operator"; lppt "receives the previous brush
 * origin ... can be NULL"; "The default brush origin coordinates are (0,0)". (GetBrushOrgEx): "retrieves the current brush origin"; "The brush origin is a set of coordinates with values between 0 and
 * 7". Answered: x and y from 0 to 7, stored as given; a value outside stops (the reduction needs the brush size, which the pages do not give for every brush). */
void fle_imp_SetBrushOrgEx(void)
{
    static const char api[] = "SetBrushOrgEx";
    int32_t x = (int32_t)(uint32_t)R[2], y = (int32_t)(uint32_t)R[8];
    uint64_t lppt = R[9];
    gdi_dc *dc;

    fle_gdi_check_thread(api);
    if (!attr_dc(api, R[1], &dc)) return;
    if (x < 0 || x > FLE_GDI_BRUSHORG_MAX || y < 0 || y > FLE_GDI_BRUSHORG_MAX)
        fle_stop(api, "x or y is outside 0 to 7: the page reduces it by the brush size and gives no size for every brush (decision 0111)");
    if (lppt) write_point(lppt, dc->borg[0], dc->borg[1]);
    dc->borg[0] = x;
    dc->borg[1] = y;
    R[0] = 1;
}

void fle_imp_GetBrushOrgEx(void)
{
    static const char api[] = "GetBrushOrgEx";
    uint64_t lppt = R[2];
    gdi_dc *dc;

    fle_gdi_check_thread(api);
    if (!attr_dc(api, R[1], &dc)) return;
    if (lppt == 0) fle_stop(api, "lppt is NULL: the page does not define it (decision 0111)");
    write_point(lppt, dc->borg[0], dc->borg[1]);
    R[0] = 1;
}

/* gdi32!SaveDC: int f(HDC hdc rcx); RestoreDC: BOOL f(HDC hdc rcx, int nSavedDC edx). Learn (SaveDC): "saves the current state of the specified device context (DC) by copying data describing selected objects and
 * graphic modes (such as the bitmap, brush, palette, font, pen, region, drawing mode, and mapping mode) to a context stack"; "the return value identifies the saved state", zero on failure. Learn (RestoreDC):
 * "restores a device context (DC) to the specified state ... by popping state information off a stack"; "If this parameter is positive, nSavedDC represents a specific instance of the state to be restored.
 * If this parameter is negative, nSavedDC represents an instance relative to the current state. For example, -1 restores the most recently saved state"; "Each DC maintains a stack of saved states ... After a
 * state is restored, the saved state is destroyed and cannot be reused. Furthermore, any states saved after the restored state was created are also destroyed"; nonzero on success, zero on failure.
 * Answered: SaveDC pushes a state and returns its number, 1 for the first (a CHOICE: the page says "identifies"); RestoreDC with n > 0 restores state n, with n < 0 the state |n| below the top, and pops it and
 * the ones above; a number that names no saved state returns 0 (the page's failure value); 0 stops. What is saved and restored: the five selected objects and the graphic modes (background, drawing, mapping, stretching)
 * and the window and viewport origins (a READING: they are the translation of the mapping mode). The pages do not say whether the text colour, the background colour, the current position and the brush origin are
 * part of a state: if one of them differs from the saved state at RestoreDC, the call stops. A saved object that was deleted since, or a saved bitmap that another DC holds now, stops (the pages do not say). */
static void dc_snapshot(const gdi_dc *dc, gdi_dcsave *s)
{
    s->bmp = dc->bmp;
    s->brush = dc->brush;
    s->pen = dc->pen;
    s->font = dc->font;
    s->pal = dc->pal;
    s->bk_mode = dc->bk_mode;
    s->rop2 = dc->rop2;
    s->map_mode = dc->map_mode;
    s->stretch = dc->stretch;
    s->text = dc->text;
    s->bk = dc->bk;
    clip_copy(&s->clip, &dc->clip);
    for (int i = 0; i < 2; i++) {
        s->pos[i] = dc->pos[i];
        s->worg[i] = dc->worg[i];
        s->vorg[i] = dc->vorg[i];
        s->borg[i] = dc->borg[i];
    }
}

void fle_imp_SaveDC(void)
{
    static const char api[] = "SaveDC";
    gdi_dc *dc;

    fle_gdi_check_thread(api);
    if (!attr_dc(api, R[1], &dc)) return;
    if (dc->depth == dc->cap) {
        uint32_t nc = dc->cap ? dc->cap * 2u : 4u;
        gdi_dcsave *n;

        if (nc > 0x100000u) fle_stop(api, "the DC stack is deeper than 1,048,576 states");
        n = (gdi_dcsave *)realloc(dc->stack, (size_t)nc * sizeof *n);
        if (!n) fle_stop(api, "out of memory");
        memset(n + dc->cap, 0, (size_t)(nc - dc->cap) * sizeof *n);   /* decision 0229: the clip of a state is a heap array: the new states start empty */
        dc->stack = n;
        dc->cap = nc;
    }
    dc_snapshot(dc, &dc->stack[dc->depth]);
    dc->depth++;
    R[0] = dc->depth;
}

static void restore_object(const char *api, uint64_t *at, uint64_t want, int is_bitmap)
{
    gdi_obj *o;

    if (*at == want) return;
    o = live_obj(want);
    if (!o) fle_stop(api, "an object of the saved state was deleted since: the pages do not say what RestoreDC does then");
    if (is_bitmap && o->sel) fle_stop(api, "the bitmap of the saved state is selected into another DC now: the pages do not say what RestoreDC does then");
    sel_drop(*at);
    sel_add(want);
    *at = want;
}

void fle_imp_RestoreDC(void)
{
    static const char api[] = "RestoreDC";
    int32_t n = (int32_t)(uint32_t)R[2];
    gdi_dc *dc;
    const gdi_dcsave *s;
    int64_t idx;

    fle_gdi_check_thread(api);
    if (!attr_dc(api, R[1], &dc)) return;
    if (n == 0) fle_stop(api, "nSavedDC is 0: the page defines positive and negative numbers only");
    idx = n > 0 ? (int64_t)n - 1 : (int64_t)dc->depth + n;
    if (idx < 0 || idx >= (int64_t)dc->depth) {
        R[0] = 0;
        return;
    }
    s = &dc->stack[idx];
    if (dc->text != s->text || dc->bk != s->bk) fle_stop(api, "the text colour or the background colour differs from the saved state: the pages do not say whether RestoreDC restores colours");
    if (dc->pos[0] != s->pos[0] || dc->pos[1] != s->pos[1]) fle_stop(api, "the current position differs from the saved state: the pages do not say whether RestoreDC restores it");
    if (dc->borg[0] != s->borg[0] || dc->borg[1] != s->borg[1]) fle_stop(api, "the brush origin differs from the saved state: the pages do not say whether RestoreDC restores it");
    restore_object(api, &dc->bmp, s->bmp, 1);
    restore_object(api, &dc->brush, s->brush, 0);
    restore_object(api, &dc->pen, s->pen, 0);
    restore_object(api, &dc->font, s->font, 0);
    restore_object(api, &dc->pal, s->pal, 0);
    dc->bk_mode = s->bk_mode;
    dc->rop2 = s->rop2;
    dc->map_mode = s->map_mode;
    dc->stretch = s->stretch;
    clip_copy(&dc->clip, &s->clip);
    for (int i = 0; i < 2; i++) {
        dc->worg[i] = s->worg[i];
        dc->vorg[i] = s->vorg[i];
    }
    for (uint32_t k = (uint32_t)idx; k < dc->depth; k++) clip_free(&dc->stack[k].clip);
    dc->depth = (uint32_t)idx;
    R[0] = 1;
}

/* gdi32!GdiFlush: BOOL f(void). Learn (GdiFlush): "flushes the calling thread's current batch"; "If all functions in the current batch succeed, the return value is nonzero". This runtime does not batch: every
 * call has completed or stopped the run before it returns, so the batch holds no failed call: 1. */
void fle_imp_GdiFlush(void)
{
    static const char api[] = "GdiFlush";

    fle_gdi_check_thread(api);
    R[0] = 1;
}

/* gdi32!GetClipBox: int f(HDC hdc rcx, LPRECT lprect rdx). Learn (GetClipBox): "retrieves the dimensions of the tightest bounding rectangle that can be drawn around the current visible area on the device. The visible
 * area is defined by the current clipping region or clip path, as well as any overlapping windows"; the rectangle is in logical units ("GetClipBox returns logical coordinates based on the given device context"); the
 * return value is NULLREGION (1, "Region is empty"), SIMPLEREGION (2, "Region is a single rectangle"), COMPLEXREGION (3, "Region is more than one rectangle") or ERROR (0). Answered: a memory DC in MM_TEXT. The visible
 * area is the bitmap selected into the DC (a READING of the memory DC page: "an array large enough to store color information for the specified rectangle of pixels") cut by the DC's application-defined clipping
 * region when it has one (decisions 0123 and 0126: SelectClipRgn, IntersectClipRect ...; decision 0148: before it, the box was the whole bitmap whatever the region was, so a guest that draws "into its clip box" drew
 * over the whole surface); no window overlaps a memory DC. The device box (the tightest rectangle around the pieces of the region inside the bitmap) is mapped back to logical units (Lx = Dx + WOx - VOx). The type is
 * NULLREGION when no piece is left (the rectangle is then all zero: S-clipbox-null-rect, the page does not say what lprect holds), SIMPLEREGION for one piece (or no region), COMPLEXREGION for more. A NULL lprect
 * stops. Another mapping mode and a screen DC stop. */
void fle_imp_GetClipBox(void)
{
    static const char api[] = "GetClipBox";
    uint64_t p = R[2];
    gdi_dc *dc;
    gdi_obj *b;
    int64_t l, t, r, bt, wd, ht;
    uint32_t type = FLE_GDI_SIMPLEREGION;
    char why[120];

    fle_gdi_check_thread(api);
    dc = need_mem_dc(api, R[1], "hdc is a DC of GetDC(NULL): its visible area (the screen minus overlapping windows) is not modelled (only memory DCs are)");
    if (dc->map_mode != FLE_GDI_MM_TEXT) {
        snprintf(why, sizeof why, "the mapping mode is %u, not MM_TEXT: the extents of the other modes are not on the pages", (unsigned)dc->map_mode);
        fle_stop(api, why);
    }
    if (p == 0) fle_stop(api, "lprect is NULL: the page does not define it (decision 0111)");
    b = live_obj(dc->bmp);
    if (!b || b->kind != FLE_GDI_KIND_BITMAP) fle_stop(api, "the bitmap selected into the DC is gone");
    wd = (int64_t)b->bm[0];
    ht = (int64_t)b->bm[1];
    l = 0;
    t = 0;
    r = wd;
    bt = ht;
    if (dc->clip.has) {
        uint32_t used = 0;

        l = INT64_MAX;
        t = INT64_MAX;
        r = INT64_MIN;
        bt = INT64_MIN;
        for (uint32_t i = 0; i < dc->clip.n; i++) {
            int64_t pl = dc->clip.r[i].l < 0 ? 0 : dc->clip.r[i].l, pt = dc->clip.r[i].t < 0 ? 0 : dc->clip.r[i].t;
            int64_t pr = dc->clip.r[i].r > wd ? wd : dc->clip.r[i].r, pb = dc->clip.r[i].b > ht ? ht : dc->clip.r[i].b;

            if (pr <= pl || pb <= pt) continue;
            used++;
            if (pl < l) l = pl;
            if (pt < t) t = pt;
            if (pr > r) r = pr;
            if (pb > bt) bt = pb;
        }
        type = used == 0 ? FLE_GDI_NULLREGION : used == 1 ? FLE_GDI_SIMPLEREGION : 3u;
        if (used == 0) l = t = r = bt = 0;
    }
    if (type != FLE_GDI_NULLREGION) {
        l += (int64_t)dc->worg[0] - dc->vorg[0];
        t += (int64_t)dc->worg[1] - dc->vorg[1];
        r += (int64_t)dc->worg[0] - dc->vorg[0];
        bt += (int64_t)dc->worg[1] - dc->vorg[1];
    }
    if (l < INT32_MIN || t < INT32_MIN || r > INT32_MAX || bt > INT32_MAX) fle_stop(api, "the logical rectangle does not fit 32 bits");
    fle_st(p, 4, (uint32_t)(int32_t)l);
    fle_st(p + 4u, 4, (uint32_t)(int32_t)t);
    fle_st(p + 8u, 4, (uint32_t)(int32_t)r);
    fle_st(p + 12u, 4, (uint32_t)(int32_t)bt);
    R[0] = type;
}

/* gdi32!UnrealizeObject: BOOL f(HGDIOBJ h rcx). Learn (UnrealizeObject): "resets the origin of a brush or resets a logical palette"; "If hgdiobj is a brush, UnrealizeObject does nothing, and the function returns TRUE";
 * for a palette it "directs the system to realize the palette as though it had not previously been realized" (this runtime never realizes a palette: RealizePalette stops, so there is nothing to reset); "should not be
 * used with stock objects"; nonzero on success. Answered: a live brush made by CreateBrushIndirect or CreateSolidBrush, and a live palette made by CreatePalette or CreateHalftonePalette: 1. Stock objects, the
 * default objects of a memory DC and every other handle stop (the page gives no result for them). */
void fle_imp_UnrealizeObject(void)
{
    static const char api[] = "UnrealizeObject";
    gdi_obj *o;

    fle_gdi_check_thread(api);
    o = live_obj(R[1]);
    if (!o) fle_stop(api, "h is not a live object of this runtime (NULL, deleted, or a number never issued): the page defines no failure for it");
    if (o->kind != FLE_GDI_KIND_BRUSH && o->kind != FLE_GDI_KIND_PALETTE)
        fle_stop(api, "h is not a brush or a palette made by the guest: the page says not to use a stock object and defines brushes and palettes only");
    R[0] = 1;
}

/* gdi32!CreateSolidBrush: HBRUSH f(COLORREF color ecx). Learn (CreateSolidBrush): "creates a logical brush that has the specified solid color"; the return value "identifies a logical brush", NULL on failure; "When you
 * no longer need the HBRUSH object, call the DeleteObject function". Answered: a colour whose high byte is 0 (0x00bbggrr; the palette forms need a palette): a new brush of the kind of CreateBrushIndirect, BS_SOLID. */
void fle_imp_CreateSolidBrush(void)
{
    static const char api[] = "CreateSolidBrush";
    uint32_t color = (uint32_t)R[1];
    uint64_t slot;
    gdi_obj *o;

    fle_gdi_check_thread(api);
    if (!color_plain(color, &color)) fle_stop(api, "color has a high-order byte other than 0 and 2 (PALETTEINDEX needs a palette): the page does not describe it");
    slot = new_slot(api);
    o = &objs[slot];
    o->u[0] = FLE_GDI_BS_SOLID;
    o->u[1] = color;
    issue(o, FLE_GDI_KIND_BRUSH);
    brush_live++;
    R[0] = slot_handle(slot);
}

/* gdi32!CreatePenIndirect: HPEN f(const LOGPEN *plpen rcx). Learn (CreatePenIndirect): "creates a logical cosmetic pen that has the style, width, and color specified in a structure"; the return value is a handle,
 * NULL on failure; "When you no longer need the pen, call the DeleteObject function". Learn (LOGPEN): { UINT lopnStyle; POINT lopnWidth; COLORREF lopnColor } = 16 bytes; the styles are PS_SOLID, PS_DASH, PS_DOT,
 * PS_DASHDOT, PS_DASHDOTDOT, PS_NULL, PS_INSIDEFRAME (0 to 6: SDK header, UNOFFICIAL); "If the x member is NULL, then the pen is one pixel wide on raster devices"; "The y member ... isn't used". Answered: a
 * style from 0 to 6, a width x that is not negative, a colour whose high byte is 0: a new pen (kind FLE_GDI_KIND_PEN) that SelectObject selects and DeleteObject deletes; nothing is drawn with it (LineTo and the
 * other drawing calls stop by name). Another style (PS_USERSTYLE, PS_ALTERNATE, the geometric pens of ExtCreatePen), a negative width, a palette colour and NULL stop. */
void fle_imp_CreatePenIndirect(void)
{
    static const char api[] = "CreatePenIndirect";
    uint64_t p = R[1], slot;
    uint32_t style, color;
    int32_t width;
    gdi_obj *o;
    char why[140];

    fle_gdi_check_thread(api);
    if (p == 0) fle_stop(api, "plpen is NULL: the page does not define it");
    style = (uint32_t)fle_ld(p, 4);
    width = (int32_t)(uint32_t)fle_ld(p + 4u, 4);
    color = (uint32_t)fle_ld(p + 12u, 4);
    if (style > FLE_GDI_PS_INSIDEFRAME) {
        snprintf(why, sizeof why, "lopnStyle is %u: only PS_SOLID to PS_INSIDEFRAME (0 to 6) are answered", (unsigned)style);
        fle_stop(api, why);
    }
    if (width < 0) fle_stop(api, "lopnWidth.x is negative: the page does not define it");
    if (!color_plain(color, &color)) fle_stop(api, "lopnColor has a high-order byte other than 0 and 2 (PALETTEINDEX needs a palette): the page does not describe it");
    slot = new_slot(api);
    o = &objs[slot];
    o->u[0] = style;
    o->u[1] = (uint32_t)width;
    o->u[2] = color;
    issue(o, FLE_GDI_KIND_PEN);
    pen_live++;
    R[0] = slot_handle(slot);
}

/* gdi32!GetPaletteEntries: UINT f(HPALETTE hpal rcx, UINT iStart edx, UINT cEntries r8d, LPPALETTEENTRY pPalEntries r9). Learn (GetPaletteEntries): "retrieves a specified range of palette entries from the given logical
 * palette"; the return value is "the number of entries retrieved from the logical palette", zero on failure; "If the nEntries parameter specifies more entries than exist in the palette, the remaining members of the
 * PALETTEENTRY structure are not altered". Learn (PALETTEENTRY): four bytes peRed, peGreen, peBlue, peFlags. Answered: a live palette made by CreatePalette (its 16 entries are kept) and a non-NULL pPalEntries:
 * min(cEntries, 16 - iStart) entries from iStart are copied (4 bytes each) and that number returned (0 when iStart is 16 or more). The page's sentence about a NULL handle or pointer is garbled (it returns the number of
 * entries in the palette for one of them): a NULL hpal or pPalEntries stops. A halftone palette (its entries are an implementation detail), the default palette of a memory DC and any other handle stop. */
void fle_imp_GetPaletteEntries(void)
{
    static const char api[] = "GetPaletteEntries";
    uint64_t h = R[1], out = R[9];
    uint32_t start = (uint32_t)R[2], count = (uint32_t)R[8], n = 0;
    gdi_obj *o;

    fle_gdi_check_thread(api);
    if (h == 0) fle_stop(api, "hpal is NULL: the page's sentence for it is garbled and defines no result (decision 0111)");
    if (out == 0) fle_stop(api, "pPalEntries is NULL: the page's sentence for it is garbled and defines no result (decision 0111)");
    o = live_obj(h);
    if (!o || o->kind != FLE_GDI_KIND_PALETTE) fle_stop(api, "hpal is not a live palette made by CreatePalette: the default palette of a memory DC and the other handles have no entries here");
    if (o->flags & GF_OPAQUE_PAL) fle_stop(api, "hpal is a halftone palette: the page says its entries are an implementation detail");
    if (start < FLE_GDI_PALETTE_ENTRIES) {
        n = FLE_GDI_PALETTE_ENTRIES - start;
        if (count < n) n = count;
    }
    for (uint32_t i = 0; i < n; i++) fle_st(out + 4u * i, 4, o->pal[start + i]);
    R[0] = n;
}

/* fle_cursor.c is linked in the full runtime; a build without it has no standard cursor, so SetCursor stops on one (a weak reference, like fle_call_guest above). */
extern uint32_t fle_cursor_image_idc_of(uint64_t h) __attribute__((weak));
/* fle_cursor.c (decision 0113 wall 8): the shared cursors of LoadCursorW, LoadImageW(NULL, ...) and LoadImageW(hInst, id, ...) of the lifted DLL and the real modules; weak like the one above. */
extern int fle_cursor_is_shared_handle(uint64_t h) __attribute__((weak));

/* user32!DestroyIcon / DestroyCursor: BOOL f(HICON hIcon rcx). Learn (DestroyIcon): "destroys an icon and frees any memory the icon occupied"; "It is only necessary to call DestroyIcon for icons and cursors created with
 * ... CreateIconIndirect"; "Do not use this function to destroy a shared icon"; nonzero on success, zero on failure. Learn (DestroyCursor): "destroys a nonshared cursor. Do not use this function to destroy a shared
 * cursor"; "The cursor must not be in use". Answered: a live icon or cursor made by CreateIconIndirect that is not the current cursor of SetCursor: it is deleted and 1 is returned (its handle stays invalid and is
 * never reused); DestroyCursor takes a cursor (fIcon FALSE) only, DestroyIcon either. NULL and an icon or cursor that is already destroyed are not valid handles: 0. A shared handle (LoadIconW, LoadCursorW,
 * LoadImageW), the current cursor, an icon given to DestroyCursor and every other handle stop (the pages give no result for them).
 * Decision 0113 wall 8 (PROVISIONAL; decision 0111 stopped on it): DestroyCursor of a standard cursor of the virtual cursor table (fle_cursor.h: LoadCursorW, LoadImageW) or of a cursor that
 * LoadImageW(LR_SHARED) loaded from a module (fle_cursor_is_shared_handle: the lifted DLL or a real module; the guest's Delphi TScreen.DeleteCursor passed 0x7FDA5008 = id 32763 of ILTools) -- the shared
 * cursors -- returns 1 and does nothing: the page says not to use the function on a shared cursor and gives no result, the DestroyIcon page says a shared icon "is valid as long as the module from
 * which it was loaded remains in memory", so the cursor stays valid; the return value 1 is a choice without a source (the guest, a Delphi TScreen.DeleteCursor, ignores it). DestroyIcon of
 * such a cursor, and every other form above, still stop. */
static void destroy_icon_or_cursor(const char *api, int cursor_only)
{
    uint64_t h = R[1];
    int64_t i;
    gdi_obj *o;

    fle_gdi_check_thread(api);
    if (h == 0) {
        R[0] = 0;
        return;
    }
    i = slot_of(h);
    if (i < 0 && cursor_only && fle_cursor_is_shared_handle != NULL && fle_cursor_is_shared_handle(h)) {
        R[0] = 1;
        return;
    }
    if (i < 0) fle_stop(api, "hIcon was not issued by this runtime's GDI table (a shared cursor, a window, another kind of handle or a number never issued): the page defines no result for it");
    o = &objs[i];
    if (o->kind == FLE_GDI_KIND_ICONCUR && o->state == ST_DELETED) {
        R[0] = 0;
        return;
    }
    if (o->state == ST_DELETED || o->kind == FLE_GDI_KIND_NONE) fle_stop(api, "hIcon is a deleted object of another kind: the page defines no result for it");
    if (o->kind != FLE_GDI_KIND_ICONCUR) fle_stop(api, "hIcon is not an icon or cursor made by CreateIconIndirect: the page says not to destroy a shared icon or cursor and gives no result for one");
    if (cursor_only && o->u[0]) fle_stop(api, "hCursor is an icon (fIcon TRUE): DestroyCursor is defined for cursors, DestroyIcon for icons (decision 0111)");
    if (h == cur_cursor) fle_stop(api, "the handle is the current cursor of SetCursor: the page says the cursor must not be in use and gives no result for one that is");
    o->state = ST_DELETED;                  /* the kind stays (like a deleted DC): a second call is told from a handle that never was an icon */
    objs_live--;
    slot_release((uint64_t)i);
    R[0] = 1;
}

void fle_imp_DestroyIcon(void) { destroy_icon_or_cursor("DestroyIcon", 0); }
void fle_imp_DestroyCursor(void) { destroy_icon_or_cursor("DestroyCursor", 1); }

/* user32!GetIconInfo: BOOL f(HICON hIcon rcx, PICONINFO piconinfo rdx). Learn (GetIconInfo): "retrieves information about the specified icon or cursor"; "GetIconInfo creates bitmaps for the hbmMask and hbmColor or
 * members of ICONINFO. The calling application must manage these bitmaps and delete them with DeleteObject call"; nonzero on success. Learn (ICONINFO): fIcon TRUE for an icon, FALSE for a cursor; "If this structure
 * defines an icon, the hot spot is always in the center of the icon". Answered: a live cursor made by CreateIconIndirect (fIcon FALSE): fIcon, the hot spot it was given, and two new bitmaps of this table, like
 * the ones CreateBitmap and CreateCompatibleBitmap make (no pixels): hbmMask a monochrome bitmap of the size of the mask given at creation, hbmColor (only if a colour bitmap was given) a colour bitmap of the
 * virtual display's format and the same size (the page gives neither format: a READING). An icon made by CreateIconIndirect (fIcon TRUE: the hot spot "in the center" is not given as a number), a shared handle, NULL and a NULL piconinfo stop.
 * Decision 0118 wall 8 (PROVISIONAL): the shared icon of LoadIconW(hInstance, name) (GF_MODICON) is answered: fIcon TRUE, the hot spot (w / 2, h / 2) taken from the page's "always in the center" (a READING; the
 * page gives no number and says the member is ignored for an icon), hbmMask a monochrome bitmap and, for a colour icon, hbmColor a colour bitmap of the virtual display's format, both the size of the chosen entry (the ICONINFO page:
 * "the hbmMask and hbmColor bitmaps are the same size, each of which is the size of the icon"); for a monochrome entry hbmMask is twice the height and hbmColor NULL. New bitmaps every call, to be deleted by the guest. */
void fle_imp_GetIconInfo(void)
{
    static const char api[] = "GetIconInfo";
    uint64_t h = R[1], p = R[2], sm, sc = 0;
    gdi_obj *o, *m, *c;
    uint32_t w, ht;
    int has_color;

    fle_gdi_check_thread(api);
    if (p == 0) fle_stop(api, "piconinfo is NULL: the page does not define it");
    o = live_obj(h);
    if (!o || (o->kind != FLE_GDI_KIND_ICONCUR && !(o->kind == FLE_GDI_KIND_ICON && (o->flags & GF_MODICON)))) fle_stop(api, "hIcon is not an icon or cursor made by CreateIconIndirect (a shared handle, NULL, a deleted object or another kind): the page defines no result for it");
    if (o->u[0] && o->kind == FLE_GDI_KIND_ICONCUR) fle_stop(api, "hIcon is an icon (fIcon TRUE): its hot spot is 'always in the center' on the page, with no number (decision 0111)");
    w = o->u[3];
    ht = o->u[4];
    has_color = (int)o->u[5];
    sm = new_slot(api);
    m = &objs[sm];
    m->bm[0] = w;
    m->bm[1] = ht;
    m->bm[2] = 1;
    m->bm[3] = 1;
    m->fmt = FLE_GDI_BMP_MONO;
    issue(m, FLE_GDI_KIND_BITMAP);
    bmp_live++;
    if (has_color) {
        sc = new_slot(api);
        c = &objs[sc];
        c->bm[0] = w;
        c->bm[1] = ht;
        c->bm[2] = FLE_GDC_PLANES;
        c->bm[3] = FLE_GDC_BITSPIXEL;
        c->fmt = FLE_GDI_BMP_COLOR;
        issue(c, FLE_GDI_KIND_BITMAP);
        bmp_live++;
    }
    o = live_obj(h);
    fle_st(p, 4, o->u[0]);
    fle_st(p + 4u, 4, o->u[1]);
    fle_st(p + 8u, 4, o->u[2]);
    fle_st(p + 12u, 4, 0);
    fle_st(p + 16u, 8, slot_handle(sm));
    fle_st(p + 24u, 8, has_color ? slot_handle(sc) : 0);
    R[0] = 1;
}

/* user32!SetCursor: HCURSOR f(HCURSOR hCursor rcx); GetCursor: HCURSOR f(void); ShowCursor: int f(BOOL bShow ecx). Learn (SetCursor): "The cursor must have been created by either the CreateCursor or the
 * CreateIconIndirect function, or loaded by either the LoadCursor or the LoadImage function. If this parameter is NULL, the cursor is removed from the screen"; "The return value is the handle to the previous cursor,
 * if there was one. If there was no previous cursor, the return value is NULL". Learn (GetCursor): "the handle to the current cursor. If there is no cursor, the return value is NULL". Learn (ShowCursor): "If bShow is
 * TRUE, the display count is incremented by one. If bShow is FALSE, the display count is decremented by one"; "the return value specifies the new display counter"; "If a mouse is installed, the initial display count is
 * 0" (GetSystemMetrics(SM_MOUSEPRESENT) is 1 here: decision 0057). Answered: SetCursor with NULL, a standard cursor of LoadCursorW or LoadImageW (fle_cursor.h) or a live icon or cursor of CreateIconIndirect: it
 * becomes the current cursor and the previous one (NULL at the start: no cursor was ever set) is returned; GetCursor returns the current one; ShowCursor counts from 0. Any other handle stops. */
void fle_imp_SetCursor(void)
{
    static const char api[] = "SetCursor";
    uint64_t h = R[1], prev;
    int standard = fle_cursor_image_idc_of != NULL && h != 0 && fle_cursor_image_idc_of(h) != 0;

    /* decision 0125 (wall 54): a cursor that LoadCursorW or LoadImageW loaded from a module (the shared handle forms of fle_cursor.h) is a cursor "loaded by either the LoadCursor or the LoadImage function" (the page) */
    if (!standard && h != 0 && fle_cursor_is_shared_handle != NULL && fle_cursor_is_shared_handle(h)) standard = 1;
    fle_gdi_check_thread(api);
    if (h != 0 && !standard && fle_gdi_kind(h) != FLE_GDI_KIND_ICONCUR)
        fle_stop(api, "hCursor is not a cursor of LoadCursorW, LoadImageW or CreateIconIndirect (an icon of LoadIconW, a deleted handle, another kind of handle): the page lists those four sources");
    prev = cur_cursor;
    cur_cursor = h;
    R[0] = prev;
}

void fle_imp_GetCursor(void)
{
    static const char api[] = "GetCursor";

    fle_gdi_check_thread(api);
    R[0] = cur_cursor;
}

void fle_imp_ShowCursor(void)
{
    static const char api[] = "ShowCursor";

    fle_gdi_check_thread(api);
    if ((uint32_t)R[1]) show_count++;
    else show_count--;
    R[0] = (uint32_t)show_count;
}

/* The inspection functions of decision 0111 (fle_gdi.h). */
int fle_gdi_dc_attr(uint64_t h, int32_t out[13])
{
    gdi_obj *o = live_obj(h);
    const gdi_dc *dc;

    if (!o || o->kind != FLE_GDI_KIND_DC) return 0;
    dc = (const gdi_dc *)o->ext;
    out[0] = (int32_t)dc->bk_mode;
    out[1] = (int32_t)dc->rop2;
    out[2] = (int32_t)dc->map_mode;
    out[3] = (int32_t)dc->stretch;
    out[4] = dc->pos[0];
    out[5] = dc->pos[1];
    out[6] = dc->worg[0];
    out[7] = dc->worg[1];
    out[8] = dc->vorg[0];
    out[9] = dc->vorg[1];
    out[10] = dc->borg[0];
    out[11] = dc->borg[1];
    out[12] = (int32_t)dc->depth;
    return 1;
}

uint32_t fle_gdi_screen_states(void)
{
    uint32_t n = 0;

    for (uint64_t i = 0; i < scr_cap; i++) n += scr_dc[i] != NULL;
    return n;
}

int fle_gdi_screen_attr(uint64_t h, int32_t out[13])
{
    const gdi_dc *dc;

    if (!fle_gdc_is_live(h)) return 0;
    dc = screen_state("fle_gdi_screen_attr", h);
    out[0] = (int32_t)dc->bk_mode;
    out[1] = (int32_t)dc->rop2;
    out[2] = (int32_t)dc->map_mode;
    out[3] = (int32_t)dc->stretch;
    out[4] = dc->pos[0];
    out[5] = dc->pos[1];
    out[6] = dc->worg[0];
    out[7] = dc->worg[1];
    out[8] = dc->vorg[0];
    out[9] = dc->vorg[1];
    out[10] = dc->borg[0];
    out[11] = dc->borg[1];
    out[12] = (int32_t)dc->depth;
    return 1;
}

int fle_gdi_pen_info(uint64_t h, uint32_t out[3])
{
    gdi_obj *o = live_obj(h);

    if (!o || o->kind != FLE_GDI_KIND_PEN) return 0;
    out[0] = o->u[0];
    out[1] = o->u[1];
    out[2] = o->u[2];
    return 1;
}

uint64_t fle_gdi_pen_live_count(void) { return pen_live; }
uint64_t fle_gdi_cursor_current(void) { return cur_cursor; }
int32_t fle_gdi_cursor_display_count(void) { return show_count; }

/* The memory-DC answer of gdi32!GetDeviceCaps (decision 0111, OPTIONAL: fle_gdc.c calls it only when the hook of g1-optional-getdevicecaps.diff is applied). Learn (CreateCompatibleBitmap, memory DCs): the bitmap
 * selected into a memory DC gives it its colour format ("an array large enough to store color information", the format of the bitmap); Learn (GetDeviceCaps): BITSPIXEL "Number of adjacent color bits for each
 * pixel" (16 for a 15 or 16 bit device), PLANES "Number of color planes". READING: a memory DC reports the bits and the planes of the bitmap selected into it. The pages say HORZRES, VERTRES and the others describe the
 * screen: every other index stops by name. Returns 0 when h is not a live memory DC (the caller goes on with its own rules). */
int fle_gdi_memdc_caps(uint64_t h, uint32_t idx, int32_t *out)
{
    static const char api[] = "GetDeviceCaps";
    gdi_dc *dc = NULL;
    gdi_obj *b;
    char why[260];

    if (dc_class(h, &dc) != DCK_MEM) return 0;
    b = live_obj(dc->bmp);
    if (!b || b->kind != FLE_GDI_KIND_BITMAP) fle_stop(api, "the bitmap selected into the memory DC is gone");
    if (idx == 12u) {
        *out = (int32_t)b->bm[3];
        return 1;
    }
    if (idx == 14u) {
        *out = (int32_t)b->bm[2];
        return 1;
    }
    snprintf(why, sizeof why, "index %u of a memory DC: the page describes the screen for it (only BITSPIXEL 12 and PLANES 14 are answered, from the bitmap selected into the DC; decision 0111)", (unsigned)idx);
    fle_stop(api, why);
}


/* The named stops. Each is a call whose result the pages leave to the rasterizer, the font files, the regions, the system or the metafile format: the run stops with the name, the reason and the decision, and no
 * value is made up (decision 0111, class C). The functions are listed in the order of the rows of imports_table.rs. */
#define GDI_WHY_DRAW   "drawing with the selected pen and brush needs the rasterizer: the page gives the effect, not the pixels GDI writes"
#define GDI_WHY_PIXEL  "this pixel transfer or read-back is not one of FillRect, PatBlt and BitBlt(SRCCOPY) between DIB sections: the conversions and the bytes GDI writes are not on the pages, and a device-dependent bitmap keeps no pixels"
#define GDI_WHY_GLYPH  "glyph rasterization needs the font files and the rasterizer: not modelled"
#define GDI_WHY_TEXT   "text metrics and extents need the glyph data of the font: not modelled"
#define GDI_WHY_REGION "clip regions and region objects are not modelled: BitBlt, PatBlt and FillRect assume that no clip region exists"
#define GDI_WHY_PAL    "the system palette and the colour matching of palettes are not on the pages for this virtual display"
#define GDI_WHY_ORG    "the final translation origin of a memory DC is not on the page"
#define GDI_WHY_META   "metafile data and playback are not modelled"
#define GDI_STOP(Name, Why) void fle_imp_##Name(void) { fle_stop(#Name, Why " (decision 0111)"); }

GDI_STOP(RoundRect, GDI_WHY_DRAW)
GDI_STOP(Ellipse, GDI_WHY_DRAW)
GDI_STOP(Pie, GDI_WHY_DRAW)
GDI_STOP(Chord, GDI_WHY_DRAW)
GDI_STOP(Arc, GDI_WHY_DRAW)
GDI_STOP(ArcTo, GDI_WHY_DRAW)
GDI_STOP(AngleArc, GDI_WHY_DRAW)
GDI_STOP(Polyline, GDI_WHY_DRAW)
GDI_STOP(Polygon, GDI_WHY_DRAW)
GDI_STOP(PolyBezier, GDI_WHY_DRAW)
GDI_STOP(PolyBezierTo, GDI_WHY_DRAW)
GDI_STOP(ExtFloodFill, GDI_WHY_DRAW)
GDI_STOP(FrameRgn, GDI_WHY_DRAW)
/* ---- Decision 0126: the DIB transfers, the pixel calls, the pen and the rectangles (the part of the model that follows the Learn pages named in the block comment above) ---- */

/* The image of a DIB passed by the guest (BITMAPINFO + bits) as a surface view: a 40-byte BITMAPINFOHEADER, one plane, 1, 4, 8, 24 or 32 bits (8 and less with a colour table of iUsage DIB_RGB_COLORS), BI_RGB (or
 * BI_BITFIELDS with the 8-8-8 masks for 32 bits). Anything else stops (JPEG and PNG are the stop (d')). `ct` receives the colour table (256 entries). */
static void dib_image(const char *api, const char *pre, uint64_t pbmi, uint64_t bits, uint32_t usage, uint32_t start, uint32_t clines, int whole, fle_rsurf *v, uint32_t ct[256], int32_t *height_out)
{
    uint32_t size, bpp, comp, ncol = 0, clrused, masks[3] = { 0, 0, 0 };
    int32_t w, h;
    uint64_t stride, nabs, bytes, ctab;

    if (pbmi == 0 || bits == 0) pixel_stop(api, pre, "lpbmi or lpBits is NULL: the page does not define it");
    size = (uint32_t)fle_ld(pbmi, 4);
    w = (int32_t)(uint32_t)fle_ld(pbmi + 4u, 4);
    h = (int32_t)(uint32_t)fle_ld(pbmi + 8u, 4);
    bpp = (uint32_t)fle_ld(pbmi + 14u, 2);
    comp = (uint32_t)fle_ld(pbmi + 16u, 4);
    clrused = (uint32_t)fle_ld(pbmi + 32u, 4);
    if (size != FLE_GDI_BMIH_SIZE) pixel_stop(api, pre, "biSize is not 40: the core, V4 and V5 headers (and JPEG or PNG data) are not modelled (stop condition (d') of decision 0126)");
    if ((uint32_t)fle_ld(pbmi + 12u, 2) != 1 || w <= 0 || h == 0) pixel_stop(api, pre, "biPlanes is not 1, biWidth is not above 0 or biHeight is 0: the page does not define it");
    if (bpp != 1 && bpp != 4 && bpp != 8 && bpp != 24 && bpp != 32) pixel_stop(api, pre, "biBitCount is not 1, 4, 8, 24 or 32: the 16-bit forms (5-5-5, 5-6-5) have no page for the conversion to a colour");
    if (comp != FLE_GDI_BI_RGB && !(comp == FLE_GDI_BI_BITFIELDS && bpp == 32)) pixel_stop(api, pre, "biCompression is not BI_RGB (or BI_BITFIELDS for 32 bits): compressed and RLE images are not modelled");
    if (usage != FLE_GDI_DIB_RGB_COLORS) pixel_stop(api, pre, "iUsage is not DIB_RGB_COLORS: DIB_PAL_COLORS needs the logical palette of the DC");
    ctab = pbmi + FLE_GDI_BMIH_SIZE;
    if (comp == FLE_GDI_BI_BITFIELDS) {
        for (uint32_t i = 0; i < 3; i++) masks[i] = (uint32_t)fle_ld(ctab + 4u * i, 4);
        if (masks[0] != 0x00FF0000u || masks[1] != 0x0000FF00u || masks[2] != 0x000000FFu) pixel_stop(api, pre, "the BI_BITFIELDS masks are not the 8-8-8 layout: the other layouts have no page for the conversion to a colour");
    } else if (bpp <= 8) {
        uint32_t maxc = 1u << bpp;

        ncol = clrused ? clrused : maxc;
        if (ncol > maxc) pixel_stop(api, pre, "biClrUsed is above 2^biBitCount: the page does not define it");
        for (uint32_t i = 0; i < ncol; i++) ct[i] = (uint32_t)fle_ld(ctab + 4u * i, 4);
    }
    stride = (((uint64_t)(uint32_t)w * bpp + 31u) & ~UINT64_C(31)) >> 3;
    nabs = h < 0 ? (uint64_t)(-(int64_t)h) : (uint64_t)h;
    bytes = stride * nabs;
    if (bytes >= UINT64_C(0x80000000)) pixel_stop(api, pre, "the image is 2 GiB or more: the page gives no rule for it");
    (void)start;
    (void)clines;
    (void)whole;
    (void)fle_addr(bits, 1, 0);
    (void)fle_addr(bits + bytes - 1u, 1, 0);
    memset(v, 0, sizeof *v);
    v->bits = (uint8_t *)fle_addr(bits, 1, 0);
    v->width = (uint32_t)w;
    v->height = (uint32_t)nabs;
    v->stride = (uint32_t)stride;
    v->bpp = bpp;
    v->comp = comp;
    for (int i = 0; i < 3; i++) v->masks[i] = masks[i];
    v->ncolors = ncol;
    v->clr = ct;
    v->bottom_up = h > 0;
    v->keep_x = 1;   /* the guest's image is a DIB like a DIB section: a same-format SRCCOPY copies the unused byte too (decision 0111 wall 6, the memcpy of StretchDIBits) */
    *height_out = h;
}

/* gdi32!StretchDIBits: int f(HDC hdc rcx, int xDest edx, int yDest r8d, int DestWidth r9d, int DestHeight [5], int xSrc [6], int ySrc [7], int SrcWidth [8], int SrcHeight [9], const VOID *lpBits [10], const
 * BITMAPINFO *lpbmi [11], UINT iUsage [12], DWORD rop [13]). Learn (StretchDIBits): "copies the color data for a rectangle of pixels in a DIB, JPEG, or PNG image to the specified destination rectangle. If the
 * destination rectangle is larger than the source rectangle, this function stretches the rows and columns of color data to fit the destination rectangle. If the destination rectangle is smaller than the source
 * rectangle, this function compresses the rows and columns by using the specified raster operation"; rop is "a raster-operation code that specifies how the source pixels, the destination device context's current
 * brush, and the destination pixels are to be combined"; "the return value is the number of scan lines copied. Note that this value can be negative for mirrored content"; "If the function fails, or no scan lines are
 * copied, the return value is 0"; "The origin of a bottom-up DIB is the lower-left corner; the origin of a top-down DIB is the upper-left corner"; "StretchDIBits creates a mirror image of a bitmap if the signs of the
 * nSrcWidth and nDestWidth parameters, or if the signs of the nSrcHeight and nDestHeight parameters differ". Answered (decision 0126): a 1-, 4-, 8-, 24- or 32-bit DIB (BI_RGB; 32 bits also BI_BITFIELDS 8-8-8) of
 * iUsage DIB_RGB_COLORS, any ternary code, any target of get_target; the source rectangle is (xSrc, ySrc, SrcWidth, SrcHeight) in the image with the image's own origin (the lower-left corner of a bottom-up DIB: ySrc
 * counts up from the bottom row, as the page's origin sentence says, a READING of how ySrc is read); a source rectangle not inside the image: the part outside is not read, the destination keeps its pixels there
 * (S-source-outside). The return value is the number of scan lines (DestHeight, or its absolute value when the source and destination heights have the same sign: the page's "negative for mirrored" is read as the
 * sign of the mirror, S-stretchdibits-return); 0 when nothing was copied. */
void fle_imp_StretchDIBits(void)
{
    static const char api[] = "StretchDIBits";
    int32_t xd = (int32_t)(uint32_t)R[2], yd = (int32_t)(uint32_t)R[8], wd = (int32_t)(uint32_t)R[9], hd = (int32_t)(uint32_t)fle_arg(5);
    int32_t xs = (int32_t)(uint32_t)fle_arg(6), ys = (int32_t)(uint32_t)fle_arg(7), ws = (int32_t)(uint32_t)fle_arg(8), hs = (int32_t)(uint32_t)fle_arg(9);
    uint64_t bits = fle_arg(10), pbmi = fle_arg(11);
    uint32_t usage = (uint32_t)fle_arg(12), rop = (uint32_t)fle_arg(13), idx, pat = 0, ct[256];
    int32_t hdr_h;
    target t;
    gdi_dc *dc;
    fle_rsurf img;
    fle_rrect dr;
    int chk = 0;   /* decision 0245: fle_r_changed_begin was called */
    int64_t sx0, sy0, nrows;
    uint64_t n;
    char pre[260];

    fle_gdi_check_thread(api);
    snprintf(pre, sizeof pre, "pixel transfer from a DIB (StretchDIBits rop 0x%x, iUsage %u, %d x %d to (%d, %d) from %d x %d at (%d, %d))", (unsigned)rop, (unsigned)usage, (int)wd, (int)hd, (int)xd, (int)yd, (int)ws, (int)hs, (int)xs, (int)ys);
    idx = ternary(api, pre, rop, 1);
    get_target(api, pre, R[1], "hdc", &t);
    dc = target_attrs(R[1]);
    mode_check(api, pre, dc, "hdc");
    fill_check(api, pre, &t, "hdc");
    dib_image(api, pre, pbmi, bits, usage, 0, 0, 0, &img, ct, &hdr_h);
    if (!pattern_of(api, pre, idx, dc, &pat)) {
        R[0] = 0;
        return;
    }
    if (wd == 0 || hd == 0 || ws == 0 || hs == 0) {
        R[0] = 0;
        return;
    }
    if (!target_live(api, &t)) {
        R[0] = (uint32_t)(hd < 0 ? -hd : hd);
        return;
    }
    nrows = (int64_t)img.height;
    /* the source rectangle in image rows from the top: a bottom-up image counts ySrc from its lower-left corner (the page's origin sentence) */
    sx0 = ws < 0 ? (int64_t)xs + ws : xs;
    sy0 = hs < 0 ? (int64_t)ys + hs : ys;
    if (img.bottom_up) sy0 = nrows - ((int64_t)(hs < 0 ? -(int64_t)hs : hs) + (hs < 0 ? (int64_t)ys + hs : (int64_t)ys));
    {
        int64_t aw = ws < 0 ? -(int64_t)ws : ws, ah = hs < 0 ? -(int64_t)hs : hs;
        int64_t ex = sx0 + aw, ey = sy0 + ah;
        fle_rrect in = { sx0, sy0, ex, ey }, whole = { 0, 0, (int64_t)img.width, nrows };

        dr = dev_rect(dc, &t, xd, yd, wd, hd);
        if (sx0 < whole.x0 || sy0 < whole.y0 || ex > whole.x1 || ey > whole.y1) {
            /* the source rectangle is partly outside the image: only a 1:1 copy has a defined place for the part that is inside (S-source-outside); a stretched one stops */
            if ((wd < 0 ? -(int64_t)wd : wd) != aw || (hd < 0 ? -(int64_t)hd : hd) != ah) pixel_stop(api, pre, "the source rectangle is not inside the image and the copy is stretched: which destination pixels it covers is not on the page");
            fle_r_clip(&in, &whole);
            if (fle_r_empty(&in)) {
                R[0] = 0;
                return;
            }
            n = 0;
            chk = blit_begin(&t);
            EACH_CLIP(t, n += fle_r_blit(&t.s, CL_, dr.x0 + (in.x0 - sx0), dr.y0 + (in.y0 - sy0), in.x1 - in.x0, in.y1 - in.y0, &img, in.x0, in.y0, idx, pat));
            blit_end(chk, &dr, &n);
            target_wrote(api, &t, dr, n);
            R[0] = (uint32_t)(int32_t)(in.y1 - in.y0);
            return;
        }
        if ((wd < 0 ? -(int64_t)wd : wd) == aw && (hd < 0 ? -(int64_t)hd : hd) == ah && (wd < 0) == (ws < 0) && (hd < 0) == (hs < 0)) {
            n = 0;
            chk = blit_begin(&t);
            EACH_CLIP(t, n += fle_r_blit(&t.s, CL_, wd < 0 ? dr.x0 + wd : dr.x0, hd < 0 ? dr.y0 + hd : dr.y0, aw, ah, &img, sx0, sy0, idx, pat));
        } else {
            n = 0;
            EACH_CLIP(t, n += fle_r_stretch(&t.s, CL_, dr.x0, dr.y0, wd, hd, &img, sx0 + (ws < 0 ? aw : 0), sy0 + (hs < 0 ? ah : 0), ws, hs, idx, pat, dc->stretch));
        }
    }
    blit_end(chk, &dr, &n);
    target_wrote(api, &t, dr, n);
    R[0] = (uint32_t)(int32_t)((hd < 0) != (hs < 0) ? -(hd < 0 ? -hd : hd) : (hd < 0 ? -hd : hd));
}

/* gdi32!SetDIBitsToDevice: int f(HDC hdc rcx, int xDest edx, int yDest r8d, DWORD w r9d, DWORD h [5], int xSrc [6], int ySrc [7], UINT StartScan [8], UINT cLines [9], const VOID *lpvBits [10], const BITMAPINFO
 * *lpbmi [11], UINT ColorUse [12]). Learn (SetDIBitsToDevice): "sets the pixels in the specified rectangle on the device that is associated with the destination device context using color data from a DIB, JPEG, or PNG
 * image"; (xSrc, ySrc) is "the lower-left corner of the image"; "StartScan: the starting scan line in the image", "cLines: the number of DIB scan lines contained in the array pointed to by the lpvBits parameter";
 * "If zero scan lines are set (such as when dwHeight is 0) or the function fails, the function returns zero"; the return value is "the number of scan lines set". "The values of the uStartScan and cScanLines parameters
 * identify the portion of the bitmap contained in the lpvBits array" (banding). Answered: the whole image given (StartScan 0 and cLines = |biHeight|: S-banding: a banded call stops: the page gives the geometry of
 * a band in words only), a 1-, 4-, 8-, 24- or 32-bit DIB of DIB_RGB_COLORS, the destination rectangle (xDest, yDest, w, h) at the lower-left source corner (xSrc, ySrc) of the image: the rows of the image from its bottom
 * (for a bottom-up one) or counted up from its last row (for a top-down one: the page's origin sentence). The copy is SRCCOPY, 1:1; the number of scan lines set is the number of rows written (clipped ones count). */
void fle_imp_SetDIBitsToDevice(void)
{
    static const char api[] = "SetDIBitsToDevice";
    int32_t xd = (int32_t)(uint32_t)R[2], yd = (int32_t)(uint32_t)R[8];
    uint32_t w = (uint32_t)R[9], h = (uint32_t)fle_arg(5);
    int32_t xs = (int32_t)(uint32_t)fle_arg(6), ys = (int32_t)(uint32_t)fle_arg(7);
    uint32_t start = (uint32_t)fle_arg(8), nl = (uint32_t)fle_arg(9), usage = (uint32_t)fle_arg(12), ct[256];
    uint64_t bits = fle_arg(10), pbmi = fle_arg(11);
    int32_t hdr_h;
    target t;
    gdi_dc *dc;
    fle_rsurf img;
    fle_rrect dr;
    int chk = 0;   /* decision 0245: fle_r_changed_begin was called */
    int64_t top_row, nrows;
    uint64_t n = 0;
    char pre[260];

    fle_gdi_check_thread(api);
    snprintf(pre, sizeof pre, "pixel transfer from a DIB (SetDIBitsToDevice %u x %u to (%d, %d) from (%d, %d), StartScan %u, cLines %u, ColorUse %u)", (unsigned)w, (unsigned)h, (int)xd, (int)yd, (int)xs, (int)ys, (unsigned)start, (unsigned)nl, (unsigned)usage);
    get_target(api, pre, R[1], "hdc", &t);
    dc = target_attrs(R[1]);
    mode_check(api, pre, dc, "hdc");
    fill_check(api, pre, &t, "hdc");
    dib_image(api, pre, pbmi, bits, usage, start, nl, 1, &img, ct, &hdr_h);
    nrows = (int64_t)img.height;
    if (start != 0 || (int64_t)nl != nrows) pixel_stop(api, pre, "StartScan is not 0 or cLines is not all the rows of the image: a banded call is not modelled (S-banding)");
    R[0] = 0;
    if (w == 0 || h == 0 || !target_live(api, &t)) return;
    /* the source rectangle: x from xSrc; y counts up from the lower-left corner of the image, so the rows [ys, ys + h) from the bottom are the image rows [nrows - ys - h, nrows - ys) from the top */
    top_row = nrows - (int64_t)ys - (int64_t)h;
    dr = dev_rect(dc, &t, xd, yd, w, h);
    {
        fle_rrect in = { xs, top_row, (int64_t)xs + w, top_row + h }, whole = { 0, 0, (int64_t)img.width, nrows };
        int64_t ox = in.x0, oy = in.y0;

        fle_r_clip(&in, &whole);
        if (fle_r_empty(&in)) return;
        n = 0;
        chk = blit_begin(&t);
        EACH_CLIP(t, n += fle_r_blit(&t.s, CL_, dr.x0 + (in.x0 - ox), dr.y0 + (in.y0 - oy), in.x1 - in.x0, in.y1 - in.y0, &img, in.x0, in.y0, FLE_R_SRCCOPY_IDX, 0));
        R[0] = (uint32_t)(in.y1 - in.y0);
    }
    blit_end(chk, &dr, &n);
    target_wrote(api, &t, dr, n);
}

/* gdi32!StretchBlt: BOOL f(HDC hdcDest rcx, int xDest edx, int yDest r8d, int wDest r9d, int hDest [5], HDC hdcSrc [6], int xSrc [7], int ySrc [8], int wSrc [9], int hSrc [10], DWORD rop [11]). Learn (StretchBlt):
 * "copies a bitmap from a source rectangle into a destination rectangle, stretching or compressing the bitmap to fit the dimensions of the destination rectangle, if necessary. The system stretches or compresses the
 * bitmap according to the stretching mode currently set in the destination device context"; "The color data for pattern or destination pixels is merged after the stretching or compression occurs"; "If the specified
 * raster operation requires a brush, the system uses the brush currently selected into the destination device context"; "The destination coordinates are transformed by using the transformation currently specified for
 * the destination device context; the source coordinates ... for the source device context"; "StretchBlt creates a mirror image of a bitmap if the signs of the nWidthSrc and nWidthDest parameters or if the nHeightSrc
 * and nHeightDest parameters differ"; nonzero on success. Answered: every ternary code, DCs as BitBlt; the pixel rule of stretching is S-stretch-groups and S-stretch-halftone (fle_raster.c). A zero extent paints nothing
 * and returns 1 (S-negative-extent: not on the page). */
void fle_imp_StretchBlt(void)
{
    static const char api[] = "StretchBlt";
    int32_t x = (int32_t)(uint32_t)R[2], y = (int32_t)(uint32_t)R[8], wd = (int32_t)(uint32_t)R[9], hd = (int32_t)(uint32_t)fle_arg(5);
    int32_t xs = (int32_t)(uint32_t)fle_arg(7), ys = (int32_t)(uint32_t)fle_arg(8), ws = (int32_t)(uint32_t)fle_arg(9), hs = (int32_t)(uint32_t)fle_arg(10);
    uint32_t rop = (uint32_t)fle_arg(11);
    char pre[220];

    fle_gdi_check_thread(api);
    snprintf(pre, sizeof pre, "pixel transfer between DCs (StretchBlt rop 0x%x, %d x %d to (%d, %d) from %d x %d at (%d, %d))", (unsigned)rop, (int)wd, (int)hd, (int)x, (int)y, (int)ws, (int)hs, (int)xs, (int)ys);
    blit_call(api, pre, R[1], x, y, wd, hd, fle_arg(6), xs, ys, rop, 1, ws, hs);
}

/* ---- single pixels ---- */

/* gdi32!GetPixel: COLORREF f(HDC hdc rcx, int x edx, int y r8d). Learn (GetPixel): "retrieves the red, green, blue (RGB) color value of the pixel at the specified coordinates"; "If the pixel is outside of the current
 * clipping region, the return value is CLR_INVALID (0xFFFFFFFF)"; "A bitmap must be selected within the device context, otherwise, CLR_INVALID is returned on all pixels". Answered: any readable target; a pixel outside
 * the clip (the bitmap, or the visible rectangle of a window) gives CLR_INVALID. The window that is not visible has an empty visible region: CLR_INVALID too (S-draw-hidden). */
void fle_imp_GetPixel(void)
{
    static const char api[] = "GetPixel";
    int32_t x = (int32_t)(uint32_t)R[2], y = (int32_t)(uint32_t)R[8];
    target t;
    gdi_dc *dc;
    fle_rrect r;
    char pre[100];

    fle_gdi_check_thread(api);
    snprintf(pre, sizeof pre, "reading a pixel (GetPixel at (%d, %d))", (int)x, (int)y);
    get_target(api, pre, R[1], "hdc", &t);
    dc = target_attrs(R[1]);
    mode_check(api, pre, dc, "hdc");
    source_ok(api, pre, &t, "hdc");
    r = dev_rect(dc, &t, x, y, 1, 1);
    if (t.hidden || !target_has_point(&t, r.x0, r.y0)) {
        R[0] = 0xFFFFFFFFu;
        return;
    }
    R[0] = fle_r_get(&t.s, (uint32_t)r.x0, (uint32_t)r.y0);
}

/* gdi32!SetPixel / SetPixelV: COLORREF f(HDC hdc rcx, int x edx, int y r8d, COLORREF color r9d) / BOOL f(...). Learn (SetPixel): "sets the pixel at the specified coordinates to the specified color"; "the return
 * value is the RGB value that the function sets the pixel to. This value may differ from the color specified by crColor; that occurs when an exact match for the specified color cannot be found. If the function fails,
 * the return value is -1" (ERROR_INVALID_PARAMETER); "The function fails if the pixel coordinates lie outside of the current clipping region". (SetPixelV): "sets the pixel at the specified coordinates to the closest
 * approximation of the specified color. The point must be in the clipping region and the visible part of the device surface"; nonzero on success, zero on failure. Answered: a direct target (24- or 32-bit): the pixel
 * is set to the colour exactly (the 24-bit colours are all representable on a 24- or 32-bit surface: a READING), the result is that colour; a monochrome bitmap and the other formats stop (the nearest colour of a 1-bit
 * or palette surface is not on the pages). A pixel outside the clip: SetPixel returns -1 with last error 87, SetPixelV returns 0. A colour with a nonzero high byte (a palette form) stops. */
static void set_pixel(const char *api, int v)
{
    int32_t x = (int32_t)(uint32_t)R[2], y = (int32_t)(uint32_t)R[8];
    uint32_t color = (uint32_t)R[9];
    target t;
    gdi_dc *dc;
    fle_rrect r;
    char pre[110];

    fle_gdi_check_thread(api);
    snprintf(pre, sizeof pre, "setting a pixel (%s at (%d, %d), colour 0x%x)", api, (int)x, (int)y, (unsigned)color);
    get_target(api, pre, R[1], "hdc", &t);
    dc = target_attrs(R[1]);
    mode_check(api, pre, dc, "hdc");
    fill_check(api, pre, &t, "hdc");
    if (!color_plain(color, &color)) pixel_stop(api, pre, "the colour has a high-order byte other than 0 and 2 (PALETTEINDEX needs a palette): the page does not describe it for this call");
    r = dev_rect(dc, &t, x, y, 1, 1);
    if (t.hidden || !target_has_point(&t, r.x0, r.y0)) {
        if (v) {
            R[0] = 0;
        } else {
            fle_set_last_error(FLE_GDI_ERROR_INVALID_PARAMETER_);
            R[0] = 0xFFFFFFFFu;
        }
        return;
    }
    fle_r_put(&t.s, (uint32_t)r.x0, (uint32_t)r.y0, color);
    target_wrote(api, &t, r, 1);
    R[0] = v ? 1u : color;
}

void fle_imp_SetPixel(void) { set_pixel("SetPixel", 0); }
void fle_imp_SetPixelV(void) { set_pixel("SetPixelV", 1); }

/* ---- bitmap bits ---- */

/* The device-dependent bitmap handle `h` and its view, or a stop. A DIB section is also accepted by GetBitmapBits/SetBitmapBits? The pages say "device-dependent bitmap" and "This must be a compatible bitmap (DDB)"
 * for the others: a DIB section stops. */
static gdi_obj *ddb_arg(const char *api, const char *pre, uint64_t h, fle_rsurf *v)
{
    gdi_obj *o = live_obj(h);

    if (!o || o->kind != FLE_GDI_KIND_BITMAP) pixel_stop(api, pre, "the handle is not a live bitmap of this runtime");
    if (o->fmt == FLE_GDI_BMP_DIB) pixel_stop(api, pre, "the handle is a DIB section: the page asks for a device-dependent bitmap");
    if (!ddb_view(o, v)) pixel_stop(api, pre, "out of memory for the pixels of a device-dependent bitmap");
    return o;
}

/* gdi32!GetBitmapBits: LONG f(HBITMAP hbit rcx, LONG cb edx, LPVOID lpvBits r8) (and SetBitmapBits: LONG f(HBITMAP hbm rcx, DWORD cb edx, const VOID *pvBits r8): written with it, not a row of the response table: no module of the chain
 * imports it, decision 0126). Learn (GetBitmapBits): "copies the bitmap bits of a
 * specified device-dependent bitmap into a buffer"; cb is "the number of bytes to copy"; "the return value is the number of bytes copied to the buffer"; zero on failure. (SetBitmapBits): "sets the bits of color data
 * for a bitmap to the specified values"; "the return value is the number of bytes used in setting the bitmap bits"; "The array identified by lpBits must be WORD aligned" (CreateBitmap: each scan line "must be word
 * aligned"). The layout of the bytes is the WORD-aligned one of CreateBitmap's cj formula (rows padded to 16 bits): the block of the model is DWORD-aligned (S-ddb-stride), so the bytes are converted row by row between
 * the two strides. The pixel format of a colour bitmap is 32 bits BGRX (S-ddb-format), of a monochrome one 1 bit, the leftmost pixel in the most significant bit (the page Bitmap Storage for the DIB; a READING for
 * the DDB). cb above the bitmap's size is clamped to it (S-bitmapbits-clamp: the pages say "the number of bytes to copy"). */
static uint64_t ddb_word_stride(const gdi_obj *o) { return (((uint64_t)o->bm[0] * o->bm[2] * o->bm[3] + 15u) >> 4) << 1; }

static void bitmap_bits(const char *api, int set)
{
    uint64_t h = R[1], buf = R[8], cb = (uint32_t)R[2], total, ws, left, done = 0;
    fle_rsurf v;
    gdi_obj *o;
    char pre[90];

    fle_gdi_check_thread(api);
    snprintf(pre, sizeof pre, "%s of bitmap 0x%" PRIx64 ", %u bytes", api, h, (unsigned)cb);
    o = ddb_arg(api, pre, h, &v);
    if (buf == 0) pixel_stop(api, pre, "the buffer is NULL: the page does not define it");
    ws = ddb_word_stride(o);
    total = ws * o->bm[1];
    if (cb > total) cb = total;
    if (!set && (int32_t)(uint32_t)R[2] < 0) {
        R[0] = 0;
        return;
    }
    if (cb == 0) {
        R[0] = 0;
        return;
    }
    (void)fle_addr(buf, 1, set ? 0 : 1);
    (void)fle_addr(buf + cb - 1u, 1, set ? 0 : 1);
    left = cb;
    for (uint32_t row = 0; row < o->bm[1] && left > 0; row++) {
        uint64_t take = ws < left ? ws : left;
        uint8_t *mem = v.bits + (uint64_t)row * v.stride;

        if (set) {
            uint64_t inrow = ((uint64_t)o->bm[0] * o->bm[3] + 7u) >> 3;

            for (uint64_t k = 0; k < take && k < inrow; k++) mem[k] = (uint8_t)fle_ld(buf + done + k, 1);
        } else {
            for (uint64_t k = 0; k < take; k++) fle_st(buf + done + k, 1, k < v.stride ? mem[k] : 0u);
        }
        done += take;
        left -= take;
    }
    R[0] = done;
}

/* Decision 0132 (PROVISIONAL), wall 18: CreateBitmap with lpBits != NULL (it was a stop: "initial pixels are not implemented"). Learn (CreateBitmap): lpBits "A pointer to an array of color data used to set the colors in a
 * rectangle of pixels. Each scan line in the rectangle must be word aligned (scan lines that are not word aligned must be padded with zeros). The buffer size expected, cj, can be calculated using the formula: cj = (((nWidth
 * * nPlanes * nBitCount + 15) >> 4) << 1) * nHeight"; "If this parameter is NULL, then the contents of the new bitmap are undefined"; "If the bitmap is monochrome, zeros represent the foreground color and ones represent the
 * background color for the destination device context". The bitmap of this runtime that CreateBitmap makes is monochrome only (1 plane, 1 bit), so the initial bits are the bytes of SetBitmapBits (above): the same word-aligned rows,
 * copied into the bitmap's 1-bit block (the leftmost pixel in the most significant bit, S-ddb-format). The whole cj bytes are read (the page says the buffer "expected" is cj); a zero width or height (the 1-by-1 bitmap) takes none. */
static void ddb_init_bits(const char *api, uint64_t h, uint64_t bits)
{
    fle_rsurf v;
    gdi_obj *o;
    char pre[90];
    uint64_t ws, total, inrow;

    snprintf(pre, sizeof pre, "%s of bitmap 0x%" PRIx64 " with initial bits", api, h);
    o = ddb_arg(api, pre, h, &v);
    ws = ddb_word_stride(o);
    total = ws * o->bm[1];
    inrow = ((uint64_t)o->bm[0] * o->bm[3] + 7u) >> 3;
    (void)fle_addr(bits, 1, 0);
    (void)fle_addr(bits + total - 1u, 1, 0);
    for (uint32_t row = 0; row < o->bm[1]; row++) {
        uint8_t *mem = v.bits + (uint64_t)row * v.stride;

        for (uint64_t k = 0; k < inrow; k++) mem[k] = (uint8_t)fle_ld(bits + (uint64_t)row * ws + k, 1);
    }
}

void fle_imp_GetBitmapBits(void) { bitmap_bits("GetBitmapBits", 0); }

/* gdi32!CreatePatternBrush: HBRUSH f(HBITMAP hbm rcx). Decision 0132 (PROVISIONAL), wall 19 (the guest makes the 8-by-8 monochrome bitmap of CreateBitmap with initial bits, then this). Learn (CreatePatternBrush): "creates a logical
 * brush with the specified bitmap pattern. The bitmap can be a DIB section bitmap ... or it can be a device-dependent bitmap"; the return value "identifies a logical brush", NULL on failure; "A pattern brush is a bitmap that the
 * system uses to paint the interiors of filled shapes"; "it can select that brush into any device context by calling the SelectObject function"; "You can delete a pattern brush without affecting the associated bitmap by using
 * the DeleteObject function. Therefore, you can then use this bitmap to create any number of pattern brushes"; "A brush created by using a monochrome (1 bit per pixel) bitmap has the text and background colors of the device
 * context to which it is drawn. Pixels represented by a 0 bit are drawn with the current text color; pixels represented by a 1 bit are drawn with the current background color". Answered: hbm a live device-dependent bitmap of
 * 8 x 8 pixels, 1 plane, 1 bit (the guest's): a new brush handle of the kind of CreateBrushIndirect with style BS_PATTERN; the table keeps a COPY of the eight rows (S-patternbrush-copy: a choice without a source; the page does not say
 * whether later writes to the bitmap reach the brush). The page does not say how a pattern is laid on the surface (the brush origin, SetBrushOrgEx, is the Learn page's) and no pixel of a pattern fill is written: a fill, PatBlt or
 * BitBlt with a pattern operation that uses such a brush stops by name (brush_color). Every other bitmap (a colour one, another size, a DIB section, a handle that is not a live bitmap) stops: the page names no failure case. */
void fle_imp_CreatePatternBrush(void)
{
    static const char api[] = "CreatePatternBrush";
    uint64_t h = R[1], slot;
    gdi_obj *o, *b;
    fle_rsurf v;
    uint32_t rows[2] = { 0, 0 };

    fle_gdi_check_thread(api);
    o = live_obj(h);
    if (!o || o->kind != FLE_GDI_KIND_BITMAP) fle_stop(api, "hbm is not a live bitmap of this runtime: the page does not define it");
    if (o->fmt == FLE_GDI_BMP_DIB) fle_stop(api, "hbm is a DIB section: only the 8-by-8 monochrome device-dependent bitmap of the guest is answered");
    if (o->bm[0] != 8 || o->bm[1] != 8 || o->bm[2] != 1 || o->bm[3] != 1) fle_stop(api, "hbm is not 8 by 8 pixels, 1 plane, 1 bit: only the monochrome pattern of the guest is answered");
    if (!ddb_view(o, &v)) fle_stop(api, "out of memory for the pixels of a device-dependent bitmap");
    for (uint32_t row = 0; row < 8; row++) rows[row >> 2] |= (uint32_t)v.bits[(uint64_t)row * v.stride] << (8u * (row & 3u));
    slot = new_slot(api);
    b = &objs[slot];
    b->u[0] = FLE_GDI_BS_PATTERN;
    b->u[1] = 0;
    b->u[2] = rows[0];
    b->u[3] = rows[1];
    issue(b, FLE_GDI_KIND_BRUSH);
    brush_live++;
    R[0] = slot_handle(slot);
}
void fle_imp_SetBitmapBits(void) { bitmap_bits("SetBitmapBits", 1); }

/* ---- pens and lines ---- */

/* The pen selected into a DC as a colour and a width: a pen made by CreatePen / CreatePenIndirect (kind PEN: style, width, COLORREF), the stock pens (WHITE_PEN 6, BLACK_PEN 7, NULL_PEN 8: Learn GetStockObject names
 * them; their colours white and black are the names') and the DEFAULT pen of a DC (BLACK_PEN: Display Device Context Defaults "Pen BLACK_PEN"). `*invisible` = PS_NULL. Only cosmetic one-pixel solid pens are drawn
 * (Learn CreatePenIndirect "a logical cosmetic pen"; LOGPEN: "If the x member is NULL, then the pen is one pixel wide on raster devices"; the widths above 1, and PS_DASH to PS_DASHDOTDOT, stop: S-pen-width,
 * S-pen-dashes: the pixels of a wide line and of the dash pattern are not on the pages). PS_INSIDEFRAME of width <= 1 is solid (CreatePen: "The pen is solid"). */
static uint32_t pen_color(const char *api, const char *pre, uint64_t hpen, int *invisible)
{
    gdi_obj *o = live_obj(hpen);

    *invisible = 0;
    if (!o) pixel_stop(api, pre, "the pen selected into the DC is gone");
    switch (o->kind) {
    case FLE_GDI_KIND_PEN:
        if (o->u[0] == FLE_GDI_PS_NULL) {
            *invisible = 1;
            return 0;
        }
        if (o->u[0] != FLE_GDI_PS_SOLID && o->u[0] != FLE_GDI_PS_INSIDEFRAME) pixel_stop(api, pre, "the pen is dashed or dotted (PS_DASH to PS_DASHDOTDOT): the dash pattern is not on the pages (S-pen-dashes)");
        if (o->u[1] > 1u) pixel_stop(api, pre, "the pen is wider than one pixel: the pixels of a wide line are not on the pages (S-pen-width)");
        return o->u[2];
    case FLE_GDI_KIND_STOCK_PEN:
        if (o->fmt == 8) {
            *invisible = 1;
            return 0;
        }
        return o->fmt == 6 ? 0xFFFFFFu : 0x000000u;
    case FLE_GDI_KIND_DEFAULT:
        if (o->fmt == FLE_GDI_DEFAULT_PEN) return 0x000000u;
        pixel_stop(api, pre, "the handle is not a pen");
    default: pixel_stop(api, pre, "the handle is not a pen");
    }
}

/* gdi32!LineTo: BOOL f(HDC hdc rcx, int x edx, int y r8d). Learn (LineTo): "draws a line from the current position up to, but not including, the specified point"; "The line is drawn by using the current pen and, if
 * the pen is a geometric pen, the current brush"; "If LineTo succeeds, the current position is set to the specified ending point"; nonzero on success. Learn (Lines): "The pixel located at the starting point is always
 * included in the line, and the pixel located at the ending point is always excluded". The pixels between the two ends are S-line-dda (fle_raster.c). The colour is combined with the destination by the drawing mode
 * of the DC (SetROP2, Binary Raster Operations: R2_COPYPEN by default). A PS_NULL pen draws nothing and the position still moves. */
void fle_imp_LineTo(void)
{
    static const char api[] = "LineTo";
    int32_t x = (int32_t)(uint32_t)R[2], y = (int32_t)(uint32_t)R[8];
    target t;
    gdi_dc *dc;
    fle_rrect a, b;
    int inv;
    uint32_t col;
    uint64_t n = 0;
    char pre[100];

    fle_gdi_check_thread(api);
    snprintf(pre, sizeof pre, "drawing a line (LineTo (%d, %d))", (int)x, (int)y);
    get_target(api, pre, R[1], "hdc", &t);
    dc = target_attrs(R[1]);
    mode_check(api, pre, dc, "hdc");
    fill_check(api, pre, &t, "hdc");
    col = pen_color(api, pre, dc->pen, &inv);
    a = dev_rect(dc, &t, dc->pos[0], dc->pos[1], 0, 0);
    b = dev_rect(dc, &t, x, y, 0, 0);
    if (!inv && target_live(api, &t)) {
        int64_t adx = b.x0 > a.x0 ? b.x0 - a.x0 : a.x0 - b.x0, ady = b.y0 > a.y0 ? b.y0 - a.y0 : a.y0 - b.y0;

        if ((adx > ady ? adx : ady) > FLE_R_LINE_MAX) pixel_stop(api, pre, "the line is longer than 1,048,576 pixels: a limit of this runtime");
        n = 0;
        EACH_CLIP(t, n += fle_r_line(&t.s, CL_, a.x0, a.y0, b.x0, b.y0, col, dc->rop2));
        {
            fle_rrect bb = { a.x0 < b.x0 ? a.x0 : b.x0, a.y0 < b.y0 ? a.y0 : b.y0, (a.x0 > b.x0 ? a.x0 : b.x0) + 1, (a.y0 > b.y0 ? a.y0 : b.y0) + 1 };

            target_wrote(api, &t, bb, n);
        }
    }
    dc->pos[0] = x;
    dc->pos[1] = y;
    R[0] = 1;
}

/* gdi32!Rectangle: BOOL f(HDC hdc rcx, int left edx, int top r8d, int right r9d, int bottom [5]). Learn (Rectangle): "draws a rectangle. The rectangle is outlined by using the current pen and filled by using
 * the current brush"; "The current position is neither used nor updated by Rectangle"; "The rectangle that is drawn excludes the bottom and right edges"; "If a PS_NULL pen is used, the dimensions of the rectangle
 * are 1 pixel less in height and 1 pixel less in width"; nonzero on success. Answered: a one-pixel cosmetic pen: the outline is the pixels of the border of [left, right) x [top, bottom) (the page's exclusion of the
 * bottom and right edges, read as the pen covering the rows left, top, right - 1, bottom - 1 and the columns of them), the interior [left + 1, right - 1) x [top + 1, bottom - 1) is filled with the brush (solid,
 * stock, hollow = nothing: a PS_NULL pen fills [left, right - 1) x [top, bottom - 1) as the page says "1 pixel less"); the outline is drawn with the pen colour and the DC's drawing mode, the fill is PATCOPY. A
 * rectangle with right <= left or bottom <= top draws nothing (S-rectangle-empty: the page does not say). */
void fle_imp_Rectangle(void)
{
    static const char api[] = "Rectangle";
    int32_t l = (int32_t)(uint32_t)R[2], t_ = (int32_t)(uint32_t)R[8], r = (int32_t)(uint32_t)R[9], b = (int32_t)(uint32_t)fle_arg(5);
    target t;
    gdi_dc *dc;
    fle_rrect q, in;
    int inv, hollow;
    uint32_t pen, fill;
    uint64_t n = 0;
    char pre[110];

    fle_gdi_check_thread(api);
    snprintf(pre, sizeof pre, "drawing a rectangle (Rectangle (%d, %d, %d, %d))", (int)l, (int)t_, (int)r, (int)b);
    get_target(api, pre, R[1], "hdc", &t);
    dc = target_attrs(R[1]);
    mode_check(api, pre, dc, "hdc");
    fill_check(api, pre, &t, "hdc");
    pen = pen_color(api, pre, dc->pen, &inv);
    fill = brush_color(api, pre, dc->brush, 0, &hollow);
    R[0] = 1;
    if (r <= l || b <= t_ || !target_live(api, &t)) return;
    q = dev_rect(dc, &t, l, t_, (int64_t)r - l, (int64_t)b - t_);
    if (inv) {
        in = q;
        in.x1 -= 1;
        in.y1 -= 1;
        if (!hollow) EACH_CLIP(t, n += fle_r_rop_rect(&t.s, CL_, in, FLE_R_PATCOPY_IDX, fill));
    } else {
        fle_rrect top = { q.x0, q.y0, q.x1, q.y0 + 1 }, bot = { q.x0, q.y1 - 1, q.x1, q.y1 }, lef = { q.x0, q.y0 + 1, q.x0 + 1, q.y1 - 1 }, rig = { q.x1 - 1, q.y0 + 1, q.x1, q.y1 - 1 };

        in.x0 = q.x0 + 1;
        in.y0 = q.y0 + 1;
        in.x1 = q.x1 - 1;
        in.y1 = q.y1 - 1;
        if (!hollow) EACH_CLIP(t, n += fle_r_rop_rect(&t.s, CL_, in, FLE_R_PATCOPY_IDX, fill));
        EACH_CLIP(t, n += fle_r_mix_rect(&t.s, CL_, top, dc->rop2, pen));
        if (q.y1 - q.y0 > 1) EACH_CLIP(t, n += fle_r_mix_rect(&t.s, CL_, bot, dc->rop2, pen));
        EACH_CLIP(t, n += fle_r_mix_rect(&t.s, CL_, lef, dc->rop2, pen));
        if (q.x1 - q.x0 > 1) EACH_CLIP(t, n += fle_r_mix_rect(&t.s, CL_, rig, dc->rop2, pen));
    }
    target_wrote(api, &t, q, n);
}

/* user32!FrameRect: int f(HDC hDC rcx, const RECT *lprc rdx, HBRUSH hbr r8). Learn (FrameRect): "draws a border around the specified rectangle by using the specified brush. The width and height of the border are
 * always one logical unit"; "If the bottom member of the RECT structure is less than the top member, or if the right member is less than the left member, the function does not draw the rectangle"; the brush "must
 * have been created by using the CreateHatchBrush, CreatePatternBrush, or CreateSolidBrush function, or retrieved by using the GetStockObject function" (a system colour index plus 1 is not a brush here: it stops with
 * the other forms the page does not allow); nonzero on success. Answered: the border of [left, right) x [top, bottom) is the rows top and bottom - 1 and the columns left and right - 1 inside it, in the brush colour
 * (PATCOPY). A rectangle of width or height 0 draws nothing (the page's rule says only "less than"; for equal edges the border has no pixel: a READING). */
void fle_imp_FrameRect(void)
{
    static const char api[] = "FrameRect";
    uint64_t prc = R[2], hbr = R[8];
    int32_t l, t_, rt, b;
    uint32_t col;
    int hollow;
    target t;
    gdi_dc *dc;
    fle_rrect q;
    uint64_t n = 0;
    char pre[110];

    fle_gdi_check_thread(api);
    snprintf(pre, sizeof pre, "drawing a frame (FrameRect hbr 0x%" PRIx64 ")", hbr);
    get_target(api, pre, R[1], "hDC", &t);
    dc = target_attrs(R[1]);
    mode_check(api, pre, dc, "hDC");
    fill_check(api, pre, &t, "hDC");
    if (prc == 0) pixel_stop(api, pre, "lprc is NULL: the page does not define it");
    if (hbr <= FLE_GDI_SYS_COLOR_COUNT) pixel_stop(api, pre, "hbr is a system colour index plus 1 or NULL: the page asks for a brush");
    col = brush_color(api, pre, hbr, 0, &hollow);
    l = (int32_t)(uint32_t)fle_ld(prc, 4);
    t_ = (int32_t)(uint32_t)fle_ld(prc + 4u, 4);
    rt = (int32_t)(uint32_t)fle_ld(prc + 8u, 4);
    b = (int32_t)(uint32_t)fle_ld(prc + 12u, 4);
    R[0] = 1;
    if (hollow || rt <= l || b <= t_ || !target_live(api, &t)) return;
    q = dev_rect(dc, &t, l, t_, (int64_t)rt - l, (int64_t)b - t_);
    {
        fle_rrect top = { q.x0, q.y0, q.x1, q.y0 + 1 }, bot = { q.x0, q.y1 - 1, q.x1, q.y1 }, lef = { q.x0, q.y0 + 1, q.x0 + 1, q.y1 - 1 }, rig = { q.x1 - 1, q.y0 + 1, q.x1, q.y1 - 1 };

        EACH_CLIP(t, n += fle_r_rop_rect(&t.s, CL_, top, FLE_R_PATCOPY_IDX, col));
        if (q.y1 - q.y0 > 1) EACH_CLIP(t, n += fle_r_rop_rect(&t.s, CL_, bot, FLE_R_PATCOPY_IDX, col));
        EACH_CLIP(t, n += fle_r_rop_rect(&t.s, CL_, lef, FLE_R_PATCOPY_IDX, col));
        if (q.x1 - q.x0 > 1) EACH_CLIP(t, n += fle_r_rop_rect(&t.s, CL_, rig, FLE_R_PATCOPY_IDX, col));
    }
    target_wrote(api, &t, q, n);
}

/* gdi32!CreatePen: HPEN f(int iStyle ecx, int cWidth edx, COLORREF color r8d) (a row of decision 0126, appended: FLEngine imports it, no other module does). Learn (CreatePen): the styles are PS_SOLID .. PS_INSIDEFRAME; "If nWidth is zero, the pen is a
 * single pixel wide"; "CreatePen returns a pen with the specified width but with the PS_SOLID style if you specify a width greater than one for the following styles: PS_DASH, PS_DOT, PS_DASHDOT, PS_DASHDOTDOT"; NULL on
 * failure. Answered: style 0 to 6, width not negative, colour with a zero high byte: the pen of CreatePenIndirect with the same arguments, the dashed styles of width above 1 become PS_SOLID as the page says. */
void fle_imp_CreatePen(void)
{
    static const char api[] = "CreatePen";
    int32_t style = (int32_t)(uint32_t)R[1], width = (int32_t)(uint32_t)R[2];
    uint32_t color = (uint32_t)R[8];
    uint64_t slot;
    gdi_obj *o;
    char why[140];

    fle_gdi_check_thread(api);
    if (style < 0 || style > FLE_GDI_PS_INSIDEFRAME) {
        snprintf(why, sizeof why, "iStyle is %d: only PS_SOLID to PS_INSIDEFRAME (0 to 6) are answered", (int)style);
        fle_stop(api, why);
    }
    if (width < 0) fle_stop(api, "cWidth is negative: the page does not define it");
    if (!color_plain(color, &color)) fle_stop(api, "color has a high-order byte other than 0 and 2 (PALETTEINDEX needs a palette): the page does not describe it");
    if (width > 1 && style >= 1 && style <= 4) style = FLE_GDI_PS_SOLID;
    slot = new_slot(api);
    o = &objs[slot];
    o->u[0] = (uint32_t)style;
    o->u[1] = (uint32_t)width;
    o->u[2] = color;
    issue(o, FLE_GDI_KIND_PEN);
    pen_live++;
    R[0] = slot_handle(slot);
}

/* ---- GetDIBits and SetDIBits ---- */

/* gdi32!SetDIBits: int f(HDC hdc rcx, HBITMAP hbm rdx, UINT start r8d, UINT cLines r9d, const VOID *lpBits [5], const BITMAPINFO *lpbmi [6], UINT ColorUse [7]). Learn (SetDIBits): "sets the pixels in a compatible
 * bitmap (DDB) using the color data found in the specified DIB"; hbm "must be a compatible bitmap (DDB)" and "must not be selected into a device context when the application calls this function"; start is "the starting
 * scan line for the device-independent color data in the array pointed to by the lpvBits parameter", cLines "the number of scan lines found in the array"; "The device context ... is used only if the DIB_PAL_COLORS
 * constant is set"; the return value is "the number of scan lines copied", zero on failure (ERROR_INVALID_PARAMETER); "The origin for bottom-up DIBs is the lower-left corner of the bitmap; the origin for top-down
 * DIBs is the upper-left corner of the bitmap". Answered: a device-dependent bitmap that is not selected into a DC (selected: 0 and last error 87, S-selected-ddb: the page says "must not"), a 1-, 4-, 8-, 24- or 32-bit DIB
 * of DIB_RGB_COLORS; the scan lines [start, start + cLines) of the DIB (counted from the lower-left corner for a bottom-up one, from the top for a top-down one, the page's origin sentence) are converted to the bitmap's
 * format and stored at the same rows of the bitmap (the rows of the bitmap are counted from its top: a READING: the bitmap has the orientation of its device, top-down); the width of the DIB and of the bitmap must
 * be equal (S-setdibits-width: the page does not say what a different width does). The return value is the number of scan lines stored. */
void fle_imp_SetDIBits(void)
{
    static const char api[] = "SetDIBits";
    uint64_t hbm = R[2], bits = fle_arg(5), pbmi = fle_arg(6);
    uint32_t start = (uint32_t)R[8], nl = (uint32_t)R[9], usage = (uint32_t)fle_arg(7), ct[256];
    int32_t hdr_h;
    fle_rsurf v, img;
    gdi_obj *o;
    uint32_t done = 0;
    char pre[160];

    fle_gdi_check_thread(api);
    snprintf(pre, sizeof pre, "SetDIBits of bitmap 0x%" PRIx64 " (start %u, cLines %u, usage %u)", hbm, (unsigned)start, (unsigned)nl, (unsigned)usage);
    o = ddb_arg(api, pre, hbm, &v);
    if (!fle_r_direct(&v) && v.bpp != 1) pixel_stop(api, pre, "the bitmap has a format that cannot be written");
    dib_image(api, pre, pbmi, bits, usage, start, nl, 0, &img, ct, &hdr_h);
    R[0] = 0;
    if (o->sel) {
        fle_set_last_error(FLE_GDI_ERROR_INVALID_PARAMETER_);
        return;
    }
    if (img.width != v.width) pixel_stop(api, pre, "the width of the DIB differs from the width of the bitmap: the page does not say what is stored");
    if ((uint64_t)start + nl > img.height) {
        fle_set_last_error(FLE_GDI_ERROR_INVALID_PARAMETER_);
        return;
    }
    for (uint32_t k = 0; k < nl; k++) {
        uint32_t dib_line = start + k;
        uint32_t src_row = img.bottom_up ? img.height - 1u - dib_line : dib_line;
        uint32_t dst_row = img.bottom_up ? v.height - 1u - dib_line : dib_line;     /* the bitmap's rows from its top; a bottom-up DIB line n is the n-th row from the bottom of the bitmap (the page's origin sentence) */

        if (dst_row >= v.height) break;
        for (uint32_t x = 0; x < v.width; x++) {
            uint32_t c = fle_r_get(&img, x, src_row);

            if (v.bpp == 1) {
                uint8_t *p = fle_r_row(&v, dst_row) + (x >> 3);
                uint8_t m = (uint8_t)(0x80u >> (x & 7u));

                if (c == 0xFFFFFFu) *p |= m;        /* S-ddb-mono-convert: white is 1, every other colour is 0 (the page StretchBlt: "pixels that match the background color to white ... all other pixels to black" is the opposite rule for the DC's colours) */
                else *p &= (uint8_t)~m;
            } else {
                fle_r_put(&v, x, dst_row, c);
            }
        }
        done++;
    }
    R[0] = done;
}

/* gdi32!GetDIBits: int f(HDC hdc rcx, HBITMAP hbm rdx, UINT start r8d, UINT cLines r9d, LPVOID lpvBits [5], LPBITMAPINFO lpbmi [6], UINT usage [7]). Learn (GetDIBits): "retrieves the bits of the specified compatible
 * bitmap and copies them into a buffer as a DIB using the specified format"; hbm "must be a compatible bitmap (DDB)" and "must not be selected into a device context"; "If the lpvBits parameter is NULL, the function
 * passes the dimensions and format of the bitmap to the BITMAPINFO structure"; "If the lpvBits parameter is non-NULL and the function succeeds, the return value is the number of scan lines copied from the
 * bitmap"; "If the lpvBits parameter is NULL and GetDIBits successfully fills the BITMAPINFO structure, the return value is nonzero"; "If lpvBits is NULL and the bit count member of BITMAPINFO is initialized to zero,
 * GetDIBits fills in a BITMAPINFOHEADER ... without the color table"; "If the lpvBits parameter is a valid pointer, the first six members of the BITMAPINFOHEADER structure must be initialized to specify the size and
 * format of the DIB"; "If the requested format for the DIB matches its internal format, the RGB values for the bitmap are copied. If the requested format doesn't match the internal format, a color table is
 * synthesized" (1 bpp: black and white; 4 and 8 bpp: tables the pages describe in words: they stop; 24 bpp: none). Answered: a DDB not selected into a DC; (1) lpvBits NULL and biBitCount 0: the header is filled
 * (biSize 40, biWidth, biHeight = the positive height of a bottom-up image, biPlanes 1, biBitCount of the bitmap, BI_RGB, biSizeImage 0, the rest 0) and 1 is returned; (2) lpvBits given and a requested 24- or 32-bit
 * BI_RGB DIB (biBitCount 24 or 32, biPlanes 1, biWidth equal to the bitmap's, biHeight not 0): the lines [start, start + cLines) are written (bottom-up for a positive height, top-down for a negative one) and their
 * number returned; a monochrome bitmap converted to a 24- or 32-bit DIB gives black (bit 0) and white (bit 1) (S-ddb-mono-convert). Every other request stops (the colour tables of 4 and 8 bits, a different width). */
void fle_imp_GetDIBits(void)
{
    static const char api[] = "GetDIBits";
    uint64_t hbm = R[2], bits = fle_arg(5), pbmi = fle_arg(6);
    uint32_t start = (uint32_t)R[8], nl = (uint32_t)R[9], usage = (uint32_t)fle_arg(7);
    fle_rsurf v;
    gdi_obj *o;
    char pre[160];

    fle_gdi_check_thread(api);
    snprintf(pre, sizeof pre, "GetDIBits of bitmap 0x%" PRIx64 " (start %u, cLines %u, lpvBits %s, usage %u)", hbm, (unsigned)start, (unsigned)nl, bits ? "given" : "NULL", (unsigned)usage);
    o = ddb_arg(api, pre, hbm, &v);
    if (pbmi == 0) pixel_stop(api, pre, "lpbmi is NULL: the page does not define it");
    if (usage != FLE_GDI_DIB_RGB_COLORS) pixel_stop(api, pre, "usage is not DIB_RGB_COLORS: DIB_PAL_COLORS needs the logical palette of the DC");
    if (o->sel) {
        fle_set_last_error(FLE_GDI_ERROR_INVALID_PARAMETER_);
        R[0] = 0;
        return;
    }
    if ((uint32_t)fle_ld(pbmi, 4) != FLE_GDI_BMIH_SIZE) pixel_stop(api, pre, "biSize is not 40: the core header is not modelled");
    if (bits == 0) {
        if ((uint32_t)fle_ld(pbmi + 14u, 2) != 0) pixel_stop(api, pre, "lpvBits is NULL and biBitCount is not 0: the synthesized colour table of the page is not modelled");
        fle_st(pbmi + 4u, 4, v.width);
        fle_st(pbmi + 8u, 4, v.height);                 /* a positive height: a bottom-up DIB, the form of a DDB's rows from the bottom (the page's lower-left origin) */
        fle_st(pbmi + 12u, 2, 1);
        fle_st(pbmi + 14u, 2, v.bpp);
        fle_st(pbmi + 16u, 4, 0);
        fle_st(pbmi + 20u, 4, 0);
        for (uint64_t k = 24; k < 40; k += 4) fle_st(pbmi + k, 4, 0);
        R[0] = 1;
        return;
    }
    {
        uint32_t bpp = (uint32_t)fle_ld(pbmi + 14u, 2), comp = (uint32_t)fle_ld(pbmi + 16u, 4), w = (uint32_t)fle_ld(pbmi + 4u, 4);
        int32_t h = (int32_t)(uint32_t)fle_ld(pbmi + 8u, 4);
        uint64_t stride;
        uint32_t rows, done = 0;

        if ((bpp != 24 && bpp != 32) || comp != FLE_GDI_BI_RGB || (uint32_t)fle_ld(pbmi + 12u, 2) != 1 || h == 0)
            pixel_stop(api, pre, "the requested DIB is not a 24- or 32-bit BI_RGB one of one plane: the colour tables synthesized for the other formats are described in words only");
        if (w != v.width) pixel_stop(api, pre, "biWidth differs from the width of the bitmap: the page does not say what is copied");
        stride = (((uint64_t)w * bpp + 31u) & ~UINT64_C(31)) >> 3;
        rows = h < 0 ? (uint32_t)(-(int64_t)h) : (uint32_t)h;
        if ((uint64_t)start + nl > v.height || nl > rows) {
            fle_set_last_error(FLE_GDI_ERROR_INVALID_PARAMETER_);
            R[0] = 0;
            return;
        }
        (void)fle_addr(bits, 1, 1);
        (void)fle_addr(bits + stride * (h > 0 ? (uint64_t)start + nl : nl) - 1u, 1, 1);
        for (uint32_t k = 0; k < nl; k++) {
            uint32_t bm_row = start + k;                        /* the bitmap's row from its top; a bottom-up DIB's line n is the n-th row from the bottom */
            uint32_t src_row = h > 0 ? v.height - 1u - bm_row : bm_row;
            uint64_t line = h > 0 ? (uint64_t)bm_row : (uint64_t)k;     /* memory line of the buffer */
            uint64_t dst = bits + line * stride;

            if (src_row >= v.height) break;
            for (uint32_t x = 0; x < v.width; x++) {
                uint32_t c = fle_r_get(&v, x, src_row);

                fle_st(dst + (uint64_t)x * (bpp / 8u), 1, (c >> 16) & 0xFFu);
                fle_st(dst + (uint64_t)x * (bpp / 8u) + 1u, 1, (c >> 8) & 0xFFu);
                fle_st(dst + (uint64_t)x * (bpp / 8u) + 2u, 1, c & 0xFFu);
                if (bpp == 32) fle_st(dst + (uint64_t)x * 4u + 3u, 1, 0);
            }
            done++;
        }
        R[0] = done;
    }
}

/* gdi32!MaskBlt: BOOL f(HDC hdcDest rcx, int xDest edx, int yDest r8d, int width r9d, int height [5], HDC hdcSrc [6], int xSrc [7], int ySrc [8], HBITMAP hbmMask [9], int xMask [10], int yMask [11], DWORD rop [12]).
 * Learn (MaskBlt): "combines the color data for the source and destination bitmaps using the specified mask and raster operation"; rop: "The background raster operation code is stored in the high-order byte of the
 * high-order word of this value; the foreground raster operation code is stored in the low-order byte of the high-order word of this value; the low-order word of this value is ignored" (MAKEROP4); "A value of 1 in
 * the mask specified by hbmMask indicates that the foreground raster operation code ... should be applied at that location. A value of 0 ... the background"; hdcSrc "must be zero if the dwRop parameter specifies a
 * raster operation that does not include a source"; "If the raster operations require a source, the mask rectangle must cover the source rectangle. If it does not, the function will fail. If the raster operations do
 * not require a source, the mask rectangle must cover the destination rectangle. If it does not, the function will fail"; "If the mask bitmap is not a monochrome bitmap, an error occurs"; "If no mask bitmap is supplied,
 * this function behaves exactly like BitBlt, using the foreground raster operation code"; "MaskBlt uses device-dependent bitmaps"; nonzero on success. Answered (decision 0126): a monochrome device-dependent mask (or NULL
 * = BitBlt with the foreground code), the destination and the source as BitBlt; failure (0, last error 87: S-err-param) when the mask is not monochrome or does not cover the rectangle; a source rectangle partly outside
 * the source leaves the destination unchanged there as in BitBlt (S-source-outside). */
void fle_imp_MaskBlt(void)
{
    static const char api[] = "MaskBlt";
    int32_t x = (int32_t)(uint32_t)R[2], y = (int32_t)(uint32_t)R[8], w = (int32_t)(uint32_t)R[9], h = (int32_t)(uint32_t)fle_arg(5);
    uint64_t hsrc = fle_arg(6), hmask = fle_arg(9);
    int32_t xs = (int32_t)(uint32_t)fle_arg(7), ys = (int32_t)(uint32_t)fle_arg(8), xm = (int32_t)(uint32_t)fle_arg(10), ym = (int32_t)(uint32_t)fle_arg(11);
    uint32_t rop = (uint32_t)fle_arg(12), fg_i, bg_i, pat = 0;
    uint32_t fgcode = rop & 0x00FF0000u, bgcode = rop & 0xFF000000u;
    target td, ts;
    gdi_dc *dd, *sd;
    gdi_obj *mo = NULL;
    fle_rsurf mv;
    fle_rrect dr, sr, in, dest_in;
    int us;
    uint64_t n;
    char pre[240];

    fle_gdi_check_thread(api);
    snprintf(pre, sizeof pre, "masked pixel transfer (MaskBlt rop 0x%x, %d x %d to (%d, %d) from (%d, %d), mask 0x%" PRIx64 " at (%d, %d))", (unsigned)rop, (int)w, (int)h, (int)x, (int)y, (int)xs, (int)ys, hmask, (int)xm, (int)ym);
    /* "the low-order word of this value is ignored, and should be zero": the guest passes 0x0029 there (wall 58; the word of the foreground code 0xAA with its low word 0x0029: the guest's MAKEROP4 carries the low word of the code of the page); it is ignored as the page says */
    fg_i = fgcode >> 16;
    bg_i = bgcode >> 24;
    us = fle_r_idx_uses_source(fg_i) || fle_r_idx_uses_source(bg_i);
    get_target(api, pre, R[1], "hdcDest", &td);
    dd = target_attrs(R[1]);
    mode_check(api, pre, dd, "hdcDest");
    fill_check(api, pre, &td, "hdcDest");
    if (us) {
        get_target(api, pre, hsrc, "hdcSrc", &ts);
        sd = target_attrs(hsrc);
        mode_check(api, pre, sd, "hdcSrc");
        source_ok(api, pre, &ts, "hdcSrc");
    } else {
        memset(&ts, 0, sizeof ts);
        sd = dd;
    }
    if (us && ts.s.bpp == 1 && td.s.bpp != 1) {
        ts.s.conv_set = 1;
        ts.s.conv_fg = dd->text;
        ts.s.conv_bg = dd->bk;
    }
    if (us && td.s.bpp == 1 && ts.s.bpp != 1) {
        td.s.mono_key_set = 1;
        td.s.mono_key = sd->bk;
    }
    if ((fle_r_idx_uses_pattern(fg_i) || fle_r_idx_uses_pattern(bg_i))) {
        int hollow;

        pat = brush_color(api, pre, dd->brush, 0, &hollow);
        if (hollow) pixel_stop(api, pre, "the raster operation uses the brush and the brush is hollow: not modelled for MaskBlt");
    }
    if (hmask == 0) {
        /* "If no mask bitmap is supplied, this function behaves exactly like BitBlt, using the foreground raster operation code" */
        blit_call(api, pre, R[1], x, y, w, h, hsrc, xs, ys, fle_r_rop3_code(fg_i), 0, 0, 0);
        return;
    }
    mo = live_obj(hmask);
    if (!mo || mo->kind != FLE_GDI_KIND_BITMAP) pixel_stop(api, pre, "hbmMask is not a live bitmap of this runtime");
    if (mo->fmt == FLE_GDI_BMP_DIB || mo->bm[3] != 1 || mo->bm[2] != 1) {
        fle_set_last_error(FLE_GDI_ERROR_INVALID_PARAMETER_);      /* "If the mask bitmap is not a monochrome bitmap, an error occurs" */
        R[0] = 0;
        return;
    }
    if (!ddb_view(mo, &mv)) pixel_stop(api, pre, "out of memory for the pixels of a device-dependent bitmap");
    R[0] = 1;
    if (w < 0 || h < 0 || w == 0 || h == 0) {
        if (w < 0 || h < 0) {
            fle_set_last_error(FLE_GDI_ERROR_INVALID_PARAMETER_);
            R[0] = 0;
        }
        return;
    }
    if (xm < 0 || ym < 0 || (int64_t)xm + w > (int64_t)mv.width || (int64_t)ym + h > (int64_t)mv.height) {
        fle_set_last_error(FLE_GDI_ERROR_INVALID_PARAMETER_);      /* "the mask rectangle must cover the source rectangle ... If it does not, the function will fail" */
        R[0] = 0;
        return;
    }
    if (!target_live(api, &td)) return;
    dr = dev_rect(dd, &td, x, y, w, h);
    if (us) {
        fle_rrect srcbounds = ts.is_window ? ts.clip : fle_r_bounds(&ts.s);

        sr = dev_rect(sd, &ts, xs, ys, w, h);
        in = sr;
        fle_r_clip(&in, &srcbounds);
        if (fle_r_empty(&in)) return;
        dest_in.x0 = dr.x0 + (in.x0 - sr.x0);
        dest_in.y0 = dr.y0 + (in.y0 - sr.y0);
        dest_in.x1 = dest_in.x0 + (in.x1 - in.x0);
        dest_in.y1 = dest_in.y0 + (in.y1 - in.y0);
        n = 0;
        EACH_CLIP(td, n += fle_r_maskblt(&td.s, CL_, dest_in.x0, dest_in.y0, in.x1 - in.x0, in.y1 - in.y0, &ts.s, in.x0, in.y0, &mv, (int64_t)xm + (in.x0 - sr.x0), (int64_t)ym + (in.y0 - sr.y0), fg_i, bg_i, pat));
        dr = dest_in;
    } else {
        n = 0;
        EACH_CLIP(td, n += fle_r_maskblt(&td.s, CL_, dr.x0, dr.y0, w, h, NULL, 0, 0, &mv, xm, ym, fg_i, bg_i, pat));
    }
    target_wrote(api, &td, dr, n);
}
GDI_STOP(CreateDIBitmap, GDI_WHY_PIXEL)
/* Decision 0129 (stop condition (d'), the first call of a text-drawing API): the stop is the same named stop, but the harness may first record who called and with what (fle_text_hook, weak: a link without
 * it stops exactly as before). The record needs the font that is selected into the DC: fle_gdi_dc_font returns the handle (0 when hdc is not a DC of this table); fle_gdi_font_logfont gives its LOGFONTW
 * (a stock or default font has none: 0). Nothing here changes what the call answers: it still stops by name. */
void fle_text_hook(const char *api, uint64_t hdc) __attribute__((weak));
uint64_t fle_gdi_dc_font(uint64_t hdc)
{
    gdi_dc *dc = NULL;

    switch (dc_class(hdc, &dc)) {
    case DCK_SCREEN: return screen_state("fle_gdi_dc_font", hdc)->font;
    case DCK_MEM: return dc->font;
    default: return 0;
    }
}
#define GDI_TEXT_STOP(Name, Why) void fle_imp_##Name(void) { if (fle_text_hook) fle_text_hook(#Name, R[1]); fle_stop(#Name, Why " (decision 0111)"); }
GDI_TEXT_STOP(ExtTextOutW, GDI_WHY_GLYPH)
GDI_TEXT_STOP(GetTextMetricsW, GDI_WHY_TEXT)
GDI_TEXT_STOP(GetTextExtentPoint32W, GDI_WHY_TEXT)
GDI_TEXT_STOP(GetTextExtentPointW, GDI_WHY_TEXT)
/* gdi32!RectVisible: BOOL f(HDC hdc rcx, const RECT *lprect rdx). Decision 0191 (PROVISIONAL; decision 0284: no longer opt-in -- until then only a build with the layer-4 marker row of harness/seeds/ia2-imports.txt, else the stop of decision 0111;
 * wall ia2-W27: Options > Manage accounts). Learn (wingdi.h, "RectVisible function"): "determines whether any part of the specified rectangle lies within the clipping region of a device context"; the rectangle is in
 * logical coordinates; the return value is nonzero when "some portion of the rectangle lies within the clipping region", zero otherwise. The clipping region of a DC here is what BitBlt and FillRect use: the visible
 * rectangle of the target (or the pieces of an application-defined clipping region inside it, decision 0123/0142). An empty or inverted rectangle has no part inside: 0 (a reading); a window that is not visible: 0
 * (nothing is drawn there); lprect NULL stops (the page does not define it). The last error is left alone. */
void fle_imp_RectVisible(void)
{
    static const char api[] = "RectVisible";
    uint64_t prc = R[2];
    int32_t l, tp, rt, b;
    int vis = 0;
    target t;
    gdi_dc *dc;
    fle_rrect r;
    char pre[120] = "testing a rectangle against the clipping region";

    fle_gdi_check_thread(api);
    get_target(api, pre, R[1], "hdc", &t);
    dc = target_attrs(R[1]);
    mode_check(api, pre, dc, "hdc");
    if (prc == 0) pixel_stop(api, pre, "lprect is NULL: the page does not define it");
    l = (int32_t)(uint32_t)fle_ld(prc, 4);
    tp = (int32_t)(uint32_t)fle_ld(prc + 4u, 4);
    rt = (int32_t)(uint32_t)fle_ld(prc + 8u, 4);
    b = (int32_t)(uint32_t)fle_ld(prc + 12u, 4);
    R[0] = 0;
    if (b <= tp || rt <= l || (t.is_window && t.hidden)) return;
    r = dev_rect(dc, &t, l, tp, (int64_t)rt - l, (int64_t)b - tp);
    EACH_CLIP(t, { fle_rrect x = r; fle_r_clip(&x, CL_); if (!fle_r_empty(&x)) vis = 1; });
    R[0] = (uint64_t)vis;
}
GDI_STOP(GetRandomRgn, GDI_WHY_REGION)
GDI_STOP(GetNearestPaletteIndex, GDI_WHY_PAL)
GDI_STOP(ResizePalette, GDI_WHY_PAL)
GDI_STOP(GetSystemPaletteEntries, GDI_WHY_PAL)
GDI_STOP(GetDCOrgEx, GDI_WHY_ORG)
GDI_STOP(GetEnhMetaFileHeader, GDI_WHY_META)
GDI_STOP(PlayEnhMetaFile, GDI_WHY_META)
GDI_STOP(GetEnhMetaFileBits, GDI_WHY_META)
GDI_STOP(GetEnhMetaFilePaletteEntries, GDI_WHY_META)
GDI_STOP(GetWinMetaFileBits, GDI_WHY_META)
GDI_STOP(GetEnhMetaFileDescriptionW, GDI_WHY_META)
GDI_STOP(DeleteEnhMetaFile, GDI_WHY_META)
GDI_STOP(SetEnhMetaFileBits, GDI_WHY_META)
GDI_STOP(CopyEnhMetaFileW, GDI_WHY_META)
GDI_STOP(SetWinMetaFileBits, GDI_WHY_META)
GDI_STOP(DrawEdge, GDI_WHY_DRAW)
GDI_STOP(DrawFocusRect, GDI_WHY_DRAW)
GDI_STOP(DrawFrameControl, GDI_WHY_DRAW)
GDI_TEXT_STOP(DrawTextW, GDI_WHY_GLYPH)
GDI_TEXT_STOP(DrawTextExW, GDI_WHY_GLYPH)
GDI_STOP(DrawIcon, GDI_WHY_DRAW)
GDI_STOP(DrawIconEx, GDI_WHY_DRAW)
GDI_STOP(CreateIcon, GDI_WHY_PIXEL)
GDI_STOP(LoadBitmapW, GDI_WHY_PIXEL)

static void put_u32_(uint64_t a, uint32_t v) { fle_st(a, 4, v); }

/* ---- Regions (decision 0123 walls 46-49, PROVISIONAL; STATE ONLY: nothing is drawn) ----
 * A region is a set of pixels held as non-overlapping rectangles in the normal form of Y-X bands (rectangles sorted by top and then by left; the rectangles of one band have the same top and bottom, do not touch each other horizontally, and a
 * band is merged with the band below it when the two have the same horizontal spans and touch): two regions with the same pixels have the same rectangles, so EqualRgn compares the lists. (Decision 0229: there is no limit on the number of rectangles any more; decision 0123 had 64, a limit of this environment,
 * not a Windows rule: a result with more rectangles stops by name). Learn (the page of each API): CreateRectRgn and CreateRectRgnIndirect return "the handle to the region" (NULL on failure), "Region coordinates are represented as 27-bit signed integers",
 * "The region will be exclusive of the bottom and right edges"; DeleteObject frees it; GetRgnBox copies the bounding rectangle and returns NULLREGION (1), SIMPLEREGION (2, "a single rectangle") or COMPLEXREGION (3, "more than a single
 * rectangle"), 0 for an invalid region; SetRectRgn, OffsetRgn (returns the complexity, ERROR 0 "region is unaffected"), RectInRegion ("any part of the specified rectangle lies within the boundaries of the region"; "The lower and right edges of the
 * rectangle are not included"), PtInRegion, EqualRgn ("A return value of ERROR means at least one of the region handles is invalid"), CombineRgn (RGN_AND 1, RGN_OR 2, RGN_XOR 3, RGN_DIFF 4, RGN_COPY 5: the numbers are those of wingdi.h,
 * UNOFFICIAL; hrgnDst "must exist before CombineRgn is called"). The empty region has no rectangles; its bounding box is (0, 0, 0, 0) (a choice: the page gives none, S-region). A region with a 27-bit coordinate out of range: NULL/0 (a choice: the page names
 * the range and not the failure). The logical coordinates are the device coordinates (MM_TEXT: no DC is an argument). The clip regions of a DC (SelectClipRgn, ExtSelectClipRgn, IntersectClipRect, ExcludeClipRect, GetClipRgn, RectVisible, GetRandomRgn),
 * CreatePolygonRgn, GetRegionData and FrameRgn stay named stops. */
#define RGN_MIN (-(1 << 26))
#define RGN_MAX ((1 << 26) - 1)
#define RGN_AND_  1
#define RGN_OR_   2
#define RGN_XOR_  3
#define RGN_DIFF_ 4
#define RGN_COPY_ 5

typedef gdi_crect rrect;
typedef struct { uint32_t n, cap; rrect *r; } rgnset;   /* decision 0229: a heap array that grows (it was rrect r[64]) */

static void rgn_reserve(rgnset *s, uint32_t n)
{
    if (n > s->cap) {
        uint32_t nc = s->cap ? s->cap : 4u;
        rrect *p;

        while (nc < n) nc *= 2u;
        p = (rrect *)realloc(s->r, (size_t)nc * sizeof *p);
        if (!p) fle_stop("region", "out of memory");
        s->r = p;
        s->cap = nc;
    }
}

static void rgn_free(rgnset *s)
{
    free(s->r);
    s->r = NULL;
    s->n = s->cap = 0;
}

static void rgn_set_rect_(rgnset *s, int32_t l, int32_t t, int32_t r, int32_t b)
{
    s->n = 0;
    if (r > l && b > t) {
        rgn_reserve(s, 1);
        s->n = 1;
        s->r[0].l = l;
        s->r[0].t = t;
        s->r[0].r = r;
        s->r[0].b = b;
    }
}

/* Decision 0174: a pointer per slot, the 1,028-byte set made when a region is made at the slot (zero-filled) and freed when the object is deleted (it was an array of 1,028 bytes for every slot up to the highest region). */
static rgnset **rgn_sets;                 /* indexed by the slot of the object (the same index as objs[]) */
static uint64_t rgn_sets_cap;

static rgnset *rgn_set_of(const char *api, uint64_t slot)
{
    if (slot >= rgn_sets_cap) {
        uint64_t nc = rgn_sets_cap ? rgn_sets_cap * 2 : 64;
        rgnset **n;

        while (nc <= slot) nc *= 2;
        n = (rgnset **)realloc(rgn_sets, nc * sizeof *n);
        if (!n) fle_stop(api, "out of memory");
        rgn_sets = n;
        memset(rgn_sets + rgn_sets_cap, 0, (nc - rgn_sets_cap) * sizeof *n);
        rgn_sets_cap = nc;
    }
    if (!rgn_sets[slot]) {
        rgn_sets[slot] = (rgnset *)calloc(1, sizeof(rgnset));   /* decision 0229: the header only; the rectangles are a heap array of the set */
        if (!rgn_sets[slot]) fle_stop(api, "out of memory");
    }
    return rgn_sets[slot];
}

static void rgn_forget(uint64_t slot)
{
    if (slot < rgn_sets_cap) {
        if (rgn_sets[slot]) free(rgn_sets[slot]->r);
        free(rgn_sets[slot]);
        rgn_sets[slot] = NULL;
    }
}

/* Decision 0174 (a measurement, read by the G3 harness with FLE_G3_LEAKSTATS=1): the sizes of the object table. o[0] objs_n (slots ever used), o[1] objs_live, o[2] objs_cap, o[3] deleted slots waiting for reuse, o[4] rgn_sets_cap, o[5] region sets held. */
void fle_gdi_table_stats(uint64_t o[6])
{
    uint64_t k = 0;

    for (uint64_t i = 0; i < rgn_sets_cap; i++) k += rgn_sets[i] != NULL;
    o[0] = objs_n;
    o[1] = objs_live;
    o[2] = objs_cap;
    o[3] = dead_n;
    o[4] = rgn_sets_cap;
    o[5] = k;
}

static int cmp_i32(const void *a, const void *b) { int32_t x = *(const int32_t *)a, y = *(const int32_t *)b; return x < y ? -1 : x > y; }
static int cmp_rrect_tl(const void *a, const void *b)
{
    const rrect *x = (const rrect *)a, *y = (const rrect *)b;
    if (x->t != y->t) return x->t < y->t ? -1 : 1;
    return x->l < y->l ? -1 : x->l > y->l;
}

/* Decision 0229 (ref1, PROVISIONAL): the band builder of rgn_combine. Bands are given from top to bottom; each is a list of disjoint, non-touching, left-to-right x intervals. A rectangle of the
 * band directly above (the one whose bottom is this band's top) that has the same left and right is extended downwards (the merge rule of decision 0123: per rectangle, not per band); the others start new rectangles.
 * Only the rectangles of the band directly above can match, so the builder keeps their indices (prev) instead of scanning the whole list. */
typedef struct { rgnset *out; uint32_t *prev, *cur; uint32_t nprev, ncur, capp, capc; } rgn_bld;

static void rb_push(uint32_t **a, uint32_t *n, uint32_t *cap, uint32_t v)
{
    if (*n == *cap) {
        uint32_t nc = *cap ? *cap * 2u : 16u;
        uint32_t *p = (uint32_t *)realloc(*a, (size_t)nc * sizeof *p);
        if (!p) fle_stop("region", "out of memory");
        *a = p;
        *cap = nc;
    }
    (*a)[(*n)++] = v;
}

static void rb_band(rgn_bld *bld, int32_t y0, int32_t y1, const int32_t *xl, const int32_t *xr, uint32_t m)
{
    uint32_t i, q;
    uint32_t *t;

    bld->ncur = 0;
    for (i = 0; i < m; i++) {
        int found = 0;
        for (q = 0; q < bld->nprev; q++) {
            rrect *pr = &bld->out->r[bld->prev[q]];
            if (pr->l == xl[i] && pr->r == xr[i] && pr->b == y0) {
                pr->b = y1;
                rb_push(&bld->cur, &bld->ncur, &bld->capc, bld->prev[q]);
                found = 1;
                break;
            }
        }
        if (found) continue;
        rgn_reserve(bld->out, bld->out->n + 1u);
        bld->out->r[bld->out->n].l = xl[i];
        bld->out->r[bld->out->n].t = y0;
        bld->out->r[bld->out->n].r = xr[i];
        bld->out->r[bld->out->n].b = y1;
        rb_push(&bld->cur, &bld->ncur, &bld->capc, bld->out->n);
        bld->out->n++;
    }
    t = bld->prev; bld->prev = bld->cur; bld->cur = t;
    q = bld->capp; bld->capp = bld->capc; bld->capc = q;
    bld->nprev = bld->ncur;
}

static void rb_end(rgn_bld *bld)
{
    rgnset *o = bld->out;
    if (o->n > 1) qsort(o->r, o->n, sizeof o->r[0], cmp_rrect_tl);   /* by top then left (the merge can leave a taller rectangle before a shorter one of an earlier top) */
    free(bld->prev);
    free(bld->cur);
}

/* sorts the intervals (xl, xr) by left edge and merges the overlapping or touching ones; returns the count */
static uint32_t iv_merge(int32_t *xl, int32_t *xr, uint32_t nx)
{
    uint32_t i, m = 0;

    for (i = 1; i < nx; i++) {
        int32_t l = xl[i], r = xr[i];
        uint32_t j = i;

        while (j > 0 && xl[j - 1] > l) {
            xl[j] = xl[j - 1];
            xr[j] = xr[j - 1];
            j--;
        }
        xl[j] = l;
        xr[j] = r;
    }
    for (i = 0; i < nx; i++) {
        if (m > 0 && xl[i] <= xr[m - 1]) {
            if (xr[i] > xr[m - 1]) xr[m - 1] = xr[i];
        } else {
            xl[m] = xl[i];
            xr[m] = xr[i];
            m++;
        }
    }
    return m;
}

static uint32_t uniq_sorted_i32(int32_t *v, uint32_t n)
{
    uint32_t i, w = 0;

    qsort(v, n, sizeof *v, cmp_i32);
    for (i = 0; i < n; i++)
        if (w == 0 || v[w - 1] != v[i]) v[w++] = v[i];
    return w;
}

/* out = a (mode) b for RGN_AND_ / RGN_OR_ / RGN_XOR_ / RGN_DIFF_ (decision 0229: replaces the grid of cells of decision 0123 that was limited to 192 cells and 64 rectangles; the result is the same normal form). `out` may not alias a or b. */
static void rgn_combine(rgnset *out, const rgnset *a, const rgnset *b, int mode)
{
    uint32_t na = a->n, nb = b->n, tot = na + nb, i, k, ny = 0;
    int32_t *ys = (int32_t *)malloc(((size_t)tot * 2u + 2u) * sizeof *ys);
    int32_t *al = (int32_t *)malloc(((size_t)na + 1u) * sizeof *al), *ar = (int32_t *)malloc(((size_t)na + 1u) * sizeof *ar);
    int32_t *bl = (int32_t *)malloc(((size_t)nb + 1u) * sizeof *bl), *br = (int32_t *)malloc(((size_t)nb + 1u) * sizeof *br);
    int32_t *xs = (int32_t *)malloc(((size_t)tot * 2u + 2u) * sizeof *xs), *rl = (int32_t *)malloc(((size_t)tot * 2u + 2u) * sizeof *rl), *rr = (int32_t *)malloc(((size_t)tot * 2u + 2u) * sizeof *rr);
    rgn_bld bld;

    if (!ys || !al || !ar || !bl || !br || !xs || !rl || !rr) fle_stop("region", "out of memory");
    out->n = 0;
    memset(&bld, 0, sizeof bld);
    bld.out = out;
    for (i = 0; i < na; i++) { ys[ny++] = a->r[i].t; ys[ny++] = a->r[i].b; }
    for (i = 0; i < nb; i++) { ys[ny++] = b->r[i].t; ys[ny++] = b->r[i].b; }
    ny = uniq_sorted_i32(ys, ny);
    for (k = 0; k + 1 < ny; k++) {
        int32_t y0 = ys[k], y1 = ys[k + 1];
        uint32_t nal = 0, nbl = 0, nxs = 0, nres = 0, ia = 0, ib = 0, j;

        for (i = 0; i < na; i++)
            if (a->r[i].t <= y0 && a->r[i].b >= y1 && a->r[i].r > a->r[i].l) { al[nal] = a->r[i].l; ar[nal] = a->r[i].r; nal++; }
        for (i = 0; i < nb; i++)
            if (b->r[i].t <= y0 && b->r[i].b >= y1 && b->r[i].r > b->r[i].l) { bl[nbl] = b->r[i].l; br[nbl] = b->r[i].r; nbl++; }
        nal = iv_merge(al, ar, nal);
        nbl = iv_merge(bl, br, nbl);
        for (i = 0; i < nal; i++) { xs[nxs++] = al[i]; xs[nxs++] = ar[i]; }
        for (i = 0; i < nbl; i++) { xs[nxs++] = bl[i]; xs[nxs++] = br[i]; }
        nxs = uniq_sorted_i32(xs, nxs);
        for (j = 0; j + 1 < nxs; j++) {
            int in1, in2, in;

            while (ia < nal && ar[ia] <= xs[j]) ia++;
            while (ib < nbl && br[ib] <= xs[j]) ib++;
            in1 = ia < nal && al[ia] <= xs[j];
            in2 = ib < nbl && bl[ib] <= xs[j];
            in = mode == RGN_AND_ ? (in1 && in2) : mode == RGN_OR_ ? (in1 || in2) : mode == RGN_XOR_ ? (in1 != in2) : (in1 && !in2);
            if (!in) continue;
            if (nres > 0 && rr[nres - 1] == xs[j]) rr[nres - 1] = xs[j + 1];
            else { rl[nres] = xs[j]; rr[nres] = xs[j + 1]; nres++; }
        }
        rb_band(&bld, y0, y1, rl, rr, nres);
    }
    rb_end(&bld);
    free(ys); free(al); free(ar); free(bl); free(br); free(xs); free(rl); free(rr);
}

static int rgn_has(const rgnset *s, int32_t x, int32_t y)
{
    uint32_t i;

    for (i = 0; i < s->n; i++)
        if (x >= s->r[i].l && x < s->r[i].r && y >= s->r[i].t && y < s->r[i].b) return 1;
    return 0;
}

static uint64_t rgn_make(const char *api, int32_t l, int32_t t, int32_t r, int32_t b)
{
    uint64_t slot;
    gdi_obj *o;
    rgnset *s;

    if (l < RGN_MIN || l > RGN_MAX || t < RGN_MIN || t > RGN_MAX || r < RGN_MIN || r > RGN_MAX || b < RGN_MIN || b > RGN_MAX) return 0;
    slot = new_slot(api);
    o = &objs[slot];
    issue(o, FLE_GDI_KIND_REGION);
    s = rgn_set_of(api, slot);
    rgn_set_rect_(s, l, t, r, b);
    return slot_handle(slot);
}

static gdi_obj *rgn_get(uint64_t h, rgnset **set)
{
    int64_t i = slot_of(h);

    if (i < 0 || objs[i].state != ST_LIVE || objs[i].kind != FLE_GDI_KIND_REGION) return NULL;
    if (set) *set = rgn_set_of("region", (uint64_t)i);
    return &objs[i];
}

static uint32_t rgn_complexity(const rgnset *s) { return s->n == 0 ? FLE_GDI_NULLREGION : s->n == 1 ? FLE_GDI_SIMPLEREGION : 3u; }

void fle_imp_CreateRectRgn(void)
{
    static const char api[] = "CreateRectRgn";

    fle_gdi_check_thread(api);
    R[0] = rgn_make(api, (int32_t)(uint32_t)R[1], (int32_t)(uint32_t)R[2], (int32_t)(uint32_t)R[8], (int32_t)(uint32_t)R[9]);
}

void fle_imp_CreateRectRgnIndirect(void)
{
    static const char api[] = "CreateRectRgnIndirect";
    uint64_t p = R[1];

    fle_gdi_check_thread(api);
    if (p == 0) fle_stop(api, "lprect is NULL: the page gives no behaviour");
    R[0] = rgn_make(api, (int32_t)fle_ld(p, 4), (int32_t)fle_ld(p + 4, 4), (int32_t)fle_ld(p + 8, 4), (int32_t)fle_ld(p + 12, 4));
}

void fle_imp_SetRectRgn(void)
{
    static const char api[] = "SetRectRgn";
    rgnset *s;
    int32_t l = (int32_t)(uint32_t)R[2], t = (int32_t)(uint32_t)R[8], r = (int32_t)(uint32_t)R[9], b = (int32_t)(uint32_t)fle_arg(5);

    fle_gdi_check_thread(api);
    if (!rgn_get(R[1], &s) || l < RGN_MIN || l > RGN_MAX || t < RGN_MIN || t > RGN_MAX || r < RGN_MIN || r > RGN_MAX || b < RGN_MIN || b > RGN_MAX) {
        R[0] = 0;
        return;
    }
    rgn_set_rect_(s, l, t, r, b);
    R[0] = 1;
}

/* Decision 0126 (wall 93): for the window table (GetUpdateRgn): replaces the region `h` by the one rectangle (l, t, r, b), or by the empty region when the rectangle is empty. Returns 1, or 0 when `h` is not a live region of this file
 * or a coordinate is outside the range of regions. */
int fle_gdi_region_set_rect(uint64_t h, int32_t l, int32_t t, int32_t r, int32_t b)
{
    rgnset *s;

    if (!rgn_get(h, &s)) return 0;
    if (r > l && b > t && (l < RGN_MIN || l > RGN_MAX || t < RGN_MIN || t > RGN_MAX || r < RGN_MIN || r > RGN_MAX || b < RGN_MIN || b > RGN_MAX)) return 0;
    rgn_set_rect_(s, l, t, r, b);
    return 1;
}

/* Decision 0126 (wall 96): for the window table (ValidateRgn): 1 when the rectangle (l, t, r, b) lies entirely inside the region `h`, 0 when some of it does not, -1 when `h` is not a live region of this file. The rectangles of a
 * region are disjoint (Y-X bands), so the areas of their intersections with the rectangle add up to the area of the rectangle exactly when it is covered. An empty rectangle is covered. */
int fle_gdi_region_covers(uint64_t h, int32_t l, int32_t t, int32_t r, int32_t b)
{
    rgnset *s;
    uint32_t i;
    int64_t area, got = 0;

    if (!rgn_get(h, &s)) return -1;
    if (r <= l || b <= t) return 1;
    area = (int64_t)(r - l) * (int64_t)(b - t);
    for (i = 0; i < s->n; i++) {
        int32_t il = s->r[i].l > l ? s->r[i].l : l, it = s->r[i].t > t ? s->r[i].t : t, ir = s->r[i].r < r ? s->r[i].r : r, ib = s->r[i].b < b ? s->r[i].b : b;

        if (ir > il && ib > it) got += (int64_t)(ir - il) * (int64_t)(ib - it);
    }
    return got == area;
}

/* Decision 0126 (wall 91): for the window table (RedrawWindow with hrgnUpdate): 1 when `h` is a live region made by this file, with its bounding box (an empty region: 0, 0, 0, 0); 0 otherwise. */
int fle_gdi_region_bounds(uint64_t h, int32_t *l, int32_t *t, int32_t *r, int32_t *b)
{
    rgnset *s;
    uint32_t i;

    if (!rgn_get(h, &s)) return 0;
    *l = *t = *r = *b = 0;
    for (i = 0; i < s->n; i++) {
        if (i == 0 || s->r[i].l < *l) *l = s->r[i].l;
        if (i == 0 || s->r[i].t < *t) *t = s->r[i].t;
        if (i == 0 || s->r[i].r > *r) *r = s->r[i].r;
        if (i == 0 || s->r[i].b > *b) *b = s->r[i].b;
    }
    return 1;
}

int fle_gdi_region_pieces(uint64_t h, int32_t (*out)[4], uint32_t max, uint32_t *n)
{
    rgnset *s;
    uint32_t i;

    if (!rgn_get(h, &s)) return 0;
    *n = s->n;
    if (s->n > max) return -1;
    for (i = 0; i < s->n; i++) {
        out[i][0] = s->r[i].l;
        out[i][1] = s->r[i].t;
        out[i][2] = s->r[i].r;
        out[i][3] = s->r[i].b;
    }
    return 1;
}

static int rgn_cmp_i32(const void *a, const void *b)
{
    int32_t x = *(const int32_t *)a, y = *(const int32_t *)b;

    return x < y ? -1 : x > y;
}

int fle_gdi_region_set_pieces(uint64_t h, const int32_t (*p)[4], uint32_t n)
{
    rgnset *s;
    uint32_t i, k, nb, m = 0, ny = 0;
    int32_t ys[2 * 64];
    int32_t iv[64][2];
    int32_t prev[64][2];
    uint32_t nprev = 0;
    int32_t prev_t = 0, prev_b = 0;
    uint32_t first = 1;

    if (!rgn_get(h, &s)) return 0;
    if (n > 64u) return 0;
    for (i = 0; i < n; i++) {
        if (p[i][2] <= p[i][0] || p[i][3] <= p[i][1]) continue;
        if (p[i][0] < RGN_MIN || p[i][0] > RGN_MAX || p[i][1] < RGN_MIN || p[i][1] > RGN_MAX || p[i][2] < RGN_MIN || p[i][2] > RGN_MAX || p[i][3] < RGN_MIN || p[i][3] > RGN_MAX) return 0;
        ys[ny++] = p[i][1];
        ys[ny++] = p[i][3];
    }
    s->n = 0;
    if (ny == 0) return 1;
    qsort(ys, ny, sizeof ys[0], rgn_cmp_i32);
    for (i = 0, nb = 0; i < ny; i++) if (i == 0 || ys[i] != ys[i - 1]) ys[nb++] = ys[i];
    /* the bands between consecutive y edges: the x intervals of the pieces that cover the band, merged; a band with the same intervals as the one above it, and touching it, extends it (Y-X bands, as the other regions are kept) */
    for (k = 0; k + 1 < nb; k++) {
        uint32_t ni = 0, j;

        for (i = 0; i < n; i++) {
            if (p[i][2] <= p[i][0] || p[i][3] <= p[i][1]) continue;
            if (p[i][1] <= ys[k] && p[i][3] >= ys[k + 1]) {
                iv[ni][0] = p[i][0];
                iv[ni][1] = p[i][2];
                ni++;
            }
        }
        if (!ni) {
            first = 1;
            nprev = 0;
            continue;
        }
        for (i = 1; i < ni; i++) {                      /* insertion sort by left edge */
            int32_t a0 = iv[i][0], a1 = iv[i][1];

            for (j = i; j > 0 && iv[j - 1][0] > a0; j--) { iv[j][0] = iv[j - 1][0]; iv[j][1] = iv[j - 1][1]; }
            iv[j][0] = a0;
            iv[j][1] = a1;
        }
        for (i = 0, j = 0; i < ni; i++) {               /* merge overlapping or touching intervals */
            if (j > 0 && iv[i][0] <= iv[j - 1][1]) { if (iv[i][1] > iv[j - 1][1]) iv[j - 1][1] = iv[i][1]; }
            else { iv[j][0] = iv[i][0]; iv[j][1] = iv[i][1]; j++; }
        }
        ni = j;
        if (!first && nprev == ni && prev_b == ys[k]) {
            int same = 1;

            for (i = 0; i < ni; i++) if (prev[i][0] != iv[i][0] || prev[i][1] != iv[i][1]) same = 0;
            if (same) {
                for (i = 0; i < ni; i++) s->r[s->n - ni + i].b = ys[k + 1];
                prev_b = ys[k + 1];
                continue;
            }
        }
        rgn_reserve(s, s->n + ni);
        for (i = 0; i < ni; i++) {
            s->r[s->n + i].l = iv[i][0];
            s->r[s->n + i].t = ys[k];
            s->r[s->n + i].r = iv[i][1];
            s->r[s->n + i].b = ys[k + 1];
            prev[i][0] = iv[i][0];
            prev[i][1] = iv[i][1];
        }
        s->n += ni;
        nprev = ni;
        prev_t = ys[k];
        prev_b = ys[k + 1];
        first = 0;
        m++;
    }
    (void)prev_t;
    (void)m;
    return 1;
}

void fle_imp_GetRgnBox(void)
{
    static const char api[] = "GetRgnBox";
    rgnset *s;
    uint64_t p = R[2];
    int32_t l = 0, t = 0, r = 0, b = 0;
    uint32_t i;

    fle_gdi_check_thread(api);
    if (!rgn_get(R[1], &s)) {
        R[0] = 0;
        return;
    }
    if (p == 0) fle_stop(api, "lprc is NULL: the page gives no behaviour");
    for (i = 0; i < s->n; i++) {
        if (i == 0 || s->r[i].l < l) l = s->r[i].l;
        if (i == 0 || s->r[i].t < t) t = s->r[i].t;
        if (i == 0 || s->r[i].r > r) r = s->r[i].r;
        if (i == 0 || s->r[i].b > b) b = s->r[i].b;
    }
    put_u32_(p, (uint32_t)l);
    put_u32_(p + 4, (uint32_t)t);
    put_u32_(p + 8, (uint32_t)r);
    put_u32_(p + 12, (uint32_t)b);
    R[0] = rgn_complexity(s);
}

/* gdi32!GetRegionData: DWORD f(HRGN hrgn rcx, DWORD nCount edx, LPRGNDATA lpRgnData r8) (decision 0126, PROVISIONAL, wall 94). Learn: "fills the specified buffer with data describing a region. This data includes the dimensions of the
 * rectangles that make up the region"; "If lpRgnData is NULL, then the return value is the required number of bytes"; "If the function succeeds and nCount specifies an adequate number of bytes, then the return value equals the actual
 * number of bytes used"; failure returns 0, "If the failure is due to hrgn being invalid, then GetLastError returns ERROR_INVALID_HANDLE. Otherwise, GetLastError returns ERROR_INVALID_PARAMETER. If nCount is too small, then the
 * function fails". RGNDATAHEADER { DWORD dwSize (the size of the header: 32), DWORD iType ("must be RDH_RECTANGLES": 1, the number from wingdi.h, unofficial), DWORD nCount (the number of rectangles), DWORD nRgnSize ("the size of the
 * RGNDATA buffer required to receive the RECT structures ... If the size is not known, this member can be zero": nCount * 16, S-rgndata-rgnsize, a choice), RECT rcBound ("a bounding rectangle for the region in logical units") } followed by
 * the RECTs "sorted top to bottom, left to right. They do not overlap" (the band form of this file is that order). The empty region: nCount 0, rcBound (0, 0, 0, 0) (the choice of GetRgnBox: S-region), 32 bytes. A NULL lpRgnData with a valid
 * region returns the size whatever nCount is (the page: "If this parameter is NULL, then the return value contains the number of bytes needed"). */
#define FLE_RGNDATAHEADER_SIZE 32u
#define FLE_RDH_RECTANGLES     1u

void fle_imp_GetRegionData(void)
{
    static const char api[] = "GetRegionData";
    rgnset *s;
    uint64_t p = R[8];
    uint32_t cnt = (uint32_t)R[2], need, i;
    int32_t l = 0, t = 0, r = 0, b = 0;

    fle_gdi_check_thread(api);
    if (!rgn_get(R[1], &s)) {
        fle_set_last_error(FLE_GDI_ERROR_INVALID_HANDLE);
        R[0] = 0;
        return;
    }
    need = FLE_RGNDATAHEADER_SIZE + 16u * s->n;
    if (p == 0) {
        R[0] = need;
        return;
    }
    if (cnt < need) {
        fle_set_last_error(FLE_GDI_ERROR_INVALID_PARAMETER_);
        R[0] = 0;
        return;
    }
    for (i = 0; i < s->n; i++) {
        if (i == 0 || s->r[i].l < l) l = s->r[i].l;
        if (i == 0 || s->r[i].t < t) t = s->r[i].t;
        if (i == 0 || s->r[i].r > r) r = s->r[i].r;
        if (i == 0 || s->r[i].b > b) b = s->r[i].b;
    }
    put_u32_(p, FLE_RGNDATAHEADER_SIZE);
    put_u32_(p + 4, FLE_RDH_RECTANGLES);
    put_u32_(p + 8, s->n);
    put_u32_(p + 12, 16u * s->n);
    put_u32_(p + 16, (uint32_t)l);
    put_u32_(p + 20, (uint32_t)t);
    put_u32_(p + 24, (uint32_t)r);
    put_u32_(p + 28, (uint32_t)b);
    for (i = 0; i < s->n; i++) {
        put_u32_(p + FLE_RGNDATAHEADER_SIZE + 16u * i, (uint32_t)s->r[i].l);
        put_u32_(p + FLE_RGNDATAHEADER_SIZE + 16u * i + 4, (uint32_t)s->r[i].t);
        put_u32_(p + FLE_RGNDATAHEADER_SIZE + 16u * i + 8, (uint32_t)s->r[i].r);
        put_u32_(p + FLE_RGNDATAHEADER_SIZE + 16u * i + 12, (uint32_t)s->r[i].b);
    }
    R[0] = need;
}

void fle_imp_OffsetRgn(void)
{
    static const char api[] = "OffsetRgn";
    rgnset *s;
    int32_t dx = (int32_t)(uint32_t)R[2], dy = (int32_t)(uint32_t)R[8];
    uint32_t i;

    fle_gdi_check_thread(api);
    if (!rgn_get(R[1], &s)) {
        R[0] = 0;   /* ERROR (0): "An error occurred; region is unaffected" */
        return;
    }
    for (i = 0; i < s->n; i++) {
        if ((int64_t)s->r[i].l + dx < RGN_MIN || (int64_t)s->r[i].r + dx > RGN_MAX || (int64_t)s->r[i].t + dy < RGN_MIN || (int64_t)s->r[i].b + dy > RGN_MAX) {
            R[0] = 0;
            return;
        }
    }
    for (i = 0; i < s->n; i++) {
        s->r[i].l += dx;
        s->r[i].r += dx;
        s->r[i].t += dy;
        s->r[i].b += dy;
    }
    R[0] = rgn_complexity(s);
}

void fle_imp_RectInRegion(void)
{
    static const char api[] = "RectInRegion";
    rgnset *s;
    uint64_t p = R[2];
    int32_t l, t, r, b;
    uint32_t i;

    fle_gdi_check_thread(api);
    if (!rgn_get(R[1], &s)) {
        R[0] = 0;
        return;
    }
    if (p == 0) fle_stop(api, "lprect is NULL: the page gives no behaviour");
    l = (int32_t)fle_ld(p, 4);
    t = (int32_t)fle_ld(p + 4, 4);
    r = (int32_t)fle_ld(p + 8, 4);
    b = (int32_t)fle_ld(p + 12, 4);
    R[0] = 0;
    if (r <= l || b <= t) return;      /* an empty rectangle has no part in the region (a choice: the page gives none) */
    for (i = 0; i < s->n; i++)
        if (l < s->r[i].r && r > s->r[i].l && t < s->r[i].b && b > s->r[i].t) R[0] = 1;
}

void fle_imp_PtInRegion(void)
{
    static const char api[] = "PtInRegion";
    rgnset *s;

    fle_gdi_check_thread(api);
    if (!rgn_get(R[1], &s)) {
        R[0] = 0;
        return;
    }
    R[0] = rgn_has(s, (int32_t)(uint32_t)R[2], (int32_t)(uint32_t)R[8]) ? 1u : 0u;
}

void fle_imp_EqualRgn(void)
{
    static const char api[] = "EqualRgn";
    rgnset *a, *b;

    fle_gdi_check_thread(api);
    if (!rgn_get(R[1], &a) || !rgn_get(R[2], &b)) {
        R[0] = 0;
        return;
    }
    R[0] = (a->n == b->n && (a->n == 0 || memcmp(a->r, b->r, a->n * sizeof a->r[0]) == 0)) ? 1u : 0u;
}

/* gdi32!CombineRgn: int f(HRGN hrgnDst rcx, HRGN hrgnSrc1 rdx, HRGN hrgnSrc2 r8, int iMode r9d). The result is built from the rectangles of the two sources: AND = every pairwise intersection; OR = both lists; XOR and DIFF: the pieces are cut with the
 * y and x boundaries of both (the complement of an operand inside the bounding band): done by sampling the cells of the grid that the boundaries of both regions make: a cell is in the result when the mode's predicate of (in Src1, in Src2) holds. */
void fle_imp_CombineRgn(void)
{
    static const char api[] = "CombineRgn";
    rgnset *d, *a, *b = NULL;
    gdi_obj *od = rgn_get(R[1], &d), *oa = rgn_get(R[2], &a), *ob = NULL;
    int32_t mode = (int32_t)(uint32_t)R[9];
    rgnset res;
    uint32_t cx;

    fle_gdi_check_thread(api);
    if (mode >= RGN_AND_ && mode <= RGN_DIFF_) ob = rgn_get(R[8], &b);
    if (!od || !oa || (mode != RGN_COPY_ && !ob) || mode < RGN_AND_ || mode > RGN_COPY_) {
        R[0] = 0;
        return;
    }
    memset(&res, 0, sizeof res);
    if (mode == RGN_COPY_) {
        rgn_reserve(&res, a->n);
        if (a->n) memcpy(res.r, a->r, (size_t)a->n * sizeof res.r[0]);
        res.n = a->n;
    } else {
        rgn_combine(&res, a, b, mode);   /* decision 0229: no limit on the result (it stopped above 192 cells / 64 rectangles) */
    }
    free(d->r);   /* the destination may be an operand: the result was built in `res` first */
    *d = res;
    cx = rgn_complexity(d);
    R[0] = cx;
}

/* ---- the clipping region of a DC (decision 0126, PROVISIONAL) ----
 * Learn: GetClipRgn: "retrieves a handle identifying the current application-defined clipping region for the specified device context"; "If the function succeeds and there is no clipping region for the given
 * device context, the return value is zero. If the function succeeds and there is a clipping region ..., the return value is 1. If an error occurs, the return value is -1"; "An application-defined clipping region is
 * a clipping region identified by the SelectClipRgn function. It is not a clipping region created when the application calls the BeginPaint function"; the hrgn argument "A handle to an existing region before the
 * function is called. After the function returns, this parameter is a handle to a copy of the current clipping region". SelectClipRgn: "selects a region as the current clipping region"; "Only a copy of the selected
 * region is used. The region itself can be selected for any number of other device contexts or it can be deleted"; "assumes that the coordinates for a region are specified in device units"; "To remove a device-context's
 * clipping region, specify a NULL region handle"; the return value is the region's complexity (NULLREGION 1, SIMPLEREGION 2, COMPLEXREGION 3) or ERROR (0: "An error occurred. (The previous clipping region is
 * unaffected.)"). ExtSelectClipRgn(hdc, hrgn, mode): RGN_AND 1, RGN_OR 2, RGN_XOR 3, RGN_DIFF 4, RGN_COPY 5 (the numbers are those of fle_gdi.c's regions, UNOFFICIAL); "This handle must not be NULL unless the RGN_COPY mode
 * is specified"; RGN_COPY with NULL "the new clipping region is the default clipping region (the default clipping region is a null region)"; the combination of the current region with hrgn, the result's complexity.
 * IntersectClipRect: "The lower and right-most edges of the given rectangle are excluded from the clipping region"; "If a clipping region does not already exist then the system may apply a default clipping region to the
 * specified HDC. A clipping region is then created from the intersection of that default clipping region and the rectangle specified in the function parameters": the default region of a DC is its surface (a memory
 * DC: the bitmap; a window DC: its visible rectangle, S-default-clip: a reading, the page says "may"); the rectangle is in LOGICAL units (it goes through the DC's origins); ExcludeClipRect: "The lower and right edges of
 * the specified rectangle are not excluded from the clipping region" (the rectangle is excluded from the region as a half-open set: S-exclude-edges: the sentence is read as the page's description of the part kept).
 * An application-defined region that selected none is "no region": GetClipRgn returns 0 and leaves hrgn alone. The region of a DC is part of the saved state of SaveDC ("region": the SaveDC page) and goes with a
 * released common DC (the Display Device Context Defaults page). */
static void clip_from_rgnset(gdi_clip *c, const rgnset *s)
{
    clip_reserve(c, s->n);
    c->has = 1;
    c->n = s->n;
    if (s->n) memcpy(c->r, s->r, (size_t)s->n * sizeof c->r[0]);
}

/* `s` must be a set that owns its array (or a zeroed one): its rectangles are replaced */
static void rgnset_from_clip(rgnset *s, const gdi_clip *c)
{
    rgn_reserve(s, c->n);
    s->n = c->n;
    if (c->n) memcpy(s->r, c->r, (size_t)c->n * sizeof s->r[0]);
}

static uint32_t clip_complexity(const gdi_clip *c) { return c->n == 0 ? FLE_GDI_NULLREGION : c->n == 1 ? FLE_GDI_SIMPLEREGION : 3u; }

/* The state of a DC of a clip call: a memory DC's, or the state of a live window / screen DC; every other handle stops (the pages name ERROR 0 for "an error" and do not say which handles are errors). */
static gdi_dc *clip_dc(const char *api, uint64_t h)
{
    if (fle_gdc_is_live(h)) return screen_state(api, h);
    return need_mem_dc(api, h, "hdc is a DC of GetDC(NULL)");
}

/* The default region of a DC as a rectangle in device units of the DC (its surface as the DC sees it): a memory DC: the bitmap; a window DC: the visible rectangle of the window's client area (the origin of the DC
 * subtracted: the region's coordinates are the DC's device units, the origin of the window being (0, 0)). */
static fle_rrect clip_default(const char *api, uint64_t h)
{
    target t;
    fle_rrect d;

    get_target0(api, "the default clipping region", h, "hdc", &t);
    d = t.clip;
    d.x0 -= t.org[0];
    d.x1 -= t.org[0];
    d.y0 -= t.org[1];
    d.y1 -= t.org[1];
    return d;
}

void fle_imp_GetClipRgn(void)
{
    static const char api[] = "GetClipRgn";
    gdi_dc *dc;
    rgnset *dst = NULL;

    fle_gdi_check_thread(api);
    dc = clip_dc(api, R[1]);
    if (!rgn_get(R[2], &dst)) {
        R[0] = 0xFFFFFFFFu;                 /* "If an error occurs, the return value is -1": hrgn is not a region of this table */
        return;
    }
    if (!dc->clip.has) {
        R[0] = 0;
        return;
    }
    rgnset_from_clip(dst, &dc->clip);
    R[0] = 1;
}

void fle_imp_SelectClipRgn(void)
{
    static const char api[] = "SelectClipRgn";
    gdi_dc *dc;
    rgnset *src = NULL;

    fle_gdi_check_thread(api);
    dc = clip_dc(api, R[1]);
    if (R[2] == 0) {
        dc->clip.has = 0;
        dc->clip.n = 0;
        R[0] = FLE_GDI_SIMPLEREGION;        /* the region of a DC without an application-defined region is its default region, a single rectangle (S-select-null-result: the page gives the type of "the region", NULL removes it) */
        return;
    }
    if (!rgn_get(R[2], &src)) {
        R[0] = 0;                           /* ERROR: "An error occurred. (The previous clipping region is unaffected.)" */
        return;
    }
    clip_from_rgnset(&dc->clip, src);
    R[0] = clip_complexity(&dc->clip);
}

void fle_imp_ExtSelectClipRgn(void)
{
    static const char api[] = "ExtSelectClipRgn";
    gdi_dc *dc;
    rgnset *src = NULL, cur, res;
    int32_t mode = (int32_t)(uint32_t)R[8];
    fle_rrect def;

    fle_gdi_check_thread(api);
    dc = clip_dc(api, R[1]);
    if (mode < RGN_AND_ || mode > RGN_COPY_) {
        R[0] = 0;
        return;
    }
    if (R[2] == 0) {
        if (mode != RGN_COPY_) {
            R[0] = 0;                       /* "This handle must not be NULL unless the RGN_COPY mode is specified" */
            return;
        }
        dc->clip.has = 0;
        dc->clip.n = 0;
        R[0] = FLE_GDI_SIMPLEREGION;
        return;
    }
    if (!rgn_get(R[2], &src)) {
        R[0] = 0;
        return;
    }
    if (mode == RGN_COPY_) {
        clip_from_rgnset(&dc->clip, src);
        R[0] = clip_complexity(&dc->clip);
        return;
    }
    /* the current region: the application-defined one, or the default (S-default-clip) */
    memset(&cur, 0, sizeof cur);
    memset(&res, 0, sizeof res);
    if (dc->clip.has) {
        rgnset_from_clip(&cur, &dc->clip);
    } else {
        def = clip_default(api, R[1]);
        rgn_set_rect_(&cur, (int32_t)def.x0, (int32_t)def.y0, (int32_t)def.x1, (int32_t)def.y1);
    }
    rgn_combine(&res, &cur, src, mode);   /* decision 0229: no limit on the result */
    clip_from_rgnset(&dc->clip, &res);
    rgn_free(&cur);
    rgn_free(&res);
    R[0] = clip_complexity(&dc->clip);
}

/* The rectangle (left, top, right, bottom) of the clip calls is in LOGICAL units: the device rectangle is Dx = Lx - WOx + VOx (MM_TEXT; decision 0111). */
static void clip_rect_op(const char *api, int exclude)
{
    int32_t l = (int32_t)(uint32_t)R[2], t = (int32_t)(uint32_t)R[8], r = (int32_t)(uint32_t)R[9], b = (int32_t)(uint32_t)fle_arg(5);
    gdi_dc *dc;
    rgnset cur, res, rc;
    fle_rrect def;
    int64_t dl, dt, dr, db;
    char why[100];

    fle_gdi_check_thread(api);
    dc = clip_dc(api, R[1]);
    if (dc->map_mode != FLE_GDI_MM_TEXT) {
        snprintf(why, sizeof why, "the mapping mode is %u, not MM_TEXT: the extents of the other modes are not on the pages", (unsigned)dc->map_mode);
        fle_stop(api, why);
    }
    dl = (int64_t)l - dc->worg[0] + dc->vorg[0];
    dt = (int64_t)t - dc->worg[1] + dc->vorg[1];
    dr = (int64_t)r - dc->worg[0] + dc->vorg[0];
    db = (int64_t)b - dc->worg[1] + dc->vorg[1];
    if (dl < RGN_MIN || dl > RGN_MAX || dt < RGN_MIN || dt > RGN_MAX || dr < RGN_MIN || dr > RGN_MAX || db < RGN_MIN || db > RGN_MAX) {
        R[0] = 0;
        return;
    }
    memset(&cur, 0, sizeof cur);
    memset(&res, 0, sizeof res);
    memset(&rc, 0, sizeof rc);
    if (dc->clip.has) {
        rgnset_from_clip(&cur, &dc->clip);
    } else {
        def = clip_default(api, R[1]);
        rgn_set_rect_(&cur, (int32_t)def.x0, (int32_t)def.y0, (int32_t)def.x1, (int32_t)def.y1);
    }
    rgn_set_rect_(&rc, (int32_t)dl, (int32_t)dt, (int32_t)dr, (int32_t)db);
    rgn_combine(&res, &cur, &rc, exclude ? RGN_DIFF_ : RGN_AND_);   /* decision 0229: no limit on the result */
    clip_from_rgnset(&dc->clip, &res);
    rgn_free(&cur);
    rgn_free(&rc);
    rgn_free(&res);
    R[0] = clip_complexity(&dc->clip);
}

void fle_imp_IntersectClipRect(void) { clip_rect_op("IntersectClipRect", 0); }
void fle_imp_ExcludeClipRect(void) { clip_rect_op("ExcludeClipRect", 1); }

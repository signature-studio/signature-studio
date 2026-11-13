/* The virtual screen DC (docs/decisions/0053-virtual-screen-dc.md; the pages read are listed there and in
 * docs/investigations/getdc-caller.md). Single guest thread. A DC is {live or released}; the table has one entry per GetDC
 * call and an entry is never reused. A case the Microsoft pages leave open, a case that needs a window, and every index the
 * guest does not use, stops the run through fle_stop (fle_env.h). Nothing here reads the host: no display, no DPI setting, no
 * window, no clock, no file. */
#define _GNU_SOURCE
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_host.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_gdc.h"
#include "fle_win.h"
#include "fle_thread.h"

/* weak: a link without fle_win.c (the unit tests of this file) has no window table; a hWnd that is not NULL then stops as before */
int fle_win_dc_query(uint64_t hwnd, fle_win_dcq *out) __attribute__((weak));


static int64_t fle_gdc_owner_tid;

static void fle_gdc_check_thread(const char *api)
{
    int64_t tid = (int64_t)fle_host_tid();

    if (fle_gdc_owner_tid == 0) fle_gdc_owner_tid = tid;
    if (tid != fle_gdc_owner_tid) fle_stop(api, "called from a second thread, not supported");
}

/* Decision 0125 (PROVISIONAL, PM's decision 1 of 0125): a DC has a record. kind SCREEN: the DC of the entire screen (hWnd NULL, as before). The other kinds are the DCs of a WINDOW, STATE ONLY: nothing is
 * drawn (the first pixel call that reaches one stops by name, fle_gdi.c). Learn (GetDC): "retrieves a common, class, or private DC depending on the class style of the specified window. For class and private
 * DCs, GetDC leaves the previously assigned attributes unchanged. However, for common DCs, GetDC assigns default attributes to the DC each time it is retrieved"; "Class and private DCs do not have to be
 * released". So: CS_OWNDC (0x20) gives ONE DC per window (the same handle on every call, attributes kept), CS_CLASSDC (0x40) ONE DC per class (shared by its windows), every other class a common DC (a new handle
 * every call, never reused, released by ReleaseDC; its attributes start again from the defaults because fle_gdi.c drops them at the release). `root_lid` is the surface (R10 of docs/investigations/detach-window.md:
 * one drawing surface per top-level window; the logical id of the top-level ancestor, so that a re-created HWND does not orphan it, R5). */
typedef struct {
    uint8_t live;
    uint8_t kind;               /* FLE_GDC_K_* */
    uint64_t hwnd;              /* the window (0 = the entire screen) */
    uint32_t flags;             /* GetDCEx flags (0 for the others) */
    uint32_t thread;            /* the guest thread that got the DC */
    uint32_t class_style;
    uint16_t atom;              /* the class of the window (class DCs are found by it) */
    uint32_t lid, root_lid;
    int32_t cx, cy;
    uint8_t from_ex;            /* decision 0142: the DC was made by GetDCEx (its flags are the clip flags) */
    uint8_t paint_valid;        /* decision 0142: between BeginPaint and EndPaint: the DC is clipped to the paint rectangle (client coordinates) */
    int32_t paint[4];
    uint32_t paint_np;          /* decision 0247: the update region is these paint_np disjoint rectangles pp (0: the rectangle `paint`) */
    int32_t pp[FLE_GDC_PAINT_PIECES][4];
} dcrec;
static int new_from_ex;         /* set around the dc_new of GetDCEx */

static dcrec *dcs;
static uint64_t dcs_n, dcs_cap, dcs_live;

uint64_t fle_gdc_live_count(void) { return dcs_live; }

static int in_gdc_region(uint64_t h)
{
    return h > FLE_GDC_HANDLE_BASE && h < FLE_GDC_HANDLE_BASE + FLE_GDC_REGION_SIZE;
}

/* The slot index of an issued handle value, or -1 when the value was never issued. */
static int64_t dc_index(uint64_t h)
{
    uint64_t off, i;

    if (!in_gdc_region(h)) return -1;
    off = h - FLE_GDC_HANDLE_BASE;
    if (off % 4u) return -1;
    i = off / 4u - 1u;
    return i < dcs_n ? (int64_t)i : -1;
}

int fle_gdc_is_live(uint64_t h)
{
    int64_t i = dc_index(h);

    return i >= 0 && dcs[i].live;
}

static uint32_t cur_guest_thread(void) { return fle_thread_current ? (uint32_t)fle_thread_current() : 0u; }

/* Decision 0174 (PROVISIONAL): the entry of a released DC is reused, after a delay. Before, the table had one entry per GetDC call for good (72 bytes, to FLE_GDC_REGION_SIZE / 4 entries and then "DC handle region
 * exhausted"), and the lookup of the private DC of a window read every entry. The handle VALUE format is the same (FLE_GDC_HANDLE_BASE + 4 * (i + 1)); the entry is reused first-in first-out and only once
 * FLE_GDC_REUSE_AFTER released entries are waiting, so a stale handle of the last FLE_GDC_REUSE_AFTER released DCs still reads as released (ReleaseDC 0) and a run that releases fewer DCs is identical to the earlier
 * behaviour. The pages give no reuse rule for the handle value (Windows reuses the table entry: choice). */
#define FLE_GDC_REUSE_AFTER 32768u
static uint64_t *dead_q;                /* the entries of released DCs, oldest first: dead_q[dead_head .. dead_head + dead_n - 1] */
static uint64_t dead_head, dead_n, dead_cap;
static uint64_t *pers;                  /* the entries of the live private (CS_OWNDC) and class DCs: the lookup of dc_persistent and the window destruction read these and not the whole table */
static uint64_t pers_n, pers_cap;

/* The one place that grows a table of this file (tests/wall_fr10.rs counts the calls of the allocator in this file). */
static void *tab_grow(void *p, uint64_t n, size_t size) { return realloc(p, (size_t)(n * size)); }

static void dc_dead(uint64_t i)
{
    if (dead_head > 0 && dead_head * 2 >= dead_cap) {
        memmove(dead_q, dead_q + dead_head, dead_n * sizeof *dead_q);
        dead_head = 0;
    }
    if (dead_head + dead_n == dead_cap) {
        uint64_t nc = dead_cap ? dead_cap * 2 : 1024;
        uint64_t *n = (uint64_t *)tab_grow(dead_q, nc, sizeof *n);

        if (!n) return;                 /* out of memory: the entry is simply not reused */
        dead_q = n;
        dead_cap = nc;
    }
    dead_q[dead_head + dead_n++] = i;
}

/* Decision 0174 (a measurement, G3 harness FLE_G3_LEAKSTATS=1): o[0] dcs_n (entries ever used), o[1] dcs_live, o[2] dcs_cap, o[3] released entries waiting for reuse, o[4] persistent DCs. */
void fle_gdc_table_stats(uint64_t o[5])
{
    o[0] = dcs_n;
    o[1] = dcs_live;
    o[2] = dcs_cap;
    o[3] = dead_n;
    o[4] = pers_n;
}

/* A new entry (a new handle value, reused only as described above). */
static uint64_t dc_new(const char *api, uint8_t kind, uint64_t hwnd, uint32_t flags, const fle_win_dcq *q)
{
    dcrec *r;
    uint64_t idx;

    if (dead_n > FLE_GDC_REUSE_AFTER) {
        idx = dead_q[dead_head++];
        dead_n--;
    } else {
        if (dcs_n == dcs_cap) {
            uint64_t nc = dcs_cap ? dcs_cap * 2 : 16;
            dcrec *n = (dcrec *)tab_grow(dcs, nc, sizeof *dcs);

            if (!n) fle_stop(api, "out of memory");
            dcs = n;
            dcs_cap = nc;
        }
        if (dcs_n + 2 >= FLE_GDC_REGION_SIZE / 4u) fle_stop(api, "DC handle region exhausted");
        idx = dcs_n++;
    }
    r = &dcs[idx];
    memset(r, 0, sizeof *r);
    r->live = 1;
    r->kind = kind;
    r->hwnd = hwnd;
    r->flags = flags;
    r->thread = cur_guest_thread();
    r->from_ex = (uint8_t)new_from_ex;
    if (q) {
        r->class_style = q->class_style;
        r->atom = q->atom;
        r->lid = q->lid;
        r->root_lid = q->root_lid;
        r->cx = q->cx;
        r->cy = q->cy;
    }
    if (kind == FLE_GDC_K_OWN || kind == FLE_GDC_K_CLASS) {
        if (pers_n == pers_cap) {
            uint64_t nc = pers_cap ? pers_cap * 2 : 64;
            uint64_t *n = (uint64_t *)tab_grow(pers, nc, sizeof *n);

            if (!n) fle_stop(api, "out of memory");
            pers = n;
            pers_cap = nc;
        }
        pers[pers_n++] = idx;
    }
    dcs_live++;
    return FLE_GDC_HANDLE_BASE + 4u * (idx + 1u);
}

int fle_gdc_is_window_dc(uint64_t h)
{
    int64_t i = dc_index(h);

    return i >= 0 && dcs[i].live && dcs[i].kind != FLE_GDC_K_SCREEN;
}

int fle_gdc_dc_info(uint64_t h, fle_gdc_info *out)
{
    int64_t i = dc_index(h);

    if (i < 0 || !dcs[i].live) return 0;
    out->hwnd = dcs[i].hwnd;
    out->kind = dcs[i].kind;
    out->flags = dcs[i].flags;
    out->thread = dcs[i].thread;
    out->class_style = dcs[i].class_style;
    out->lid = dcs[i].lid;
    out->root_lid = dcs[i].root_lid;
    out->cx = dcs[i].cx;
    out->cy = dcs[i].cy;
    out->from_ex = dcs[i].from_ex;
    out->paint_valid = dcs[i].paint_valid;
    for (int k = 0; k < 4; k++) out->paint[k] = dcs[i].paint[k];
    out->paint_np = dcs[i].paint_np;
    for (uint32_t k = 0; k < dcs[i].paint_np; k++)
        for (int c = 0; c < 4; c++) out->pp[k][c] = dcs[i].pp[k][c];
    return 1;
}

/* Decision 0142: BeginPaint clips its DC to the update region (Learn BeginPaint: "automatically sets the clipping region of the device context to exclude any area outside the update region"); EndPaint lifts it.
 * valid = 0 clears; the rectangle is the paint rectangle of the PAINTSTRUCT in client coordinates (the update region is one bounding rectangle: S-updregion of decision 0126). */
void fle_gdc_set_paint_rect(uint64_t h, int valid, int32_t l, int32_t t, int32_t r, int32_t b)
{
    int64_t i = dc_index(h);

    if (i < 0 || !dcs[i].live) return;
    dcs[i].paint_valid = valid ? 1u : 0u;
    dcs[i].paint[0] = l;
    dcs[i].paint[1] = t;
    dcs[i].paint[2] = r;
    dcs[i].paint[3] = b;
    dcs[i].paint_np = 0;
}

void fle_gdc_set_paint_pieces(uint64_t h, const int32_t (*p)[4], uint32_t n)
{
    int64_t i = dc_index(h);

    if (i < 0 || !dcs[i].live || !dcs[i].paint_valid || n < 2 || n > FLE_GDC_PAINT_PIECES) return;
    for (uint32_t k = 0; k < n; k++)
        for (int c = 0; c < 4; c++) dcs[i].pp[k][c] = p[k][c];
    dcs[i].paint_np = n;
}

/* The persistent DC of a window (CS_OWNDC) or of a class (CS_CLASSDC), or 0 when it has not been made yet. */
static uint64_t dc_persistent(uint8_t kind, uint64_t hwnd, uint16_t atom)
{
    uint64_t k, i;

    for (k = 0; k < pers_n; k++) {
        i = pers[k];
        if (!dcs[i].live || dcs[i].kind != kind) continue;
        if (kind == FLE_GDC_K_OWN ? dcs[i].hwnd == hwnd : dcs[i].atom == atom) return FLE_GDC_HANDLE_BASE + 4u * (i + 1u);
    }
    return 0;
}

extern void fle_gdi_screen_released(uint64_t h) __attribute__((weak));

/* Decision 0125: called by fle_win.c when a window is destroyed. The private DC of a CS_OWNDC window goes with it (READING, S-owndc-life: Learn does not say; a private DC belongs to its window); the common DCs
 * that were not released stay live until the guest releases them (S-common-after-destroy); a class DC belongs to the class and stays. */
void fle_gdc_window_destroyed(uint64_t hwnd)
{
    uint64_t k = 0, i;

    while (k < pers_n) {
        i = pers[k];
        if (dcs[i].live && dcs[i].kind == FLE_GDC_K_OWN && dcs[i].hwnd == hwnd) {
            dcs[i].live = 0;
            dcs_live--;
            if (fle_gdi_screen_released != NULL) fle_gdi_screen_released(FLE_GDC_HANDLE_BASE + 4u * (i + 1u));
            pers[k] = pers[--pers_n];
            dc_dead(i);
            continue;
        }
        k++;
    }
}

/* The DC of a window for GetDC / GetDCEx without DCX_CACHE and DCX_WINDOW, and for BeginPaint: private, class or common. `q` is the window's answer (fle_win_dc_query). */
static uint64_t dc_of_client(const char *api, uint64_t hwnd, const fle_win_dcq *q, uint32_t flags)
{
    uint64_t h;

    if (q->class_style & FLE_CS_OWNDC) {
        h = dc_persistent(FLE_GDC_K_OWN, hwnd, 0);
        return h ? h : dc_new(api, FLE_GDC_K_OWN, hwnd, flags, q);
    }
    if (q->class_style & FLE_CS_CLASSDC) {
        h = dc_persistent(FLE_GDC_K_CLASS, 0, q->atom);
        return h ? h : dc_new(api, FLE_GDC_K_CLASS, hwnd, flags, q);
    }
    return dc_new(api, FLE_GDC_K_COMMON, hwnd, flags, q);
}

/* A window of the table, or the failure: NULL and ERROR_INVALID_WINDOW_HANDLE (1400: a CHOICE WITHOUT A SOURCE, S-err of decision 0123: the pages say "If the function fails, the return value is NULL" and name no
 * code). Without a window table (the unit tests of this file) a hWnd that is not NULL stops as before. */
static int window_of(const char *api, uint64_t hwnd, fle_win_dcq *q)
{
    if (fle_win_dc_query == NULL) fle_stop(api, "hWnd is not NULL: the DC of a window needs a window object, not implemented (only the DC of the entire screen is)");
    if (!fle_win_dc_query(hwnd, q)) {
        fle_set_last_error(FLE_ERROR_INVALID_WINDOW_HANDLE);
        R[0] = 0;
        return 0;
    }
    return 1;
}

/* user32!GetDC: HDC f(HWND hWnd rcx). Learn (GetDC): "If this value is NULL, GetDC retrieves the DC for the entire screen"; the return value is a handle to the DC for the specified window's client area, NULL on
 * failure; "the number of DCs is limited only by available memory"; a common DC has to be released with ReleaseDC, once for each call. hWnd NULL: a new handle every call (choice: the page does not say whether
 * two calls may return the same value), as before. A window (decision 0125): the private, class or common DC of its class style (above); an hWnd that is not a window of the table fails with NULL and 1400. The
 * last error is otherwise left alone (the page does not say it changes). */
void fle_imp_GetDC(void)
{
    static const char api[] = "GetDC";
    uint64_t hwnd = R[1];
    fle_win_dcq q;

    fle_gdc_check_thread(api);
    if (hwnd == 0) {
        R[0] = dc_new(api, FLE_GDC_K_SCREEN, 0, 0, NULL);
        return;
    }
    if (!window_of(api, hwnd, &q)) return;
    R[0] = dc_of_client(api, hwnd, &q, 0);
}

/* user32!GetWindowDC: HDC f(HWND hWnd rcx). Learn (GetWindowDC): "retrieves the DC for the entire window, including title bar, menus, and scroll bars" (the page says "a device context for the specified
 * window"); hWnd NULL "retrieves the device context for the entire screen" ("the primary display monitor"); NULL on failure ("an error or an invalid hWnd parameter"); "the ReleaseDC function must be called to
 * release the device context. Not releasing the window device context has serious effects on painting". The class style is not mentioned on the page: the DC is always a common one (a new handle every call,
 * its device origin the window's upper-left corner), a CHOICE WITHOUT A SOURCE (S-windc-common). hWnd NULL: the DC of the entire screen, as GetDC(NULL). */
void fle_imp_GetWindowDC(void)
{
    static const char api[] = "GetWindowDC";
    uint64_t hwnd = R[1];
    fle_win_dcq q;

    fle_gdc_check_thread(api);
    if (hwnd == 0) {
        R[0] = dc_new(api, FLE_GDC_K_SCREEN, 0, 0, NULL);
        return;
    }
    if (!window_of(api, hwnd, &q)) return;
    R[0] = dc_new(api, FLE_GDC_K_WINDOW, hwnd, 0, &q);
}

/* user32!GetDCEx: HDC f(HWND hWnd rcx, HRGN hrgnClip rdx, DWORD flags r8d). Learn (GetDCEx): DCX_WINDOW "a DC that corresponds to the window rectangle rather than the client rectangle"; DCX_CACHE "a DC from the cache,
 * rather than the OWNDC or CLASSDC window. Essentially overrides CS_OWNDC and CS_CLASSDC"; DCX_PARENTCLIP, DCX_CLIPSIBLINGS, DCX_CLIPCHILDREN, DCX_LOCKWINDOWUPDATE: the visible region (not modelled: they
 * change nothing that a state-only DC can show); DCX_NORESETATTRS "This flag is ignored"; DCX_INTERSECTUPDATE and DCX_VALIDATE "Reserved; do not use"; DCX_EXCLUDERGN and DCX_INTERSECTRGN combine hrgnClip with
 * the visible region and give the region to the system ("the operating system assumes ownership of the region and will automatically delete it"): THE CLIP REGION IS NOT MODELLED, so those two stop by name.
 * "The function returns a handle to a DC that belongs to the window's class if CS_CLASSDC, CS_OWNDC or CS_PARENTDC was specified": CS_PARENTDC is read as the Window Class Styles page says ("receives a regular
 * device context from the system's cache": a common DC), a contradiction between two pages (S-gdcex-parentdc). The numbers of the DCX_ flags are not on the Learn pages (UNOFFICIAL: windows-sys on docs.rs,
 * logs/dcx-constants.txt of the run): the bits that the page does not list (the guest passes 0x10000 with DCX_WINDOW and with others) are kept in the record and have no effect (S-gdcex-bits). DCX_WINDOW always
 * gives a common window DC, with or without DCX_CACHE (S-gdcex-window: the private and the class DC are DCs of the client area). hWnd NULL: the DC of the entire screen. An invalid hWnd fails with NULL (the page
 * says so) and 1400 (S-err). */
void fle_imp_GetDCEx(void)
{
    static const char api[] = "GetDCEx";
    uint64_t hwnd = R[1];
    uint32_t flags = (uint32_t)R[8];
    fle_win_dcq q;

    fle_gdc_check_thread(api);
    if (flags & (FLE_DCX_EXCLUDERGN | FLE_DCX_INTERSECTRGN)) fle_stop(api, "DCX_EXCLUDERGN or DCX_INTERSECTRGN: the clip region (and the ownership the system takes of it) is not modelled (decision 0125; clip regions are a stop of decision 0123)");
    if (hwnd == 0) {
        R[0] = dc_new(api, FLE_GDC_K_SCREEN, 0, flags, NULL);
        return;
    }
    if (!window_of(api, hwnd, &q)) return;
    new_from_ex = 1;
    if (flags & FLE_DCX_WINDOW) R[0] = dc_new(api, FLE_GDC_K_WINDOW, hwnd, flags, &q);
    else if (flags & FLE_DCX_CACHE) R[0] = dc_new(api, FLE_GDC_K_COMMON, hwnd, flags, &q);
    else R[0] = dc_of_client(api, hwnd, &q, flags);
    new_from_ex = 0;
}

/* user32!ReleaseDC: int f(HWND hWnd rcx, HDC hDC rdx). Learn (ReleaseDC): "If the DC was released, the return value is 1. If the DC was not released, the return value is zero"; the DC to release is a common DC from
 * GetDC, GetWindowDC or GetDCEx; "ReleaseDC must be called from the same thread that called GetDC". A handle that this runtime issued and that is already released, and NULL, are "not released": 0 (the page's own
 * return value). A value that is not a DC of this runtime (another kind of handle, a number in the region that was never issued) is not described and stops. Decision 0125, the DC of a window: hWnd must be the
 * window the DC was got for (the page: "the window whose DC is to be released"), otherwise 0, not released (READING, S-release-mismatch: the pages do not say what a wrong hWnd does); a DC got in another guest
 * thread is not released: 0 (S-release-thread: the page says "must" and not what happens); a private or class DC is "not required" to be released: ReleaseDC returns 1 and the DC stays (S-release-owndc: the page does
 * not say what ReleaseDC does to one); the DC of the entire screen with a non-NULL hWnd stops as before (the page does not say). */
/* Decision 0121: the attribute state of a screen DC (fle_gdi.c) is lost when the DC is released (a weak reference: a build without fle_gdi.c keeps no state). */

void fle_imp_ReleaseDC(void)
{
    static const char api[] = "ReleaseDC";
    uint64_t hwnd = R[1], h = R[2];
    int64_t i;

    fle_gdc_check_thread(api);
    if (h == 0) {
        if (hwnd != 0 && fle_win_dc_query == NULL) fle_stop(api, "hWnd is not NULL: the DC of a window needs a window object, not implemented");
        R[0] = 0;
        return;
    }
    i = dc_index(h);
    if (i < 0) fle_stop(api, "hDC was not issued by GetDC(NULL) of this runtime (other DC kinds and handle kinds are not defined here)");
    if (!dcs[i].live) {
        R[0] = 0;
        return;
    }
    if (dcs[i].kind == FLE_GDC_K_SCREEN) {
        if (hwnd != 0) fle_stop(api, "hWnd is not NULL: the DC of a window needs a window object, not implemented");
    } else if (hwnd != dcs[i].hwnd || dcs[i].thread != cur_guest_thread()) {
        R[0] = 0;
        return;
    }
    if (dcs[i].kind == FLE_GDC_K_OWN || dcs[i].kind == FLE_GDC_K_CLASS) {
        R[0] = 1;
        return;
    }
    dcs[i].live = 0;
    dcs_live--;
    if (fle_gdi_screen_released != NULL) fle_gdi_screen_released(h);
    dc_dead((uint64_t)i);
    R[0] = 1;
}

/* gdi32!GetDeviceCaps: int f(HDC hdc rcx, int index edx). Learn (GetDeviceCaps): the return value is the value of the desired
 * item; BITSPIXEL is 16 for a 15 or 16 bits per pixel device; SIZEPALETTE is "valid only if the device driver sets the
 * RC_PALETTE bit in the RASTERCAPS index". Answered: the seven indexes that the guest's call sites use and for which this
 * environment has a value (fle_gdc.h), and (decision 0121) HORZSIZE and VERTSIZE (millimetres of the physical screen: 508 and 286,
 * derived from the virtual display at 96 DPI, fle_gdc.h). Every other index stops. A hdc that is not a live DC of this runtime is not
 * described by the page (no failure value is given) and stops. Only the low DWORD of rdx is the index (an int). */
/* Decision 0111 (OPTIONAL): a memory DC reports the bits per pixel and the planes of the bitmap selected into it (fle_gdi.c: fle_gdi_memdc_caps; a weak reference, so a
 * build without fle_gdi.c keeps the stop below for it). VCL's TBitmap.Draw asks GetDeviceCaps(BITSPIXEL) and (PLANES) of the DC of the canvas it draws on. */
extern int fle_gdi_memdc_caps(uint64_t h, uint32_t idx, int32_t *out) __attribute__((weak));

void fle_imp_GetDeviceCaps(void)
{
    static const char api[] = "GetDeviceCaps";
    uint64_t h = R[1];
    uint32_t idx = (uint32_t)R[2];
    int32_t mem = 0;
    char why[160];

    fle_gdc_check_thread(api);
    if (fle_gdi_memdc_caps != NULL && fle_gdi_memdc_caps(h, idx, &mem)) {
        R[0] = (uint32_t)mem;
        return;
    }
    if (!fle_gdc_is_live(h)) fle_stop(api, "hdc is not a live DC of GetDC(NULL) of this runtime (NULL, released, other kinds: not defined here)");
    switch (idx) {
    case FLE_GDC_IDX_HORZRES: R[0] = FLE_GDC_HORZRES; return;
    case FLE_GDC_IDX_VERTRES: R[0] = FLE_GDC_VERTRES; return;
    case FLE_GDC_IDX_BITSPIXEL: R[0] = FLE_GDC_BITSPIXEL; return;
    case FLE_GDC_IDX_PLANES: R[0] = FLE_GDC_PLANES; return;
    case FLE_GDC_IDX_LOGPIXELSX: R[0] = FLE_GDC_LOGPIXELSX; return;
    case FLE_GDC_IDX_LOGPIXELSY: R[0] = FLE_GDC_LOGPIXELSY; return;
    case FLE_GDC_IDX_SIZEPALETTE: R[0] = FLE_GDC_SIZEPALETTE; return;
    case FLE_GDC_IDX_HORZSIZE: R[0] = FLE_GDC_HORZSIZE; return;     /* decision 0121: derived from 1920 pixels at 96 DPI */
    case FLE_GDC_IDX_VERTSIZE: R[0] = FLE_GDC_VERTSIZE; return;     /* decision 0121: 1080 pixels at 96 DPI = 285.75 mm, rounded */
    default:
        snprintf(why, sizeof why, "index %u is not answered (the guest's call sites do not use it)", (unsigned)idx);
        fle_stop(api, why);
    }
}

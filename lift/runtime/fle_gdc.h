/* The virtual screen DC: user32!GetDC(NULL), gdi32!GetDeviceCaps and user32!ReleaseDC (docs/decisions/0053-virtual-screen-dc.md;
 * the pages read and the sources of every number are listed there and in docs/investigations/getdc-caller.md). Same convention
 * as fle_imports.h: arguments in R[1] (rcx), R[2] (rdx); the Win32 return value in R[0]; the guest return address stays on the
 * stack for the caller's ret. Single guest thread only (the GetDC page: a DC handle is used by one thread at a time, and
 * ReleaseDC is called from the thread that called GetDC).
 *
 * Nothing is read from the host: the screen size, the resolution in dots per inch and the colour depth below are values of this
 * lift environment, fixed, so that a run does not depend on the machine it runs on. The pages give the meaning of every index and
 * none of these values.
 *
 * Every case that needs a window, and every index that the guest does not use, stops the run through fle_stop (fle_env.h):
 * "fle: <api>: <why>" and abort(). A value is never made up for them. */
#ifndef FLE_GDC_H
#define FLE_GDC_H

#include <stdint.h>

/* Handles of the virtual screen DCs (decision 0053): FLE_GDC_HANDLE_BASE + 4 * (i + 1) for the i-th GetDC, never reused. The
 * pages give no value for a DC handle. The Learn page "Interprocess communication" (64-bit Windows programming) says that only
 * the lower 32 bits of a handle to a GDI object are significant (it may be truncated to 32 bits and sign-extended back). So the
 * values stay below 0x80000000: truncation and sign extension give the same 64-bit value. Unlike the other handle regions of
 * this runtime (0x7FFAxxxxxxxx) this one is therefore below 4 GiB; it is far above the image (0x400000..0x18C0000). */
#define FLE_GDC_HANDLE_BASE UINT64_C(0x7FA90000)
#define FLE_GDC_REGION_SIZE UINT64_C(0x00100000)

/* The virtual display (decision 0053: choices, the pages do not give them). */
#define FLE_GDC_HORZRES     1920    /* pixels */
#define FLE_GDC_VERTRES     1080    /* raster lines */
#define FLE_GDC_BITSPIXEL   32      /* adjacent colour bits per pixel */
#define FLE_GDC_PLANES      1
#define FLE_GDC_LOGPIXELSX  96      /* pixels per logical inch: the Learn DPI page's base value (USER_DEFAULT_SCREEN_DPI) */
#define FLE_GDC_LOGPIXELSY  96
#define FLE_GDC_SIZEPALETTE 0       /* not a palette device: the page says the index is valid only for a palette device */

/* GetDeviceCaps index numbers. The Learn page lists names only; the numbers are from the Windows SDK header copy of mingw-w64
 * and windows-sys (docs.rs): UNOFFICIAL (decision 0053). Every other index stops the run. */
#define FLE_GDC_IDX_HORZRES     8
#define FLE_GDC_IDX_VERTRES     10
#define FLE_GDC_IDX_BITSPIXEL   12
#define FLE_GDC_IDX_PLANES      14
#define FLE_GDC_IDX_LOGPIXELSX  88
#define FLE_GDC_IDX_LOGPIXELSY  90
#define FLE_GDC_IDX_SIZEPALETTE 104
/* The two the guest also asks for (HORZSIZE 4, VERTSIZE 6, millimetres of the physical screen). Decision 0121 (PROVISIONAL, PM's decision 1): the
 * physical size is derived from the virtual display above and the 96 pixels per logical inch (25.4 mm per inch): HORZSIZE = 1920 * 25.4 / 96 = 508 mm
 * exactly, VERTSIZE = 1080 * 25.4 / 96 = 285.75 mm, rounded to the nearest millimetre = 286 (the rounding is a CHOICE WITHOUT A SOURCE: the page gives
 * "millimeters" and no rounding rule). Not compared with a real Windows (UNVERIFIED): a real display answers what its driver (or the 96 DPI
 * calculation of the page's note) gives. The host's screen and DPI are not read. */
#define FLE_GDC_IDX_HORZSIZE    4
#define FLE_GDC_IDX_VERTSIZE    6
#define FLE_GDC_HORZSIZE        508     /* millimetres */
#define FLE_GDC_VERTSIZE        286     /* millimetres (285.75 rounded) */
/* Decision 0212 (screen1, PROVISIONAL, opt-in): the four numbers above are the DEFAULTS. fle_screen.h makes the names the values of the run (FLE_MONITOR_W / FLE_MONITOR_H in the engine's environment; unset: these
 * defaults, so nothing differs); HORZSIZE and VERTSIZE follow the pixels at 96 DPI by the same rule. */
#include "fle_screen.h"
_Static_assert(FLE_GDC_HORZRES == FLE_SCREEN_DEFAULT_W && FLE_GDC_VERTRES == FLE_SCREEN_DEFAULT_H, "the defaults of fle_screen.h are the constants above");
_Static_assert(FLE_GDC_HORZSIZE == (FLE_GDC_HORZRES * 254 + 480) / 960 && FLE_GDC_VERTSIZE == (FLE_GDC_VERTRES * 254 + 480) / 960, "the physical size is the rule of fle_screen.h at the defaults");
#undef FLE_GDC_HORZRES
#undef FLE_GDC_VERTRES
#undef FLE_GDC_HORZSIZE
#undef FLE_GDC_VERTSIZE
#define FLE_GDC_HORZRES  FLE_SCREEN_PX_W
#define FLE_GDC_VERTRES  FLE_SCREEN_PX_H
#define FLE_GDC_HORZSIZE FLE_SCREEN_HORZSIZE_MM
#define FLE_GDC_VERTSIZE FLE_SCREEN_VERTSIZE_MM
_Static_assert(FLE_GDC_LOGPIXELSX == FLE_SCREEN_DPI_DEFAULT && FLE_GDC_LOGPIXELSY == FLE_SCREEN_DPI_DEFAULT, "the default DPI of fle_screen.h is the constant above");
#undef FLE_GDC_LOGPIXELSX
#undef FLE_GDC_LOGPIXELSY
#define FLE_GDC_LOGPIXELSX FLE_SCREEN_PX_DPI
#define FLE_GDC_LOGPIXELSY FLE_SCREEN_PX_DPI

/* user32!GetDC: HDC f(HWND hWnd). rcx. Only hWnd NULL (the DC of the entire screen) is implemented; every call returns a new
 * handle that has to be released.
 * user32!ReleaseDC: int f(HWND hWnd, HDC hDC). rcx, rdx. Returns 1.
 * gdi32!GetDeviceCaps: int f(HDC hdc, int index). rcx, edx. */
void fle_imp_GetDC(void);
void fle_imp_ReleaseDC(void);
void fle_imp_GetDeviceCaps(void);

/* Decision 0125 (PROVISIONAL, PM's decision 1 of 0125): the DCs of a WINDOW, state only (docs/decisions/0125-window-dc-and-walls.md).
 * user32!GetWindowDC: HDC f(HWND hWnd). rcx. user32!GetDCEx: HDC f(HWND hWnd, HRGN hrgnClip, DWORD flags). rcx, rdx, r8d. GetDC(hWnd) and ReleaseDC(hWnd, hDC) take a window of the table.
 * The kinds of a DC (the rules are in fle_gdc.c): FLE_GDC_K_SCREEN the DC of the entire screen (hWnd NULL); FLE_GDC_K_COMMON the common DC of a window's client area (a new handle every call, released by ReleaseDC);
 * FLE_GDC_K_OWN the private DC of a CS_OWNDC window (one per window, never released); FLE_GDC_K_CLASS the DC of a CS_CLASSDC class (one per class); FLE_GDC_K_WINDOW a common DC of the whole window rectangle
 * (GetWindowDC, GetDCEx with DCX_WINDOW). Class styles (Learn Window Class Styles: CS_OWNDC 0x0020, CS_CLASSDC 0x0040, CS_PARENTDC 0x0080) and the DCX_ flags of GetDCEx (Learn names them and gives no number;
 * UNOFFICIAL: windows-sys on docs.rs, 2026-10-31: DCX_WINDOW 1, DCX_CACHE 2, DCX_NORESETATTRS 4, DCX_CLIPCHILDREN 8, DCX_CLIPSIBLINGS 0x10, DCX_PARENTCLIP 0x20, DCX_EXCLUDERGN 0x40, DCX_INTERSECTRGN 0x80,
 * DCX_LOCKWINDOWUPDATE 0x400). Nothing is drawn: the surface of a DC is the top-level window (root_lid) and has no pixels. */
#define FLE_GDC_K_SCREEN 0
#define FLE_GDC_K_COMMON 1
#define FLE_GDC_K_OWN    2
#define FLE_GDC_K_CLASS  3
#define FLE_GDC_K_WINDOW 4
#define FLE_CS_OWNDC     0x0020u
#define FLE_CS_CLASSDC   0x0040u
#define FLE_CS_PARENTDC  0x0080u
#define FLE_DCX_WINDOW       0x00000001u
#define FLE_DCX_CACHE        0x00000002u
#define FLE_DCX_EXCLUDERGN   0x00000040u
#define FLE_DCX_INTERSECTRGN 0x00000080u
void fle_imp_GetWindowDC(void);
void fle_imp_GetDCEx(void);

/* Inspection (for tests and tools; no state moves): 1 and the record of a live DC of this table, 0 for any other handle. `hwnd` is the window the DC was got for (0 = the entire screen), `thread` the index of the
 * guest thread that got it, `class_style` the style of the window's class, `lid` the logical id of the window and `root_lid` that of its top-level window (the surface), cx and cy the window size at the call. */
#define FLE_GDC_PAINT_PIECES 32u
typedef struct {
    uint64_t hwnd;
    uint32_t kind, flags, thread, class_style, lid, root_lid;
    int32_t cx, cy;
    uint32_t from_ex;          /* decision 0142: made by GetDCEx (`flags` are then the clip flags of the DC) */
    uint32_t paint_valid;      /* decision 0142: inside BeginPaint / EndPaint: paint = the rectangle of the PAINTSTRUCT in client coordinates */
    int32_t paint[4];
    uint32_t paint_np;         /* decision 0247 (alpha3): the update region has more than one rectangle: paint_np disjoint rectangles pp (client coordinates, l, t, r, b), whose bounding box is `paint`; 0 = the region is the rectangle `paint` */
    int32_t pp[FLE_GDC_PAINT_PIECES][4];
} fle_gdc_info;
int fle_gdc_dc_info(uint64_t h, fle_gdc_info *out);
/* Decision 0142: BeginPaint / EndPaint set and lift the paint rectangle of the DC (the clipping to the update region). */
void fle_gdc_set_paint_rect(uint64_t h, int valid, int32_t l, int32_t t, int32_t r, int32_t b);
/* Decision 0247 (alpha3): after fle_gdc_set_paint_rect(h, 1, ...), the update region of the PAINTSTRUCT when it is not one rectangle: n (2..FLE_GDC_PAINT_PIECES) disjoint rectangles (l, t, r, b). EndPaint's fle_gdc_set_paint_rect(h, 0, ...) clears them. */
void fle_gdc_set_paint_pieces(uint64_t h, const int32_t (*p)[4], uint32_t n);

/* fle_win.c tells this file that a window was destroyed: the private DC of a CS_OWNDC window goes with it. */
void fle_gdc_window_destroyed(uint64_t hwnd);

/* DCs that were handed out and not released yet (for tests and tools), and whether `h` is one of them. Neither moves any state. */
uint64_t fle_gdc_live_count(void);
int fle_gdc_is_live(uint64_t h);

#endif

/* The virtual GDI object table: gdi32!CreatePalette, gdi32!DeleteObject, gdi32!GetStockObject and user32!LoadIconW
 * (docs/decisions/0054-virtual-gdi-objects.md; the pages read and the sources of every number are listed there and in
 * docs/investigations/gdi-objects-caller.md). Same convention as fle_imports.h: arguments in R[1] (rcx), R[2] (rdx); the Win32
 * return value in R[0]; the guest return address stays on the stack for the caller's ret. Single guest thread only (the same
 * rule as fle_gdc.h).
 *
 * A handle is a slot of one table of objects: {never issued, live, deleted} and a kind and contents. Nothing is read from the
 * host. The handle values are values of this lift environment (no page gives one), fixed, so that a run does not depend on the
 * machine it runs on.
 *
 * Only what the guest's initialization unit calls is answered: a palette of 16 entries, the stock objects 5, 7 and 13, the icon
 * IDI_APPLICATION. Every other argument, object kind or calling form stops the run through fle_stop (fle_env.h):
 * "fle: <api>: <why>" and abort(). A value is never made up for them. */
#ifndef FLE_GDI_H
#define FLE_GDI_H

#include <stdint.h>

#include "fle_raster.h"

/* Handles of the virtual GDI objects (decision 0054): FLE_GDI_HANDLE_BASE + 4 * (slot + 1). The pages give no value for a GDI
 * handle. The Learn page "Interprocess communication" (64-bit Windows programming) says that only the lower 32 bits of a handle
 * to a GDI object are significant, so the values stay below 0x80000000 (truncation and sign extension give the same 64-bit
 * value). The region lies after the one of the virtual screen DCs (fle_gdc.h: 0x7FA90000 + 1 MiB) and does not overlap it. */
#define FLE_GDI_HANDLE_BASE UINT64_C(0x7FBA0000)
#define FLE_GDI_REGION_SIZE UINT64_C(0x00100000)

/* Slots 0..FLE_GDI_STOCK_SLOTS-1: the stock objects, slot = the GetStockObject index, so a stock object has the same handle on
 * every call (choice: the page says "retrieves a handle to one of the stock ..." and does not say that two calls return the same
 * value). Slot FLE_GDI_ICON_SLOT: the icon of LoadIconW(NULL, IDI_APPLICATION), one value for every call (the LoadIconW page:
 * "LoadIcon loads the icon resource only if it has not been loaded; otherwise, it retrieves a handle to the existing resource").
 * Created palettes take the slots from FLE_GDI_FIRST_CREATED_SLOT up and are never reused. */
#define FLE_GDI_STOCK_SLOTS         32
#define FLE_GDI_ICON_SLOT           32
#define FLE_GDI_FIRST_CREATED_SLOT  33

/* GetStockObject indexes that the guest uses (call sites 0x5d26ba, 0x5d26cb, 0x5d26dc). The Learn page lists names only; the
 * numbers are from the Windows SDK header copy of mingw-w64 (wingdi.h): UNOFFICIAL (decision 0054). Every other index stops. */
#define FLE_GDI_STOCK_NULL_BRUSH   5
#define FLE_GDI_STOCK_BLACK_PEN    7
#define FLE_GDI_STOCK_SYSTEM_FONT  13

/* The icon: LoadIconW(NULL, IDI_APPLICATION). Learn "About Icons": IDI_APPLICATION is MAKEINTRESOURCE(32512). */
#define FLE_GDI_IDI_APPLICATION 32512

/* Decision 0069 (provisional): LoadIconW(NULL, L"MAINICON") returns NULL. The name is read as UTF-16 through fle_ld, at most this
 * many units (the string has to end with a NUL inside them, otherwise the run stops). */
#define FLE_GDI_ICON_NAME_MAX_UNITS 16

/* The only LOGPALETTE that is accepted: the guest's (16 entries, palVersion 0x300). The Learn page gives the structure and no
 * value for palVersion ("The version number of the system"); 0x300 is what the guest passes. */
#define FLE_GDI_PALETTE_VERSION 0x300
#define FLE_GDI_PALETTE_ENTRIES 16

/* Object kinds (for tests and tools). */
#define FLE_GDI_KIND_NONE        0   /* not a live object of this table */
#define FLE_GDI_KIND_PALETTE     1
#define FLE_GDI_KIND_STOCK_BRUSH 2
#define FLE_GDI_KIND_STOCK_PEN   3
#define FLE_GDI_KIND_STOCK_FONT  4
#define FLE_GDI_KIND_ICON        5
#define FLE_GDI_KIND_FONT        6   /* decision 0066: a logical font made by CreateFontIndirectW */
#define FLE_GDI_KIND_BITMAP      7   /* decision 0106: a device-dependent bitmap made by CreateBitmap (size and format only, no pixels) */
#define FLE_GDI_KIND_DC          8   /* decision 0108: a memory DC made by CreateCompatibleDC */
#define FLE_GDI_KIND_BRUSH       9   /* decision 0108: a logical brush made by CreateBrushIndirect (solid or hollow) */
#define FLE_GDI_KIND_DEFAULT    10   /* decision 0108: one of the four default objects (brush, pen, font, palette) every memory DC starts with */
#define FLE_GDI_KIND_ICONCUR    11   /* decision 0108: an icon or a cursor made by CreateIconIndirect */

/* Decision 0066: gdi32!CreateFontIndirectW adds the kind "font" to this table. A created font takes the next free created slot
 * like a palette (the same handle region, the same formula, no new region). sizeof(LOGFONTW) = 5 x 4 + 8 x 1 + 32 x 2 = 92 (the
 * Learn LOGFONTW page: LONG x 5, BYTE x 8, WCHAR lfFaceName[LF_FACESIZE = 32]); the table keeps a copy of the 92 bytes. */
#define FLE_GDI_LOGFONTW_SIZE 92

/* gdi32!CreatePalette: HPALETTE f(const LOGPALETTE *plpal). rcx.
 * gdi32!DeleteObject: BOOL f(HGDIOBJ ho). rcx.
 * gdi32!GetStockObject: HGDIOBJ f(int i). ecx.
 * user32!LoadIconW: HICON f(HINSTANCE hInstance, LPCWSTR lpIconName). rcx, rdx.
 * gdi32!CreateFontIndirectW: HFONT f(const LOGFONTW *lplf). rcx.
 * gdi32!GetObjectW: int f(HANDLE h, int c, LPVOID pv). rcx, edx, r8 (decision 0068): answers only a live created font with
 * c == sizeof(LOGFONTW) = 0x5c and a non-NULL pv (the copy of the 92 bytes is stored, 92 is returned); every other form stops. */
void fle_imp_CreatePalette(void);
void fle_imp_DeleteObject(void);
void fle_imp_GetStockObject(void);
void fle_imp_LoadIconW(void);
void fle_imp_CreateFontIndirectW(void);
void fle_imp_GetObjectW(void);

/* gdi32!CreateBitmap (decision 0106, PROVISIONAL): HBITMAP f(int nWidth ecx, int nHeight edx, UINT nPlanes r8d, UINT nBitCount r9d, const VOID *lpBits
 * [stack 5th]). The guest's form only (ILTools' bitmap-to-cursor function f_625ee30 makes the 1-bit mask of a cursor): nWidth and nHeight not negative
 * (zero gives the 1-by-1 monochrome bitmap the page names), nPlanes 1, nBitCount 1, lpBits NULL ("If this parameter is NULL, then the contents of the new bitmap are undefined": the table keeps the size and
 * the format and no pixels, so nothing can read them). The handle is a new slot of this table like a palette's. Every other form stops by name. A
 * bitmap is deleted by DeleteObject (no DC exists, so it is never "currently selected into a DC"). */
void fle_imp_CreateBitmap(void);

/* Inspection: the number of live bitmaps, and the width, height, planes and bit count of a live bitmap (1 and four values written to out; 0 otherwise). */
uint64_t fle_gdi_bitmap_live_count(void);
int fle_gdi_bitmap_info(uint64_t h, uint32_t out[4]);

/* gdi32!EnumFontFamiliesExW (decision 0098, PROVISIONAL): int f(HDC hdc rcx, LPLOGFONTW lpLogfont rdx, FONTENUMPROCW lpProc r8, LPARAM lParam r9, DWORD
 * dwFlags [stack]). The virtual fonts of this environment are one family, the system font of decision 0066 (face FLE_SPI_FONT_FACE), so a
 * call that asks for all families (lfCharSet DEFAULT_CHARSET, empty lfFaceName) or for that family by name calls lpProc once, through
 * fle_call_guest (decision 0064), as int f(const ENUMLOGFONTEXW *rcx, const NEWTEXTMETRICEXW *rdx, DWORD FontType r8, LPARAM r9), and returns the
 * value the callback returned. Both structures are written on the guest stack for the duration of the call; the members that are not decided are 0.
 * Every other form stops by name. The numbers FLE_GDI_ENUMLOGFONTEXW_SIZE and the member offsets are computed from the member types of the Learn
 * pages (LOGFONTW 92, WCHAR[64] + WCHAR[32] + WCHAR[32] = 128 + 64 + 64: 348; NEWTEXTMETRICW 76 + FONTSIGNATURE 24 = 100); TRUETYPE_FONTTYPE 4 is the
 * value of the Windows SDK header (mingw-w64 wingdi.h; the Learn page names it). */
#define FLE_GDI_ENUMLOGFONTEXW_SIZE  348
#define FLE_GDI_NEWTEXTMETRICEXW_SIZE 100
#define FLE_GDI_ELF_FULLNAME         92      /* after the LOGFONTW (92 bytes) */
#define FLE_GDI_ELF_STYLE            220
#define FLE_GDI_ELF_SCRIPT           284
#define FLE_GDI_LF_WEIGHT            16
#define FLE_GDI_LF_CHARSET           23
#define FLE_GDI_LF_PITCHANDFAMILY    27
#define FLE_GDI_LF_FACENAME          28
#define FLE_GDI_LF_FACESIZE          32
#define FLE_GDI_DEFAULT_CHARSET      1
#define FLE_GDI_TRUETYPE_FONTTYPE    4
void fle_imp_EnumFontFamiliesExW(void);

/* Inspection of fonts (for tests and tools; no state moves): the number of live created fonts, and the copy of the LOGFONTW of a
 * live created font (1 and 92 bytes written to out; 0 and nothing written for any other handle, a stock font included). */
uint64_t fle_gdi_font_live_count(void);
int fle_gdi_font_logfont(uint64_t h, uint8_t out[FLE_GDI_LOGFONTW_SIZE]);
/* Decision 0129: the handle of the font that is selected into the DC `hdc` (a window or screen DC, or a memory DC); 0 for any other value. For the record of the first text-drawing call (stop condition (d\')). */
uint64_t fle_gdi_dc_font(uint64_t hdc);

/* Inspection (for tests and tools; none of them moves any state). The kind of a live object (FLE_GDI_KIND_NONE otherwise), the
 * number of live objects, the number of live created palettes, the number of entries of a live palette (0 otherwise) and its
 * entry i as the 32-bit little-endian value peRed | peGreen << 8 | peBlue << 16 | peFlags << 24 (0 when out of range). */
int fle_gdi_kind(uint64_t h);
uint64_t fle_gdi_live_count(void);
uint64_t fle_gdi_palette_live_count(void);
uint32_t fle_gdi_palette_count(uint64_t h);
uint32_t fle_gdi_palette_entry(uint64_t h, uint32_t i);

/* ---- Decision 0108 (PROVISIONAL): memory DCs, DIB sections, brushes, selection (docs/decisions/0108-gdi-memory-dc.md) ----
 * ILTools' DllMain reaches gdi32!CreateCompatibleDC through the VCL bitmap canvas (TBitmapCanvas.CreateHandle). The model: a memory DC is a
 * slot of this table with the handles of what is selected into it, a text and a background colour, and a 1-by-1 monochrome placeholder
 * bitmap; a bitmap is a device-dependent bitmap (size and format, no pixels: decision 0106) or a DIB section (a zero-filled block of
 * pixels that the guest gets a pointer to); a brush is solid or hollow. Nothing here reads or writes a pixel: BitBlt, PatBlt, FillRect,
 * GetDIBits and RealizePalette stop by name. Same convention as above: arguments in R[1] (rcx), R[2] (rdx), R[8] (r8), R[9] (r9), the fifth
 * and later through fle_arg (fle_env.h); the Win32 return value in R[0]; the last error is left alone. Single guest thread only. */

/* Bitmap formats (gdi_obj.fmt of a FLE_GDI_KIND_BITMAP): the CreateBitmap form is monochrome; a bitmap compatible with the virtual display
 * (fle_gdc.h: BITSPIXEL 32, PLANES 1) is colour; a DIB section is made by CreateDIBSection. */
#define FLE_GDI_BMP_MONO  0
#define FLE_GDI_BMP_COLOR 1
#define FLE_GDI_BMP_DIB   2

/* The four default objects of a memory DC (fle_gdi_default_object). The pages do not say which objects a new DC starts with (choice). */
#define FLE_GDI_DEFAULT_BRUSH   0
#define FLE_GDI_DEFAULT_PEN     1
#define FLE_GDI_DEFAULT_FONT    2
#define FLE_GDI_DEFAULT_PALETTE 3

/* Colours a memory DC starts with (choice: no Learn page gives the initial text or background colour; these are white background and black
 * text, the values of every Windows DC as far as is known, UNVERIFIED), and the one system colour that is answered. COLOR_WINDOWTEXT is 8
 * (Learn GetSysColor lists the name and the number; no value). The value 0 (black) is a choice without a source (decision 0108). */
#define FLE_GDI_DC_TEXT_COLOR           0x00000000u
#define FLE_GDI_DC_BK_COLOR             0x00FFFFFFu
#define FLE_GDI_COLOR_WINDOWTEXT        8
#define FLE_GDI_COLOR_WINDOWTEXT_RGB    0x00000000u
/* Decision 0123: GetSysColor answers the indices 0..30 (FLE_GDI_SYS_COLOR_COUNT entries); every value is a choice without a source (fle_gdi.c). */
#define FLE_GDI_SYS_COLOR_COUNT         31

/* Structures (sizes from the member types on the Learn pages, x64): BITMAP = LONG x 4 + WORD x 2 + (4 padding) + LPVOID = 32; DIBSECTION =
 * BITMAP 32 + BITMAPINFOHEADER 40 + DWORD x 3 + (4) + HANDLE + DWORD + (4) = 104; BITMAPINFOHEADER = 40; LOGBRUSH = UINT + COLORREF +
 * ULONG_PTR = 16; ICONINFO = BOOL + DWORD x 2 + (4) + HBITMAP x 2 = 32. */
#define FLE_GDI_BITMAP_SIZE      32
#define FLE_GDI_DIBSECTION_SIZE  104
#define FLE_GDI_BMIH_SIZE        40
#define FLE_GDI_LOGBRUSH_SIZE    16
#define FLE_GDI_ICONINFO_SIZE    32
#define FLE_GDI_BI_RGB           0
#define FLE_GDI_BI_BITFIELDS     3
#define FLE_GDI_DIB_RGB_COLORS   0
#define FLE_GDI_DIB_PAL_COLORS   1
#define FLE_GDI_BS_SOLID         0
#define FLE_GDI_BS_HOLLOW        1
#define FLE_GDI_BS_NULL          1
#define FLE_GDI_BS_PATTERN       3   /* wingdi.h (UNOFFICIAL here: LOGBRUSH page names BS_PATTERN without the number): decision 0132 */

/* gdi32!CreateCompatibleDC: HDC f(HDC hdc). rcx. hdc NULL or a DC of GetDC(NULL): a new memory DC (all the DCs of this environment are compatible with the
 * one virtual display). The new DC has the placeholder bitmap selected, the four default objects, text colour FLE_GDI_DC_TEXT_COLOR and background colour
 * FLE_GDI_DC_BK_COLOR. A memory DC as hdc stops.
 * gdi32!DeleteDC: BOOL f(HDC hdc). rcx. A live memory DC: deleted, 1. NULL and a memory DC already deleted: 0. A DC of GetDC(NULL) stops (the page).
 * gdi32!SelectObject: HGDIOBJ f(HDC hdc, HGDIOBJ h). rcx, rdx. A bitmap, a brush, a pen or a font into a memory DC; the previous object of that type is returned.
 * gdi32!SelectPalette: HPALETTE f(HDC hdc, HPALETTE hPal, BOOL bForceBkgd). rcx, rdx, r8. A palette into a memory DC; the previous palette is returned.
 * gdi32!CreateCompatibleBitmap: HBITMAP f(HDC hdc, int cx, int cy). rcx, edx, r8d.
 * gdi32!CreateDIBSection: HBITMAP f(HDC hdc, const BITMAPINFO *pbmi, UINT usage, VOID **ppvBits, HANDLE hSection, DWORD offset). rcx, rdx, r8d, r9, then fle_arg(5), (6).
 * gdi32!GetDIBColorTable: UINT f(HDC hdc, UINT iStart, UINT cEntries, RGBQUAD *prgbq). gdi32!SetDIBColorTable: the same with a const array.
 * gdi32!SetTextColor, gdi32!SetBkColor: COLORREF f(HDC hdc, COLORREF color). rcx, edx.
 * gdi32!CreateBrushIndirect: HBRUSH f(const LOGBRUSH *plbrush). rcx. BS_SOLID and BS_HOLLOW only.
 * gdi32!CreateHalftonePalette: HPALETTE f(HDC hdc). rcx. An opaque palette: the page says the entries are an implementation detail.
 * user32!GetSysColor: DWORD f(int nIndex). ecx. The indices 0..30 (decision 0123: the default light-theme colours, a choice without a source); any other index 0.
 * user32!CreateIconIndirect: HICON f(PICONINFO piconinfo). rcx.
 * The five that stop by name: gdi32!BitBlt, gdi32!PatBlt, user32!FillRect, gdi32!GetDIBits, gdi32!RealizePalette. */
void fle_imp_CreateCompatibleDC(void);
void fle_imp_DeleteDC(void);
void fle_imp_SelectObject(void);
void fle_imp_SelectPalette(void);
void fle_imp_CreateCompatibleBitmap(void);
void fle_imp_CreateDIBSection(void);
void fle_imp_GetDIBColorTable(void);
void fle_imp_SetDIBColorTable(void);
void fle_imp_SetTextColor(void);
void fle_imp_SetBkColor(void);
void fle_imp_CreateBrushIndirect(void);
void fle_imp_CreateHalftonePalette(void);
void fle_imp_GetSysColor(void);
void fle_imp_CreateIconIndirect(void);
void fle_imp_BitBlt(void);
void fle_imp_PatBlt(void);
void fle_imp_FillRect(void);
void fle_imp_GetDIBits(void);
void fle_imp_RealizePalette(void);

/* Decision 0126 (PROVISIONAL): the pixel model (docs/decisions/0126-pixel-model-and-walls.md; fle_raster.h). Every drawing call below writes pixels: a memory DC writes the bitmap selected into it (a DIB section or a
 * device-dependent bitmap, which now keeps pixels), a DC of a window or of the screen writes the drawing surface of the top-level window (R10). gdi32!CreatePen: HPEN f(int iStyle ecx, int cWidth edx, COLORREF color r8d). */
void fle_imp_CreatePen(void);
void fle_imp_SetBitmapBits(void);

/* Inspection (for tests and tools; none of them moves any state). The number of live memory DCs and of live created brushes. The state of a live memory DC: 1 and
 * the handles selected into it (bitmap, brush, pen, font, palette) and its colours (text, background); 0 for any other handle. The placeholder bitmap of a live
 * memory DC (0 otherwise). The default object FLE_GDI_DEFAULT_* (0 before the first memory DC). The number of DCs an object is selected into. The format
 * FLE_GDI_BMP_* of a live bitmap (-1 for any other handle). A live DIB section: 1 and the pointer to its bits in *bits and width, |height|, bit count,
 * compression, stride and the number of colour table entries in out. Entry i of the colour table of a live DIB section as blue | green << 8 | red << 16 |
 * reserved << 24 (0 when out of range). A live created brush: 1 and its style and COLORREF. A live icon or cursor: 1 and fIcon, xHotspot, yHotspot, width and
 * height of the mask, and whether a colour bitmap was given. */
uint64_t fle_gdi_dc_live_count(void);
uint64_t fle_gdi_brush_live_count(void);
int fle_gdi_dc_state(uint64_t h, uint64_t sel[5], uint32_t col[2]);
uint64_t fle_gdi_dc_placeholder(uint64_t h);
uint64_t fle_gdi_default_object(uint32_t which);
uint32_t fle_gdi_selected_count(uint64_t h);
int fle_gdi_bitmap_format(uint64_t h);
int fle_gdi_dib_info(uint64_t h, uint64_t *bits, uint32_t out[6]);
/* Decision 0140: the pixel view of the bitmap selected into the memory DC `h` (1 and *out), or 0 (not a live memory DC with a bitmap). */
int fle_gdi_memdc_surface(uint64_t h, fle_rsurf *out);
uint32_t fle_gdi_dib_color(uint64_t h, uint32_t i);
int fle_gdi_brush_info(uint64_t h, uint32_t out[2]);
int fle_gdi_iconcur_info(uint64_t h, uint32_t out[6]);

/* ---- Decision 0111 (PROVISIONAL): DC attributes, pens, solid brushes, the DC stack, the pixel operations on DIB sections, cursors, named stops ----
 * (docs/decisions/0111-gdi-bulk.md). Same conventions as above: arguments in R[1] (rcx), R[2] (rdx), R[8] (r8), R[9] (r9), the fifth and later through
 * fle_arg; the Win32 return value in R[0]; the last error is left alone; single guest thread. A new memory DC starts with the values of the Learn page
 * "Display Device Context Defaults" (read as the "normal default values" of the CreateCompatibleDC page): background mode OPAQUE, drawing mode R2_COPYPEN,
 * mapping mode MM_TEXT, stretching mode BLACKONWHITE, current position, brush origin, window origin and viewport origin (0, 0). The numbers of the
 * constants are from the Windows SDK header copy of mingw-w64 (wingdi.h, winuser.h): UNOFFICIAL, the Learn pages give names only (as decisions 0053, 0054). */
#define FLE_GDI_KIND_PEN         12  /* decision 0111: a logical pen made by CreatePenIndirect (style, width, colour; nothing is drawn with it) */
#define FLE_GDI_KIND_REGION      13  /* decision 0123: a rectangular region made by CreateRectRgn / CreateRectRgnIndirect (one rectangle; combining regions and the clip regions of a DC are still named stops) */

#define FLE_GDI_TRANSPARENT      1
#define FLE_GDI_OPAQUE           2
#define FLE_GDI_R2_BLACK         1
#define FLE_GDI_R2_COPYPEN       13
#define FLE_GDI_R2_WHITE         16
#define FLE_GDI_MM_TEXT          1
#define FLE_GDI_MM_LOMETRIC      2       /* decision 0121: the numbers 1 to 8 are from the SDK header (mingw-w64 wingdi.h, windows-sys docs.rs): UNOFFICIAL; the API pages list names only.
                                          * Learn's [MS-WMF] and [MS-EMF] "MapMode Enumeration" (metafile formats) give MM_TEXT 1, MM_LOMETRIC 2, MM_HIMETRIC 3 ... MM_ANISOTROPIC 8: the same numbers */
#define FLE_GDI_MM_HIMETRIC      3
#define FLE_GDI_MM_ANISOTROPIC   8
#define FLE_GDI_BLACKONWHITE     1
#define FLE_GDI_WHITEONBLACK     2
#define FLE_GDI_COLORONCOLOR     3
#define FLE_GDI_HALFTONE         4
#define FLE_GDI_PS_SOLID         0
#define FLE_GDI_PS_NULL          5
#define FLE_GDI_PS_INSIDEFRAME   6
#define FLE_GDI_NULLREGION       1
#define FLE_GDI_SIMPLEREGION     2
#define FLE_GDI_COMPLEXREGION    3   /* decision 0247 (alpha3): GetUpdateRgn of a region of several rectangles */
#define FLE_GDI_LOGPEN_SIZE      16
#define FLE_GDI_PALETTEENTRY_SIZE 4
#define FLE_GDI_SRCCOPY          0x00CC0020u
#define FLE_GDI_PATCOPY          0x00F00021u
#define FLE_GDI_BLACKNESS        0x00000042u
#define FLE_GDI_WHITENESS        0x00FF0062u
/* The brush origin the page gives: "a set of coordinates with values between 0 and 7" (GetBrushOrgEx); SetBrushOrgEx reduces a larger value by the brush size,
 * which the pages do not give, so only these values are accepted. */
#define FLE_GDI_BRUSHORG_MAX     7

/* State of a memory DC (the part that is not in fle_imp_*): gdi32!MoveToEx BOOL f(HDC, int x, int y, LPPOINT lppt), GetCurrentPositionEx BOOL f(HDC, LPPOINT),
 * SetBkMode int f(HDC, int mode), SetROP2 int f(HDC, int rop2), SetMapMode int f(HDC, int mode), SetStretchBltMode int f(HDC, int mode), GetStretchBltMode int f(HDC),
 * SetWindowOrgEx and SetViewportOrgEx BOOL f(HDC, int x, int y, LPPOINT lppt), GetWindowOrgEx BOOL f(HDC, LPPOINT), LPtoDP BOOL f(HDC, LPPOINT, int c),
 * SetBrushOrgEx BOOL f(HDC, int x, int y, LPPOINT lppt), GetBrushOrgEx BOOL f(HDC, LPPOINT), SaveDC int f(HDC), RestoreDC BOOL f(HDC, int nSavedDC),
 * GdiFlush BOOL f(void), GetClipBox int f(HDC, LPRECT). Memory DCs only; every other handle stops by name. Decision 0121 (PROVISIONAL): a screen DC (GetDC(NULL)) also has the attribute
 * state (the same defaults) for the attribute calls (not GetClipBox): fle_gdi_screen_released(h) (called by ReleaseDC of fle_gdc.c) discards it. */
void fle_imp_MoveToEx(void);
void fle_imp_GetCurrentPositionEx(void);
void fle_imp_SetBkMode(void);
void fle_imp_SetROP2(void);
void fle_imp_SetMapMode(void);
void fle_imp_SetStretchBltMode(void);
void fle_imp_GetStretchBltMode(void);
void fle_imp_SetWindowOrgEx(void);
void fle_imp_GetWindowOrgEx(void);
void fle_imp_SetViewportOrgEx(void);
void fle_imp_LPtoDP(void);
void fle_imp_SetBrushOrgEx(void);
void fle_imp_GetBrushOrgEx(void);
void fle_imp_SaveDC(void);
void fle_imp_RestoreDC(void);
void fle_imp_GdiFlush(void);
void fle_imp_GetClipBox(void);

/* Objects: gdi32!UnrealizeObject BOOL f(HGDIOBJ), CreateSolidBrush HBRUSH f(COLORREF), CreatePenIndirect HPEN f(const LOGPEN *), GetPaletteEntries UINT f(HPALETTE,
 * UINT iStart, UINT cEntries, LPPALETTEENTRY). user32!DestroyIcon, DestroyCursor BOOL f(HICON), GetIconInfo BOOL f(HICON, PICONINFO), SetCursor HCURSOR f(HCURSOR),
 * GetCursor HCURSOR f(void), ShowCursor int f(BOOL). */
void fle_imp_UnrealizeObject(void);
void fle_imp_CreateSolidBrush(void);
/* Decision 0132 (PROVISIONAL), wall 19: gdi32!CreatePatternBrush: HBRUSH f(HBITMAP hbm rcx); see fle_gdi.c. */
void fle_imp_CreatePatternBrush(void);
void fle_imp_CreatePenIndirect(void);
void fle_imp_GetPaletteEntries(void);
void fle_imp_DestroyIcon(void);
void fle_imp_DestroyCursor(void);
void fle_imp_GetIconInfo(void);
void fle_imp_SetCursor(void);
void fle_imp_GetCursor(void);
void fle_imp_ShowCursor(void);

/* The pixel operations (fle_imp_PatBlt, fle_imp_FillRect, fle_imp_BitBlt, declared above) now write into DIB sections in the cases that have one result: a memory DC
 * with a 24- or 32-bit DIB section selected (PatBlt with PATCOPY and a solid brush, BLACKNESS, WHITENESS; FillRect with a solid brush) and BitBlt(SRCCOPY) between
 * two memory DCs with DIB sections of the same format (8, 16, 24 or 32 bits), mapping mode MM_TEXT, no clip region (none can be set: the region calls stop by name).
 * The unused byte of a 32-bit pixel is written as 0 by the fills (a CHOICE WITHOUT A SOURCE, decision 0111) and copied by BitBlt. Everything else stops by name. */

/* The 59 gdi32 and 10 user32 calls that need rasterization, glyphs, regions, metafiles or system data, answered by a named stop (fle_stop: "fle: <API>: <why>"). */
void fle_imp_LineTo(void);
void fle_imp_Rectangle(void);
void fle_imp_RoundRect(void);
void fle_imp_Ellipse(void);
void fle_imp_Pie(void);
void fle_imp_Chord(void);
void fle_imp_Arc(void);
void fle_imp_ArcTo(void);
void fle_imp_AngleArc(void);
void fle_imp_Polyline(void);
void fle_imp_Polygon(void);
void fle_imp_PolyBezier(void);
void fle_imp_PolyBezierTo(void);
void fle_imp_ExtFloodFill(void);
void fle_imp_FrameRgn(void);
void fle_imp_GetPixel(void);
void fle_imp_SetPixel(void);
void fle_imp_SetPixelV(void);
void fle_imp_GetBitmapBits(void);
void fle_imp_SetDIBits(void);
void fle_imp_SetDIBitsToDevice(void);
void fle_imp_StretchDIBits(void);
void fle_imp_StretchBlt(void);
void fle_imp_MaskBlt(void);
void fle_imp_CreateDIBitmap(void);
void fle_imp_ExtTextOutW(void);
void fle_imp_GetTextMetricsW(void);
void fle_imp_GetTextExtentPoint32W(void);
void fle_imp_GetTextExtentPointW(void);
void fle_imp_SelectClipRgn(void);
void fle_imp_ExtSelectClipRgn(void);
void fle_imp_IntersectClipRect(void);
void fle_imp_ExcludeClipRect(void);
void fle_imp_GetClipRgn(void);
void fle_imp_RectVisible(void);
void fle_imp_GetRandomRgn(void);
void fle_imp_CreateRectRgn(void);
void fle_imp_CreateRectRgnIndirect(void);
void fle_imp_SetRectRgn(void);
void fle_imp_CombineRgn(void);
void fle_imp_GetRgnBox(void);
/* Decision 0126 (wall 91): 1 and the bounding box of a live region of this file (0, 0, 0, 0 when it is empty), 0 for any other handle. */
int fle_gdi_region_bounds(uint64_t h, int32_t *l, int32_t *t, int32_t *r, int32_t *b);
/* Decision 0126 (wall 93): replaces a live region of this file by one rectangle (the empty region for an empty rectangle); 0 for any other handle or a coordinate out of range. */
int fle_gdi_region_set_rect(uint64_t h, int32_t l, int32_t t, int32_t r, int32_t b);
/* Decision 0247 (alpha3), for the window table: the disjoint rectangles of region `h` into out[max] (l, t, r, b), *n = their number; 1, 0 when `h` is not a live region, -1 when it has more than `max` rectangles. */
int fle_gdi_region_pieces(uint64_t h, int32_t (*out)[4], uint32_t max, uint32_t *n);
/* Decision 0247 (alpha3), for GetUpdateRgn: replaces region `h` by the union of the n disjoint rectangles p (empty ones are ignored), kept as bands; 1, or 0 when `h` is not a live region or a coordinate is out of range. */
int fle_gdi_region_set_pieces(uint64_t h, const int32_t (*p)[4], uint32_t n);
/* Decision 0126 (wall 96): 1 when the rectangle lies entirely inside the region, 0 when it does not, -1 when the handle is not a live region of this file. */
int fle_gdi_region_covers(uint64_t h, int32_t l, int32_t t, int32_t r, int32_t b);
/* Decision 0123 (wall 49): PtInRegion, a new answer (the region table of fle_gdi.c: regions are sets of rectangles in Y-X bands; the clip regions of a DC stay named stops). */
void fle_imp_PtInRegion(void);
void fle_imp_OffsetRgn(void);
void fle_imp_EqualRgn(void);
void fle_imp_RectInRegion(void);
void fle_imp_GetRegionData(void);
void fle_imp_GetNearestPaletteIndex(void);
void fle_imp_ResizePalette(void);
void fle_imp_GetSystemPaletteEntries(void);
void fle_imp_GetDCOrgEx(void);
void fle_imp_GetEnhMetaFileHeader(void);
void fle_imp_PlayEnhMetaFile(void);
void fle_imp_GetEnhMetaFileBits(void);
void fle_imp_GetEnhMetaFilePaletteEntries(void);
void fle_imp_GetWinMetaFileBits(void);
void fle_imp_GetEnhMetaFileDescriptionW(void);
void fle_imp_DeleteEnhMetaFile(void);
void fle_imp_SetEnhMetaFileBits(void);
void fle_imp_CopyEnhMetaFileW(void);
void fle_imp_SetWinMetaFileBits(void);
void fle_imp_FrameRect(void);
void fle_imp_DrawEdge(void);
void fle_imp_DrawFocusRect(void);
void fle_imp_DrawFrameControl(void);
void fle_imp_DrawTextW(void);
void fle_imp_DrawTextExW(void);
void fle_imp_DrawIcon(void);
void fle_imp_DrawIconEx(void);
void fle_imp_CreateIcon(void);
void fle_imp_LoadBitmapW(void);

/* Inspection (for tests and tools; none of them moves any state). A live memory DC: 1 and out = background mode, drawing mode, mapping mode, stretching mode,
 * current position x y, window origin x y, viewport origin x y, brush origin x y, the depth of the DC stack (13 values); 0 for any other handle. A live created pen:
 * 1 and style, width, COLORREF; the number of live created pens. The current cursor of SetCursor (0 = none) and the display count of ShowCursor. The answer of
 * GetDeviceCaps for a live memory DC (the optional hook of decision 0111, not wired to the row by default): 1 and *out for BITSPIXEL (12) and PLANES (14) of the
 * bitmap selected into the DC, a stop by name for any other index, 0 when h is not a live memory DC. */
int fle_gdi_dc_attr(uint64_t h, int32_t out[13]);
int fle_gdi_pen_info(uint64_t h, uint32_t out[3]);
uint64_t fle_gdi_pen_live_count(void);
uint64_t fle_gdi_cursor_current(void);
int32_t fle_gdi_cursor_display_count(void);
int fle_gdi_memdc_caps(uint64_t h, uint32_t idx, int32_t *out);

/* Decision 0121: ReleaseDC of a screen DC calls this (a weak reference in fle_gdc.c): the attribute state of that GetDC handle is lost (Display Device Context Defaults page). */
void fle_gdi_screen_released(uint64_t h);
/* The attribute state of a live screen DC as fle_gdi_dc_attr gives it for a memory DC (13 values); 0 when h is not a live screen DC. A screen DC that no attribute call has touched has the defaults. */
int fle_gdi_screen_attr(uint64_t h, int32_t out[13]);
/* The number of screen DCs that hold an attribute state at this moment (a state is made by the first attribute call on a live screen DC and dropped by ReleaseDC). For tests and tools. */
uint32_t fle_gdi_screen_states(void);

#endif

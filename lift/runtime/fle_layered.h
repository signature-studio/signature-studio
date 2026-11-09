/* Layered windows (decision 0140, PROVISIONAL; docs/decisions/0140-layered-windows.md): user32!UpdateLayeredWindow, UpdateLayeredWindowIndirect, SetLayeredWindowAttributes and GetLayeredWindowAttributes.
 * The guest takes the first three with GetProcAddress (Delphi unit initialisation at 0x81c8a0, 0x65ccba, 0x86cabb) and TLightweightLayeredControl (f_71e570) calls UpdateLayeredWindow(Indirect).
 * Same convention as fle_win.h: arguments in R[1], R[2], R[8], R[9] and from the fifth on on the guest stack (fle_arg); the Win32 return value in R[0]; the guest return address stays on the stack.
 *
 * The pixel model (decision 0126) is not changed. A layered window that UpdateLayeredWindow supplies content to keeps that content in the drawing surface of its top-level window (fle_raster.c, one surface per
 * top-level window, the size of the window): 32-bit top-down, bytes B, G, R, A in memory with A = the alpha of the pixel and B, G, R PREMULTIPLIED by it (Learn BLENDFUNCTION, AC_SRC_ALPHA). The unused byte of
 * the other window surfaces is 0 ("BGRX"); the byte of a layered window's surface is a real alpha. Nothing is composed over the windows underneath: a Presenter does that from the layer attributes below.
 * fle_layered_query gives those attributes. */
#ifndef FLE_LAYERED_H
#define FLE_LAYERED_H

#include <stdint.h>
#include <stdio.h>

/* Learn UpdateLayeredWindow / SetLayeredWindowAttributes / BLENDFUNCTION / Extended Window Styles: ULW_COLORKEY 1, ULW_ALPHA 2, ULW_OPAQUE 4, ULW_EX_NORESIZE 8 (UpdateLayeredWindowIndirect only), LWA_COLORKEY 1,
 * LWA_ALPHA 2, AC_SRC_OVER 0, AC_SRC_ALPHA 1, WS_EX_LAYERED 0x00080000. */
#define FLE_ULW_COLORKEY    0x1u
#define FLE_ULW_ALPHA       0x2u
#define FLE_ULW_OPAQUE      0x4u
#define FLE_ULW_EX_NORESIZE 0x8u
#define FLE_LWA_COLORKEY    0x1u
#define FLE_LWA_ALPHA       0x2u
#define FLE_AC_SRC_OVER     0u
#define FLE_AC_SRC_ALPHA    1u
#define FLE_WS_EX_LAYERED   0x00080000u
/* UPDATELAYEREDWINDOWINFO on x64 (Learn lists the members; offsets derived from the member types): cbSize @0, hdcDst @8, pptDst @16, psize @24, hdcSrc @32, pptSrc @40, crKey @48, pblend @56, dwFlags @64, prcDirty @72 = 80 bytes. */
#define FLE_ULWINFO_SIZE 80u

/* How a layered window was set up (Learn: after SetLayeredWindowAttributes, UpdateLayeredWindow fails until the layering bit is cleared and set again; GetLayeredWindowAttributes fails if the window was set up with
 * UpdateLayeredWindow). NONE = WS_EX_LAYERED and neither call yet (Learn: "the layered window will not become visible until ... has been called"). */
#define FLE_LAYER_NONE 0
#define FLE_LAYER_ULW  1
#define FLE_LAYER_SLWA 2

typedef struct {
    int mode;                   /* FLE_LAYER_NONE, FLE_LAYER_ULW, FLE_LAYER_SLWA */
    uint32_t lwa_flags;         /* SLWA: the dwFlags given (FLE_LWA_*) */
    uint32_t alpha;             /* the constant alpha 0..255 for the whole window: ULW: SourceConstantAlpha of the last blend function (255 for ULW_OPAQUE, ULW_COLORKEY and no blend); SLWA: bAlpha when LWA_ALPHA, else 255 */
    uint32_t colorkey;          /* SLWA with LWA_COLORKEY: the key as a COLORREF 0x00BBGGRR; ULW: 0 (the key is applied to the pixels: their alpha is 0 where they equal it) */
    uint32_t per_pixel;         /* ULW: 1 when the bytes of the surface hold an alpha (always, once pixels were supplied: A = 255 where the source has none); SLWA and NONE: 0 (the fourth byte is 0 and means nothing) */
    uint32_t width, height;     /* ULW: the size of the last content (the window size then) */
    uint64_t updates;           /* ULW calls that supplied content */
} fle_layered_info;

void fle_imp_UpdateLayeredWindow(void);
void fle_imp_UpdateLayeredWindowIndirect(void);
void fle_imp_SetLayeredWindowAttributes(void);
void fle_imp_GetLayeredWindowAttributes(void);

/* Inspection and hooks. fle_layered_query: 1 and the state when `hwnd` is a live window with WS_EX_LAYERED (mode NONE if no call was made yet), else 0. fle_layered_exstyle_changed: called by SetWindowLongPtrW(GWL_EXSTYLE)
 * of fle_win.c (weak): a change of the WS_EX_LAYERED bit in either direction forgets the state ("until the layering style bit is cleared and set again"). fle_layered_window_destroyed: called when a window is destroyed.
 * fle_layered_report: one line per layered window with a state. */
int fle_layered_query(uint64_t hwnd, fle_layered_info *out);
void fle_layered_exstyle_changed(uint64_t hwnd, uint32_t old_ex, uint32_t new_ex);
void fle_layered_window_destroyed(uint64_t hwnd);
void fle_layered_report(FILE *f);

#endif

/* The pixel model (decision 0126, PROVISIONAL; docs/decisions/0126-pixel-model-and-walls.md). Pure C: no guest memory, no handle, no DC, no fle_* header. fle_gdi.c turns a DC into a surface view and a clip
 * rectangle and calls these functions; this file knows only pixels. Formats that can be WRITTEN: 24-bit BGR and 32-bit BGRX (the byte order of RGBTRIPLE and RGBQUAD: a READING, the pages give no sentence for
 * the BI_RGB layout, decision 0111). Formats that can be READ: those, and 1-, 4- and 8-bit with a colour table (the leftmost pixel is the most significant bits of the first byte: Learn "Bitmap Storage"). A colour is a COLORREF 0x00BBGGRR. The unused byte of a 32-bit pixel is written as 0 by every operation except
 * a SRCCOPY between two surfaces of the same format, which copies the bytes (decision 0111). The drawing surface of a window (one per top-level window, R10 of docs/investigations/detach-window.md) is kept here
 * by the logical id of the top-level window (lid 0 = the desktop, the screen DC): a 32-bit top-down BGRX surface whose initial bytes are all zero (S-surface-init of decision 0126). */
#ifndef FLE_RASTER_H
#define FLE_RASTER_H

#include <stdint.h>
#include <stdio.h>

typedef struct {
    int64_t x0, y0, x1, y1;                 /* [x0, x1) x [y0, y1) */
} fle_rrect;

typedef struct {
    uint8_t *bits;
    uint32_t width, height;                 /* pixels and image rows */
    uint32_t stride;                        /* bytes between two memory rows */
    uint32_t bpp, comp;                     /* bits per pixel; comp 0 = BI_RGB, 3 = BI_BITFIELDS */
    uint32_t masks[3];                      /* BI_BITFIELDS */
    uint32_t ncolors;                       /* colour table entries (bpp <= 8) */
    const uint32_t *clr;                    /* RGBQUAD of an entry as blue | green << 8 | red << 16 | reserved << 24 */
    int bottom_up;                          /* memory row 0 is the bottom image row (a DIB with a positive height) */
    int keep_x;                             /* a DIB section: a plain SRCCOPY between two such surfaces copies the unused byte of a 32-bit pixel; a window surface or a DDB has none */
    /* Colour conversion between a monochrome and a colour surface (Learn StretchBlt: "If StretchBlt must convert a monochrome bitmap to a color bitmap, it sets white bits (1) to the background color and black bits (0)
     * to the foreground color. To convert a color bitmap to a monochrome bitmap, it sets pixels that match the background color to white (1) and sets all other pixels to black (0). The foreground and background colors
     * of the device context with color are used": decision 0126 applies it to BitBlt too, whose page says "the BitBlt function converts the source color format to match the destination format"). conv_set on a
     * monochrome SOURCE: its bit 0 reads as conv_fg and its bit 1 as conv_bg when it is copied into a colour destination. mono_key_set on a monochrome DESTINATION: a colour source pixel equal to mono_key becomes 1, any
     * other becomes 0. */
    int conv_set, mono_key_set;
    uint32_t conv_fg, conv_bg, mono_key;
} fle_rsurf;

/* Raster operations. A ternary code (Learn "Ternary Raster Operations": 256 codes, the high word is the operation index, the index is the result of the Boolean operation on P, S and D = the bits of
 * 0xF0, 0xCC and 0xAA) is valid when it is one of the 256 of the page; a binary one (mix mode, SetROP2) is R2_* 1..16 (the numbers are the SDK's: UNOFFICIAL). Colours are 24 bits; each bit plane is combined alike. */
#define FLE_R_SRCCOPY_IDX   0xCCu
#define FLE_R_PATCOPY_IDX   0xF0u
#define FLE_R_BLACKNESS_IDX 0x00u
#define FLE_R_WHITENESS_IDX 0xFFu
#define FLE_R_DSTINVERT_IDX 0x55u
int fle_r_rop3_valid(uint32_t code, uint32_t *idx);                          /* `code` without the flag bits (0x40000000, 0x80000000) */
uint32_t fle_r_rop3_code(uint32_t idx);                                      /* the code of the page for an index */
int fle_r_idx_uses_source(uint32_t idx);
int fle_r_idx_uses_pattern(uint32_t idx);
int fle_r_idx_uses_dest(uint32_t idx);
uint32_t fle_r_rop3(uint32_t idx, uint32_t p, uint32_t s, uint32_t d);       /* the 24 colour bits */
int fle_r_rop2_valid(uint32_t mode);
uint32_t fle_r_rop2(uint32_t mode, uint32_t p, uint32_t d);

uint8_t *fle_r_row(const fle_rsurf *s, uint32_t y);                          /* image row y (0 = top) */
int fle_r_direct(const fle_rsurf *s);                                        /* 24-bit, or 32-bit BI_RGB or with the 8-8-8 masks: can be written */
int fle_r_writable(const fle_rsurf *s);                                      /* direct, or 1-bit (a bit is 1 when the colour put is not 0) */
int fle_r_readable(const fle_rsurf *s);                                      /* direct, or 8-bit with a colour table */
int fle_r_same_format(const fle_rsurf *a, const fle_rsurf *b);
uint32_t fle_r_get(const fle_rsurf *s, uint32_t x, uint32_t y);              /* readable surface, inside */
void fle_r_put(const fle_rsurf *s, uint32_t x, uint32_t y, uint32_t cref);   /* writable surface, inside; the unused byte becomes 0; a 1-bit surface takes bit 1 for a colour that is not 0 */
fle_rrect fle_r_bounds(const fle_rsurf *s);
void fle_r_clip(fle_rrect *r, const fle_rrect *c);                           /* r = r intersect c */
int fle_r_empty(const fle_rrect *r);

/* A clip given as a list of rectangles (the application-defined clipping region of a DC in the normal form of decision 0123, intersected with the visible rectangle of the target): each drawing function takes one
 * rectangle; fle_raster_cliplist is the set the caller loops over. `n` = 0 means that nothing can be drawn. */
#define FLE_R_CLIPLIST_MAX 65u
typedef struct {
    uint32_t n;
    fle_rrect r[FLE_R_CLIPLIST_MAX];
} fle_rcliplist;

/* Operations on a direct destination, clipped to `clip` (may be NULL) and to the surface. The rectangles and points are surface coordinates. Return the number of pixels written. */
uint64_t fle_r_rop_rect(const fle_rsurf *d, const fle_rrect *clip, fle_rrect r, uint32_t idx, uint32_t pat);                   /* ternary operation without a source (PatBlt, FillRect, InvertRect) */
uint64_t fle_r_mix_rect(const fle_rsurf *d, const fle_rrect *clip, fle_rrect r, uint32_t rop2, uint32_t pen);                  /* a pen colour with a mix mode */
uint64_t fle_r_blit(const fle_rsurf *d, const fle_rrect *clip, int64_t dx, int64_t dy, int64_t w, int64_t h, const fle_rsurf *s, int64_t sx, int64_t sy, uint32_t idx, uint32_t pat);
/* Decision 0245 (alpha2): after fle_r_changed_begin(), the plain 32-bit copies of fle_r_blit compare what they write; fle_r_changed(&r) says whether the destination changed (0 = no, exactly as before) and narrows r to the rows from the
 * first one that differed (1; alpha2b: to the end of the call that found it, and the later calls whole). See fle_raster.c. */
void fle_r_changed_begin(void);
int fle_r_changed(fle_rrect *r);
/* mode = the stretching mode of the DC (1 BLACKONWHITE = AND of the pixels that a shrink merges, 2 WHITEONBLACK = OR, 3 COLORONCOLOR = delete, 4 HALFTONE = mean of the colours); a pixel that is not merged is copied. */
/* MaskBlt (Learn): per destination pixel, mask bit 1 applies the foreground ternary operation fg, mask bit 0 the background one bg. `mask` is a 1-bit surface read at (mx + i, my + j). Returns the pixels written. */
uint64_t fle_r_maskblt(const fle_rsurf *d, const fle_rrect *clip, int64_t dx, int64_t dy, int64_t w, int64_t h, const fle_rsurf *s, int64_t sx, int64_t sy, const fle_rsurf *mask, int64_t mx, int64_t my, uint32_t fg, uint32_t bg, uint32_t pat);
uint64_t fle_r_stretch(const fle_rsurf *d, const fle_rrect *clip, int64_t dx, int64_t dy, int64_t dw, int64_t dh, const fle_rsurf *s, int64_t sx, int64_t sy, int64_t sw, int64_t sh, uint32_t idx, uint32_t pat, uint32_t mode);
uint64_t fle_r_line(const fle_rsurf *d, const fle_rrect *clip, int64_t x0, int64_t y0, int64_t x1, int64_t y1, uint32_t pen, uint32_t rop2);   /* start included, end excluded; 0 when longer than FLE_R_LINE_MAX */
#define FLE_R_LINE_MAX 1048576

/* The drawing surface of a top-level window. fle_raster_window_surface makes it on first use with the given size (a size change keeps the overlapping pixels, the rest is zero); NULL when the size is not
 * acceptable (0, above 16384 in a direction, or 2 GiB or more) or memory is short. fle_raster_wrote(s, r, api): the rectangle r of s (clipped) was written by `api`: the first time a window surface holds a
 * pixel whose three colour bytes are not all 0 (the first non-background pixel), a block is written to stderr (and kept for the stop report): the window table (fle_win_describe, weak) and the summary of the
 * surface. A call to a window that is not visible is discarded (S-draw-hidden of decision 0126: its visible region is empty): fle_raster_discarded counts it and the first one is written to stderr. */
fle_rsurf *fle_raster_window_surface(uint32_t lid, uint32_t w, uint32_t h);
void fle_raster_wrote(const fle_rsurf *s, fle_rrect r, const char *api);
void fle_raster_discarded(const char *api, uint32_t lid);                    /* a drawing call to a window that is not visible: nothing is written (S-draw-hidden) */
void fle_raster_window_destroyed(uint32_t lid);

/* Inspection and reports. fle_raster_describe prints one line for the surface of `lid` (size, calls that wrote, non-zero pixels, distinct colours, the most frequent colours); 0 when there is none. */
int fle_raster_describe(FILE *f, uint32_t lid);
void fle_raster_report(FILE *f);                                             /* every live surface and the counters, for the stop report */
uint32_t fle_raster_surface_count(void);
uint64_t fle_raster_discarded_count(void);
int fle_raster_evidence_seen(void);
const fle_rsurf *fle_raster_surface_peek(uint32_t lid);
uint64_t fle_raster_nonzero_pixels(const fle_rsurf *s);                      /* pixels whose three colour bytes are not all 0 */
uint32_t fle_raster_live_lids(uint32_t *out, uint32_t cap);                       /* decision 0129: the lids of the live surfaces, in creation order; returns how many were written */

#endif

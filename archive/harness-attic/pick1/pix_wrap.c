/* pick1 (decision 0242): link-time counters of the runtime's raster primitives (native build only; -Wl,--wrap=fle_r_*). One 16-byte record per call: u32 virtual clock ms, u32 kind, u64 pixels written (the return value of the primitive).
 * $PARITY_BLIT_LOG is the file (the same name and record layout as lift/harness/parity/parity_wrap.c so that blitvol.py reads it); kinds 1 blit, 2 rop_rect, 3 mix_rect, 4 stretch, 5 maskblt, 6 line.
 * It counts only; the primitives are called with the same arguments and return the same value. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include "fle_raster.h"
extern uint64_t fle_sys_clock_ticks(void);
typedef struct { uint32_t ms, kind; uint64_t px; } rec;
static FILE *lf; static int lfopen; static size_t nrec;
static void add(uint32_t kind, uint64_t px)
{
    rec r;
    if (!lfopen) { const char *p = getenv("PARITY_BLIT_LOG"); lfopen = 1; if (p && *p) lf = fopen(p, "wb"); }
    if (!lf) return;
    r.ms = (uint32_t)(fle_sys_clock_ticks() / 10000u); r.kind = kind; r.px = px;
    fwrite(&r, sizeof r, 1, lf);
    if ((++nrec & 255u) == 0) fflush(lf);
}
uint64_t __real_fle_r_blit(const fle_rsurf *d, const fle_rrect *clip, int64_t dx, int64_t dy, int64_t w, int64_t h, const fle_rsurf *s, int64_t sx, int64_t sy, uint32_t idx, uint32_t pat);
uint64_t __wrap_fle_r_blit(const fle_rsurf *d, const fle_rrect *clip, int64_t dx, int64_t dy, int64_t w, int64_t h, const fle_rsurf *s, int64_t sx, int64_t sy, uint32_t idx, uint32_t pat) { uint64_t r = __real_fle_r_blit(d, clip, dx, dy, w, h, s, sx, sy, idx, pat); add(1, r); return r; }
uint64_t __real_fle_r_rop_rect(const fle_rsurf *d, const fle_rrect *clip, fle_rrect r, uint32_t idx, uint32_t pat);
uint64_t __wrap_fle_r_rop_rect(const fle_rsurf *d, const fle_rrect *clip, fle_rrect r, uint32_t idx, uint32_t pat) { uint64_t x = __real_fle_r_rop_rect(d, clip, r, idx, pat); add(2, x); return x; }
uint64_t __real_fle_r_mix_rect(const fle_rsurf *d, const fle_rrect *clip, fle_rrect r, uint32_t rop2, uint32_t pen);
uint64_t __wrap_fle_r_mix_rect(const fle_rsurf *d, const fle_rrect *clip, fle_rrect r, uint32_t rop2, uint32_t pen) { uint64_t x = __real_fle_r_mix_rect(d, clip, r, rop2, pen); add(3, x); return x; }
uint64_t __real_fle_r_stretch(const fle_rsurf *d, const fle_rrect *clip, int64_t dx, int64_t dy, int64_t dw, int64_t dh, const fle_rsurf *s, int64_t sx, int64_t sy, int64_t sw, int64_t sh, uint32_t idx, uint32_t pat, uint32_t mode);
uint64_t __wrap_fle_r_stretch(const fle_rsurf *d, const fle_rrect *clip, int64_t dx, int64_t dy, int64_t dw, int64_t dh, const fle_rsurf *s, int64_t sx, int64_t sy, int64_t sw, int64_t sh, uint32_t idx, uint32_t pat, uint32_t mode) { uint64_t x = __real_fle_r_stretch(d, clip, dx, dy, dw, dh, s, sx, sy, sw, sh, idx, pat, mode); add(4, x); return x; }
uint64_t __real_fle_r_maskblt(const fle_rsurf *d, const fle_rrect *clip, int64_t dx, int64_t dy, int64_t w, int64_t h, const fle_rsurf *s, int64_t sx, int64_t sy, const fle_rsurf *mask, int64_t mx, int64_t my, uint32_t fg, uint32_t bg, uint32_t pat);
uint64_t __wrap_fle_r_maskblt(const fle_rsurf *d, const fle_rrect *clip, int64_t dx, int64_t dy, int64_t w, int64_t h, const fle_rsurf *s, int64_t sx, int64_t sy, const fle_rsurf *mask, int64_t mx, int64_t my, uint32_t fg, uint32_t bg, uint32_t pat) { uint64_t x = __real_fle_r_maskblt(d, clip, dx, dy, w, h, s, sx, sy, mask, mx, my, fg, bg, pat); add(5, x); return x; }
uint64_t __real_fle_r_line(const fle_rsurf *d, const fle_rrect *clip, int64_t x0, int64_t y0, int64_t x1, int64_t y1, uint32_t pen, uint32_t rop2);
uint64_t __wrap_fle_r_line(const fle_rsurf *d, const fle_rrect *clip, int64_t x0, int64_t y0, int64_t x1, int64_t y1, uint32_t pen, uint32_t rop2) { uint64_t x = __real_fle_r_line(d, clip, x0, y0, x1, y1, pen, rop2); add(6, x); return x; }

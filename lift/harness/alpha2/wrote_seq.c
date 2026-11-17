/* Decision 0245 (alpha2, test tool; the scaffold of harness/alpha1/glass_seq.c): a plain copy that leaves the destination as it was is not told to the compositor.
 * Part 1 calls fle_r_blit with fle_r_changed_begin / fle_r_changed on surfaces made here; part 2 draws into real windows (BitBlt, StretchDIBits, SetDIBitsToDevice) and counts the calls of fle_desk_wrote (-Wl,--wrap=fle_desk_wrote).
 * The expected values are what the rule says (same bytes: 0; some byte differs: 1 and the rows from the first one that differs to the end of the call, alpha2b), not read back from the code. Scaffold text follows. (Decision 0244 alpha1, test tool; scaffold of harness/txt1/create_seq.c): windows whose frame is extended into the client area (DwmExtendFrameIntoClientArea), the alpha byte that a BitBlt from a 32-bit DIB leaves in their surface,
 * the desktop compositor (fle_desk.c) and SetWindowRgn (the part of the window outside its region is not shown and is not hit), run on the runtime itself (fle_win.c, fle_gdi.c, fle_raster.c, fle_dwm.c ...).
 * The expected pixels are PIX / OVER_RED / OVER_BG, made by tools/mk-expect.py from the formula of Microsoft Learn (BLENDFUNCTION, AC_SRC_ALPHA: Dst = Src + (1 - Src.A) * Dst, premultiplied) in exact rational arithmetic;
 * the script does not read the C code. The build and the comparison are lift/crates/fe-lift/tests/alpha1_glass.rs. Prints one line per check ("ok" / "FAIL"); a failure ends with exit status 1. */
#define _GNU_SOURCE
#include <stdio.h>
#include <string.h>
#include <stdint.h>
#include "fle_runtime.h"
#include "fle_sse.h"
#include "fle_dispatch.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_callback.h"
#include "fle_class.h"
#include "fle_win.h"
#include "fle_gdi.h"
#include "fle_thread.h"
#include "fle_cursor.h"
#include "fle_wall_ia2.h"
#include "fle_api2.h"
#include "fle_desk.h"
#include "fle_dwm.h"
#include "fle_gdc.h"
#include "fle_layered.h"
#include "fle_input.h"
#include "fle_raster.h"
#include "fle_screen.h"

uint64_t R[16];
fle_eflags FL;
fle_xmm X[16];
void fle_imp_PeekMessageW(void);
void fle_imp_DispatchMessageW(void);
typedef uint64_t HWND;
#define A(x) ((uint64_t)(uintptr_t)(x))
#define VA_WP 0x2000
#define WS_POPUP 0x80000000u
#define WS_VISIBLE 0x10000000u

static void wndproc(void) { R[0] = fle_api2_call(fle_imp_DefWindowProcW, R[1], R[2], R[8], R[9], 0, 0, 0, 0); }
const fle_fn_entry fle_dispatch_table[] = { { VA_WP, wndproc } };
const uint64_t fle_dispatch_count = 1;
static uint64_t stk[0x4000] __attribute__((aligned(4096)));
static uint16_t clsname[] = { 'g', 'l', 'a', 0 };

static void regclass(void)
{
    static uint8_t wc[72];
    uint64_t proc = VA_WP, inst = 0x400000, nmp = A(clsname), cur = FLE_CURSOR_HANDLE_BASE + 4u;
    uint32_t style = 0;

    R[4] = (uint64_t)(uintptr_t)stk + 0x10000 + 8;
    fle_st(R[4], 8, 0xdead0000);
    fle_teb_set_stack(A(stk), A(stk) + sizeof stk);
    memset(wc, 0, sizeof wc);
    memcpy(wc, &style, 4); memcpy(wc + 8, &proc, 8); memcpy(wc + 24, &inst, 8); memcpy(wc + 40, &cur, 8); memcpy(wc + 64, &nmp, 8);
    (void)fle_api2_call(fle_imp_RegisterClassW, A(wc), 0, 0, 0, 0, 0, 0, 0);
}

static HWND mk(uint32_t exstyle, uint32_t style, int x, int y, int cx, int cy)
{
    uint64_t h, sp0 = R[4];
    uint16_t empty[1] = { 0 };

    R[4] = (R[4] - 0x80) & ~UINT64_C(15);
    R[4] -= 8;
    fle_st(R[4], 8, 0xAAAA0000);
    fle_st(R[4] + 0x28, 8, (uint32_t)x); fle_st(R[4] + 0x30, 8, (uint32_t)y); fle_st(R[4] + 0x38, 8, (uint32_t)cx); fle_st(R[4] + 0x40, 8, (uint32_t)cy);
    fle_st(R[4] + 0x48, 8, 0); fle_st(R[4] + 0x50, 8, 0); fle_st(R[4] + 0x58, 8, 0x400000); fle_st(R[4] + 0x60, 8, 0);
    R[1] = exstyle; R[2] = A(clsname); R[8] = A(empty); R[9] = style;
    fle_imp_CreateWindowExW();
    h = R[0];
    R[4] = sp0;
    return h;
}

static void pump(void)
{
    uint8_t msg[FLE_MSG_SIZE];

    while (fle_api2_call(fle_imp_PeekMessageW, A(msg), 0, 0, 0, 1, 0, 0, 0) & 1) (void)fle_api2_call(fle_imp_DispatchMessageW, A(msg), 0, 0, 0, 0, 0, 0, 0);
}

static uint64_t bitblt(uint64_t hdc, int x, int y, int w, int h, uint64_t src, int sx, int sy, uint32_t rop)
{
    uint64_t r, sp0 = R[4];

    R[4] = (R[4] - 0x80) & ~UINT64_C(15);
    R[4] -= 8;
    fle_st(R[4], 8, 0xAAAA0001);
    fle_st(R[4] + 0x28, 8, (uint32_t)h); fle_st(R[4] + 0x30, 8, src); fle_st(R[4] + 0x38, 8, (uint32_t)sx); fle_st(R[4] + 0x40, 8, (uint32_t)sy); fle_st(R[4] + 0x48, 8, rop);
    R[1] = hdc; R[2] = (uint32_t)x; R[8] = (uint32_t)y; R[9] = (uint32_t)w;
    fle_imp_BitBlt();
    r = R[0];
    R[4] = sp0;
    return r;
}

/* a top-down 32-bit DIB section of w x h in a memory DC; returns the DC, *bits is the pixel memory (B, G, R, A bytes) */
static uint64_t mem_dib(int w, int h, uint8_t **bits)
{
    uint32_t bmi[11];
    uint64_t dc, bmp, pv = 0;

    memset(bmi, 0, sizeof bmi);
    bmi[0] = 40; bmi[1] = (uint32_t)w; bmi[2] = (uint32_t)-h; bmi[3] = 1u | (32u << 16);
    dc = fle_api2_call(fle_imp_CreateCompatibleDC, 0, 0, 0, 0, 0, 0, 0, 0);
    bmp = fle_api2_call(fle_imp_CreateDIBSection, dc, A(bmi), 0, A(&pv), 0, 0, 0, 0);
    (void)fle_api2_call(fle_imp_SelectObject, dc, bmp, 0, 0, 0, 0, 0, 0);
    *bits = (uint8_t *)(uintptr_t)pv;
    return dc;
}

/* draws a w x h DIB (rows of the same 16 pixels from `row`, columns beyond 16 repeat; `a_keep`: write the A byte as given, else 255) to the window with a SRCCOPY BitBlt */
static void paint(HWND win, int w, int h, const uint8_t (*row)[4], int nrow_pix)
{
    uint8_t *bits;
    uint64_t mdc = mem_dib(w, h, &bits), wdc = fle_api2_call(fle_imp_GetDC, win, 0, 0, 0, 0, 0, 0, 0);

    for (int y = 0; y < h; y++)
        for (int x = 0; x < w; x++) {
            const uint8_t *p = row[x % nrow_pix];
            uint8_t *q = bits + 4 * ((size_t)y * w + x);
            q[0] = p[3]; q[1] = p[2]; q[2] = p[1]; q[3] = p[0];
        }
    if (!bitblt(wdc, 0, 0, w, h, mdc, 0, 0, 0x00CC0020u)) printf("FAIL: BitBlt returned 0\n");
    (void)fle_api2_call(fle_imp_ReleaseDC, win, wdc, 0, 0, 0, 0, 0, 0);
}


/* one pixel (A, R, G, B as PIX gives them) written to (x, y) of the window by a 1 x 1 BitBlt */
static void paint_px(HWND win, int x, int y, const uint8_t p[4])
{
    uint8_t *bits;
    uint64_t mdc = mem_dib(1, 1, &bits), wdc = fle_api2_call(fle_imp_GetDC, win, 0, 0, 0, 0, 0, 0, 0);

    bits[0] = p[3]; bits[1] = p[2]; bits[2] = p[1]; bits[3] = p[0];
    if (!bitblt(wdc, x, y, 1, 1, mdc, 0, 0, 0x00CC0020u)) printf("FAIL: BitBlt returned 0\n");
    (void)fle_api2_call(fle_imp_ReleaseDC, win, wdc, 0, 0, 0, 0, 0, 0);
}

static void extend(HWND win, int32_t l, int32_t r, int32_t t, int32_t b)
{
    int32_t m[4] = { l, r, t, b };

    R[1] = win; R[2] = A(m);
    fle_imp_DwmExtendFrameIntoClientArea();
    if (R[0] != 0) printf("FAIL: DwmExtendFrameIntoClientArea returned %#llx\n", (unsigned long long)R[0]);
}

static int fails, checks;
static void pixel(const char *what, int x, int y, const uint8_t *want)
{
    const uint8_t *pic = fle_desk_picture();
    const uint8_t *q = pic + 4 * ((size_t)y * 1920 + x);

    checks++;
    if (q[0] != want[0] || q[1] != want[1] || q[2] != want[2] || q[3] != 255) {
        fails++;
        printf("FAIL %s (%d,%d): got %u,%u,%u,%u want %u,%u,%u,255\n", what, x, y, q[0], q[1], q[2], q[3], want[0], want[1], want[2]);
    } else printf("ok %s (%d,%d): %u,%u,%u\n", what, x, y, q[0], q[1], q[2]);
}
static void yes(const char *what, int cond)
{
    checks++;
    if (!cond) { fails++; printf("FAIL %s\n", what); } else printf("ok %s\n", what);
}


static const uint8_t PIXA[1][4] = { { 255, 10, 20, 30 } };

/* ---- the wrap: every call of fle_desk_wrote (the compositor's damage) is counted with its rectangle ---- */
static int wr_calls;
static int64_t wr_x0, wr_y0, wr_x1, wr_y1;
void __real_fle_desk_wrote(uint32_t lid, int64_t x0, int64_t y0, int64_t x1, int64_t y1);
void __wrap_fle_desk_wrote(uint32_t lid, int64_t x0, int64_t y0, int64_t x1, int64_t y1)
{
    wr_calls++; wr_x0 = x0; wr_y0 = y0; wr_x1 = x1; wr_y1 = y1;
    __real_fle_desk_wrote(lid, x0, y0, x1, y1);
}

/* ---- part 1: fle_r_blit and fle_r_changed on surfaces ---- */
#define W 64
#define H 12
static uint32_t dbuf[W * H], sbuf[W * H];
static fle_rsurf mk_surf(uint32_t *bits, int keep_x)
{
    fle_rsurf s;

    memset(&s, 0, sizeof s);
    s.bits = (uint8_t *)bits; s.width = W; s.height = H; s.stride = W * 4; s.bpp = 32; s.comp = 0; s.keep_x = keep_x;
    return s;
}
static void fill(uint32_t seed)
{
    for (int i = 0; i < W * H; i++) { sbuf[i] = (uint32_t)(i * 2654435761u + seed) & 0x00FFFFFFu; dbuf[i] = sbuf[i]; }
}
/* one run: begin, a copy of the whole area, end; returns what fle_r_changed said, r narrowed */
static int run_copy(const fle_rsurf *d, const fle_rsurf *s, uint32_t idx, fle_rrect *r)
{
    fle_rrect all = { 0, 0, W, H };

    fle_r_changed_begin();
    (void)fle_r_blit(d, NULL, 0, 0, W, H, s, 0, 0, idx, 0);
    *r = all;
    return fle_r_changed(r);
}
static int bytes_equal_src(int keep_x)
{
    for (int i = 0; i < W * H; i++) {
        uint32_t want = keep_x ? sbuf[i] : (sbuf[i] & 0x00FFFFFFu);

        if (dbuf[i] != want) return 0;
    }
    return 1;
}

static void part1(void)
{
    for (int keep = 0; keep <= 1; keep++) {
        fle_rsurf d = mk_surf(dbuf, keep), s = mk_surf(sbuf, 1);
        fle_rrect r;
        char nm[160];
        const char *kind = keep ? "keep_x (the fourth byte is copied)" : "window-like (the fourth byte is 0)";

        printf("-- part 1, destination %s\n", kind);
        fill(7);
        yes(keep ? "identical: 0" : "identical: 0", run_copy(&d, &s, FLE_R_SRCCOPY_IDX, &r) == 0);
        /* the first pixel, the last pixel of the first row, the first pixel of the last row, the last pixel, a pixel in the middle */
        static const int px[5][2] = { { 0, 0 }, { W - 1, 0 }, { 0, H - 1 }, { W - 1, H - 1 }, { 31, 5 } };
        for (int k = 0; k < 5; k++) {
            fill(7);
            dbuf[px[k][1] * W + px[k][0]] ^= 0x00000001u;          /* the lowest byte of one destination pixel differs */
            snprintf(nm, sizeof nm, "%s: destination pixel (%d,%d) differs in the first byte: 1, rows [%d, %d)", kind, px[k][0], px[k][1], px[k][1], H);
            yes(nm, run_copy(&d, &s, FLE_R_SRCCOPY_IDX, &r) == 1 && r.y0 == px[k][1] && r.y1 == H);
            yes("  after the copy the destination holds the source bytes", bytes_equal_src(keep));
            fill(7);
            sbuf[px[k][1] * W + px[k][0]] ^= 0x00000100u;          /* or the source differs (the second byte) */
            snprintf(nm, sizeof nm, "%s: source pixel (%d,%d) differs in the second byte: 1, rows [%d, %d)", kind, px[k][0], px[k][1], px[k][1], H);
            yes(nm, run_copy(&d, &s, FLE_R_SRCCOPY_IDX, &r) == 1 && r.y0 == px[k][1] && r.y1 == H);
            yes("  after the copy the destination holds the source bytes", bytes_equal_src(keep));
        }
        fill(7);
        dbuf[3 * W + 1] ^= 0x00010000u; dbuf[5 * W + 60] ^= 0x00800000u;
        yes("two rows differ (3 and 5): 1, rows from the first one to the end of the call [3, H)", run_copy(&d, &s, FLE_R_SRCCOPY_IDX, &r) == 1 && r.y0 == 3 && r.y1 == H);
        yes("  and the rows after the first difference are copied too (the destination holds the source bytes)", bytes_equal_src(keep));
        /* the fourth byte: a source whose fourth byte is set */
        fill(7);
        sbuf[4 * W + 9] |= 0xFF000000u;
        if (keep) {
            yes("keep_x: the source fourth byte differs from the destination's: 1, rows [4, H)", run_copy(&d, &s, FLE_R_SRCCOPY_IDX, &r) == 1 && r.y0 == 4 && r.y1 == H);
            yes("  the destination holds the fourth byte", dbuf[4 * W + 9] >> 24 == 0xFF);
        } else {
            yes("window-like: the source's fourth byte is not copied, so nothing differs: 0", run_copy(&d, &s, FLE_R_SRCCOPY_IDX, &r) == 0);
            yes("  the destination's fourth byte stays 0", dbuf[4 * W + 9] >> 24 == 0);
            fill(7);
            dbuf[2 * W + 2] |= 0x7F000000u;                         /* garbage in the destination's fourth byte: the copy zeroes it, so it is a change */
            yes("window-like: a fourth byte in the destination that the copy zeroes: 1, rows [2, H)", run_copy(&d, &s, FLE_R_SRCCOPY_IDX, &r) == 1 && r.y0 == 2 && r.y1 == H);
            yes("  and it is zero afterwards", dbuf[2 * W + 2] >> 24 == 0);
        }
        /* a run of two copies (the clip pieces): the first identical, the second with a difference */
        fill(7);
        dbuf[10 * W + 40] ^= 4u;
        {
            fle_rrect all = { 0, 0, W, H };

            fle_r_changed_begin();
            (void)fle_r_blit(&d, NULL, 0, 0, 32, H, &s, 0, 0, FLE_R_SRCCOPY_IDX, 0);       /* the left half: identical */
            (void)fle_r_blit(&d, NULL, 32, 0, 32, H, &s, 32, 0, FLE_R_SRCCOPY_IDX, 0);     /* the right half: row 10 differs */
            r = all;
            yes("two copies in one run, the second changes row 10: 1, rows [10, H) (the first copy found nothing: its rows are not in the answer unless the second call is whole later: here it is the second that found the difference)", fle_r_changed(&r) == 1 && r.y0 == 10 && r.y1 == H);
        }
        /* alpha2b: the first call finds a difference at row 5 of its rows [0, 10); a later identical call over rows [6, 8) is counted whole (it is copied without a comparison), a later call over rows [2, 4) lowers the start (the picture here is 64 x 12) */
        fill(7);
        dbuf[5 * W + 3] ^= 8u;
        {
            fle_rrect all = { 0, 0, W, H };

            fle_r_changed_begin();
            (void)fle_r_blit(&d, NULL, 0, 0, 32, 10, &s, 0, 0, FLE_R_SRCCOPY_IDX, 0);
            (void)fle_r_blit(&d, NULL, 40, 6, 8, 2, &s, 40, 6, FLE_R_SRCCOPY_IDX, 0);
            r = all;
            yes("a difference at row 5 of a call over [0, 10), then an identical call over [6, 8): 1, rows [5, 10)", fle_r_changed(&r) == 1 && r.y0 == 5 && r.y1 == 10);
        }
        fill(7);
        dbuf[7 * W + 3] ^= 8u;
        {
            fle_rrect all = { 0, 0, W, H };

            fle_r_changed_begin();
            (void)fle_r_blit(&d, NULL, 0, 0, 32, 10, &s, 0, 0, FLE_R_SRCCOPY_IDX, 0);
            (void)fle_r_blit(&d, NULL, 40, 2, 8, 2, &s, 40, 2, FLE_R_SRCCOPY_IDX, 0);
            r = all;
            yes("a difference at row 7 of a call over [0, 10), then an identical call over [2, 4): 1, rows [2, 10)", fle_r_changed(&r) == 1 && r.y0 == 2 && r.y1 == 10);
            yes("  and every row is copied (the destination holds the source bytes)", bytes_equal_src(keep));
        }
        /* other operations make the answer 'unknown' (1, r as it was) */
        fill(7);
        yes("a ternary operation other than SRCCOPY: unknown (1) although nothing differs", run_copy(&d, &s, 0x00EE0086u & 0xFF, &r) == 1 && r.y0 == 0 && r.y1 == H);
        fill(7);
        {
            fle_rsurf same = mk_surf(dbuf, keep);
            fle_rrect all = { 0, 0, W, H };

            fle_r_changed_begin();
            (void)fle_r_blit(&d, NULL, 0, 0, 16, 8, &same, 16, 2, FLE_R_SRCCOPY_IDX, 0);   /* inside one surface */
            r = all;
            yes("a copy inside one surface: unknown (1)", fle_r_changed(&r) == 1 && r.y0 == 0 && r.y1 == H);
        }
        /* without fle_r_changed_begin nothing is compared and the result is the old one */
        fill(7);
        (void)fle_r_blit(&d, NULL, 0, 0, W, H, &s, 0, 0, FLE_R_SRCCOPY_IDX, 0);
        yes("without begin the copy writes as before", bytes_equal_src(keep));
        fle_r_changed_begin(); { fle_rrect q = { 0, 0, W, H }; (void)fle_r_changed(&q); }       /* leave the state clean */
    }
}

/* ---- part 2: the calls of fle_desk_wrote that real GDI calls make ---- */
/* a w x h picture from argb (A, R, G, B per pixel, row by row) drawn to the window with one SRCCOPY BitBlt */
static void paint_img(HWND win, int w, int h, const uint8_t *argb)
{
    uint8_t *bits;
    uint64_t mdc = mem_dib(w, h, &bits), wdc = fle_api2_call(fle_imp_GetDC, win, 0, 0, 0, 0, 0, 0, 0);

    for (int i = 0; i < w * h; i++) { bits[4 * i] = argb[4 * i + 3]; bits[4 * i + 1] = argb[4 * i + 2]; bits[4 * i + 2] = argb[4 * i + 1]; bits[4 * i + 3] = argb[4 * i]; }
    if (!bitblt(wdc, 0, 0, w, h, mdc, 0, 0, 0x00CC0020u)) printf("FAIL: BitBlt returned 0\n");
    (void)fle_api2_call(fle_imp_ReleaseDC, win, wdc, 0, 0, 0, 0, 0, 0);
}

static void part2(void)
{
    HWND win;
    uint8_t row[16][4];

    printf("-- part 2, real windows\n");
    regclass();
    win = mk(0x10000, WS_POPUP | WS_VISIBLE, 100, 100, 16, 8);
    pump();
    for (int i = 0; i < 16; i++) { row[i][0] = 255; row[i][1] = (uint8_t)(10 + i); row[i][2] = (uint8_t)(20 + 2 * i); row[i][3] = (uint8_t)(30 + 3 * i); }
    paint(win, 16, 8, (const uint8_t (*)[4])row, 16);
    (void)fle_desk_compose();
    wr_calls = 0;
    paint(win, 16, 8, (const uint8_t (*)[4])row, 16);
    yes("BitBlt of the same picture again: fle_desk_wrote is not called", wr_calls == 0);
    row[7][3] ^= 1;                                   /* one pixel of the picture changes */
    paint(win, 16, 8, (const uint8_t (*)[4])row, 16);
    yes("one pixel changed: fle_desk_wrote is called once", wr_calls == 1);
    yes("  with the rows of that pixel (the picture is 8 rows of the same 16 pixels: every row changed)", wr_y0 == 0 && wr_y1 == 8 && wr_x0 == 0 && wr_x1 == 16);
    {
        static uint8_t img[16 * 8 * 4];

        for (int i = 0; i < 16 * 8; i++) { img[4 * i] = 255; img[4 * i + 1] = (uint8_t)i; img[4 * i + 2] = (uint8_t)(i * 3); img[4 * i + 3] = (uint8_t)(i * 5); }
        paint_img(win, 16, 8, img);
        (void)fle_desk_compose();
        wr_calls = 0;
        paint_img(win, 16, 8, img);
        yes("a 16x8 picture drawn again: not called", wr_calls == 0);
        img[4 * (3 * 16 + 9) + 2] ^= 0x40;          /* one pixel in row 3 */
        paint_img(win, 16, 8, img);
        yes("a 16x8 picture with one changed pixel in row 3: called once, for the rows [3, 8) of the picture (from the first row that differs to the end of the call)", wr_calls == 1 && wr_y0 == 3 && wr_y1 == 8 && wr_x0 == 0 && wr_x1 == 16);
        img[4 * (7 * 16 + 15) + 3] ^= 0x01;         /* the last pixel of the last row */
        img[4 * (0 * 16 + 0) + 1] ^= 0x01;          /* the first pixel of the first row */
        wr_calls = 0;
        paint_img(win, 16, 8, img);
        yes("first and last pixel changed: called once, for all 8 rows", wr_calls == 1 && wr_y0 == 0 && wr_y1 == 8);
        (void)fle_desk_compose();
    }
    wr_calls = 0;
    paint_px(win, 5, 3, (const uint8_t[4]){ 255, 99, 98, 97 });
    yes("a 1x1 BitBlt of a new colour: called once", wr_calls == 1);
    yes("  for that pixel", wr_x0 == 5 && wr_x1 == 6 && wr_y0 == 3 && wr_y1 == 4);
    wr_calls = 0;
    paint_px(win, 5, 3, (const uint8_t[4]){ 255, 99, 98, 97 });
    yes("the same 1x1 again: not called", wr_calls == 0);
    paint_px(win, 5, 3, (const uint8_t[4]){ 0, 99, 98, 97 });
    yes("a source with only the fourth byte different (an ordinary window drops that byte): not called", wr_calls == 0);
    /* a window whose frame is extended keeps the fourth byte: it is a change there */
    {
        HWND g = mk(0x10000, WS_POPUP | WS_VISIBLE, 200, 100, 16, 8);

        pump();
        extend(g, -1, -1, -1, -1);
        paint(g, 16, 8, (const uint8_t (*)[4])row, 16);
        (void)fle_desk_compose();
        wr_calls = 0;
        paint_px(g, 5, 3, (const uint8_t[4]){ 255, 99, 98, 97 });
        yes("extended frame: new colour: called once", wr_calls == 1);
        wr_calls = 0;
        paint_px(g, 5, 3, (const uint8_t[4]){ 255, 99, 98, 97 });
        yes("extended frame: the same again: not called", wr_calls == 0);
        paint_px(g, 5, 3, (const uint8_t[4]){ 128, 99, 98, 97 });
        yes("extended frame: only the fourth byte different: called (it is the alpha)", wr_calls == 1);
        (void)fle_desk_compose();
        {
            const uint8_t *q = fle_desk_picture() + 4 * ((size_t)103 * 1920 + 205);

            yes("  and the picture follows (premultiplied 99,98,97 with A 128 over the background 5,7,10: 99+0.498*5=101.5)", q[0] >= 100 && q[0] <= 102);
        }
    }
}

int main(void)
{
    setvbuf(stdout, NULL, _IONBF, 0);
    part1();
    part2();
    printf("checks %d failed %d\n", checks, fails);
    return fails ? 1 : 0;
}

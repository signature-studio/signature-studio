/* Decision 0244 (alpha1, test tool; scaffold of harness/txt1/create_seq.c): windows whose frame is extended into the client area (DwmExtendFrameIntoClientArea), the alpha byte that a BitBlt from a 32-bit DIB leaves in their surface,
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

/* @@ tables from tools/mk-expect.py @@ */
static const uint8_t PIX[16][4] = {   /* A, R, G, B (premultiplied, as BitBlt'd from the DIB) */
    {255, 200, 100, 50}, {0, 0, 0, 0}, {128, 128, 64, 32}, {255, 0, 255, 0}, {200, 100, 100, 100}, {0, 7, 8, 9}, {1, 0, 0, 0}, {254, 0, 0, 0}, {64, 64, 0, 0}, {64, 0, 0, 64}, {100, 50, 50, 50}, {255, 1, 2, 3}, {128, 128, 128, 128}, {128, 0, 0, 0}, {2, 1, 1, 1}, {253, 250, 250, 250}
};
static const uint8_t OVER_RED[16][3] = {   /* Dst = Src + (1 - A/255) * Dst, Dst = (255, 0, 0) */
    {200, 100, 50}, {255, 0, 0}, {255, 64, 32}, {0, 255, 0}, {155, 100, 100}, {255, 8, 9}, {254, 0, 0}, {1, 0, 0}, {255, 0, 0}, {191, 0, 64}, {205, 50, 50}, {1, 2, 3}, {255, 128, 128}, {127, 0, 0}, {254, 1, 1}, {252, 250, 250}
};
static const uint8_t OVER_BG[16][3] = {   /* Dst = Src + (1 - A/255) * Dst, Dst = (5, 7, 10) */
    {200, 100, 50}, {5, 7, 10}, {130, 67, 37}, {0, 255, 0}, {101, 102, 102}, {12, 15, 19}, {5, 7, 10}, {0, 0, 0}, {68, 5, 7}, {4, 5, 71}, {53, 54, 56}, {1, 2, 3}, {130, 131, 133}, {2, 3, 5}, {6, 8, 11}, {250, 250, 250}
};
static const uint8_t ULW_PM_128[16][3] = {   /* ULW_ALPHA, AC_SRC_ALPHA, SourceConstantAlpha 128: Src*128/255 in all four channels, then Src + (1 - Src.A) * Dst; Dst = (5, 7, 10) */
    {103, 54, 30}, {5, 7, 10}, {68, 37, 24}, {2, 131, 5}, {53, 54, 56}, {9, 11, 15}, {5, 7, 10}, {3, 4, 5}, {36, 6, 9}, {4, 6, 41}, {29, 31, 33}, {3, 4, 6}, {68, 69, 72}, {4, 5, 7}, {5, 7, 10}, {128, 129, 131}
};
static const uint8_t ULW_CONST_100[16][3] = {   /* ULW_ALPHA, no AC_SRC_ALPHA (the A byte of the DIB is not used), SourceConstantAlpha 100: Dst = Src * 100/255 + Dst * (1 - 100/255) */
    {81, 43, 26}, {3, 4, 6}, {53, 29, 19}, {3, 104, 6}, {42, 43, 45}, {6, 7, 10}, {3, 4, 6}, {3, 4, 6}, {28, 4, 6}, {3, 4, 31}, {23, 24, 26}, {3, 5, 7}, {53, 54, 56}, {3, 4, 6}, {3, 5, 6}, {101, 102, 104}
};
static const uint8_t ULW_OPAQUE[16][3] = {   /* ULW_OPAQUE: Dst = Src */
    {200, 100, 50}, {0, 0, 0}, {128, 64, 32}, {0, 255, 0}, {100, 100, 100}, {7, 8, 9}, {0, 0, 0}, {0, 0, 0}, {64, 0, 0}, {0, 0, 64}, {50, 50, 50}, {1, 2, 3}, {128, 128, 128}, {0, 0, 0}, {1, 1, 1}, {250, 250, 250}
};
static const uint8_t ULW_KEY[16][3] = {   /* ULW_COLORKEY (0, 0, 0): the key colour is transparent, the rest opaque */
    {200, 100, 50}, {5, 7, 10}, {128, 64, 32}, {0, 255, 0}, {100, 100, 100}, {7, 8, 9}, {5, 7, 10}, {5, 7, 10}, {64, 0, 0}, {0, 0, 64}, {50, 50, 50}, {1, 2, 3}, {128, 128, 128}, {5, 7, 10}, {1, 1, 1}, {250, 250, 250}
};
static const uint8_t SLWA_A200[16][3] = {   /* LWA_ALPHA 200: Dst = Src * 200/255 + Dst * (1 - 200/255) */
    {158, 80, 41}, {1, 2, 2}, {101, 52, 27}, {1, 202, 2}, {80, 80, 81}, {7, 8, 9}, {1, 2, 2}, {1, 2, 2}, {51, 2, 2}, {1, 2, 52}, {40, 41, 41}, {2, 3, 5}, {101, 102, 103}, {1, 2, 2}, {2, 2, 3}, {197, 198, 198}
};
static const uint8_t SLWA_KEY[16][3] = {   /* LWA_COLORKEY (0, 0, 0) */
    {200, 100, 50}, {5, 7, 10}, {128, 64, 32}, {0, 255, 0}, {100, 100, 100}, {7, 8, 9}, {5, 7, 10}, {5, 7, 10}, {64, 0, 0}, {0, 0, 64}, {50, 50, 50}, {1, 2, 3}, {128, 128, 128}, {5, 7, 10}, {1, 1, 1}, {250, 250, 250}
};
static const uint8_t SLWA_BOTH[16][3] = {   /* LWA_COLORKEY (0, 0, 0) | LWA_ALPHA 200 */
    {158, 80, 41}, {5, 7, 10}, {101, 52, 27}, {1, 202, 2}, {80, 80, 81}, {7, 8, 9}, {5, 7, 10}, {5, 7, 10}, {51, 2, 2}, {1, 2, 52}, {40, 41, 41}, {2, 3, 5}, {101, 102, 103}, {5, 7, 10}, {2, 2, 3}, {197, 198, 198}
};
static const uint8_t TWO_LAYERS[16][3] = {   /* above: ULW AC_SRC_ALPHA pixel 15 - i, SCA 255; below: LWA_ALPHA 200 pixel i; background (5, 7, 10) */
    {251, 251, 250}, {2, 3, 3}, {50, 26, 13}, {128, 229, 129}, {1, 2, 3}, {54, 55, 55}, {1, 1, 65}, {65, 1, 1}, {0, 0, 0}, {1, 2, 52}, {47, 49, 50}, {100, 101, 101}, {0, 255, 0}, {128, 65, 33}, {2, 2, 3}, {200, 100, 50}
};
static const uint8_t SLWA_A77_LIGHT[16][3] = {   /* LWA_ALPHA 77 over a window of (240, 200, 120): Dst = Src * 77/255 + Dst * (1 - 77/255) */
    {228, 170, 99}, {168, 140, 84}, {206, 159, 93}, {168, 217, 84}, {198, 170, 114}, {170, 142, 86}, {168, 140, 84}, {168, 140, 84}, {187, 140, 84}, {168, 140, 103}, {183, 155, 99}, {168, 140, 85}, {206, 178, 122}, {168, 140, 84}, {168, 140, 84}, {243, 215, 159}
};
static const uint8_t ULW_PM_77_LIGHT[16][3] = {   /* ULW_ALPHA, AC_SRC_ALPHA, SourceConstantAlpha 77 over (240, 200, 120) */
    {228, 170, 99}, {240, 200, 120}, {242, 189, 111}, {168, 217, 84}, {213, 183, 122}, {242, 202, 123}, {240, 200, 120}, {168, 140, 84}, {241, 185, 111}, {222, 185, 130}, {227, 191, 121}, {168, 140, 85}, {242, 208, 140}, {204, 170, 102}, {240, 200, 120}, {244, 216, 160}
};
static const uint8_t RED_PIX[1][4] = { { 255, 255, 0, 0 } };
static const uint8_t BG[3] = { 5, 7, 10 };

/* UpdateLayeredWindow(hWnd, hdcDst NULL, pptDst NULL, psize NULL, hdcSrc, pptSrc (0,0), crKey, pblend, dwFlags): nine arguments */
static uint64_t ulw(HWND win, uint64_t hdc_src, uint32_t key, uint8_t sca, uint8_t afmt, uint32_t flags)
{
    uint64_t r, sp0 = R[4];
    int32_t src_pt[2] = { 0, 0 };
    uint8_t blend[4] = { 0, 0, sca, afmt };

    R[4] = (R[4] - 0x80) & ~UINT64_C(15);
    R[4] -= 8;
    fle_st(R[4], 8, 0xAAAA0002);
    fle_st(R[4] + 0x28, 8, hdc_src); fle_st(R[4] + 0x30, 8, A(src_pt)); fle_st(R[4] + 0x38, 8, key); fle_st(R[4] + 0x40, 8, A(blend)); fle_st(R[4] + 0x48, 8, flags);
    R[1] = win; R[2] = 0; R[8] = 0; R[9] = 0;
    fle_imp_UpdateLayeredWindow();
    r = R[0];
    R[4] = sp0;
    return r;
}
static uint64_t slwa(HWND win, uint32_t key, uint8_t alpha, uint32_t flags) { return fle_api2_call(fle_imp_SetLayeredWindowAttributes, win, key, alpha, flags, 0, 0, 0, 0); }
/* a 16 x 1 layered window at (x, y) */
static HWND layered(int x, int y) { return mk(0x10000u | 0x80000u, WS_POPUP | WS_VISIBLE, x, y, 16, 1); }
/* the DIB of the 16 pixels of PIX (B, G, R, A as PIX gives them) */
static uint64_t pix_dib(void)
{
    uint8_t *bits;
    uint64_t dc = mem_dib(16, 1, &bits);

    for (int x = 0; x < 16; x++) { bits[4 * x] = PIX[x][3]; bits[4 * x + 1] = PIX[x][2]; bits[4 * x + 2] = PIX[x][1]; bits[4 * x + 3] = PIX[x][0]; }
    return dc;
}
static void row16(const char *what, int x0, int y, const uint8_t (*want)[3])
{
    for (int i = 0; i < 16; i++) {
        char nm[80];

        snprintf(nm, sizeof nm, "%s px %d", what, i);
        pixel(nm, x0 + i, y, want[i]);
    }
}


int main(void)
{
    HWND under, glass, part, plain, g2;
    int32_t m[4];
    const fle_rsurf *sp_;

    setvbuf(stdout, NULL, _IONBF, 0);
    regclass();

    /* U: an ordinary window (100,100) 16x8, solid red, under G */
    under = mk(0x10000, WS_POPUP | WS_VISIBLE, 100, 100, 16, 8);
    pump();
    paint(under, 16, 8, RED_PIX, 1);
    /* G: (104,100) 16x4, the sheet of glass (all margins -1), the 16 pixels of PIX; its first 12 columns are over U, the last 4 over the background */
    glass = mk(0x10000, WS_POPUP | WS_VISIBLE, 104, 100, 16, 4);
    pump();
    extend(glass, -1, -1, -1, -1);
    paint(glass, 16, 4, PIX, 16);
    pump();
    (void)fle_desk_compose();
    printf("== 1. the sheet of glass: Dst = Src + (1 - Src.A) * Dst, premultiplied\n");
    for (int i = 0; i < 16; i++) {
        int sx = 104 + i;
        char nm[40];

        snprintf(nm, sizeof nm, "glass px %d over %s", i, sx < 116 ? "red" : "background");
        pixel(nm, sx, 101, sx < 116 ? OVER_RED[i] : OVER_BG[i]);
    }
    pixel("ordinary window U (x 100..103) is opaque red", 101, 101, RED_PIX[0] + 1);
    pixel("outside every window: the background", 50, 50, BG);

    printf("== 2. the alpha byte: kept in the surface of the extended window, not in the surface of another\n");
    {
        fle_win_dcq q;
        const fle_rsurf *s;
        int ok = 1;

        yes("glass window has a drawing record", fle_win_dc_query(glass, &q));
        s = fle_raster_surface_peek(q.root_lid);
        yes("glass surface exists", s != NULL);
        if (s) for (int i = 0; i < 16; i++) if (s->bits[4 * i + 3] != PIX[i][0]) { ok = 0; printf("  glass byte 3 of px %d is %u want %u\n", i, s->bits[4 * i + 3], PIX[i][0]); }
        yes("glass surface keeps A of every pixel", ok);
        yes("ordinary window has a drawing record", fle_win_dc_query(under, &q));
        s = fle_raster_surface_peek(q.root_lid);
        yes("ordinary surface exists", s != NULL);
        if (s) { int z = 1; for (int i = 0; i < 16 * 8; i++) if (s->bits[4 * i + 3] != 0) z = 0; yes("ordinary surface: the fourth byte is 0 although the DIB's A is 255", z); }
    }

    printf("== 3. the margins: only the extended strip uses the alpha\n");
    part = mk(0x10000, WS_POPUP | WS_VISIBLE, 200, 100, 16, 4);
    pump();
    extend(part, 4, 0, 0, 0);
    paint(part, 16, 4, PIX, 16);
    pump();
    (void)fle_desk_compose();
    for (int i = 0; i < 16; i++) {
        char nm[48];

        snprintf(nm, sizeof nm, "left margin 4, px %d (%s)", i, i < 4 ? "glass" : "inner");
        if (i < 4) pixel(nm, 200 + i, 101, OVER_BG[i]);
        else pixel(nm, 200 + i, 101, PIX[i] + 1);          /* the inner client area is opaque: the stored colour as it is */
    }
    R[1] = part; R[2] = A(m);
    m[0] = m[1] = m[2] = m[3] = 0;
    fle_imp_DwmExtendFrameIntoClientArea();
    (void)fle_desk_compose();
    pixel("margins 0,0,0,0 take the extension away: px 2 is opaque", 202, 101, PIX[2] + 1);

    {
        HWND t = mk(0x10000, WS_POPUP | WS_VISIBLE, 400, 100, 16, 4);

        pump();
        extend(t, 0, 4, 1, 1);
        paint(t, 16, 4, PIX, 16);
        pump();
        (void)fle_desk_compose();
        printf("-- margins right 4, top 1, bottom 1: glass at x >= 12 and at y 0 and y 3, opaque inside\n");
        pixel("top strip (x 4 y 0), px 4", 404, 100, OVER_BG[4]);
        pixel("inner (x 4 y 1), px 4", 404, 101, PIX[4] + 1);
        pixel("inner (x 4 y 2), px 4", 404, 102, PIX[4] + 1);
        pixel("bottom strip (x 4 y 3), px 4", 404, 103, OVER_BG[4]);
        pixel("right strip (x 13 y 1), px 13", 413, 101, OVER_BG[13]);
        pixel("inner (x 11 y 1), px 11", 411, 101, PIX[11] + 1);
        pixel("right strip (x 12 y 2), px 12", 412, 102, OVER_BG[12]);
    }

    printf("== 4. a window without the extension is opaque and its surface has no alpha byte\n");
    plain = mk(0x10000, WS_POPUP | WS_VISIBLE, 300, 100, 16, 4);
    pump();
    paint(plain, 16, 4, PIX, 16);
    pump();
    (void)fle_desk_compose();
    pixel("plain px 2 shows the stored colour", 302, 101, PIX[2] + 1);
    pixel("plain px 1 (A 0, colour 0) is black, not transparent", 301, 101, PIX[1] + 1);
    {
        fle_win_dcq q;

        if (fle_win_dc_query(plain, &q)) {
            sp_ = fle_raster_surface_peek(q.root_lid);
            yes("plain surface byte 3 of px 2 is 0", sp_ && sp_->bits[4 * 2 + 3] == 0);
        }
    }

    printf("== 5. SetWindowRgn: the window shows only the part inside its region\n");
    g2 = mk(0x10000, WS_POPUP | WS_VISIBLE, 104, 200, 16, 4);
    pump();
    extend(g2, -1, -1, -1, -1);
    paint(g2, 16, 4, PIX, 16);
    pump();
    (void)fle_desk_compose();
    pixel("before the region, px 4 over the background", 108, 201, OVER_BG[4]);
    {
        uint64_t rgn = fle_api2_call(fle_imp_CreateRectRgn, 2, 1, 10, 3, 0, 0, 0, 0);

        R[1] = g2; R[2] = rgn; R[8] = 1;
        fle_imp_SetWindowRgn();
        yes("SetWindowRgn succeeds", R[0] != 0);
    }
    (void)fle_desk_compose();
    yes("the description names the glass and the region of the window", strstr(fle_desk_describe(), " glass(0,0,0,0) region(2,1,10,3) alpha 0 key 000000 class ") != NULL);
    pump();
    (void)fle_desk_compose();
    pixel("inside the region (x 4 y 1): px 4", 108, 201, OVER_BG[4]);
    pixel("inside the region (x 9 y 2): px 9", 113, 202, OVER_BG[9]);
    pixel("left of the region (x 0 y 1; px 0 is opaque): the background", 104, 201, BG);
    pixel("left of the region (x 1 y 1): the background", 105, 201, BG);
    pixel("right of the region (x 10 y 1): the background", 114, 201, BG);
    pixel("above the region (x 4 y 0): the background", 108, 200, BG);
    pixel("above the region (x 0 y 0; px 0 is opaque): the background", 104, 200, BG);
    pixel("the region's left edge (x 2 y 1): px 2", 106, 201, OVER_BG[2]);
    pixel("below the region (x 4 y 3): the background", 108, 203, BG);
    {
        uint64_t in_ = fle_api2_call(fle_imp_WindowFromPoint, (uint64_t)(uint32_t)108 | ((uint64_t)(uint32_t)201 << 32), 0, 0, 0, 0, 0, 0, 0);
        uint64_t out_ = fle_api2_call(fle_imp_WindowFromPoint, (uint64_t)(uint32_t)105 | ((uint64_t)(uint32_t)201 << 32), 0, 0, 0, 0, 0, 0, 0);

        yes("WindowFromPoint inside the region finds the window", in_ == g2);
        yes("WindowFromPoint outside the region does not (no window below)", out_ == 0);
        yes("fle_win_input_hit inside the region finds the window", fle_win_input_hit(108, 201) == g2);
        yes("fle_win_input_hit outside the region does not", fle_win_input_hit(105, 201) == 0);
    }

    {
        uint64_t rgn = fle_api2_call(fle_imp_CreateRectRgn, (uint64_t)(uint32_t)-5, (uint64_t)(uint32_t)-5, 100, 100, 0, 0, 0, 0);

        R[1] = g2; R[2] = rgn; R[8] = 1;
        fle_imp_SetWindowRgn();
        pump();
        (void)fle_desk_compose();
        pixel("a region larger than the window shows all of it: (x 0 y 0), px 0", 104, 200, OVER_BG[0]);
        pixel("a region larger than the window shows all of it: (x 15 y 3), px 15", 119, 203, OVER_BG[15]);
        yes("a region larger than the window is clamped to the window (the description names no region)", strstr(fle_desk_describe(), "region") == NULL);
        {
            /* the description of every window ends with its class and its title (the page and the drivers read the line so); the glass and region words are in front of them */
            const char *d = fle_desk_describe(), *e;
            int lines = 0, good = 1;

            for (; *d; d = e + 1) {
                e = strchr(d, '\n');
                if (!e) { good = 0; break; }
                lines++;
                if (e - d < 8 || e[-1] != '"' || !memmem(d, (size_t)(e - d), " class \"", 8) || !memmem(d, (size_t)(e - d), "\" title \"", 9)) good = 0;
            }
            yes("every line of the description ends with class and title", good && lines >= 5);
        }
        rgn = fle_api2_call(fle_imp_CreateRectRgn, 2, 1, 10, 3, 0, 0, 0, 0);
        R[1] = g2; R[2] = rgn; R[8] = 1;
        fle_imp_SetWindowRgn();
        pump();
    }


    printf("== 7. layered windows: UpdateLayeredWindow and SetLayeredWindowAttributes, hand-made tables\n");
    {
        HWND w1 = layered(500, 100), w2 = layered(500, 102), w3 = layered(500, 104), w4 = layered(500, 106), w5 = layered(500, 108), w6 = layered(500, 110), w7 = layered(500, 112), w8 = layered(500, 114), w9 = layered(500, 114);
        uint64_t dc;

        pump();
        dc = pix_dib();
        yes("ULW_ALPHA | AC_SRC_ALPHA, SCA 128", ulw(w1, dc, 0, 128, 1, 2) == 1);
        yes("ULW_ALPHA, SCA 100, no AC_SRC_ALPHA", ulw(w2, dc, 0, 100, 0, 2) == 1);
        yes("ULW_OPAQUE", ulw(w3, dc, 0, 0, 0, 4) == 1);
        yes("ULW_COLORKEY (0,0,0)", ulw(w4, dc, 0, 0, 0, 1) == 1);
        yes("LWA_ALPHA 200", slwa(w5, 0, 200, 2) == 1);
        yes("LWA_COLORKEY (0,0,0), bAlpha 77 is ignored without LWA_ALPHA", slwa(w6, 0, 77, 1) == 1);
        yes("LWA_COLORKEY | LWA_ALPHA", slwa(w7, 0, 200, 3) == 1);
        yes("layered window below (LWA_ALPHA 200)", slwa(w8, 0, 200, 2) == 1);
        pump();
        {
            uint64_t sdc, tdc;
            const HWND sw[3] = { w5, w6, w7 };
            const HWND tw = w8;

            /* a layered window with SetLayeredWindowAttributes keeps its GDI surface: the 16 colours are drawn with BitBlt (the A byte of the DIB is nothing to it) */
            sdc = pix_dib();
            for (int k = 0; k < 3; k++) {
                uint64_t wdc = fle_api2_call(fle_imp_GetDC, sw[k], 0, 0, 0, 0, 0, 0, 0);

                (void)bitblt(wdc, 0, 0, 16, 1, sdc, 0, 0, 0x00CC0020u);
                (void)fle_api2_call(fle_imp_ReleaseDC, sw[k], wdc, 0, 0, 0, 0, 0, 0);
            }
            tdc = fle_api2_call(fle_imp_GetDC, tw, 0, 0, 0, 0, 0, 0, 0);
            (void)bitblt(tdc, 0, 0, 16, 1, sdc, 0, 0, 0x00CC0020u);
            (void)fle_api2_call(fle_imp_ReleaseDC, tw, tdc, 0, 0, 0, 0, 0, 0);
        }
        {
            /* the upper window of the pair: its pixel i is PIX[15 - i] */
            uint8_t *bits;
            uint64_t rdc = mem_dib(16, 1, &bits);

            for (int x = 0; x < 16; x++) { const uint8_t *p = PIX[15 - x]; bits[4 * x] = p[3]; bits[4 * x + 1] = p[2]; bits[4 * x + 2] = p[1]; bits[4 * x + 3] = p[0]; }
            yes("the upper layered window gets its pixels (ULW AC_SRC_ALPHA, SCA 255)", ulw(w9, rdc, 0, 255, 1, 2) == 1);
        }
        {
            static const uint8_t LIGHT_PIX[1][4] = { { 255, 240, 200, 120 } };
            HWND lu = mk(0x10000, WS_POPUP | WS_VISIBLE, 500, 116, 16, 2), la = layered(500, 116), lb = layered(500, 117);
            uint64_t ldc, wdc;

            pump();
            paint(lu, 16, 2, LIGHT_PIX, 1);
            ldc = pix_dib();
            wdc = fle_api2_call(fle_imp_GetDC, la, 0, 0, 0, 0, 0, 0, 0);
            (void)bitblt(wdc, 0, 0, 16, 1, ldc, 0, 0, 0x00CC0020u);
            (void)fle_api2_call(fle_imp_ReleaseDC, la, wdc, 0, 0, 0, 0, 0, 0);
            yes("LWA_ALPHA 77 over a light window", slwa(la, 0, 77, 2) == 1);
            yes("ULW_ALPHA | AC_SRC_ALPHA, SCA 77 over a light window", ulw(lb, ldc, 0, 77, 1, 2) == 1);
        }
        pump();
        (void)fle_desk_compose();
        row16("LWA_ALPHA 77 over a light window", 500, 116, SLWA_A77_LIGHT);
        row16("ULW AC_SRC_ALPHA SCA 77 over a light window", 500, 117, ULW_PM_77_LIGHT);
        row16("ULW AC_SRC_ALPHA SCA 128", 500, 100, ULW_PM_128);
        row16("ULW SCA 100 constant alpha", 500, 102, ULW_CONST_100);
        row16("ULW_OPAQUE", 500, 104, ULW_OPAQUE);
        row16("ULW_COLORKEY", 500, 106, ULW_KEY);
        row16("LWA_ALPHA 200", 500, 108, SLWA_A200);
        row16("LWA_COLORKEY", 500, 110, SLWA_KEY);
        row16("LWA_COLORKEY | LWA_ALPHA", 500, 112, SLWA_BOTH);
        row16("two layered windows, the upper over the lower", 500, 114, TWO_LAYERS);
    }

    printf("== 6. a fade: a region and an alpha that falls from 251 to 0 shows the colour below, never black\n");
    {
        static const int alphas[] = { 251, 221, 191, 161, 131, 101, 71, 41, 31, 21, 11, 1, 0 };
        uint8_t fade[1][4];
        int prev = -1, mono = 1, black = 0;

        for (unsigned k = 0; k < sizeof alphas / sizeof alphas[0]; k++) {
            int a = alphas[k];

            /* the colour of the item in the menu, (186, 194, 199), premultiplied by a / 255 */
            fade[0][0] = (uint8_t)a; fade[0][1] = (uint8_t)((186 * a + 127) / 255); fade[0][2] = (uint8_t)((194 * a + 127) / 255); fade[0][3] = (uint8_t)((199 * a + 127) / 255);
            paint(g2, 16, 4, fade, 1);
            pump();
            (void)fle_desk_compose();
            {
                const uint8_t *q = fle_desk_picture() + 4 * ((size_t)201 * 1920 + 108);
                int v = q[0] + q[1] + q[2];

                if (v == 0) black = 1;
                if (prev >= 0 && v > prev) mono = 0;
                prev = v;
            }
        }
        yes("the fade is monotonic from the item to the background", mono);
        yes("the fade never reaches black (the background is 5,7,10)", !black);
        pixel("the last step is the background", 108, 201, BG);
    }
    (void)m;

    printf("== 8. a write that leaves the surface as it was reports nothing; a write that changes it reports what changed (the fourth byte included)\n");
    {
        const fle_desk_state *st;
        uint32_t gen0;
        static const uint8_t NEW15[4] = { 253, 100, 100, 100 }, A12[4] = { 255, 128, 128, 128 };

        (void)fle_desk_compose();
        st = fle_desk_compose();
        yes("nothing written since the last compose: nothing reported", st->n_rects == 0);
        gen0 = st->gen;
        paint(glass, 16, 4, PIX, 16);                 /* the same bytes again, the whole window */
        pump();
        st = fle_desk_compose();
        yes("the same picture drawn again: no rectangle, the generation does not move", st->n_rects == 0 && st->gen == gen0);
        paint_px(glass, 15, 3, NEW15);                /* the last pixel of the last row */
        st = fle_desk_compose();
        yes("one pixel of the last row and column changed: one rectangle of that pixel", st->n_rects == 1 && st->rects[0].x0 == 119 && st->rects[0].y0 == 103 && st->rects[0].x1 == 120 && st->rects[0].y1 == 104);
        pixel("the changed pixel (x 15 y 3) over the background", 119, 103, (const uint8_t[3]){ 100, 100, 100 });
        pixel("its neighbour above (x 15 y 2) is as before", 119, 102, OVER_BG[15]);
        paint_px(glass, 15, 3, PIX[15]);              /* back to the old bytes: the picture goes back (the shadow followed the change) */
        st = fle_desk_compose();
        pixel("written back: (x 15 y 3) is as it was", 119, 103, OVER_BG[15]);
        yes("written back: one rectangle", st->n_rects == 1);
        paint_px(glass, 12, 0, A12);                  /* only the fourth byte differs (the colour is the same) */
        st = fle_desk_compose();
        yes("only the alpha byte changed: reported", st->n_rects == 1);
        pixel("only the alpha byte changed: the picture follows (opaque 128,128,128)", 116, 100, (const uint8_t[3]){ 128, 128, 128 });
        paint_px(glass, 12, 0, PIX[12]);
        (void)fle_desk_compose();
        pixel("alpha byte written back: the picture follows", 116, 100, OVER_BG[12]);
    }


    printf("== 9. a sweep: a 256 x 256 sheet of glass over three solid windows, every alpha (rows) and every colour value (columns) in the three channels\n");
    {
        static const uint8_t UNDER[3][3] = { { 13, 128, 250 }, { 255, 1, 77 }, { 0, 0, 0 } };
        uint64_t bad = 0, n = 0;

        for (int k = 0; k < 3; k++) {
            uint8_t col[1][4] = { { 255, UNDER[k][0], UNDER[k][1], UNDER[k][2] } };
            HWND u = mk(0x10000, WS_POPUP | WS_VISIBLE, 600 + 300 * k, 300, 256, 256), g = mk(0x10000, WS_POPUP | WS_VISIBLE, 600 + 300 * k, 300, 256, 256);
            uint8_t *bits;
            uint64_t mdc, wdc;

            pump();
            paint(u, 256, 256, col, 1);
            extend(g, -1, -1, -1, -1);
            mdc = mem_dib(256, 256, &bits);
            wdc = fle_api2_call(fle_imp_GetDC, g, 0, 0, 0, 0, 0, 0, 0);
            for (int y = 0; y < 256; y++)
                for (int x = 0; x < 256; x++) {
                    uint8_t *q = bits + 4 * ((size_t)y * 256 + x);

                    q[2] = (uint8_t)x; q[1] = (uint8_t)(255 - x); q[0] = (uint8_t)((x * 7) & 255); q[3] = (uint8_t)y;       /* R = x, G = 255 - x, B = 7x mod 256, A = y (the colour may be above A: the compositor clamps) */
                }
            (void)bitblt(wdc, 0, 0, 256, 256, mdc, 0, 0, 0x00CC0020u);
            (void)fle_api2_call(fle_imp_ReleaseDC, g, wdc, 0, 0, 0, 0, 0, 0);
            pump();
            (void)fle_desk_compose();
            for (int y = 0; y < 256; y++)
                for (int x = 0; x < 256; x++) {
                    const uint8_t *q = fle_desk_picture() + 4 * ((size_t)(300 + y) * 1920 + (size_t)(600 + 300 * k + x));
                    int c[3] = { x, 255 - x, (x * 7) & 255 };

                    n++;
                    for (int ch = 0; ch < 3; ch++) {
                        /* Dst' = Src + (1 - A/255) * Dst, rounded to the nearest (no tie is possible: the denominator 255 is odd), clamped */
                        long t = (long)c[ch] * 255 + (long)(255 - y) * UNDER[k][ch], v = (2 * t + 255) / 510;

                        if (v > 255) v = 255;
                        if (q[ch] != v) { if (bad < 5) printf("  sweep (%d,%d) channel %d under %d: got %u want %ld\n", x, y, ch, UNDER[k][ch], q[ch], v); bad++; }
                    }
                }
        }
        printf("  %llu pixels, %llu channel values differ\n", (unsigned long long)n, (unsigned long long)bad);
        yes("the sweep: all 196,608 pixels equal the formula rounded to the nearest", bad == 0);
    }

    printf("checks %d failed %d\n", checks, fails);
    return fails ? 1 : 0;
}

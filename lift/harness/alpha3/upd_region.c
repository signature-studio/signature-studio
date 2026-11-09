/* Decision 0247 (alpha3, test tool; the scaffold of harness/alpha2/wrote_seq.c): the update region of a window is a set of rectangles (Learn "The Update Region", InvalidateRect, ValidateRect, ValidateRgn, RedrawWindow,
 * GetUpdateRect, GetUpdateRgn, BeginPaint). It runs the runtime itself (fle_win.c, fle_gdi.c, fle_gdc.c, fle_raster.c, fle_desk.c ...) with a window procedure that is DefWindowProcW. The expected values are what the
 * Learn pages say (a rectangle invalidated is added; the bounding box is what GetUpdateRect and PAINTSTRUCT.rcPaint give; GetUpdateRgn is COMPLEXREGION for more than one rectangle; a DC of BeginPaint clips to the
 * region), not read back from the code. Prints one line per check ("ok" / "FAIL"); a failure ends with exit status 1. */
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

static HWND mkp(HWND parent, uint32_t exstyle, uint32_t style, int x, int y, int cx, int cy)
{
    uint64_t h, sp0 = R[4];
    uint16_t empty[1] = { 0 };

    R[4] = (R[4] - 0x80) & ~UINT64_C(15);
    R[4] -= 8;
    fle_st(R[4], 8, 0xAAAA0000);
    fle_st(R[4] + 0x28, 8, (uint32_t)x); fle_st(R[4] + 0x30, 8, (uint32_t)y); fle_st(R[4] + 0x38, 8, (uint32_t)cx); fle_st(R[4] + 0x40, 8, (uint32_t)cy);
    fle_st(R[4] + 0x48, 8, parent); fle_st(R[4] + 0x50, 8, 0); fle_st(R[4] + 0x58, 8, 0x400000); fle_st(R[4] + 0x60, 8, 0);
    R[1] = exstyle; R[2] = A(clsname); R[8] = A(empty); R[9] = style;
    fle_imp_CreateWindowExW();
    h = R[0];
    R[4] = sp0;
    return h;
}

static HWND mk(uint32_t exstyle, uint32_t style, int x, int y, int cx, int cy) { return mkp(0, exstyle, style, x, y, cx, cy); }

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



static void rect4(int32_t *m, int l, int t, int r, int b) { m[0] = l; m[1] = t; m[2] = r; m[3] = b; }
static uint64_t gur(HWND w, int32_t *rc) { return fle_api2_call(fle_imp_GetUpdateRect, w, A(rc), 0, 0, 0, 0, 0, 0); }
static void inval(HWND w, int l, int t, int r, int b)
{
    int32_t m[4];

    rect4(m, l, t, r, b);
    (void)fle_api2_call(fle_imp_InvalidateRect, w, A(m), 0, 0, 0, 0, 0, 0);
}
static int same_rect(const int32_t *m, int l, int t, int r, int b) { return m[0] == l && m[1] == t && m[2] == r && m[3] == b; }
static uint64_t rect_rgn(int l, int t, int r, int b) { return fle_api2_call(fle_imp_CreateRectRgn, (uint64_t)l, (uint64_t)t, (uint64_t)r, (uint64_t)b, 0, 0, 0, 0); }
extern int fle_gdi_region_covers(uint64_t h, int32_t l, int32_t t, int32_t r, int32_t b);
extern int fle_gdi_region_bounds(uint64_t h, int32_t *l, int32_t *t, int32_t *r, int32_t *b);
extern uint64_t fle_win_update_folds(void) __attribute__((weak));   /* absent in a runtime without the region (the test then shows what fails) */
static uint64_t folds(void) { return fle_win_update_folds ? fle_win_update_folds() : 0; }

/* decision 0247b (alpha3b2): a sibling M above a parent P moves away and uncovers an area over P's child C. A window that does not clip its children (no WS_CLIPCHILDREN) can paint over them, so P has the uncovered area over C as well; with
 * WS_CLIPCHILDREN it has only the part beside C. C itself always has its part, and the top-level window T (whose own visible region does not change) has nothing. Wine 10.0 (harness/alpha3/win_unc.c) gives P the part over C in both cases and T the
 * whole uncovered rectangle (150,0,300,150): both differences are recorded in the decision document (tables/win-vs-0214-vs-alpha3b2.tsv). */
static void uncover_scene(const char *what, int clipchildren, int tx)
{
    HWND t = mk(0x10000, WS_POPUP | WS_VISIBLE | (clipchildren ? 0x02000000u : 0u), tx, 300, 600, 400);   /* with WS_CLIPCHILDREN on the top-level window too: the rectangle that a window without it gets is passed to its children (InvalidateRect) */
    HWND p = mkp(t, 0, 0x54000000u | (clipchildren ? 0x02000000u : 0u), 0, 0, 300, 300);
    HWND c = mkp(p, 0, 0x56000000u, 250, 0, 50, 300);
    HWND m = mkp(t, 0, 0x54000000u, 150, 0, 150, 150);
    uint64_t rgn = fle_api2_call(fle_imp_CreateRectRgn, 0, 0, 0, 0, 0, 0, 0, 0);
    int32_t rc[4];
    char buf[200];

    (void)fle_api2_call(fle_imp_SetWindowPos, m, 0 /* HWND_TOP */, 0, 0, 0, 0, 0x13 /* NOSIZE | NOMOVE | NOACTIVATE */, 0);
    pump();
    (void)fle_desk_compose();
    (void)fle_api2_call(fle_imp_ValidateRect, t, 0, 0, 0, 0, 0, 0, 0);
    (void)fle_api2_call(fle_imp_ValidateRect, p, 0, 0, 0, 0, 0, 0, 0);
    (void)fle_api2_call(fle_imp_ValidateRect, c, 0, 0, 0, 0, 0, 0, 0);
    (void)fle_api2_call(fle_imp_ValidateRect, m, 0, 0, 0, 0, 0, 0, 0);
    (void)fle_api2_call(fle_imp_SetWindowPos, m, 0, 400, 0, 150, 150, 0x14, 0);
    (void)fle_api2_call(fle_imp_GetUpdateRgn, p, rgn, 0, 0, 0, 0, 0, 0);
    snprintf(buf, sizeof buf, "%s: the parent's update region has the part that the moved sibling uncovered beside the child [150,250)", what);
    yes(buf, fle_gdi_region_covers(rgn, 150, 0, 250, 150) == 1);
    snprintf(buf, sizeof buf, "%s: the parent's update region %s the part uncovered over the child [250,300)", what, clipchildren ? "does not have" : "has");
    yes(buf, fle_gdi_region_covers(rgn, 250, 0, 300, 150) == (clipchildren ? 0 : 1));
    snprintf(buf, sizeof buf, "%s: nothing outside what was uncovered (0,0,150,150) is invalid in the parent", what);
    yes(buf, fle_gdi_region_covers(rgn, 0, 0, 150, 150) == 0 && fle_gdi_region_covers(rgn, 0, 150, 300, 300) == 0);
    snprintf(buf, sizeof buf, "%s: the child's own update rectangle is the uncovered part (0,0,50,150)", what);
    yes(buf, gur(c, rc) == 1 && same_rect(rc, 0, 0, 50, 150));
    snprintf(buf, sizeof buf, "%s: the top-level window, whose own visible region does not change, has nothing (decision 0214; Wine 10.0 gives it the uncovered rectangle)", what);
    yes(buf, gur(t, rc) == 0);
}

int main(void)
{
    HWND win;
    int32_t rc[4], bx[4];
    uint64_t rgn, r;
    const uint8_t black[3] = { 0, 0, 0 }, white[3] = { 255, 255, 255 };

    setvbuf(stdout, NULL, _IONBF, 0);
    regclass();
    win = mk(0x10000, WS_POPUP | WS_VISIBLE, 100, 100, 200, 100);
    pump();
    (void)fle_desk_compose();
    yes("a new window has nothing to repaint after the first WM_PAINT", gur(win, rc) == 0 && same_rect(rc, 0, 0, 0, 0));
    rgn = rect_rgn(0, 0, 1, 1);

    /* ---- two disjoint rectangles: the update region is both; the bounding box is what GetUpdateRect gives ---- */
    inval(win, 10, 10, 30, 20);
    inval(win, 100, 60, 120, 80);
    yes("GetUpdateRect of two disjoint rectangles: the bounding box (10,10,120,80)", gur(win, rc) == 1 && same_rect(rc, 10, 10, 120, 80));
    r = fle_api2_call(fle_imp_GetUpdateRgn, win, rgn, 0, 0, 0, 0, 0, 0);
    yes("GetUpdateRgn of two disjoint rectangles: COMPLEXREGION (3)", r == 3);
    yes("  the region covers the first rectangle", fle_gdi_region_covers(rgn, 10, 10, 30, 20) == 1);
    yes("  the region covers the second rectangle", fle_gdi_region_covers(rgn, 100, 60, 120, 80) == 1);
    yes("  the region does not cover the gap between them", fle_gdi_region_covers(rgn, 50, 30, 60, 40) == 0);
    yes("  the bounding box of the region", fle_gdi_region_bounds(rgn, &bx[0], &bx[1], &bx[2], &bx[3]) == 1 && same_rect(bx, 10, 10, 120, 80));

    /* ---- a rectangle inside the region adds nothing; a rectangle that overlaps adds only the new part ---- */
    inval(win, 12, 12, 20, 18);
    r = fle_api2_call(fle_imp_GetUpdateRgn, win, rgn, 0, 0, 0, 0, 0, 0);
    yes("a rectangle inside the region: still the same region (complex, bounding box unchanged)", r == 3 && gur(win, rc) == 1 && same_rect(rc, 10, 10, 120, 80));
    inval(win, 25, 15, 45, 25);
    (void)fle_api2_call(fle_imp_GetUpdateRgn, win, rgn, 0, 0, 0, 0, 0, 0);
    yes("an overlapping rectangle is added as a union: the region covers both rectangles, not the corner outside both", fle_gdi_region_covers(rgn, 10, 10, 30, 20) == 1 && fle_gdi_region_covers(rgn, 25, 15, 45, 25) == 1 && fle_gdi_region_covers(rgn, 30, 10, 45, 15) == 0);

    /* ---- ValidateRect removes a rectangle; ValidateRgn removes a region ---- */
    {
        int32_t m[4];

        rect4(m, 0, 0, 60, 40);
        (void)fle_api2_call(fle_imp_ValidateRect, win, A(m), 0, 0, 0, 0, 0, 0);
    }
    yes("ValidateRect of a rectangle that covers the first part leaves the second rectangle only", gur(win, rc) == 1 && same_rect(rc, 100, 60, 120, 80));
    r = fle_api2_call(fle_imp_GetUpdateRgn, win, rgn, 0, 0, 0, 0, 0, 0);
    yes("  and the region is one rectangle (SIMPLEREGION)", r == 2);
    {
        uint64_t half = rect_rgn(100, 60, 110, 80);

        (void)fle_api2_call(fle_imp_ValidateRgn, win, half, 0, 0, 0, 0, 0, 0);
    }
    yes("ValidateRgn of the left half of the rectangle (it was a no-op when the region was a bounding box)", gur(win, rc) == 1 && same_rect(rc, 110, 60, 120, 80));

    /* ---- BeginPaint: rcPaint is the bounding box, the DC clips to the region ---- */
    inval(win, 0, 0, 200, 100);
    {
        uint8_t ps[72];
        uint8_t *bits;
        uint64_t mdc, dc;

        (void)fle_api2_call(fle_imp_ValidateRect, win, 0, 0, 0, 0, 0, 0, 0);
        yes("ValidateRect(NULL) removes the whole region", gur(win, rc) == 0);
        inval(win, 10, 10, 30, 20);
        inval(win, 100, 60, 120, 80);
        mdc = mem_dib(200, 100, &bits);
        memset(bits, 0xFF, (size_t)200 * 100 * 4);
        dc = fle_api2_call(fle_imp_BeginPaint, win, A(ps), 0, 0, 0, 0, 0, 0);
        {
            int32_t pr[4];

            memcpy(pr, ps + 12, 16);
            yes("BeginPaint: PAINTSTRUCT.rcPaint is the bounding box (10,10,120,80)", same_rect(pr, 10, 10, 120, 80));
        }
        yes("  the update region is empty afterwards", gur(win, rc) == 0);
        yes("  a BitBlt of the whole client area through the DC of BeginPaint succeeds", bitblt(dc, 0, 0, 200, 100, mdc, 0, 0, 0x00CC0020u) != 0);
        (void)fle_api2_call(fle_imp_EndPaint, win, A(ps), 0, 0, 0, 0, 0, 0);
        (void)fle_desk_compose();
        pixel("inside the first rectangle: painted", 100 + 20, 100 + 15, white);
        pixel("inside the second rectangle: painted", 100 + 110, 100 + 70, white);
        pixel("in the gap, inside the bounding box: NOT painted (the DC is clipped to the region)", 100 + 60, 100 + 40, black);
        pixel("outside the bounding box: not painted", 100 + 5, 100 + 5, black);
    }

    /* ---- RedrawWindow with a region adds its rectangles ---- */
    {
        uint64_t a = rect_rgn(10, 10, 30, 20), b = rect_rgn(100, 60, 120, 80), u = rect_rgn(0, 0, 1, 1);

        (void)fle_api2_call(fle_imp_CombineRgn, u, a, b, 2 /* RGN_OR */, 0, 0, 0, 0);
        (void)fle_api2_call(fle_imp_RedrawWindow, win, 0, u, 0x41 /* RDW_INVALIDATE | RDW_NOCHILDREN */, 0, 0, 0, 0);
        r = fle_api2_call(fle_imp_GetUpdateRgn, win, rgn, 0, 0, 0, 0, 0, 0);
        yes("RedrawWindow(hrgn of two rectangles, RDW_INVALIDATE): the update region is both (COMPLEXREGION)", r == 3);
        yes("  it does not cover the gap", fle_gdi_region_covers(rgn, 50, 30, 60, 40) == 0);
        yes("  GetUpdateRect is the bounding box", gur(win, rc) == 1 && same_rect(rc, 10, 10, 120, 80));
        (void)fle_api2_call(fle_imp_RedrawWindow, win, 0, u, 0x8 /* RDW_VALIDATE */, 0, 0, 0, 0);
        yes("RedrawWindow(hrgn, RDW_VALIDATE) removes the same rectangles", gur(win, rc) == 0);
    }

    /* ---- too many rectangles: folded to the bounding box, counted ---- */
    {
        uint64_t f0 = folds();

        (void)fle_api2_call(fle_imp_ValidateRect, win, 0, 0, 0, 0, 0, 0, 0);
        for (int i = 0; i < 40; i++) inval(win, 4 * i, (i % 2) * 20, 4 * i + 2, (i % 2) * 20 + 2);
        r = fle_api2_call(fle_imp_GetUpdateRgn, win, rgn, 0, 0, 0, 0, 0, 0);
        yes("40 disjoint rectangles: folded to the bounding box at least once (counted)", folds() > f0);
        yes("  the bounding box is right (0,0,158,22), and the folded part is one rectangle that covers what lay between the small ones (an over-estimate, as every region was before)", gur(win, rc) == 1 && same_rect(rc, 0, 0, 158, 22) && fle_gdi_region_covers(rgn, 0, 0, 100, 22) == 1);
        (void)fle_api2_call(fle_imp_ValidateRect, win, 0, 0, 0, 0, 0, 0, 0);
    }

    /* ---- the fold is on the safe side: random invalidations and validations against an exact bitmap model; the region never misses a model pixel (folds), and is exactly the model when nothing was folded ---- */
    {
        static uint8_t mdl[100][200];
        uint32_t seed = 12345u;
        uint64_t f0;
        int missed = 0, exact_bad = 0, nfold_trials = 0;

#define RND(n) (seed = seed * 1103515245u + 12345u, (int)((seed >> 16) % (uint32_t)(n)))
        for (int trial = 0; trial < 6; trial++) {
            int nops = trial < 3 ? 16 : 400;   /* 16 operations stay within the 32 rectangles; 400 fold */

            (void)fle_api2_call(fle_imp_ValidateRect, win, 0, 0, 0, 0, 0, 0, 0);
            memset(mdl, 0, sizeof mdl);
            f0 = folds();
            for (int k = 0; k < nops; k++) {
                int x = RND(190), y = RND(92), w = 1 + RND(6), h = 1 + RND(6);
                int32_t m[4];

                if (RND(10) == 0) { w = 1 + RND(60); h = 1 + RND(40); rect4(m, x, y, x + w, y + h); (void)fle_api2_call(fle_imp_ValidateRect, win, A(m), 0, 0, 0, 0, 0, 0); for (int yy = y; yy < y + h && yy < 100; yy++) for (int xx = x; xx < x + w && xx < 200; xx++) mdl[yy][xx] = 0; }
                else { inval(win, x, y, x + w, y + h); for (int yy = y; yy < y + h && yy < 100; yy++) for (int xx = x; xx < x + w && xx < 200; xx++) mdl[yy][xx] = 1; }
                if (trial >= 3 && k % 8 == 7) {
                    (void)fle_api2_call(fle_imp_GetUpdateRgn, win, rgn, 0, 0, 0, 0, 0, 0);
                    for (int yy = 0; yy < 100; yy++) for (int xx = 0; xx < 200; xx++) if (mdl[yy][xx] && !fle_gdi_region_covers(rgn, xx, yy, xx + 1, yy + 1)) missed++;
                }
            }
            (void)fle_api2_call(fle_imp_GetUpdateRgn, win, rgn, 0, 0, 0, 0, 0, 0);
            for (int yy = 0; yy < 100; yy++) for (int xx = 0; xx < 200; xx++) {
                int in = fle_gdi_region_covers(rgn, xx, yy, xx + 1, yy + 1);
                if (mdl[yy][xx] && !in) missed++;
                if (trial < 3 && folds() == f0 && in != mdl[yy][xx]) exact_bad++;
            }
            if (folds() > f0) nfold_trials++;
        }
#undef RND
        yes("random invalidate/validate (6 trials): the update region never misses a pixel the exact union holds (the fold is on the safe side)", missed == 0);
        yes("  the 3 trials of 16 operations are not folded and the region is exactly the union (0 pixels differ)", exact_bad == 0);
        yes("  the 3 trials of 400 operations are folded (the fold path is run)", nfold_trials >= 3);
        (void)fle_api2_call(fle_imp_ValidateRect, win, 0, 0, 0, 0, 0, 0, 0);
    }

    uncover_scene("parent without WS_CLIPCHILDREN", 0, 400);
    uncover_scene("parent with WS_CLIPCHILDREN", 1, 1100);

    /* ---- showing a window: the whole client area, whatever was invalid while it was hidden ---- */
    (void)fle_api2_call(fle_imp_ShowWindow, win, 0, 0, 0, 0, 0, 0, 0);
    inval(win, 10, 10, 30, 20);
    (void)fle_api2_call(fle_imp_ShowWindow, win, 8 /* SW_SHOWNA */, 0, 0, 0, 0, 0, 0);
    yes("a window shown again has its whole client area to repaint even when a part was invalid while hidden", gur(win, rc) == 1 && same_rect(rc, 0, 0, 200, 100));

    printf("checks %d failed %d\n", checks, fails);
    return fails ? 1 : 0;
}

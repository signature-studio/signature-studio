/* Decision 0253 (alpha5, test tool; the scaffold of harness/alpha3/upd_region.c): the WM_ERASEBKGND that RedrawWindow(RDW_ERASENOW / RDW_UPDATENOW) sends carries a device context in wParam (Learn WM_ERASEBKGND: "wParam: A handle to the device context").
 * It runs the runtime itself with a window procedure of the shape of FL's TEdgeBrowser (WM_ERASEBKGND: FillRect(wParam, ...), which stopped the guest with "hDC is not a DC of this runtime" when wParam was 0). Prints one line per check ("ok" / "FAIL") and ends with `checks N failed M`. */
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

/* the window procedure of FL's TEdgeBrowser shape: WM_ERASEBKGND fills with the DC of the message (FillRect(wParam, ...)); WM_PAINT is BeginPaint ... EndPaint; the rest is DefWindowProcW. It records what it was given. */
#define WM_PAINT_ 0x000Fu
#define WM_ERASEBKGND_ 0x0014u
static struct { uint64_t hwnd, wp; uint64_t fill; int info_ok; uint64_t info_hwnd; uint64_t ps_hdc; } erase_log[64];
static int erase_n, paint_n;
static uint64_t erase_result = 1;   /* what the window procedure answers to WM_ERASEBKGND */
static uint32_t last_ps_ferase;
static uint64_t last_ps_hdc;
static uint8_t ps_buf[72];
static void wndproc(void)
{
    uint64_t hwnd = R[1], msg = R[2], wp = R[8], lp = R[9], res;

    if (msg == WM_ERASEBKGND_) {
        int k = erase_n++ < 64 ? erase_n - 1 : 63;
        static int32_t rect[4] = { 0, 0, 10, 10 };
        fle_gdc_info gi;

        erase_log[k].hwnd = hwnd;
        erase_log[k].wp = wp;
        erase_log[k].fill = 0xFFFFFFFFu;
        erase_log[k].info_ok = wp != 0 && fle_gdc_dc_info(wp, &gi);
        erase_log[k].info_hwnd = erase_log[k].info_ok ? gi.hwnd : 0;
        erase_log[k].ps_hdc = last_ps_hdc;
        if (wp != 0) erase_log[k].fill = fle_api2_call(fle_imp_FillRect, wp, A(rect), 6 /* COLOR_WINDOW + 1 */, 0, 0, 0, 0, 0);
        R[0] = erase_result;
        return;
    }
    if (msg == WM_PAINT_) {
        paint_n++;
        last_ps_hdc = 0;
        (void)fle_api2_call(fle_imp_BeginPaint, hwnd, A(ps_buf), 0, 0, 0, 0, 0, 0);
        memcpy(&last_ps_hdc, ps_buf, 8);
        memcpy(&last_ps_ferase, ps_buf + 8, 4);
        if (erase_n > 0 && erase_n <= 64) erase_log[erase_n - 1].ps_hdc = last_ps_hdc;
        (void)fle_api2_call(fle_imp_EndPaint, hwnd, A(ps_buf), 0, 0, 0, 0, 0, 0);
        R[0] = 0;
        return;
    }
    res = fle_api2_call(fle_imp_DefWindowProcW, hwnd, msg, wp, lp, 0, 0, 0, 0);
    R[0] = res;
}
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


static int fails, checks;
static void yes(const char *what, int cond)
{
    checks++;
    if (!cond) { fails++; printf("FAIL %s\n", what); } else printf("ok %s\n", what);
}
static void redraw(HWND w, uint32_t flags) { (void)fle_api2_call(fle_imp_RedrawWindow, w, 0, 0, flags, 0, 0, 0, 0); }

int main(void)
{
    HWND win, par, c1, c2;
    int n0;

    setvbuf(stdout, NULL, _IONBF, 0);
    regclass();
    win = mk(0x10000, WS_POPUP | WS_VISIBLE, 100, 100, 200, 100);
    pump();
    (void)fle_desk_compose();

    /* ---- RedrawWindow(RDW_INVALIDATE | RDW_ERASE | RDW_UPDATENOW): Learn WM_ERASEBKGND "wParam: A handle to the device context" ---- */
    n0 = erase_n;
    redraw(win, 0x0105u);
    yes("RedrawWindow(INVALIDATE | ERASE | UPDATENOW) sends WM_ERASEBKGND once: the window procedure answered nonzero, so the WM_PAINT that follows does not erase again (Learn: nonzero means no further erasing is required)", erase_n == n0 + 1);
    yes("  its wParam is a device context (not 0)", erase_n > n0 && erase_log[n0].wp != 0);
    yes("  the DC is the DC of that window (GetDC(hWnd))", erase_n > n0 && erase_log[n0].info_ok && erase_log[n0].info_hwnd == win);
    yes("  FillRect with it succeeds (returns nonzero)", erase_n > n0 && erase_log[n0].fill == 1);
    yes("  the DC is released when the message has been answered (not live any more)", erase_n > n0 && erase_log[n0].wp != 0 && !fle_gdc_is_live(erase_log[n0].wp));
    yes("  the WM_PAINT of UPDATENOW follows", paint_n >= 1);

    /* ---- a window procedure that answers 0 leaves the window marked for erasing (Learn): PAINTSTRUCT.fErase is TRUE. Decision 0323 (1): the erase is a delayed erase (Wine 10.0 UPDATE_DELAYED_ERASE), so BeginPaint does not send the WM_ERASEBKGND a second time ---- */
    erase_result = 0;
    n0 = erase_n;
    redraw(win, 0x0105u);
    yes("a WM_ERASEBKGND answered with 0 leaves the erase request pending (delayed erase): BeginPaint does not send it again (1 message in all)", erase_n == n0 + 1);
    yes("  and PAINTSTRUCT.fErase is TRUE", last_ps_ferase == 1);
    erase_result = 1;

    /* ---- RDW_ERASENOW alone ---- */
    n0 = erase_n;
    redraw(win, 0x0205u);
    yes("RedrawWindow(INVALIDATE | ERASE | ERASENOW) sends a WM_ERASEBKGND with a DC (no WM_PAINT)", erase_n == n0 + 1 && erase_log[n0].wp != 0 && erase_log[n0].fill == 1);

    /* ---- a parent and two children (ALLCHILDREN): each window gets its own DC ---- */
    par = mk(0x10000, WS_POPUP | WS_VISIBLE, 400, 100, 300, 200);
    c1 = mkp(par, 0, 0x54000000u, 0, 0, 100, 100);
    c2 = mkp(par, 0, 0x54000000u, 150, 50, 100, 100);
    pump();
    (void)fle_desk_compose();
    n0 = erase_n;
    redraw(par, 0x0185u);
    {
        int seen_p = 0, seen_1 = 0, seen_2 = 0;

        for (int k = n0; k < erase_n && k < 64; k++) {
            if (erase_log[k].hwnd == par) seen_p = 1;
            if (erase_log[k].hwnd == c1) seen_1 = 1;
            if (erase_log[k].hwnd == c2) seen_2 = 1;
        }
        yes("RedrawWindow(0x185) on a parent: WM_ERASEBKGND to the parent and both children", seen_p && seen_1 && seen_2);
    }
    {
        int all = erase_n >= n0 + 3, own = erase_n >= n0 + 3, distinct = erase_n >= n0 + 3;

        for (int k = n0; k < erase_n && k < 64; k++) {
            if (erase_log[k].wp == 0 || erase_log[k].fill != 1) all = 0;
            if (!erase_log[k].info_ok || erase_log[k].info_hwnd != erase_log[k].hwnd) own = 0;
            for (int j = n0; j < k; j++) if (erase_log[j].wp == erase_log[k].wp && 0) distinct = 0;
        }
        yes("  every message has a DC and FillRect with it succeeds", all);
        yes("  every DC belongs to the window that gets the message", own);
        (void)distinct;
    }

    /* ---- the BeginPaint path is unchanged: the DC of the erase message is the DC of PAINTSTRUCT.hdc ---- */
    pump();
    n0 = erase_n;
    (void)fle_api2_call(fle_imp_InvalidateRect, win, 0, 1, 0, 0, 0, 0, 0);
    pump();
    yes("InvalidateRect(bErase) then the message loop: WM_ERASEBKGND comes from BeginPaint with PAINTSTRUCT.hdc", erase_n == n0 + 1 && erase_log[n0].wp != 0 && erase_log[n0].wp == erase_log[n0].ps_hdc);

    printf("checks %d failed %d\n", checks, fails);
    return fails ? 1 : 0;
}

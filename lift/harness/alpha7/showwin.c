/* Decision 0258 (alpha7, test tool; the scaffold of harness/alpha5/erase_dc.c): ShowWindow and the maximized / minimized / restored states (Learn ShowWindow: "SW_SHOWMAXIMIZED: Activates the window and displays it as a maximized window",
 * "SW_SHOWNORMAL / SW_RESTORE: ... the system restores it to its original size and position"; Learn WM_SIZE: SIZE_MAXIMIZED 2 / SIZE_MINIMIZED 1 / SIZE_RESTORED 0, lParam the client size, "The DefWindowProc function sends the WM_SIZE and WM_MOVE
 * messages when it processes the WM_WINDOWPOSCHANGED message"; Learn WM_GETMINMAXINFO / MINMAXINFO; Learn WM_WINDOWPOSCHANGING: "For a window with the WS_OVERLAPPED or WS_THICKFRAME style, the DefWindowProc function sends the
 * WM_GETMINMAXINFO message to the window"). It runs the runtime itself with a window procedure that records the messages (number, wParam, lParam; for WM_GETMINMAXINFO also the MINMAXINFO it was given) and passes them to
 * DefWindowProcW. Prints one line per check ("ok ..." / "FAIL ...") and ends with "checks N failed M". */
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

#define WM_MOVE_ 0x0003u
#define WM_SIZE_ 0x0005u
#define WM_SHOWWINDOW_ 0x0018u
#define WM_GETMINMAXINFO_ 0x0024u
#define WM_WINDOWPOSCHANGING_ 0x0046u
#define WM_WINDOWPOSCHANGED_ 0x0047u
#define WM_NCCALCSIZE_ 0x0083u
typedef struct { uint32_t msg; uint64_t wp, lp; int32_t mm[10]; } mlog_t;
static mlog_t mlog[256];
static int mlog_n;
static int mm_override;              /* 1: the window procedure sets ptMaxSize = 800 x 600 and ptMaxPosition = 10, 20 */
static void wndproc(void)
{
    uint64_t hwnd = R[1], msg = R[2], wp = R[8], lp = R[9], res;

    if (msg == WM_MOVE_ || msg == WM_SIZE_ || msg == WM_SHOWWINDOW_ || msg == WM_GETMINMAXINFO_ || msg == WM_WINDOWPOSCHANGING_ || msg == WM_WINDOWPOSCHANGED_ || msg == WM_NCCALCSIZE_) {
        if (mlog_n < 256) {
            mlog[mlog_n].msg = (uint32_t)msg;
            mlog[mlog_n].wp = wp;
            mlog[mlog_n].lp = lp;
            if (msg == WM_GETMINMAXINFO_)
                for (int k = 0; k < 10; k++) mlog[mlog_n].mm[k] = (int32_t)fle_ld(lp + 4u * k, 4);
            if (msg == WM_GETMINMAXINFO_ && mm_override) {
                fle_st(lp + 8, 4, 800); fle_st(lp + 12, 4, 600); fle_st(lp + 16, 4, 10); fle_st(lp + 20, 4, 20);
            }
        }
        mlog_n++;
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
#define PACK(x, y) (((uint64_t)(uint16_t)(y) << 16) | (uint16_t)(x))
static int32_t rc[4];
static void rect_of(HWND w) { (void)fle_api2_call(fle_imp_GetWindowRect, w, A(rc), 0, 0, 0, 0, 0, 0); }
static int is_rect(HWND w, int l, int t, int r, int b) { rect_of(w); return rc[0] == l && rc[1] == t && rc[2] == r && rc[3] == b; }
static void show(HWND w, int cmd) { mlog_n = 0; (void)fle_api2_call(fle_imp_ShowWindow, w, (uint64_t)cmd, 0, 0, 0, 0, 0, 0); }
static int zoomed(HWND w) { return (int)(fle_api2_call(fle_imp_IsZoomed, w, 0, 0, 0, 0, 0, 0, 0) & 1); }
static int iconic(HWND w) { return (int)(fle_api2_call(fle_imp_IsIconic, w, 0, 0, 0, 0, 0, 0, 0) & 1); }
static int count_of(uint32_t msg) { int n = 0; for (int k = 0; k < mlog_n && k < 256; k++) if (mlog[k].msg == msg) n++; return n; }
static int pos_of(uint32_t msg) { for (int k = 0; k < mlog_n && k < 256; k++) if (mlog[k].msg == msg) return k; return -1; }
static int last_pos_of(uint32_t msg) { int r = -1; for (int k = 0; k < mlog_n && k < 256; k++) if (mlog[k].msg == msg) r = k; return r; }
/* the order GETMINMAXINFO < WINDOWPOSCHANGING < NCCALCSIZE < WINDOWPOSCHANGED < MOVE < SIZE */
static int ordered(int with_minmax)
{
    int a = pos_of(WM_GETMINMAXINFO_), b = pos_of(WM_WINDOWPOSCHANGING_), c = pos_of(WM_NCCALCSIZE_), d = pos_of(WM_WINDOWPOSCHANGED_), e = pos_of(WM_MOVE_), f = pos_of(WM_SIZE_);
    return (!with_minmax || (a >= 0 && a < b)) && b >= 0 && c > b && d > c && e > d && f > e;
}

int main(void)
{
    HWND win, ovl;
    int k;

    setvbuf(stdout, NULL, _IONBF, 0);
    regclass();
    win = mk(0x10000, WS_POPUP | WS_VISIBLE, 100, 100, 200, 100);
    pump();
    (void)fle_desk_compose();

    /* ---- SW_SHOWMAXIMIZED of a visible window: WM_GETMINMAXINFO first, then the rectangle change ---- */
    mlog_n = 0;
    show(win, 3);
    yes("ShowWindow(SW_SHOWMAXIMIZED): IsZoomed is TRUE", zoomed(win) == 1);
    yes("  WM_GETMINMAXINFO, WM_WINDOWPOSCHANGING, WM_NCCALCSIZE, WM_WINDOWPOSCHANGED, WM_MOVE, WM_SIZE in this order", ordered(1));
    k = pos_of(WM_GETMINMAXINFO_);
    yes("  WM_GETMINMAXINFO: wParam 0, lParam a MINMAXINFO", k >= 0 && mlog[k].wp == 0 && mlog[k].lp != 0);
    yes("  MINMAXINFO ptMaxSize is the work area (1920 x 1080), ptMaxPosition 0, 0", k >= 0 && mlog[k].mm[2] == 1920 && mlog[k].mm[3] == 1080 && mlog[k].mm[4] == 0 && mlog[k].mm[5] == 0);
    yes("  MINMAXINFO ptMaxTrackSize is the virtual screen (1920 x 1080)", k >= 0 && mlog[k].mm[8] == 1920 && mlog[k].mm[9] == 1080);
    yes("  the window rectangle is the maximized rectangle (0, 0, 1920, 1080)", is_rect(win, 0, 0, 1920, 1080));
    k = last_pos_of(WM_SIZE_);
    yes("  WM_SIZE: wParam SIZE_MAXIMIZED (2), lParam the client size 1920 x 1080", k >= 0 && mlog[k].wp == 2 && mlog[k].lp == PACK(1920, 1080));
    k = last_pos_of(WM_MOVE_);
    yes("  WM_MOVE: lParam the client origin 0, 0", k >= 0 && mlog[k].lp == 0);

    /* ---- the same again: nothing changes, nothing is sent ---- */
    show(win, 3);
    yes("ShowWindow(SW_SHOWMAXIMIZED) of a window that is already maximized sends nothing", mlog_n == 0);

    /* ---- SW_RESTORE: the original rectangle, WM_SIZE with SIZE_RESTORED ---- */
    show(win, 9);
    yes("ShowWindow(SW_RESTORE): IsZoomed is FALSE, the rectangle is the original (100, 100, 300, 200)", zoomed(win) == 0 && is_rect(win, 100, 100, 300, 200));
    yes("  WM_WINDOWPOSCHANGING, WM_NCCALCSIZE, WM_WINDOWPOSCHANGED, WM_MOVE, WM_SIZE in this order and no WM_GETMINMAXINFO", ordered(0) && count_of(WM_GETMINMAXINFO_) == 0);
    k = last_pos_of(WM_SIZE_);
    yes("  WM_SIZE: wParam SIZE_RESTORED (0), lParam 200 x 100", k >= 0 && mlog[k].wp == 0 && mlog[k].lp == PACK(200, 100));

    /* ---- SW_SHOWNORMAL on a normal visible window: nothing ---- */
    show(win, 1);
    yes("ShowWindow(SW_SHOWNORMAL) of a normal visible window sends nothing", mlog_n == 0);

    /* ---- SW_MINIMIZE then SW_RESTORE ---- */
    show(win, 6);
    yes("ShowWindow(SW_MINIMIZE): IsIconic is TRUE", iconic(win) == 1);
    k = last_pos_of(WM_SIZE_);
    yes("  WM_SIZE: wParam SIZE_MINIMIZED (1)", k >= 0 && mlog[k].wp == 1);
    yes("  WM_WINDOWPOSCHANGING, WM_NCCALCSIZE, WM_WINDOWPOSCHANGED, WM_MOVE, WM_SIZE in this order", ordered(0));
    show(win, 9);
    yes("ShowWindow(SW_RESTORE) of a minimized window: not iconic, the rectangle is the original (100, 100, 300, 200)", iconic(win) == 0 && is_rect(win, 100, 100, 300, 200));
    k = last_pos_of(WM_SIZE_);
    yes("  WM_SIZE: wParam SIZE_RESTORED (0), lParam 200 x 100", k >= 0 && mlog[k].wp == 0 && mlog[k].lp == PACK(200, 100));

    /* ---- the round trip maximize -> restore -> minimize -> restore leaves the original rectangle ---- */
    show(win, 3);
    show(win, 1);
    show(win, 2);
    show(win, 1);
    yes("maximize, restore, minimize, restore: back to the original rectangle, neither zoomed nor iconic", is_rect(win, 100, 100, 300, 200) && !zoomed(win) && !iconic(win));

    /* ---- a maximized window that is minimized comes back maximized ---- */
    show(win, 3);
    show(win, 6);
    yes("a maximized window that is minimized: IsIconic, not IsZoomed", iconic(win) == 1 && zoomed(win) == 0);
    show(win, 9);
    k = last_pos_of(WM_SIZE_);
    yes("  SW_RESTORE gives the maximized state back: IsZoomed, rectangle (0, 0, 1920, 1080), WM_SIZE SIZE_MAXIMIZED", zoomed(win) == 1 && iconic(win) == 0 && is_rect(win, 0, 0, 1920, 1080) && k >= 0 && mlog[k].wp == 2);
    show(win, 1);
    yes("  SW_SHOWNORMAL then gives the original rectangle", zoomed(win) == 0 && is_rect(win, 100, 100, 300, 200));

    /* ---- the window procedure overrides the defaults ---- */
    mm_override = 1;
    show(win, 3);
    mm_override = 0;
    yes("the window procedure sets ptMaxSize 800 x 600, ptMaxPosition 10, 20: the maximized rectangle is (10, 20, 810, 620)", is_rect(win, 10, 20, 810, 620));
    k = last_pos_of(WM_SIZE_);
    yes("  WM_SIZE: SIZE_MAXIMIZED, lParam 800 x 600", k >= 0 && mlog[k].wp == 2 && mlog[k].lp == PACK(800, 600));
    show(win, 1);

    /* ---- a hidden window shown maximized: WM_SHOWWINDOW (Learn) between the rectangle request and the change ---- */
    ovl = mk(0x10000, WS_POPUP, 300, 300, 150, 80);
    pump();
    show(ovl, 3);
    yes("ShowWindow(SW_SHOWMAXIMIZED) of a hidden window: shown and maximized, rectangle (0, 0, 1920, 1080)", is_rect(ovl, 0, 0, 1920, 1080) && zoomed(ovl) == 1);
    yes("  WM_GETMINMAXINFO, WM_SHOWWINDOW(TRUE, 0), then WM_WINDOWPOSCHANGING ... WM_SIZE", pos_of(WM_GETMINMAXINFO_) >= 0 && pos_of(WM_GETMINMAXINFO_) < pos_of(WM_SHOWWINDOW_) && pos_of(WM_SHOWWINDOW_) < pos_of(WM_WINDOWPOSCHANGING_) && ordered(1));
    k = pos_of(WM_SHOWWINDOW_);
    yes("  WM_SHOWWINDOW: wParam TRUE, lParam 0", k >= 0 && mlog[k].wp == 1 && mlog[k].lp == 0);

    /* ---- DefWindowProc(WM_WINDOWPOSCHANGING) of an overlapped window sends WM_GETMINMAXINFO; of a pop-up window it does not ---- */
    ovl = mk(0x10000, 0u, 500, 100, 150, 80);        /* neither WS_POPUP nor WS_CHILD: an overlapped window */
    pump();
    mlog_n = 0;
    (void)fle_api2_call(fle_imp_SetWindowPos, ovl, 0, 500, 100, 160, 90, 0x0014, 0);   /* SWP_NOZORDER | SWP_NOACTIVATE: a size change */
    yes("SetWindowPos on an overlapped window: its WM_WINDOWPOSCHANGING reaches DefWindowProc, which sends WM_GETMINMAXINFO (before the change is applied)", count_of(WM_GETMINMAXINFO_) == 1 && pos_of(WM_GETMINMAXINFO_) > pos_of(WM_WINDOWPOSCHANGING_) && pos_of(WM_GETMINMAXINFO_) < pos_of(WM_WINDOWPOSCHANGED_));
    mlog_n = 0;
    (void)fle_api2_call(fle_imp_SetWindowPos, win, 0, 100, 100, 210, 110, 0x0014, 0);
    yes("SetWindowPos on a pop-up window sends no WM_GETMINMAXINFO", count_of(WM_GETMINMAXINFO_) == 0 && count_of(WM_WINDOWPOSCHANGING_) == 1);

    /* ---- DefWindowProc(WM_GETMINMAXINFO) leaves the defaults and returns 0 (it stopped the process before) ---- */
    {
        uint8_t buf[40];
        int32_t *p = (int32_t *)buf;
        uint64_t r;

        memset(buf, 0, sizeof buf);
        p[2] = 111; p[3] = 222; p[4] = 3; p[5] = 4;
        r = fle_api2_call(fle_imp_DefWindowProcW, win, WM_GETMINMAXINFO_, 0, A(buf), 0, 0, 0, 0);
        yes("DefWindowProc(WM_GETMINMAXINFO) returns 0 and leaves the structure as it was", r == 0 && p[2] == 111 && p[3] == 222 && p[4] == 3 && p[5] == 4);
    }

    printf("checks %d failed %d\n", checks, fails);
    return fails ? 1 : 0;
}

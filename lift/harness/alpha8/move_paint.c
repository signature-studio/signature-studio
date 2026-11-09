/* Decision 0262 (alpha8 investigation, test tool): see the comment below. */
#include <stdio.h>
#include <string.h>
#include <stdint.h>

#ifdef API2_WINE
#include <windows.h>
static void paint_cb(HWND h, int32_t rc[4]);
static LRESULT CALLBACK wp(HWND h, UINT m, WPARAM w, LPARAM l)
{
    if (m == WM_ERASEBKGND) return 1;
    if (m == WM_PAINT) { PAINTSTRUCT ps; BeginPaint(h, &ps); { int32_t rc[4] = { ps.rcPaint.left, ps.rcPaint.top, ps.rcPaint.right, ps.rcPaint.bottom }; paint_cb(h, rc); } EndPaint(h, &ps); return 0; }
    return DefWindowProcW(h, m, w, l);
}
static void pump(void) { MSG msg; while (PeekMessageW(&msg, NULL, 0, 0, PM_REMOVE)) DispatchMessageW(&msg); }
static HWND mk(DWORD style, int x, int y, int cx, int cy, HWND parent) { return CreateWindowExW(0, L"upd", L"", style, x, y, cx, cy, parent, NULL, GetModuleHandleW(NULL), NULL); }
static void regclass(void) { WNDCLASSW wc; memset(&wc, 0, sizeof wc); wc.lpfnWndProc = wp; wc.hInstance = GetModuleHandleW(NULL); wc.lpszClassName = L"upd"; wc.hbrBackground = (HBRUSH)(COLOR_WINDOW + 1); RegisterClassW(&wc); }
#define W_(x) x
#else
#define _GNU_SOURCE
#include "fle_runtime.h"
#include "fle_sse.h"
#include "fle_dispatch.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_callback.h"
#include "fle_class.h"
#include "fle_win.h"
#include "fle_thread.h"
#include "fle_cursor.h"
#include "fle_wall_ia2.h"
#include "fle_api2.h"
#include "fle_sysmetrics.h"
#include "fle_raster.h"
uint64_t R[16];
fle_eflags FL;
fle_xmm X[16];
void fle_imp_PeekMessageW(void);
void fle_imp_DispatchMessageW(void);
typedef uint64_t HWND;
typedef struct { int32_t left, top, right, bottom; } RECT;
typedef unsigned int DWORD, UINT;
typedef int BOOL;
#define NULL_ ((void *)0)
#define A(x) ((uint64_t)(uintptr_t)(x))
#define VA_WP 0x2000
#define WS_POPUP 0x80000000u
#define WS_CHILD 0x40000000u
#define WS_VISIBLE 0x10000000u
#define WS_CLIPCHILDREN 0x02000000u
#define SW_INVALIDATE 2
static void paint_cb(HWND h, int32_t rc[4]);
static void wndproc(void)
{
    uint64_t hw = R[1], msg = R[2], wp_ = R[8], lp = R[9];
    if (msg == 0x14) { R[0] = 1; return; }
    if (msg == 0x0F) {
        uint8_t ps[72];
        memset(ps, 0, sizeof ps);
        (void)fle_api2_call(fle_imp_BeginPaint, hw, A(ps), 0, 0, 0, 0, 0, 0);
        paint_cb(hw, (int32_t *)(ps + 12));
        (void)fle_api2_call(fle_imp_EndPaint, hw, A(ps), 0, 0, 0, 0, 0, 0);
        R[0] = 0;
        return;
    }
    R[0] = fle_api2_call(fle_imp_DefWindowProcW, hw, msg, wp_, lp, 0, 0, 0, 0);
}
const fle_fn_entry fle_dispatch_table[] = { { VA_WP, wndproc } };
const uint64_t fle_dispatch_count = 1;
static uint64_t stk[0x4000] __attribute__((aligned(4096)));
static uint16_t clsname[] = { 'u', 'p', 'd', 0 };
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
static HWND mk(DWORD style, int x, int y, int cx, int cy, HWND parent)
{
    uint64_t h, sp0 = R[4], sv[16];
    uint16_t empty[1] = { 0 };
    memcpy(sv, R, sizeof sv);
    R[4] = (R[4] - 0x80) & ~UINT64_C(15);
    R[4] -= 8;
    fle_st(R[4], 8, 0xAAAA0000);
    fle_st(R[4] + 0x28, 8, (uint32_t)x); fle_st(R[4] + 0x30, 8, (uint32_t)y); fle_st(R[4] + 0x38, 8, (uint32_t)cx); fle_st(R[4] + 0x40, 8, (uint32_t)cy);
    fle_st(R[4] + 0x48, 8, parent); fle_st(R[4] + 0x50, 8, 0); fle_st(R[4] + 0x58, 8, 0x400000); fle_st(R[4] + 0x60, 8, 0);
    R[1] = 0; R[2] = A(clsname); R[8] = A(empty); R[9] = style;
    fle_imp_CreateWindowExW();
    h = R[0];
    R[4] = sp0;
    (void)sv;
    return h;
}
static void pump(void)
{
    uint8_t msg[FLE_MSG_SIZE];
    while (fle_api2_call(fle_imp_PeekMessageW, A(msg), 0, 0, 0, 1, 0, 0, 0) & 1) (void)fle_api2_call(fle_imp_DispatchMessageW, A(msg), 0, 0, 0, 0, 0, 0, 0);
}
static BOOL InvalidateRect(HWND h, const RECT *r, BOOL e) { return (BOOL)fle_api2_call(fle_imp_InvalidateRect, h, A(r), e, 0, 0, 0, 0, 0); }
static BOOL ValidateRect(HWND h, const RECT *r) { return (BOOL)fle_api2_call(fle_imp_ValidateRect, h, A(r), 0, 0, 0, 0, 0, 0); }
static BOOL GetUpdateRect(HWND h, RECT *r, BOOL e) { return (BOOL)fle_api2_call(fle_imp_GetUpdateRect, h, A(r), e, 0, 0, 0, 0, 0); }
static BOOL ScrollWindowEx(HWND h, int dx, int dy, const RECT *a, const RECT *b, void *c, RECT *d, UINT f) { return (BOOL)fle_api2_call(fle_imp_ScrollWindowEx, h, (uint32_t)dx, (uint32_t)dy, A(a), A(b), A(c), A(d), f); }
static BOOL DestroyWindow(HWND h) { return (BOOL)fle_api2_call(fle_imp_DestroyWindow, h, 0, 0, 0, 0, 0, 0, 0); }
#endif



/* Decision 0262 (alpha8 investigation, test tool; the scaffold of harness/alpha7/show_seq.c): the update regions and the paints after a child window is moved with SetWindowPos(SWP_NOZORDER | SWP_NOACTIVATE), the way FL moves its Mixer
 * panel (TFXForm, lid 64) while the header is dragged (decision 0240). Run (a) against Wine 10.0's user32 (API2_WINE: mingw build, headless Wine; UNOFFICIAL) and (b) against the runtime (tools/build-movepaint.sh).
 * The tree (the rectangles of the real FL window table of the Mixer drag scene): MAIN popup (-5,-6,1930x1092) > WP (child 378,88,1546x998) > { RACK (0,0,499x126), PLAY (499,0,1047x998), MIX (16,466,1514x500) > { CAP (4,4,1506x23),
 * PANA (105,28,1167x465), PANB (1287,28,220x465) } }. Styles of FL: WP / RACK / PLAY / MIX = WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS (0x54000000), CAP / PANA / PANB = those + WS_CLIPCHILDREN (0x56000000), MAIN = WS_POPUP | WS_VISIBLE.
 * Every window procedure answers WM_ERASEBKGND with 1 and paints nothing (BeginPaint / EndPaint), recording the PAINTSTRUCT rcPaint (the clip box of the paint DC is not modelled by the runtime: GetClipBox on a screen DC stops, so it is not recorded on either side). After each move the update rectangle of every window is printed (GetUpdateRect)
 * before the paints are dispatched, then the paints. */
#define NWIN 10
static HWND g_w[NWIN];
static const char *g_n[NWIN] = { "MAIN", "WP", "RACK", "PLAY", "MIX", "CAP", "PANA", "PANB", "HID", "SUB" };
static const char *nm2_(HWND h) { for (int k = 0; k < NWIN; k++) if (g_w[k] == h) return g_n[k]; return "?"; }
static void paint_cb(HWND h, int32_t rc[4]) { printf("    PAINT %s rcPaint=(%d,%d,%d,%d) %dx%d\n", nm2_(h), rc[0], rc[1], rc[2], rc[3], rc[2] - rc[0], rc[3] - rc[1]); }
static void step(const char *s) { printf("%s:\n", s); }
static void screen_size(void)
{
#ifdef API2_WINE
    printf("screen (SM_CXSCREEN x SM_CYSCREEN): %d x %d\n", GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN));
#else
    printf("screen (SM_CXSCREEN x SM_CYSCREEN): %d x %d\n", (int)fle_api2_call(fle_imp_GetSystemMetrics, 0, 0, 0, 0, 0, 0, 0, 0), (int)fle_api2_call(fle_imp_GetSystemMetrics, 1, 0, 0, 0, 0, 0, 0, 0));
#endif
}
#ifndef API2_WINE
static BOOL ShowWindow_(HWND h, int c) { return (BOOL)fle_api2_call(fle_imp_ShowWindow, h, (uint32_t)c, 0, 0, 0, 0, 0, 0); }
static BOOL SetWindowPos_(HWND h, HWND after, int x, int y, int cx, int cy, UINT f) { return (BOOL)fle_api2_call(fle_imp_SetWindowPos, h, after, (uint32_t)x, (uint32_t)y, (uint32_t)cx, (uint32_t)cy, f, 0); }
static int uprect_(HWND h, int32_t r[4]) { return (int)(fle_api2_call(fle_imp_GetUpdateRect, h, A(r), 0, 0, 0, 0, 0, 0) & 1); }
#else
#define SetWindowPos_(h, a, x, y, cx, cy, f) SetWindowPos(h, a, x, y, cx, cy, f)
#define ShowWindow_(h, c) ShowWindow(h, c)
static int uprect_(HWND h, int32_t r[4]) { RECT q; BOOL b = GetUpdateRect(h, &q, FALSE); r[0] = q.left; r[1] = q.top; r[2] = q.right; r[3] = q.bottom; return b ? 1 : 0; }
#endif
static void report_updates(void)
{
    for (int k = 0; k < NWIN; k++) {
        int32_t r[4] = { 0, 0, 0, 0 };
        int has = uprect_(g_w[k], r);
        if (has) printf("    UPDATE %s rect=(%d,%d,%d,%d) %dx%d\n", g_n[k], r[0], r[1], r[2], r[3], r[2] - r[0], r[3] - r[1]);
    }
}
#define CH 0x54000000u
#define CHC 0x56000000u
int main(int argc, char **argv)
{
    setvbuf(stdout, NULL, _IONBF, 0);
    int top = argc > 1 && !strcmp(argv[1], "top");
    regclass();
    screen_size();
    printf("configuration: %s\n", top ? "MIX brought to the top of its siblings (SetWindowPos HWND_TOP) before the moves, the z-order of FL: the Mixer above the Playlist" : "z-order as created (a child created later is below the siblings created before)");
#ifndef API2_WINE
    if (!fle_raster_window_surface(1, 1930, 1092)) printf("no surface\n");
#endif
    g_w[0] = mk(WS_POPUP | WS_VISIBLE, -5, -6, 1930, 1092, 0);
    g_w[1] = mk(CH, 378, 88, 1546, 998, g_w[0]);
    g_w[2] = mk(CH, 0, 0, 499, 126, g_w[1]);
    g_w[3] = mk(CH, 499, 0, 1047, 998, g_w[1]);
    g_w[4] = mk(CH, 16, 466, 1514, 500, g_w[1]);
    g_w[5] = mk(CHC, 4, 4, 1506, 23, g_w[4]);
    g_w[6] = mk(CHC, 105, 28, 1167, 465, g_w[4]);
    g_w[7] = mk(CHC, 1287, 28, 220, 465, g_w[4]);
    g_w[9] = mk(CHC, 0, 32, 220, 191, g_w[7]);   /* a visible child of PANB: lid 84 of FL */
    g_w[8] = mk(0x46000000u, 0, 446, 1211, 19, g_w[6]);   /* a hidden child of PANA: lid 87 (TQuickScrollBoxScroller) of FL */
    step("initial paints (the windows are shown)");
    pump();
    if (top) { SetWindowPos_(g_w[4], 0, 0, 0, 0, 0, 0x13); step("MIX to the top, then its paints"); pump(); }
    printf("  (after the initial paints, update rectangles:)\n"); report_updates();
    step("HID: SetWindowPos(SWP_SHOWWINDOW | NOMOVE | NOSIZE | NOZORDER | NOACTIVATE), then SetWindowPos(0, 446, 1254 x 19, NOZORDER | NOACTIVATE), as FL does while it builds the Mixer");
    SetWindowPos_(g_w[8], 0, 0, 0, 0, 0, 0x57);
    printf("  update rectangles after SWP_SHOWWINDOW alone:\n"); report_updates();
    SetWindowPos_(g_w[8], 0, 0, 446, 1254, 19, 0x114);
    printf("  update rectangles after the size change (HID is visible, no paint yet):\n"); report_updates();
    step("HID: SetWindowPos(SWP_HIDEWINDOW | NOMOVE | NOSIZE | NOZORDER | NOACTIVATE) before any paint");
    SetWindowPos_(g_w[8], 0, 0, 0, 0, 0, 0x97);
    printf("  update rectangles right after the hide:\n"); report_updates();
    printf("  paints (pump):\n"); pump();
    step("HID: ShowWindow(SW_SHOWNA) and ShowWindow(SW_HIDE) without a paint in between");
    ShowWindow_(g_w[8], 8);
    printf("  update rectangles after SW_SHOWNA:\n"); report_updates();
    ShowWindow_(g_w[8], 0);
    printf("  update rectangles after SW_HIDE:\n"); report_updates();
    printf("  paints (pump):\n"); pump();
    step("SUB: PANB and its child SUB are both invalidated (InvalidateRect(NULL)), then PANB is hidden with ShowWindow(SW_HIDE)");
    InvalidateRect(g_w[9], NULL, 0);
    InvalidateRect(g_w[7], NULL, 0);
    printf("  update rectangles after the two InvalidateRect:\n"); report_updates();
    ShowWindow_(g_w[7], 0);
    printf("  update rectangles after ShowWindow(PANB, SW_HIDE):\n"); report_updates();
    printf("  paints (pump):\n"); pump();
    ShowWindow_(g_w[7], 8);
    printf("  update rectangles after ShowWindow(PANB, SW_SHOWNA):\n"); report_updates();
    printf("  paints (pump):\n"); pump();
    for (int m = 0; m < 3; m++) {
        char t[96];
        snprintf(t, sizeof t, "move %d: SetWindowPos(MIX, x = %d, y = 466, 1514 x 500, SWP_NOZORDER | SWP_NOACTIVATE)", m + 1, 31 + 15 * m);
        step(t);
        SetWindowPos_(g_w[4], 0, 31 + 15 * m, 466, 1514, 500, 0x14);
        printf("  update rectangles right after the call:\n"); report_updates();
        printf("  paints (pump):\n"); pump();
        printf("  update rectangles after the paints:\n"); report_updates();
    }
    step("move 4: the same position again (SetWindowPos with the rectangle the window already has)");
    SetWindowPos_(g_w[4], 0, 31 + 15 * 2, 466, 1514, 500, 0x14);
    printf("  update rectangles right after the call:\n"); report_updates();
    pump();
    step("size change: SetWindowPos(MIX, 1514 x 480, NOMOVE | NOZORDER | NOACTIVATE)");
    SetWindowPos_(g_w[4], 0, 0, 0, 1514, 480, 0x16);
    printf("  update rectangles right after the call:\n"); report_updates();
    printf("  paints (pump):\n"); pump();
    return 0;
}

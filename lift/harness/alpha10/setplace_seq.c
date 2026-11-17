/* Decision 0267 (alpha10, test tool; the scaffold of harness/alpha7/show_seq.c). SetWindowPlacement (the call FL's forms make with SW_HIDE and the rectangle of the window when they are created, and again when a form is shown): what it does to the state, the rectangle and the messages of a top-level window. Wine 10.0 (UNOFFICIAL) and the runtime run the same program. The next comment is the one of the scaffold: */
/* Decision 0258 (alpha7, test tool; the scaffold of harness/api2/win_seq.c): the message sequence and the state of ShowWindow(SW_SHOWMAXIMIZED / SW_RESTORE / SW_SHOWNORMAL / SW_MINIMIZE) on a top-level pop-up window, run (a) against Wine 10.0's user32 (API2_WINE: mingw build, headless Wine; UNOFFICIAL: Wine is a reference, not Windows) and (b) against the runtime (the stub window procedure calls DefWindowProcW). Same lines out of both; build: tools/build-showseq.sh. Prints the window procedure's messages (the window-management protocol) and the state after each step. */
/* Decision 0210 (api2, opt-in test tool): the message-sequence cases (WM_* order of window creation, move, size, show, hide, destroy), run (a) against Wine 10.0's user32 (API2_WINE: mingw build, headless Wine) and (b) against the REAL runtime window layer
 * (fle_win.c, fle_class.c, fle_input.c ... with a stub guest window procedure that calls DefWindowProcW, the scaffold of harness/web/g3/g3_scenarios.c). Same lines out of both; the lines on which the runtime differs are listed in
 * expected/win-known-divergence.txt (each is a gap of the runtime's window model, not a choice). */
#include <stdio.h>
#include <string.h>
#include <stdint.h>

#ifdef API2_WINE
#include <windows.h>
static const char *nm_(HWND h);
static void logmsg(HWND h, UINT m, WPARAM w, LPARAM l);
static LRESULT CALLBACK wp(HWND h, UINT m, WPARAM w, LPARAM l) { logmsg(h, m, w, l); return DefWindowProcW(h, m, w, l); }
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
#include "fle_input.h"
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
static void logmsg(HWND h, unsigned m, uint64_t w, uint64_t l);
static void wndproc(void)
{
    uint64_t hw = R[1], msg = R[2], wp_ = R[8], lp = R[9];
    logmsg(hw, (unsigned)msg, wp_, lp);
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



static HWND g_top, g_two;
static const char *nm_(HWND h) { return h == g_top ? "top" : h == g_two ? "two" : "?"; }
#ifdef API2_WINE
static void logmsg(HWND h, UINT m, WPARAM w, LPARAM l)
#else
static void logmsg(HWND h, unsigned m, uint64_t w, uint64_t l)
#endif
{
    switch (m) {
    case 0x0112: printf("  %s WM_SYSCOMMAND cmd=%#x lp=%lld\n", nm_(h), (unsigned)(w & 0xFFF0u), (long long)(int64_t)l); break;
    case 0x0083: printf("  %s WM_NCCALCSIZE wp=%d\n", nm_(h), (int)w); break;
    case 0x0046: case 0x0047: { int32_t *p = (int32_t *)(uintptr_t)l; printf("  %s %s x=%d y=%d cx=%d cy=%d flags=%#x\n", nm_(h), m == 0x46 ? "WM_WINDOWPOSCHANGING" : "WM_WINDOWPOSCHANGED", p[4], p[5], p[6], p[7], (unsigned)p[8]); break; }
    case 0x0003: printf("  %s WM_MOVE x=%d y=%d\n", nm_(h), (int)(int16_t)(l & 0xFFFF), (int)(int16_t)((l >> 16) & 0xFFFF)); break;
    case 0x0005: printf("  %s WM_SIZE wp=%d cx=%d cy=%d\n", nm_(h), (int)w, (int)(l & 0xFFFF), (int)((l >> 16) & 0xFFFF)); break;
    case 0x0018: printf("  %s WM_SHOWWINDOW wp=%d lp=%d\n", nm_(h), (int)w, (int)l); break;
    case 0x0002: printf("  %s WM_DESTROY\n", nm_(h)); break;
    case 0x0010: printf("  %s WM_CLOSE\n", nm_(h)); break;
    case 0x0006: printf("  %s WM_ACTIVATE wp=%d\n", nm_(h), (int)w); break;
    case 0x001c: printf("  %s WM_ACTIVATEAPP wp=%d\n", nm_(h), (int)w); break;
    case 0x0086: printf("  %s WM_NCACTIVATE wp=%d\n", nm_(h), (int)w); break;
    case 0x0024: { int32_t *p = (int32_t *)(uintptr_t)l; printf("  %s WM_GETMINMAXINFO maxsize=%d,%d maxpos=%d,%d\n", nm_(h), p[2], p[3], p[4], p[5]); break; }
    case 0x0007: printf("  %s WM_SETFOCUS\n", nm_(h)); break;
    case 0x0008: printf("  %s WM_KILLFOCUS\n", nm_(h)); break;
    default: break;
    }
}
static void step(const char *s) { printf("%s:\n", s); }
#define SC_MINIMIZE 0xF020u
#define SC_MAXIMIZE 0xF030u
#define SC_RESTORE 0xF120u
#define SC_CLOSE 0xF060u
#define WM_SYSCOMMAND_ 0x0112u
#ifndef API2_WINE
static int setpl_(HWND h, uint32_t len, uint32_t flags, uint32_t show, int minx, int miny, int maxx, int maxy, int l, int t, int r, int b) { uint32_t w[11] = { len, flags, show, (uint32_t)minx, (uint32_t)miny, (uint32_t)maxx, (uint32_t)maxy, (uint32_t)l, (uint32_t)t, (uint32_t)r, (uint32_t)b }; return (int)(fle_api2_call(fle_imp_SetWindowPlacement, h, A(w), 0, 0, 0, 0, 0, 0) & 1); }
static BOOL ShowWindow_(HWND h, int c) { return (BOOL)fle_api2_call(fle_imp_ShowWindow, h, (uint32_t)c, 0, 0, 0, 0, 0, 0); }
static void rect_(HWND h, int32_t r[4]) { (void)fle_api2_call(fle_imp_GetWindowRect, h, A(r), 0, 0, 0, 0, 0, 0); }
static void crect_(HWND h, int32_t r[4]) { (void)fle_api2_call(fle_imp_GetClientRect, h, A(r), 0, 0, 0, 0, 0, 0); }
static int zoomed_(HWND h) { return (int)(fle_api2_call(fle_imp_IsZoomed, h, 0, 0, 0, 0, 0, 0, 0) & 1); }
static int iconic_(HWND h) { return (int)(fle_api2_call(fle_imp_IsIconic, h, 0, 0, 0, 0, 0, 0, 0) & 1); }
static int visible_(HWND h) { return (int)(fle_api2_call(fle_imp_IsWindowVisible, h, 0, 0, 0, 0, 0, 0, 0) & 1); }
static uint64_t send_(HWND h, unsigned m, uint64_t w, uint64_t l) { return fle_api2_call(fle_imp_SendMessageW, h, m, w, l, 0, 0, 0, 0); }
static int fg_is_(HWND h) { return fle_api2_call(fle_imp_GetForegroundWindow, 0, 0, 0, 0, 0, 0, 0, 0) == h; }
static void place_(HWND h, int32_t o[11]) { uint32_t b[11]; memset(b, 0, sizeof b); b[0] = 44; (void)fle_api2_call(fle_imp_GetWindowPlacement, h, A(b), 0, 0, 0, 0, 0, 0); memcpy(o, b, sizeof b); }
void fle_imp_GetSystemMetrics(void);
static int sm_(int i) { return (int)fle_api2_call(fle_imp_GetSystemMetrics, (uint32_t)i, 0, 0, 0, 0, 0, 0, 0); }
#else
static int setpl_(HWND h, uint32_t len, uint32_t flags, uint32_t show, int minx, int miny, int maxx, int maxy, int l, int t, int r, int b) { WINDOWPLACEMENT p; memset(&p, 0, sizeof p); p.length = len; p.flags = flags; p.showCmd = show; p.ptMinPosition.x = minx; p.ptMinPosition.y = miny; p.ptMaxPosition.x = maxx; p.ptMaxPosition.y = maxy; p.rcNormalPosition.left = l; p.rcNormalPosition.top = t; p.rcNormalPosition.right = r; p.rcNormalPosition.bottom = b; SetLastError(0); return SetWindowPlacement(h, &p) ? 1 : 0; }
#define ShowWindow_(h, c) ShowWindow(h, c)
static void rect_(HWND h, int32_t r[4]) { RECT q; GetWindowRect(h, &q); r[0] = q.left; r[1] = q.top; r[2] = q.right; r[3] = q.bottom; }
static void crect_(HWND h, int32_t r[4]) { RECT q; GetClientRect(h, &q); r[0] = q.left; r[1] = q.top; r[2] = q.right; r[3] = q.bottom; }
static int zoomed_(HWND h) { return IsZoomed(h) ? 1 : 0; }
static int iconic_(HWND h) { return IsIconic(h) ? 1 : 0; }
static int visible_(HWND h) { return IsWindowVisible(h) ? 1 : 0; }
static uint64_t send_(HWND h, unsigned m, uint64_t w, uint64_t l) { return (uint64_t)SendMessageW(h, m, (WPARAM)w, (LPARAM)l); }
static int fg_is_(HWND h) { return GetForegroundWindow() == h; }
static void place_(HWND h, int32_t o[11]) { WINDOWPLACEMENT p; memset(&p, 0, sizeof p); p.length = sizeof p; GetWindowPlacement(h, &p); o[0] = (int32_t)p.length; o[1] = (int32_t)p.flags; o[2] = (int32_t)p.showCmd; o[3] = p.ptMinPosition.x; o[4] = p.ptMinPosition.y; o[5] = p.ptMaxPosition.x; o[6] = p.ptMaxPosition.y; o[7] = p.rcNormalPosition.left; o[8] = p.rcNormalPosition.top; o[9] = p.rcNormalPosition.right; o[10] = p.rcNormalPosition.bottom; }
static int sm_(int i) { return GetSystemMetrics(i); }
#endif
static void state(HWND h)
{
    int32_t r[4], c[4], p[11];
    int sw = sm_(0), sh = sm_(1);

    rect_(h, r); crect_(h, c); place_(h, p);
    printf("  state: visible=%d zoomed=%d iconic=%d window=(%d,%d,%d,%d)%s client=%dx%d placement: flags=%d showCmd=%d minpos=(%d,%d) maxpos=(%d,%d) normal=(%d,%d,%d,%d)\n", visible_(h), zoomed_(h), iconic_(h), r[0], r[1], r[2], r[3],
           (r[0] == 0 && r[1] == 0 && r[2] == sw && r[3] == sh) ? " [= screen]" : "", c[2] - c[0], c[3] - c[1], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8], p[9], p[10]);
}
static void cmd(const char *what, HWND h, unsigned sc)
{
    char t[120];

    snprintf(t, sizeof t, "SendMessage(%s, WM_SYSCOMMAND, %s)", nm_(h), what);
    step(t);
    (void)send_(h, WM_SYSCOMMAND_, sc, 0);
    pump();
    state(h);
}
#define SW_SHOWNORMAL 1
#define SW_MINIMIZE 6
#define SW_RESTORE 9
#define SW_SHOWMAXIMIZED 3


static void setpl(const char *what, HWND h, uint32_t flags, uint32_t show, int l, int t, int r, int b)
{
    char tt[200];
    int ok;

    snprintf(tt, sizeof tt, "SetWindowPlacement(%s, %s): showCmd=%u flags=%u normal=(%d,%d,%d,%d)", nm_(h), what, show, flags, l, t, r, b);
    step(tt);
    ok = setpl_(h, 44, flags, show, -1, -1, -1, -1, l, t, r, b);
    printf("  returned %d\n", ok);
    pump();
    state(h);
}
int main(void)
{
    int ok;

    setvbuf(stdout, NULL, _IONBF, 0);
    regclass();
    printf("screen %dx%d\n", sm_(0), sm_(1));
    step("create a HIDDEN top-level popup WS_POPUP | MINIMIZEBOX | MAXIMIZEBOX 50,50 200x100 (the state of FL's main form when it makes its SetWindowPlacement)");
    g_top = mk(WS_POPUP | 0x30000u, 50, 50, 200, 100, 0);
    pump();
    state(g_top);
    setpl("SW_HIDE, the rectangle of the form", g_top, 0, 0, -5, -6, 1925, 1086);
    step("ShowWindow(top, SW_SHOWNORMAL) after that");
    ShowWindow_(g_top, 1); pump(); state(g_top);
    step("create a visible top-level popup 100,100 300x200");
    g_two = mk(WS_POPUP | WS_VISIBLE | 0x30000u, 100, 100, 300, 200, 0);
    pump();
    state(g_two);
    setpl("SW_SHOWNORMAL, a new rectangle", g_two, 0, 1, 10, 20, 310, 220);
    setpl("SW_SHOWMAXIMIZED, a new rectangle", g_two, 0, 3, 30, 40, 330, 240);
    setpl("SW_SHOWNORMAL of the maximized window", g_two, 0, 1, 50, 60, 350, 260);
    setpl("SW_SHOWMINIMIZED", g_two, 0, 2, 50, 60, 350, 260);
    setpl("SW_SHOWNORMAL of the minimized window", g_two, 0, 1, 70, 80, 370, 280);
    setpl("SW_SHOWMINIMIZED with WPF_RESTORETOMAXIMIZED", g_two, 2, 2, 70, 80, 370, 280);
    setpl("SW_SHOWNORMAL of that minimized window (WPF_RESTORETOMAXIMIZED is not set)", g_two, 0, 1, 70, 80, 370, 280);
    setpl("SW_HIDE of the visible window", g_two, 0, 0, 70, 80, 370, 280);
    {
        HWND h3, h4, h5;

        step("create three HIDDEN top-level popups 60,60 100x80 and give each a SetWindowPlacement with SW_SHOWNORMAL / SW_SHOWMAXIMIZED / SW_SHOWMINIMIZED");
        h3 = mk(WS_POPUP | 0x30000u, 60, 60, 100, 80, 0);
        h4 = mk(WS_POPUP | 0x30000u, 60, 60, 100, 80, 0);
        h5 = mk(WS_POPUP | 0x30000u, 60, 60, 100, 80, 0);
        pump();
        g_top = h3; setpl("a hidden window, SW_SHOWNORMAL", h3, 0, 1, 5, 6, 205, 106);
        g_top = h4; setpl("a hidden window, SW_SHOWMAXIMIZED", h4, 0, 3, 5, 6, 205, 106);
        g_top = h5; setpl("a hidden window, SW_SHOWMINIMIZED", h5, 0, 2, 5, 6, 205, 106);
        g_top = h3; setpl("SW_SHOWNOACTIVATE (4) of a hidden window", mk(WS_POPUP | 0x30000u, 60, 60, 100, 80, 0), 0, 4, 5, 6, 205, 106);
    }
    step("SetWindowPlacement with a wrong length (60)");
    ok = setpl_(g_two, 60, 0, 1, -1, -1, -1, -1, 0, 0, 100, 100);
    printf("  returned %d\n", ok);
    return 0;
}

/* Decision 0222 (win2, opt-in test tool): z-order, activation, mouse activation, focus and close of top-level, owned and child windows, run (a) against Wine 10.0's user32 (API2_WINE: mingw build, headless Wine, the null
 * display driver) and (b) against the REAL runtime window layer (fle_win.c, fle_input.c ... with a stub guest window procedure that calls DefWindowProcW; the scaffold of win_seq.c). Same lines out of both; the lines on which
 * the runtime differs are expected/win-act-known-divergence.txt. Wine is the reference, not the truth: a difference is judged against the Learn page (docs/0222-window-manager-fidelity.md).
 * Every line is a message of the window-management protocol ("  NAME MESSAGE args"), or the state after a step ("= z: ..."). Coordinates are inside 0..700 x 0..500 (the smallest screen of the two sides). */
#include <stdio.h>
#include <string.h>
#include <stdint.h>

#ifdef API2_WINE
#include <windows.h>
static LRESULT CALLBACK wp(HWND h, UINT m, WPARAM w, LPARAM l);
static void pump(void) { MSG msg; while (PeekMessageW(&msg, NULL, 0, 0, PM_REMOVE)) DispatchMessageW(&msg); }
static HWND mk(DWORD style, DWORD ex, int x, int y, int cx, int cy, HWND parent) { return CreateWindowExW(ex, L"act", L"", style, x, y, cx, cy, parent, NULL, GetModuleHandleW(NULL), NULL); }
static void regclass(void) { WNDCLASSW wc; memset(&wc, 0, sizeof wc); wc.style = CS_DBLCLKS; wc.lpfnWndProc = wp; wc.hInstance = GetModuleHandleW(NULL); wc.lpszClassName = L"act"; wc.hbrBackground = (HBRUSH)(COLOR_WINDOW + 1); RegisterClassW(&wc); }
static void mouse(int kind, int x, int y)
{
    INPUT in;
    memset(&in, 0, sizeof in);
    in.type = INPUT_MOUSE;
    in.mi.dx = (LONG)((x * 65535L) / (GetSystemMetrics(SM_CXSCREEN) - 1));
    in.mi.dy = (LONG)((y * 65535L) / (GetSystemMetrics(SM_CYSCREEN) - 1));
    in.mi.dwFlags = MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_MOVE | (kind == 2 ? MOUSEEVENTF_LEFTDOWN : kind == 3 ? MOUSEEVENTF_LEFTUP : 0);
    SendInput(1, &in, sizeof in);
}
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
#include "fle_input.h"
#include "fle_wall_ia2.h"
#include "fle_api2.h"
uint64_t R[16];
fle_eflags FL;
fle_xmm X[16];
void fle_imp_PeekMessageW(void);
void fle_imp_DispatchMessageW(void);
typedef uint64_t HWND;
typedef uint64_t WPARAM;
typedef uint64_t LPARAM;
typedef unsigned int DWORD, UINT;
typedef int BOOL;
#define A(x) ((uint64_t)(uintptr_t)(x))
#define VA_WP 0x2000
#define WS_POPUP 0x80000000u
#define WS_CHILD 0x40000000u
#define WS_VISIBLE 0x10000000u
#define WS_DISABLED 0x08000000u
#define WS_CLIPSIBLINGS 0x04000000u
#define WS_CLIPCHILDREN 0x02000000u
#define WS_EX_NOACTIVATE 0x08000000u
#define WS_EX_TOPMOST 8u
#define SWP_NOSIZE 1u
#define SWP_NOMOVE 2u
#define SWP_NOZORDER 4u
#define SWP_NOACTIVATE 0x10u
#define SWP_HIDEWINDOW 0x80u
#define SWP_FRAMECHANGED 0x20u
#define SWP_SHOWWINDOW 0x40u
#define SW_HIDE 0
#define SW_SHOWNORMAL 1
#define SW_SHOWMINIMIZED 2
#define SW_SHOWMAXIMIZED 3
#define SW_MINIMIZE 6
#define SW_SHOWMINNOACTIVE 7
#define SW_SHOWNA 8
#define SW_SHOWNOACTIVATE 4
#define SW_SHOW 5
#define SW_RESTORE 9
#define HWND_TOP ((HWND)0)
#define HWND_BOTTOM ((HWND)1)
#define HWND_TOPMOST ((HWND)-1)
#define HWND_NOTOPMOST ((HWND)-2)
#define GW_HWNDNEXT 2
#define WM_SYSCOMMAND 0x0112
#define SC_CLOSE 0xF060
static void wndproc(void);
const fle_fn_entry fle_dispatch_table[] = { { VA_WP, wndproc } };
const uint64_t fle_dispatch_count = 1;
static uint64_t stk[0x4000] __attribute__((aligned(4096)));
static uint16_t clsname[] = { 'a', 'c', 't', 0 };
static void regclass(void)
{
    static uint8_t wc[72];
    uint64_t proc = VA_WP, inst = 0x400000, nmp = A(clsname), cur = FLE_CURSOR_HANDLE_BASE + 4u;
    uint32_t style = 0x8u;   /* CS_DBLCLKS */
    R[4] = (uint64_t)(uintptr_t)stk + 0x10000 + 8;
    fle_st(R[4], 8, 0xdead0000);
    fle_teb_set_stack(A(stk), A(stk) + sizeof stk);
    memset(wc, 0, sizeof wc);
    memcpy(wc, &style, 4); memcpy(wc + 8, &proc, 8); memcpy(wc + 24, &inst, 8); memcpy(wc + 40, &cur, 8); memcpy(wc + 64, &nmp, 8);
    (void)fle_api2_call(fle_imp_RegisterClassW, A(wc), 0, 0, 0, 0, 0, 0, 0);
}
static HWND mk(DWORD style, DWORD ex, int x, int y, int cx, int cy, HWND parent)
{
    uint64_t h, sp0 = R[4], sv[16];
    uint16_t empty[1] = { 0 };
    memcpy(sv, R, sizeof sv);
    R[4] = (R[4] - 0x80) & ~UINT64_C(15);
    R[4] -= 8;
    fle_st(R[4], 8, 0xAAAA0000);
    fle_st(R[4] + 0x28, 8, (uint32_t)x); fle_st(R[4] + 0x30, 8, (uint32_t)y); fle_st(R[4] + 0x38, 8, (uint32_t)cx); fle_st(R[4] + 0x40, 8, (uint32_t)cy);
    fle_st(R[4] + 0x48, 8, parent); fle_st(R[4] + 0x50, 8, 0); fle_st(R[4] + 0x58, 8, 0x400000); fle_st(R[4] + 0x60, 8, 0);
    R[1] = ex; R[2] = A(clsname); R[8] = A(empty); R[9] = style;
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
static void mouse(int kind, int x, int y) { (void)fle_input_host_mouse(kind == 2 ? FLE_IN_BDOWN : kind == 3 ? FLE_IN_BUP : FLE_IN_MOVE, x, y, FLE_IN_BTN_L); }
#define API(name, ...) fle_api2_call(fle_imp_##name, __VA_ARGS__)
static BOOL ShowWindow(HWND h, int c) { return (BOOL)API(ShowWindow, h, (uint32_t)c, 0, 0, 0, 0, 0, 0); }
static BOOL SetWindowPos(HWND h, HWND a, int x, int y, int cx, int cy, UINT f) { return (BOOL)API(SetWindowPos, h, a, (uint32_t)x, (uint32_t)y, (uint32_t)cx, (uint32_t)cy, f, 0); }
static BOOL BringWindowToTop(HWND h) { return (BOOL)API(BringWindowToTop, h, 0, 0, 0, 0, 0, 0, 0); }
static HWND SetActiveWindow(HWND h) { return API(SetActiveWindow, h, 0, 0, 0, 0, 0, 0, 0); }
static BOOL SetForegroundWindow(HWND h) { return (BOOL)API(SetForegroundWindow, h, 0, 0, 0, 0, 0, 0, 0); }
static HWND SetFocus(HWND h) { return API(SetFocus, h, 0, 0, 0, 0, 0, 0, 0); }
static HWND GetActiveWindow(void) { return API(GetActiveWindow, 0, 0, 0, 0, 0, 0, 0, 0); }
static HWND GetForegroundWindow(void) { return API(GetForegroundWindow, 0, 0, 0, 0, 0, 0, 0, 0); }
static HWND GetFocus(void) { return API(GetFocus, 0, 0, 0, 0, 0, 0, 0, 0); }
static HWND GetTopWindow(HWND h) { return API(GetTopWindow, h, 0, 0, 0, 0, 0, 0, 0); }
static HWND GetWindow(HWND h, UINT c) { return API(GetWindow, h, c, 0, 0, 0, 0, 0, 0); }
static BOOL IsWindowVisible(HWND h) { return (BOOL)API(IsWindowVisible, h, 0, 0, 0, 0, 0, 0, 0); }
static BOOL IsWindow(HWND h) { return (BOOL)API(IsWindow, h, 0, 0, 0, 0, 0, 0, 0); }
static BOOL DestroyWindow(HWND h) { return (BOOL)API(DestroyWindow, h, 0, 0, 0, 0, 0, 0, 0); }
static BOOL EnableWindow(HWND h, BOOL e) { return (BOOL)API(EnableWindow, h, (uint32_t)e, 0, 0, 0, 0, 0, 0); }
static uint64_t SendMessageW(HWND h, UINT m, WPARAM w, LPARAM l) { return API(SendMessageW, h, m, w, l, 0, 0, 0, 0); }
static BOOL PostMessageW(HWND h, UINT m, WPARAM w, LPARAM l) { return (BOOL)API(PostMessageW, h, m, w, l, 0, 0, 0, 0); }
static HWND SetCapture(HWND h) { return API(SetCapture, h, 0, 0, 0, 0, 0, 0, 0); }
static BOOL ReleaseCapture(void) { return (BOOL)API(ReleaseCapture, 0, 0, 0, 0, 0, 0, 0, 0); }
static HWND GetCapture(void) { return API(GetCapture, 0, 0, 0, 0, 0, 0, 0, 0); }
#endif

#define NW 12
static HWND g_h[NW];
static const char *g_n[NW];
static const char *nm_(HWND h)
{
    int k;
    for (k = 0; k < NW; k++) if (g_h[k] && g_h[k] == h) return g_n[k];
    return h ? "?" : "none";
}
static HWND def(HWND h, UINT m, WPARAM w, LPARAM l);
#ifdef API2_WINE
static LRESULT CALLBACK wp(HWND h, UINT m, WPARAM w, LPARAM l);
static void logmsg(HWND h, UINT m, WPARAM w, LPARAM l);
static LRESULT CALLBACK wp(HWND h, UINT m, WPARAM w, LPARAM l) { logmsg(h, m, w, l); return DefWindowProcW(h, m, w, l); }
static HWND def(HWND h, UINT m, WPARAM w, LPARAM l) { return (HWND)DefWindowProcW(h, m, w, l); }
static void logmsg(HWND h, UINT m, WPARAM w, LPARAM l)
#else
static void logmsg(HWND h, unsigned m, uint64_t w, uint64_t l);
static void wndproc(void)
{
    uint64_t hw = R[1], msg = R[2], wp_ = R[8], lp = R[9];
    logmsg(hw, (unsigned)msg, wp_, lp);
    R[0] = fle_api2_call(fle_imp_DefWindowProcW, hw, msg, wp_, lp, 0, 0, 0, 0);
}
static void logmsg(HWND h, unsigned m, uint64_t w, uint64_t l)
#endif
{
    const char *n = nm_(h);
    switch (m) {
    case 0x0001: printf("  %s WM_CREATE\n", n); break;
    case 0x0002: printf("  %s WM_DESTROY\n", n); break;
    case 0x0082: printf("  %s WM_NCDESTROY\n", n); break;
    case 0x0010: printf("  %s WM_CLOSE\n", n); break;
    case 0x0006: printf("  %s WM_ACTIVATE wp=%d other=%s\n", n, (int)(w & 0xFFFF), nm_((HWND)l)); break;
    case 0x001c: printf("  %s WM_ACTIVATEAPP wp=%d\n", n, (int)w); break;
    case 0x0086: printf("  %s WM_NCACTIVATE wp=%d\n", n, (int)w); break;
    case 0x0007: printf("  %s WM_SETFOCUS old=%s\n", n, nm_((HWND)w)); break;
    case 0x0008: printf("  %s WM_KILLFOCUS new=%s\n", n, nm_((HWND)w)); break;
    case 0x0018: printf("  %s WM_SHOWWINDOW wp=%d lp=%d\n", n, (int)w, (int)l); break;
    case 0x0046: case 0x0047: { int32_t *p = (int32_t *)(uintptr_t)l; printf("  %s %s flags=%#x\n", n, m == 0x46 ? "WM_WINDOWPOSCHANGING" : "WM_WINDOWPOSCHANGED", (unsigned)p[8] & ~0x1800u); break; }
    case 0x0084: printf("  %s WM_NCHITTEST\n", n); break;
    case 0x0020: printf("  %s WM_SETCURSOR hit=%d msg=%#x\n", n, (int)(int16_t)(l & 0xFFFF), (unsigned)((l >> 16) & 0xFFFF)); break;
    case 0x0021: printf("  %s WM_MOUSEACTIVATE top=%s hit=%d msg=%#x\n", n, nm_((HWND)w), (int)(int16_t)(l & 0xFFFF), (unsigned)((l >> 16) & 0xFFFF)); break;
    case 0x0201: printf("  %s WM_LBUTTONDOWN x=%d y=%d\n", n, (int)(int16_t)(l & 0xFFFF), (int)(int16_t)((l >> 16) & 0xFFFF)); break;
    case 0x0202: printf("  %s WM_LBUTTONUP x=%d y=%d\n", n, (int)(int16_t)(l & 0xFFFF), (int)(int16_t)((l >> 16) & 0xFFFF)); break;
    case 0x00A1: printf("  %s WM_NCLBUTTONDOWN ht=%d\n", n, (int)w); break;
    case 0x00A2: printf("  %s WM_NCLBUTTONUP ht=%d\n", n, (int)w); break;
    case 0x0210: printf("  %s WM_PARENTNOTIFY ev=%#x\n", n, (unsigned)(w & 0xFFFF)); break;
    case 0x0112: printf("  %s WM_SYSCOMMAND cmd=%#x\n", n, (unsigned)(w & 0xFFF0)); break;
    case 0x0215: printf("  %s WM_CAPTURECHANGED\n", n); break;
    case 0x000a: printf("  %s WM_ENABLE wp=%d\n", n, (int)w); break;
    default: break;
    }
}

static void step(const char *s) { printf("%s:\n", s); }

/* the state after a step: the z-order of the top-level windows and of the children of every window of the set, the active, foreground and focus windows, visibility */
static void state(void)
{
    int k;
    HWND t;
    int n = 0;
    printf("= z:");
    for (t = GetTopWindow(0); t && n < 200; t = GetWindow(t, GW_HWNDNEXT), n++) {
        const char *nm = nm_(t);
        if (nm[0] != '?') printf(" %s", nm);
    }
    printf("\n");
    for (k = 0; k < NW; k++) {
        if (g_h[k] && IsWindow(g_h[k]) && GetTopWindow(g_h[k])) {
            HWND c;
            printf("= z(%s):", g_n[k]);
            for (c = GetTopWindow(g_h[k]), n = 0; c && n < 50; c = GetWindow(c, GW_HWNDNEXT), n++) printf(" %s", nm_(c));
            printf("\n");
        }
    }
    printf("= active=%s foreground=%s focus=%s capture=%s\n", nm_(GetActiveWindow()), nm_(GetForegroundWindow()), nm_(GetFocus()), nm_(GetCapture()));
    printf("= vis:");
    for (k = 0; k < NW; k++) if (g_h[k]) printf(" %s=%d", g_n[k], IsWindow(g_h[k]) ? IsWindowVisible(g_h[k]) : -1);
    printf("\n");
}
static HWND add(int slot, const char *nm, DWORD style, DWORD ex, int x, int y, int cx, int cy, HWND parent)
{
    g_n[slot] = nm;
    g_h[slot] = mk(style, ex, x, y, cx, cy, parent);
    return g_h[slot];
}
static void clearall(void)
{
    int k;
    step("(cleanup)");
    for (k = NW - 1; k >= 0; k--) if (g_h[k] && IsWindow(g_h[k])) DestroyWindow(g_h[k]);
    pump();
    memset(g_h, 0, sizeof g_h);
}
static void click(int x, int y)
{
    mouse(1, x, y); pump();
    mouse(2, x, y); pump();
    mouse(3, x, y); pump();
}

int main(int argc, char **argv)
{
    int only = 0;
    if (argc > 1) { const char *a = argv[1]; while (*a >= '0' && *a <= '9') only = only * 10 + (*a++ - '0'); }
    setvbuf(stdout, NULL, _IONBF, 0);
    regclass();
#define SC(n, title) if (!only || only == (n)) { printf("#### scenario %d: %s\n", n, title); } if (!only || only == (n))
    SC(1, "two overlapping visible pop-ups: creation, activation, z-order") {
        HWND p1, p2;
        p1 = add(0, "P1", WS_POPUP | WS_VISIBLE, 0, 50, 50, 200, 150, 0);
        p2 = add(1, "P2", WS_POPUP | WS_VISIBLE, 0, 150, 100, 200, 150, 0); pump();
        (void)SetActiveWindow(p2); pump();
        step("initial (P2 created last, active)");
        state();
        step("SetActiveWindow(P1)");
        (void)SetActiveWindow(p1); pump(); state();
        step("SetForegroundWindow(P2)");
        (void)SetForegroundWindow(p2); pump(); state();
        step("BringWindowToTop(P1)");
        (void)BringWindowToTop(p1); pump(); state();
        step("SetWindowPos(P2, HWND_TOP, NOMOVE|NOSIZE|NOACTIVATE)");
        (void)SetWindowPos(p2, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE); pump(); state();
        step("SetWindowPos(P2, HWND_BOTTOM, NOMOVE|NOSIZE)");
        (void)SetWindowPos(p2, HWND_BOTTOM, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE); pump(); state();
        step("SetWindowPos(P2, HWND_TOP, NOMOVE|NOSIZE)  (activates)");
        (void)SetWindowPos(p2, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE); pump(); state();
        clearall();
    }
    SC(2, "click on a window that is not active: activation by the mouse") {
        HWND p1, p2;
        p1 = add(0, "P1", WS_POPUP | WS_VISIBLE, 0, 50, 50, 200, 150, 0);
        p2 = add(1, "P2", WS_POPUP | WS_VISIBLE, 0, 150, 100, 200, 150, 0); pump();
        (void)SetActiveWindow(p2); pump();
        step("initial");
        state();
        step("click on the part of P1 that P2 does not cover (80,80)");
        click(80, 80); state();
        step("click on P2 (200,200: only P2)");
        click(300, 220); state();
        step("click on P1 again at a point covered by P2 (200,150)");
        click(200, 150); state();
        (void)p1;
        clearall();
    }
    SC(3, "children of one top-level window: BringWindowToTop, SetWindowPos z-order, no activation of siblings") {
        HWND t, c1, c2, c3;
        t = add(0, "T", WS_POPUP | WS_VISIBLE | WS_CLIPCHILDREN, 0, 50, 50, 500, 300, 0);
        c1 = add(1, "C1", WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS, 0, 10, 10, 200, 150, t);
        c2 = add(2, "C2", WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS, 0, 100, 10, 200, 150, t);
        c3 = add(3, "C3", WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS, 0, 150, 100, 200, 150, t); pump();
        (void)SetActiveWindow(t); pump();
        step("initial (T active)");
        state();
        step("BringWindowToTop(C1)");
        (void)BringWindowToTop(c1); pump(); state();
        step("SetWindowPos(C2, HWND_TOP, NOMOVE|NOSIZE)");
        (void)SetWindowPos(c2, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE); pump(); state();
        step("SetWindowPos(C3, HWND_BOTTOM, NOMOVE|NOSIZE|NOACTIVATE)");
        (void)SetWindowPos(c3, HWND_BOTTOM, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE); pump(); state();
        step("SetWindowPos(C1, after C3, NOMOVE|NOSIZE)");
        (void)SetWindowPos(c1, c3, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE); pump(); state();
        step("ShowWindow(C2, SW_HIDE)");
        (void)ShowWindow(c2, SW_HIDE); pump(); state();
        step("ShowWindow(C2, SW_SHOW)");
        (void)ShowWindow(c2, SW_SHOW); pump(); state();
        clearall();
    }
    SC(4, "click on a child window of the active top-level window (the child is covered partly by a sibling)") {
        HWND t, c1, c2;
        t = add(0, "T", WS_POPUP | WS_VISIBLE | WS_CLIPCHILDREN, 0, 50, 50, 500, 300, 0);
        c1 = add(1, "C1", WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS, 0, 10, 10, 250, 200, t);
        c2 = add(2, "C2", WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS, 0, 150, 10, 250, 200, t); pump();
        (void)SetActiveWindow(t); pump();
        step("initial");
        state();
        step("click on C1 where C2 does not cover it (80,100)");
        click(80, 100); state();
        step("click on C1 where C2 covers it (250,100): the click goes to C2");
        click(250, 100); state();
        (void)c1; (void)c2;
        clearall();
    }
    SC(5, "click on a child of a top-level window that is not active") {
        HWND t1, t2, c1, c2;
        t1 = add(0, "T1", WS_POPUP | WS_VISIBLE | WS_CLIPCHILDREN, 0, 20, 20, 300, 250, 0);
        c1 = add(1, "C1", WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS, 0, 10, 10, 150, 100, t1);
        t2 = add(2, "T2", WS_POPUP | WS_VISIBLE | WS_CLIPCHILDREN, 0, 200, 100, 300, 250, 0);
        c2 = add(3, "C2", WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS, 0, 10, 10, 150, 100, t2); pump();
        (void)SetActiveWindow(t2); pump();
        step("initial (T2 active, above T1)");
        state();
        step("click on C1 (40,40): inside T1 and outside T2");
        click(40, 40); state();
        (void)c1; (void)c2; (void)t1;
        clearall();
    }
    SC(6, "close: SC_CLOSE sent to a top-level window, WM_CLOSE handled by DefWindowProc, and the activation that follows") {
        HWND p1, p2, p3;
        p1 = add(0, "P1", WS_POPUP | WS_VISIBLE, 0, 50, 50, 200, 150, 0);
        p2 = add(1, "P2", WS_POPUP | WS_VISIBLE, 0, 150, 100, 200, 150, 0);
        p3 = add(2, "P3", WS_POPUP | WS_VISIBLE, 0, 250, 150, 200, 150, 0); pump();
        (void)SetActiveWindow(p3); pump();
        step("initial (P3 active)");
        state();
        step("SendMessage(P3, WM_SYSCOMMAND, SC_CLOSE)");
        (void)SendMessageW(p3, WM_SYSCOMMAND, SC_CLOSE, 0); pump(); state();
        step("PostMessage(P2, WM_CLOSE) while P2 is not the active window");
        (void)PostMessageW(p2, 0x0010, 0, 0); pump(); state();
        (void)p1;
        clearall();
    }
    SC(7, "hide and destroy of the active window: which window becomes active") {
        HWND p1, p2, p3;
        p1 = add(0, "P1", WS_POPUP | WS_VISIBLE, 0, 50, 50, 200, 150, 0);
        p2 = add(1, "P2", WS_POPUP | WS_VISIBLE, 0, 150, 100, 200, 150, 0);
        p3 = add(2, "P3", WS_POPUP | WS_VISIBLE, 0, 250, 150, 200, 150, 0); pump();
        (void)SetActiveWindow(p3); pump();
        step("ShowWindow(P3, SW_HIDE)");
        (void)ShowWindow(p3, SW_HIDE); pump(); state();
        step("DestroyWindow(active window)");
        (void)DestroyWindow(GetActiveWindow()); pump(); state();
        (void)p1; (void)p2;
        clearall();
    }
    SC(8, "owned windows: z-order with the owner, activation of the owner, hiding the owner") {
        HWND o, w, other;
        o = add(0, "O", WS_POPUP | WS_VISIBLE, 0, 50, 50, 250, 200, 0);
        w = add(1, "W", WS_POPUP | WS_VISIBLE, 0, 120, 120, 250, 200, o);
        other = add(2, "X", WS_POPUP | WS_VISIBLE, 0, 200, 80, 250, 200, 0); pump();
        (void)SetActiveWindow(other); pump();
        step("initial (X created last, active)");
        state();
        step("SetActiveWindow(O)");
        (void)SetActiveWindow(o); pump(); state();
        step("BringWindowToTop(X)");
        (void)BringWindowToTop(other); pump(); state();
        step("SetWindowPos(O, HWND_TOP, NOMOVE|NOSIZE|NOACTIVATE)");
        (void)SetWindowPos(o, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE); pump(); state();
        step("SetWindowPos(W, HWND_BOTTOM, NOMOVE|NOSIZE|NOACTIVATE)");
        (void)SetWindowPos(w, HWND_BOTTOM, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE); pump(); state();
        step("ShowWindow(O, SW_HIDE)");
        (void)ShowWindow(o, SW_HIDE); pump(); state();
        step("ShowWindow(O, SW_SHOW)");
        (void)ShowWindow(o, SW_SHOW); pump(); state();
        clearall();
    }
    SC(9, "topmost windows") {
        HWND p1, p2, p3;
        p1 = add(0, "P1", WS_POPUP | WS_VISIBLE, 0, 50, 50, 200, 150, 0);
        p2 = add(1, "P2", WS_POPUP | WS_VISIBLE, WS_EX_TOPMOST, 150, 100, 200, 150, 0);
        p3 = add(2, "P3", WS_POPUP | WS_VISIBLE, 0, 250, 150, 200, 150, 0); pump();
        (void)SetActiveWindow(p3); pump();
        step("initial (P3 active)");
        state();
        step("BringWindowToTop(P1): below the topmost P2");
        (void)BringWindowToTop(p1); pump(); state();
        step("SetWindowPos(P2, HWND_NOTOPMOST, NOMOVE|NOSIZE)");
        (void)SetWindowPos(p2, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE); pump(); state();
        step("SetWindowPos(P3, HWND_TOPMOST, NOMOVE|NOSIZE|NOACTIVATE)");
        (void)SetWindowPos(p3, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE); pump(); state();
        clearall();
    }
    SC(10, "WS_EX_NOACTIVATE and disabled windows: click and activation") {
        HWND p1, p2, p3;
        p1 = add(0, "P1", WS_POPUP | WS_VISIBLE, 0, 50, 50, 200, 150, 0);
        p2 = add(1, "P2", WS_POPUP | WS_VISIBLE, WS_EX_NOACTIVATE, 150, 100, 200, 150, 0);
        p3 = add(2, "P3", WS_POPUP | WS_VISIBLE, 0, 400, 100, 200, 150, 0); pump();
        (void)SetActiveWindow(p1); pump();
        step("initial (P1 active)");
        state();
        step("click on P2 (WS_EX_NOACTIVATE) at 300,200");
        click(300, 200); state();
        step("EnableWindow(P3, FALSE) then click on P3 at 450,150");
        (void)EnableWindow(p3, 0); pump();
        click(450, 150); state();
        step("SetActiveWindow(P3) (disabled)");
        (void)SetActiveWindow(p3); pump(); state();
        clearall();
    }
    SC(11, "SetFocus: between top-level windows, to a child, to a disabled window, to NULL") {
        HWND t1, c1, t2;
        t1 = add(0, "T1", WS_POPUP | WS_VISIBLE | WS_CLIPCHILDREN, 0, 20, 20, 300, 250, 0);
        c1 = add(1, "C1", WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS, 0, 10, 10, 150, 100, t1);
        t2 = add(2, "T2", WS_POPUP | WS_VISIBLE, 0, 350, 100, 250, 200, 0); pump();
        (void)SetActiveWindow(t2); pump();
        step("initial (T2 active)");
        state();
        step("SetFocus(T1): the top-level window is activated");
        (void)SetFocus(t1); pump(); state();
        step("SetFocus(C1): a child of the active window");
        (void)SetFocus(c1); pump(); state();
        step("SetFocus(T2)");
        (void)SetFocus(t2); pump(); state();
        step("EnableWindow(T2, FALSE): the focus window is disabled");
        (void)EnableWindow(t2, 0); pump(); state();
        step("SetFocus(T2) while it is disabled");
        (void)SetFocus(t2); pump(); state();
        step("SetFocus(NULL)");
        (void)SetFocus(0); pump(); state();
        clearall();
    }
    SC(12, "mouse capture: the captured window gets the clicks, with client coordinates outside its rectangle; ReleaseCapture") {
        HWND p1, p2;
        p1 = add(0, "P1", WS_POPUP | WS_VISIBLE, 0, 50, 50, 200, 150, 0);
        p2 = add(1, "P2", WS_POPUP | WS_VISIBLE, 0, 350, 100, 200, 150, 0); pump();
        (void)SetActiveWindow(p2); pump();
        step("initial (P2 active)");
        state();
        step("SetCapture(P1)");
        (void)SetCapture(p1); pump(); state();
        step("click on P2 (400,150) while P1 has the capture: P1 gets it, in its own client coordinates");
        click(400, 150); state();
        step("ReleaseCapture");
        (void)ReleaseCapture(); pump(); state();
        step("click on P2 (400,150) after the release");
        click(400, 150); state();
        step("SetCapture(P2), then SetCapture(P1): the first loses it (WM_CAPTURECHANGED)");
        (void)SetCapture(p2); pump(); (void)SetCapture(p1); pump(); state();
        (void)ReleaseCapture(); pump();
        clearall();
    }
    SC(13, "ShowWindow on a window that is already visible changes nothing and activates nothing (a pop-up shown with SW_SHOWNOACTIVATE, as FL's menu pop-up is)") {
        HWND a, p, q;
        q = add(2, "Q", WS_POPUP | WS_VISIBLE, 0, 300, 150, 200, 150, 0);
        a = add(0, "A", WS_POPUP | WS_VISIBLE, 0, 20, 20, 400, 300, 0);
        p = add(1, "P", WS_POPUP | WS_VISIBLE, 0, 100, 100, 200, 150, a); pump();
        (void)SetActiveWindow(a); pump();
        step("(setup)");
        (void)ShowWindow(p, SW_HIDE); pump(); (void)ShowWindow(p, SW_SHOWNOACTIVATE); pump();
        step("initial (A active, P owned by A shown with SW_SHOWNOACTIVATE, Q)");
        state();
        step("ShowWindow(P, SW_SHOWNORMAL) on the visible P");
        (void)ShowWindow(p, SW_SHOWNORMAL); pump(); state();
        step("ShowWindow(P, SW_SHOW) on the visible P");
        (void)ShowWindow(p, SW_SHOW); pump(); state();
        step("ShowWindow(P, SW_RESTORE) on the visible P");
        (void)ShowWindow(p, SW_RESTORE); pump(); state();
        step("ShowWindow(P, SW_SHOWNOACTIVATE) on the visible P");
        (void)ShowWindow(p, SW_SHOWNOACTIVATE); pump(); state();
        (void)q;
        clearall();
    }
    SC(15, "the call sequence FL makes for its menu pop-up (recorded from the engine: create owned by the active window, TOPMOST, frame, size, SW_SHOWNOACTIVATE, SW_SHOWNORMAL), then the window that is created while the pop-up is up") {
        HWND a, p, l;
        a = add(0, "A", WS_POPUP | WS_VISIBLE, 0, 20, 20, 600, 400, 0); pump();
        (void)SetActiveWindow(a); pump();
        step("initial (A active)");
        state();
        step("CreateWindowExW(P, WS_POPUP, owner = GetActiveWindow() = A)");
        p = add(1, "P", WS_POPUP, 0, 230, 220, 186, 330, GetActiveWindow()); pump(); state();
        step("SetWindowPos(P, HWND_TOPMOST, NOMOVE|NOSIZE|NOACTIVATE)");
        (void)SetWindowPos(p, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE); pump(); state();
        step("SetWindowPos(P, 0, FRAMECHANGED|NOACTIVATE|NOZORDER|NOMOVE|NOSIZE)");
        (void)SetWindowPos(p, 0, 0, 0, 0, 0, SWP_FRAMECHANGED | SWP_NOACTIVATE | SWP_NOZORDER | SWP_NOMOVE | SWP_NOSIZE); pump(); state();
        step("SetWindowPos(P, 0, 495, 294, 176, 132, NOZORDER|NOACTIVATE)");
        (void)SetWindowPos(p, 0, 295, 194, 176, 132, SWP_NOZORDER | SWP_NOACTIVATE); pump(); state();
        step("ShowWindow(P, SW_SHOWNOACTIVATE)");
        (void)ShowWindow(p, SW_SHOWNOACTIVATE); pump(); state();
        step("ShowWindow(P, SW_SHOWNORMAL) (the VCL call, P is visible)");
        (void)ShowWindow(p, SW_SHOWNORMAL); pump(); state();
        step("click on P (330,250): the harness window procedure uses DefWindowProc, so WM_MOUSEACTIVATE is answered MA_ACTIVATE (FL's pop-up answers MA_NOACTIVATE)");
        click(330, 250); state();
        step("GetActiveWindow() is what FL passes as the owner of the next window: CreateWindowExW(L, owner = GetActiveWindow())");
        l = add(2, "L", WS_POPUP, 0, 100, 60, 300, 200, GetActiveWindow()); pump(); state();
        step("ShowWindow(L, SW_SHOWNORMAL)");
        (void)ShowWindow(l, SW_SHOWNORMAL); pump(); state();
        step("DestroyWindow(P): the owned L goes with its owner");
        (void)DestroyWindow(p); pump(); state();
        clearall();
    }
    /* scenario 14 (only when named on the command line, not in the default run): the activation / show contract as a matrix. 10 initial states of a window T (A is the other top-level window) x the calls ShowWindow(0..9), SetWindowPos with flag sets,
     * SetActiveWindow, SetForegroundWindow, BringWindowToTop, SetFocus, EnableWindow. One step per cell, titled "state | call"; the output of Wine 10.0 and of the runtime are compared cell by cell by tools/mk-act-matrix.py (pick1, decision 0242). */
    if (only == 14) {
        static const char *sn[] = { "S0 T hidden", "S1 T hidden owned by A", "S2 T visible", "S3 T visible owned by A", "S4 T visible and active", "S5 T minimized", "S6 T maximized", "S7 T visible WS_EX_NOACTIVATE", "S8 T visible disabled", "S9 T hidden WS_EX_NOACTIVATE" };
        static const char *wn[] = { "SW_HIDE", "SW_SHOWNORMAL", "SW_SHOWMINIMIZED", "SW_SHOWMAXIMIZED", "SW_SHOWNOACTIVATE", "SW_SHOW", "SW_MINIMIZE", "SW_SHOWMINNOACTIVE", "SW_SHOWNA", "SW_RESTORE" };
        static const UINT pf[] = { SWP_SHOWWINDOW, SWP_HIDEWINDOW, SWP_SHOWWINDOW | SWP_NOACTIVATE, 0, SWP_NOZORDER, SWP_NOACTIVATE, SWP_NOZORDER | SWP_NOACTIVATE, SWP_SHOWWINDOW | SWP_NOZORDER };
        static const char *pn[] = { "SHOWWINDOW", "HIDEWINDOW", "SHOWWINDOW|NOACTIVATE", "(none)", "NOZORDER", "NOACTIVATE", "NOZORDER|NOACTIVATE", "SHOWWINDOW|NOZORDER" };
        int s, o, nw = 10, np = 8, no = nw + np + 7;
        printf("#### scenario 14: the activation / show matrix (state of T x call on T)\n");
        for (s = 0; s < 10; s++) {
            for (o = 0; o < no; o++) {
                HWND a, t = 0;
                char title[200], call[120];
                step("(setup)");
                a = add(0, "A", WS_POPUP | WS_VISIBLE, 0, 20, 20, 400, 300, 0);
                switch (s) {
                case 0: t = add(1, "T", WS_POPUP, 0, 300, 150, 200, 150, 0); break;
                case 1: t = add(1, "T", WS_POPUP, 0, 100, 100, 200, 150, a); break;
                case 3: t = add(1, "T", WS_POPUP | WS_VISIBLE, 0, 100, 100, 200, 150, a); break;
                case 7: t = add(1, "T", WS_POPUP | WS_VISIBLE, WS_EX_NOACTIVATE, 300, 150, 200, 150, 0); break;
                case 9: t = add(1, "T", WS_POPUP, WS_EX_NOACTIVATE, 300, 150, 200, 150, 0); break;
                default: t = add(1, "T", WS_POPUP | WS_VISIBLE, 0, 300, 150, 200, 150, 0); break;
                }
                pump();
                if (s == 4) (void)SetActiveWindow(t); else (void)SetActiveWindow(a);
                pump();
                if (s == 5) { (void)ShowWindow(t, SW_SHOWMINNOACTIVE); pump(); }
                if (s == 6) { (void)ShowWindow(t, SW_SHOWMAXIMIZED); pump(); (void)SetActiveWindow(a); pump(); }
                if (s == 8) { (void)EnableWindow(t, 0); pump(); }
                if (o < nw) snprintf(call, sizeof call, "ShowWindow(T, %s)", wn[o]);
                else if (o < nw + np) snprintf(call, sizeof call, "SetWindowPos(T, HWND_TOP, NOMOVE|NOSIZE|%s)", pn[o - nw]);
                else snprintf(call, sizeof call, "%s", (const char *[]){ "SetActiveWindow(T)", "SetForegroundWindow(T)", "BringWindowToTop(T)", "SetFocus(T)", "EnableWindow(T, FALSE)", "EnableWindow(T, TRUE)", "SetWindowPos(T, HWND_BOTTOM, NOMOVE|NOSIZE)" }[o - nw - np]);
                snprintf(title, sizeof title, "%s | %s", sn[s], call);
                step(title);
                if (o < nw) (void)ShowWindow(t, o);
                else if (o < nw + np) (void)SetWindowPos(t, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | pf[o - nw]);
                else switch (o - nw - np) {
                    case 0: (void)SetActiveWindow(t); break;
                    case 1: (void)SetForegroundWindow(t); break;
                    case 2: (void)BringWindowToTop(t); break;
                    case 3: (void)SetFocus(t); break;
                    case 4: (void)EnableWindow(t, 0); break;
                    case 5: (void)EnableWindow(t, 1); break;
                    default: (void)SetWindowPos(t, HWND_BOTTOM, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE); break;
                }
                pump(); state();
                clearall();
            }
        }
    }
    return 0;
}

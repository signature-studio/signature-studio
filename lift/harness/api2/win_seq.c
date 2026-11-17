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


static HWND g_top, g_child;
static const char *nm_(HWND h) { return h == g_top ? "top" : h == g_child ? "child" : "?"; }
#ifdef API2_WINE
static void logmsg(HWND h, UINT m, WPARAM w, LPARAM l)
#else
static void logmsg(HWND h, unsigned m, uint64_t w, uint64_t l)
#endif
{
    /* only the messages of the window-management protocol; the pointers inside lParam are printed as their contents */
    switch (m) {
    case 0x0081: printf("  %s WM_NCCREATE\n", nm_(h)); break;
    case 0x0083: printf("  %s WM_NCCALCSIZE wp=%d\n", nm_(h), (int)w); break;
    case 0x0001: printf("  %s WM_CREATE\n", nm_(h)); break;
    case 0x0046: case 0x0047: { int32_t *p = (int32_t *)(uintptr_t)l; printf("  %s %s x=%d y=%d cx=%d cy=%d flags=%#x\n", nm_(h), m == 0x46 ? "WM_WINDOWPOSCHANGING" : "WM_WINDOWPOSCHANGED", p[4], p[5], p[6], p[7], (unsigned)p[8]); break; }
    case 0x0003: printf("  %s WM_MOVE x=%d y=%d\n", nm_(h), (int)(int16_t)(l & 0xFFFF), (int)(int16_t)((l >> 16) & 0xFFFF)); break;
    case 0x0005: printf("  %s WM_SIZE wp=%d cx=%d cy=%d\n", nm_(h), (int)w, (int)(l & 0xFFFF), (int)((l >> 16) & 0xFFFF)); break;
    case 0x0018: printf("  %s WM_SHOWWINDOW wp=%d lp=%d\n", nm_(h), (int)w, (int)l); break;
    case 0x0002: printf("  %s WM_DESTROY\n", nm_(h)); break;
    case 0x0082: printf("  %s WM_NCDESTROY\n", nm_(h)); break;
    case 0x0010: printf("  %s WM_CLOSE\n", nm_(h)); break;
    case 0x0006: printf("  %s WM_ACTIVATE wp=%d\n", nm_(h), (int)w); break;
    case 0x0007: printf("  %s WM_SETFOCUS\n", nm_(h)); break;
    case 0x0008: printf("  %s WM_KILLFOCUS\n", nm_(h)); break;
    case 0x001c: printf("  %s WM_ACTIVATEAPP wp=%d\n", nm_(h), (int)w); break;
    case 0x0086: printf("  %s WM_NCACTIVATE wp=%d\n", nm_(h), (int)w); break;
    case 0x000f: printf("  %s WM_PAINT\n", nm_(h)); break;
    case 0x0014: printf("  %s WM_ERASEBKGND\n", nm_(h)); break;
    case 0x0085: printf("  %s WM_NCPAINT\n", nm_(h)); break;
    case 0x0024: printf("  %s WM_GETMINMAXINFO\n", nm_(h)); break;
    case 0x0231: printf("  %s WM_ENTERSIZEMOVE\n", nm_(h)); break;
    case 0x0232: printf("  %s WM_EXITSIZEMOVE\n", nm_(h)); break;
    default: break;
    }
}

static void step(const char *s) { printf("%s:\n", s); }

#ifndef API2_WINE
static BOOL SetWindowPos_(HWND h, HWND after, int x, int y, int cx, int cy, UINT f) { return (BOOL)fle_api2_call(fle_imp_SetWindowPos, h, after, (uint32_t)x, (uint32_t)y, (uint32_t)cx, (uint32_t)cy, f, 0); }
static BOOL ShowWindow_(HWND h, int c) { return (BOOL)fle_api2_call(fle_imp_ShowWindow, h, (uint32_t)c, 0, 0, 0, 0, 0, 0); }
static BOOL MoveWindow_(HWND h, int x, int y, int cx, int cy, BOOL r) { return (BOOL)fle_api2_call(fle_imp_MoveWindow, h, (uint32_t)x, (uint32_t)y, (uint32_t)cx, (uint32_t)cy, r, 0, 0); }
#define SWP_NOSIZE 1u
#define SWP_NOMOVE 2u
#define SWP_NOZORDER 4u
#define SWP_NOACTIVATE 0x10u
#define SW_HIDE 0
#define SW_SHOWNOACTIVATE 4
#define SW_SHOW 5
#else
#define SetWindowPos_(h, a, x, y, cx, cy, f) SetWindowPos(h, a, x, y, cx, cy, f)
#define ShowWindow_(h, c) ShowWindow(h, c)
#define MoveWindow_(h, x, y, cx, cy, r) MoveWindow(h, x, y, cx, cy, r)
#endif

int main(void)
{
    setvbuf(stdout, NULL, _IONBF, 0);
    regclass();
    step("create top-level popup (no WS_VISIBLE) 100,100 300x200");
    g_top = mk(WS_POPUP, 100, 100, 300, 200, 0);
    pump();
    step("create child 10,10 100x50 (not visible)");
    g_child = mk(WS_CHILD, 10, 10, 100, 50, g_top);
    pump();
    step("ShowWindow(top, SW_SHOWNOACTIVATE)");
    ShowWindow_(g_top, SW_SHOWNOACTIVATE);
    pump();
    step("SetWindowPos(top, move to 150,130, NOSIZE|NOZORDER|NOACTIVATE)");
    SetWindowPos_(g_top, 0, 150, 130, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
    pump();
    step("SetWindowPos(top, size to 320x210, NOMOVE|NOZORDER|NOACTIVATE)");
    SetWindowPos_(g_top, 0, 0, 0, 320, 210, SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE);
    pump();
    step("SetWindowPos(top, same position and size)");
    SetWindowPos_(g_top, 0, 150, 130, 320, 210, SWP_NOZORDER | SWP_NOACTIVATE);
    pump();
    step("MoveWindow(child, 20,20, 110x60, repaint)");
    MoveWindow_(g_child, 20, 20, 110, 60, 1);
    pump();
    step("ShowWindow(child, SW_SHOW)");
    ShowWindow_(g_child, SW_SHOW);
    pump();
    step("ShowWindow(top, SW_HIDE)");
    ShowWindow_(g_top, SW_HIDE);
    pump();
    step("DestroyWindow(top)");
    DestroyWindow(g_top);
    pump();
    return 0;
}

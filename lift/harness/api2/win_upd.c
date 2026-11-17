/* Decision 0210 (api2, opt-in test tool): the update-region cases of tools/wine_update_region.c, run (a) against Wine 10.0's user32 (API2_WINE: mingw build, headless Wine) and (b) against the REAL runtime window layer
 * (fle_win.c, fle_class.c, fle_input.c ... with a stub guest window procedure that calls DefWindowProcW, the scaffold of harness/web/g3/g3_scenarios.c). Same lines out of both; the lines on which the runtime differs are listed in
 * expected/win-known-divergence.txt (each is a gap of the runtime's window model, not a choice). */
#include <stdio.h>
#include <string.h>
#include <stdint.h>

#ifdef API2_WINE
#include <windows.h>
static LRESULT CALLBACK wp(HWND h, UINT m, WPARAM w, LPARAM l) { return DefWindowProcW(h, m, w, l); }
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
static void wndproc(void)
{
    uint64_t hw = R[1], msg = R[2], wp_ = R[8], lp = R[9];
    if (msg == 0x81) { R[0] = 1; return; }
    if (msg == 0x83 || msg == 0x01) { R[0] = 0; return; }
    (void)fle_api2_call(fle_imp_DefWindowProcW, hw, msg, wp_, lp, 0, 0, 0, 0);
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

static void show(const char *label, HWND h)
{
    RECT r;
    int has;
    memset(&r, 0, sizeof r);
    has = GetUpdateRect(h, &r, 0);
    printf("%s: update-rect=%d (%ld,%ld,%ld,%ld)\n", label, has, has ? (long)r.left : 0L, has ? (long)r.top : 0L, has ? (long)r.right : 0L, has ? (long)r.bottom : 0L);
}

static void run(DWORD pstyle, const char *tag)
{
    HWND p, c;
    RECT r1 = { 30, 30, 60, 60 };
    p = mk(pstyle | WS_POPUP | WS_VISIBLE, 0, 0, 200, 200, 0);
    c = mk(WS_CHILD | WS_VISIBLE, 20, 20, 100, 100, p);
    pump();
    ValidateRect(p, NULL); ValidateRect(c, NULL);
    printf("== parent style %s\n", tag);
    show("start parent", p); show("start child", c);
    InvalidateRect(p, NULL, 0); show("after Invalidate(parent) parent", p); show("after Invalidate(parent) child", c);
    ValidateRect(p, NULL); ValidateRect(c, NULL);
    InvalidateRect(c, NULL, 0); show("after Invalidate(child) parent", p); show("after Invalidate(child) child", c);
    ValidateRect(p, NULL); ValidateRect(c, NULL);
    InvalidateRect(p, &r1, 0); show("after Invalidate(parent,[30,30,60,60]) parent", p); show("after that child", c);
    ValidateRect(p, NULL); ValidateRect(c, NULL);
#ifdef API2_WINE   /* the runtime's ScrollWindowEx needs a drawing surface of the window and stops without one (fle_wall_ia2w.c): Wine-only line, recorded in expected/win-wine-only.txt */
    ScrollWindowEx(p, 10, 0, NULL, NULL, NULL, NULL, SW_INVALIDATE); show("after ScrollWindowEx(parent,10,0,SW_INVALIDATE) parent", p); show("child", c);
#endif
    DestroyWindow(p);
}

int main(void)
{
    setvbuf(stdout, NULL, _IONBF, 0);
    regclass();
    run(0, "none");
    run(WS_CLIPCHILDREN, "WS_CLIPCHILDREN");
    return 0;
}

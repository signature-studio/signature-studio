/* Decision 0247 (alpha3, opt-in test tool; after harness/api2/win_upd.c): the update region as a set of rectangles (InvalidateRect, ValidateRect, ValidateRgn, RedrawWindow with an hrgn, GetUpdateRect, GetUpdateRgn, BeginPaint + GetClipBox/RectVisible, ShowWindow), run (a) against Wine 10.0 (mingw, headless) and (b) against the REAL runtime window layer. Same lines out of both. tools/build-wine2.sh. */
/* (scaffold) Decision 0210 (api2, opt-in test tool): the update-region cases of tools/wine_update_region.c, run (a) against Wine 10.0's user32 (API2_WINE: mingw build, headless Wine) and (b) against the REAL runtime window layer
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

typedef HRGN HRGN_;
typedef HDC HDC_;
static void ps_rect(const PAINTSTRUCT *ps, RECT *r) { *r = ps->rcPaint; }
#define RGN_OR_ RGN_OR
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
#include "fle_gdi.h"
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

typedef uint64_t HRGN_;
typedef uint64_t HDC_;
static HRGN_ CreateRectRgn(int l, int t, int r, int b) { return fle_api2_call(fle_imp_CreateRectRgn, (uint32_t)l, (uint32_t)t, (uint32_t)r, (uint32_t)b, 0, 0, 0, 0); }
static int GetUpdateRgn(HWND h, HRGN_ g, BOOL e) { return (int)fle_api2_call(fle_imp_GetUpdateRgn, h, g, e, 0, 0, 0, 0, 0); }
static BOOL PtInRegion(HRGN_ g, int x, int y) { return (BOOL)fle_api2_call(fle_imp_PtInRegion, g, (uint32_t)x, (uint32_t)y, 0, 0, 0, 0, 0); }
static int GetRgnBox(HRGN_ g, RECT *r) { return (int)fle_api2_call(fle_imp_GetRgnBox, g, A(r), 0, 0, 0, 0, 0, 0); }
static BOOL ValidateRgn(HWND h, HRGN_ g) { return (BOOL)fle_api2_call(fle_imp_ValidateRgn, h, g, 0, 0, 0, 0, 0, 0); }
static BOOL RedrawWindow(HWND h, const RECT *r, HRGN_ g, UINT f) { return (BOOL)fle_api2_call(fle_imp_RedrawWindow, h, A(r), g, f, 0, 0, 0, 0); }
static int CombineRgn(HRGN_ d, HRGN_ a, HRGN_ b, int m) { return (int)fle_api2_call(fle_imp_CombineRgn, d, a, b, (uint32_t)m, 0, 0, 0, 0); }
static BOOL DeleteObject(HRGN_ g) { return (BOOL)fle_api2_call(fle_imp_DeleteObject, g, 0, 0, 0, 0, 0, 0, 0); }
static BOOL ShowWindow(HWND h, int c) { return (BOOL)fle_api2_call(fle_imp_ShowWindow, h, (uint32_t)c, 0, 0, 0, 0, 0, 0); }
static int GetClipBox(HDC_ dc, RECT *r) { return (int)fle_api2_call(fle_imp_GetClipBox, dc, A(r), 0, 0, 0, 0, 0, 0); }
static BOOL RectVisible(HDC_ dc, const RECT *r) { return (BOOL)fle_api2_call(fle_imp_RectVisible, dc, A(r), 0, 0, 0, 0, 0, 0); }
typedef struct { uint8_t b[72]; } PAINTSTRUCT;
static HDC_ BeginPaint(HWND h, PAINTSTRUCT *ps) { return fle_api2_call(fle_imp_BeginPaint, h, A(ps), 0, 0, 0, 0, 0, 0); }
static BOOL EndPaint(HWND h, PAINTSTRUCT *ps) { return (BOOL)fle_api2_call(fle_imp_EndPaint, h, A(ps), 0, 0, 0, 0, 0, 0); }
static void ps_rect(const PAINTSTRUCT *ps, RECT *r) { memcpy(r, ps->b + 12, 16); }
#define RGN_OR_ 2
#endif


#define SW_HIDE_ 0
#define SW_SHOWNA_ 8

static const char *pt(HRGN_ g, int x, int y) { return PtInRegion(g, x, y) ? "in" : "out"; }
static void rgn_show(const char *label, HWND h, HRGN_ g)
{
    RECT b;
    int code, bc;

    memset(&b, 0, sizeof b);
    code = GetUpdateRgn(h, g, 0);
    bc = GetRgnBox(g, &b);
    printf("%s: GetUpdateRgn=%d box-code=%d box=(%ld,%ld,%ld,%ld) A(20,15)=%s gap(60,40)=%s B(110,70)=%s B-left(102,70)=%s corner(40,12)=%s\n", label, code, bc, (long)b.left, (long)b.top, (long)b.right, (long)b.bottom,
           pt(g, 20, 15), pt(g, 60, 40), pt(g, 110, 70), pt(g, 102, 70), pt(g, 40, 12));
}
static void show(const char *label, HWND h)
{
    RECT r;
    int has;

    memset(&r, 0, sizeof r);
    has = GetUpdateRect(h, &r, 0);
    printf("%s: update-rect=%d (%ld,%ld,%ld,%ld)\n", label, has, has ? (long)r.left : 0L, has ? (long)r.top : 0L, has ? (long)r.right : 0L, has ? (long)r.bottom : 0L);
}

int main(void)
{
    HWND p;
    HRGN_ g, rA, rB, rU, half;
    RECT A_ = { 10, 10, 30, 20 }, B_ = { 100, 60, 120, 80 }, cover = { 0, 0, 60, 40 }, ov = { 25, 15, 45, 25 }, tmp;
    PAINTSTRUCT ps;
    HDC_ dc;
    RECT gap = { 50, 30, 60, 40 }, inA = { 12, 12, 14, 14 };
    int i;

    setvbuf(stdout, NULL, _IONBF, 0);
    regclass();
    p = mk(WS_POPUP | WS_VISIBLE, 0, 0, 200, 200, 0);
    pump();
    ValidateRect(p, NULL);
    g = CreateRectRgn(0, 0, 1, 1);
    printf("== update region as rectangles (no child)\n");
    show("start", p);
    InvalidateRect(p, &A_, 0); InvalidateRect(p, &B_, 0);
    show("after Invalidate(A), Invalidate(B)", p);
    rgn_show("  GetUpdateRgn", p, g);
    InvalidateRect(p, &A_, 0);
    rgn_show("  after Invalidate(A) again", p, g);
    InvalidateRect(p, &ov, 0);
    rgn_show("  after Invalidate(overlap [25,15,45,25])", p, g);
    ValidateRect(p, &cover);
    show("after ValidateRect([0,0,60,40])", p);
    rgn_show("  GetUpdateRgn", p, g);
    half = CreateRectRgn(100, 60, 110, 80);
    ValidateRgn(p, half);
    show("after ValidateRgn(left half of B)", p);
    rgn_show("  GetUpdateRgn", p, g);
    ValidateRect(p, NULL);
    show("after ValidateRect(NULL)", p);
    rA = CreateRectRgn(10, 10, 30, 20); rB = CreateRectRgn(100, 60, 120, 80); rU = CreateRectRgn(0, 0, 1, 1);
    CombineRgn(rU, rA, rB, RGN_OR_);
    RedrawWindow(p, NULL, rU, 0x41);
    show("after RedrawWindow(hrgn=A|B, INVALIDATE|NOCHILDREN)", p);
    rgn_show("  GetUpdateRgn", p, g);
    RedrawWindow(p, NULL, rU, 0x8);
    show("after RedrawWindow(hrgn=A|B, VALIDATE)", p);
    InvalidateRect(p, &A_, 0); InvalidateRect(p, &B_, 0);
    dc = BeginPaint(p, &ps);
    ps_rect(&ps, &tmp);
    printf("BeginPaint (A and B invalid): rcPaint=(%ld,%ld,%ld,%ld)\n", (long)tmp.left, (long)tmp.top, (long)tmp.right, (long)tmp.bottom);
    (void)dc;
    EndPaint(p, &ps);
    show("after EndPaint", p);
    printf("== a window shown again\n");
    ValidateRect(p, NULL);
    ShowWindow(p, SW_HIDE_);
    InvalidateRect(p, &A_, 0);
    show("hidden, Invalidate(A)", p);
    ShowWindow(p, SW_SHOWNA_);
    show("after ShowWindow(SW_SHOWNA)", p);
    ValidateRect(p, NULL);
    printf("== 40 small disjoint rectangles\n");
    for (i = 0; i < 40; i++) { RECT s; s.left = 4 * i; s.top = (i % 2) * 20; s.right = 4 * i + 2; s.bottom = (i % 2) * 20 + 2; InvalidateRect(p, &s, 0); }
    show("update-rect", p);
    rgn_show("  GetUpdateRgn", p, g);
    DeleteObject(rA); DeleteObject(rB); DeleteObject(rU); DeleteObject(half); DeleteObject(g);
    DestroyWindow(p);
    return 0;
}

/* Decision 0323 (ci1, test tool; the scaffold of harness/api2/win_act.c): three Win32 contracts of the window layer, run (a) against Wine 10.0's user32 (C0323_WINE: mingw build, headless Wine, the null display
 * driver) and (b) against the REAL runtime (fle_win.c, fle_input.c, fle_gdi.c ... with a harness window procedure that calls DefWindowProcW). Same lines out of both; the lines on which the runtime differs are listed in
 * expected/known-divergence.txt. Wine is the reference, not the truth (UNOFFICIAL): a difference is judged against the Learn page.
 *   E  DefWindowProc(WM_ERASEBKGND): the class background brush fills the clip box of the DC, the answer is 1 with a brush and 0 without (memory DCs; no window surface is read)
 *   P  the erase of a window DC: BeginPaint / RedrawWindow(RDW_ERASENOW) / the default WM_PAINT send WM_ERASEBKGND with a DC clipped to the update region; fErase; the erase flag after RDW_ERASENOW
 *   T  WM_NCHITTEST answered HTTRANSPARENT: the message goes on to the windows beneath in the same thread
 *   H  the WH_MOUSE and WH_KEYBOARD hooks: called when GetMessage / PeekMessage is about to return a mouse or key message; HC_ACTION / HC_NOREMOVE; a nonzero answer discards the message; CallNextHookEx
 * Coordinates are inside 0..700 x 0..500 (the smallest screen of the two sides). usage: contract_0323 [E|P|T|H] */
#include <stdio.h>
#include <string.h>
#include <stdint.h>

#ifdef C0323_WINE
#include <windows.h>
typedef LRESULT LR;
static void pump(void) { MSG msg; while (PeekMessageW(&msg, NULL, 0, 0, PM_REMOVE)) DispatchMessageW(&msg); }
static LRESULT CALLBACK wp(HWND h, UINT m, WPARAM w, LPARAM l);
static LRESULT CALLBACK hk_mouse(int n, WPARAM w, LPARAM l);
static LRESULT CALLBACK hk_key(int n, WPARAM w, LPARAM l);
static void regclass(const char *name, uint32_t style, uint64_t bg)
{
    WNDCLASSW wc;
    wchar_t nm[32];
    int i;
    for (i = 0; name[i]; i++) nm[i] = (wchar_t)name[i];
    nm[i] = 0;
    memset(&wc, 0, sizeof wc);
    wc.style = style;
    wc.lpfnWndProc = wp;
    wc.hInstance = GetModuleHandleW(NULL);
    wc.lpszClassName = nm;
    wc.hbrBackground = (HBRUSH)(uintptr_t)bg;
    RegisterClassW(&wc);
}
static HBRUSH mkbrush(uint32_t colorref) { return CreateSolidBrush(colorref); }
static HWND mkwin(const char *cls, DWORD style, int x, int y, int cx, int cy, HWND parent)
{
    wchar_t nm[32];
    int i;
    for (i = 0; cls[i]; i++) nm[i] = (wchar_t)cls[i];
    nm[i] = 0;
    return CreateWindowExW(0, nm, L"", style, x, y, cx, cy, parent, NULL, GetModuleHandleW(NULL), NULL);
}
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
static void key(int down, unsigned vk, unsigned scan)
{
    INPUT in;
    memset(&in, 0, sizeof in);
    in.type = INPUT_KEYBOARD;
    in.ki.wVk = (WORD)vk;
    in.ki.wScan = (WORD)scan;
    in.ki.dwFlags = down ? 0 : KEYEVENTF_KEYUP;
    SendInput(1, &in, sizeof in);
}
static LR defproc(HWND h, UINT m, WPARAM w, LPARAM l) { return DefWindowProcW(h, m, w, l); }
static LR sendmsg(HWND h, UINT m, WPARAM w, LPARAM l) { return SendMessageW(h, m, w, l); }
static HDC memdc(int w, int h) { HDC dc = CreateCompatibleDC(NULL), scr = GetDC(NULL); HBITMAP b = CreateCompatibleBitmap(scr, w, h); ReleaseDC(NULL, scr); SelectObject(dc, b); return dc; }
static void fillrect(HDC dc, int l, int t, int r, int b, uint32_t colorref) { RECT rc = { l, t, r, b }; HBRUSH br = CreateSolidBrush(colorref); FillRect(dc, &rc, br); DeleteObject(br); }
static uint32_t getpixel(HDC dc, int x, int y) { return (uint32_t)GetPixel(dc, x, y) & 0xFFFFFFu; }
static void clip(HDC dc, int l, int t, int r, int b) { IntersectClipRect(dc, l, t, r, b); }
static void unclip(HDC dc) { SelectClipRgn(dc, NULL); }
static int rectvisible(HDC dc, int l, int t, int r, int b) { RECT rc = { l, t, r, b }; return RectVisible(dc, &rc) ? 1 : 0; }
static void invalidate(HWND h, int l, int t, int r, int b, int erase) { RECT rc = { l, t, r, b }; InvalidateRect(h, &rc, erase); }
static void update(HWND h) { UpdateWindow(h); }
static void redraw(HWND h, int l, int t, int r, int b, unsigned fl) { RECT rc = { l, t, r, b }; RedrawWindow(h, &rc, NULL, fl); }
static void paint_probe(HWND h, int *ferase, int rc[4]) { PAINTSTRUCT ps; BeginPaint(h, &ps); *ferase = ps.fErase; rc[0] = ps.rcPaint.left; rc[1] = ps.rcPaint.top; rc[2] = ps.rcPaint.right; rc[3] = ps.rcPaint.bottom; EndPaint(h, &ps); }
static int getupdate(HWND h, int rc[4]) { RECT r = { 0, 0, 0, 0 }; BOOL b = GetUpdateRect(h, &r, FALSE); rc[0] = r.left; rc[1] = r.top; rc[2] = r.right; rc[3] = r.bottom; return b ? 1 : 0; }
static void showwin(HWND h, int c) { ShowWindow(h, c); }
static void enablewin(HWND h, int e) { EnableWindow(h, e); }
static void destroywin(HWND h) { DestroyWindow(h); }
static HWND setfocus(HWND h) { SetActiveWindow(h); return SetFocus(h); }
static void *hook_install(int id, int which) { return (void *)SetWindowsHookExW(id, which ? (HOOKPROC)hk_key : (HOOKPROC)hk_mouse, NULL, GetCurrentThreadId()); }
static void hook_remove(void *hh) { UnhookWindowsHookEx((HHOOK)hh); }
static LR hook_next(void *hh, int n, WPARAM w, LPARAM l) { return CallNextHookEx((HHOOK)hh, n, w, l); }
static int peek(int remove) { MSG msg; BOOL b = PeekMessageW(&msg, NULL, 0, 0, remove ? PM_REMOVE : PM_NOREMOVE); if (b && remove) DispatchMessageW(&msg); return b ? (int)msg.message : 0; }
static uint64_t mhs_u64(uint64_t l, int off) { return *(const uint64_t *)(uintptr_t)(l + off); }
static int32_t mhs_i32(uint64_t l, int off) { return *(const int32_t *)(uintptr_t)(l + off); }
#define W_(x) x
typedef HWND H_;
typedef WPARAM WP_;
typedef LPARAM LP_;
typedef HDC DC_;
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
#include "fle_gdc.h"
#include "fle_gdi.h"
#include "fle_wall_ia2.h"
#include "fle_api2.h"
uint64_t R[16];
fle_eflags FL;
fle_xmm X[16];
void fle_imp_PeekMessageW(void);
void fle_imp_DispatchMessageW(void);
void fle_imp_CreateSolidBrush(void);
void fle_imp_CreateCompatibleDC(void);
void fle_imp_CreateCompatibleBitmap(void);
void fle_imp_GetDC(void);
void fle_imp_ReleaseDC(void);
void fle_imp_SelectObject(void);
void fle_imp_DeleteObject(void);
void fle_imp_IntersectClipRect(void);
void fle_imp_SelectClipRgn(void);
void fle_imp_GetPixel(void);
void fle_imp_FillRect(void);
void fle_imp_RectVisible(void);
void fle_imp_InvalidateRect(void);
void fle_imp_UpdateWindow(void);
void fle_imp_RedrawWindow(void);
void fle_imp_BeginPaint(void);
void fle_imp_EndPaint(void);
void fle_imp_GetUpdateRect(void);
void fle_imp_SetWindowsHookExW(void);
void fle_imp_UnhookWindowsHookEx(void);
void fle_imp_CallNextHookEx(void);
void fle_imp_SetFocus(void);
void fle_imp_SetActiveWindow(void);
void fle_imp_ShowWindow(void);
void fle_imp_DestroyWindow(void);
void fle_imp_EnableWindow(void);
void fle_imp_DefWindowProcW(void);
void fle_imp_SendMessageW(void);
void fle_imp_GetCurrentThreadId(void);
typedef uint64_t HWND;
typedef uint64_t WPARAM;
typedef uint64_t LPARAM;
typedef uint64_t LR;
typedef uint64_t DC_;
typedef unsigned int DWORD, UINT;
typedef int BOOL;
#define A(x) ((uint64_t)(uintptr_t)(x))
#define VA_WP 0x2000
#define VA_HK_MOUSE 0x2010
#define VA_HK_KEY 0x2020
#define WS_POPUP 0x80000000u
#define WS_CHILD 0x40000000u
#define WS_VISIBLE 0x10000000u
#define SW_SHOWNOACTIVATE 4
#define RDW_INVALIDATE 1u
#define RDW_ERASE 4u
#define RDW_UPDATENOW 0x100u
#define RDW_ERASENOW 0x200u
#define CS_PARENTDC 0x80u
#define WH_KEYBOARD 2
#define WH_MOUSE 7
static void wndproc(void);
static void hkproc_mouse(void);
static void hkproc_key(void);
const fle_fn_entry fle_dispatch_table[] = { { VA_WP, wndproc }, { VA_HK_MOUSE, hkproc_mouse }, { VA_HK_KEY, hkproc_key } };
const uint64_t fle_dispatch_count = 3;
static uint64_t stk[0x4000] __attribute__((aligned(4096)));
#define API(name, ...) fle_api2_call(fle_imp_##name, __VA_ARGS__)
static uint16_t *w16(const char *s)
{
    static uint16_t pool[16][32];
    static int n;
    uint16_t *p = pool[n++ & 15];
    int i;
    for (i = 0; s[i]; i++) p[i] = (uint16_t)(uint8_t)s[i];
    p[i] = 0;
    return p;
}
static void regclass(const char *name, uint32_t style, uint64_t bg)
{
    static uint8_t wc[72];
    uint64_t proc = VA_WP, inst = 0x400000, nmp = A(w16(name)), cur = FLE_CURSOR_HANDLE_BASE + 4u, bgv = bg;
    memset(wc, 0, sizeof wc);
    memcpy(wc, &style, 4); memcpy(wc + 8, &proc, 8); memcpy(wc + 24, &inst, 8); memcpy(wc + 40, &cur, 8); memcpy(wc + 48, &bgv, 8); memcpy(wc + 64, &nmp, 8);
    (void)fle_api2_call(fle_imp_RegisterClassW, A(wc), 0, 0, 0, 0, 0, 0, 0);
}
static void rt_init(void)
{
    R[4] = (uint64_t)(uintptr_t)stk + 0x10000 + 8;
    fle_st(R[4], 8, 0xdead0000);
    fle_teb_set_stack(A(stk), A(stk) + sizeof stk);
}
static uint64_t mkbrush(uint32_t colorref) { return API(CreateSolidBrush, colorref, 0, 0, 0, 0, 0, 0, 0); }
static HWND mkwin(const char *cls, DWORD style, int x, int y, int cx, int cy, HWND parent)
{
    uint64_t h, sp0 = R[4];
    uint16_t empty[1] = { 0 };
    R[4] = (R[4] - 0x80) & ~UINT64_C(15);
    R[4] -= 8;
    fle_st(R[4], 8, 0xAAAA0000);
    fle_st(R[4] + 0x28, 8, (uint32_t)x); fle_st(R[4] + 0x30, 8, (uint32_t)y); fle_st(R[4] + 0x38, 8, (uint32_t)cx); fle_st(R[4] + 0x40, 8, (uint32_t)cy);
    fle_st(R[4] + 0x48, 8, parent); fle_st(R[4] + 0x50, 8, 0); fle_st(R[4] + 0x58, 8, 0x400000); fle_st(R[4] + 0x60, 8, 0);
    R[1] = 0; R[2] = A(w16(cls)); R[8] = A(empty); R[9] = style;
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
static void mouse(int kind, int x, int y) { (void)fle_input_host_mouse(kind == 2 ? FLE_IN_BDOWN : kind == 3 ? FLE_IN_BUP : FLE_IN_MOVE, x, y, FLE_IN_BTN_L); }
static void key(int down, unsigned vk, unsigned scan) { (void)fle_input_host_key(down ? FLE_IN_KDOWN : FLE_IN_KUP, vk, scan, 0); }
static LR defproc(HWND h, UINT m, WPARAM w, LPARAM l) { return fle_api2_call(fle_imp_DefWindowProcW, h, m, w, l, 0, 0, 0, 0); }
static LR sendmsg(HWND h, UINT m, WPARAM w, LPARAM l) { return API(SendMessageW, h, m, w, l, 0, 0, 0, 0); }
static DC_ memdc(int w, int h)
{
    uint64_t dc = API(CreateCompatibleDC, 0, 0, 0, 0, 0, 0, 0, 0), scr = API(GetDC, 0, 0, 0, 0, 0, 0, 0, 0), b = API(CreateCompatibleBitmap, scr, (uint32_t)w, (uint32_t)h, 0, 0, 0, 0, 0);
    (void)API(ReleaseDC, 0, scr, 0, 0, 0, 0, 0, 0);
    (void)API(SelectObject, dc, b, 0, 0, 0, 0, 0, 0);
    return dc;
}
static void fillrect(DC_ dc, int l, int t, int r, int b, uint32_t colorref)
{
    int32_t rc[4] = { l, t, r, b };
    uint64_t br = mkbrush(colorref);
    (void)API(FillRect, dc, A(rc), br, 0, 0, 0, 0, 0);
    (void)API(DeleteObject, br, 0, 0, 0, 0, 0, 0, 0);
}
static uint32_t getpixel(DC_ dc, int x, int y) { return (uint32_t)API(GetPixel, dc, (uint32_t)x, (uint32_t)y, 0, 0, 0, 0, 0) & 0xFFFFFFu; }
static void clip(DC_ dc, int l, int t, int r, int b) { (void)API(IntersectClipRect, dc, (uint32_t)l, (uint32_t)t, (uint32_t)r, (uint32_t)b, 0, 0, 0); }
static void unclip(DC_ dc) { (void)API(SelectClipRgn, dc, 0, 0, 0, 0, 0, 0, 0); }
static int rectvisible(DC_ dc, int l, int t, int r, int b) { int32_t rc[4] = { l, t, r, b }; return (int)(API(RectVisible, dc, A(rc), 0, 0, 0, 0, 0, 0) & 0xFFFFFFFFu) ? 1 : 0; }
static void invalidate(HWND h, int l, int t, int r, int b, int erase) { int32_t rc[4] = { l, t, r, b }; (void)API(InvalidateRect, h, A(rc), (uint32_t)erase, 0, 0, 0, 0, 0); }
static void update(HWND h) { (void)API(UpdateWindow, h, 0, 0, 0, 0, 0, 0, 0); }
static void redraw(HWND h, int l, int t, int r, int b, unsigned fl) { int32_t rc[4] = { l, t, r, b }; (void)API(RedrawWindow, h, A(rc), 0, fl, 0, 0, 0, 0); }
static void paint_probe(HWND h, int *ferase, int rc[4])
{
    uint8_t ps[72];
    memset(ps, 0, sizeof ps);
    (void)API(BeginPaint, h, A(ps), 0, 0, 0, 0, 0, 0);
    *ferase = (int)*(uint32_t *)(ps + 8);
    memcpy(rc, ps + 12, 16);
    (void)API(EndPaint, h, A(ps), 0, 0, 0, 0, 0, 0);
}
static int getupdate(HWND h, int rc[4]) { int32_t r[4] = { 0, 0, 0, 0 }; uint64_t b = API(GetUpdateRect, h, A(r), 0, 0, 0, 0, 0, 0); memcpy(rc, r, 16); return (b & 0xFFFFFFFFu) ? 1 : 0; }
static void showwin(HWND h, int c) { (void)API(ShowWindow, h, (uint32_t)c, 0, 0, 0, 0, 0, 0); }
static void enablewin(HWND h, int e) { (void)API(EnableWindow, h, (uint32_t)e, 0, 0, 0, 0, 0, 0); }
static void destroywin(HWND h) { (void)API(DestroyWindow, h, 0, 0, 0, 0, 0, 0, 0); }
static HWND setfocus(HWND h) { (void)API(SetActiveWindow, h, 0, 0, 0, 0, 0, 0, 0); return API(SetFocus, h, 0, 0, 0, 0, 0, 0, 0); }
static void *hook_install(int id, int which) { return (void *)(uintptr_t)API(SetWindowsHookExW, (uint32_t)id, which ? VA_HK_KEY : VA_HK_MOUSE, 0, API(GetCurrentThreadId, 0, 0, 0, 0, 0, 0, 0, 0) & 0xFFFFFFFFu, 0, 0, 0, 0); }
static void hook_remove(void *hh) { (void)API(UnhookWindowsHookEx, (uint64_t)(uintptr_t)hh, 0, 0, 0, 0, 0, 0, 0); }
static LR hook_next(void *hh, int n, WPARAM w, LPARAM l) { return API(CallNextHookEx, (uint64_t)(uintptr_t)hh, (uint32_t)n, w, l, 0, 0, 0, 0); }
static int peek(int remove)
{
    uint8_t msg[FLE_MSG_SIZE];
    uint64_t r = fle_api2_call(fle_imp_PeekMessageW, A(msg), 0, 0, 0, remove ? 1 : 0, 0, 0, 0);
    int m = (int)*(uint32_t *)(msg + 8);
    if ((r & 1) && remove) (void)fle_api2_call(fle_imp_DispatchMessageW, A(msg), 0, 0, 0, 0, 0, 0, 0);
    return (r & 1) ? m : 0;
}
static uint64_t mhs_u64(uint64_t l, int off) { return *(const uint64_t *)(uintptr_t)(l + off); }
static int32_t mhs_i32(uint64_t l, int off) { return *(const int32_t *)(uintptr_t)(l + off); }
#define W_(x) x
typedef uint64_t H_;
typedef uint64_t WP_;
typedef uint64_t LP_;
#endif

#define NW 12
static HWND g_h[NW];
static const char *g_n[NW];
static int g_mute;
static const char *nm_(HWND h)
{
    int k;
    for (k = 0; k < NW; k++) if (g_h[k] && g_h[k] == h) return g_n[k];
    return h ? "?" : "none";
}
#define P(...) do { if (!g_mute) printf(__VA_ARGS__); } while (0)
static int slot_of(HWND h) { int k; for (k = 0; k < NW; k++) if (g_h[k] && g_h[k] == h) return k; return -1; }
/* behaviours of the harness window procedure, by the first letter of the window's name: 'T' answers WM_NCHITTEST with HTTRANSPARENT; 'C' handles WM_PAINT with BeginPaint / EndPaint and prints PAINTSTRUCT;
 * 'Q' probes the DC of WM_ERASEBKGND with RectVisible (g_probe_in must be visible, g_probe_out must not). */
static int g_probe_in[4] = { 12, 12, 14, 14 }, g_probe_out[4] = { 80, 60, 82, 62 };
static LR wproc_common(HWND h, UINT m, WPARAM w, LPARAM l)
{
    const char *n = nm_(h);
    LR r;
    switch (m) {
    case 0x0084:
        P("  %s WM_NCHITTEST\n", n);
        if (n[0] == 'T') return (LR)-1;
        break;
    case 0x0014:
        r = defproc(h, m, w, l);
        P("  %s WM_ERASEBKGND dc=%d", n, w ? 1 : 0);
        if (n[0] == 'Q' && w) P(" probe(inside the update rect)=%d probe(outside)=%d", rectvisible((DC_)w, g_probe_in[0], g_probe_in[1], g_probe_in[2], g_probe_in[3]), rectvisible((DC_)w, g_probe_out[0], g_probe_out[1], g_probe_out[2], g_probe_out[3]));
        P(" -> %d\n", (int)(r & 0xFFFFFFFFu));
        return r;
    case 0x000F:
        if (n[0] == 'C' || n[0] == 'Q') {
            int fe, rc[4];
            paint_probe(h, &fe, rc);
            P("  %s WM_PAINT BeginPaint fErase=%d rcPaint=(%d,%d,%d,%d)\n", n, fe, rc[0], rc[1], rc[2], rc[3]);
            return 0;
        }
        P("  %s WM_PAINT (default)\n", n);
        break;
    case 0x0201: P("  %s WM_LBUTTONDOWN x=%d y=%d\n", n, (int)(int16_t)(l & 0xFFFF), (int)(int16_t)((l >> 16) & 0xFFFF)); break;
    case 0x0202: P("  %s WM_LBUTTONUP x=%d y=%d\n", n, (int)(int16_t)(l & 0xFFFF), (int)(int16_t)((l >> 16) & 0xFFFF)); break;
    case 0x0200: P("  %s WM_MOUSEMOVE x=%d y=%d\n", n, (int)(int16_t)(l & 0xFFFF), (int)(int16_t)((l >> 16) & 0xFFFF)); break;
    case 0x0100: P("  %s WM_KEYDOWN vk=%#x lp=%#x\n", n, (unsigned)w, (unsigned)(l & 0xFFFFFFFFu)); break;
    case 0x0101: P("  %s WM_KEYUP vk=%#x lp=%#x\n", n, (unsigned)w, (unsigned)(l & 0xFFFFFFFFu)); break;
    case 0x0020: P("  %s WM_SETCURSOR hit=%d msg=%#x\n", n, (int)(int16_t)(l & 0xFFFF), (unsigned)((l >> 16) & 0xFFFF)); break;
    case 0x0021: P("  %s WM_MOUSEACTIVATE hit=%d msg=%#x\n", n, (int)(int16_t)(l & 0xFFFF), (unsigned)((l >> 16) & 0xFFFF)); break;
    case 0x00A0: P("  %s WM_NCMOUSEMOVE ht=%d\n", n, (int)w); break;
    case 0x00A1: P("  %s WM_NCLBUTTONDOWN ht=%d\n", n, (int)w); break;
    case 0x00A2: P("  %s WM_NCLBUTTONUP ht=%d\n", n, (int)w); break;
    default: break;
    }
    return defproc(h, m, w, l);
}

/* the hooks: each call prints one line; g_hook_swallow is the message number the hook answers 1 for (0 = none); g_hook_next: call CallNextHookEx; the handle of the hook is in g_hh[] */
static void *g_hh[4];
static unsigned g_hook_swallow_mouse, g_hook_swallow_key;
static int g_hook_next = 1;
static LR hook_body(int which, int slot, int n, WPARAM w, LPARAM l)
{
    LR r = 0;
    if (n < 0) return hook_next(g_hh[slot], n, w, l);
    if (which == 0) {
        P("  HOOK%d mouse nCode=%d msg=%#x pt=(%d,%d) hwnd=%s ht=%d\n", slot, n, (unsigned)w, (int)mhs_i32(l, 0), (int)mhs_i32(l, 4), nm_((HWND)mhs_u64(l, 8)), (int)mhs_i32(l, 16));
        if (g_hook_swallow_mouse && (unsigned)w == g_hook_swallow_mouse && slot == 0) return 1;
    } else {
        P("  HOOK%d key nCode=%d vk=%#x lp=%#x\n", slot, n, (unsigned)w, (unsigned)(l & 0xFFFFFFFFu));
        if (g_hook_swallow_key && (unsigned)w == g_hook_swallow_key && slot == 2) return 1;
    }
    if (g_hook_next) r = hook_next(g_hh[slot], n, w, l);
    P("  HOOK%d -> %d\n", slot, (int)(r & 0xFFFFFFFFu));
    return r;
}
#ifdef C0323_WINE
static LRESULT CALLBACK wp(HWND h, UINT m, WPARAM w, LPARAM l) { return wproc_common(h, m, w, l); }
static LRESULT CALLBACK hk_mouse(int n, WPARAM w, LPARAM l) { return hook_body(0, 0, n, w, l); }
static LRESULT CALLBACK hk_key(int n, WPARAM w, LPARAM l) { return hook_body(1, 2, n, w, l); }
#else
static void wndproc(void) { uint64_t hw = R[1], msg = R[2], wp_ = R[8], lp = R[9]; R[0] = wproc_common(hw, (unsigned)msg, wp_, lp); }
static void hkproc_mouse(void) { int n = (int)(int32_t)R[1]; uint64_t w = R[2], l = R[8]; R[0] = hook_body(0, 0, n, w, l); }
static void hkproc_key(void) { int n = (int)(int32_t)R[1]; uint64_t w = R[2], l = R[8]; R[0] = hook_body(1, 2, n, w, l); }
#endif

static void step(const char *s) { printf("%s:\n", s); }
static HWND add(int slot, const char *nm, const char *cls, DWORD style, int x, int y, int cx, int cy, HWND parent)
{
    int mu = g_mute;
    g_n[slot] = nm;
    g_mute = 1;                    /* the messages of the creation (WM_ERASEBKGND of a visible window ...) are not what the scenarios are about */
    g_h[slot] = mkwin(cls, style, x, y, cx, cy, parent);
    g_mute = mu;
    return g_h[slot];
}
static void clearall(void)
{
    int k;
    for (k = NW - 1; k >= 0; k--) if (g_h[k]) destroywin(g_h[k]);
    pump();
    memset(g_h, 0, sizeof g_h);
}
static void click(int x, int y)
{
    mouse(1, x, y); pump();
    mouse(2, x, y); pump();
    mouse(3, x, y); pump();
}

static uint64_t g_solid;
static void classes(void)
{
    g_solid = (uint64_t)(uintptr_t)mkbrush(0x00563412u);
    regclass("cnone", 0, 0);
    regclass("csolid", 0, g_solid);
    regclass("cwin", 0, 5 + 1);                /* COLOR_WINDOW + 1 */
    regclass("cpar", CS_PARENTDC, g_solid);
}

static void erase_mem(const char *label, const char *cls)
{
    HWND w = add(0, "E", cls, WS_POPUP, 300, 300, 100, 80, 0);
    DC_ dc;
    LR r;
    unsigned c0, c1, c2;
    printf("%s (class %s)\n", label, cls);
    dc = memdc(16, 16);
    fillrect(dc, 0, 0, 16, 16, 0x00404040u);
    r = sendmsg(w, 0x0014, (WPARAM)dc, 0);
    c0 = getpixel(dc, 0, 0); c1 = getpixel(dc, 8, 8); c2 = getpixel(dc, 15, 15);
    printf("  whole DC: result=%d pixels (0,0)=%06x (8,8)=%06x (15,15)=%06x\n", (int)(r & 0xFFFFFFFFu), c0, c1, c2);
    fillrect(dc, 0, 0, 16, 16, 0x00404040u);
    clip(dc, 4, 4, 10, 8);
    r = sendmsg(w, 0x0014, (WPARAM)dc, 0);
    unclip(dc);   /* GetPixel outside the clip: CLR_INVALID in the Learn page, the pixel itself in Wine 10.0 */
    printf("  clip (4,4,10,8): result=%d pixels (5,5)=%06x (4,4)=%06x (9,7)=%06x (10,7)=%06x (3,5)=%06x (12,6)=%06x (5,8)=%06x\n", (int)(r & 0xFFFFFFFFu), getpixel(dc, 5, 5), getpixel(dc, 4, 4), getpixel(dc, 9, 7), getpixel(dc, 10, 7), getpixel(dc, 3, 5), getpixel(dc, 12, 6), getpixel(dc, 5, 8));
    clearall();
}

static void sc_E(void)
{
    printf("#### scenario E: DefWindowProc(WM_ERASEBKGND) with a memory DC\n");
    erase_mem("E1 solid brush", "csolid");
    erase_mem("E2 no brush", "cnone");
    erase_mem("E3 COLOR_WINDOW + 1", "cwin");
    erase_mem("E4 CS_PARENTDC with a brush", "cpar");
}

static void paint_case(const char *label, const char *cls, const char *name, int how)
{
    HWND w;
    int rc[4], gu;
    w = add(0, name, cls, WS_POPUP | WS_VISIBLE, 30, 30, 120, 90, 0);
    g_mute = 1; pump(); update(w); pump(); g_mute = 0;
    printf("%s (window %s, class %s)\n", label, name, cls);
    if (how == 0) {            /* InvalidateRect(rect, TRUE) + UpdateWindow */
        invalidate(w, 10, 10, 50, 40, 1);
        update(w); pump();
    } else if (how == 1) {     /* RedrawWindow(RDW_INVALIDATE | RDW_ERASE | RDW_ERASENOW), then UpdateWindow */
        redraw(w, 10, 10, 50, 40, RDW_INVALIDATE | RDW_ERASE | RDW_ERASENOW);
        gu = getupdate(w, rc);
        printf("  after RDW_ERASENOW: update rect %d (%d,%d,%d,%d)\n", gu, rc[0], rc[1], rc[2], rc[3]);
        update(w); pump();
    } else {                   /* InvalidateRect(rect, FALSE): no erase */
        invalidate(w, 10, 10, 50, 40, 0);
        update(w); pump();
    }
    gu = getupdate(w, rc);
    printf("  after: update rect %d (%d,%d,%d,%d)\n", gu, rc[0], rc[1], rc[2], rc[3]);
    clearall();
}

static void sc_P(void)
{
    printf("#### scenario P: the erase of a window DC\n");
    paint_case("P1 default WM_PAINT, class with a brush", "csolid", "WS", 0);
    paint_case("P2 default WM_PAINT, class without a brush", "cnone", "WN", 0);
    paint_case("P3 own BeginPaint, class with a brush", "csolid", "C1", 0);
    paint_case("P4 own BeginPaint, class without a brush", "cnone", "C2", 0);
    paint_case("P5 own BeginPaint, bErase FALSE", "csolid", "C3", 2);
    paint_case("P6 RDW_ERASENOW then BeginPaint, class with a brush", "csolid", "C4", 1);
    paint_case("P7 RDW_ERASENOW then BeginPaint, class without a brush", "cnone", "C5", 1);
    paint_case("P8 the DC of WM_ERASEBKGND is clipped to the update region (BeginPaint)", "csolid", "Q1", 0);
    paint_case("P9 the DC of WM_ERASEBKGND is clipped to the update region (RDW_ERASENOW)", "csolid", "Q2", 1);
    paint_case("P10 the DC of WM_ERASEBKGND is clipped to the update region (default WM_PAINT)", "csolid", "Q3", 0);
}

static void sc_T(void)
{
    HWND a, b, p, c1;
    printf("#### scenario T: WM_NCHITTEST answered HTTRANSPARENT\n");
    step("T1 two top-level pop-ups of one thread, the upper one (TB) transparent: a click where they overlap goes to the one beneath (A)");
    a = add(0, "A", "csolid", WS_POPUP | WS_VISIBLE, 20, 20, 200, 150, 0);
    b = add(1, "TB", "csolid", WS_POPUP | WS_VISIBLE, 120, 80, 200, 150, 0);
    (void)a; (void)b;
    g_mute = 1; pump(); g_mute = 0;
    step(" click (160,120): in both");
    click(160, 120);
    step(" click (280,200): only in TB: nothing beneath");
    click(280, 200);
    step(" click (60,50): only in A");
    click(60, 50);
    clearall();
    step("T2 a transparent child (TC) of a pop-up (P): the click goes to P");
    p = add(0, "P", "csolid", WS_POPUP | WS_VISIBLE, 40, 40, 300, 200, 0);
    c1 = add(1, "TC", "csolid", WS_CHILD | WS_VISIBLE, 20, 20, 100, 80, p);
    (void)c1;
    g_mute = 1; pump(); g_mute = 0;
    step(" click (80,80): in TC");
    click(80, 80);
    clearall();
    step("T3 an opaque child (C) of a pop-up (P) over a transparent child (TC): C is above TC and takes the click; a click on TC alone goes to P");
    p = add(0, "P", "csolid", WS_POPUP | WS_VISIBLE, 40, 40, 300, 200, 0);
    add(1, "TC", "csolid", WS_CHILD | WS_VISIBLE, 20, 20, 150, 100, p);
    add(2, "C", "csolid", WS_CHILD | WS_VISIBLE, 100, 60, 100, 80, p);
    g_mute = 1; pump(); g_mute = 0;
    step(" click (160,120): in C (and in TC)");
    click(160, 120);
    step(" click (80,80): in TC only");
    click(80, 80);
    clearall();
    step("T4 (not part of the HTTRANSPARENT change; S-disabled-drops of decision 0136) a disabled child (D) of a pop-up (P): Wine 10.0 skips a disabled child, so P gets the click; the runtime drops it");
    p = add(0, "P", "csolid", WS_POPUP | WS_VISIBLE, 40, 40, 300, 200, 0);
    c1 = add(1, "D", "csolid", WS_CHILD | WS_VISIBLE, 20, 20, 100, 80, p);
    enablewin(c1, 0);
    g_mute = 1; pump(); g_mute = 0;
    step(" click (80,80): in D");
    click(80, 80);
    clearall();
    step("T5 (same) a disabled top-level pop-up (DP): Wine 10.0 answers the hit with HTERROR and sends WM_SETCURSOR to it; the runtime drops the event");
    a = add(0, "DP", "csolid", WS_POPUP | WS_VISIBLE, 20, 20, 200, 150, 0);
    enablewin(a, 0);
    g_mute = 1; pump(); g_mute = 0;
    step(" click (100,80): in DP");
    click(100, 80);
    clearall();
}

static void sc_H(void)
{
    HWND w, v;
    int k;
    printf("#### scenario H: WH_MOUSE and WH_KEYBOARD\n");
    w = add(0, "W", "csolid", WS_POPUP | WS_VISIBLE, 30, 30, 200, 150, 0);
    v = add(1, "V", "csolid", WS_POPUP | WS_VISIBLE, 300, 30, 100, 100, 0);
    (void)v;
    g_mute = 1; pump(); (void)setfocus(w); pump(); g_mute = 0;
    g_hh[0] = hook_install(WH_MOUSE, 0);
    g_hh[2] = hook_install(WH_KEYBOARD, 1);
    step("H1 click on W with a mouse hook that lets everything pass");
    click(100, 80);
    step("H2 the key A down and up with a keyboard hook that lets everything pass");
    key(1, 0x41, 0x1e); pump();
    key(0, 0x41, 0x1e); pump();
    step("H3 the mouse hook answers 1 for WM_LBUTTONDOWN: W does not get it");
    g_hook_swallow_mouse = 0x0201;
    click(100, 80);
    g_hook_swallow_mouse = 0;
    step("H4 the keyboard hook answers 1 for VK_A: W does not get the key down or up");
    g_hook_swallow_key = 0x41;
    key(1, 0x41, 0x1e); pump();
    key(0, 0x41, 0x1e); pump();
    g_hook_swallow_key = 0;
    step("H5 PeekMessage with PM_NOREMOVE (HC_NOREMOVE), then with PM_REMOVE (HC_ACTION)");
    mouse(1, 120, 90);
    k = peek(0); printf("  peek(noremove) -> %#x\n", k);
    k = peek(1); printf("  peek(remove) -> %#x\n", k);
    step("H6 the mouse hook is removed: no hook line");
    hook_remove(g_hh[0]); g_hh[0] = 0;
    click(100, 80);
    hook_remove(g_hh[2]); g_hh[2] = 0;
    clearall();
}

#ifndef C0323_WINE
/* scenario N (the runtime only: Wine's null driver has no window surface to read): the pixels of a window DC after the erase. The class brush colour is 0x563412; the window is first filled with 0x404040. */
static void pixels_case(const char *label, const char *cls, const char *name, int how)
{
    HWND w;
    DC_ dc;
    w = add(0, name, cls, WS_POPUP | WS_VISIBLE, 30, 30, 120, 90, 0);
    g_mute = 1; pump(); update(w); pump(); g_mute = 0;
    dc = API(GetDC, w, 0, 0, 0, 0, 0, 0, 0);
    fillrect(dc, 0, 0, 120, 90, 0x00404040u);
    (void)API(ReleaseDC, w, dc, 0, 0, 0, 0, 0, 0);
    g_mute = 1;
    if (how == 0) { invalidate(w, 10, 10, 50, 40, 1); update(w); }
    else if (how == 1) { redraw(w, 10, 10, 50, 40, RDW_INVALIDATE | RDW_ERASE | RDW_ERASENOW); }
    else { invalidate(w, 10, 10, 50, 40, 0); update(w); }
    pump(); g_mute = 0;
    dc = API(GetDC, w, 0, 0, 0, 0, 0, 0, 0);
    printf("%s: inside (10,10)=%06x (20,20)=%06x (49,39)=%06x; outside (9,10)=%06x (50,39)=%06x (10,40)=%06x (60,60)=%06x\n", label, getpixel(dc, 10, 10), getpixel(dc, 20, 20), getpixel(dc, 49, 39), getpixel(dc, 9, 10), getpixel(dc, 50, 39), getpixel(dc, 10, 40), getpixel(dc, 60, 60));
    (void)API(ReleaseDC, w, dc, 0, 0, 0, 0, 0, 0);
    clearall();
}

static void sc_N(void)
{
    printf("#### scenario N (runtime only): pixels of a window after the erase of the update rectangle (10,10,50,40)\n");
    pixels_case("N1 default WM_PAINT, class with a brush", "csolid", "WS", 0);
    pixels_case("N2 RDW_ERASENOW, class with a brush", "csolid", "C4", 1);
    pixels_case("N3 own BeginPaint, class with a brush", "csolid", "C1", 0);
    pixels_case("N4 default WM_PAINT, class without a brush", "cnone", "WN", 0);
    pixels_case("N5 bErase FALSE, class with a brush", "csolid", "C3", 2);
    pixels_case("N6 default WM_PAINT, COLOR_WINDOW + 1", "cwin", "WW", 0);
}
#endif

int main(int argc, char **argv)
{
    char only = argc > 1 ? argv[1][0] : 0;
    setvbuf(stdout, NULL, _IONBF, 0);
#ifndef C0323_WINE
    rt_init();
#endif
    classes();
    if (!only || only == 'E') sc_E();
    if (!only || only == 'P') sc_P();
    if (!only || only == 'T') sc_T();
    if (!only || only == 'H') sc_H();
#ifndef C0323_WINE
    if (only == 'N') sc_N();
#endif
    (void)slot_of;
    return 0;
}

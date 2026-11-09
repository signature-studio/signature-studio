/* Decision 0262 (alpha8, test tool; the scaffold of harness/alpha8/move_paint.c): (1) the move of a child window (SetWindowPos(SWP_NOZORDER | SWP_NOACTIVATE), the way FL drags its Mixer panel TFXForm, decision 0240) copies the bits of the
 * window and invalidates only what the move uncovered, even when a hidden window in the tree still holds an update rectangle from the time it was visible (FL's TQuickScrollBoxScroller, lid 87: shown, sized and hidden again before any
 * paint). Wine 10.0 (harness/alpha8/move_paint.c, UNOFFICIAL): after the hide only the parent's area is invalid and a move leaves only the strip of the parent that it uncovered. (2) RedrawWindow(RDW_INVALIDATE | RDW_NOCHILDREN) leaves the
 * children of a window without WS_CLIPCHILDREN alone (Learn RedrawWindow: "RDW_NOCHILDREN: Excludes child windows, if any, from the repainting operation"); the SDK value of RDW_NOCHILDREN is 0x40 and of RDW_NOFRAME 0x800 (the runtime had
 * them swapped). The tree: the real FL rectangles of the Mixer drag scene (lid 16 > { 17, 53, 64 > { 85, 86 > 87, 83 } }). Prints one line per check ("ok ..." / "FAIL ...") and ends with "checks N failed M". */
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
#include "fle_thread.h"
#include "fle_cursor.h"
#include "fle_wall_ia2.h"
#include "fle_api2.h"
#include "fle_raster.h"

uint64_t R[16];
fle_eflags FL;
fle_xmm X[16];
void fle_imp_PeekMessageW(void);
void fle_imp_DispatchMessageW(void);
typedef uint64_t HWND;
typedef unsigned int DWORD, UINT;
typedef int BOOL;
#define A(x) ((uint64_t)(uintptr_t)(x))
#define VA_WP 0x2000
#define WS_POPUP 0x80000000u
#define WS_CHILD 0x40000000u
#define WS_VISIBLE 0x10000000u
#define WS_CLIPSIBLINGS 0x04000000u
#define WS_CLIPCHILDREN 0x02000000u

#define NWIN 10
static HWND g_w[NWIN];
static const char *g_n[NWIN] = { "MAIN", "WP", "RACK", "PLAY", "MIX", "CAP", "PANA", "PANB", "HID", "SUB" };
static int painted[NWIN];

static void wndproc(void)
{
    uint64_t hw = R[1], msg = R[2], wp_ = R[8], lp = R[9];

    if (msg == 0x14) { R[0] = 1; return; }
    if (msg == 0x0F) {
        uint8_t ps[72];
        int k;

        memset(ps, 0, sizeof ps);
        (void)fle_api2_call(fle_imp_BeginPaint, hw, A(ps), 0, 0, 0, 0, 0, 0);
        for (k = 0; k < NWIN; k++) if (g_w[k] == hw) painted[k]++;
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
    uint64_t h, sp0 = R[4];
    uint16_t empty[1] = { 0 };

    R[4] = (R[4] - 0x80) & ~UINT64_C(15);
    R[4] -= 8;
    fle_st(R[4], 8, 0xAAAA0000);
    fle_st(R[4] + 0x28, 8, (uint32_t)x); fle_st(R[4] + 0x30, 8, (uint32_t)y); fle_st(R[4] + 0x38, 8, (uint32_t)cx); fle_st(R[4] + 0x40, 8, (uint32_t)cy);
    fle_st(R[4] + 0x48, 8, parent); fle_st(R[4] + 0x50, 8, 0); fle_st(R[4] + 0x58, 8, 0x400000); fle_st(R[4] + 0x60, 8, 0);
    R[1] = 0; R[2] = A(clsname); R[8] = A(empty); R[9] = style;
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
static BOOL SetWindowPos_(HWND h, HWND after, int x, int y, int cx, int cy, UINT f) { return (BOOL)fle_api2_call(fle_imp_SetWindowPos, h, after, (uint32_t)x, (uint32_t)y, (uint32_t)cx, (uint32_t)cy, f, 0); }
static BOOL ShowWindow_(HWND h, int c) { return (BOOL)fle_api2_call(fle_imp_ShowWindow, h, (uint32_t)c, 0, 0, 0, 0, 0, 0); }
static int uprect(HWND h, int32_t r[4]) { r[0] = r[1] = r[2] = r[3] = 0; return (int)(fle_api2_call(fle_imp_GetUpdateRect, h, A(r), 0, 0, 0, 0, 0, 0) & 1); }
static void redraw(HWND h, uint32_t flags) { (void)fle_api2_call(fle_imp_RedrawWindow, h, 0, 0, flags, 0, 0, 0, 0); }

static int nchecks, nfail;
static void yes(const char *what, int ok)
{
    nchecks++;
    if (!ok) nfail++;
    printf("%s %s\n", ok ? "ok" : "FAIL", what);
}
static int is_rect(HWND h, int l, int t, int r, int b)
{
    int32_t q[4];

    return uprect(h, q) && q[0] == l && q[1] == t && q[2] == r && q[3] == b;
}
static int clean_(HWND h) { int32_t q[4]; return !uprect(h, q); }
static int keep_hid;   /* 1: clean_all leaves HID alone (its stale rectangle is the point of the first scenario) */
static void clean_all(void)
{
    int k;

    for (k = 0; k < NWIN; k++) if (!(keep_hid && k == 8)) (void)fle_api2_call(fle_imp_ValidateRect, g_w[k], 0, 0, 0, 0, 0, 0, 0);
}
static void dump(const char *tag)
{
    int k;

    for (k = 0; k < NWIN; k++) {
        int32_t q[4];

        if (g_w[k] && uprect(g_w[k], q)) printf("    [%s] %s update (%d,%d,%d,%d)\n", tag, g_n[k], q[0], q[1], q[2], q[3]);
    }
}
#define CH (WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS)
#define CHC (WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | WS_CLIPCHILDREN)

int main(void)
{
    char buf[160];
    int m, k, only_wp;

    setvbuf(stdout, NULL, _IONBF, 0);
    regclass();
    if (!fle_raster_window_surface(1, 1930, 1092)) { printf("FAIL no surface\n"); return 1; }
    g_w[0] = mk(WS_POPUP | WS_VISIBLE, -5, -6, 1930, 1092, 0);
    g_w[1] = mk(CH, 378, 88, 1546, 998, g_w[0]);
    g_w[2] = mk(CH, 0, 0, 499, 126, g_w[1]);
    g_w[3] = mk(CH, 499, 0, 1047, 998, g_w[1]);
    g_w[4] = mk(CH, 16, 466, 1514, 500, g_w[1]);
    g_w[5] = mk(CHC, 4, 4, 1506, 23, g_w[4]);
    g_w[6] = mk(CHC, 105, 28, 1167, 465, g_w[4]);
    g_w[7] = mk(CHC, 1287, 28, 220, 465, g_w[4]);
    g_w[9] = mk(CHC, 0, 32, 220, 191, g_w[7]);
    g_w[8] = mk(0x46000000u, 0, 446, 1211, 19, g_w[6]);
    SetWindowPos_(g_w[4], 0, 0, 0, 0, 0, 0x13);      /* the Mixer is above the Playlist */
    pump();
    clean_all();
    memset(painted, 0, sizeof painted);

    /* ---- 1. a window that is hidden holds no pending area ---- */
    SetWindowPos_(g_w[8], 0, 0, 0, 0, 0, 0x57);                 /* SWP_SHOWWINDOW | NOMOVE | NOSIZE | NOZORDER | NOACTIVATE */
    SetWindowPos_(g_w[8], 0, 0, 446, 1254, 19, 0x114);          /* the size change invalidates the whole client area of the visible window */
    yes("the shown and sized HID is invalid (that is where the stale rectangle comes from)", !clean_(g_w[8]));
    SetWindowPos_(g_w[8], 0, 0, 0, 0, 0, 0x97);                 /* SWP_HIDEWINDOW before any paint, as FL does */
    keep_hid = 1;
    pump();
    printf("note: the hidden HID %s an update rectangle (Wine 10.0 drops it at the hide, the runtime keeps it; what the moves below check does not depend on that)\n", clean_(g_w[8]) ? "holds no longer" : "still holds");
    yes("after the hide the parent PANA is invalid where HID was and is painted", painted[6] == 1);
    clean_all();
    memset(painted, 0, sizeof painted);

    for (m = 0; m < 3; m++) {
        int ok_wp;

        snprintf(buf, sizeof buf, "move %d (+15 px): SetWindowPos(MIX, %d, 466, 1514 x 500, NOZORDER | NOACTIVATE)", m + 1, 31 + 15 * m);
        SetWindowPos_(g_w[4], 0, 31 + 15 * m, 466, 1514, 500, 0x14);
        ok_wp = is_rect(g_w[1], 16 + 15 * m, 466, 31 + 15 * m, 966);
        yes(buf, 1);
        yes("  WP: the strip of 15 x 500 that the move uncovered is invalid", ok_wp);
        yes("  MIX: its bits were copied, nothing of it is invalid", clean_(g_w[4]));
        yes("  CAP, PANA, PANB: nothing is invalid (the stale rectangle of the hidden HID did not stop the copy)", clean_(g_w[5]) && clean_(g_w[6]) && clean_(g_w[7]));
        yes("  SUB: nothing is invalid", clean_(g_w[9]));
        if (!ok_wp) dump("move");
        pump();
        for (k = 0, only_wp = 1; k < NWIN; k++) if (k != 1 && painted[k]) only_wp = 0;
        yes("  only WP is painted", only_wp && painted[1] == m + 1);
        clean_all();
    }

    /* ---- 2. RDW_NOCHILDREN is 0x40, RDW_NOFRAME 0x800 ---- */
    keep_hid = 0;
    clean_all();
    redraw(g_w[4], 0x1u);
    yes("RedrawWindow(MIX, RDW_INVALIDATE): MIX has no WS_CLIPCHILDREN, so its children are reached (Learn RedrawWindow: the default)", !clean_(g_w[4]) && is_rect(g_w[5], 0, 0, 1506, 23));
    clean_all();
    redraw(g_w[4], 0x1u | 0x40u);
    yes("RedrawWindow(MIX, RDW_INVALIDATE | RDW_NOCHILDREN = 0x41): MIX is invalid, its children are left alone", is_rect(g_w[4], 0, 0, 1514, 500) && clean_(g_w[5]) && clean_(g_w[6]) && clean_(g_w[7]));
    clean_all();
    redraw(g_w[4], 0x1u | 0x800u);
    yes("RedrawWindow(MIX, RDW_INVALIDATE | RDW_NOFRAME = 0x801): NOFRAME is not NOCHILDREN, the children are reached", is_rect(g_w[5], 0, 0, 1506, 23));
    clean_all();
    redraw(g_w[4], 0x1u | 0x80u);
    yes("RedrawWindow(MIX, RDW_INVALIDATE | RDW_ALLCHILDREN) still reaches the children", is_rect(g_w[5], 0, 0, 1506, 23));
    clean_all();

    printf("checks %d failed %d\n", nchecks, nfail);
    return nfail ? 1 : 0;
}

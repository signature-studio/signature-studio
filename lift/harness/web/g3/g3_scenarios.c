/* Scenarios of decision 0136 (docs/decisions/0136-input-roundtrip-and-timers.md, PROVISIONAL): the input round trip and the idle clock, with the REAL runtime files (fle_input.c, fle_comctl.c, fle_win.c, fle_class.c,
 * fle_thread.c, fle_sys.c, fle_imports.c, fle_callback.c ...) and STUB guest functions (a C function in a replacement fle_dispatch_table is a "guest function": the lifted engine is not involved). Native only
 * (tests/wall_fr14.rs builds and runs it and compares each scenario's stdout with the sequence written by hand from the Learn pages and the choices of the decision).
 *
 * usage: <program> list | <scenario name>
 * Every event goes to stdout (unbuffered). Nothing printed may depend on the host: no host address, no host time. */
#define _GNU_SOURCE
#include <stdint.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "fle_runtime.h"
#include "fle_sse.h"
#include "fle_dispatch.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_modules.h"
#include "fle_callback.h"
#include "fle_class.h"
#include "fle_sys.h"
#include "fle_proc.h"
#include "fle_event.h"
#include "fle_win.h"
#include "fle_thread.h"
#include "fle_input.h"
#include "fle_cursor.h"
#include "fle_vfs.h"
#include "fle_locale.h"
#include "fle_sysinfo.h"
#include "fle_x87ctl.h"
#include "fle_mxcsr.h"

void fle_sys_clock_advance_to(uint64_t ticks);
void fle_imp__TrackMouseEvent(void);
void fle_imp_GetKeyboardState(void);
void fle_imp_MapVirtualKeyW(void);
void fle_imp_GetMessagePos(void);
void fle_imp_GetMessageTime(void);
void fle_imp_GetKeyState(void);
void fle_imp_GetAsyncKeyState(void);
void fle_imp_GetCursorPos(void);
void fle_imp_SetCapture(void);
void fle_imp_ReleaseCapture(void);
void fle_imp_EnableWindow(void);
void fle_imp_TranslateMessage(void);
void fle_imp_SetTimer(void);
void fle_imp_KillTimer(void);
void fle_imp_ShowWindow(void);
void fle_imp_SetFocus(void);
void fle_imp_SetForegroundWindow(void);
void fle_imp_SetActiveWindow(void);
void fle_imp_GetActiveWindow(void);
void fle_imp_OpenProcessToken(void);
void fle_imp_GetTokenInformation(void);
void fle_imp_CloseHandle(void);
void fle_imp_GetLastError(void);
void fle_imp_CompareStringW(void);

uint64_t R[16];
fle_eflags FL;
fle_xmm X[16];
static uint64_t stk[0x4000] __attribute__((aligned(4096)));
#define E0 ((uint64_t)(uintptr_t)stk + 0x10000 + 8)
#define RET0 UINT64_C(0xdead0000)
#define U(x) ((unsigned long long)(x))
#define A(x) ((uint64_t)(uintptr_t)(x))

#define CALL4(f, a, b, c, d) do { R[1] = (a); R[2] = (b); R[8] = (c); R[9] = (d); R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000); f(); R[4] += 8; } while (0)
#define CALL5(f, a, b, c, d, e) do { uint64_t sp0_ = R[4]; R[4] = (R[4] - 0x40) & ~UINT64_C(15); R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000); fle_st(R[4] + 0x28, 8, (e)); \
    R[1] = (a); R[2] = (b); R[8] = (c); R[9] = (d); f(); R[4] = sp0_; } while (0)
#define CALL6(f, a, b, c, d, e, g) do { uint64_t sp0_ = R[4]; R[4] = (R[4] - 0x40) & ~UINT64_C(15); R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000); fle_st(R[4] + 0x28, 8, (e)); fle_st(R[4] + 0x30, 8, (g)); \
    R[1] = (a); R[2] = (b); R[8] = (c); R[9] = (d); f(); R[4] = sp0_; } while (0)
static uint32_t CLOCK_MS(void) { return (uint32_t)(fle_sys_clock_ticks() / FLE_TICKS_PER_MS); }
static void SLEEP(uint32_t ms) { CALL4(fle_imp_Sleep, ms, 0, 0, 0); }

#define VA_WPA 0x2000
#define VA_WPB 0x2010
static int logging = 1;
static uint64_t g_a, g_b, g_c, g_child;      /* the windows */
static uint64_t cur_pt(void) { return 0; }
static const char *nm(uint64_t h) { return h == g_a ? "A" : h == g_b ? "B" : h == g_c ? "C" : h == g_child ? "child" : h == 0 ? "-" : "?"; }
/* what the stub window procedures do with a message they are sent: DefWindowProcW for the messages of the input path (so that the default processing is the runtime's), the answers Learn gives for the creation messages */
static int absorb_nchittest, absorb_setcursor_true, activate_mode = 0;
static int hit_override = 0x7FFF;       /* window B answers WM_NCHITTEST with this value when it is not 0x7FFF */
static void wndproc_common(int which)
{
    uint64_t hw = R[1], msg = R[2], wp = R[8], lp = R[9];

    if (msg == 0x81) { R[0] = 1; return; }
    if (msg == 0x83 || msg == 0x01) { R[0] = 0; return; }
    if (logging && msg != 0x46 && msg != 0x47 && msg != 0x05 && msg != 0x03 && msg != 0x18 && msg != 0x0f && msg != 0x14 && msg != 0x85 && msg != 0x7f && msg != 0x0b) {
        if (msg == 0x84) printf("  %s msg 84 NCHITTEST x %d y %d\n", nm(hw), (int)(int16_t)(uint16_t)lp, (int)(int16_t)(uint16_t)(lp >> 16));
        else if (msg == 0x20) printf("  %s msg 20 SETCURSOR wp %s hit %d mouse %llx\n", nm(hw), nm(wp), (int)(int16_t)(uint16_t)lp, U(lp >> 16));
        else if (msg == 0x21) printf("  %s msg 21 MOUSEACTIVATE top %s hit %d mouse %llx\n", nm(hw), nm(wp), (int)(int16_t)(uint16_t)lp, U(lp >> 16));
        else if (msg >= 0x200 && msg <= 0x20a) printf("  %s msg %llx wp %llx x %d y %d\n", nm(hw), U(msg), U(wp), (int)(int16_t)(uint16_t)lp, (int)(int16_t)(uint16_t)(lp >> 16));
        else if (msg == 0x2a3 || msg == 0x2a1 || msg == 0x2a2) printf("  %s msg %llx wp %llx lp %llx\n", nm(hw), U(msg), U(wp), U(lp));
        else printf("  %s msg %llx wp %llx lp %llx\n", nm(hw), U(msg), U(wp), U(lp));
    }
    if (which == 1 && msg == 0x84 && hit_override != 0x7FFF) { R[0] = (uint64_t)(int64_t)hit_override; return; }
    if (which == 1 && msg == 0x21 && activate_mode) { R[0] = (uint64_t)activate_mode; return; }
    if (msg == 0x84 || msg == 0x20 || msg == 0x21 || (msg >= 0x100 && msg <= 0x109) || (msg >= 0x200 && msg <= 0x20a) || msg == 0x2a3 || msg == 0x2a1 || msg == 0x2a2 || msg == 0x112 || msg == 0x10) {
        CALL4(fle_imp_DefWindowProcW, hw, msg, wp, lp);
        if (logging && (msg == 0x84 || msg == 0x20 || msg == 0x21)) printf("    -> %lld\n", (long long)(int64_t)R[0]);
        return;
    }
    R[0] = 0;
}
static void wndproc_a(void) { wndproc_common(0); }
static void wndproc_b(void) { wndproc_common(1); }

/* a hook the scheduler calls at idle (decision 0136): the scenario says what the "host" does */
static uint64_t idle_script_target;
static int idle_calls, idle_jump;
int fle_thread_idle_hook(uint64_t now, uint64_t next, uint64_t *adv)
{
    idle_calls++;
    if (idle_calls > 200) return 0;     /* a bound for the mutation runs of the test: a scheduler that does not move the clock must end, not spin */
    printf("  idle now %llu next %llu\n", U(now / FLE_TICKS_PER_MS), U(next / FLE_TICKS_PER_MS));
    if (idle_jump && next) { *adv = next; return 2; }
    if (idle_script_target) { *adv = idle_script_target; return 2; }
    return 0;
}

const fle_fn_entry fle_dispatch_table[] = {
    { VA_WPA, wndproc_a },
    { VA_WPB, wndproc_b },
};
const uint64_t fle_dispatch_count = sizeof fle_dispatch_table / sizeof fle_dispatch_table[0];

static uint16_t cls_a[] = { 'G','3','A', 0 }, cls_b[] = { 'G','3','B', 0 };
static void regcls(uint16_t *name, uint64_t proc, uint32_t style)
{
    static uint8_t wc[72];
    uint64_t inst = 0x400000, nmp = A(name), cur = FLE_CURSOR_HANDLE_BASE + 4u;

    memset(wc, 0, sizeof wc);
    memcpy(wc, &style, 4);
    memcpy(wc + 8, &proc, 8);
    memcpy(wc + 24, &inst, 8);
    memcpy(wc + 40, &cur, 8);
    memcpy(wc + 64, &nmp, 8);
    CALL4(fle_imp_RegisterClassW, A(wc), 0, 0, 0);
}
/* CreateWindowExW(0, class, "", style, x, y, cx, cy, parent, 0, inst, 0) */
static uint64_t mkwin(uint16_t *cls, uint32_t style, int x, int y, int cx, int cy, uint64_t parent)
{
    uint64_t h, sp0 = R[4];
    uint16_t empty[1] = { 0 };

    R[4] = (R[4] - 0x80) & ~UINT64_C(15);
    R[4] -= 8;
    fle_st(R[4], 8, 0xAAAA0000);
    fle_st(R[4] + 0x28, 8, (uint32_t)x); fle_st(R[4] + 0x30, 8, (uint32_t)y); fle_st(R[4] + 0x38, 8, (uint32_t)cx); fle_st(R[4] + 0x40, 8, (uint32_t)cy);
    fle_st(R[4] + 0x48, 8, parent); fle_st(R[4] + 0x50, 8, 0); fle_st(R[4] + 0x58, 8, 0x400000); fle_st(R[4] + 0x60, 8, 0);
    R[1] = 0; R[2] = A(cls); R[8] = A(empty); R[9] = style;
    fle_imp_CreateWindowExW();
    h = R[0];
    R[4] = sp0;
    return h;
}
#define WS_VISIBLE_ 0x10000000u
#define WS_POPUP_ 0x80000000u
#define WS_CHILD_ 0x40000000u

typedef struct { uint64_t hwnd; uint32_t msg; uint32_t pad; uint64_t wp, lp; uint32_t time; int32_t ptx, pty; uint32_t priv; } MSGX;
_Static_assert(sizeof(MSGX) == FLE_MSG_SIZE, "sizeof MSG");
static uint32_t PEEK(MSGX *m, uint64_t h, uint32_t lo, uint32_t hi, uint32_t rm) { CALL5(fle_imp_PeekMessageW, A(m), h, lo, hi, rm); return (uint32_t)R[0]; }
static uint32_t GET(MSGX *m) { CALL4(fle_imp_GetMessageW, A(m), 0, 0, 0); return (uint32_t)R[0]; }
static void SETCAP(uint64_t h) { CALL4(fle_imp_SetCapture, h, 0, 0, 0); }
static void RELCAP(void) { CALL4(fle_imp_ReleaseCapture, 0, 0, 0, 0); }
static void ENABLE(uint64_t h, int on) { CALL4(fle_imp_EnableWindow, h, on, 0, 0); }
static void TRANSLATE(MSGX *m) { CALL4(fle_imp_TranslateMessage, A(m), 0, 0, 0); }
static void DISPATCH(MSGX *m) { CALL4(fle_imp_DispatchMessageW, A(m), 0, 0, 0); }

/* the message loop of the scenarios: every message that is available now is retrieved (PeekMessage, PM_REMOVE), TranslateMessage'd and dispatched; returns how many */
static int pump(void)
{
    MSGX m;
    int n = 0;
    while (PEEK(&m, 0, 0, 0, 1)) {
        n++;
        if (m.msg == 0x113) { DISPATCH(&m); continue; }
        TRANSLATE(&m);
        DISPATCH(&m);
    }
    return n;
}

static void boot(void)
{
    R[4] = E0;
    fle_st(E0, 8, RET0);
    fle_teb_set_stack(A(stk), A(stk) + sizeof stk);
    regcls(cls_a, VA_WPA, 0);
    regcls(cls_b, VA_WPB, 0);
}
static void world(void)
{
    /* A: pop-up (100,100) 300 x 200, visible; B: pop-up (500,100) 200 x 200 visible, class CS_DBLCLKS-less; child of A at (10,10) 100 x 50 */
    uint16_t cc[] = { 'G','3','C', 0 };

    regcls(cc, VA_WPA, 0x8);                 /* CS_DBLCLKS */
    g_a = mkwin(cls_a, WS_POPUP_ | WS_VISIBLE_, 100, 100, 300, 200, 0);
    g_b = mkwin(cls_b, WS_POPUP_ | WS_VISIBLE_, 500, 100, 200, 200, 0);
    g_child = mkwin(cls_a, WS_CHILD_ | WS_VISIBLE_, 10, 10, 100, 50, g_a);
    g_c = mkwin(cc, WS_POPUP_ | WS_VISIBLE_, 100, 400, 100, 100, 0);
    pump();
}

/* ===================================== scenarios ===================================== */
static void sc_move(void)
{
    world();
    printf("move to (150,130):\n");
    fle_input_host_mouse(FLE_IN_MOVE, 150, 130, 0);
    printf("pending %u\n", fle_input_host_pending());
    printf("pump %d\n", pump());
    printf("move to (150,130) is in A, outside the child (A client origin 100,100, child at 110..210 x 110..160 screen: x 150 y 130 is inside the child)\n");
    fle_imp_GetCursorPos();
}
static void sc_coalesce(void)
{
    world();
    fle_input_host_mouse(FLE_IN_MOVE, 300, 250, 0);
    fle_input_host_mouse(FLE_IN_MOVE, 310, 260, 0);
    fle_input_host_mouse(FLE_IN_MOVE, 320, 270, 0);
    printf("pump %d\n", pump());
    uint64_t p = A(&stk[0x100]);
    CALL4(fle_imp_GetCursorPos, p, 0, 0, 0);
    printf("cursor %u %u\n", (unsigned)fle_ld(p, 4), (unsigned)fle_ld(p + 4, 4));
}
static void sc_click(void)
{
    world();
    printf("click on A at (300,250):\n");
    fle_input_host_mouse(FLE_IN_MOVE, 300, 250, 0);
    fle_input_host_mouse(FLE_IN_BDOWN, 300, 250, FLE_IN_BTN_L);
    CALL4(fle_imp_GetKeyState, 1, 0, 0, 0);
    printf("GetKeyState(VK_LBUTTON) before the message: %llx\n", U(R[0]));
    CALL4(fle_imp_GetAsyncKeyState, 1, 0, 0, 0);
    printf("GetAsyncKeyState(VK_LBUTTON) after the host event: %llx\n", U(R[0]));
    printf("pump %d\n", pump());
    CALL4(fle_imp_GetKeyState, 1, 0, 0, 0);
    printf("GetKeyState(VK_LBUTTON) after: %llx\n", U(R[0]));
    fle_input_host_mouse(FLE_IN_BUP, 300, 250, FLE_IN_BTN_L);
    printf("pump %d\n", pump());
    CALL4(fle_imp_GetKeyState, 1, 0, 0, 0);
    printf("GetKeyState(VK_LBUTTON) after the release: %llx\n", U(R[0]));
    CALL4(fle_imp_GetActiveWindow, 0, 0, 0, 0);
    printf("active %s\n", nm(R[0]));
}
static void sc_capture(void)
{
    world();
    SETCAP(g_b);
    printf("captured by B; click on A at (300,250):\n");
    fle_input_host_mouse(FLE_IN_MOVE, 300, 250, 0);
    fle_input_host_mouse(FLE_IN_BDOWN, 300, 250, FLE_IN_BTN_L);
    printf("pump %d\n", pump());
    RELCAP();
    printf("released\n");
    fle_input_host_mouse(FLE_IN_BUP, 300, 250, FLE_IN_BTN_L);
    printf("pump %d\n", pump());
}
static void sc_disabled(void)
{
    world();
    ENABLE(g_a, 0);
    printf("A is disabled; click on A at (300,250):\n");
    fle_input_host_mouse(FLE_IN_MOVE, 300, 250, 0);
    fle_input_host_mouse(FLE_IN_BDOWN, 300, 250, FLE_IN_BTN_L);
    fle_input_host_mouse(FLE_IN_BUP, 300, 250, FLE_IN_BTN_L);
    printf("pump %d\n", pump());
}
static void sc_dblclk(void)
{
    world();
    printf("C has CS_DBLCLKS; two clicks 100 ms apart, then a third after 600 ms:\n");
    fle_input_host_mouse(FLE_IN_MOVE, 120, 420, 0);
    fle_input_host_mouse(FLE_IN_BDOWN, 120, 420, FLE_IN_BTN_L);
    fle_input_host_mouse(FLE_IN_BUP, 120, 420, FLE_IN_BTN_L);
    pump();
    fle_sys_clock_advance_to(fle_sys_clock_ticks() + 100u * FLE_TICKS_PER_MS);
    fle_input_host_mouse(FLE_IN_BDOWN, 120, 420, FLE_IN_BTN_L);
    fle_input_host_mouse(FLE_IN_BUP, 120, 420, FLE_IN_BTN_L);
    pump();
    fle_sys_clock_advance_to(fle_sys_clock_ticks() + 600u * FLE_TICKS_PER_MS);
    fle_input_host_mouse(FLE_IN_BDOWN, 120, 420, FLE_IN_BTN_L);
    fle_input_host_mouse(FLE_IN_BUP, 120, 420, FLE_IN_BTN_L);
    pump();
}
static void sc_hit_transparent(void)
{
    world();
    hit_override = FLE_HTTRANSPARENT;
    printf("B answers HTTRANSPARENT; move to (600,200):\n");
    fle_input_host_mouse(FLE_IN_MOVE, 600, 200, 0);
    printf("pump %d\n", pump());
    hit_override = 0x7FFF;
}
static void sc_mouseactivate(void)
{
    world();
    activate_mode = 2;
    printf("B answers MA_ACTIVATEANDEAT; press on B at (600,200):\n");
    fle_input_host_mouse(FLE_IN_MOVE, 600, 200, 0);
    fle_input_host_mouse(FLE_IN_BDOWN, 600, 200, FLE_IN_BTN_L);
    printf("pump %d\n", pump());
    CALL4(fle_imp_GetActiveWindow, 0, 0, 0, 0);
    printf("active %s\n", nm(R[0]));
    fle_input_host_mouse(FLE_IN_BUP, 600, 200, FLE_IN_BTN_L);
    pump();
}
static void sc_mouseactivate_no(void)
{
    world();
    CALL4(fle_imp_SetActiveWindow, g_a, 0, 0, 0);
    pump();
    activate_mode = 3;
    printf("A is active; B answers MA_NOACTIVATE; press on B at (600,200):\n");
    fle_input_host_mouse(FLE_IN_MOVE, 600, 200, 0);
    fle_input_host_mouse(FLE_IN_BDOWN, 600, 200, FLE_IN_BTN_L);
    printf("pump %d\n", pump());
    CALL4(fle_imp_GetActiveWindow, 0, 0, 0, 0);
    printf("active %s\n", nm(R[0]));
    activate_mode = 0;
    printf("press on A (already active): MOUSEACTIVATE is not asked\n");
    fle_input_host_mouse(FLE_IN_MOVE, 300, 250, 0);
    fle_input_host_mouse(FLE_IN_BDOWN, 300, 250, FLE_IN_BTN_L);
    printf("pump %d\n", pump());
}

/* ---- TrackMouseEvent ---- */
static uint64_t tme(uint32_t flags, uint64_t h, uint32_t hover, uint32_t *ret)
{
    uint64_t p = A(&stk[0x200]);
    fle_st(p, 4, 24); fle_st(p + 4, 4, flags); fle_st(p + 8, 8, h); fle_st(p + 16, 4, hover);
    CALL4(fle_imp__TrackMouseEvent, p, 0, 0, 0);
    if (ret) *ret = (uint32_t)R[0];
    return p;
}
static void sc_track_leave(void)
{
    uint32_t r;
    uint64_t p;
    world();
    fle_input_host_mouse(FLE_IN_MOVE, 600, 200, 0);
    pump();
    printf("pointer over B; TME_LEAVE on B:\n");
    p = tme(FLE_TME_LEAVE, g_b, 0, &r);
    printf("ret %u\n", r);
    p = tme(FLE_TME_QUERY, g_b, 0, &r);
    printf("query: ret %u flags %llx hwnd %s hover %llu\n", r, U(fle_ld(p + 4, 4)), nm(fle_ld(p + 8, 8)), U(fle_ld(p + 16, 4)));
    printf("pump (nothing yet) %d\n", pump());
    printf("move to A (300,250):\n");
    fle_input_host_mouse(FLE_IN_MOVE, 300, 250, 0);
    printf("pump %d\n", pump());
    p = tme(FLE_TME_QUERY, g_b, 0, &r);
    printf("query after the leave: flags %llx\n", U(fle_ld(p + 4, 4)));
    printf("TME_LEAVE on B while the pointer is over A: leave at once\n");
    tme(FLE_TME_LEAVE, g_b, 0, &r);
    printf("pump %d\n", pump());
}
static void sc_track_hover(void)
{
    uint32_t r;
    uint64_t p;
    world();
    fle_input_host_mouse(FLE_IN_MOVE, 600, 200, 0);
    pump();
    printf("TME_HOVER with HOVER_DEFAULT on B at %u ms:\n", CLOCK_MS());
    p = tme(FLE_TME_HOVER, g_b, FLE_HOVER_DEFAULT, &r);
    printf("ret %u\n", r);
    p = tme(FLE_TME_QUERY, g_b, 0, &r);
    printf("query: flags %llx hover %llu\n", U(fle_ld(p + 4, 4)), U(fle_ld(p + 16, 4)));
    fle_sys_clock_advance_to(fle_sys_clock_ticks() + 399u * FLE_TICKS_PER_MS);
    printf("399 ms later: pump %d\n", pump());
    fle_sys_clock_advance_to(fle_sys_clock_ticks() + 1u * FLE_TICKS_PER_MS);
    printf("400 ms later (%u ms): pump %d\n", CLOCK_MS(), pump());
    p = tme(FLE_TME_QUERY, g_b, 0, &r);
    printf("query: flags %llx (hover tracking stopped)\n", U(fle_ld(p + 4, 4)));
    printf("again, 100 ms hover; a move of 2 px stays inside the rectangle, a move of 20 px restarts the time:\n");
    tme(FLE_TME_HOVER, g_b, 100, &r);
    fle_sys_clock_advance_to(fle_sys_clock_ticks() + 60u * FLE_TICKS_PER_MS);
    fle_input_host_mouse(FLE_IN_MOVE, 602, 201, 0);
    printf("2 px at +60 ms: pump %d\n", pump());
    fle_sys_clock_advance_to(fle_sys_clock_ticks() + 30u * FLE_TICKS_PER_MS);
    fle_input_host_mouse(FLE_IN_MOVE, 622, 221, 0);
    printf("20 px at +90 ms: pump %d\n", pump());
    fle_sys_clock_advance_to(fle_sys_clock_ticks() + 99u * FLE_TICKS_PER_MS);
    printf("+189 ms (99 ms after the move): pump %d\n", pump());
    fle_sys_clock_advance_to(fle_sys_clock_ticks() + 1u * FLE_TICKS_PER_MS);
    printf("+190 ms (100 ms after the move): pump %d\n", pump());
}
static void sc_track_hover_edge(void)
{
    uint32_t r;
    world();
    fle_input_host_mouse(FLE_IN_MOVE, 600, 200, 0);
    pump();
    tme(FLE_TME_HOVER, g_b, 100, &r);
    printf("hover 100 ms from the anchor (600,200); the rectangle is 4 x 4:\n");
    fle_sys_clock_advance_to(fle_sys_clock_ticks() + 60u * FLE_TICKS_PER_MS);
    fle_input_host_mouse(FLE_IN_MOVE, 602, 200, 0);
    printf("2 px at +60 ms (on the edge, inside): pump %d\n", pump());
    fle_sys_clock_advance_to(fle_sys_clock_ticks() + 10u * FLE_TICKS_PER_MS);
    fle_input_host_mouse(FLE_IN_MOVE, 603, 200, 0);
    printf("3 px at +70 ms (outside, restarts at the new point): pump %d\n", pump());
    fle_sys_clock_advance_to(fle_sys_clock_ticks() + 30u * FLE_TICKS_PER_MS);
    printf("+100 ms (the first time-out has run out, the restarted one has not): pump %d\n", pump());
    fle_sys_clock_advance_to(fle_sys_clock_ticks() + 69u * FLE_TICKS_PER_MS);
    printf("+169 ms: pump %d\n", pump());
    fle_sys_clock_advance_to(fle_sys_clock_ticks() + 1u * FLE_TICKS_PER_MS);
    printf("+170 ms (100 ms after the move): pump %d\n", pump());
}
static void sc_key_target(void)
{
    world();
    CALL4(fle_imp_SetForegroundWindow, g_a, 0, 0, 0);
    CALL4(fle_imp_SetFocus, g_child, 0, 0, 0);
    printf("foreground A, focus the child of A: a key goes to the focus window, not to the active window:\n");
    fle_input_host_key(FLE_IN_KDOWN, 65, 0, 0);
    fle_input_host_key(FLE_IN_KUP, 65, 0, 0);
    printf("pump %d\n", pump());
}
static void sc_key_dead_window(void)
{
    MSGX m;

    /* decision 0218 (ship3): Enter is pressed and released; the key down is retrieved for the focus window (the child of A); while the guest handles it (an Enter that closes a dialog) the translation of the key up has already
     * been made for the child (a peek without removal translates ahead); then the child is destroyed. The key up must not reach the gone window: Windows decides a hardware message's window at retrieval, so it goes to the window
     * that has the focus now (the child is gone, the active window A takes it: Learn WM_KEYUP "If no window has the keyboard focus, the active window receives"). */
    world();
    CALL4(fle_imp_SetForegroundWindow, g_a, 0, 0, 0);
    CALL4(fle_imp_SetFocus, g_child, 0, 0, 0);
    fle_input_host_key(FLE_IN_KDOWN, 13, 0, 0);
    fle_input_host_key(FLE_IN_KUP, 13, 0, 0);
    printf("Enter down and up, the focus window is the child of A:\n");
    if (PEEK(&m, 0, 0, 0, 1)) printf("  retrieved %x for the child %d\n", (unsigned)m.msg, m.hwnd == g_child);
    if (PEEK(&m, 0, 0, 0, 0)) printf("  peeked %x for the child %d\n", (unsigned)m.msg, m.hwnd == g_child);
    CALL4(fle_imp_DestroyWindow, g_child, 0, 0, 0);
    printf("DestroyWindow(child): ret %llu\n", U(R[0]));
    if (PEEK(&m, 0, 0, 0, 1)) printf("  retrieved %x for A %d for the child %d\n", (unsigned)m.msg, m.hwnd == g_a, m.hwnd == g_child);
    printf("pump %d\n", pump());
}
static void sc_post_dead_window(void)
{
    /* decision 0218 (ship3): messages posted to a window are removed when the window is destroyed (the classic DestroyWindow page: "flushes the thread message queue"; Wine 10.0 queue_cleanup_window, UNOFFICIAL). A is the
     * control: its posted message is delivered. */
    world();
    CALL4(fle_imp_PostMessageW, g_child, 0x102, 'a', 0x1e0001);
    CALL4(fle_imp_PostMessageW, g_a, 0x102, 'b', 0x1e0001);
    CALL4(fle_imp_PostMessageW, g_child, 0x102, 'c', 0x1e0001);
    printf("posted WM_CHAR a to the child of A, b to A, c to the child of A; DestroyWindow(child):\n");
    CALL4(fle_imp_DestroyWindow, g_child, 0, 0, 0);
    printf("DestroyWindow(child): ret %llu\n", U(R[0]));
    printf("pump %d\n", pump());
}
static void sc_token(void)
{
    uint64_t ph = A(&stk[0x340]), buf = A(&stk[0x360]), rl = A(&stk[0x380]), h;

    CALL4(fle_imp_OpenProcessToken, UINT64_C(0xFFFFFFFFFFFFFFFF), 8, ph, 0);
    printf("OpenProcessToken(current process, TOKEN_QUERY): ret %llu\n", U(R[0]));
    h = fle_ld(ph, 8);
    printf("token handle nonzero %d\n", h != 0);
    fle_st(buf, 4, 0xEEEEEEEEu);
    fle_st(rl, 4, 0xEEEEEEEEu);
    CALL5(fle_imp_GetTokenInformation, h, 20, buf, 4, rl);
    printf("GetTokenInformation(TokenElevation, 4 bytes): ret %llu TokenIsElevated %llu ReturnLength %llu\n", U(R[0]), U(fle_ld(buf, 4)), U(fle_ld(rl, 4)));
    fle_st(buf, 4, 0xEEEEEEEEu);
    fle_st(rl, 4, 0);
    CALL5(fle_imp_GetTokenInformation, h, 20, buf, 2, rl);
    printf("GetTokenInformation(TokenElevation, 2 bytes): ret %llu ReturnLength %llu buffer %llx", U(R[0]), U(fle_ld(rl, 4)), U(fle_ld(buf, 4)));
    CALL4(fle_imp_GetLastError, 0, 0, 0, 0);
    printf(" last error %llu\n", U(R[0]));
    CALL4(fle_imp_CloseHandle, h, 0, 0, 0);
    printf("CloseHandle(token): ret %llu\n", U(R[0]));
    CALL4(fle_imp_CloseHandle, h, 0, 0, 0);
    printf("CloseHandle(token) again: ret %llu", U(R[0]));
    CALL4(fle_imp_GetLastError, 0, 0, 0, 0);
    printf(" last error %llu\n", U(R[0]));
}
static void sc_cmp_control(void)
{
    /* the guest's compare of the key character of Enter with the accelerator of the OK button (NORM_IGNORECASE = 1, LOCALE_USER_DEFAULT 0x400): CSTR_LESS_THAN 1, CSTR_EQUAL 2, CSTR_GREATER_THAN 3 */
    static uint16_t cr[] = { 0x0d, 0 }, lf[] = { 0x0a, 0 }, tab[] = { 0x09, 0 }, o[] = { 'O', 0 }, sp[] = { ' ', 0 }, abc[] = { 'a', 0x0d, 'b', 0 }, aob[] = { 'a', 'o', 'b', 0 };
    struct { uint16_t *x, *y; const char *what; } t[] = {
        { cr, o, "CR vs 'O'" }, { o, cr, "'O' vs CR" }, { lf, o, "LF vs 'O'" }, { tab, o, "TAB vs 'O'" }, { cr, sp, "CR vs space" }, { sp, cr, "space vs CR" }, { abc, aob, "a CR b vs a o b" }, { cr, cr, "CR vs CR" },
    };
    unsigned i;

    for (i = 0; i < sizeof t / sizeof t[0]; i++) {
        CALL6(fle_imp_CompareStringW, 0x400, 1, A(t[i].x), UINT64_C(0xFFFFFFFF), A(t[i].y), UINT64_C(0xFFFFFFFF));
        printf("CompareStringW(%s): %llu\n", t[i].what, U(R[0]));
    }
}
static void sc_track_cancel(void)
{
    uint32_t r;
    uint64_t p;
    world();
    fle_input_host_mouse(FLE_IN_MOVE, 600, 200, 0);
    pump();
    tme(FLE_TME_LEAVE | FLE_TME_HOVER, g_b, 100, &r);
    p = tme(FLE_TME_QUERY, g_b, 0, &r);
    printf("query: flags %llx hover %llu\n", U(fle_ld(p + 4, 4)), U(fle_ld(p + 16, 4)));
    tme(FLE_TME_CANCEL | FLE_TME_HOVER, g_b, 0, &r);
    p = tme(FLE_TME_QUERY, g_b, 0, &r);
    printf("cancel hover: flags %llx\n", U(fle_ld(p + 4, 4)));
    tme(FLE_TME_CANCEL | FLE_TME_LEAVE, g_b, 0, &r);
    p = tme(FLE_TME_QUERY, g_b, 0, &r);
    printf("cancel leave: flags %llx hwnd %s\n", U(fle_ld(p + 4, 4)), nm(fle_ld(p + 8, 8)));
    {
        uint64_t q = A(&stk[0x200]);
        fle_st(q, 4, 16); fle_st(q + 4, 4, FLE_TME_LEAVE); fle_st(q + 8, 8, g_b); fle_st(q + 16, 4, 0);
        CALL4(fle_imp__TrackMouseEvent, q, 0, 0, 0);
        printf("cbSize 16: ret %llu error %u\n", U(R[0]), (fle_imp_GetLastError(), (unsigned)R[0]));
        fle_st(q, 4, 24); fle_st(q + 8, 8, 0x1234);
        CALL4(fle_imp__TrackMouseEvent, q, 0, 0, 0);
        printf("invalid window: ret %llu error %u\n", U(R[0]), (fle_imp_GetLastError(), (unsigned)R[0]));
    }
}

/* ---- keys ---- */
static void sc_keys(void)
{
    world();
    CALL4(fle_imp_SetFocus, g_a, 0, 0, 0);
    printf("focus A; key down 'A' (65), up:\n");
    fle_input_host_key(FLE_IN_KDOWN, 65, 0, 0);
    printf("pump %d\n", pump());
    fle_input_host_key(FLE_IN_KUP, 65, 0, 0);
    printf("pump %d\n", pump());
    printf("shift (160) down, 'A' down, 'A' up, shift up:\n");
    fle_input_host_key(FLE_IN_KDOWN, 0xA0, 0, 0);
    fle_input_host_key(FLE_IN_KDOWN, 65, 0, 0);
    fle_input_host_key(FLE_IN_KUP, 65, 0, 0);
    fle_input_host_key(FLE_IN_KUP, 0xA0, 0, 0);
    printf("pump %d\n", pump());
    printf("auto-repeat of 'B' (66) and the right arrow (39, extended):\n");
    fle_input_host_key(FLE_IN_KDOWN, 66, 0, 0);
    fle_input_host_key(FLE_IN_KDOWN, 66, 0, 0);
    fle_input_host_key(FLE_IN_KUP, 66, 0, 0);
    fle_input_host_key(FLE_IN_KDOWN, 39, 0, 1);
    fle_input_host_key(FLE_IN_KUP, 39, 0, 1);
    printf("pump %d\n", pump());
    printf("Space (32), Enter (13) and Escape (27), each down and up (the keys of the welcome wizard):\n");
    fle_input_host_key(FLE_IN_KDOWN, 32, 0, 0);
    fle_input_host_key(FLE_IN_KUP, 32, 0, 0);
    fle_input_host_key(FLE_IN_KDOWN, 13, 0, 0);
    fle_input_host_key(FLE_IN_KUP, 13, 0, 0);
    fle_input_host_key(FLE_IN_KDOWN, 27, 0, 0);
    fle_input_host_key(FLE_IN_KUP, 27, 0, 0);
    printf("pump %d\n", pump());
}
static void sc_syskeys(void)
{
    world();
    CALL4(fle_imp_SetFocus, g_a, 0, 0, 0);
    printf("alt (164) down, F4 (115) down, F4 up, alt up:\n");
    fle_input_host_key(FLE_IN_KDOWN, 0xA4, 0, 0);
    fle_input_host_key(FLE_IN_KDOWN, 0x73, 0, 0);
    fle_input_host_key(FLE_IN_KUP, 0x73, 0, 0);
    fle_input_host_key(FLE_IN_KUP, 0xA4, 0, 0);
    printf("pump %d\n", pump());
    CALL4(fle_imp_IsWindow, g_a, 0, 0, 0);
    printf("IsWindow(A) after Alt+F4: %llu\n", U(R[0]));
}
static void sc_keystate(void)
{
    uint64_t p = A(&stk[0x300]);
    world();
    CALL4(fle_imp_SetFocus, g_a, 0, 0, 0);
    fle_input_host_key(FLE_IN_KDOWN, 0x14, 0, 0);       /* CapsLock */
    fle_input_host_key(FLE_IN_KUP, 0x14, 0, 0);
    fle_input_host_key(FLE_IN_KDOWN, 0xA2, 0, 0);       /* left ctrl */
    CALL4(fle_imp_GetAsyncKeyState, 0x11, 0, 0, 0);
    printf("async VK_CONTROL %llx\n", U(R[0]));
    CALL4(fle_imp_GetKeyState, 0x11, 0, 0, 0);
    printf("sync  VK_CONTROL before the message %llx\n", U(R[0]));
    pump();
    CALL4(fle_imp_GetKeyState, 0x11, 0, 0, 0);
    printf("sync  VK_CONTROL after %llx\n", U(R[0]));
    CALL4(fle_imp_GetKeyState, 0xA2, 0, 0, 0);
    printf("sync  VK_LCONTROL %llx, VK_RCONTROL ", U(R[0]));
    CALL4(fle_imp_GetKeyState, 0xA3, 0, 0, 0);
    printf("%llx\n", U(R[0]));
    CALL4(fle_imp_GetKeyState, 0x14, 0, 0, 0);
    printf("sync  VK_CAPITAL %llx\n", U(R[0]));
    CALL4(fle_imp_GetKeyboardState, p, 0, 0, 0);
    printf("GetKeyboardState ret %llu: [0x11]=%x [0xA2]=%x [0x14]=%x [0x41]=%x\n", U(R[0]), (unsigned)fle_ld(p + 0x11, 1), (unsigned)fle_ld(p + 0xA2, 1), (unsigned)fle_ld(p + 0x14, 1), (unsigned)fle_ld(p + 0x41, 1));
    CALL4(fle_imp_MapVirtualKeyW, 65, 0, 0, 0);
    printf("MapVirtualKeyW(65, VK_TO_VSC) %llx, ", U(R[0]));
    CALL4(fle_imp_MapVirtualKeyW, 0x1E, 1, 0, 0);
    printf("(0x1E, VSC_TO_VK) %llx, ", U(R[0]));
    CALL4(fle_imp_MapVirtualKeyW, 65, 2, 0, 0);
    printf("(65, VK_TO_CHAR) %llx\n", U(R[0]));
}
static void sc_msgpos(void)
{
    MSGX m;
    world();
    fle_input_host_mouse(FLE_IN_MOVE, 600, 200, 0);
    fle_sys_clock_advance_to(fle_sys_clock_ticks() + 5u * FLE_TICKS_PER_MS);
    if (PEEK(&m, 0, 0, 0, 1)) printf("peeked msg %x time %u pt %d,%d\n", m.msg, m.time, m.ptx, m.pty);
    CALL4(fle_imp_GetMessageTime, 0, 0, 0, 0);
    printf("GetMessageTime %llu\n", U(R[0]));
    CALL4(fle_imp_GetMessagePos, 0, 0, 0, 0);
    printf("GetMessagePos %llx\n", U(R[0]));
}

/* ---- the timers and the idle ---- */
static void sc_timer_periodic(void)
{
    MSGX m;
    uint64_t t0;
    int n = 0;
    g_a = mkwin(cls_a, WS_POPUP_ | WS_VISIBLE_, 0, 0, 10, 10, 0);
    pump();
    CALL4(fle_imp_SetTimer, g_a, 7, 100, 0);
    idle_jump = 1;
    t0 = fle_sys_clock_ticks();
    printf("SetTimer(A, 7, 100 ms) at %u ms\n", CLOCK_MS());
    while (n < 5) {
        GET(&m);
        if (m.msg == 0x113) { printf("WM_TIMER id %llu delivered at %llu ms (+%llu)\n", U(m.wp), U(fle_sys_clock_ticks() / FLE_TICKS_PER_MS), U((fle_sys_clock_ticks() - t0) / FLE_TICKS_PER_MS)); n++; }
    }
    {
        unsigned i, c = fle_win_timer_log_count();
        for (i = 0; i < c; i++) {
            fle_timer_log_t e;
            fle_win_timer_log_get(i, &e);
            printf("log #%u id %llu period %u due %llu delivered %llu late %llu ticks\n", i, U(e.id), e.period_ms, U(e.due_ticks / FLE_TICKS_PER_MS), U(e.delivered_ticks / FLE_TICKS_PER_MS), U(e.delivered_ticks - e.due_ticks));
        }
    }
}
extern void fle_thread_set_idle_stop(int on);
static void sc_idle_clamp(void)
{
    MSGX m;
    int n = 0;
    g_a = mkwin(cls_a, WS_POPUP_ | WS_VISIBLE_, 0, 0, 10, 10, 0);
    pump();
    CALL4(fle_imp_SetTimer, g_a, 9, 250, 0);
    CALL4(fle_imp_CreateThread, 0, 0, 0, 0);
    (void)n; (void)m;
}
static void sc_idle_hook(void)
{
    MSGX m;
    int n = 0;
    g_a = mkwin(cls_a, WS_POPUP_ | WS_VISIBLE_, 0, 0, 10, 10, 0);
    pump();
    CALL4(fle_imp_SetTimer, g_a, 9, 250, 0);
    idle_script_target = fle_sys_clock_ticks() + 10000u * FLE_TICKS_PER_MS;
    printf("SetTimer(A, 9, 250 ms) at %u ms; the host asks for the clock at +10000 ms at every idle\n", CLOCK_MS());
    while (n < 3) {
        GET(&m);
        if (m.msg == 0x113) { printf("WM_TIMER id %llu at %u ms\n", U(m.wp), CLOCK_MS()); n++; }
    }
    printf("idle calls %d\n", idle_calls);
}

typedef struct { const char *name; void (*fn)(void); int idle; } scenario;
static const scenario S[] = {
    { "move", sc_move, 0 }, { "coalesce", sc_coalesce, 0 }, { "click", sc_click, 0 }, { "capture", sc_capture, 0 }, { "disabled", sc_disabled, 0 }, { "dblclk", sc_dblclk, 0 },
    { "hit_transparent", sc_hit_transparent, 0 }, { "mouseactivate", sc_mouseactivate, 0 }, { "mouseactivate_no", sc_mouseactivate_no, 0 },
    { "track_leave", sc_track_leave, 0 }, { "track_hover", sc_track_hover, 0 }, { "track_hover_edge", sc_track_hover_edge, 0 }, { "key_target", sc_key_target, 0 }, { "key_dead_window", sc_key_dead_window, 0 }, { "post_dead_window", sc_post_dead_window, 0 }, { "token", sc_token, 0 }, { "cmp_control", sc_cmp_control, 0 }, { "track_cancel", sc_track_cancel, 0 },
    { "keys", sc_keys, 0 }, { "syskeys", sc_syskeys, 0 }, { "keystate", sc_keystate, 0 }, { "msgpos", sc_msgpos, 0 },
    { "timer_periodic", sc_timer_periodic, 0 }, { "idle_hook", sc_idle_hook, 1 },
};

int main(int argc, char **argv)
{
    size_t i;

    (void)cur_pt;
    (void)sc_idle_clamp;
    setvbuf(stdout, NULL, _IONBF, 0);
    if (argc < 2) { fprintf(stderr, "usage: %s list | <scenario>\n", argv[0]); return 2; }
    if (!strcmp(argv[1], "list")) {
        for (i = 0; i < sizeof S / sizeof S[0]; i++) printf("%s\n", S[i].name);
        return 0;
    }
    for (i = 0; i < sizeof S / sizeof S[0]; i++) {
        if (strcmp(argv[1], S[i].name)) continue;
        boot();
        if (S[i].idle) fle_thread_set_idle_stop(0);
        S[i].fn();
        printf("scenario %s done, virtual clock %u ms\n", S[i].name, CLOCK_MS());
        return 0;
    }
    fprintf(stderr, "unknown scenario %s\n", argv[1]);
    return 2;
}

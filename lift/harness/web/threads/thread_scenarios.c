/* Thread scenarios for decision 0124 (docs/decisions/0124-wasm-thread-switching.md, PROVISIONAL): ONE C source, built natively (clang, swapcontext) and for wasm (emcc, JSPI), run with
 * the real runtime files (fle_thread.c, fle_win.c, fle_event.c, fle_sys.c, fle_imports.c, fle_proc.c, fle_callback.c ...) and STUB guest functions (a C function in a dispatch table is a
 * "guest function"): the lifted engine is not involved. The scenarios are those of tests/wall_fr8.rs (decision 0122) plus two threads of the shape the real engine will run (QFC's
 * TTimerThread = a DwmFlush + Sleep loop that posts a message to its owner window and calls a subscriber; FLEngine's TLayeredControlThread = a window of its own and a GetMessageW loop).
 *
 * usage: <program> list | <scenario name>
 * Every event goes to stdout through printf (unbuffered), the scheduler's own trace (fle_thread_set_trace) to stderr. The acceptance test of decision 0124 is that stdout AND
 * stderr are byte-identical between the native run and the wasm run (node and Chrome). Nothing printed may depend on the host: no host address, no host time. */
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
#include "fle_vfs.h"
#include "fle_locale.h"
#include "fle_sysinfo.h"
#include "fle_x87ctl.h"
#include "fle_mxcsr.h"
void fle_imp_CloseHandle(void);

uint64_t R[16];
fle_eflags FL;
fle_xmm X[16];
static uint64_t stk[0x4000] __attribute__((aligned(4096))); /* 128 KiB: the first thread's guest stack */
#define E0 ((uint64_t)(uintptr_t)stk + 0x10000 + 8)
#define RET0 UINT64_C(0xdead0000)
#define U(x) ((unsigned long long)(x))
#define A(x) ((uint64_t)(uintptr_t)(x))
#define INF 0xFFFFFFFFu

#define CALL4(f, a, b, c, d) do { R[1] = (a); R[2] = (b); R[8] = (c); R[9] = (d); R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000); f(); R[4] += 8; } while (0)
#define CALL6(f, a, b, c, d, e, g) do { uint64_t sp0_ = R[4]; R[4] = (R[4] - 0x40) & ~UINT64_C(15); R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000); fle_st(R[4] + 0x28, 8, (e)); fle_st(R[4] + 0x30, 8, (g)); \
    R[1] = (a); R[2] = (b); R[8] = (c); R[9] = (d); f(); R[4] = sp0_; } while (0)
#define CALL5(f, a, b, c, d, e) CALL6(f, a, b, c, d, e, 0)
static uint32_t LE(void) { fle_imp_GetLastError(); return (uint32_t)R[0]; }
static void SETLE(uint32_t v) { fle_set_last_error(v); }
static uint64_t H_CREATE(uint64_t start, uint64_t param, uint32_t flags, uint64_t ptid) { CALL6(fle_imp_CreateThread, 0, 0, start, param, flags, ptid); return R[0]; }
static uint64_t H_EVENT(int manual, int init) { CALL4(fle_imp_CreateEventW, 0, manual, init, 0); return R[0]; }
static uint32_t W1(uint64_t h, uint32_t ms) { CALL4(fle_imp_WaitForSingleObject, h, ms, 0, 0); return (uint32_t)R[0]; }
static void SETEV(uint64_t h) { CALL4(fle_imp_SetEvent, h, 0, 0, 0); }
static void SLEEP(uint32_t ms) { CALL4(fle_imp_Sleep, ms, 0, 0, 0); }
static void DWMFLUSH(void) { CALL4(fle_imp_DwmFlush, 0, 0, 0, 0); }
static uint32_t EXITCODE(uint64_t h) { uint32_t c = 0xEEEE; CALL4(fle_imp_GetExitCodeThread, h, A(&c), 0, 0); return c; }
static uint32_t CLOCK_MS(void) { return (uint32_t)(fle_sys_clock_ticks() / FLE_TICKS_PER_MS); }

#include <time.h>
static double now_ms(void) { struct timespec t; clock_gettime(CLOCK_MONOTONIC, &t); return (double)t.tv_sec * 1e3 + (double)t.tv_nsec / 1e6; }

static int ord[256], on_;
static void note(int v) { if (on_ < 256) ord[on_++] = v; }
static void print_order(void) { int i; printf("order:"); for (i = 0; i < on_; i++) printf(" %d", ord[i]); printf("\n"); }

/* ---- the guest functions: VA = 0x1000 + 0x10 * k, sorted ---- */
#define VA(k) (0x1000u + 0x10u * (k))
#define VA_WP 0x2000
#define VA_SUB 0x2010

/* a window procedure stub that logs (thread, hwnd, message, wParam, lParam) of every call, answers the creation messages as Learn says and 0x1000 + message for the rest; 0x8010 posts WM_QUIT;
 * 0x8020 sleeps (a switch INSIDE a window procedure, inside fle_call_guest) */
static int wp_calls;
static void wndproc(void)
{
    uint64_t hw = R[1], msg = R[2], wp = R[8], lp = R[9];
    (void)hw;
    wp_calls++;
    /* lp of the creation messages is a pointer into host memory: not printed (the output must not depend on the host) */
    if (msg == 0x81 || msg == 0x83 || msg == 0x01) printf("  wndproc thread %u msg %llx depth %u\n", fle_thread_current(), U(msg), fle_callback_depth());
    else printf("  wndproc thread %u msg %llx wp %llu lp %llu depth %u\n", fle_thread_current(), U(msg), U(wp), U(lp), fle_callback_depth());
    if (msg == 0x81) { R[0] = 1; return; }
    if (msg == 0x83) { R[0] = 0; return; }
    if (msg == 0x01) { R[0] = 0; return; }
    if (msg == 0x8010) { CALL4(fle_imp_PostQuitMessage, wp, 0, 0, 0); R[0] = 0; return; }
    if (msg == 0x8020) { SLEEP((uint32_t)wp); printf("  wndproc thread %u back from Sleep(%llu) at depth %u\n", fle_thread_current(), U(wp), fle_callback_depth()); R[0] = 0x8020; return; }
    R[0] = 0x1000 + msg;
}
static int cls_done;
static uint64_t mkwnd(void)
{
    static uint16_t cls[] = { 'T','P','U','t','i','l','W','i','n','d','o','w', 0 };
    static uint8_t wc[72];
    uint64_t w = VA_WP, inst = 0x400000, nm = A(cls), sp0 = R[4], h;
    if (!cls_done) {
        memset(wc, 0, sizeof wc); memcpy(wc + 8, &w, 8); memcpy(wc + 24, &inst, 8); memcpy(wc + 64, &nm, 8);
        CALL4(fle_imp_GetClassInfoW, 0x400000, A(cls), A(wc), 0);
        CALL4(fle_imp_RegisterClassW, A(wc), 0, 0, 0);
        cls_done = 1;
    }
    R[4] = (R[4] - 0x80) & ~UINT64_C(15); R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000);
    fle_st(R[4] + 0x28, 8, 0); fle_st(R[4] + 0x30, 8, 0); fle_st(R[4] + 0x38, 8, 0); fle_st(R[4] + 0x40, 8, 0);
    fle_st(R[4] + 0x48, 8, 0); fle_st(R[4] + 0x50, 8, 0); fle_st(R[4] + 0x58, 8, inst); fle_st(R[4] + 0x60, 8, 0);
    R[1] = 0x80; R[2] = A(cls); R[8] = 0; R[9] = 0x80000000u;
    fle_imp_CreateWindowExW();
    h = R[0];
    R[4] = sp0;
    return h;
}
typedef struct { uint64_t hwnd; uint32_t msg; uint32_t pad; uint64_t wp, lp; uint32_t time; int32_t ptx, pty; uint32_t priv; } MSGX;
_Static_assert(sizeof(MSGX) == FLE_MSG_SIZE, "sizeof MSG");
static uint32_t GET(uint64_t pmsg, uint64_t h, uint32_t lo, uint32_t hi) { CALL4(fle_imp_GetMessageW, pmsg, h, lo, hi); return (uint32_t)R[0]; }
static uint32_t PEEK(uint64_t pmsg, uint64_t h, uint32_t lo, uint32_t hi, uint32_t rm) { CALL5(fle_imp_PeekMessageW, pmsg, h, lo, hi, rm); return (uint32_t)R[0]; }
static uint32_t POST(uint64_t h, uint32_t m, uint64_t wp, uint64_t lp) { CALL4(fle_imp_PostMessageW, h, m, wp, lp); return (uint32_t)R[0]; }

/* ---- shared state of the scenarios ---- */
static uint64_t g_ev, g_ev2, g_w, g_cs[5], g_sp[2];

/* thread programs */
static void th_note_id(void) { fle_imp_GetCurrentThreadId(); note((int)R[0]); R[0] = 0; }
static void th_note_param(void) { note(10 + (int)R[1]); R[0] = 0; }
static void th_ret5(void) { note(10 + (int)R[1]); R[0] = 5; }
static void th_sleep5_then_event(void) { note(10); SLEEP(5); note(11); SETEV(g_ev); note(12); R[0] = 7; }
static void th_wait_event(void) { note(20); note(21 + (int)W1(g_ev, INF)); R[0] = 8; }
static void th_rr_a(void) { note(1); SLEEP(0); note(4); SLEEP(0); note(7); R[0] = 0; }
static void th_rr_b(void) { note(2); SLEEP(0); note(5); SLEEP(0); note(8); R[0] = 0; }
static void th_rr_c(void) { note(3); SLEEP(0); note(6); SLEEP(0); note(9); R[0] = 0; }
static void th_prio_hi(void) { note(30); SLEEP(3); note(31); R[0] = 0; }   /* decision 0234 (mt3) */
static void th_prio_note(void) { note(40 + (int)R[1]); R[0] = 0; }
static void th_sleep30(void) { SLEEP(30); R[0] = 3; }
static void th_wfmo(void) { SLEEP(2); SETEV(g_ev2); SLEEP(2); SETEV(g_ev); R[0] = 0; }
static void th_auto_a(void) { note(10 + (int)W1(g_ev, INF)); R[0] = 0; }
static void th_auto_b(void) { note(20 + (int)W1(g_ev, INF)); R[0] = 0; }
static void th_cs(void) { CALL4(fle_imp_EnterCriticalSection, A(g_cs), 0, 0, 0); note(10); CALL4(fle_imp_LeaveCriticalSection, A(g_cs), 0, 0, 0); note(11); R[0] = 0; }
static void deep_exit(int n) { if (n == 0) { note(12); CALL4(fle_imp_ExitThread, 42, 0, 0, 0); note(13); } else deep_exit(n - 1); }
static void th_exit(void) { note(10); deep_exit(5); note(11); }
static void th_exit_in_callback(void) { note(10); fle_call_guest(VA_SUB + 0x10, 1, 0, 0, 0); note(11); R[0] = 0; }
static void sub_exit(void) { note(20 + (int)fle_callback_depth()); CALL4(fle_imp_ExitThread, 43, 0, 0, 0); note(21); }
static void th_tls(void)
{
    uint64_t idx = g_sp[0];
    SETLE(77); CALL4(fle_imp_TlsGetValue, idx, 0, 0, 0); note((int)R[0]);
    CALL4(fle_imp_TlsSetValue, idx, 222, 0, 0); SETLE(88); SLEEP(1); note(1000 + (int)LE());
    CALL4(fle_imp_TlsGetValue, idx, 0, 0, 0); note((int)R[0]); R[0] = 0;
}
static void th_regs(void)
{
    R[10] = 0xAAAA; R[11] = 0xBBBB; X[2].u64[0] = 0x2222; fle_mxcsr = 0x9F80; fle_x87.cw = 0x027F; FL.cf = 1;
    SLEEP(1);
    note((int)(R[10] == 0xAAAA && R[11] == 0xBBBB && X[2].u64[0] == 0x2222 && fle_mxcsr == 0x9F80 && fle_x87.cw == 0x027F && FL.cf == 1));
    R[0] = 0;
}
static uint64_t gs_self[2], gs_tls[2], gs_lim;
static void th_gs(void) { gs_self[1] = fle_gs_ld8(0x30); gs_tls[1] = fle_gs_ld8(0x58); gs_lim = fle_gs_ld8(0x10); R[0] = 0; }
static void th_depth(void) { note(10 + (int)fle_callback_depth()); SLEEP(0); note(20 + (int)fle_callback_depth()); R[0] = 0; }
static void th_locale(void)
{
    CALL4(fle_imp_GetThreadLocale, 0, 0, 0, 0); note((int)R[0]);
    CALL4(fle_imp_SetThreadLocale, 0x409, 0, 0, 0);
    CALL4(fle_imp_GetThreadLocale, 0, 0, 0, 0); note((int)R[0]); R[0] = 0;
}
static void th_post2(void) { note(POST(g_w, 0x8001, 5, 6)); note(POST(g_w, 0x8002, 7, 8)); R[0] = 0; }
static void th_post_late(void) { note(10); SLEEP(3); note(11); POST(g_w, 0x8005, 9, 0); note(12); R[0] = 0; }
static void th_post_waitmsg(void) { SLEEP(2); POST(g_w, 0x8001, 1, 0); SLEEP(2); POST(g_w, 0x8002, 2, 0); R[0] = 0; }
static void th_post_then_event(void) { SLEEP(2); POST(g_w, 0x8001, 1, 0); SLEEP(2); SETEV(g_ev); R[0] = 0; }
static void th_post_msgwait(void) { SLEEP(2); POST(g_w, 0x8001, 1, 0); R[0] = 0; }
static void th_send_plain(void) { note(10); CALL4(fle_imp_SendMessageW, g_w, 0x8011, 3, 4); note(11 + (int)(R[0] - 0x9011)); R[0] = 0; }
static void th_landing(void);
static int lj_phase[8];
#define VA_L 0x3000

/* TTimerThread shape (thread-model.md: QFC TTimerThread.Execute = DwmFlush + Sleep loop; each period posts 0x8ffc to the owner window and calls the subscribers directly) */
static int tt_terminated, tt_ticks;
static void sub_timer(void) { note(300 + (int)R[1]); R[0] = 0; }
static void th_timer(void)
{
    while (!tt_terminated) {
        DWMFLUSH();
        SLEEP(2);
        if (tt_terminated) break;
        POST(g_w, 0x8ffc, (uint64_t)tt_ticks++, 0);
        fle_call_guest(VA_SUB, (uint64_t)tt_ticks, 0, 0, 0);
    }
    R[0] = 99;
}
/* TLayeredControlThread shape (FLEngine: a window of its own created by the thread, then a GetMessageW loop) */
static uint64_t lc_wnd;
static void th_layered(void)
{
    MSGX m;
    int n = 0;

    lc_wnd = mkwnd();
    SETEV(g_ev);
    while (GET(A(&m), 0, 0, 0) > 0) {
        CALL4(fle_imp_TranslateMessage, A(&m), 0, 0, 0);
        CALL4(fle_imp_DispatchMessageW, A(&m), 0, 0, 0);
        note(100 + (int)(m.msg & 0xff));
        n++;
    }
    note(200 + n);
    R[0] = 77;
}
static void th_spawn(void)
{
    uint64_t h2 = H_CREATE(VA(0), 0, 0, 0);
    note(50);
    W1(h2, INF);
    note(51);
    R[0] = 0;
}
static uint32_t big_hash;
static void th_many(void)
{
    uint32_t id = (uint32_t)R[1];
    int k;
    for (k = 0; k < 10; k++) {
        big_hash = big_hash * 31u + id * 7u + (uint32_t)k;
        if ((id + (uint32_t)k) % 3 == 0) SLEEP(0);
        else if ((id + (uint32_t)k) % 3 == 1) SLEEP(1 + (id % 4));
        else { CALL4(fle_imp_SwitchToThread, 0, 0, 0, 0); big_hash += (uint32_t)R[0]; }
    }
    R[0] = id;
}
static void th_wait_forever(void) { W1(g_ev, INF); R[0] = 0; }
static void th_pingpong(void)
{
    uint32_t n = (uint32_t)R[1], i;
    for (i = 0; i < n; i++) { W1(g_ev, INF); SETEV(g_ev2); }
    R[0] = 0;
}
static void th_bad_resume(void)
{
    printf("thread %u resumes a thread handle that was never issued\n", fle_thread_current());
    CALL4(fle_imp_ResumeThread, 0x7FFAA0000100ull, 0, 0, 0);
    R[0] = 0;
}

static void th_deep_locals(void), th_churn(void);
/* the measurement threads (defined below with their drivers; not part of `list`) */
static void th_parked(void), th_deep(void), th_b_ping(void), th_b_yield(void), th_b_sleep0(void), th_b_timer(void), th_b_idle(void);

/* the table: sorted by VA (a binary search) */
const fle_fn_entry fle_dispatch_table[] = {
    { VA(0), th_note_id },          /* 0x1000 */
    { VA(1), th_note_param },       /* 0x1010 */
    { VA(2), th_ret5 },
    { VA(3), th_sleep5_then_event },
    { VA(4), th_wait_event },
    { VA(5), th_rr_a },
    { VA(6), th_rr_b },
    { VA(7), th_rr_c },
    { VA(8), th_sleep30 },
    { VA(9), th_wfmo },
    { VA(10), th_auto_a },
    { VA(11), th_auto_b },
    { VA(12), th_cs },
    { VA(13), th_exit },
    { VA(14), th_exit_in_callback },
    { VA(15), th_tls },
    { VA(16), th_regs },
    { VA(17), th_gs },
    { VA(18), th_depth },
    { VA(19), th_locale },
    { VA(20), th_post2 },
    { VA(21), th_post_late },
    { VA(22), th_post_waitmsg },
    { VA(23), th_post_then_event },
    { VA(24), th_post_msgwait },
    { VA(25), th_send_plain },
    { VA(26), th_landing },
    { VA(27), th_timer },
    { VA(28), th_layered },
    { VA(29), th_spawn },
    { VA(30), th_many },
    { VA(31), th_wait_forever },
    { VA(32), th_pingpong },
    { VA(33), th_bad_resume },
    { VA(34), th_deep },
    { VA(35), th_b_ping },
    { VA(36), th_b_yield },
    { VA(37), th_b_sleep0 },
    { VA(38), th_b_timer },
    { VA(39), th_b_idle },
    { VA(40), th_parked },
    { VA(41), th_deep_locals },
    { VA(42), th_churn },
    { VA(43), th_prio_hi },
    { VA(44), th_prio_note },
    { VA_WP, wndproc },
    { VA_SUB, sub_timer },
    { VA_SUB + 0x10, sub_exit },
};
const uint64_t fle_dispatch_count = sizeof fle_dispatch_table / sizeof fle_dispatch_table[0];

/* a function as fe-lift emits it for the code after a guest setjmp (decision 0100): the C frame is live and registered while the rest runs; the guest's longjmp is fle_dispatch(L). */
static void th_landing(void)
{
    fle_landing lj;
    uint32_t id = (uint32_t)R[1];
    volatile uint32_t mine = id * 100;
    uint64_t sp = R[4];

    fle_landing_push(&lj, VA_L, sp);
    if (FLE_LANDING_SETJMP(&lj) == 0) {
        note((int)(1000 + id));
        SLEEP(1 + id);           /* the switch happens between the setjmp and the longjmp */
        SLEEP(0);
        mine += 1;
        R[4] = sp;               /* the guest's longjmp restores rsp */
        fle_dispatch(VA_L);      /* longjmp to the live registration whose rsp is R[4] */
        note(-1);
    } else {
        note((int)(2000 + id));
        note((int)mine);
    }
    fle_landing_pop(&lj);
    R[0] = id;
}


/* ===================== measurement families (NOT in `list`; their output carries host timings and is never compared) ===================== */
/* deep <depth> <nthreads> <mask> <frame>: thread i (0 = the first thread) recurses `depth` C frames of `frame` bytes of shadow stack each when bit i of mask is set (1 frame otherwise),
 * yields once at the bottom (SwitchToThread) and returns; prints one line per thread. The JSPI stack limit of V8 is what ends a too deep thread (RangeError). */
static int dp_depth, dp_frame, dp_mask;
static volatile unsigned dp_sink;
static __attribute__((noinline)) int deep_small(int n)
{
    volatile char c = (char)n;
    if (n == 0) { CALL4(fle_imp_SwitchToThread, 0, 0, 0, 0); return 0; }
    return 1 + deep_small(n - 1) + (int)(c & 0);
}
static __attribute__((noinline)) int deep_big(int n)
{
    volatile char buf[512];
    buf[0] = (char)n; buf[511] = (char)n;
    if (n == 0) { CALL4(fle_imp_SwitchToThread, 0, 0, 0, 0); return 0; }
    return 1 + deep_big(n - 1) + (int)((buf[0] + buf[511]) & 0);
}
static __attribute__((noinline)) int deep_c(int n, int frame) { return frame > 16 ? deep_big(n) : deep_small(n); }
static void th_deep(void)
{
    unsigned id = fle_thread_current();
    int r = deep_c((dp_mask >> id) & 1 ? dp_depth : 1, dp_frame);
    printf("thread %u deep %d ok\n", id, r);
    R[0] = 0;
}
static void run_deep(int depth, int nthreads, int mask, int frame)
{
    uint64_t h[64];
    int i;
    dp_depth = depth; dp_mask = mask; dp_frame = frame;
    for (i = 1; i < nthreads; i++) h[i] = H_CREATE(VA(34), 0, 0, 0);
    th_deep();
    for (i = 1; i < nthreads; i++) W1(h[i], INF);
    printf("deep done: %d threads, depth %d, mask %x, frame %d\n", nthreads, depth, mask, frame);
}
/* bench <kind> <n>: transfers per second. The transfer count is exact and printed; the time is the host's. */
static uint64_t b_ev, b_ev2;
static void th_b_ping(void) { uint32_t n = (uint32_t)R[1], i; for (i = 0; i < n; i++) { W1(b_ev, INF); SETEV(b_ev2); } R[0] = 0; }
static void th_b_yield(void) { uint32_t n = (uint32_t)R[1], i; for (i = 0; i < n; i++) CALL4(fle_imp_SwitchToThread, 0, 0, 0, 0); R[0] = 0; }
static void th_b_sleep0(void) { uint32_t n = (uint32_t)R[1], i; for (i = 0; i < n; i++) SLEEP(0); R[0] = 0; }
static void th_b_timer(void)    /* TTimerThread shape: DwmFlush + Sleep(2), one message per period */
{
    uint32_t n = (uint32_t)R[1], i;
    for (i = 0; i < n; i++) { DWMFLUSH(); SLEEP(2); POST(g_w, 0x8ffc, i, 0); }
    R[0] = 0;
}
static void th_parked(void) { W1(g_ev, INF); R[0] = 0; }
static void th_b_idle(void) { uint32_t n = (uint32_t)R[1], i; for (i = 0; i < n; i++) SLEEP(1); R[0] = 0; }
static void run_bench(const char *kind, uint32_t n)
{
    uint64_t h, h2 = 0;
    uint32_t i;
    double t0, t1;
    uint64_t transfers = 0;
    int idx = !strcmp(kind, "ping") ? 35 : !strcmp(kind, "yield") ? 36 : !strcmp(kind, "sleep0") ? 37 : !strcmp(kind, "timer") ? 38 : 39;
    b_ev = H_EVENT(0, 0); b_ev2 = H_EVENT(0, 0);
    g_w = mkwnd();
    h = H_CREATE(VA(idx), n, 0, 0);
    if (!strcmp(kind, "yield") || !strcmp(kind, "sleep0")) h2 = H_CREATE(VA(idx), n, 0, 0);
    t0 = now_ms();
    if (!strcmp(kind, "ping")) {
        for (i = 0; i < n; i++) { SETEV(b_ev); W1(b_ev2, INF); }
        transfers = 2ull * n;                    /* main -> thread (W1 parks) and thread -> main (W1 parks) per round, plus the start and the end */
    } else if (!strcmp(kind, "yield") || !strcmp(kind, "sleep0")) {
        for (i = 0; i < n; i++) { if (!strcmp(kind, "yield")) CALL4(fle_imp_SwitchToThread, 0, 0, 0, 0); else SLEEP(0); }
        transfers = 3ull * n;                    /* three runnable threads, one transfer per yield of each */
    } else if (!strcmp(kind, "timer")) {
        MSGX m;
        for (i = 0; i < n; i++) GET(A(&m), 0, 0, 0);
        transfers = 2ull * n;                    /* thread -> main when it posts and sleeps, main -> thread when main waits for the next tick */
    } else {
        for (i = 0; i < n; i++) SLEEP(1);
        transfers = 2ull * n;
    }
    W1(h, INF);
    if (h2) W1(h2, INF);
    t1 = now_ms();
    printf("bench %s n=%u transfers~%llu elapsed_ms=%.3f virtual_clock_ms=%u\n", kind, n, U(transfers), t1 - t0, CLOCK_MS());
}

/* deep_locals: every thread (the first too) keeps 64 bytes of its own locals in each of 12 nested C frames (-O0: the locals live on the wasm shadow stack) and yields at every level; after each yield
 * the bytes are checked. A thread that is resumed on another thread's shadow stack region (a lost __stack_pointer) finds them overwritten. The result is a count of corrupted checks: 0. */
static int dl_bad;
static __attribute__((noinline)) int dl_rec(int id, int level)
{
    volatile char buf[64];
    int i;
    for (i = 0; i < 64; i++) buf[i] = (char)(id * 16 + level);
    if (level < 12) {
        SLEEP(0);
        for (i = 0; i < 64; i++) if (buf[i] != (char)(id * 16 + level)) { dl_bad++; break; }
        dl_rec(id, level + 1);
    }
    SLEEP(0);
    for (i = 0; i < 64; i++) if (buf[i] != (char)(id * 16 + level)) { dl_bad++; break; }
    return level;
}
static void th_deep_locals(void)
{
    unsigned id = fle_thread_current();
    int r = dl_rec((int)id, 0);
    printf("thread %u deep_locals bottom level %d\n", id, r);
    R[0] = 0;
}
static void sc_deep_locals(void)
{
    uint64_t h[3];
    int i;
    for (i = 0; i < 3; i++) h[i] = H_CREATE(VA(41), 0, 0, 0);
    th_deep_locals();
    for (i = 0; i < 3; i++) W1(h[i], INF);
    printf("corrupted checks %d\n", dl_bad);
}

/* stack_churn: 4 threads, 30 iterations each; iteration k of thread i recurses 1 + ((i * 7 + k * 3) % 11) levels with 64-byte locals, yields at every level on the way down and on the way up,
 * and checks the bytes after every yield. The depths differ per thread and per iteration, so one thread holds live frames deep below the point where another thread unwinds and re-descends
 * (the interleaving that a lock-step recursion does not make). Count of corrupted checks: 0. */
static int ch_bad, ch_checks;
static __attribute__((noinline)) int ch_rec(int id, int k, int level, int depth)
{
    volatile char buf[64];
    int i;
    unsigned char tag = (unsigned char)(id * 61 + k * 7 + level * 3 + 1);
    for (i = 0; i < 64; i++) buf[i] = (char)tag;
    SLEEP(0);
    ch_checks++;
    for (i = 0; i < 64; i++) if ((unsigned char)buf[i] != tag) { ch_bad++; break; }
    if (level + 1 < depth) ch_rec(id, k, level + 1, depth);
    SLEEP(0);
    ch_checks++;
    for (i = 0; i < 64; i++) if ((unsigned char)buf[i] != tag) { ch_bad++; break; }
    return level;
}
static void th_churn(void)
{
    unsigned id = fle_thread_current();
    int k;
    for (k = 0; k < 30; k++) ch_rec((int)id, k, 0, 1 + (int)((id * 7 + (unsigned)k * 3) % 11));
    R[0] = 0;
}
static void sc_stack_churn(void)
{
    uint64_t h[3];
    int i;
    for (i = 0; i < 3; i++) h[i] = H_CREATE(VA(42), 0, 0, 0);
    th_churn();
    for (i = 0; i < 3; i++) W1(h[i], INF);
    printf("stack_churn: %d checks, corrupted %d\n", ch_checks, ch_bad);
}

static void boot(void) { R[4] = E0; fle_st(E0, 8, RET0); fle_teb_set_stack(A(stk), A(stk) + sizeof stk); }

/* ===================================== scenarios ===================================== */
static void sc_ids(void)
{
    uint64_t tid1 = 0, tid2 = 0, h1, h2;
    fle_imp_GetCurrentThreadId(); printf("main %llx\n", U(R[0]));
    h1 = H_CREATE(VA(0), 1, 0, A(&tid1));
    h2 = H_CREATE(VA(0), 2, 0, A(&tid2));
    printf("h1 %llx tid %llx\nh2 %llx tid %llx\n", U(h1), U(tid1), U(h2), U(tid2));
    printf("start-not-yet %d\n", on_);
    W1(h1, INF); W1(h2, INF);
    print_order();
}
static void sc_flags0(void)
{
    uint64_t h1 = H_CREATE(VA(1), 1, 0, 0), h2;
    note(1);
    CALL4(fle_imp_ResumeThread, h1, 0, 0, 0); printf("resume-flags0 %llu\n", U(R[0]));
    h2 = H_CREATE(VA(1), 2, 0, 0);
    note(2);
    printf("ran-before-wait %d\n", on_);
    SLEEP(0);
    printf("after-sleep0 %d\n", on_);
    W1(h2, INF);
    print_order();
}
static void sc_suspended(void)
{
    uint64_t h = H_CREATE(VA(2), 7, 4, 0);
    SLEEP(0); SLEEP(5);
    printf("ran-suspended %d\n", on_);
    printf("code %u\n", EXITCODE(h));
    CALL4(fle_imp_ResumeThread, h, 0, 0, 0); printf("resume %llu\n", U(R[0]));
    printf("ran-at-resume %d\n", on_);
    printf("wait %x\n", W1(h, INF));
    printf("ran %d code %u\n", on_, EXITCODE(h));
}
static void sc_suspend_count(void)
{
    uint64_t h = H_CREATE(VA(1), 1, 4, 0);
    CALL4(fle_imp_SuspendThread, h, 0, 0, 0); printf("suspend %llu\n", U(R[0]));
    CALL4(fle_imp_ResumeThread, h, 0, 0, 0); printf("r1 %llu\n", U(R[0]));
    SLEEP(0); printf("ran-with-count-1 %d\n", on_);
    CALL4(fle_imp_ResumeThread, h, 0, 0, 0); printf("r2 %llu\n", U(R[0]));
    W1(h, INF); printf("ran %d\n", on_);
}
static void sc_clock_order(void)
{
    uint64_t t0 = fle_sys_clock_ticks(), h1, h2;
    g_ev = H_EVENT(0, 0);
    h1 = H_CREATE(VA(3), 0, 0, 0);
    h2 = H_CREATE(VA(4), 0, 0, 0);
    note(1);
    W1(h1, INF);
    note(2);
    W1(h2, INF);
    note(3);
    print_order();
    printf("elapsed-ticks %llu\n", U(fle_sys_clock_ticks() - t0));
    printf("codes %u %u\n", EXITCODE(h1), EXITCODE(h2));
}
static void sc_round_robin(void)
{
    uint64_t h[3]; int i;
    h[0] = H_CREATE(VA(5), 0, 0, 0); h[1] = H_CREATE(VA(6), 0, 0, 0); h[2] = H_CREATE(VA(7), 0, 0, 0);
    for (i = 0; i < 3; i++) W1(h[i], INF);
    print_order();
}
/* decision 0234 (mt3) (mt3): the scheduling priorities (Learn "Scheduling Priorities"): a thread made suspended, given its level by SetThreadPriority, then resumed */
static uint64_t H_CREATE_PRIO(uint64_t start, uint64_t param, int32_t level)
{
    uint64_t h = H_CREATE(start, param, 4, 0);
    CALL4(fle_imp_SetThreadPriority, h, (uint32_t)level, 0, 0);
    CALL4(fle_imp_ResumeThread, h, 0, 0, 0);
    return h;
}
/* three threads that each yield twice (Sleep(0)): a = NORMAL, b = HIGHEST, c = BELOW_NORMAL; the main thread (NORMAL) waits for all three */
static void sc_priority_yield(void)
{
    uint64_t h[3]; int i;
    fle_thread_set_priorities(1);
    h[0] = H_CREATE_PRIO(VA(5), 0, 0); h[1] = H_CREATE_PRIO(VA(6), 0, 2); h[2] = H_CREATE_PRIO(VA(7), 0, -1);
    for (i = 0; i < 3; i++) W1(h[i], INF);
    print_order();
}
/* the end of a slice (fle_progress_slice): a ready thread of a LOWER priority does not take the processor, one of the SAME priority does (the rotation of decision 0122), one of a HIGHER priority does as soon as its
 * wait is over; th_prio_hi sleeps 3 ms, the clock moves 1 ms per slice */
static void sc_priority_slice(void)
{
    uint64_t h[3]; int i;
    fle_thread_set_priorities(1);
    h[0] = H_CREATE_PRIO(VA(43), 2, 2);        /* HIGHEST: th_prio_hi */
    h[1] = H_CREATE_PRIO(VA(44), 1, -2);       /* LOWEST: th_prio_note(1) */
    h[2] = H_CREATE_PRIO(VA(44), 2, 0);        /* NORMAL, as the main thread: th_prio_note(2) */
    for (i = 0; i < 8; i++) { fle_progress_slice(); printf("slice %d: ran %d clock %u\n", i, on_, CLOCK_MS()); }
    for (i = 0; i < 3; i++) W1(h[i], INF);
    print_order();
}
static void sc_switch_to_thread(void)
{
    uint64_t h;
    CALL4(fle_imp_SwitchToThread, 0, 0, 0, 0); printf("none %llu\n", U(R[0]));
    h = H_CREATE(VA(1), 0, 0, 0);
    CALL4(fle_imp_SwitchToThread, 0, 0, 0, 0); printf("one %llu on %d\n", U(R[0]), on_);
    CALL4(fle_imp_SwitchToThread, 0, 0, 0, 0); printf("again %llu\n", U(R[0]));
    (void)h;
}
static void sc_wait_thread(void)
{
    uint64_t h = H_CREATE(VA(8), 0, 0, 0);
    uint64_t t0 = fle_sys_clock_ticks();
    printf("zero %x\n", W1(h, 0));
    printf("ten %x\n", W1(h, 10));
    printf("inf %x\n", W1(h, INF));
    printf("code %u\n", EXITCODE(h));
    printf("signaled-again %x\n", W1(h, 0));
    printf("elapsed-ticks %llu\n", U(fle_sys_clock_ticks() - t0));
}
static void sc_wfmo(void)
{
    uint64_t arr[2], h;
    uint32_t r;
    g_ev = H_EVENT(1, 0); g_ev2 = H_EVENT(1, 0);
    arr[0] = g_ev; arr[1] = g_ev2;
    h = H_CREATE(VA(9), 0, 0, 0);
    CALL4(fle_imp_WaitForMultipleObjects, 2, A(arr), 0, INF); r = (uint32_t)R[0]; printf("any %u\n", r);
    CALL4(fle_imp_WaitForMultipleObjects, 2, A(arr), 1, INF); r = (uint32_t)R[0]; printf("all %u\n", r);
    SETEV(g_ev);
    CALL4(fle_imp_WaitForMultipleObjects, 2, A(arr), 0, 0); printf("lowest %u\n", (uint32_t)R[0]);
    CALL4(fle_imp_WaitForMultipleObjects, 0, A(arr), 0, 0); printf("n0 %x le %u\n", (uint32_t)R[0], LE());
    W1(h, INF);
}
static void sc_auto_reset(void)
{
    uint64_t h1, h2;
    g_ev = H_EVENT(0, 0);
    SETEV(g_ev);
    printf("stays %d\n", fle_event_state(g_ev));
    W1(g_ev, 0);
    printf("consumed %d\n", fle_event_state(g_ev));
    h1 = H_CREATE(VA(10), 0, 0, 0); h2 = H_CREATE(VA(11), 0, 0, 0);
    SLEEP(0);
    SETEV(g_ev);
    SLEEP(0);
    printf("after-first-set %d state %d\n", on_, fle_event_state(g_ev));
    SETEV(g_ev);
    W1(h1, INF); W1(h2, INF);
    print_order();
}
static void sc_critical_section(void)
{
    uint64_t h;
    CALL4(fle_imp_InitializeCriticalSection, A(g_cs), 0, 0, 0);
    CALL4(fle_imp_EnterCriticalSection, A(g_cs), 0, 0, 0);
    h = H_CREATE(VA(12), 0, 0, 0);
    SLEEP(5);
    printf("blocked %d\n", on_);
    CALL4(fle_imp_LeaveCriticalSection, A(g_cs), 0, 0, 0);
    W1(h, INF);
    printf("after %d\n", on_);
    print_order();
}
static void sc_exit_thread(void)
{
    uint64_t h = H_CREATE(VA(13), 0, 0, 0);
    W1(h, INF);
    printf("code %u\n", EXITCODE(h));
    print_order();
}
static void sc_exit_in_callback(void)
{
    uint64_t h = H_CREATE(VA(14), 0, 0, 0);
    W1(h, INF);
    printf("code %u\n", EXITCODE(h));
    print_order();
}
static void sc_tls_last_error(void)
{
    uint64_t th, v;
    CALL4(fle_imp_TlsAlloc, 0, 0, 0, 0); g_sp[0] = R[0];
    CALL4(fle_imp_TlsSetValue, g_sp[0], 111, 0, 0);
    SETLE(5);
    th = H_CREATE(VA(15), 0, 0, 0);
    SLEEP(0);
    SETLE(6);
    CALL4(fle_imp_TlsGetValue, g_sp[0], 0, 0, 0); v = R[0];
    printf("main slot %llu le %u\n", U(v), LE());
    W1(th, INF);
    CALL4(fle_imp_TlsGetValue, g_sp[0], 0, 0, 0); v = R[0];
    printf("main slot after %llu\n", U(v));
    SETLE(9);
    printf("main le %u\n", LE());
    print_order();
}
static void sc_registers(void)
{
    uint64_t th;
    R[10] = 0x1111; R[11] = 0x1212; X[2].u64[0] = 0x1010; fle_mxcsr = 0x1F80; fle_x87.cw = 0x037F; FL.cf = 0;
    th = H_CREATE(VA(16), 0, 0, 0);
    SLEEP(0);
    printf("main after first switch: %d\n", R[10] == 0x1111 && R[11] == 0x1212 && X[2].u64[0] == 0x1010 && fle_mxcsr == 0x1F80 && fle_x87.cw == 0x037F && FL.cf == 0);
    W1(th, INF);
    printf("main after the end: %d\n", R[10] == 0x1111 && R[11] == 0x1212 && X[2].u64[0] == 0x1010 && fle_mxcsr == 0x1F80 && fle_x87.cw == 0x037F && FL.cf == 0);
    print_order();
}
static void sc_gs_teb(void)
{
    uint64_t th;
    gs_self[0] = fle_gs_ld8(0x30); gs_tls[0] = fle_gs_ld8(0x58);
    th = H_CREATE(VA(17), 0, 0, 0);
    W1(th, INF);
    printf("first teb %d tls %d\n", gs_self[0] == A(fle_teb), gs_tls[0] == A(fle_tls_values));
    printf("second teb differs %d tls differs %d\n", gs_self[1] != gs_self[0], gs_tls[1] != gs_tls[0]);
    printf("second limit page aligned %d\n", (gs_lim & 0xfff) == 0 && gs_lim != 0);
    printf("first limit back %d\n", fle_gs_ld8(0x10) == ((A(stk) + 0xfff) & ~0xfffull));
}
static void sc_callback_depth(void)
{
    uint64_t th = H_CREATE(VA(18), 0, 0, 0);
    uint64_t r;
    note(100 + (int)fle_callback_depth());
    r = fle_call_guest(VA_WP, 0, 0x8001, 0, 0);
    printf("callback result %llx\n", U(r));
    r = fle_call_guest(VA_WP, 0, 0x8020, 3, 0);
    printf("callback with a Sleep inside: %llx\n", U(r));
    W1(th, INF);
    print_order();
}
static void sc_locale_priority(void)
{
    uint64_t th;
    CALL4(fle_imp_SetThreadLocale, 0x7f, 0, 0, 0);
    th = H_CREATE(VA(19), 0, 0, 0);
    W1(th, INF);
    CALL4(fle_imp_GetThreadLocale, 0, 0, 0, 0); printf("main %llx\n", U(R[0]));
    CALL4(fle_imp_SetThreadPriority, th, (uint32_t)-2, 0, 0); printf("set-prio-ended-thread %llu\n", U(R[0]));
    CALL4(fle_imp_SetThreadPriority, 0xFFFFFFFFFFFFFFFEull, 2, 0, 0); printf("set-self %llu\n", U(R[0]));
    print_order();
}
static void sc_post_get(void)
{
    MSGX m1, m2;
    uint64_t th;
    g_w = mkwnd();
    th = H_CREATE(VA(20), 0, 0, 0);
    printf("get1 %u\n", GET(A(&m1), 0, 0, 0));
    printf("m1 hwnd %llx msg %x wp %llx lp %llx\n", U(m1.hwnd), m1.msg, U(m1.wp), U(m1.lp));
    printf("get2 %u\n", GET(A(&m2), g_w, 0x8000, 0x8fff));
    printf("m2 hwnd %llx msg %x wp %llx lp %llx pt %d %d\n", U(m2.hwnd), m2.msg, U(m2.wp), U(m2.lp), m2.ptx, m2.pty);
    W1(th, INF);
    print_order();
}
static void sc_get_blocks(void)
{
    MSGX m;
    uint64_t th, t0;
    g_w = mkwnd();
    th = H_CREATE(VA(21), 0, 0, 0);
    t0 = fle_sys_clock_ticks();
    printf("get %u msg %x wp %llu\n", GET(A(&m), g_w, 0, 0), m.msg, U(m.wp));
    note(1);
    printf("waited>=3ms %d\n", fle_sys_clock_ticks() - t0 >= 30000);
    SETLE(0);
    printf("bad-hwnd %x le %u\n", GET(A(&m), 0x7f9900ff, 0, 0), LE());
    W1(th, INF);
    print_order();
}
static void sc_wait_message(void)
{
    MSGX m;
    uint64_t th;
    g_w = mkwnd();
    th = H_CREATE(VA(22), 0, 0, 0);
    CALL4(fle_imp_WaitMessage, 0, 0, 0, 0); printf("w1 %llu pending %u\n", U(R[0]), fle_thread_queue(0, 0)->count);
    PEEK(A(&m), 0, 0, 0, 0);
    printf("peeked %x\n", m.msg);
    CALL4(fle_imp_WaitMessage, 0, 0, 0, 0); printf("w2 %llu pending %u\n", U(R[0]), fle_thread_queue(0, 0)->count);
    W1(th, INF);
}
static void sc_msg_wait(void)
{
    MSGX m;
    uint64_t th, harr[1];
    g_w = mkwnd();
    g_ev = H_EVENT(0, 0);
    harr[0] = g_ev;
    th = H_CREATE(VA(23), 0, 0, 0);
    CALL5(fle_imp_MsgWaitForMultipleObjects, 1, A(harr), 0, INF, 0x40); printf("sendmask %u\n", (uint32_t)R[0]);
    CALL5(fle_imp_MsgWaitForMultipleObjects, 1, A(harr), 0, 5, 0x8); printf("old-input %x\n", (uint32_t)R[0]);
    printf("peek %u\n", PEEK(A(&m), 0, 0, 0, 1));
    CALL5(fle_imp_MsgWaitForMultipleObjects, 1, A(harr), 0, 0, 0xff); printf("zero-timeout %x\n", (uint32_t)R[0]);
    W1(th, INF);
}
static void sc_msg_wait_input(void)
{
    uint64_t th, harr[1];
    g_w = mkwnd();
    harr[0] = H_EVENT(1, 0);
    th = H_CREATE(VA(24), 0, 0, 0);
    CALL5(fle_imp_MsgWaitForMultipleObjects, 1, A(harr), 0, INF, 0x4ff); printf("input %u\n", (uint32_t)R[0]);
    W1(th, INF);
}
static void sc_send_message(void)
{
    MSGX m;
    uint64_t th;
    g_w = mkwnd();
    th = H_CREATE(VA(25), 0, 0, 0);
    SLEEP(0);
    note(1);
    printf("before-retrieval calls %d\n", wp_calls);
    PEEK(A(&m), 0, 0, 0, 1);
    note(2);
    printf("calls %d\n", wp_calls);
    W1(th, INF);
    print_order();
}
static void sc_landing(void)
{
    uint64_t h[3];
    uint32_t i;
    for (i = 0; i < 3; i++) h[i] = H_CREATE(VA(26), i + 1, 0, 0);
    R[1] = 0;
    th_landing();                     /* the first thread too: id 0 */
    for (i = 0; i < 3; i++) W1(h[i], INF);
    print_order();
}
static void sc_timer_thread(void)
{
    MSGX m;
    uint64_t h;
    int n = 0;
    g_w = mkwnd();
    h = H_CREATE(VA(27), 0, 0, 0);
    while (n < 6) {
        GET(A(&m), 0, 0, 0);
        printf("tick message %x wp %llu at %u ms\n", m.msg, U(m.wp), CLOCK_MS());
        CALL4(fle_imp_DispatchMessageW, A(&m), 0, 0, 0);
        n++;
    }
    tt_terminated = 1;
    printf("terminate flag set; wait %x\n", W1(h, INF));
    printf("exit code %u at %u ms\n", EXITCODE(h), CLOCK_MS());
    print_order();
}
static void sc_layered_thread(void)
{
    uint64_t h;
    g_ev = H_EVENT(1, 0);
    h = H_CREATE(VA(28), 0, 0, 0);
    W1(g_ev, INF);
    printf("layered window %llx\n", U(lc_wnd));
    printf("post %u %u\n", POST(lc_wnd, 0x8001, 1, 1), POST(lc_wnd, 0x8002, 2, 2));
    SLEEP(2);
    CALL4(fle_imp_SendMessageW, lc_wnd, 0x8003, 3, 3); printf("send result %llx\n", U(R[0]));
    printf("post-quit-trigger %u\n", POST(lc_wnd, 0x8010, 5, 0));
    printf("wait %x code %u\n", W1(h, INF), EXITCODE(h));
    print_order();
}
static void sc_spawn(void)
{
    uint64_t h = H_CREATE(VA(29), 0, 0, 0);
    W1(h, INF);
    note(52);
    print_order();
}
static void sc_many_threads(void)
{
    uint64_t h[40];
    uint32_t i, sum = 0;
    for (i = 0; i < 40; i++) h[i] = H_CREATE(VA(30), i, 0, 0);
    for (i = 0; i < 40; i++) { W1(h[i], INF); sum += EXITCODE(h[i]); }
    printf("hash %08x sum %u clock %u ms\n", big_hash, sum, CLOCK_MS());
}
static void sc_dwm_flush_threads(void)
{
    uint64_t t;
    int i;
    for (i = 0; i < 3; i++) {
        t = fle_sys_clock_ticks();
        DWMFLUSH();
        printf("frame %d: %llu -> %llu\n", i, U(t), U(fle_sys_clock_ticks()));
    }
    (void)H_CREATE(VA(8), 0, 0, 0);
    for (i = 0; i < 3; i++) {
        t = fle_sys_clock_ticks();
        DWMFLUSH();
        printf("frame with a sleeping thread %d: %llu -> %llu\n", i, U(t), U(fle_sys_clock_ticks()));
    }
}
static void sc_ping_pong(void)
{
    uint64_t h;
    uint32_t i, n = 2000;
    g_ev = H_EVENT(0, 0); g_ev2 = H_EVENT(0, 0);
    h = H_CREATE(VA(32), n, 0, 0);
    for (i = 0; i < n; i++) { SETEV(g_ev); W1(g_ev2, INF); }
    W1(h, INF);
    printf("ping-pong %u rounds done, clock %u ms\n", n, CLOCK_MS());
}
static void sc_stop_deadlock(void)
{
    uint64_t h;
    g_ev = H_EVENT(1, 0);
    h = H_CREATE(VA(31), 0, 0, 0);
    printf("before the deadlock\n");
    W1(h, INF);
    printf("returned\n");
}
static void sc_stop_bad_start(void)
{
    uint64_t h = H_CREATE(0x5555, 0, 0, 0);
    printf("created %llx\n", U(h));
    W1(h, INF);
    printf("returned\n");
}
static void sc_stop_in_thread(void)
{
    /* a named stop raised by the runtime in a thread other than the first, while the first waits for it */
    uint64_t h = H_CREATE(VA(33), 0, 0, 0);
    W1(h, INF);
    printf("returned\n");
}

typedef struct { const char *name; void (*fn)(void); int stops; } scenario;
static const scenario S[] = {
    { "ids", sc_ids, 0 }, { "priority_yield", sc_priority_yield, 0 }, { "priority_slice", sc_priority_slice, 0 }, { "flags0", sc_flags0, 0 }, { "suspended", sc_suspended, 0 }, { "suspend_count", sc_suspend_count, 0 },
    { "clock_order", sc_clock_order, 0 }, { "round_robin", sc_round_robin, 0 }, { "switch_to_thread", sc_switch_to_thread, 0 }, { "wait_thread", sc_wait_thread, 0 },
    { "wfmo", sc_wfmo, 0 }, { "auto_reset", sc_auto_reset, 0 }, { "critical_section", sc_critical_section, 0 }, { "exit_thread", sc_exit_thread, 0 },
    { "exit_in_callback", sc_exit_in_callback, 0 }, { "tls_last_error", sc_tls_last_error, 0 }, { "registers", sc_registers, 0 }, { "gs_teb", sc_gs_teb, 0 },
    { "callback_depth", sc_callback_depth, 0 }, { "locale_priority", sc_locale_priority, 0 }, { "post_get", sc_post_get, 0 }, { "get_blocks", sc_get_blocks, 0 },
    { "wait_message", sc_wait_message, 0 }, { "msg_wait", sc_msg_wait, 0 }, { "msg_wait_input", sc_msg_wait_input, 0 }, { "send_message", sc_send_message, 0 },
    { "landing", sc_landing, 0 }, { "timer_thread", sc_timer_thread, 0 }, { "layered_thread", sc_layered_thread, 0 }, { "spawn", sc_spawn, 0 },
    { "many_threads", sc_many_threads, 0 }, { "dwm_flush", sc_dwm_flush_threads, 0 }, { "ping_pong", sc_ping_pong, 0 },
    { "deep_locals", sc_deep_locals, 0 }, { "stack_churn", sc_stack_churn, 0 },
    { "stop_deadlock", sc_stop_deadlock, 1 }, { "stop_bad_start", sc_stop_bad_start, 1 }, { "stop_in_thread", sc_stop_in_thread, 1 },
};

int main(int argc, char **argv)
{
    size_t i;

    setvbuf(stdout, NULL, _IONBF, 0);
    if (argc < 2) { fprintf(stderr, "usage: %s list | <scenario>\n", argv[0]); return 2; }
    if (!strcmp(argv[1], "list")) {
        for (i = 0; i < sizeof S / sizeof S[0]; i++) printf("%s%s\n", S[i].name, S[i].stops ? " (stops)" : "");
        return 0;
    }
    for (i = 0; i < sizeof S / sizeof S[0]; i++) {
        if (strcmp(argv[1], S[i].name)) continue;
        boot();
        fle_thread_set_trace(1);
        S[i].fn();
        printf("scenario %s done, threads %u, virtual clock %u ms\n", S[i].name, fle_thread_count(), CLOCK_MS());
        return 0;
    }
    if (!strcmp(argv[1], "deep") && argc >= 6) {
        boot();
        run_deep(atoi(argv[2]), atoi(argv[3]), (int)strtol(argv[4], NULL, 0), atoi(argv[5]));
        return 0;
    }
    if (!strcmp(argv[1], "parked") && argc >= 3) {
        /* N threads all parked in WaitForSingleObject at once (N JSPI stacks live), then released one event at a time by the first thread */
        uint64_t hs[256];
        int i, n = atoi(argv[2]);
        boot();
        g_ev = H_EVENT(1, 0);
        for (i = 1; i <= n && i < 256; i++) hs[i] = H_CREATE(VA(40), 0, 0, 0);
        SLEEP(1);
        printf("parked: %d threads created, threads in the table %u\n", n, fle_thread_count());
        SETEV(g_ev);
        for (i = 1; i <= n && i < 256; i++) W1(hs[i], INF);
        printf("parked: all %d done, clock %u ms\n", n, CLOCK_MS());
        return 0;
    }
    if (!strcmp(argv[1], "bench") && argc >= 4) {
        boot();
        run_bench(argv[2], (uint32_t)atoi(argv[3]));
        return 0;
    }
    fprintf(stderr, "unknown scenario %s\n", argv[1]);
    return 2;
}

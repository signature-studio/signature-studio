/* Decision 0280 (gpu7, PROVISIONAL): tests of the scheduler hook that lets the engine take the page's input at a scheduler point (fle_thread.c schedule_ex -> fle_input_ring_poll; harness/fruity_idle.c).
 * Native, with the real runtime files (the list of tests/gpu7_ring_poll.rs) and STUB guest functions; the scaffold is that of harness/web/threads/thread_scenarios.c (decision 0124). Built with -DFLE_RING_POLL_HOOK=1 so
 * that the native fle_thread.c has the hook (the wasm build has it always; the native object without the define is byte-identical to the tree without the hook).
 *   usage: ring_poll hook_counts | inject_wakes_ui | apply_direct | apply_records
 * apply_direct / apply_records are the SAME script of events applied by the exports' functions (fle_input_host_*) and by fle_input_ring_poll() of harness/fruity_idle.c with fake imports (the function text is cut out of that file by the test);
 * their outputs must be identical. */
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
#include "fle_input.h"
void fle_imp_CloseHandle(void);
void fle_imp_GetAsyncKeyState(void);
void fle_imp_GetCursorPos(void);
void fle_imp_PostThreadMessageW(void);

uint64_t R[16];
fle_eflags FL;
fle_xmm X[16];
static uint64_t stk[0x4000] __attribute__((aligned(4096)));
#define E0 ((uint64_t)(uintptr_t)stk + 0x10000 + 8)
#define RET0 UINT64_C(0xdead0000)
#define U(x) ((unsigned long long)(x))
#define A(x) ((uint64_t)(uintptr_t)(x))
#define INF 0xFFFFFFFFu
#define CALL4(f, a, b, c, d) do { R[1] = (a); R[2] = (b); R[8] = (c); R[9] = (d); R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000); f(); R[4] += 8; } while (0)
static uint64_t H_CREATE(uint64_t start, uint64_t param, uint32_t flags, uint64_t ptid) { uint64_t sp0 = R[4]; R[4] = (R[4] - 0x40) & ~UINT64_C(15); R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000); fle_st(R[4] + 0x28, 8, flags); fle_st(R[4] + 0x30, 8, ptid); R[1] = 0; R[2] = 0; R[8] = start; R[9] = param; fle_imp_CreateThread(); R[4] = sp0; return R[0]; }
static uint32_t W1(uint64_t h, uint32_t ms) { CALL4(fle_imp_WaitForSingleObject, h, ms, 0, 0); return (uint32_t)R[0]; }
static void SLEEP(uint32_t ms) { CALL4(fle_imp_Sleep, ms, 0, 0, 0); }
static uint32_t CLOCK_MS(void) { return (uint32_t)(fle_sys_clock_ticks() / FLE_TICKS_PER_MS); }

/* ---- the hook: counts its calls, and at the call numbers of `inj_at` queues what the scenario wants (as fle_input_ring_poll does with the records) ---- */
static int hook_calls, hook_calls_before_threads, inj_at = -1, inj_kind, mixer_iter, injected_iter = -1, got_iter = -1;
static int settled_before_inj = -1, settled_after_inj = -1, settled_in_hand = -1, settled_first_iter = -1;   /* decision 0280, L1c */
#ifndef RING_APPLY_RECORDS
void fle_input_ring_poll(void)
{
    hook_calls++;
    if (fle_thread_count() < 2) hook_calls_before_threads++;
    if (hook_calls == inj_at) {
        uint64_t c0 = fle_sys_clock_ticks();
        int r;
        settled_before_inj = fle_input_settled();
        r = inj_kind == 0 ? fle_input_host_mouse(FLE_IN_BDOWN, 100, 100, FLE_IN_BTN_L) : 0;
        settled_after_inj = fle_input_settled();
        injected_iter = mixer_iter;
        printf("hook call %d: input queued (result %d) while the mixer-like thread is at iteration %d; the virtual clock was moved by %llu ticks\n", hook_calls, r, mixer_iter, U(fle_sys_clock_ticks() - c0));
    }
}
#endif

#define VA(k) (0x1000u + 0x10u * (k))
#define VA_WP 0x2000
static void wndproc(void)
{
    uint64_t msg = R[2];
    if (msg == 0x81 || msg == 0x83 || msg == 0x01) { R[0] = msg == 0x81 ? 1 : 0; return; }
    if (msg == 0x84) { R[0] = 1; return; }   /* WM_NCHITTEST -> HTCLIENT */
    R[0] = 0;
}
/* a thread like the mixer: it never waits for anything but Sleep(1), n times */
static void th_mixer(void)
{
    int n = (int)R[1], i;
    for (i = 0; i < n; i++) { mixer_iter = i; if (injected_iter >= 0 && settled_in_hand >= 0 && settled_first_iter < 0 && fle_input_settled()) settled_first_iter = i; SLEEP(1); }
    mixer_iter = n;
    CALL4(fle_imp_PostThreadMessageW, fle_thread_vid(0), 0x12, 0, 0);   /* WM_QUIT for the main thread: its message loop ends */
    R[0] = 0;
}
const fle_fn_entry fle_dispatch_table[] = { { VA(0), th_mixer }, { VA_WP, wndproc } };
const uint64_t fle_dispatch_count = sizeof fle_dispatch_table / sizeof fle_dispatch_table[0];

static int cls_done;
static uint64_t mkwnd(void)
{
    static uint16_t cls[] = { 'R','P','W','i','n', 0 };
    static uint8_t wc[72];
    uint64_t w = VA_WP, inst = 0x400000, nm = A(cls), sp0 = R[4], h;
    if (!cls_done) {
        memset(wc, 0, sizeof wc); memcpy(wc + 8, &w, 8); memcpy(wc + 24, &inst, 8); memcpy(wc + 64, &nm, 8);
        CALL4(fle_imp_RegisterClassW, A(wc), 0, 0, 0);
        cls_done = 1;
    }
    R[4] = (R[4] - 0x80) & ~UINT64_C(15); R[4] -= 8; fle_st(R[4], 8, 0xAAAA0000);
    fle_st(R[4] + 0x28, 8, 0); fle_st(R[4] + 0x30, 8, 0); fle_st(R[4] + 0x38, 8, 300); fle_st(R[4] + 0x40, 8, 300);
    fle_st(R[4] + 0x48, 8, 0); fle_st(R[4] + 0x50, 8, 0); fle_st(R[4] + 0x58, 8, inst); fle_st(R[4] + 0x60, 8, 0);
    R[1] = 0x80; R[2] = A(cls); R[8] = 0; R[9] = 0x90000000u;   /* WS_POPUP | WS_VISIBLE, 300 x 300 at (0, 0) */
    fle_imp_CreateWindowExW();
    h = R[0];
    R[4] = sp0;
    return h;
}
typedef struct { uint64_t hwnd; uint32_t msg; uint32_t pad; uint64_t wp, lp; uint32_t time; int32_t ptx, pty; uint32_t priv; } MSGX;
static uint32_t GET(uint64_t pmsg, uint64_t h, uint32_t lo, uint32_t hi) { CALL4(fle_imp_GetMessageW, pmsg, h, lo, hi); return (uint32_t)R[0]; }
static void boot(void) { R[4] = E0; fle_st(E0, 8, RET0); fle_teb_set_stack(A(stk), A(stk) + sizeof stk); }

/* the hook is called at scheduler points only: never before a second thread exists, and at every switch after */
static void sc_hook_counts(void)
{
    uint64_t h;
    SLEEP(1); SLEEP(1);
    printf("before CreateThread: hook calls %d\n", hook_calls);
    h = H_CREATE(VA(0), 6, 0, 0);
    W1(h, INF);
    printf("after the mixer-like thread ran 6 Sleep(1): hook called %s, calls with fewer than 2 threads %d, mixer iteration %d, clock %u ms\n", hook_calls >= 6 ? "at least once per switch" : "too rarely", hook_calls_before_threads, mixer_iter, CLOCK_MS());
}

/* an input queued by the hook at a scheduler point reaches the thread that waits in GetMessageW at the very next switch (not at an idle): the main thread (the 'UI') sits in GetMessageW while the mixer-like thread Sleeps */
static void sc_inject_wakes_ui(void)
{
    static MSGX m;
    uint64_t h, w;
    int r;
    w = mkwnd();
    printf("window %s\n", w ? "created" : "NOT created");
    inj_at = 5; inj_kind = 0;
    h = H_CREATE(VA(0), 20, 0, 0);
    for (;;) {
        r = (int)GET(A(&m), 0, 0, 0);
        if (r <= 0) break;
        if (m.msg == 0x201) { if (got_iter < 0) settled_in_hand = fle_input_settled(); if (got_iter < 0) { got_iter = mixer_iter; printf("the UI thread got WM_LBUTTONDOWN (0x201) wParam %llx while the mixer-like thread is at iteration %d\n", U(m.wp), mixer_iter); } }
    }
    printf("delay between the hook's queueing and the UI thread's message: %d iteration(s) of the mixer-like thread (must be at most 2; at an idle it would be all 20)\n", got_iter - injected_iter);
    printf("settled (decision 0280, L1c): before the injection %d, right after it %d, while the UI thread holds the message %d, first seen again at mixer iteration %d (UI thread back in GetMessageW)\n", settled_before_inj, settled_after_inj, settled_in_hand, settled_first_iter);
    (void)h;
}

/* ---- the same script, two ways ---- */
static uint32_t rec[16 * 8];
static int nrec;
static void add_mouse(int kind, int x, int y, int data) { uint32_t *r = rec + nrec * 8; r[0] = 1; r[1] = (uint32_t)kind; r[2] = (uint32_t)x; r[3] = (uint32_t)y; r[4] = (uint32_t)data; r[5] = (uint32_t)(100 + nrec); nrec++; }
static void add_key(int kind, uint32_t vk, uint32_t scan, int ext) { uint32_t *r = rec + nrec * 8; r[0] = 2; r[1] = (uint32_t)kind; r[2] = vk; r[3] = scan; r[4] = (uint32_t)ext; r[5] = (uint32_t)(100 + nrec); nrec++; }
static void add_display(uint32_t w, uint32_t h) { uint32_t *r = rec + nrec * 8; r[0] = 3; r[1] = 0; r[2] = w; r[3] = h; r[4] = 0; r[5] = (uint32_t)(100 + nrec); nrec++; }
static void script(void)
{
    add_mouse(FLE_IN_MOVE, 100, 200, 0); add_mouse(FLE_IN_BDOWN, 100, 200, FLE_IN_BTN_L); add_key(FLE_IN_KDOWN, 0x41, 0x1e, 0); add_key(FLE_IN_KDOWN, 0x41, 0x1e, 0);
    add_mouse(FLE_IN_BUP, 100, 200, FLE_IN_BTN_L); add_key(FLE_IN_KUP, 0x41, 0x1e, 0); add_mouse(FLE_IN_MOVE, 5000, -3, 0); add_mouse(FLE_IN_BDOWN, 7, 8, FLE_IN_BTN_R);
    add_key(FLE_IN_KDOWN, 0x20, 0x39, 0); add_key(FLE_IN_KDOWN, 0, 0, 0); add_mouse(99, 1, 1, 0); add_display(1920, 1080); add_display(10, 10); add_mouse(FLE_IN_WHEEL, 3, 4, -120);
}
static void dump(const char *tag)
{
    int32_t x = -1, y = -1;
    unsigned vks[] = { 0x01, 0x02, 0x41, 0x20 }, i;
    fle_input_pointer(&x, &y);
    printf("%s: pending %u pointer (%d,%d) keys", tag, fle_input_host_pending(), x, y);
    for (i = 0; i < sizeof vks / sizeof vks[0]; i++) { CALL4(fle_imp_GetAsyncKeyState, vks[i], 0, 0, 0); printf(" %02x=%04x", vks[i], (unsigned)(R[0] & 0xFFFF)); }
    printf("\n");
}
#ifdef RING_APPLY_RECORDS
/* the imports of fruity_idle.c's fle_input_ring_poll, faked: the first poll hands out the records (at most max), the next ones nothing */
static int polled;
int fle_js_input_poll(uint32_t *buf, int max) { int n = nrec < max ? nrec : max; if (polled) return 0; polled = 1; memcpy(buf, rec, (size_t)n * 8 * 4); return n; }
static int done_n, done_ok;
void fle_js_input_done(uint32_t *buf, int n) { int i; done_n = n; for (i = 0; i < n; i++) done_ok += (int)buf[i * 8 + 6]; }
#include "ring_poll_cut.inc"
#endif
static void sc_apply(int via_records)
{
    int i;
    script();
    dump("before");
    if (!via_records) {
        for (i = 0; i < nrec; i++) {
            uint32_t *r = rec + i * 8; int res = 0;
            if (r[0] == 1u) res = fle_input_host_mouse((int)r[1], (int32_t)r[2], (int32_t)r[3], (int32_t)r[4]);
            else if (r[0] == 2u) res = fle_input_host_key((int)r[1], r[2], r[3], (int)r[4]);
            else if (r[0] == 3u) res = fle_input_host_display(r[2], r[3]);
            printf("record %d (seq %u) -> %d\n", i, r[5], res);
        }
    }
#ifdef RING_APPLY_RECORDS
    else {
        fle_input_ring_poll();
        printf("done_n %d queued %d\n", done_n, done_ok);
    }
#endif
    dump("after");
}

int main(int argc, char **argv)
{
    setvbuf(stdout, NULL, _IONBF, 0);
    if (argc < 2) return 2;
    boot();
    if (!strcmp(argv[1], "hook_counts")) { sc_hook_counts(); return 0; }
    if (!strcmp(argv[1], "inject_wakes_ui")) { sc_inject_wakes_ui(); return 0; }
    if (!strcmp(argv[1], "apply_direct")) { sc_apply(0); return 0; }
#ifdef RING_APPLY_RECORDS
    if (!strcmp(argv[1], "apply_records")) { sc_apply(1); return 0; }
#endif
    fprintf(stderr, "unknown %s\n", argv[1]);
    return 2;
}

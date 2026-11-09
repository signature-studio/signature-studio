/* The browser build's idle and input (decision 0128 G3, PROVISIONAL; rewritten by decision 0136). Harness only, wasm only: compiled and linked by build_nmod_wasm.sh FRUITY=1; lift/runtime has the local edits of
 * decision 0136 (fle_input.c, the idle hook of fle_thread.c under FLE_THREAD_IDLE_HOOK).
 *
 * fle_thread_idle_hook(now, next, &advance_to) (called by the scheduler on the JSPI stack of the thread that parked last, when the first thread waits for input with an empty queue and no other thread can run):
 *   - with an input script (env FLE_G3_SCRIPT: harness/fruity_g3.c) the script drives it exactly as in the native run;
 *   - otherwise the async JS import fle_idle_wait(now, next) (harness/web/fruity/fle_idle_lib.js) suspends this JSPI stack until the page decides: the Worker's event loop runs meanwhile, and that is what returns control to
 *     the browser (G3). The answer is a double: < 0 the page ended the session; 0 input events were queued by the exports below (and/or the clock was moved by fle_g3_clock_to); > 0 the virtual clock is to be moved to
 *     that many ticks (the scheduler never skips a deadline: fle_thread.c clamps it to `next`).
 * The exports (called by the Worker while the engine is suspended; the engine and the exports run one at a time on the one wasm thread): fle_g3_mouse, fle_g3_key, fle_g3_clock_to, fle_g3_snapshot, fle_g3_timers,
 * fle_g3_ticks, fle_g3_pending. The runtime turns the queued raw events into messages when the owning thread retrieves them (runtime/fle_input.c): the page makes no message. */
#ifndef __EMSCRIPTEN__
#error "fruity_idle.c is the wasm build of the FRUITY-on harness (build_nmod_wasm.sh FRUITY=1)"
#endif
#include <emscripten.h>
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>

#include "a4.h"
#include "fle_input.h"
#include "fle_sys.h"
#include "fle_thread.h"
#include "fle_win.h"

void fruity_surface_idle_event(unsigned idle_no);
void fruity_surface_idle_sync(void);   /* decision 0145: the layered-window words of the surface table, before the page looks */
void g3_snapshot(const char *name);
void g3_timer_log(FILE *f);
int g3_script_idle(uint64_t now, uint64_t next, uint64_t *adv);
void fle_sys_clock_advance_to(uint64_t ticks);
/* the async import (a WebAssembly.Suspending function under -sJSPI; harness/web/fruity/fle_idle_lib.js) */
double fle_idle_wait(double now_ticks, double next_ticks);
/* RING-POLL-BEGIN (tests/gpu7_ring_poll.rs cuts the text between these two markers out of this file and compiles it natively with fake imports) */
/* Decision 0280 (PROVISIONAL): the page's input ring, read by the scheduler. fle_js_input_poll (fle_idle_lib.js; a plain synchronous import) copies up to `max` records of the Worker's ring into `buf` (8 words each: 0 type 1 mouse / 2 key / 3 display, 1 kind, 2..4 the values of the event as for fle_g3_mouse / fle_g3_key / fle_g3_display, 5 the page's sequence number, 6 the answer of the runtime, 7 unused) and returns the count (0 when
 * the ring is empty: one Atomics.load pair); the runtime queues each with the SAME functions the exports use, writes 1 (queued) or 0 (refused) into word 6, and tells the Worker with fle_js_input_done. No wasm export is called from JS, so nothing re-enters the engine. */
int fle_js_input_poll(uint32_t *buf, int max);
void fle_js_input_done(uint32_t *buf, int n);
#define FLE_RING_REC 8
#define FLE_RING_MAX 16

/* Decision 0280, L1c (PROVISIONAL): word FLE_RING_MAX * FLE_RING_REC after the records = fle_input_settled() as it was at this scheduler point (before the records of this call are queued): 1 when the guest has handled what the page sent so far and the thread that handled it waits in its message loop again; the Worker reads it to decide which present may answer an input */
void fle_input_ring_poll(void) {
    static uint32_t buf[FLE_RING_MAX * FLE_RING_REC + 1];
    int n, i;

    buf[FLE_RING_MAX * FLE_RING_REC] = (uint32_t)fle_input_settled();
    n = fle_js_input_poll(buf, FLE_RING_MAX);

    for (i = 0; i < n; i++) {
        uint32_t *r = buf + (size_t)i * FLE_RING_REC;
        int res = 0;

        if (r[0] == 1u) res = fle_input_host_mouse((int)r[1], (int32_t)r[2], (int32_t)r[3], (int32_t)r[4]);
        else if (r[0] == 2u) res = fle_input_host_key((int)r[1], r[2], r[3], (int)r[4]);
        else if (r[0] == 3u) res = fle_input_host_display(r[2], r[3]);
        r[6] = (uint32_t)res;
    }
    if (n > 0) fle_js_input_done(buf, n);
}
/* RING-POLL-END */


static unsigned idle_no, snap_calls;

int fle_thread_idle_hook(uint64_t now, uint64_t next, uint64_t *advance_to) {
    double d;
    int r = g3_script_idle(now, next, advance_to);
    if (r >= 0) return r;
    idle_no++;
    fruity_surface_idle_sync();
    d = fle_idle_wait((double)now, (double)next);
    if (d < 0) return 0;
    if (d == 0) return 1;
    *advance_to = (uint64_t)d;
    return 2;
}

EMSCRIPTEN_KEEPALIVE int fle_g3_mouse(int kind, int x, int y, int data) { return fle_input_host_mouse(kind, x, y, data); }
EMSCRIPTEN_KEEPALIVE int fle_g3_key(int kind, unsigned vk, unsigned scan, int ext) { return fle_input_host_key(kind, vk, scan, ext); }
/* decision 0212: the display changed to w x h (lift/runtime/fle_input.h fle_input_host_display); 1 = queued */
EMSCRIPTEN_KEEPALIVE int fle_g3_display(unsigned w, unsigned h) { return fle_input_host_display(w, h); }
/* decision 0267 (alpha10): the task bar of the host. fle_g3_taskbar_iconic: the logical window id (lid) of the first task bar button whose window is minimized, 0 when there is none (the page shows its restore button while this is
 * nonzero; the lid is the one of the window table and of the G3 record); fle_g3_taskbar_syscommand: the task bar tells that window a system command (SC_RESTORE 0xF120, SC_MINIMIZE 0xF020, SC_MAXIMIZE 0xF030, SC_CLOSE 0xF060);
 * 1 = queued (the window procedure gets WM_SYSCOMMAND when its thread next retrieves messages), 0 = refused (no such window, not a top-level window, another command, or the event queue is full). */
EMSCRIPTEN_KEEPALIVE unsigned fle_g3_taskbar_iconic(void) {
    fle_taskbar_button tb[16];
    unsigned n = fle_win_taskbar_buttons(tb, 16), k;
    for (k = 0; k < n; k++) if (tb[k].iconic) return tb[k].lid;
    return 0;
}
EMSCRIPTEN_KEEPALIVE int fle_g3_taskbar_syscommand(unsigned lid, unsigned cmd) { return fle_input_host_syscommand(FLE_WIN_HANDLE_BASE + 4u * (uint64_t)lid, cmd); }
EMSCRIPTEN_KEEPALIVE int fle_g3_pending(void) { return (int)fle_input_host_pending(); }
EMSCRIPTEN_KEEPALIVE double fle_g3_ticks(void) { return (double)fle_sys_clock_ticks(); }
/* the clock moves to `ticks` (never backwards) before an event is queued: the time a wall-clock page has let pass while the engine was idle */
EMSCRIPTEN_KEEPALIVE void fle_g3_clock_to(double ticks) {
    uint64_t t = (uint64_t)ticks;
    if (t > fle_sys_clock_ticks()) fle_sys_clock_advance_to(t);
}
/* the "g3-snap:" block on stdout and every live surface as a Module.fleSurfaceEvent (kind 3) for the page's canvases */
EMSCRIPTEN_KEEPALIVE void fle_g3_snapshot(void) {
    snap_calls++;
    g3_snapshot("page");
    fruity_surface_idle_event(snap_calls);
}
/* a timer made by the host for the window with logical id `lid` (a probe of the G3 record, not a call of the guest: runtime fle_win_host_timer) */
EMSCRIPTEN_KEEPALIVE int fle_g3_probe_timer(unsigned lid, unsigned id, unsigned ms) { return fle_win_host_timer(FLE_WIN_HANDLE_BASE + 4u * (uint64_t)lid, id, ms); }
EMSCRIPTEN_KEEPALIVE void fle_g3_timers(void) { g3_timer_log(stdout); fflush(stdout); }

/* the export of decision 0128 (a message by window id), kept because the link line names it: the allow-list is the one of decision 0128 */
EMSCRIPTEN_KEEPALIVE int fle_host_post_input(unsigned lid, unsigned msg, unsigned wparam, unsigned lparam) {
    uint64_t h = FLE_WIN_HANDLE_BASE + 4u * (uint64_t)lid;
    fle_win_dcq q;
    fle_win_view v;
    if (msg != 0x0200 && msg != 0x0201 && msg != 0x0202) return -1;
    if (!fle_win_dc_query(h, &q) || q.lid != lid || !fle_win_get(h, &v) || !v.live) return -2;
    return fle_thread_post(v.owner, h, msg, wparam, lparam) ? 1 : -3;
}

/* Cooperative guest threads (docs/decisions/0122-cooperative-threads.md, PROVISIONAL; see fle_thread.h). One host thread; the stack of each guest thread is a host
 * ucontext (native) and a VirtualAlloc region of the guest's own address space; the guest state that lives in globals (R, X, YH, FL, MM, the x87 and MXCSR state, the TEB pointer,
 * the landing list of setjmp, the nesting of fle_call_guest, the depth of the harness's shadow stack) is saved and loaded at every switch. A switch happens only in schedule(), which
 * only the waiting and yielding APIs reach, and (decision 0147) fle_progress_slice(), which fle_dispatch reaches once per slice of indirect calls of the guest. Nothing here reads the host: no real thread, no clock, no signal. */
#define _GNU_SOURCE
#define FLE_THREAD_IMPL
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifndef __EMSCRIPTEN__
#include <sys/mman.h>
#include <ucontext.h>
#include <unistd.h>
#elif defined(__EMSCRIPTEN__)
#include <unistd.h>
#endif

#include "fle_runtime.h"
#include "fle_host.h"
#include "fle_sse.h"
#include "fle_x87.h"
#include "fle_x87ctl.h"
#include "fle_mxcsr.h"
#include "fle_dispatch.h"
#include "fle_seh.h"
#include "fle_env.h"
#include "fle_imports.h"
#include "fle_callback.h"
#include "fle_sys.h"
#include "fle_event.h"
#include "fle_vfs.h"
#include "fle_proc.h"
#include "fle_realmod.h"
#include "fle_thread.h"
#include "fle_input.h"
#ifdef __EMSCRIPTEN__
#include "fle_thread_wasm.h"
#endif

/* The shadow stack of the harness's trace hook (harness/wasm/trace_native.c): its depth is kept per thread so that the frames of the thread that stops are the ones that are shown. Weak: not every link has it. */
size_t trace_depth(void) __attribute__((weak));
void trace_set_depth(size_t d) __attribute__((weak));
/* fle_win.c: the cross-thread SendMessageW (a message sent to the running thread is answered by the window procedure) and the window report. Weak: a link without fle_win.c has no windows. */
void fle_win_process_sent(const char *api) __attribute__((weak));
void fle_win_report(FILE *f) __attribute__((weak));
/* Decision 0123: the timers and the invalid windows of the window model (fle_win.c) are messages of the queue of the window's thread: a thread that waits for a message is woken when one is due, and a thread whose only
 * wake-up is a timer is not idle (the virtual clock is moved to the timer's deadline). Weak: a link without fle_win.c has none. */
uint64_t fle_win_next_timer_tick(unsigned idx) __attribute__((weak));
int fle_win_has_paint(unsigned idx) __attribute__((weak));
/* Decision 0139 (PROVISIONAL): the audio device of the virtual machine (runtime/fle_dsound.c, fle_audio_ring.c) is a source of events on the virtual clock: a play cursor that reaches a notification offset signals the event, and the
 * time of the next such offset is a due time like a timer's. fle_audio_poll() signals what the clock has reached (called before the waits are tested); fle_audio_next_tick() gives, in ticks of the virtual clock, the next time at which
 * a playing buffer reaches a notification offset (0 when none: nothing is due). Weak: a link without the audio files has no device. */
void fle_audio_poll(void) __attribute__((weak));
int fle_audio_next_tick(uint64_t *out) __attribute__((weak));
/* Decision 0139: real-time pacing with an external audio consumer (the browser's AudioWorklet; fle_audio_ring.h): while a buffer plays, the clock is not moved to the next deadline before that deadline has come in real time. Weak. */
int fle_audio_paced(void) __attribute__((weak));
/* Decision 0139: told at every switch (the thread that gives up the processor, the thread that gets it): the browser build measures how long each thread ran between two switches (the mixer's block, the UI thread's frame). Weak. */
void fle_audio_switch_note(unsigned from, unsigned to) __attribute__((weak));
int fle_audio_wait_until(uint64_t dl) __attribute__((weak));
/* Decision 0128 (PROVISIONAL, G3), changed by decision 0136: the idle of the embedding host. Provided by harness/fruity_idle.c (wasm) or harness/fruity_g3.c (native) (weak here: a link without it keeps the stop or
 * the clock jump). Called by schedule() when the first thread waits for input with an empty queue and no other thread can run, and the idle STOP of decision 0122 is off (FLE_FRUITY_NO_IDLE_STOP=1). `now` is the
 * virtual clock and `next` the earliest deadline of any waiting thread or window timer (0 = none), in ticks. The host runs while the engine is idle (in a browser: the Worker's event loop turns) and answers:
 * 0 the session ends (the scheduler stops by name); 1 an input event was queued (fle_input_host_*) and/or the host moved the clock itself: the scheduler goes on; 2 the virtual clock is to be moved to *advance_to
 * (at most to `next`: the scheduler never skips a deadline): the scheduler goes on. A weak symbol that no link defines is NULL: a link
 * without the host's idle is unchanged. */
int fle_thread_idle_hook(uint64_t now, uint64_t next, uint64_t *advance_to) __attribute__((weak));
/* Decision 0234 (mt3, PROVISIONAL): the priority level that SetThreadPriority kept for the first thread (fle_proc.c; the other threads keep theirs in `ps`). Weak: a link without fle_proc.c has only NORMAL. */
int32_t fle_proc_first_thread_priority(void) __attribute__((weak));
/* Decision 0280 (PROVISIONAL): harness/fruity_idle.c (wasm, FRUITY on): takes the input events that the page has put into its ring since the last look and queues them with fle_input_host_* (the same functions as the exports fle_g3_mouse / fle_g3_key that the Worker calls at the idle). Called from the scheduler only (schedule_ex), where the idle hook also runs. Weak: other links have no ring. */
#if defined(__EMSCRIPTEN__) || defined(FLE_RING_POLL_HOOK)
void fle_input_ring_poll(void) __attribute__((weak));
#endif
/* fle_sys.c */
int fle_sys_cs_try_take(uint64_t addr, unsigned thread);
void fle_sys_clock_advance_to(uint64_t ticks);

#define MEM_COMMIT_RESERVE 0x3000u
#define MEM_RELEASE_       0x8000u
#define PAGE_READWRITE_    0x04u
#define INFINITE_          0xFFFFFFFFu
#define PENDING            0xFFFFFFFEu   /* "not satisfied yet" inside this file (never a Win32 result) */

enum { T_READY = 0, T_RUNNING, T_WAITING, T_DONE };
enum { W_NONE = 0, W_SLEEP, W_OBJECTS, W_MSG_GET, W_MSG_WAIT, W_CS, W_SENT, W_SUSPEND };

typedef struct {
    uint64_t r[16];
    fle_xmm x[16], yh[16];
    fle_eflags fl;
    uint64_t mm[8];
    fle_f80 st[8];
    fle_x87_state x87;
    uint8_t mmx_active;
    uint32_t mxcsr;
    fle_landing *landing;
    struct fle_seh_reg *seh;       /* decision 0132: the registrations of functions with an except landing, and the exception being dispatched */
    struct fle_seh_disp *sehd;
    unsigned cbdepth;
    size_t tdepth;
} gctx;

typedef struct {
    int kind;
    const char *api;
    int has_deadline;
    uint64_t deadline;
    uint32_t n;
    uint64_t h[FLE_MAXIMUM_WAIT_OBJECTS];
    int all, msg;
    uint32_t mask;
    uint64_t fhwnd;
    uint32_t fmin, fmax;
    uint64_t cs_addr;
    fle_sent *sent;
    uint32_t result;
    uint64_t seq;
} twait;

typedef struct {
    unsigned idx;
    uint32_t vid;
    uint64_t hser;      /* decision 0196: the serial of the handle (FLE_THREAD_HANDLE_BASE + 4 * hser), not tied to the table index once slots are reused */
    uint64_t done_seq;  /* decision 0196: the order in which threads finished (0 while the thread lives) */
    int state;
    uint32_t suspend;
    int started, closed;
    uint64_t start, param;
    void (*nbody)(uint64_t); /* decision 0171: a thread made by fle_thread_spawn runs this C function (and the guest code it calls) instead of the guest function `start` */
    uint64_t narg;
    uint32_t exit_code;
    uint64_t stack_base, stack_size;
    uint8_t *teb;
    uint64_t *tls;
    uint32_t last_error, lcid;
    uint64_t *fls;
    fle_thread_pstate ps;
    gctx g;
#ifndef __EMSCRIPTEN__
    ucontext_t uc;
    void *cstack;
    size_t cstack_len;
#endif
    twait w;
    fle_tqueue q;
} thr;

static thr T[FLE_THREAD_MAX];
static unsigned nthr, cur;
static uint64_t next_hser = 2, done_clock;   /* decision 0196: serial 1 is the first thread's, which is never issued: the serial of table index i was i + 1 */
static uint64_t wait_seq;
static int idle_stop, trace_on;
static int hook_registered;
static thr *pending_free;

/* decision 0147: the pseudo-preemption (see fle_thread.h) */
static int preempt_armed, preempt_env_read;
static uint64_t slice_dispatches = FLE_PREEMPT_SLICE_DISPATCHES, slice_ticks = FLE_PREEMPT_SLICE_TICKS;
static fle_preempt_stats pst;

unsigned fle_thread_count(void) { return nthr; }
unsigned fle_thread_current(void) { return nthr ? cur : 0; }
unsigned fle_thread_vid(unsigned idx) { return FLE_EMULATED_TID + idx; }
int fle_thread_index_of_vid(uint32_t vid)
{
    if (nthr == 0) return vid == FLE_EMULATED_TID ? 0 : -1;
    if (vid < FLE_EMULATED_TID || vid - FLE_EMULATED_TID >= nthr) return -1;
    return (int)(vid - FLE_EMULATED_TID);
}
int fle_thread_is_done(unsigned idx) { return idx < nthr && T[idx].state == T_DONE; }
/* Decision 0280, L1c (PROVISIONAL): 1 when thread idx is blocked in GetMessage / WaitMessage / MsgWaitForMultipleObjects (back at its message loop), not in any other wait (a lock, Sleep, a handle) */
int fle_thread_in_message_wait(unsigned idx) { return idx < nthr && T[idx].state == T_WAITING && (T[idx].w.kind == W_MSG_GET || T[idx].w.kind == W_MSG_WAIT || (T[idx].w.kind == W_OBJECTS && T[idx].w.msg)); }
void fle_thread_set_idle_stop(int on) { idle_stop = on; }
void fle_thread_set_trace(int on) { trace_on = on; }

uint64_t *fle_thread_tls_values(void) { return nthr && cur ? T[cur].tls : NULL; }
uint32_t *fle_thread_slot_last_error(void) { return nthr && cur ? &T[cur].last_error : NULL; }
uint32_t *fle_thread_slot_lcid(void) { return nthr && cur ? &T[cur].lcid : NULL; }
uint64_t *fle_thread_slot_fls(void) { return nthr && cur ? T[cur].fls : NULL; }
void fle_thread_tls_clear(uint32_t idx)
{
    unsigned i;

    for (i = 1; i < nthr; i++)
        if (T[i].tls && idx < FLE_THREAD_TLS_SLOTS) T[i].tls[idx] = 0;
}

static void tlog(const char *what, unsigned a, unsigned b)
{
    char buf[160];
    int n;

    if (!trace_on) return;
    n = snprintf(buf, sizeof buf, "fle-thread: %s %u %u clock=%" PRIu64 "\n", what, a, b, fle_sys_clock_ticks());
    if (n > 0 && write(2, buf, (size_t)n) < 0) return;
}

/* ---- the guest context ---- */
static void save_ctx(gctx *c)
{
    memcpy(c->r, R, sizeof c->r);
    memcpy(c->x, X, sizeof c->x);
    memcpy(c->yh, YH, sizeof c->yh);
    c->fl = FL;
    memcpy(c->mm, MM, sizeof c->mm);
    memcpy(c->st, fle_x87_r, sizeof c->st);
    c->x87 = fle_x87;
    c->mmx_active = fle_x87_mmx_active;
    c->mxcsr = fle_mxcsr;
    c->landing = fle_landing_top;
    c->seh = fle_seh_top;
    c->sehd = fle_seh_cur;
    c->cbdepth = fle_callback_depth();
    c->tdepth = trace_depth ? trace_depth() : 0;
}

static void load_ctx(const gctx *c)
{
    memcpy(R, c->r, sizeof c->r);
    memcpy(X, c->x, sizeof c->x);
    memcpy(YH, c->yh, sizeof c->yh);
    FL = c->fl;
    memcpy(MM, c->mm, sizeof c->mm);
    memcpy(fle_x87_r, c->st, sizeof c->st);
    fle_x87 = c->x87;
    fle_x87_mmx_active = c->mmx_active;
    fle_mxcsr = c->mxcsr;
    fle_landing_top = c->landing;
    fle_seh_top = c->seh;
    fle_seh_cur = c->sehd;
    fle_callback_depth_set(c->cbdepth);
    if (trace_set_depth) trace_set_depth(c->tdepth);
}

static void set_cur_pointers(const thr *t)
{
    fle_teb_cur = t->idx ? t->teb : NULL;
    fle_tls_cur = t->idx ? t->tls : NULL;
}

static uint64_t now_ticks(void) { return fle_sys_clock_ticks(); }
static uint64_t ms_deadline(uint32_t ms) { return now_ticks() + (uint64_t)ms * FLE_TICKS_PER_MS; }

/* The table index of the thread that handle `h` names; -1 for a value that was never issued; -2 (decision 0196) for a value that was issued for a thread whose table slot has been used again since (the
 * handle was closed, or the thread was made by fle_thread_spawn and never had one the guest could close): to the callers that is a closed handle. */
static int handle_index(uint64_t h)
{
    uint64_t off;
    unsigned i;

    if (h <= FLE_THREAD_HANDLE_BASE || h >= FLE_THREAD_HANDLE_BASE + FLE_THREAD_REGION_SIZE) return -1;
    off = h - FLE_THREAD_HANDLE_BASE;
    if (off % 4u) return -1;
    off = off / 4u;
    if (off < 2u || off >= next_hser) return -1;
    for (i = 1; i < nthr; i++)
        if (T[i].hser == off) return (int)i;
    return -2;
}

int fle_thread_owns_handle(uint64_t h)
{
    return h > FLE_THREAD_HANDLE_BASE && h < FLE_THREAD_HANDLE_BASE + FLE_THREAD_REGION_SIZE;
}

static uint64_t handle_of(unsigned idx) { return FLE_THREAD_HANDLE_BASE + 4u * T[idx].hser; }

/* Decision 0196 (PROVISIONAL): the table index of the finished thread whose place CreateThread may use again once the table is full (the one that finished longest ago), or 0 when there is none. A place is
 * free when the thread is done, its host memory has been given back (free_pending), and nobody can use a handle of it any more: its handle was closed, or the thread was made by fle_thread_spawn (a timer-queue
 * timer), whose handle the guest never sees. Until the table is full nothing changes (the first 255 threads get the same places, ids and handles as before). */
static unsigned reusable_slot(void)
{
    unsigned i, best = 0;

    for (i = 1; i < nthr; i++) {
        const thr *t = &T[i];

        if (t->state != T_DONE || t == pending_free || t->teb != NULL) continue;
        if (!t->closed && !t->nbody) continue;
        if (best == 0 || t->done_seq < T[best].done_seq) best = i;
    }
    return best;
}

static void fail(uint32_t err, uint64_t ret)
{
    fle_set_last_error(err);
    R[0] = ret;
}

static void ensure_init(void)
{
    if (nthr) return;
    memset(&T[0], 0, sizeof T[0]);
    T[0].idx = 0;
    T[0].vid = FLE_EMULATED_TID;
    T[0].state = T_RUNNING;
    T[0].started = 1;
    nthr = 1;
    cur = 0;
}

/* ---- the objects a thread can wait for ---- */
/* 1 when signaled, 0 when not; *bad = 1 for a handle that is closed or not one of these objects. consume: an auto-reset event is reset by the wait that it satisfies. */
static int obj_signaled(const char *api, uint64_t h, int consume, int *bad)
{
    int i;

    *bad = 0;
    if (fle_thread_owns_handle(h)) {
        i = handle_index(h);
        if (i == -2) {
            *bad = 1;
            return 0;
        }
        if (i < 1 || (unsigned)i >= nthr) fle_stop(api, "a thread handle value that CreateThread never returned (the handle of the first thread is never issued)");
        if (T[i].closed) {
            *bad = 1;
            return 0;
        }
        return T[i].state == T_DONE;
    }
    {
        int r = fle_event_wait_state(h, consume);

        if (r < 0) {
            *bad = 1;
            return 0;
        }
        return r;
    }
}

/* The result of the wait, or PENDING. Consumes only when satisfied. */
static uint32_t eval_objects(const twait *w, unsigned who, int consume)
{
    uint32_t i;
    const fle_tqueue *q = &T[who].q;
    int bad;

    if (!w->all) {
        for (i = 0; i < w->n; i++) {
            if (obj_signaled(w->api, w->h[i], 0, &bad)) {
                if (consume) obj_signaled(w->api, w->h[i], 1, &bad);
                return i;
            }
        }
    } else if (w->n > 0) {
        int all = 1;

        for (i = 0; i < w->n && all; i++)
            if (!obj_signaled(w->api, w->h[i], 0, &bad)) all = 0;
        if (all) {
            if (consume)
                for (i = 0; i < w->n; i++) obj_signaled(w->api, w->h[i], 1, &bad);
            return 0;
        }
    }
    if (w->msg && q->exists && (q->changebits & w->mask)) return w->n;
    return PENDING;
}

static int msg_ready(const thr *t)
{
    const fle_tqueue *q = &t->q;

    if (!q->exists) return 0;
    if (q->sent_head) return 1;
    /* decision 0123: a WM_PAINT of a window with an update region, or a WM_TIMER that is due, is a message in the queue (Learn GetMessage/WaitMessage); the retrieval call makes it. The filter of a GetMessage
     * (window, ids) is applied by that call; a wake that finds nothing for its filter waits again. */
    if (fle_input_has_event && fle_input_has_event(t->idx)) return 1;               /* decision 0136: a mouse or key event of the host is waiting for this thread */
    if (t->w.kind == W_MSG_GET && fle_win_has_paint && fle_win_has_paint(t->idx)) return 1;
    if (fle_win_next_timer_tick) {
        uint64_t due = fle_win_next_timer_tick(t->idx);

        if (due != 0 && due <= now_ticks()) return 1;
    }
    if (t->w.kind == W_MSG_WAIT) return (q->changebits & (FLE_QS_POSTMESSAGE | FLE_QS_ALLPOSTMESSAGE)) != 0;
    return q->quit || fle_thread_queue_find(q, t->w.fhwnd, t->w.fmin, t->w.fmax) >= 0;
}

/* Tests the wait of thread t; when satisfied the thread becomes READY. */
static int try_wake(thr *t)
{
    uint32_t r;
    int ok = 0;

    switch (t->w.kind) {
    case W_SLEEP:
        if (now_ticks() >= t->w.deadline) {
            t->w.result = 0;
            ok = 1;
        }
        break;
    case W_OBJECTS:
        r = eval_objects(&t->w, t->idx, 1);
        if (r != PENDING) {
            t->w.result = r;
            ok = 1;
        } else if (t->w.has_deadline && now_ticks() >= t->w.deadline) {
            t->w.result = FLE_WAIT_TIMEOUT_RESULT;
            ok = 1;
        }
        break;
    case W_MSG_GET:
    case W_MSG_WAIT:
        ok = msg_ready(t);
        break;
    case W_CS:
        ok = fle_sys_cs_try_take(t->w.cs_addr, t->idx) != 0;
        break;
    case W_SENT:
        ok = t->w.sent->done || t->q.sent_head != NULL;
        break;
    case W_SUSPEND:
        ok = t->suspend == 0;
        break;
    default:
        break;
    }
    if (ok) {
        t->state = T_READY;
        t->w.kind = W_NONE;
    }
    return ok;
}

/* The waiting threads are tested in the order in which they began to wait, so that one SetEvent of an auto-reset event goes to the one that waited first. */
static void poll_waiters(void)
{
    uint64_t last = 0;

    if (fle_audio_poll) fle_audio_poll();   /* decision 0139: the device signals the notification events that the virtual clock has reached, before any wait is tested */
    for (;;) {
        thr *nx = NULL;
        unsigned i;

        for (i = 0; i < nthr; i++) {
            thr *t = &T[i];

            if (t->state != T_WAITING || t->w.seq <= last) continue;
            if (!nx || t->w.seq < nx->w.seq) nx = t;
        }
        if (!nx) return;
        last = nx->w.seq;
        try_wake(nx);
    }
}

void fle_thread_wake_check(void)
{
    if (nthr > 1) poll_waiters();
}

/* ---- the switch ---- */
static void free_pending(void)
{
    thr *p = pending_free;

    if (!p) return;
    pending_free = NULL;
#ifndef __EMSCRIPTEN__
    if (p->cstack) munmap(p->cstack, p->cstack_len + 4096u);
    p->cstack = NULL;
#elif defined(__EMSCRIPTEN__)
    fle_thread_wasm_stack_free(p->idx);
#endif
    free(p->teb);
    free(p->tls);
    free(p->fls);
    p->teb = NULL;
    p->tls = NULL;
    p->fls = NULL;
}

static void switch_to(unsigned to)
{
    thr *from = &T[cur], *t = &T[to];

    if (fle_audio_switch_note) fle_audio_switch_note(cur, to);
    if (from->state != T_DONE) save_ctx(&from->g);
    if (!t->started) t->g.tdepth = trace_depth ? trace_depth() : 0;
    tlog("switch", cur, to);
    cur = to;
    t->state = T_RUNNING;
    load_ctx(&t->g);
    set_cur_pointers(t);
#ifdef __EMSCRIPTEN__
    /* decision 0124: the stack is switched by JSPI (fle_thread_wasm.c, harness/web/threads/fle_thread_lib.js); a finished thread does not come back */
    fle_thread_wasm_transfer(from->idx, to, from->state == T_DONE);
#else
    swapcontext(&from->uc, &t->uc);
#endif
    free_pending();
}

/* Decision 0234 (mt3, PROVISIONAL): the scheduling priority of a thread. Learn "Scheduling Priorities": "The system treats all threads with the same priority as equal. The system assigns time slices in a round-robin fashion to all threads with
 * the highest priority. If none of these threads are ready to run, the system assigns time slices in a round-robin fashion to all threads with the next highest priority. If a higher-priority thread becomes available to run, the system
 * ceases to execute the lower-priority thread (without allowing it to finish using its time slice) and assigns a full time slice to the higher-priority thread." The base priority is made of the priority class of the process and the
 * priority level of the thread (the same page); all threads of this environment are in ONE process, so only the level orders them: for every class the table of the page is non-decreasing in the level (IDLE -15, LOWEST -2, BELOW_NORMAL -1, NORMAL 0,
 * ABOVE_NORMAL 1, HIGHEST 2, TIME_CRITICAL 15; the REALTIME levels -7..6 between them), so the level stands for the base priority. Off by default (fle_thread_set_priorities): every thread then counts as priority 0 and the rotation of decision 0122 is unchanged. Not modelled: the dynamic boosts (Learn "Priority Boosts"), THREAD_MODE_BACKGROUND_BEGIN's effect on the CPU priority (the pages say nothing about its value). */
static int prio_on;
void fle_thread_set_priorities(int on) { prio_on = on; }
static int32_t prio_of(unsigned i)
{
    if (!prio_on) return 0;
    if (i == 0) return fle_proc_first_thread_priority ? fle_proc_first_thread_priority() : 0;
    return T[i].ps.prio;
}

/* The runnable thread (never `cur` itself) that the scheduler takes next, or -1: the one with the highest priority, and among those with the same priority the first after `cur` in creation order (the round-robin). When all runnable
 * threads have the same priority this is the rotation of decision 0122. */
static int pick_other(void)
{
    unsigned k;
    int best = -1;
    int32_t bp = 0;

    for (k = 1; k < nthr; k++) {
        unsigned i = (cur + k) % nthr;

        if (T[i].state == T_READY && T[i].suspend == 0) {
            int32_t p = prio_of(i);

            if (best < 0 || p > bp) {
                best = (int)i;
                bp = p;
            }
        }
    }
    return best;
}

static const char *state_name(const thr *t)
{
    switch (t->state) {
    case T_READY: return t->suspend ? "suspended" : t->started ? "ready" : "ready (not started)";
    case T_RUNNING: return "running";
    case T_WAITING: return "waiting";
    default: return "done";
    }
}

static void describe_wait(const thr *t, char *out, size_t n)
{
    const twait *w = &t->w;
    char dl[64] = "no time-out";
    uint32_t i;
    size_t k;

    if (w->has_deadline) snprintf(dl, sizeof dl, "deadline %" PRIu64 " ticks", w->deadline);
    switch (w->kind) {
    case W_SLEEP: snprintf(out, n, "%s: sleep, %s", w->api ? w->api : "?", dl); break;
    case W_OBJECTS:
        k = (size_t)snprintf(out, n, "%s: %u object(s)%s", w->api ? w->api : "?", (unsigned)w->n, w->all ? " (all)" : " (any)");
        for (i = 0; i < w->n && k + 24 < n; i++) k += (size_t)snprintf(out + k, n - k, " %#" PRIx64, w->h[i]);
        if (w->msg && k + 40 < n) k += (size_t)snprintf(out + k, n - k, " + input mask %#x", (unsigned)w->mask);
        if (k + 40 < n) snprintf(out + k, n - k, ", %s", dl);
        break;
    case W_MSG_GET: snprintf(out, n, "%s: GetMessage, filter hwnd %#" PRIx64 " ids %#x..%#x, nothing matches", w->api ? w->api : "?", w->fhwnd, (unsigned)w->fmin, (unsigned)w->fmax); break;
    case W_MSG_WAIT: snprintf(out, n, "%s: WaitMessage", w->api ? w->api : "?"); break;
    case W_CS: snprintf(out, n, "%s: critical section %#" PRIx64 " owned by another thread", w->api ? w->api : "?", w->cs_addr); break;
    case W_SENT: snprintf(out, n, "%s: a message it sent is not processed yet", w->api ? w->api : "?"); break;
    case W_SUSPEND: snprintf(out, n, "%s: suspended itself", w->api ? w->api : "?"); break;
    default: snprintf(out, n, "-");
    }
}

static void report_threads(FILE *f)
{
    unsigned i;
    char wd[400];

    fprintf(f, "threads: %u (running index %u), virtual clock %" PRIu64 " ticks (%" PRIu64 " ms)\n", nthr, nthr ? cur : 0u, now_ticks(), now_ticks() / FLE_TICKS_PER_MS);
    for (i = 0; i < nthr; i++) {
        const thr *t = &T[i];
        const fle_tqueue *q = &t->q;
        unsigned nsent = 0;
        const fle_sent *s;

        for (s = q->sent_head; s; s = s->next) nsent++;
        wd[0] = 0;
        if (t->state == T_WAITING) describe_wait(t, wd, sizeof wd);
        fprintf(f, "  thread %u: id %#x handle %#" PRIx64 " %s%s start %#" PRIx64 " param %#" PRIx64 " exit %u suspend %u rsp %#" PRIx64 "; queue %s%u posted %u sent%s\n", i, (unsigned)t->vid,
                i ? handle_of(i) : (uint64_t)0, state_name(t), t->closed ? " (handle closed)" : "", t->start, t->param, (unsigned)t->exit_code, (unsigned)t->suspend,
                i == cur ? R[4] : t->g.r[4], q->exists ? "" : "none ", q->exists ? (unsigned)q->count : 0u, nsent, q->quit ? " QUIT pending" : "");
        if (wd[0]) fprintf(f, "      waiting: %s\n", wd);
        if (q->exists) {
            uint32_t k;

            for (k = 0; k < q->count && k < 8; k++) {
                const fle_tmsg *m = &q->q[(q->head + k) % q->cap];

                fprintf(f, "      posted #%u: hwnd %#" PRIx64 " msg %#x wp %#" PRIx64 " lp %#" PRIx64 " time %u\n", (unsigned)k, m->hwnd, (unsigned)m->msg, m->wp, m->lp, (unsigned)m->time);
            }
        }
    }
}

/* decision 0147: the counters of the pseudo-preemption, once it has ended a slice */
static void report_preempt(FILE *f)
{
    if (preempt_armed && (pst.slices || pst.deferred))
        fprintf(f, "preemption (decision 0147): slice = %" PRIu64 " indirect calls = %" PRIu64 " ticks; slices %" PRIu64 ", skipped inside a callback %" PRIu64 ", switches %" PRIu64 ", progress %" PRIu64 "\n",
                slice_dispatches, slice_ticks, pst.slices, pst.deferred, pst.switches, fle_progress_count);
}

void fle_thread_report(FILE *f)
{
    report_threads(f);
    report_preempt(f);
    if (fle_win_report) fle_win_report(f);
}

/* Stops by name; the message carries the whole thread table (and the window table when fle_win.c is linked). */
static void stop_report(const char *api, const char *head) __attribute__((noreturn));
static void stop_report(const char *api, const char *head)
{
    char *buf = NULL;
    size_t len = 0;
    FILE *m = open_memstream(&buf, &len);
    char why[6000];

    if (m) {
        report_threads(m);
        report_preempt(m);
        if (fle_win_report) fle_win_report(m);
        fclose(m);
        snprintf(why, sizeof why, "%s\n%s", head, buf ? buf : "");
        free(buf);
    } else {
        snprintf(why, sizeof why, "%s", head);
    }
    fle_stop(api, why);
}

static int is_input_wait(const thr *t)
{
    if (t->state != T_WAITING) return 0;
    if (t->w.kind == W_MSG_GET || t->w.kind == W_MSG_WAIT) return 1;
    return t->w.kind == W_OBJECTS && t->w.msg && (t->w.mask & ~FLE_QS_SENDMESSAGE) != 0;
}

static int next_deadline(uint64_t *out)
{
    unsigned i;
    int found = 0;

    for (i = 0; i < nthr; i++) {
        const thr *t = &T[i];

        if (t->state != T_WAITING) continue;
        if (t->w.has_deadline) {
            if (!found || t->w.deadline < *out) *out = t->w.deadline;
            found = 1;
        }
        /* decision 0123: a thread that waits for a message (GetMessage, WaitMessage, MsgWait with input) is also woken by the due time of one of its window timers (SetTimer) */
        if ((t->w.kind == W_MSG_GET || t->w.kind == W_MSG_WAIT || (t->w.kind == W_OBJECTS && t->w.msg)) && fle_win_next_timer_tick) {
            uint64_t due = fle_win_next_timer_tick(t->idx);

            if (due != 0 && (!found || due < *out)) {
                *out = due;
                found = 1;
            }
        }
    }
    if (fle_audio_next_tick) {
        uint64_t due;

        if (fle_audio_next_tick(&due) && (!found || due < *out)) {
            *out = due;
            found = 1;
        }
    }
    return found;
}

/* The caller made `cur` WAITING, DONE or (a yield, or the end of a slice) READY. Returns when `cur` runs again: 1 when another thread ran in between, 0 when not.
 * Decision 0234 (mt3): with `slice` set the caller is a thread that is still running whose time slice has ended (fle_progress_slice): Learn "Scheduling Priorities" lets it go on unless a thread of the same or a higher priority is ready.
 * A thread that gives the processor up itself (Sleep(0), SwitchToThread: Learn Sleep "relinquish the remainder of its time slice to any other thread that is ready to run") or that waits goes to the ready thread of the highest priority. */
static int schedule_ex(int slice)
{
    for (;;) {
        int o;
        uint64_t dl;

#if defined(__EMSCRIPTEN__) || defined(FLE_RING_POLL_HOOK)
        if (fle_input_ring_poll) fle_input_ring_poll();
#endif
        /* decision 0280 (PROVISIONAL): the host's input ring is read at every scheduler point, so that an input need not wait for the engine's idle */
        poll_waiters();
        o = pick_other();
        if (o >= 0 && !(slice && prio_of((unsigned)o) < prio_of(cur))) {
            switch_to((unsigned)o);
            return 1;
        }
        if (T[cur].state == T_READY && T[cur].suspend == 0) {
            T[cur].state = T_RUNNING;
            return 0;
        }
        if (idle_stop && is_input_wait(&T[0]) && T[0].q.exists && T[0].q.count == 0 && !T[0].q.sent_head && !T[0].q.quit)
            stop_report("MessageLoop", "idle: the first thread waits for input with an empty queue and no other thread can run (only the virtual clock can move)");
        if (!idle_stop && fle_thread_idle_hook && is_input_wait(&T[0]) && T[0].q.exists && T[0].q.count == 0 && !T[0].q.sent_head && !T[0].q.quit) {
            uint64_t nx = 0, adv = 0;
            int hr, have = next_deadline(&nx);

            hr = fle_thread_idle_hook(now_ticks(), have ? nx : 0u, &adv);
            if (hr == 0) stop_report("MessageLoop", "idle: the host ended the wait (no event came; the session was closed)");
            if (hr == 2 && adv > now_ticks()) {
                if (have && adv > nx) adv = nx;                     /* a deadline is never skipped */
                fle_sys_clock_advance_to(adv);
            }
            continue;
        }
        /* decision 0139: with an external audio consumer (the AudioWorklet) the idle host above has the first say (it knows the page); when it does not apply (the first thread is not waiting for input) a buffer that plays paces the clock in real time */
        if (fle_audio_paced && fle_audio_wait_until && fle_audio_paced() && next_deadline(&dl)) {
            if (dl > now_ticks()) {
                int w = fle_audio_wait_until(dl);

                if (w == 0) stop_report("MessageLoop", "idle: the browser ended the wait (the session was closed by the page while the audio device played)");
                if (w == 1) continue;   /* an input event was posted: evaluate again */
                if (dl > now_ticks()) fle_sys_clock_advance_to(dl);
            }
            continue;
        }
        if (!next_deadline(&dl)) stop_report("scheduler", "deadlock: no thread can run and no thread waits with a time-out (nothing can wake any of them)");
        if (dl > now_ticks()) fle_sys_clock_advance_to(dl);
    }
}

static int schedule(void) { return schedule_ex(0); }

/* cur gives up the processor for the wait that begin_wait filled in. Returns when the wait is over. */
static void park(void)
{
    T[cur].state = T_WAITING;
    T[cur].w.seq = ++wait_seq;
    schedule();
    T[cur].state = T_RUNNING;
}

static void begin_wait(int kind, const char *api, uint32_t ms)
{
    twait *w = &T[cur].w;

    memset(w, 0, sizeof *w);
    w->kind = kind;
    w->api = api;
    if (ms != INFINITE_ && (kind == W_SLEEP || kind == W_OBJECTS)) {
        w->has_deadline = 1;
        w->deadline = ms_deadline(ms);
    }
}

/* ---- Sleep, SwitchToThread, DwmFlush ---- */
void fle_thread_sleep(const char *api, uint32_t ms)
{
    if (nthr < 2) {
        if (ms == INFINITE_) fle_stop(api, "INFINITE would never return (no other thread exists)");
        fle_sys_clock_advance_to(ms_deadline(ms));
        return;
    }
    if (ms == 0) {
        T[cur].state = T_READY;
        schedule();
        return;
    }
    begin_wait(W_SLEEP, api, ms);
    park();
}

int fle_thread_switch(void)
{
    if (nthr < 2) return 0;
    T[cur].state = T_READY;
    return schedule();
}

/* Decision 0147 (PROVISIONAL). Called by fle_dispatch (fle_dispatch.h) when fle_progress_count reaches fle_progress_limit, i.e. once per slice_dispatches indirect calls of the guest after the first CreateThread.
 * The caller is guest code (never a runtime function: fle_dispatch is reached only from lifted code and from fle_call_guest), so no runtime table is half-updated by the running thread; every register of the
 * guest is in R/X/FL ... and is saved by the switch. Inside a callback of the runtime (a window procedure under DispatchMessageW, an EnumWindows callback ...) the slice is skipped: the runtime function below
 * that guest frame may hold a position in one of its tables. */
void fle_progress_slice(void)
{
    thr *t;

    fle_progress_limit = fle_progress_count + slice_dispatches;
    if (!preempt_armed || nthr < 2) return;
    t = &T[cur];
    if (t->state != T_RUNNING) return;
    if (fle_callback_depth() > FLE_PREEMPT_MAX_DEPTH) {
        pst.deferred++;
        return;
    }
    pst.slices++;
    pst.ticks += slice_ticks;
    fle_sys_clock_advance_to(now_ticks() + slice_ticks);
    t->state = T_READY;
    if (schedule_ex(1)) pst.switches++;
}

void fle_thread_preempt_stats(fle_preempt_stats *out)
{
    *out = pst;
    out->armed = preempt_armed;
    out->progress = fle_progress_count;
}

static void preempt_arm(void)
{
    if (preempt_armed || preempt_env_read) return;
    preempt_env_read = 1;
    preempt_armed = 1;
    fle_progress_limit = fle_progress_count + slice_dispatches;
}

/* ui1 (decision 0166): the refresh rate of the virtual display. FLE_DWM_HZ in the environment (read once) overrides the compile-time default FLE_DWM_HZ (60); 24..1000 are accepted. */
uint32_t fle_display_hz(void)
{
    static uint32_t hz;

    if (!hz) {
        const char *e = getenv("FLE_DWM_HZ");
        unsigned long v = 0;

        hz = FLE_DWM_HZ;
        if (e && e[0]) {
            char *end = NULL;

            v = strtoul(e, &end, 10);
            if (end && *end == 0 && v >= 24ul && v <= 1000ul) hz = (uint32_t)v;
            else fprintf(stderr, "fle: FLE_DWM_HZ=\"%s\" is not a number of hertz in 24..1000: ignored, %u Hz\n", e, (unsigned)hz);
        }
    }
    return hz;
}

/* ui1 (decision 0166): the number EnumDisplaySettingsW reports (fle_monitor.c calls this through a weak reference). FLE_DISPLAYSETTINGS_FREQUENCY in the environment (24..1000) if set, else the same value as DwmFlush (fle_display_hz), else the compile-time default
 * FLE_DISPLAYSETTINGS_FREQUENCY of fle_monitor.h. If both variables are set and differ, a line goes to stderr. */
#define FLE_DISPLAYSETTINGS_FREQUENCY 60
uint32_t fle_display_settings_hz(void)
{
    static uint32_t hz;

    if (!hz) {
        const char *e = getenv("FLE_DISPLAYSETTINGS_FREQUENCY");
        uint32_t dwm = fle_display_hz();

        hz = dwm;
        if (e && e[0]) {
            char *end = NULL;
            unsigned long v = strtoul(e, &end, 10);

            if (end && *end == 0 && v >= 24ul && v <= 1000ul) {
                hz = (uint32_t)v;
                if (hz != dwm) fprintf(stderr, "fle: FLE_DISPLAYSETTINGS_FREQUENCY=%u differs from DwmFlush's %u Hz (FLE_DWM_HZ): the guest reads one and is paced by the other\n", (unsigned)hz, (unsigned)dwm);
            } else fprintf(stderr, "fle: FLE_DISPLAYSETTINGS_FREQUENCY=\"%s\" is not a number of hertz in 24..1000: ignored\n", e);
        }
    }
    return hz;
}

/* DwmFlush waits for the next boundary of the virtual refresh (60 Hz unless FLE_DWM_HZ says otherwise) of the virtual clock, strictly after now: frame k begins at ceil(k * 10^7 / HZ) ticks (100 ns) since the virtual boot. */
uint64_t fle_thread_next_frame_tick(uint64_t now)
{
    uint64_t hz = fle_display_hz();
    uint64_t k = now * hz / 10000000u + 1u;

    return (k * 10000000u + (hz - 1u)) / hz;
}

void fle_imp_DwmFlush(void)
{
    uint64_t dl = fle_thread_next_frame_tick(now_ticks());

    if (nthr < 2) {
        fle_sys_clock_advance_to(dl);
        R[0] = 0;
        return;
    }
    begin_wait(W_SLEEP, "DwmFlush", INFINITE_);
    T[cur].w.has_deadline = 1;
    T[cur].w.deadline = dl;
    park();
    R[0] = 0;
}

/* ---- thread creation and exit ---- */
static uint64_t va_alloc(uint64_t size)
{
    uint64_t sv[5] = { R[0], R[1], R[2], R[8], R[9] }, base;

    R[1] = 0;
    R[2] = size;
    R[8] = MEM_COMMIT_RESERVE;
    R[9] = PAGE_READWRITE_;
    fle_imp_VirtualAlloc();
    base = R[0];
    R[0] = sv[0];
    R[1] = sv[1];
    R[2] = sv[2];
    R[8] = sv[3];
    R[9] = sv[4];
    return base;
}

static void va_free(uint64_t base)
{
    uint64_t sv[4] = { R[0], R[1], R[2], R[8] };

    R[1] = base;
    R[2] = 0;
    R[8] = MEM_RELEASE_;
    fle_imp_VirtualFree();
    R[0] = sv[0];
    R[1] = sv[1];
    R[2] = sv[2];
    R[8] = sv[3];
}

static int thread_close_hook(uint64_t h)
{
    int i;

    if (!fle_thread_owns_handle(h)) return 0;
    i = handle_index(h);
    if (i == -2) {
        fail(FLE_W32_ERR_INVALID_HANDLE, 0);
        return 1;
    }
    if (i < 1 || (unsigned)i >= nthr) fle_stop("CloseHandle", "thread handle value was not issued by this runtime");
    if (T[i].closed) {
        fail(FLE_W32_ERR_INVALID_HANDLE, 0);
        return 1;
    }
    T[i].closed = 1;
    R[0] = 1;
    return 1;
}

static void thread_finish(uint32_t code) __attribute__((noreturn));

static void thread_main(void)
{
    thr *t = &T[cur];
    uint32_t code;

    free_pending();
    t->started = 1;
    tlog("start", t->idx, 0);
    if (fle_realmod_thread_notify && FLE_REALMOD_PRESENT()) fle_realmod_thread_notify(2);
    if (t->nbody) {
        t->nbody(t->narg);
        thread_finish(0);
    }
    code = (uint32_t)fle_call_guest(t->start, t->param, 0, 0, 0);
    thread_finish(code);
}

static void thread_finish(uint32_t code)
{
    thr *t = &T[cur];

    if (fle_realmod_thread_notify && FLE_REALMOD_PRESENT()) fle_realmod_thread_notify(3);
    t->exit_code = code;
    tlog("exit", t->idx, code);
    if (t->stack_base) va_free(t->stack_base);
    t->stack_base = 0;
    t->state = T_DONE;
    t->done_seq = ++done_clock;
    t->w.kind = W_NONE;
    pending_free = t;
    schedule();
    fle_stop("scheduler", "a finished thread was resumed");
}

#ifdef __EMSCRIPTEN__
/* The body of a new thread on its own JSPI stack (called by the exported entry of fle_thread_wasm.c, which the JS scheduler starts with WebAssembly.promising): the switch that
 * started it has made it the current thread. It does not return: thread_finish hands over to the next thread and the stack is ended. */
void fle_thread_wasm_entry_cur(unsigned idx)
{
    if (idx != cur) fle_stop("scheduler", "a JSPI stack was started for a thread that is not the current one");
    thread_main();
}
#endif

/* The body of CreateThread (R[0] = the handle, or 0 with the last error set). `nbody` is NULL for a CreateThread of the guest; fle_thread_spawn passes a C function (decision 0171). */
static void create_thread_core(const char *api, uint64_t stk, uint64_t start, uint64_t param, uint32_t flags, uint64_t ptid, void (*nbody)(uint64_t), uint64_t narg)
{
    (void)api;
    {
        uint64_t reserve, gbase, top;
        unsigned i, slot;
        thr *t;

        ensure_init();
        slot = nthr;
        if (nthr >= FLE_THREAD_MAX) slot = reusable_slot();   /* decision 0196: when the table is full, the place of a finished thread nobody can name any more */
        if (slot == 0) {
            fail(8, 0); /* ERROR_NOT_ENOUGH_MEMORY (8): the page names no code for a limit */
            return;
        }
        if (next_hser + 2u >= FLE_THREAD_REGION_SIZE / 4u) fle_stop(api, "the thread handle region is exhausted");
        /* The guest stack: with STACK_SIZE_PARAM_IS_A_RESERVATION dwStackSize is the reserve size, else the reserve is the default (1 MiB) and dwStackSize is the commit size (Learn). Here everything is committed (a choice),
         * and at least 64 KiB so that the first frames fit (a choice). */
        reserve = (flags & FLE_STACK_SIZE_PARAM_IS_A_RESERVATION) && stk ? stk : FLE_THREAD_DEFAULT_STACK;
        reserve = (reserve + FLE_PAGE_SIZE - 1) & ~(FLE_PAGE_SIZE - 1);
        if (reserve < 0x10000) reserve = 0x10000;
        gbase = va_alloc(reserve);
        if (!gbase) {
            R[0] = 0;
            return; /* the last error was set by VirtualAlloc */
        }
        i = slot;
        t = &T[i];
        if (i < nthr) free(t->q.q);   /* a place used before: the queue of the finished thread (its sent messages are gone with it) */
        memset(t, 0, sizeof *t);
        t->idx = i;
        t->vid = FLE_EMULATED_TID + i;
        t->hser = next_hser++;
        t->start = start;
        t->param = param;
        t->nbody = nbody;
        t->narg = narg;
        t->stack_base = gbase;
        t->stack_size = reserve;
        t->teb = (uint8_t *)calloc(1, FLE_TEB_SIZE);
        t->tls = (uint64_t *)calloc(FLE_THREAD_TLS_SLOTS, sizeof(uint64_t));
        t->fls = (uint64_t *)calloc(FLE_THREAD_FLS_SLOTS, sizeof(uint64_t));
#ifdef __EMSCRIPTEN__
        /* the host C stack of the thread is a region for the wasm shadow stack (__stack_pointer); the JSPI stack itself belongs to V8 (decision 0124) */
        if (!t->teb || !t->tls || !t->fls || !fle_thread_wasm_stack_alloc(i)) {
            free(t->teb);
            free(t->tls);
            free(t->fls);
            va_free(gbase);
            memset(t, 0, sizeof *t);
            if (i < nthr) {   /* a place used before stays a free place */
                t->idx = i;
                t->state = T_DONE;
                t->closed = 1;
                t->done_seq = ++done_clock;
            }
            fail(8, 0);
            return;
        }
#else
        t->cstack_len = (size_t)FLE_THREAD_CSTACK_BYTES;
        t->cstack = mmap(NULL, t->cstack_len + 4096u, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_NORESERVE | MAP_STACK, -1, 0);
        if (!t->teb || !t->tls || !t->fls || t->cstack == MAP_FAILED) {
            if (t->cstack && t->cstack != MAP_FAILED) munmap(t->cstack, t->cstack_len + 4096u);
            free(t->teb);
            free(t->tls);
            free(t->fls);
            va_free(gbase);
            memset(t, 0, sizeof *t);
            if (i < nthr) {   /* a place used before stays a free place */
                t->idx = i;
                t->state = T_DONE;
                t->closed = 1;
                t->done_seq = ++done_clock;
            }
            fail(8, 0);
            return;
        }
        mprotect(t->cstack, 4096u, PROT_NONE); /* the guard page at the low end of the host stack */
#endif
        top = gbase + reserve;
        memcpy(t->teb + FLE_TEB_STACKBASE, &top, 8);
        memcpy(t->teb + FLE_TEB_STACKLIMIT, &gbase, 8);
        /* The initial guest state: nothing is inherited (Learn does not say); the state of a thread at its start is the reset state of the machine: flags undefined, x87 reset (037F, 0, FFFF), MXCSR 1F80. */
        memset(&t->g, 0, sizeof t->g);
        t->g.fl.cf = t->g.fl.pf = t->g.fl.af = t->g.fl.zf = t->g.fl.sf = t->g.fl.of = FLE_UNDEF;
        t->g.x87.cw = 0x037F;
        t->g.x87.sw = 0;
        t->g.x87.tw = 0xFFFF;
        t->g.mxcsr = 0x1F80;
        t->g.r[4] = (top - 64u) & ~UINT64_C(15);
#ifndef __EMSCRIPTEN__
        getcontext(&t->uc);
        t->uc.uc_stack.ss_sp = (char *)t->cstack + 4096u;
        t->uc.uc_stack.ss_size = t->cstack_len;
        t->uc.uc_link = NULL;
        makecontext(&t->uc, (void (*)(void))thread_main, 0);
#endif
        t->ps.prio = 0;
        t->state = T_READY;
        t->suspend = (flags & FLE_CREATE_SUSPENDED) ? 1u : 0u;
        if (i == nthr) nthr++;
        preempt_arm();
        if (!hook_registered) {
            fle_vfs_add_close_hook(thread_close_hook);
            hook_registered = 1;
        }
        if (ptid) fle_st(ptid, 4, t->vid);
        tlog("create", i, (unsigned)flags);
        R[0] = handle_of(i);
    }
}

void fle_imp_CreateThread(void)
{
    static const char api[] = "CreateThread";
    uint64_t attrs = R[1], stk = R[2], start = R[8], param = R[9];
    uint32_t flags = (uint32_t)fle_arg(5);
    uint64_t ptid = fle_arg(6);

    if (attrs != 0) fle_stop(api, "lpThreadAttributes is not NULL: security descriptors and inheritance are not modelled (decision 0122)");
    if (flags & ~(FLE_CREATE_SUSPENDED | FLE_STACK_SIZE_PARAM_IS_A_RESERVATION)) {
        char why[160];

        snprintf(why, sizeof why, "dwCreationFlags %#x has a bit that the page does not list (CREATE_SUSPENDED 0x4, STACK_SIZE_PARAM_IS_A_RESERVATION 0x10000)", (unsigned)flags);
        fle_stop(api, why);
    }
    create_thread_core(api, stk, start, param, flags, ptid, NULL, 0);
}

/* Decision 0171: a new cooperative thread (the same table, stacks and scheduling as CreateThread) whose body is the C function `body(arg)`; guest code it runs goes through fle_call_guest. Returns the
 * thread's handle (0 on failure; the last error is set). Used by the timer queue (fle_timerq.c). The thread ends when `body` returns. */
uint64_t fle_thread_spawn(void (*body)(uint64_t), uint64_t arg)
{
    uint64_t saved = R[0];
    uint64_t h;

    create_thread_core("fle_thread_spawn", 0, 0, 0, 0, 0, body, arg);
    h = R[0];
    R[0] = saved;
    return h;
}

/* The thread that `h` names: a handle that CreateThread returned (NULL when it was closed), or the pseudo handle of GetCurrentThread (the running thread). */
static thr *thread_of(const char *api, uint64_t h)
{
    int i;

    ensure_init();
    if (h == FLE_PSEUDO_THREAD_HANDLE) return &T[cur];
    if (!fle_thread_owns_handle(h)) fle_stop(api, "handle was not issued as a thread by this runtime (other handle kinds, INVALID_HANDLE_VALUE and NULL are not defined here)");
    i = handle_index(h);
    if (i == -2) return NULL;
    if (i < 1 || (unsigned)i >= nthr) fle_stop(api, "thread handle value was never returned by CreateThread");
    if (T[i].closed) return NULL;
    return &T[i];
}

void fle_thread_note_priority(uint64_t handle, int32_t level)
{
    int i = -1;

    if (!trace_on || !nthr) return;
    if (handle == FLE_PSEUDO_THREAD_HANDLE) i = (int)cur;
    else if (fle_thread_owns_handle(handle)) i = handle_index(handle);
    if (i >= 0) tlog("priority-set", (unsigned)i, (unsigned)level);
}

fle_thread_pstate *fle_thread_pstate_of(uint64_t handle, int *ok)
{
    thr *t;

    *ok = 1;
    if (handle == FLE_PSEUDO_THREAD_HANDLE) return nthr && cur ? &T[cur].ps : NULL;
    if (!fle_thread_owns_handle(handle)) {
        *ok = 0;
        return NULL;
    }
    t = thread_of("SetThreadPriority", handle);
    if (!t) {
        *ok = 0;
        return NULL;
    }
    return t->idx ? &t->ps : NULL;
}

void fle_imp_ResumeThread(void)
{
    static const char api[] = "ResumeThread";
    thr *t = thread_of(api, R[1]);
    uint32_t prev;

    if (!t) {
        fail(FLE_W32_ERR_INVALID_HANDLE, 0xFFFFFFFFu);
        return;
    }
    if (t->state == T_DONE) fle_stop(api, "ResumeThread of a thread that has terminated: the page does not say what it returns");
    prev = t->suspend;
    if (prev > 0) t->suspend--;
    R[0] = prev;
    if (prev > 0 && t->suspend == 0) fle_thread_wake_check();
}

void fle_imp_SuspendThread(void)
{
    static const char api[] = "SuspendThread";
    thr *t = thread_of(api, R[1]);
    uint32_t prev;

    if (!t) {
        fail(FLE_W32_ERR_INVALID_HANDLE, 0xFFFFFFFFu);
        return;
    }
    if (t->state == T_DONE) fle_stop(api, "SuspendThread of a thread that has terminated: the page does not say what it returns");
    if (t->suspend >= FLE_MAXIMUM_SUSPEND_COUNT) fle_stop(api, "the suspend count would pass MAXIMUM_SUSPEND_COUNT: the page gives no error code");
    prev = t->suspend;
    t->suspend++;
    if (t == &T[cur]) {
        /* a thread that suspends itself stops here until another thread resumes it */
        if (nthr < 2) fle_stop(api, "the only thread suspends itself: nothing can resume it");
        begin_wait(W_SUSPEND, api, INFINITE_);
        park();
    }
    R[0] = prev;
}

void fle_imp_ExitThread(void)
{
    uint32_t code = (uint32_t)R[1];

    if (!nthr || cur == 0) fle_stop("ExitThread", "the first thread (the one that entered the guest) called ExitThread: the process would end (not modelled)");
    thread_finish(code);
}

void fle_imp_GetExitCodeThread(void)
{
    static const char api[] = "GetExitCodeThread";
    uint64_t pcode = R[2];
    thr *t = thread_of(api, R[1]);

    if (pcode == 0) fle_stop(api, "lpExitCode is NULL: the page gives no behaviour");
    if (!t) {
        fail(FLE_W32_ERR_INVALID_HANDLE, 0);
        return;
    }
    fle_st(pcode, 4, t->state == T_DONE ? t->exit_code : FLE_STILL_ACTIVE);
    R[0] = 1;
}

/* ---- WaitForSingleObject (fle_event.c), WaitForMultipleObjects and MsgWaitForMultipleObjects ---- */
uint32_t fle_thread_wait_objects(const char *api, uint32_t n, const uint64_t *h, int all, uint32_t ms, int msg, uint32_t mask)
{
    twait w;
    uint32_t r, i, j;
    int bad;

    ensure_init();
    for (i = 0; i < n; i++) {
        (void)obj_signaled(api, h[i], 0, &bad);
        if (bad) {
            fle_set_last_error(FLE_W32_ERR_INVALID_HANDLE);
            return FLE_WAIT_FAILED_RESULT;
        }
        for (j = 0; j < i; j++)
            if (h[j] == h[i]) fle_stop(api, "the handle array holds the same handle twice: the page says it may not (behaviour not given)");
    }
    memset(&w, 0, sizeof w);
    w.api = api;
    w.n = n;
    memcpy(w.h, h, n * sizeof h[0]);
    w.all = all;
    w.msg = msg;
    w.mask = mask;
    if (msg) fle_thread_queue(cur, 1);
    r = eval_objects(&w, cur, 1);
    if (r != PENDING) {
        if (msg) T[cur].q.changebits = 0;
        return r;
    }
    if (ms == 0) {
        if (msg) T[cur].q.changebits = 0;
        return FLE_WAIT_TIMEOUT_RESULT;
    }
    if (nthr < 2) {
        fle_stop(api, ms == INFINITE_ ? "INFINITE time-out on objects that are not signaled would never return (no other thread exists)"
                                       : "a finite time-out on objects that are not signaled: no other thread exists, so only the virtual clock could end it (not modelled for one thread)");
    }
    begin_wait(W_OBJECTS, api, ms);
    T[cur].w.n = n;
    memcpy(T[cur].w.h, h, n * sizeof h[0]);
    T[cur].w.all = all;
    T[cur].w.msg = msg;
    T[cur].w.mask = mask;
    park();
    r = T[cur].w.result;
    if (msg) T[cur].q.changebits = 0;
    return r;
}

void fle_imp_WaitForMultipleObjects(void)
{
    static const char api[] = "WaitForMultipleObjects";
    uint32_t n = (uint32_t)R[1], all = (uint32_t)R[8] != 0, ms = (uint32_t)R[9];
    uint64_t ph = R[2], h[FLE_MAXIMUM_WAIT_OBJECTS];
    uint32_t i;

    if (n == 0 || n > FLE_MAXIMUM_WAIT_OBJECTS) {
        fail(FLE_W32_ERR_INVALID_PARAMETER, FLE_WAIT_FAILED_RESULT); /* the page: nCount cannot be zero, and at most MAXIMUM_WAIT_OBJECTS; no code is named: 87 is a choice */
        return;
    }
    if (ph == 0) fle_stop(api, "lpHandles is NULL: the page gives no behaviour");
    for (i = 0; i < n; i++) h[i] = fle_ld(ph + 8u * i, 8);
    R[0] = fle_thread_wait_objects(api, n, h, (int)all, ms, 0, 0);
}

/* kernel32!WaitForMultipleObjectsEx: DWORD f(DWORD nCount rcx, const HANDLE *lpHandles rdx, BOOL bWaitAll r8d, DWORD dwMilliseconds r9d, BOOL bAlertable [5]) (decision 0126, PROVISIONAL, wall 66). Learn
 * (WaitForMultipleObjectsEx): "Waits until one or all of the specified objects are in the signaled state, an I/O completion routine or asynchronous procedure call (APC) is queued to the thread, or the time-out interval
 * elapses"; bAlertable: "If this parameter is TRUE and the thread is in the waiting state, the function returns when the system queues an I/O completion routine or APC, and the thread runs the routine or function.
 * Otherwise, the function does not return and the completion routine or APC function is not executed"; "A completion routine is queued when the ReadFileEx or WriteFileEx function in which it was specified has
 * completed ... An APC is queued when you call QueueUserAPC"; WAIT_IO_COMPLETION 0xC0 is the result when an APC ended the wait. This runtime answers no ReadFileEx, WriteFileEx or QueueUserAPC (no row), so no
 * completion routine and no APC can ever be queued to a thread: bAlertable changes nothing, and the wait is the wait of WaitForMultipleObjects (S-alertable-no-apc: it follows from the missing APIs; it is revised the
 * day one of them is answered). The guest passes bAlertable 0 or 1 (the call at 0x481242 passes 0 in the fifth slot). Every other rule is WaitForMultipleObjects'. */
void fle_imp_WaitForMultipleObjectsEx(void)
{
    static const char api[] = "WaitForMultipleObjectsEx";
    uint32_t n = (uint32_t)R[1], all = (uint32_t)R[8] != 0, ms = (uint32_t)R[9];
    uint64_t ph = R[2], h[FLE_MAXIMUM_WAIT_OBJECTS];
    uint32_t i;

    if (n == 0 || n > FLE_MAXIMUM_WAIT_OBJECTS) {
        fail(FLE_W32_ERR_INVALID_PARAMETER, FLE_WAIT_FAILED_RESULT);
        return;
    }
    if (ph == 0) fle_stop(api, "lpHandles is NULL: the page gives no behaviour");
    for (i = 0; i < n; i++) h[i] = fle_ld(ph + 8u * i, 8);
    R[0] = fle_thread_wait_objects(api, n, h, (int)all, ms, 0, 0);
}

void fle_imp_MsgWaitForMultipleObjects(void)
{
    static const char api[] = "MsgWaitForMultipleObjects";
    uint32_t n = (uint32_t)R[1], all = (uint32_t)R[8] != 0, ms = (uint32_t)R[9], mask = (uint32_t)fle_arg(5);
    uint64_t ph = R[2], h[FLE_MAXIMUM_WAIT_OBJECTS];
    uint32_t i;

    if (n > FLE_MAXIMUM_WAIT_OBJECTS - 1u) {
        fail(FLE_W32_ERR_INVALID_PARAMETER, FLE_WAIT_FAILED_RESULT);
        return;
    }
    if (all) fle_stop(api, "fWaitAll TRUE (all the objects signaled and an input event): not modelled");
    if (n > 0 && ph == 0) fle_stop(api, "pHandles is NULL with nCount > 0: the page gives no behaviour");
    for (i = 0; i < n; i++) h[i] = fle_ld(ph + 8u * i, 8);
    R[0] = fle_thread_wait_objects(api, n, h, 0, ms, 1, mask);
}

/* ---- critical sections (fle_sys.c) ---- */
void fle_thread_wait_cs(const char *api, uint64_t addr)
{
    begin_wait(W_CS, api, INFINITE_);
    T[cur].w.cs_addr = addr;
    park();
}

/* ---- the message queues ---- */
fle_tqueue *fle_thread_queue(unsigned idx, int create)
{
    ensure_init();
    if (idx >= nthr) return NULL;
    if (!T[idx].q.exists) {
        if (!create) return NULL;
        T[idx].q.exists = 1;
    }
    return &T[idx].q;
}

static int match(const fle_tmsg *m, uint64_t fhwnd, uint32_t fmin, uint32_t fmax)
{
    if (fhwnd == (uint64_t)-1) {
        if (m->hwnd != 0) return 0;
    } else if (fhwnd != 0 && m->hwnd != fhwnd) {
        return 0;
    }
    if (fmin == 0 && fmax == 0) return 1;
    return m->msg >= fmin && m->msg <= fmax;
}

int fle_thread_queue_find(const fle_tqueue *q, uint64_t fhwnd, uint32_t fmin, uint32_t fmax)
{
    uint32_t i;

    for (i = 0; i < q->count; i++)
        if (match(&q->q[(q->head + i) % q->cap], fhwnd, fmin, fmax)) return (int)i;
    return -1;
}

fle_tmsg fle_thread_queue_remove(fle_tqueue *q, int pos)
{
    fle_tmsg m = q->q[(q->head + (uint32_t)pos) % q->cap];
    uint32_t i;

    for (i = (uint32_t)pos; i + 1 < q->count; i++) q->q[(q->head + i) % q->cap] = q->q[(q->head + i + 1) % q->cap];
    q->count--;
    return m;
}

int fle_thread_post(unsigned idx, uint64_t hwnd, uint32_t msg, uint64_t wp, uint64_t lp)
{
    fle_tqueue *q = fle_thread_queue(idx, 1);
    fle_tmsg *m;

    if (!q) return 0;
    if (q->count >= FLE_QUEUE_LIMIT) return 0;
    if (q->count == q->cap) {
        uint32_t nc = q->cap ? q->cap * 2u : 16u, i;
        fle_tmsg *nq = (fle_tmsg *)malloc(nc * sizeof *nq);

        if (!nq) fle_stop("PostMessageW", "out of memory");
        for (i = 0; i < q->count; i++) nq[i] = q->q[(q->head + i) % q->cap];
        free(q->q);
        q->q = nq;
        q->cap = nc;
        q->head = 0;
    }
    m = &q->q[(q->head + q->count) % q->cap];
    m->hwnd = hwnd;
    m->msg = msg;
    m->wp = wp;
    m->lp = lp;
    m->time = (uint32_t)(now_ticks() / FLE_TICKS_PER_MS);
    m->ptx = m->pty = 0;
    if (fle_input_pointer) fle_input_pointer(&m->ptx, &m->pty);         /* decision 0136: the cursor position when the message was posted (Learn MSG.pt) */
    q->count++;
    q->changebits |= FLE_QS_POSTMESSAGE | FLE_QS_ALLPOSTMESSAGE;
    fle_thread_wake_check();
    return 1;
}

void fle_thread_send(unsigned to, fle_sent *s)
{
    fle_tqueue *q = fle_thread_queue(to, 1);

    s->done = 0;
    s->next = NULL;
    s->sender = cur;
    if (q->sent_tail) q->sent_tail->next = s;
    else q->sent_head = s;
    q->sent_tail = s;
    q->changebits |= FLE_QS_SENDMESSAGE;
    fle_thread_wake_check();
}

fle_sent *fle_thread_dequeue_sent(unsigned idx)
{
    fle_tqueue *q = fle_thread_queue(idx, 0);
    fle_sent *s;

    if (!q || !q->sent_head) return NULL;
    s = q->sent_head;
    q->sent_head = s->next;
    if (!q->sent_head) q->sent_tail = NULL;
    s->next = NULL;
    return s;
}

void fle_thread_wait_message(const char *api, int kind, uint64_t fhwnd, uint32_t fmin, uint32_t fmax)
{
    begin_wait(kind == 2 ? W_MSG_WAIT : W_MSG_GET, api, INFINITE_);
    T[cur].w.fhwnd = fhwnd;
    T[cur].w.fmin = fmin;
    T[cur].w.fmax = fmax;
    park();
}

void fle_thread_wait_sent(const char *api, fle_sent *s, unsigned to)
{
    (void)to;
    for (;;) {
        if (s->done) return;
        begin_wait(W_SENT, api, INFINITE_);
        T[cur].w.sent = s;
        park();
        if (s->done) return;
        if (fle_win_process_sent) fle_win_process_sent(api);
    }
}

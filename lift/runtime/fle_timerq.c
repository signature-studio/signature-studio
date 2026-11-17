/* Timer-queue timers (decision 0171). See fle_timerq.h. */
#include <stdint.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_callback.h"
#include "fle_thread.h"
#include "fle_timerq.h"

/* Learn CreateTimerQueueTimer, Flags: WT_EXECUTEDEFAULT 0x0, WT_EXECUTEINIOTHREAD 0x1, WT_EXECUTEINPERSISTENTTHREAD 0x80, WT_EXECUTELONGFUNCTION 0x10, WT_EXECUTEONLYONCE 0x8,
 * WT_EXECUTEINTIMERTHREAD 0x20, WT_TRANSFER_IMPERSONATION 0x100. Where the callback runs is not modelled: every callback runs on the timer's own cooperative thread. */
#define WT_EXECUTEONLYONCE 0x00000008u
#define WT_KNOWN (0x1u | 0x80u | 0x10u | 0x8u | 0x20u | 0x100u)
/* winerror.h: ERROR_IO_PENDING (Learn DeleteTimerQueueTimer: "If the error value is ERROR_IO_PENDING, it is not necessary to call this function again") */
#define FLE_ERROR_IO_PENDING 997u
#define INVALID_HANDLE_VALUE_ UINT64_C(0xFFFFFFFFFFFFFFFF)

typedef struct {
    int used;       /* the handle is valid (created, not deleted) */
    int running;    /* the callback is executing */
    uint64_t gen;   /* a thread whose generation differs ends at its next wake-up */
    uint64_t cb, param;
    uint32_t due, period, flags;
    int has_next;   /* a ChangeTimerQueueTimer from inside the callback: the next wait is next_ms */
    uint32_t next_ms;
} slot;

static slot S[FLE_TIMERQ_MAX];

static int slot_of(uint64_t h)
{
    uint64_t off;

    if (h <= FLE_TIMERQ_HANDLE_BASE || h >= FLE_TIMERQ_HANDLE_BASE + 4u * (FLE_TIMERQ_MAX + 1u)) return -1;
    off = h - FLE_TIMERQ_HANDLE_BASE;
    if (off % 4u) return -1;
    off = off / 4u - 1u;
    return S[off].used ? (int)off : -1;
}

static void body(uint64_t arg)
{
    unsigned i = (unsigned)(arg & 0xFFu);
    uint64_t gen = arg >> 8;
    slot *s = &S[i];
    uint32_t wait = s->due;

    for (;;) {
        fle_thread_sleep("TimerQueue", wait);
        if (s->gen != gen) break;
        s->running = 1;
        fle_call_guest(s->cb, s->param, 1, 0, 0); /* WAITORTIMERCALLBACK(lpParameter, TimerOrWaitFired = TRUE) */
        s->running = 0;
        if (s->gen != gen) break;
        if (s->has_next) {
            wait = s->next_ms;
            s->has_next = 0;
        } else if ((s->flags & WT_EXECUTEONLYONCE) || s->period == 0) {
            break;
        } else {
            wait = s->period;
        }
    }
}

static void start_thread(unsigned i)
{
    slot *s = &S[i];

    s->has_next = 0;
    if (fle_thread_spawn(body, (uint64_t)i | (s->gen << 8)) == 0) fle_stop("CreateTimerQueueTimer", "no thread could be made for the timer (the thread table is full)");
}

/* Learn CreateTimerQueueTimer: "Creates a timer-queue timer. This timer expires at the specified due time, then after every specified period. When the timer expires, the callback function is called."
 * TimerQueue NULL = the default timer queue. DueTime and Period are milliseconds; Period 0 = the timer is signaled once. The timer handle is written to *phNewTimer. A handle other than NULL for the queue, a
 * NULL phNewTimer or callback, and a flag the page does not list stop by name (the page names no failure code for them). */
void fle_imp_CreateTimerQueueTimer(void)
{
    static const char api[] = "CreateTimerQueueTimer";
    uint64_t ph = R[1], queue = R[2], cb = R[8], param = R[9];
    uint32_t due = (uint32_t)fle_arg(5), period = (uint32_t)fle_arg(6), flags = (uint32_t)fle_arg(7);
    unsigned i;

    if (queue != 0) fle_stop(api, "TimerQueue is not NULL: only the default timer queue exists (no CreateTimerQueue is imported)");
    if (ph == 0) fle_stop(api, "phNewTimer is NULL: the page gives no behaviour");
    if (cb == 0) fle_stop(api, "Callback is NULL: the page gives no behaviour");
    if (flags & ~WT_KNOWN) fle_stop(api, "Flags has a bit that the page does not list");
    for (i = 0; i < FLE_TIMERQ_MAX; i++) {
        if (!S[i].used) break;
    }
    if (i == FLE_TIMERQ_MAX) fle_stop(api, "more than 64 timer-queue timers are alive (a limit of this runtime)");
    S[i].used = 1;
    S[i].running = 0;
    S[i].gen++;
    S[i].cb = cb;
    S[i].param = param;
    S[i].due = due;
    S[i].period = period;
    S[i].flags = flags;
    fle_st(ph, 8, FLE_TIMERQ_HANDLE_BASE + 4u * (i + 1u));
    start_thread(i);
    R[0] = 1;
}

/* Learn ChangeTimerQueueTimer: "Updates a timer created by CreateTimerQueueTimer. If you call this function from the callback, the timer is rescheduled" (the next wait is DueTime). DueTime is relative to
 * now; Period as above. From outside the callback the old wait is cancelled and a new one starts. Returns nonzero. */
void fle_imp_ChangeTimerQueueTimer(void)
{
    static const char api[] = "ChangeTimerQueueTimer";
    uint64_t queue = R[1], h = R[2];
    uint32_t due = (uint32_t)R[8], period = (uint32_t)R[9];
    int k;

    if (queue != 0) fle_stop(api, "TimerQueue is not NULL: only the default timer queue exists");
    k = slot_of(h);
    if (k < 0) fle_stop(api, "Timer is not a handle that CreateTimerQueueTimer returned (or it was deleted): the page gives no failure code");
    S[k].period = period;
    S[k].due = due;
    if (S[k].running) {
        S[k].has_next = 1;
        S[k].next_ms = due;
    } else {
        S[k].gen++;
        start_thread((unsigned)k);
    }
    R[0] = 1;
}

/* Learn DeleteTimerQueueTimer: "Cancels a timer in a timer queue." CompletionEvent NULL: the function returns at once; INVALID_HANDLE_VALUE: it waits for a running callback; an event: signaled when the
 * callbacks are done. "If the function fails, the return value is zero. ... If the error value is ERROR_IO_PENDING, it is not necessary to call this function again." Here: a timer whose callback is not
 * running is cancelled (nonzero); a running callback (the timer deleted from its own callback is the usual case) gives 0 with ERROR_IO_PENDING and the timer ends when the callback returns. A wait for a
 * running callback and an event handle stop by name (the wait on another thread's callback is not modelled). */
void fle_imp_DeleteTimerQueueTimer(void)
{
    static const char api[] = "DeleteTimerQueueTimer";
    uint64_t queue = R[1], h = R[2], ev = R[8];
    int k;

    if (queue != 0) fle_stop(api, "TimerQueue is not NULL: only the default timer queue exists");
    k = slot_of(h);
    if (k < 0) fle_stop(api, "Timer is not a handle that CreateTimerQueueTimer returned (or it was already deleted): the page gives no failure code");
    if (S[k].running) {
        if (ev != 0) fle_stop(api, "CompletionEvent is not NULL while the callback is running: waiting for a callback is not modelled");
        S[k].used = 0;
        S[k].gen++;
        fle_set_last_error(FLE_ERROR_IO_PENDING);
        R[0] = 0;
        return;
    }
    if (ev != 0 && ev != INVALID_HANDLE_VALUE_) fle_stop(api, "CompletionEvent is an event handle: signaling it is not modelled");
    S[k].used = 0;
    S[k].gen++;
    R[0] = 1;
}

unsigned fle_timerq_live(void)
{
    unsigned i, n = 0;

    for (i = 0; i < FLE_TIMERQ_MAX; i++) if (S[i].used) n++;
    return n;
}

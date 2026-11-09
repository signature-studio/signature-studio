/* Event objects (docs/decisions/0050-event-sync.md; the pages read are listed in docs/investigations/event-sync-run.md).
 * An event is {manual-reset or auto-reset, signaled or not}; the table has one entry per CreateEventW
 * call and an entry is never reused. A case the Microsoft pages leave open, and every case this runtime does not
 * implement, stops the run through fle_stop (fle_env.h); where a page says a function fails without naming the error
 * code, the code set here is a runtime choice and is marked "assumption".
 * Decision 0122: with cooperative guest threads (fle_thread.c) a wait that is not satisfied blocks the calling thread and runs the others; the
 * stops of decision 0050 for "no other thread exists" remain for a program with one thread (fle_thread.c decides).
 * Nothing here reads the host: no clock, no thread, no process, no file. */
#define _GNU_SOURCE
#include <stdint.h>
#include <stdlib.h>

#include "fle_runtime.h"
#include "fle_host.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_vfs.h"
#include "fle_event.h"
#include "fle_thread.h"

static int64_t fle_event_owner_tid;

static void fle_event_check_thread(const char *api)
{
    int64_t tid = (int64_t)fle_host_tid();

    if (fle_event_owner_tid == 0) fle_event_owner_tid = tid;
    if (tid != fle_event_owner_tid) fle_stop(api, "called from a second thread, not supported");
}

static void fail(uint32_t err, uint64_t ret)
{
    fle_set_last_error(err);
    R[0] = ret;
}

typedef struct {
    uint8_t manual;     /* 1: manual-reset, 0: auto-reset */
    uint8_t signaled;
    uint8_t live;       /* 0 once the handle is closed (the object is destroyed with its last handle: one handle here) */
} ev_ent;

static ev_ent *evs;
static uint64_t evs_n, evs_cap, evs_live;

/* Decision 0174 (a measurement, G3 harness FLE_G3_LEAKSTATS=1): o[0] evs_n (entries ever used), o[1] evs_live, o[2] evs_cap. */
void fle_event_table_stats(uint64_t o[3]) { o[0] = evs_n; o[1] = evs_live; o[2] = evs_cap; }
static int hook_registered;

uint64_t fle_event_handle_count(void) { return evs_live; }

static int in_event_region(uint64_t h)
{
    return h > FLE_EVENT_HANDLE_BASE && h < FLE_EVENT_HANDLE_BASE + FLE_EVENT_REGION_SIZE;
}

/* The slot index of an issued handle value, or -1 when the value was never issued. */
static int64_t ev_index(uint64_t h)
{
    uint64_t off, i;

    if (!in_event_region(h)) return -1;
    off = h - FLE_EVENT_HANDLE_BASE;
    if (off % 4u) return -1;
    i = off / 4u - 1u;
    return i < evs_n ? (int64_t)i : -1;
}

int fle_event_state(uint64_t h)
{
    int64_t i = ev_index(h);

    if (i < 0 || !evs[i].live) return -1;
    return evs[i].signaled ? 1 : 0;
}

/* CloseHandle for the handles issued here (called by fle_imp_CloseHandle through fle_vfs_add_close_hook). Page (CloseHandle):
 * the handle is invalidated and the object is destroyed after its last handle (this runtime issues one handle per event;
 * DuplicateHandle of an event is not implemented). A handle that is already closed fails with 0 and ERROR_INVALID_HANDLE
 * (6; the page names no code - inference, as fle_vfs.c). A value that is in the region and was never issued stops. */
static int event_close_hook(uint64_t h)
{
    int64_t i;

    if (!in_event_region(h)) return 0;
    fle_event_check_thread("CloseHandle");
    i = ev_index(h);
    if (i < 0) fle_stop("CloseHandle", "handle value was not issued by this runtime");
    if (!evs[i].live) {
        fail(FLE_W32_ERR_INVALID_HANDLE, 0);
        return 1;
    }
    evs[i].live = 0;
    evs_live--;
    R[0] = 1;
    return 1;
}

static void hook_once(void)
{
    if (hook_registered) return;
    fle_vfs_add_close_hook(event_close_hook);
    hook_registered = 1;
}

/* The live event for `h`, or NULL after failing with `fail_ret` and ERROR_INVALID_HANDLE (6). The pages for SetEvent,
 * ResetEvent and WaitForSingleObject say that a failure sets the last error and do not name the code for an invalid
 * handle: 6 for NULL and for a closed handle is an assumption. A value this file did not issue (another kind of handle, a
 * pseudo handle, INVALID_HANDLE_VALUE, a number in the region that was never issued) is not defined by the pages and stops. */
static ev_ent *event_arg(const char *api, uint64_t h, uint64_t fail_ret)
{
    int64_t i;

    if (h == 0) {
        fail(FLE_W32_ERR_INVALID_HANDLE, fail_ret);
        return NULL;
    }
    i = ev_index(h);
    if (i < 0) fle_stop(api, "handle was not issued as an event by this runtime (other handle kinds, pseudo handles and INVALID_HANDLE_VALUE are not defined here)");
    if (!evs[i].live) {
        fail(FLE_W32_ERR_INVALID_HANDLE, fail_ret);
        return NULL;
    }
    return &evs[i];
}

/* kernel32!CreateEventW: HANDLE f(lpEventAttributes rcx, bManualReset edx, bInitialState r8d, lpName r9). Learn
 * (CreateEventW): bManualReset TRUE makes a manual-reset event, FALSE an auto-reset event; bInitialState TRUE starts it
 * signaled; the return value is the handle. A BOOL is a 32-bit value (the guest passes edx = 0xFFFFFFFF), so only the low
 * DWORD of the register counts (inference from the type; the page does not give the width).
 *  - implemented: unnamed (lpName NULL), default security (lpEventAttributes NULL).
 *  - not implemented, stop: a name (named events, ERROR_ALREADY_EXISTS and the shared namespace need an object table
 *    across kinds) and a security attributes structure.
 * The last error is left alone on success (the page says nothing about changing it; ERROR_ALREADY_EXISTS needs a name). */
void fle_imp_CreateEventW(void)
{
    static const char api[] = "CreateEventW";
    uint64_t attrs = R[1], name = R[9];
    uint32_t manual = (uint32_t)R[2], initial = (uint32_t)R[8];

    fle_event_check_thread(api);
    if (attrs != 0) fle_stop(api, "lpEventAttributes is not NULL (security descriptors and inheritance are not implemented)");
    if (name != 0) fle_stop(api, "a named event is not implemented");
    hook_once();
    if (evs_n == evs_cap) {
        uint64_t nc = evs_cap ? evs_cap * 2 : 16;
        ev_ent *n = (ev_ent *)realloc(evs, nc * sizeof *n);

        if (!n) fle_stop(api, "out of memory");
        evs = n;
        evs_cap = nc;
    }
    if (evs_n + 2 >= FLE_EVENT_REGION_SIZE / 4u) fle_stop(api, "event handle region exhausted");
    evs[evs_n].manual = manual != 0;
    evs[evs_n].signaled = initial != 0;
    evs[evs_n].live = 1;
    evs_n++;
    evs_live++;
    R[0] = FLE_EVENT_HANDLE_BASE + 4u * evs_n;
}

/* kernel32!SetEvent: BOOL f(hEvent rcx). Learn (SetEvent): the event becomes signaled; the return value is nonzero. A
 * manual-reset event stays signaled until ResetEvent; an auto-reset event stays signaled until a single waiting thread is
 * released, and "if no threads are waiting, the event object's state remains signaled"; "setting an event that is already
 * set has no effect". There is never a waiting thread here (the single thread cannot wait and signal), so both kinds just
 * become signaled. */
void fle_imp_SetEvent(void)
{
    ev_ent *e;

    fle_event_check_thread("SetEvent");
    e = event_arg("SetEvent", R[1], 0);
    if (!e) return;
    e->signaled = 1;
    R[0] = 1;
    if (fle_thread_wake_check) fle_thread_wake_check(); /* decision 0122: a thread that waits for this event becomes ready (an auto-reset event goes to the first waiter) */
}

/* kernel32!ResetEvent: BOOL f(hEvent rcx). Learn (ResetEvent): the event becomes nonsignaled for both kinds; "resetting
 * an event that is already reset has no effect"; the return value is nonzero. */
void fle_imp_ResetEvent(void)
{
    ev_ent *e;

    fle_event_check_thread("ResetEvent");
    e = event_arg("ResetEvent", R[1], 0);
    if (!e) return;
    e->signaled = 0;
    R[0] = 1;
}

/* kernel32!WaitForSingleObject: DWORD f(hHandle rcx, dwMilliseconds edx). Learn (WaitForSingleObject): a signaled event
 * returns WAIT_OBJECT_0 at once, whatever the time-out; "the function modifies the state of some types of synchronization
 * objects" (an auto-reset event becomes nonsignaled, a manual-reset event keeps its state: CreateEventW page); with
 * dwMilliseconds 0 an object that is not signaled returns WAIT_TIMEOUT at once ("it always returns immediately"); an
 * invalid handle is WAIT_FAILED with a last error (6: assumption, see event_arg). Only the low DWORD of rdx is the time-out.
 * Stops: INFINITE on an event that is not signaled would never return (no other thread can signal it; Sleep(INFINITE)
 * stops for the same reason, decision 0017); a finite, nonzero time-out on an event that is not signaled would end in
 * WAIT_TIMEOUT only after the interval has elapsed, and the page's WAIT_TIMEOUT says exactly that, but a wait does not
 * move the virtual clock (decision 0017: only Sleep and the clock reads do; decision 0050: no clock window yet), so
 * answering would report an interval that passed without any time passing. */
void fle_imp_WaitForSingleObject(void)
{
    static const char api[] = "WaitForSingleObject";
    uint32_t ms = (uint32_t)R[2];
    ev_ent *e;

    fle_event_check_thread(api);
    if (fle_thread_owns_handle && fle_thread_owns_handle(R[1])) {
        /* decision 0122: the handle of a thread (WaitForSingleObject(thread, ...) of TThread.WaitFor) */
        uint64_t h = R[1];

        R[0] = fle_thread_wait_objects(api, 1, &h, 0, ms, 0, 0);
        return;
    }
    e = event_arg(api, R[1], FLE_WAIT_FAILED);
    if (!e) return;
    if (e->signaled) {
        if (!e->manual) e->signaled = 0;
        R[0] = FLE_WAIT_OBJECT_0;
        return;
    }
    if (ms == 0) {
        R[0] = FLE_WAIT_TIMEOUT;
        return;
    }
    if (fle_thread_wait_objects && fle_thread_count && fle_thread_count() > 1) {
        /* decision 0122: the calling thread waits and the other threads run; the virtual clock moves to the time-out when every thread waits */
        uint64_t h = R[1];

        R[0] = fle_thread_wait_objects(api, 1, &h, 0, ms, 0, 0);
        return;
    }
    if (ms == FLE_WAIT_INFINITE) fle_stop(api, "INFINITE time-out on an event that is not signaled would never return (no other thread exists)");
    fle_stop(api, "a finite, nonzero time-out on an event that is not signaled needs the virtual clock to advance (decision 0050: not implemented)");
}

/* Decision 0122: the state of an event for the wait of a thread. -1: `h` is not a live event handle (NULL, closed, another kind); 0: not signaled; 1: signaled.
 * With consume != 0 a signaled auto-reset event becomes nonsignaled (the wait that it satisfied; Learn WaitForSingleObject: the function modifies the state). */
/* Decision 0139: SetEvent from the host side (the audio device signals a notification event: runtime/fle_dsound.c). No register is read or written and fle_thread_wake_check is not called: it is called from the scheduler's own
 * poll of the waits (fle_thread.c poll_waiters), and a thread that waits for the event is tested at the next switch like at any other. 1 when `h` is a live event handle, else 0. */
int fle_event_host_set(uint64_t h)
{
    int64_t i = ev_index(h);

    if (i < 0 || !evs[i].live) return 0;
    evs[i].signaled = 1;
    return 1;
}

int fle_event_wait_state(uint64_t h, int consume)
{
    int64_t i = ev_index(h);

    if (i < 0 || !evs[i].live) return -1;
    if (!evs[i].signaled) return 0;
    if (consume && !evs[i].manual) evs[i].signaled = 0;
    return 1;
}

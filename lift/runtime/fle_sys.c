#define _GNU_SOURCE
#include <stdint.h>
#include <string.h>
#include <sys/mman.h>
#include <unistd.h>

#include "fle_runtime.h"
#include "fle_host.h"
#include "fle_imports.h"
#include "fle_sys.h"
#include "fle_thread.h"

#ifndef MAP_FIXED_NOREPLACE
#define MAP_FIXED_NOREPLACE 0x100000
#endif

/* Everything here follows docs/decisions/0017-sync-time-memory.md. The pages read (learn.microsoft.com,
 * 2026-10-20) are listed in docs/investigations/import-sys-spec.md. A case that the pages leave
 * open stops the run; where the pages say "the function fails" without naming the error code, the
 * code returned is a runtime choice and is marked "assumption" below. */

#define STOP(...) do { fprintf(stderr, "fle: " __VA_ARGS__); abort(); } while (0)

#define FLE_ERROR_NOT_ENOUGH_MEMORY 8u
#define FLE_ERROR_INVALID_PARAMETER 87u
#define FLE_ERROR_INVALID_ADDRESS   487u

#define FLE_INFINITE 0xFFFFFFFFu

/* ---- Single guest thread ----
 * The state below is not locked. The first call from these stubs records the caller's tid and a
 * call from another thread stops the run (same rule as the TLS stubs in fle_imports.c). A critical
 * section can only be held by another thread if there is another thread, so this rule also covers
 * "the section is owned by someone else and Enter would wait". */
static long fle_sys_owner_tid;

static void fle_sys_check_thread(const char *api)
{
    long tid = fle_host_tid();
    if (fle_sys_owner_tid == 0) fle_sys_owner_tid = tid;
    if (tid != fle_sys_owner_tid)
        STOP("%s called from tid %ld (owner %ld): would need to wait for another thread, not supported\n",
             api, tid, fle_sys_owner_tid);
}

/* ---- Virtual clock ---- */
static uint64_t fle_clock_elapsed;   /* 100 ns ticks added since the start value */

static uint64_t fle_clock_now(void)
{
    return FLE_CLOCK_START_MS * FLE_TICKS_PER_MS + fle_clock_elapsed;
}

uint64_t fle_sys_clock_ticks(void) { return fle_clock_now(); }

/* A read returns the current value and then moves the clock by FLE_CLOCK_READ_TICKS. */
static uint64_t fle_clock_read(void)
{
    uint64_t t = fle_clock_now();
    fle_clock_elapsed += FLE_CLOCK_READ_TICKS;
    return t;
}

/* Decision 0122: moves the virtual clock forward to `ticks` (never backward). The scheduler of fle_thread.c uses it when every thread waits, to reach the smallest deadline. */
void fle_sys_clock_advance_to(uint64_t ticks)
{
    uint64_t now = fle_clock_now();

    if (ticks > now) fle_clock_elapsed += ticks - now;
}

/* kernel32!Sleep: VOID Sleep(DWORD dwMilliseconds). rcx = ms (ecx). MSDN (Sleep): suspends the
 * current thread until the interval elapses; 0 = give up the time slice and return at once when
 * no other thread is ready; INFINITE = never time out; no return value. The clock advances by the
 * requested interval and the call returns without waiting. INFINITE would never return (there is
 * no other thread to wake it), so it stops the run. The page allows a sleep to be shorter or
 * longer than asked; the exact interval is the choice here. */
void fle_imp_Sleep(void)
{
    uint32_t ms = (uint32_t)R[1];

    fle_sys_check_thread("Sleep");
    if (fle_thread_sleep) {
        /* decision 0122: with more than one guest thread Sleep(ms > 0) waits and the others run, Sleep(0) yields; with one thread this is what it was */
        if (fle_thread_count && fle_thread_count() > 1) {
            fle_thread_sleep("Sleep", ms);
            return;
        }
    }
    if (ms == FLE_INFINITE) STOP("Sleep(INFINITE) would never return\n");
    fle_clock_elapsed += (uint64_t)ms * FLE_TICKS_PER_MS;
}

/* kernel32!GetTickCount: DWORD, ms since the (fictitious) system start, wraps at 2^32 (MSDN:
 * GetTickCount). winmm!timeGetTime: the same (MSDN: timeGetTime says "time elapsed since Windows was
 * started" and wraps every 2^32 ms; that both APIs read one clock is an assumption). */
static void fle_tick_ms(const char *api)
{
    fle_sys_check_thread(api);
    R[0] = (uint32_t)(fle_clock_read() / FLE_TICKS_PER_MS);
}

void fle_imp_GetTickCount(void) { fle_tick_ms("GetTickCount"); }
void fle_imp_timeGetTime(void) { fle_tick_ms("timeGetTime"); }

/* kernel32!QueryPerformanceCounter(LARGE_INTEGER *lpPerformanceCount): BOOL. rcx = output. MSDN:
 * with a valid pointer it always succeeds (nonzero) on Windows XP or later; a NULL or bad pointer is
 * not defined and stops the run. The counter is the tick count (FLE_QPF_HZ = 10 MHz, which MSDN
 * "Acquiring high-resolution time stamps" names as a frequency that occurs; the frequency is fixed
 * for the process, as the page requires). */
void fle_imp_QueryPerformanceCounter(void)
{
    uint64_t out = R[1];

    fle_sys_check_thread("QueryPerformanceCounter");
    if (out == 0) STOP("QueryPerformanceCounter(NULL) is not defined\n");
    fle_st(out, 8, fle_clock_read());
    R[0] = 1;
}

void fle_imp_QueryPerformanceFrequency(void)
{
    uint64_t out = R[1];

    fle_sys_check_thread("QueryPerformanceFrequency");
    if (out == 0) STOP("QueryPerformanceFrequency(NULL) is not defined\n");
    fle_st(out, 8, FLE_QPF_HZ);
    R[0] = 1;
}

/* ---- Critical sections ----
 * MSDN (InitializeCriticalSection, EnterCriticalSection, LeaveCriticalSection, DeleteCriticalSection):
 * the object is opaque to the caller and is not to be modified, so the state is kept outside it,
 * in a table keyed by the object's address (the page says the object cannot be moved or copied).
 * Initialize on a live object is undefined; Enter, Leave and Delete on an object that was never
 * initialized or was deleted are undefined; Leave without ownership is an error that "may cause
 * another thread to wait indefinitely"; Delete of an owned section leaves the caller responsible.
 * All of these stop the run. Enter on an owned section by the same thread counts one more entry
 * and Leave must be called once per entry. */
#define FLE_CS_SLOTS     (1u << 16)
#define FLE_CS_MAX_LIVE  (FLE_CS_SLOTS / 2)
#define FLE_CS_EMPTY     0u
#define FLE_CS_LIVE      1u
#define FLE_CS_DELETED   2u

typedef struct {
    uint64_t addr;
    uint32_t recursion;   /* entries not yet left */
    uint32_t state;
    uint32_t owner;       /* decision 0122: the index of the guest thread that holds the section (meaningful while recursion > 0) */
} fle_cs_slot;

static fle_cs_slot fle_cs_table[FLE_CS_SLOTS];
static uint32_t fle_cs_live;

static uint32_t fle_cs_hash(uint64_t a)
{
    return (uint32_t)(((a >> 3) * UINT64_C(0x9E3779B97F4A7C15)) >> 48) & (FLE_CS_SLOTS - 1);
}

static fle_cs_slot *fle_cs_find(uint64_t addr)
{
    uint32_t i = fle_cs_hash(addr), n;

    for (n = 0; n < FLE_CS_SLOTS; n++, i = (i + 1) & (FLE_CS_SLOTS - 1)) {
        if (fle_cs_table[i].state == FLE_CS_EMPTY) return NULL;
        if (fle_cs_table[i].state == FLE_CS_LIVE && fle_cs_table[i].addr == addr) return &fle_cs_table[i];
    }
    return NULL;
}

static uint64_t fle_cs_arg(const char *api)
{
    fle_sys_check_thread(api);
    if (R[1] == 0) STOP("%s(NULL) is not defined\n", api);
    return R[1];
}

static void fle_cs_init(const char *api)
{
    uint64_t a = fle_cs_arg(api);
    uint32_t i, n;

    if (fle_cs_find(a)) STOP("%s on an initialized section %#llx is undefined\n", api, (unsigned long long)a);
    if (fle_cs_live >= FLE_CS_MAX_LIVE) STOP("more than %u live critical sections\n", FLE_CS_MAX_LIVE);
    i = fle_cs_hash(a);
    for (n = 0; n < FLE_CS_SLOTS; n++, i = (i + 1) & (FLE_CS_SLOTS - 1)) {
        if (fle_cs_table[i].state != FLE_CS_LIVE) {
            fle_cs_table[i].addr = a;
            fle_cs_table[i].recursion = 0;
            fle_cs_table[i].state = FLE_CS_LIVE;
            fle_cs_live++;
            return;
        }
    }
    STOP("critical section table full\n");
}

void fle_imp_InitializeCriticalSection(void)
{
    fle_cs_init("InitializeCriticalSection");
}

/* kernel32!InitializeCriticalSectionAndSpinCount: BOOL f(LPCRITICAL_SECTION rcx, DWORD dwSpinCount edx) (decision 0078).
 * MSDN (InitializeCriticalSectionAndSpinCount): "On single-processor systems, the spin count is ignored and the critical
 * section spin count is set to 0"; GetSystemInfo (below) reports one processor, so the count is not kept. "This function
 * always succeeds and returns a nonzero value" (Windows Vista and later; the emulated OS is Windows 10, decision 0014).
 * "Initializing a critical section that is already initialized results in undefined behavior": same stop as
 * InitializeCriticalSection. The object is the same table entry, so Enter, Leave and Delete work on it unchanged. */
void fle_imp_InitializeCriticalSectionAndSpinCount(void)
{
    fle_cs_init("InitializeCriticalSectionAndSpinCount");
    R[0] = 1;
}

/* ---- Interlocked singly linked list heads (decision 0078) ----
 * MSDN (InitializeSListHead, InterlockedFlushSList): the SLIST_HEADER "is for system use only", so its bytes are never
 * written or read here; like the critical sections, the state is kept outside the object, in a table keyed by its address.
 * Only the two calls that the DLL imports are answered. Nothing here can put an entry on a list (InterlockedPushEntrySList
 * is not answered), so a head that InitializeSListHead was called for is always empty: InterlockedFlushSList returns NULL,
 * which the page names for an empty list. A flush of a head that was never initialized is not defined by the page and stops.
 * Calling InitializeSListHead again on a registered head is accepted (the list is empty either way; the page is silent). */
#define FLE_SLIST_MAX 256u

/* Decision 0126 (PROVISIONAL), wall 82: InterlockedPushEntrySList and InterlockedPopEntrySList are answered, and InterlockedFlushSList returns what the list holds (decision 0017 etc. answered the empty list only). The state is
 * kept as before OUTSIDE the SLIST_HEADER (its bytes "are for system use only"): per head the first entry (a guest address) and the depth. An ENTRY is the guest's: SLIST_ENTRY is { SLIST_ENTRY *Next } at offset 0
 * (learn/SLIST_ENTRY.txt: a structure whose members the page lists as "Next"; the Push page: "Inserts an item at the front of a singly linked list"; the item "must be aligned on a MEMORY_ALLOCATION_ALIGNMENT
 * boundary"): Push writes the previous first entry into the new entry's Next (offset 0; the one write this runtime makes to an entry), Pop reads the first entry's Next (offset 0) to make it the new first, Flush
 * returns the first entry and leaves the chain (the caller walks Next) and the list empty. Push returns "the previous first item in the list. If the list was previously empty, the return value is NULL"; Pop "a pointer
 * to the item removed from the list. If the list is empty, the return value is NULL"; Flush "a pointer to the items removed from the list. If the list is empty, the return value is NULL". A head that
 * InitializeSListHead was not called for stops, as before (the pages say the list "must be previously initialized"). An entry that is NULL or not 16-byte aligned stops (MEMORY_ALLOCATION_ALIGNMENT is 16 on x64: the
 * page names the constant and not the number; the SDK's value, UNOFFICIAL: "will behave unpredictably"). QueryDepthSList is not imported by a module of the chain and has no row. */
static uint64_t fle_slist_heads[FLE_SLIST_MAX];
static uint64_t fle_slist_first[FLE_SLIST_MAX];
static uint32_t fle_slist_depth[FLE_SLIST_MAX];
static uint32_t fle_slist_n;

static int fle_slist_find(uint64_t a)
{
    uint32_t i;

    for (i = 0; i < fle_slist_n; i++)
        if (fle_slist_heads[i] == a) return (int)i;
    return -1;
}

void fle_imp_InitializeSListHead(void)
{
    uint64_t a = R[1];
    int k;

    fle_sys_check_thread("InitializeSListHead");
    if (a == 0) STOP("InitializeSListHead(NULL) is not defined\n");
    k = fle_slist_find(a);
    if (k >= 0) {
        fle_slist_first[k] = 0;                 /* re-initialising a head empties it ("initializes the head of a singly linked list") */
        fle_slist_depth[k] = 0;
        return;
    }
    if (fle_slist_n >= FLE_SLIST_MAX) STOP("more than %u list heads\n", FLE_SLIST_MAX);
    fle_slist_heads[fle_slist_n] = a;
    fle_slist_first[fle_slist_n] = 0;
    fle_slist_depth[fle_slist_n] = 0;
    fle_slist_n++;
}

void fle_imp_InterlockedFlushSList(void)
{
    uint64_t a = R[1];
    int k;

    fle_sys_check_thread("InterlockedFlushSList");
    k = fle_slist_find(a);
    if (k < 0) STOP("InterlockedFlushSList on %#llx, which InitializeSListHead was not called for, is not defined\n", (unsigned long long)a);
    R[0] = fle_slist_first[k];
    fle_slist_first[k] = 0;
    fle_slist_depth[k] = 0;
}

void fle_imp_InterlockedPushEntrySList(void)
{
    uint64_t a = R[1], e = R[2];
    int k;

    fle_sys_check_thread("InterlockedPushEntrySList");
    k = fle_slist_find(a);
    if (k < 0) STOP("InterlockedPushEntrySList on %#llx, which InitializeSListHead was not called for, is not defined\n", (unsigned long long)a);
    if (e == 0 || (e & 15u)) STOP("InterlockedPushEntrySList of an entry %#llx that is NULL or not aligned on 16 bytes: the page says it behaves unpredictably\n", (unsigned long long)e);
    fle_st(e, 8, fle_slist_first[k]);
    R[0] = fle_slist_first[k];
    fle_slist_first[k] = e;
    fle_slist_depth[k]++;
}

void fle_imp_InterlockedPopEntrySList(void)
{
    uint64_t a = R[1], e;
    int k;

    fle_sys_check_thread("InterlockedPopEntrySList");
    k = fle_slist_find(a);
    if (k < 0) STOP("InterlockedPopEntrySList on %#llx, which InitializeSListHead was not called for, is not defined\n", (unsigned long long)a);
    e = fle_slist_first[k];
    if (e != 0) {
        fle_slist_first[k] = fle_ld(e, 8);
        fle_slist_depth[k]--;
    }
    R[0] = e;
}

void fle_imp_EnterCriticalSection(void)
{
    uint64_t a = fle_cs_arg("EnterCriticalSection");
    fle_cs_slot *s = fle_cs_find(a);

    if (!s) STOP("EnterCriticalSection on a section %#llx that is not initialized (or was deleted)\n", (unsigned long long)a);
    if (s->recursion == UINT32_MAX) STOP("EnterCriticalSection recursion overflow\n");
    {
        unsigned me = fle_thread_current ? fle_thread_current() : 0;

        if (s->recursion > 0 && s->owner != me) {
            /* decision 0122: owned by another guest thread: the caller waits (Learn EnterCriticalSection: "If the critical section object is currently owned by another thread, EnterCriticalSection waits indefinitely for ownership") */
            if (!fle_thread_wait_cs) STOP("EnterCriticalSection on a section %#llx that another thread owns, and no scheduler is linked\n", (unsigned long long)a);
            fle_thread_wait_cs("EnterCriticalSection", a);
            return; /* the section was taken for this thread by fle_sys_cs_try_take */
        }
        s->owner = me;
    }
    s->recursion++;
}

/* Decision 0122: called by the scheduler for a thread that waits for the section at `addr`: when the section is free it is taken for `thread` and 1 is returned. */
int fle_sys_cs_try_take(uint64_t addr, unsigned thread)
{
    fle_cs_slot *s = fle_cs_find(addr);

    if (!s || s->recursion != 0) return 0;
    s->owner = thread;
    s->recursion = 1;
    return 1;
}

void fle_imp_LeaveCriticalSection(void)
{
    uint64_t a = fle_cs_arg("LeaveCriticalSection");
    fle_cs_slot *s = fle_cs_find(a);

    if (!s) STOP("LeaveCriticalSection on a section %#llx that is not initialized (or was deleted)\n", (unsigned long long)a);
    if (s->recursion == 0) STOP("LeaveCriticalSection on a section %#llx the thread does not own\n", (unsigned long long)a);
    if (s->owner != (fle_thread_current ? fle_thread_current() : 0u))
        STOP("LeaveCriticalSection on a section %#llx that another thread owns (Learn: this may cause another thread to wait indefinitely)\n", (unsigned long long)a);
    s->recursion--;
    if (s->recursion == 0 && fle_thread_wake_check) fle_thread_wake_check(); /* decision 0122: a thread that waits for this section may take it now */
}

void fle_imp_DeleteCriticalSection(void)
{
    uint64_t a = fle_cs_arg("DeleteCriticalSection");
    fle_cs_slot *s = fle_cs_find(a);

    if (!s) STOP("DeleteCriticalSection on a section %#llx that is not initialized (or was deleted)\n", (unsigned long long)a);
    if (s->recursion != 0) STOP("DeleteCriticalSection on a section %#llx that is still owned\n", (unsigned long long)a);
    s->state = FLE_CS_DELETED;
    fle_cs_live--;
}

/* ---- Slim reader/writer locks and condition variables (decision 0092, provisional) ----
 * MSDN (Slim Reader/Writer (SRW) Locks, InitializeSRWLock): the lock "must not be modified" by the process and is "logically
 * opaque", so the state is kept outside it, in a table keyed by the lock's address, and the guest's 8 bytes are never read or
 * written. The same holds for a CONDITION_VARIABLE. A table entry exists only while the lock is held; an address with no entry is
 * an unlocked lock in its initial state ("An unlocked SRW lock with no waiting threads is in its initial state"). The page lets a
 * lock be initialized statically with SRWLOCK_INIT but gives no value for it, so treating the first use of an unknown address as
 * an unlocked lock is an assumption (the DLL does not import InitializeSRWLock). The state of a held lock is exclusive
 * (shared == 0) or shared with `shared` acquisitions.
 * There is one thread, so a held lock is never released by anyone but the caller. Acquire stops the run where the page says the
 * call deadlocks (exclusive twice by one thread) or where nothing could ever release the lock (exclusive over shared, shared over
 * exclusive: "a thread that owns an SRW lock in shared mode cannot upgrade its ownership"). Shared over shared adds one holder (the
 * page says a recursive shared acquisition "should not" be done, and only that it can deadlock "when combined with exclusive
 * acquisition"; allowing it is an assumption). Release must match the mode of the acquisition; a Release of a lock that is not
 * held, or held in the other mode, stops the run (the page says only that the lock "must be released by the same thread that
 * acquired it"; the result is not given, and stopping follows LeaveCriticalSection above, an assumption). */
#define FLE_SRW_SLOTS     (1u << 16)
#define FLE_SRW_MAX_LIVE  (FLE_SRW_SLOTS / 2)

typedef struct {
    uint64_t addr;
    uint32_t shared;   /* 0: held exclusively; n > 0: held in shared mode by n acquisitions. The entry is FLE_CS_LIVE only while held */
    uint32_t state;
} fle_srw_slot;

static fle_srw_slot fle_srw_table[FLE_SRW_SLOTS];
static uint32_t fle_srw_live;

static fle_srw_slot *fle_srw_find(uint64_t addr)
{
    uint32_t i = fle_cs_hash(addr), n;

    for (n = 0; n < FLE_SRW_SLOTS; n++, i = (i + 1) & (FLE_SRW_SLOTS - 1)) {
        if (fle_srw_table[i].state == FLE_CS_EMPTY) return NULL;
        if (fle_srw_table[i].state == FLE_CS_LIVE && fle_srw_table[i].addr == addr) return &fle_srw_table[i];
    }
    return NULL;
}

static uint64_t fle_srw_arg(const char *api)
{
    fle_sys_check_thread(api);
    if (R[1] == 0) STOP("%s(NULL) is not defined\n", api);
    return R[1];
}

/* Registers `a` (not held) as held: exclusively (shared 0) or with one shared acquisition (shared 1). */
static void fle_srw_take(uint64_t a, uint32_t shared)
{
    uint32_t i, n;

    if (fle_srw_live >= FLE_SRW_MAX_LIVE) STOP("more than %u SRW locks held at once\n", FLE_SRW_MAX_LIVE);
    i = fle_cs_hash(a);
    for (n = 0; n < FLE_SRW_SLOTS; n++, i = (i + 1) & (FLE_SRW_SLOTS - 1)) {
        if (fle_srw_table[i].state != FLE_CS_LIVE) {
            fle_srw_table[i].addr = a;
            fle_srw_table[i].shared = shared;
            fle_srw_table[i].state = FLE_CS_LIVE;
            fle_srw_live++;
            return;
        }
    }
    STOP("SRW lock table full\n");
}

/* The lock is no longer held: the entry is free again (a lookup skips FLE_CS_DELETED and a new entry reuses it). A probe that
 * reaches this entry goes on to the next one, so when that one is empty this entry can be empty too, and so can the deleted
 * entries just before it; this keeps the probe runs short for locks that are taken once and never used again. */
static void fle_srw_release_entry(fle_srw_slot *s)
{
    uint32_t i = (uint32_t)(s - fle_srw_table);

    fle_srw_live--;
    if (fle_srw_table[(i + 1) & (FLE_SRW_SLOTS - 1)].state != FLE_CS_EMPTY) {
        s->state = FLE_CS_DELETED;
        return;
    }
    s->state = FLE_CS_EMPTY;
    for (i = (i - 1) & (FLE_SRW_SLOTS - 1); fle_srw_table[i].state == FLE_CS_DELETED; i = (i - 1) & (FLE_SRW_SLOTS - 1))
        fle_srw_table[i].state = FLE_CS_EMPTY;
}

/* kernel32!AcquireSRWLockExclusive: VOID f(PSRWLOCK rcx). */
void fle_imp_AcquireSRWLockExclusive(void)
{
    uint64_t a = fle_srw_arg("AcquireSRWLockExclusive");
    fle_srw_slot *s = fle_srw_find(a);

    if (!s) {
        fle_srw_take(a, 0);
        return;
    }
    if (s->shared == 0)
        STOP("AcquireSRWLockExclusive on a lock %#llx that is already held exclusively: the page says exclusive mode cannot be acquired recursively (deadlock), and no other thread can release it\n", (unsigned long long)a);
    STOP("AcquireSRWLockExclusive on a lock %#llx held in shared mode (%u acquisitions): a shared owner cannot upgrade to exclusive, and no other thread can release it\n", (unsigned long long)a, s->shared);
}

/* kernel32!AcquireSRWLockShared: VOID f(PSRWLOCK rcx). */
void fle_imp_AcquireSRWLockShared(void)
{
    uint64_t a = fle_srw_arg("AcquireSRWLockShared");
    fle_srw_slot *s = fle_srw_find(a);

    if (!s) {
        fle_srw_take(a, 1);
        return;
    }
    if (s->shared == 0)
        STOP("AcquireSRWLockShared on a lock %#llx held exclusively: no other thread can access it until the writer releases it, and no other thread can release it\n", (unsigned long long)a);
    if (s->shared == UINT32_MAX) STOP("AcquireSRWLockShared shared count overflow on a lock %#llx\n", (unsigned long long)a);
    s->shared++;
}

/* kernel32!ReleaseSRWLockExclusive: VOID f(PSRWLOCK rcx). */
void fle_imp_ReleaseSRWLockExclusive(void)
{
    uint64_t a = fle_srw_arg("ReleaseSRWLockExclusive");
    fle_srw_slot *s = fle_srw_find(a);

    if (!s) STOP("ReleaseSRWLockExclusive on a lock %#llx that is not held (never acquired, or already released): the page does not give the result\n", (unsigned long long)a);
    if (s->shared != 0)
        STOP("ReleaseSRWLockExclusive on a lock %#llx held in shared mode (%u acquisitions): the mode does not match, the page does not give the result\n", (unsigned long long)a, s->shared);
    fle_srw_release_entry(s);
}

/* kernel32!ReleaseSRWLockShared: VOID f(PSRWLOCK rcx). */
void fle_imp_ReleaseSRWLockShared(void)
{
    uint64_t a = fle_srw_arg("ReleaseSRWLockShared");
    fle_srw_slot *s = fle_srw_find(a);

    if (!s) STOP("ReleaseSRWLockShared on a lock %#llx that is not held (never acquired, or already released): the page does not give the result\n", (unsigned long long)a);
    if (s->shared == 0)
        STOP("ReleaseSRWLockShared on a lock %#llx held exclusively: the mode does not match, the page does not give the result\n", (unsigned long long)a);
    if (--s->shared == 0) fle_srw_release_entry(s);
}

/* kernel32!WakeConditionVariable, WakeAllConditionVariable: VOID f(PCONDITION_VARIABLE rcx). MSDN: they wake one (all) of the
 * threads waiting on the variable. There is one thread and it is the caller, so nothing waits and nothing happens: R[0] is left
 * alone and no signal is kept for a later Sleep. The page does not say what a Wake with no waiter does (it compares one wake to
 * "setting an auto-reset event", which would keep the signal, but names no such case), so doing nothing is an assumption; the
 * event of decision 0050 keeps its signal because the SetEvent page says so, and the Wake pages have no such sentence. The
 * variable's bytes are not read or written. */
static void fle_cv_wake(const char *api)
{
    fle_sys_check_thread(api);
    if (R[1] == 0) STOP("%s(NULL) is not defined\n", api);
}

void fle_imp_WakeConditionVariable(void) { fle_cv_wake("WakeConditionVariable"); }
void fle_imp_WakeAllConditionVariable(void) { fle_cv_wake("WakeAllConditionVariable"); }

/* kernel32!SleepConditionVariableSRW: BOOL f(PCONDITION_VARIABLE rcx, PSRWLOCK rdx, DWORD dwMilliseconds r8d, ULONG Flags r9d).
 * Every call stops the run (decision 0092). MSDN: "If the lock is unlocked when this function is called, the function behavior is
 * undefined" (that case is named in the message). INFINITE: "the function's time-out interval never elapses", and the thread "can
 * be woken using the WakeConditionVariable or WakeAllConditionVariable function", which no other thread can call. A finite value
 * and 0 would return FALSE with ERROR_TIMEOUT: that needs the virtual clock to move (decision 0050 C adds no such entry point),
 * the value of ERROR_TIMEOUT (not on the page) and a decision on whether the lock is held again on a time-out (the page says it is
 * re-acquired "after the thread is woken", not after a time-out). The Flags value is printed as it came, not interpreted. */
void fle_imp_SleepConditionVariableSRW(void)
{
    uint64_t cv = R[1], lock = R[2];
    uint32_t ms = (uint32_t)R[8], flags = (uint32_t)R[9];
    fle_srw_slot *s;
    const char *why;

    fle_sys_check_thread("SleepConditionVariableSRW");
    s = lock ? fle_srw_find(lock) : NULL;
    if (!s)
        STOP("SleepConditionVariableSRW(ConditionVariable %#llx, SRWLock %#llx, dwMilliseconds %u, Flags %u): the lock is not held (never acquired, or released); the page says the behavior is undefined\n",
             (unsigned long long)cv, (unsigned long long)lock, ms, flags);
    if (ms == FLE_INFINITE)
        why = "INFINITE never times out and no other thread can call WakeConditionVariable or WakeAllConditionVariable, so it would never return";
    else if (ms == 0)
        why = "dwMilliseconds 0 returns at once with FALSE and ERROR_TIMEOUT; the ERROR_TIMEOUT value is not on the page and whether the lock is held again on a time-out is not given";
    else
        why = "a finite time-out needs the virtual clock to move (decision 0050 C adds no such entry point), the ERROR_TIMEOUT value (not on the page) and whether the lock is held again on a time-out (not given)";
    STOP("SleepConditionVariableSRW(ConditionVariable %#llx, SRWLock %#llx, dwMilliseconds %u, Flags %u): the lock is held %s; %s\n",
         (unsigned long long)cv, (unsigned long long)lock, ms, flags,
         s->shared == 0 ? "exclusively" : "in shared mode", why);
}

/* ---- GetSystemInfo ----
 * SYSTEM_INFO (MSDN: SYSTEM_INFO) on x64 is 48 bytes: wProcessorArchitecture at 0, wReserved 2,
 * dwPageSize 4, lpMinimumApplicationAddress 8, lpMaximumApplicationAddress 16, dwActiveProcessorMask
 * 24, dwNumberOfProcessors 32, dwProcessorType 36, dwAllocationGranularity 40, wProcessorLevel 44,
 * wProcessorRevision 46 (the offsets follow from the member types and natural alignment).
 * Values: architecture AMD64 (9, MSDN table); page size FLE_PAGE_SIZE, the page VirtualAlloc uses
 * (MSDN: dwPageSize); one processor with mask 1, because the run is a single guest thread (a choice;
 * a host core count would make the output differ between machines); processor type 8664
 * (PROCESSOR_AMD_X8664, listed in MSDN, obsolete member); level and revision 0 as the CPUID answer
 * claims no model (decision 0009; MSDN: level is for display only). The MSDN pages read do not
 * give numbers for the allocation granularity or the application address range. Granularity
 * 65536 is a choice: the guest's own code rounds with 0xffff0000 at 0x407783, which is consistent
 * with it (not a confirmation). The range 0x10000 to 0x7FFFFFFEFFFF is the commonly cited Windows
 * x64 user range, from memory and not confirmed here. wProcessorRevision is 0. */
#define FLE_SI_SIZE      48u
#define FLE_ARCH_AMD64   9u
#define FLE_PROC_AMD_X8664 8664u

void fle_imp_GetSystemInfo(void)
{
    uint64_t out = R[1];
    uint8_t si[FLE_SI_SIZE];
    uint16_t arch = FLE_ARCH_AMD64;
    uint32_t page = (uint32_t)FLE_PAGE_SIZE, nproc = 1, ptype = FLE_PROC_AMD_X8664;
    uint32_t gran = (uint32_t)FLE_ALLOC_GRANULARITY;
    uint64_t lo = FLE_MIN_APP_ADDR, hi = FLE_MAX_APP_ADDR, mask = 1;

    fle_sys_check_thread("GetSystemInfo");
    if (out == 0) STOP("GetSystemInfo(NULL) is not defined\n");
    memset(si, 0, sizeof si);
    memcpy(si + 0, &arch, 2);
    memcpy(si + 4, &page, 4);
    memcpy(si + 8, &lo, 8);
    memcpy(si + 16, &hi, 8);
    memcpy(si + 24, &mask, 8);
    memcpy(si + 32, &nproc, 4);
    memcpy(si + 36, &ptype, 4);
    memcpy(si + 40, &gran, 4);
    memcpy((void *)fle_addr(out, (unsigned)sizeof si, 1), si, sizeof si);
}

/* ---- VirtualAlloc, VirtualFree ----
 * (With FLE_VMEM_ARENA, wasm, a reservation is a range of the arena and the host protection calls are not made; see
 * "Arena" above and decision 0045. What follows describes the native build.)
 * A reservation is a host mapping made PROT_NONE and MAP_NORESERVE; a commit is mprotect on pages
 * inside a reservation; a decommit is madvise(DONTNEED) (so the next commit reads zero, MSDN:
 * VirtualAlloc, MEM_COMMIT) and PROT_NONE. A reservation is recorded as (base, size, one state byte
 * per page). The host page size must be FLE_PAGE_SIZE (checked at first use). The base of a
 * reservation made without an address is aligned to FLE_ALLOC_GRANULARITY. */
#define FLE_MEM_COMMIT   0x00001000u
#define FLE_MEM_RESERVE  0x00002000u
#define FLE_MEM_DECOMMIT 0x00004000u
#define FLE_MEM_RELEASE  0x00008000u
#define FLE_MEM_TOP_DOWN 0x00100000u

#define FLE_PAGE_NOACCESS          0x01u
#define FLE_PAGE_READONLY          0x02u
#define FLE_PAGE_READWRITE         0x04u
#define FLE_PAGE_EXECUTE           0x10u
#define FLE_PAGE_EXECUTE_READ      0x20u
#define FLE_PAGE_EXECUTE_READWRITE 0x40u

#define FLE_PG_RESERVED  0u
#define FLE_PG_COMMITTED 1u

#define FLE_MAX_REGIONS   65536u
#define FLE_MAX_SIZE      (UINT64_C(1) << 46)

typedef struct {
    uint64_t base;
    uint64_t size;      /* bytes, a multiple of FLE_PAGE_SIZE */
    uint8_t *pages;     /* size / FLE_PAGE_SIZE state bytes: FLE_PG_RESERVED (0), else the PAGE_* value of the committed page */
    uint32_t alloc_protect; /* flProtect of the call that reserved the region (decision 0024: VirtualQuery) */
} fle_region;

static fle_region *fle_regions;
static uint64_t fle_region_n, fle_region_cap;

uint64_t fle_sys_region_count(void) { return fle_region_n; }

static void fle_mem_init(void)
{
    static int done;

    if (done) return;
#ifndef FLE_VMEM_ARENA
    if (sysconf(_SC_PAGESIZE) != (long)FLE_PAGE_SIZE)
        STOP("host page size %ld is not %llu\n", sysconf(_SC_PAGESIZE), (unsigned long long)FLE_PAGE_SIZE);
#endif
    done = 1;
}

#ifdef FLE_VMEM_ARENA
/* ---- Arena (decision 0045): the host's mmap, mprotect and madvise are not used ----
 * The guest page is FLE_PAGE_SIZE (4096) whatever the host page size is (emscripten reports 65536, and its mprotect
 * and madvise do nothing). One block of FLE_VMEM_ARENA_BYTES is taken from the host heap at first use; a reservation
 * is a range of it (claimed in a table sorted by address, aligned to FLE_ALLOC_GRANULARITY). Page states and
 * protections stay in the region tables below and are the only record of them: the host does not enforce them, so an
 * access to a reserved, decommitted, read-only or PAGE_NOACCESS page does not fault (native: SIGSEGV). A page that
 * becomes committed is zeroed here (a commit reads zero, MSDN VirtualAlloc, MEM_COMMIT). An address given by the
 * guest is honoured only inside the arena; any other fixed address fails like an address that is in use. */
#define FLE_VMEM_ARENA_BYTES (UINT64_C(512) << 20)

static uint64_t fle_up(uint64_t v, uint64_t a);
static uint64_t fle_down(uint64_t v, uint64_t a);

typedef struct {
    uint64_t base;
    uint64_t size;
} fle_span;

static uint64_t fle_arena_base, fle_arena_end;
static fle_span *fle_spans;
static uint64_t fle_span_n, fle_span_cap;

static int fle_arena_init(void)
{
    void *p;

    if (fle_arena_base) return 1;
    p = aligned_alloc((size_t)FLE_ALLOC_GRANULARITY, (size_t)FLE_VMEM_ARENA_BYTES);
    if (!p) return 0;
    fle_arena_base = (uint64_t)(uintptr_t)p;
    fle_arena_end = fle_arena_base + FLE_VMEM_ARENA_BYTES;
    return 1;
}

/* Claims [base, base + size) (size is a multiple of the page size) at `want`, else (want 0) at the first free place, or
 * with `top` (MEM_TOP_DOWN, decision 0052) at the highest free place: the end of the arena, or the start of the lowest
 * claimed span above, whichever leaves room, with the start rounded down to FLE_ALLOC_GRANULARITY. Returns the base or 0. */
static uint64_t fle_host_claim(uint64_t want, uint64_t size, int top)
{
    uint64_t cand, i, at, limit;

    if (!fle_arena_init() || size == 0 || size > FLE_VMEM_ARENA_BYTES) return 0;
    if (want) {
        if (want < fle_arena_base || want > fle_arena_end || size > fle_arena_end - want) return 0;
        cand = want;
        for (i = 0; i < fle_span_n; i++)
            if (cand < fle_spans[i].base + fle_spans[i].size && fle_spans[i].base < cand + size) return 0;
    } else if (top) {
        limit = fle_arena_end;
        for (i = fle_span_n;; i--) {
            if (size > limit - fle_arena_base) return 0;
            cand = fle_down(limit - size, FLE_ALLOC_GRANULARITY);
            if (i == 0 || fle_spans[i - 1].base + fle_spans[i - 1].size <= cand) break;
            limit = fle_spans[i - 1].base;
        }
    } else {
        cand = fle_arena_base;
        for (i = 0; i < fle_span_n; i++) {
            if (cand + size <= fle_spans[i].base) break;
            if (fle_spans[i].base + fle_spans[i].size > cand) cand = fle_up(fle_spans[i].base + fle_spans[i].size, FLE_ALLOC_GRANULARITY);
        }
        if (cand > fle_arena_end || size > fle_arena_end - cand) return 0;
    }
    if (fle_span_n == fle_span_cap) {
        uint64_t cap = fle_span_cap ? fle_span_cap * 2 : 64;
        fle_span *n = realloc(fle_spans, cap * sizeof *n);

        if (!n) return 0;
        fle_spans = n;
        fle_span_cap = cap;
    }
    for (at = 0; at < fle_span_n && fle_spans[at].base < cand; at++) {}
    memmove(&fle_spans[at + 1], &fle_spans[at], (fle_span_n - at) * sizeof *fle_spans);
    fle_spans[at].base = cand;
    fle_spans[at].size = size;
    fle_span_n++;
    return cand;
}

uint64_t fle_sys_host_reserve(uint64_t want, uint64_t size)
{
    return fle_host_claim(want, size, 0);
}

void fle_sys_host_release(uint64_t base)
{
    uint64_t i;

    for (i = 0; i < fle_span_n; i++) {
        if (fle_spans[i].base == base) {
            memmove(&fle_spans[i], &fle_spans[i + 1], (fle_span_n - i - 1) * sizeof *fle_spans);
            fle_span_n--;
            return;
        }
    }
}
#endif

static uint64_t fle_down(uint64_t v, uint64_t a) { return v & ~(a - 1); }
static uint64_t fle_up(uint64_t v, uint64_t a) { return (v + a - 1) & ~(a - 1); }

static fle_region *fle_region_containing(uint64_t addr)
{
    uint64_t i;

    for (i = 0; i < fle_region_n; i++)
        if (addr >= fle_regions[i].base && addr - fle_regions[i].base < fle_regions[i].size) return &fle_regions[i];
    return NULL;
}

static int fle_region_overlaps(uint64_t base, uint64_t size)
{
    uint64_t i;

    for (i = 0; i < fle_region_n; i++)
        if (base < fle_regions[i].base + fle_regions[i].size && fle_regions[i].base < base + size) return 1;
    return 0;
}

/* Host protection for one flProtect value (MSDN: Memory Protection Constants). WRITECOPY and
 * EXECUTE_WRITECOPY are "not supported by VirtualAlloc"; the guard, no-cache and write-combine
 * modifiers are not modelled; anything else is not a protection constant. All stop the run. */
static int fle_host_prot(uint32_t p)
{
    switch (p) {
    case FLE_PAGE_NOACCESS:          return PROT_NONE;
    case FLE_PAGE_READONLY:          return PROT_READ;
    case FLE_PAGE_READWRITE:         return PROT_READ | PROT_WRITE;
    case FLE_PAGE_EXECUTE:           return PROT_EXEC;
    case FLE_PAGE_EXECUTE_READ:      return PROT_READ | PROT_EXEC;
    case FLE_PAGE_EXECUTE_READWRITE: return PROT_READ | PROT_WRITE | PROT_EXEC;
    default: STOP("VirtualAlloc flProtect %#x is not supported (not defined for VirtualAlloc or not modelled)\n", p);
    }
}

/* Maps `size` bytes PROT_NONE. With `want` it must land exactly there (free address space, else 0);
 * without, the base is FLE_ALLOC_GRANULARITY aligned. `top` (MEM_TOP_DOWN, only without `want`): arena, the highest free
 * place; native, not enforced (the host picks the place, decision 0052). Returns the base or 0. */
#ifdef FLE_VMEM_ARENA
static uint64_t fle_map_reserve(uint64_t want, uint64_t size, int top)
{
    return fle_host_claim(want, size, top);
}
#else
static uint64_t fle_map_reserve(uint64_t want, uint64_t size, int top)
{
    const int fl = MAP_PRIVATE | MAP_ANONYMOUS | MAP_NORESERVE;
    uint64_t span, a;
    void *p;

    (void)top;
    if (want) {
        p = mmap((void *)(uintptr_t)want, size, PROT_NONE, fl | MAP_FIXED_NOREPLACE, -1, 0);
        if (p == MAP_FAILED) return 0;
        if ((uint64_t)(uintptr_t)p != want) {
            munmap(p, size);
            return 0;
        }
        return want;
    }
    span = size + FLE_ALLOC_GRANULARITY - FLE_PAGE_SIZE;
    p = mmap(NULL, span, PROT_NONE, fl, -1, 0);
    if (p == MAP_FAILED) return 0;
    a = fle_up((uint64_t)(uintptr_t)p, FLE_ALLOC_GRANULARITY);
    if (a > (uint64_t)(uintptr_t)p) munmap(p, a - (uint64_t)(uintptr_t)p);
    if ((uint64_t)(uintptr_t)p + span > a + size) munmap((void *)(uintptr_t)(a + size), (uint64_t)(uintptr_t)p + span - (a + size));
    return a;
}
#endif

/* Gives a reservation back to the host (the whole range of one region). */
static void fle_unreserve(uint64_t base, uint64_t size)
{
#ifdef FLE_VMEM_ARENA
    (void)size;
    fle_sys_host_release(base);
#else
    munmap((void *)(uintptr_t)base, size);
#endif
}

static fle_region *fle_region_add(uint64_t base, uint64_t size)
{
    fle_region *r;
    uint8_t *pg = calloc(1, size / FLE_PAGE_SIZE);

    if (!pg) return NULL;
    if (fle_region_n == fle_region_cap) {
        uint64_t cap = fle_region_cap ? fle_region_cap * 2 : 64;
        fle_region *n;

        if (cap > FLE_MAX_REGIONS) STOP("more than %u VirtualAlloc regions\n", FLE_MAX_REGIONS);
        n = realloc(fle_regions, cap * sizeof *n);
        if (!n) {
            free(pg);
            return NULL;
        }
        fle_regions = n;
        fle_region_cap = cap;
    }
    r = &fle_regions[fle_region_n++];
    r->base = base;
    r->size = size;
    r->pages = pg;
    return r;
}

/* The PAGE_* value of a host protection that fle_host_prot produced (the six values map one to one). */
static uint8_t fle_win_prot(int prot)
{
    switch (prot) {
    case PROT_NONE:                            return FLE_PAGE_NOACCESS;
    case PROT_READ:                            return FLE_PAGE_READONLY;
    case PROT_READ | PROT_WRITE:               return FLE_PAGE_READWRITE;
    case PROT_EXEC:                            return FLE_PAGE_EXECUTE;
    case PROT_READ | PROT_EXEC:                return FLE_PAGE_EXECUTE_READ;
    default:                                   return FLE_PAGE_EXECUTE_READWRITE;
    }
}

/* The host side of "these pages get this protection". Native: mprotect. Arena: nothing (the table is the record);
 * pages that were only reserved are zeroed, because a fresh commit reads zero (MSDN VirtualAlloc, MEM_COMMIT) and the
 * arena is not zero-filled when it is taken from the host heap. */
static void fle_host_protect(fle_region *r, uint64_t from, uint64_t to, int prot, const char *what)
{
#ifdef FLE_VMEM_ARENA
    uint64_t pg;

    (void)what;
    (void)prot;
    for (pg = (from - r->base) / FLE_PAGE_SIZE; pg < (to - r->base) / FLE_PAGE_SIZE; pg++)
        if (r->pages[pg] == FLE_PG_RESERVED) memset((void *)(uintptr_t)(r->base + pg * FLE_PAGE_SIZE), 0, (size_t)FLE_PAGE_SIZE);
#else
    (void)r;
    if (mprotect((void *)(uintptr_t)from, to - from, prot) != 0) STOP("mprotect failed %s %#llx\n", what, (unsigned long long)from);
#endif
}

static void fle_commit(fle_region *r, uint64_t from, uint64_t to, int prot)
{
    uint64_t pg;

    fle_host_protect(r, from, to, prot, "on a reserved range");
    for (pg = (from - r->base) / FLE_PAGE_SIZE; pg < (to - r->base) / FLE_PAGE_SIZE; pg++) r->pages[pg] = fle_win_prot(prot);
}

/* Read and change access for fle_vmap.c (decision 0024). The table itself stays private to this file. */
int fle_sys_region_at(uint64_t addr, fle_sys_region_view *out)
{
    fle_region *r = fle_region_containing(addr);

    if (!r) return 0;
    out->base = r->base;
    out->size = r->size;
    out->alloc_protect = r->alloc_protect;
    out->pages = r->pages;
    return 1;
}

/* Sets the protection of the pages [from, to) of one region (both page aligned, already checked by the caller to
 * lie inside one region and to be committed). win_prot is one of the six PAGE_* values VirtualAlloc accepts. */
void fle_sys_set_protection(uint64_t from, uint64_t to, uint32_t win_prot)
{
    fle_region *r = fle_region_containing(from);
    int prot = fle_host_prot(win_prot);
    uint64_t pg;

    if (!r || to > r->base + r->size || to <= from) STOP("fle_sys_set_protection range %#llx is not inside a region\n", (unsigned long long)from);
    fle_host_protect(r, from, to, prot, "at");
    for (pg = (from - r->base) / FLE_PAGE_SIZE; pg < (to - r->base) / FLE_PAGE_SIZE; pg++) r->pages[pg] = fle_win_prot(prot);
}

/* kernel32!VirtualAlloc: LPVOID VirtualAlloc(LPVOID lpAddress, SIZE_T dwSize, DWORD flAllocationType,
 * DWORD flProtect). rcx, rdx, r8d, r9d. MSDN (VirtualAlloc, Parameters, Return value, Remarks):
 *  - flAllocationType: MEM_COMMIT (0x1000), MEM_RESERVE (0x2000), their combination, and with
 *    lpAddress NULL also MEM_TOP_DOWN (0x100000, "the highest possible address", decision 0052)
 *    added to one of those. The other values on the page (MEM_RESET, MEM_RESET_UNDO,
 *    MEM_LARGE_PAGES, MEM_PHYSICAL, MEM_WRITE_WATCH), MEM_TOP_DOWN with a non-NULL lpAddress,
 *    and a value with neither MEM_COMMIT nor MEM_RESERVE stop the run.
 *  - lpAddress NULL: the system chooses and dwSize is rounded up to the page size. Otherwise a
 *    reserve rounds the address down to the allocation granularity and a commit rounds it down to
 *    the page; the pages cover every byte of lpAddress..lpAddress+dwSize.
 *  - MEM_COMMIT without MEM_RESERVE at a non-NULL address fails with ERROR_INVALID_ADDRESS (487,
 *    named on the page) unless the whole range is reserved. Committing committed pages succeeds.
 *    Fresh pages read zero. The flProtect of the call is applied to the whole range, including
 *    pages that were already committed (the page does not say; assumption).
 *  - "VirtualAlloc cannot reserve a reserved page": a reserve (or reserve plus commit) over used
 *    address space returns NULL. The page does not name the code: 487 is set (assumption). Here
 *    "used" also covers host mappings that are not VirtualAlloc regions.
 *  - MEM_COMMIT alone with a NULL address is reserve plus commit in one step. The page says the
 *    system picks the region; that it also reserves is an inference (the guest depends on it:
 *    0x40769f, 0x40779b, 0x408847 pass rcx=0, r8=0x1000, r9=4).
 *  - dwSize 0 is not defined on the page and stops the run.
 *  - Out of memory returns NULL with ERROR_NOT_ENOUGH_MEMORY (8, assumption). The harness limits
 *    the address space (HARNESS_MAX_AS_BYTES), so a large reservation can fail this way. */
void fle_imp_VirtualAlloc(void)
{
    uint64_t addr = R[1], size = R[2];
    uint32_t type = (uint32_t)R[8], protect = (uint32_t)R[9];
    int prot, reserve, commit, top;
    uint64_t base, end;
    fle_region *r;

    fle_sys_check_thread("VirtualAlloc");
    fle_mem_init();
    top = (type & FLE_MEM_TOP_DOWN) != 0;
    if ((type & ~(FLE_MEM_COMMIT | FLE_MEM_RESERVE | FLE_MEM_TOP_DOWN)) != 0 || (type & (FLE_MEM_COMMIT | FLE_MEM_RESERVE)) == 0)
        STOP("VirtualAlloc flAllocationType %#x is not supported\n", type);
    if (top && addr != 0)
        STOP("VirtualAlloc flAllocationType %#x (MEM_TOP_DOWN) with lpAddress %#llx is not defined\n", type, (unsigned long long)addr);
    prot = fle_host_prot(protect);
    if (size == 0) STOP("VirtualAlloc with dwSize 0 is not defined\n");
    if (size > FLE_MAX_SIZE || addr > UINT64_C(0x7FFFFFFFFFFF) || addr + size < addr)
        STOP("VirtualAlloc range %#llx + %#llx is not defined\n", (unsigned long long)addr, (unsigned long long)size);
    reserve = (type & FLE_MEM_RESERVE) != 0;
    commit = (type & FLE_MEM_COMMIT) != 0;

    if (addr == 0) {
        size = fle_up(size, FLE_PAGE_SIZE);
        base = fle_map_reserve(0, size, top);
        if (!base) goto no_memory;
        r = fle_region_add(base, size);
        if (!r) {
            fle_unreserve(base, size);
            goto no_memory;
        }
        r->alloc_protect = protect;
        if (commit) fle_commit(r, base, base + size, prot);
        R[0] = base;
        return;
    }

    if (reserve) {
        base = fle_down(addr, FLE_ALLOC_GRANULARITY);
        end = fle_up(addr + size, FLE_PAGE_SIZE);
        if (base < FLE_MIN_APP_ADDR || fle_region_overlaps(base, end - base) || !fle_map_reserve(base, end - base, 0)) {
            fle_set_last_error(FLE_ERROR_INVALID_ADDRESS);
            R[0] = 0;
            return;
        }
        r = fle_region_add(base, end - base);
        if (!r) {
            fle_unreserve(base, end - base);
            goto no_memory;
        }
        r->alloc_protect = protect;
        if (commit) fle_commit(r, base, end, prot);
        R[0] = base;
        return;
    }

    base = fle_down(addr, FLE_PAGE_SIZE);
    end = fle_up(addr + size, FLE_PAGE_SIZE);
    r = fle_region_containing(base);
    if (!r || end > r->base + r->size) {
        fle_set_last_error(FLE_ERROR_INVALID_ADDRESS);
        R[0] = 0;
        return;
    }
    fle_commit(r, base, end, prot);
    R[0] = base;
    return;

no_memory:
    fle_set_last_error(FLE_ERROR_NOT_ENOUGH_MEMORY);
    R[0] = 0;
}

/* kernel32!VirtualFree: BOOL VirtualFree(LPVOID lpAddress, SIZE_T dwSize, DWORD dwFreeType). rcx,
 * rdx, r8d. MSDN (VirtualFree):
 *  - dwFreeType must be MEM_DECOMMIT (0x4000) or MEM_RELEASE (0x8000), alone. MEM_RELEASE with the
 *    placeholder modifiers, and any other value, stop the run.
 *  - MEM_RELEASE: dwSize must be 0 and lpAddress the base VirtualAlloc returned for the region;
 *    "the function fails if either of these conditions is not met" (returns 0; the code is not
 *    named, 87 is set, assumption). The whole region is released, committed or not.
 *  - MEM_DECOMMIT: the pages holding any byte of lpAddress..lpAddress+dwSize are decommitted and
 *    become reserved; uncommitted pages do not fail. dwSize 0 with the region's base decommits the
 *    whole region. A range outside every region, or one that spans two regions, is not described
 *    and stops the run. The contents are dropped (a later commit reads zero). */
void fle_imp_VirtualFree(void)
{
    uint64_t addr = R[1], size = R[2];
    uint32_t type = (uint32_t)R[8];
    fle_region *r;
    uint64_t from, to, pg;

    fle_sys_check_thread("VirtualFree");
    fle_mem_init();
    if (type == FLE_MEM_RELEASE) {
        uint64_t i;

        r = NULL;
        for (i = 0; i < fle_region_n; i++)
            if (fle_regions[i].base == addr) r = &fle_regions[i];
        if (size != 0 || !r) {
            fle_set_last_error(FLE_ERROR_INVALID_PARAMETER);
            R[0] = 0;
            return;
        }
#ifdef FLE_VMEM_ARENA
        fle_unreserve(r->base, r->size);
#else
        if (munmap((void *)(uintptr_t)r->base, r->size) != 0) STOP("munmap failed on region %#llx\n", (unsigned long long)r->base);
#endif
        free(r->pages);
        *r = fle_regions[--fle_region_n];
        R[0] = 1;
        return;
    }
    if (type != FLE_MEM_DECOMMIT) STOP("VirtualFree dwFreeType %#x is not supported\n", type);
    r = fle_region_containing(addr);
    if (!r) STOP("VirtualFree(MEM_DECOMMIT) at %#llx is outside every VirtualAlloc region\n", (unsigned long long)addr);
    if (size == 0) {
        if (addr != r->base) STOP("VirtualFree(MEM_DECOMMIT) with dwSize 0 at %#llx is not a region base\n", (unsigned long long)addr);
        from = r->base;
        to = r->base + r->size;
    } else {
        if (addr + size < addr) STOP("VirtualFree range overflows\n");
        from = fle_down(addr, FLE_PAGE_SIZE);
        to = fle_up(addr + size, FLE_PAGE_SIZE);
        if (to > r->base + r->size) STOP("VirtualFree(MEM_DECOMMIT) spans two regions at %#llx\n", (unsigned long long)addr);
    }
#ifndef FLE_VMEM_ARENA
    if (madvise((void *)(uintptr_t)from, to - from, MADV_DONTNEED) != 0) STOP("madvise failed at %#llx\n", (unsigned long long)from);
    if (mprotect((void *)(uintptr_t)from, to - from, PROT_NONE) != 0) STOP("mprotect failed at %#llx\n", (unsigned long long)from);
#endif
    for (pg = (from - r->base) / FLE_PAGE_SIZE; pg < (to - r->base) / FLE_PAGE_SIZE; pg++) r->pages[pg] = FLE_PG_RESERVED;
    R[0] = 1;
}

/* ---- Process heap ----
 * MSDN (GetProcessHeap, HeapAlloc, HeapFree). The process heap's handle is FLE_PROCESS_HEAP; HeapCreate (decision 0102 W6) adds private
 * heaps with their own tokens, and any other handle stops the run. A block is a
 * host malloc/calloc block, as in LocalAlloc (fle_imports.c), so a block from either can be freed
 * by either; malloc returns 16-byte aligned blocks on x86-64, which is MEMORY_ALLOCATION_ALIGNMENT
 * on Win64 (MSDN: HeapAlloc, Remarks). */
#define FLE_HEAP_NO_SERIALIZE        0x00000001u
#define FLE_HEAP_GENERATE_EXCEPTIONS 0x00000004u
#define FLE_HEAP_ZERO_MEMORY         0x00000008u
#define FLE_HEAP_REALLOC_IN_PLACE_ONLY 0x00000010u

void fle_imp_GetProcessHeap(void)
{
    fle_sys_check_thread("GetProcessHeap");
    R[0] = FLE_PROCESS_HEAP;
}

/* Private heaps (decision 0102 W6, provisional): HeapCreate hands out the tokens FLE_PRIVATE_HEAP_BASE + k * FLE_PRIVATE_HEAP_STEP (k from
 * 0, never reused), nothing is mapped there, and a block is a host malloc block like the process heap's. Created with flOptions 0 or
 * HEAP_NO_SERIALIZE / HEAP_GENERATE_EXCEPTIONS and dwMaximumSize 0 (growable) only; the rest stops by name. HeapDestroy is not answered. */
#define FLE_PRIVATE_HEAP_BASE UINT64_C(0x7FF700010000)
#define FLE_PRIVATE_HEAP_STEP UINT64_C(0x10000)
#define FLE_PRIVATE_HEAP_MAX  64u
static uint32_t private_heap_options[FLE_PRIVATE_HEAP_MAX];
static unsigned private_heap_count;

/* The index of the private heap whose token is h, or -1. */
static int private_heap_index(uint64_t h)
{
    uint64_t k;
    if (h < FLE_PRIVATE_HEAP_BASE || (h - FLE_PRIVATE_HEAP_BASE) % FLE_PRIVATE_HEAP_STEP != 0) return -1;
    k = (h - FLE_PRIVATE_HEAP_BASE) / FLE_PRIVATE_HEAP_STEP;
    return k < private_heap_count ? (int)k : -1;
}

static void fle_heap_handle(const char *api, uint64_t h)
{
    if (h != FLE_PROCESS_HEAP && private_heap_index(h) < 0)
        STOP("%s on heap handle %#llx (only GetProcessHeap's and HeapCreate's are known)\n", api, (unsigned long long)h);
}

/* HeapCreate(flOptions ecx, dwInitialSize rdx, dwMaximumSize r8) (MSDN: HeapCreate). Success returns a handle to a new private heap.
 * flOptions: HEAP_NO_SERIALIZE and HEAP_GENERATE_EXCEPTIONS are accepted (one thread; a failing allocation with the latter stops, as in
 * HeapAlloc); HEAP_CREATE_ENABLE_EXECUTE (code run from the heap) and any other bit stop. dwMaximumSize must be 0, the growable heap
 * ("limited only by the available memory"): a fixed-size heap's limits (the page: the heap cannot grow, blocks of about 1 MB and
 * over) are not modelled and stop. dwInitialSize only commits memory on Windows; every host malloc block is created on demand, so
 * it is not used, but the page's rule that it be smaller than a nonzero dwMaximumSize is not needed here. */
void fle_imp_HeapCreate(void)
{
    uint32_t options = (uint32_t)R[1];
    uint64_t maxsize = R[8];

    fle_sys_check_thread("HeapCreate");
    if (options & ~(FLE_HEAP_NO_SERIALIZE | FLE_HEAP_GENERATE_EXCEPTIONS)) STOP("HeapCreate flOptions %#x is not answered\n", options);
    if (maxsize != 0) STOP("HeapCreate dwMaximumSize %#llx: a fixed-size heap is not modelled\n", (unsigned long long)maxsize);
    if (private_heap_count == FLE_PRIVATE_HEAP_MAX) STOP("HeapCreate: more than %u private heaps\n", FLE_PRIVATE_HEAP_MAX);
    private_heap_options[private_heap_count] = options;
    R[0] = FLE_PRIVATE_HEAP_BASE + FLE_PRIVATE_HEAP_STEP * private_heap_count++;
}

/* HeapSetInformation(HeapHandle rcx, HeapInformationClass edx, HeapInformation r8, HeapInformationLength r9) (MSDN: HeapSetInformation).
 * HeapHandle is NULL, the process heap or a private heap. HeapCompatibilityInformation (0) needs a heap handle and a ULONG of 4 bytes:
 * the value 2 (the low-fragmentation heap) succeeds for a heap created without HEAP_NO_SERIALIZE and fails with ERROR_INVALID_PARAMETER
 * (the page names no code; the harness's choice) for one created with it ("cannot be enabled for heaps created with
 * HEAP_NO_SERIALIZE"); the page does not describe the other values, they stop. HeapEnableTerminationOnCorruption (1) with a NULL
 * HeapInformation and length 0 succeeds and changes nothing (there is no heap corruption model). Every other class, and the other
 * arguments for these two, stop. */
void fle_imp_HeapSetInformation(void)
{
    uint64_t h = R[1], info = R[8], len = R[9];
    uint32_t cls = (uint32_t)R[2];

    fle_sys_check_thread("HeapSetInformation");
    if (h != 0) fle_heap_handle("HeapSetInformation", h);
    if (cls == 1) {
        if (info != 0 || len != 0) STOP("HeapSetInformation(HeapEnableTerminationOnCorruption) with HeapInformation/Length other than NULL/0\n");
        R[0] = 1;
        return;
    }
    if (cls == 0) {
        uint32_t v;
        if (h == 0) STOP("HeapSetInformation(HeapCompatibilityInformation) with a NULL heap handle\n");
        if (info == 0 || len != 4) STOP("HeapSetInformation(HeapCompatibilityInformation) with HeapInformationLength %llu\n", (unsigned long long)len);
        v = (uint32_t)fle_ld(info, 4);
        if (v != 2) STOP("HeapSetInformation(HeapCompatibilityInformation) with the value %u (the page describes only the LFH, 2)\n", v);
        if (h != FLE_PROCESS_HEAP && (private_heap_options[private_heap_index(h)] & FLE_HEAP_NO_SERIALIZE)) {
            fle_set_last_error(87u);
            R[0] = 0;
            return;
        }
        R[0] = 1;
        return;
    }
    STOP("HeapSetInformation class %u is not answered\n", cls);
}

/* The requested size of every block that HeapAlloc returned and HeapFree has not freed (decision 0102 W11, provisional; decision 0078 left
 * HeapSize a named stop because nothing recorded it): open addressing on the block address, a tombstone for a freed block. The blocks stay
 * plain host malloc blocks, so LocalAlloc/LocalFree and HeapAlloc/HeapFree keep working on each other's blocks as before; a block that
 * HeapAlloc did not return (a LocalAlloc block) has no entry, and HeapSize on it stops. The heap handle is recorded with the size. */
typedef struct {
    uint64_t block;  /* 0 = empty, UINT64_MAX = freed */
    uint64_t size;
    uint64_t heap;
} heap_block;
static heap_block *heap_blocks;
static size_t heap_blocks_cap, heap_blocks_used; /* used counts live entries and tombstones */

static size_t heap_block_slot(uint64_t p, size_t cap)
{
    return (size_t)((p >> 4) * UINT64_C(0x9E3779B97F4A7C15) >> 20) & (cap - 1);
}

static heap_block *heap_block_find(uint64_t p)
{
    size_t i, n;
    if (heap_blocks_cap == 0) return NULL;
    i = heap_block_slot(p, heap_blocks_cap);
    for (n = 0; n < heap_blocks_cap; n++, i = (i + 1) & (heap_blocks_cap - 1)) {
        if (heap_blocks[i].block == 0) return NULL;
        if (heap_blocks[i].block == p) return &heap_blocks[i];
    }
    return NULL;
}

static void heap_block_add(uint64_t p, uint64_t size, uint64_t heap)
{
    heap_block *e = heap_block_find(p);
    size_t i, n;
    if (e) {
        e->size = size;
        e->heap = heap;
        return;
    }
    if ((heap_blocks_used + 1) * 2 > heap_blocks_cap) {
        size_t ncap = heap_blocks_cap ? heap_blocks_cap * 2 : 1024, k;
        heap_block *nt = calloc(ncap, sizeof *nt);
        if (!nt) STOP("out of memory (the table of HeapAlloc block sizes)\n");
        heap_blocks_used = 0;
        for (k = 0; k < heap_blocks_cap; k++) {
            if (heap_blocks[k].block == 0 || heap_blocks[k].block == UINT64_MAX) continue;
            i = heap_block_slot(heap_blocks[k].block, ncap);
            while (nt[i].block != 0) i = (i + 1) & (ncap - 1);
            nt[i] = heap_blocks[k];
            heap_blocks_used++;
        }
        free(heap_blocks);
        heap_blocks = nt;
        heap_blocks_cap = ncap;
    }
    i = heap_block_slot(p, heap_blocks_cap);
    for (n = 0; n < heap_blocks_cap; n++, i = (i + 1) & (heap_blocks_cap - 1)) {
        if (heap_blocks[i].block == 0 || heap_blocks[i].block == UINT64_MAX) {
            if (heap_blocks[i].block == 0) heap_blocks_used++;
            heap_blocks[i].block = p;
            heap_blocks[i].size = size;
            heap_blocks[i].heap = heap;
            return;
        }
    }
    STOP("the table of HeapAlloc block sizes is full\n");
}

/* HeapAlloc(hHeap rcx, dwFlags edx, dwBytes r8). Flags: HEAP_NO_SERIALIZE (accepted, one thread),
 * HEAP_GENERATE_EXCEPTIONS (failure would raise an exception; there is no exception model, so a
 * failure with this flag stops the run), HEAP_ZERO_MEMORY. Any other bit stops the run. Failure
 * without the exception flag returns NULL and does not set the last error (MSDN: "it does not call
 * SetLastError"). dwBytes 0 is allocated as 1 byte (the page says "at least the amount requested",
 * not what 0 gives; same choice as LocalAlloc). */
void fle_imp_HeapAlloc(void)
{
    uint32_t flags = (uint32_t)R[2];
    uint64_t bytes = R[8], requested;
    void *p;

    fle_sys_check_thread("HeapAlloc");
    fle_heap_handle("HeapAlloc", R[1]);
    if (flags & ~(FLE_HEAP_NO_SERIALIZE | FLE_HEAP_GENERATE_EXCEPTIONS | FLE_HEAP_ZERO_MEMORY))
        STOP("HeapAlloc dwFlags %#x is not defined\n", flags);
    requested = bytes;
    if (bytes == 0) bytes = 1;
    p = !fle_host_size_fits(bytes) ? NULL : (flags & FLE_HEAP_ZERO_MEMORY) ? calloc(1, (size_t)bytes) : malloc((size_t)bytes);
    if (!p && (flags & FLE_HEAP_GENERATE_EXCEPTIONS)) STOP("HeapAlloc failed with HEAP_GENERATE_EXCEPTIONS (exception not modelled)\n");
    if (p) heap_block_add((uint64_t)(uintptr_t)p, requested, R[1]);
    R[0] = (uint64_t)(uintptr_t)p;
}

/* kernel32!HeapSize and HeapReAlloc stopped by name in decision 0078 (HeapSize is answered since decision 0102 W11, HeapReAlloc since decision
 * 0113 wall 1; both use the table of requested sizes above). The process-heap blocks are plain host malloc blocks (above), so the
 * "requested size" that HeapSize returns (page: "the requested size of the allocated memory block") was not recorded anywhere, and
 * HeapReAlloc's flags (HEAP_REALLOC_IN_PLACE_ONLY, HEAP_ZERO_MEMORY on growth) need that size. The lifted dsp_ippv2_x64.dll reaches them
 * only through its CRT realloc (0x17d9ea0), _msize (0x17d9e64) and _recalloc (0x17d7264): the readings in decision 0078 find _recalloc
 * called only by the argv wildcard expansion (0x17d5d44), which _configure_narrow_argv runs for mode 2 and not for mode 1, the mode the
 * DLL's start-up passes; the IPP allocator uses malloc and free (HeapAlloc, HeapFree). */
/* HeapSize(hHeap rcx, dwFlags edx, lpMem r8) (MSDN: HeapSize; decision 0102 W11, provisional): "the requested size of the allocated memory
 * block, in bytes" for a block that HeapAlloc returned and HeapFree has not freed, from the table above. dwFlags 0 or HEAP_NO_SERIALIZE
 * (the page's two values; anything else stops), lpMem NULL, a block with no entry (not returned by HeapAlloc, or already freed) and a block
 * of another heap stop: the page defines the failure value (-1) but not when a bad pointer fails, and a pointer outside the heap is
 * undefined behaviour, so a stop is the fail-closed answer. */
void fle_imp_HeapSize(void)
{
    uint32_t flags = (uint32_t)R[2];
    heap_block *e;

    fle_sys_check_thread("HeapSize");
    fle_heap_handle("HeapSize", R[1]);
    if (flags & ~FLE_HEAP_NO_SERIALIZE) STOP("HeapSize dwFlags %#x is not defined\n", flags);
    if (R[8] == 0) STOP("HeapSize on a NULL block\n");
    e = heap_block_find(R[8]);
    if (e == NULL) STOP("HeapSize on %#llx: not a block that HeapAlloc returned (or already freed)\n", (unsigned long long)R[8]);
    if (e->heap != R[1]) STOP("HeapSize on %#llx: the block belongs to heap %#llx, not %#llx\n", (unsigned long long)R[8], (unsigned long long)e->heap, (unsigned long long)R[1]);
    R[0] = e->size;
}

/* HeapReAlloc(hHeap rcx, dwFlags edx, lpMem r8, dwBytes r9) (MSDN: HeapReAlloc; decision 0113 wall 1, provisional): the block must be one that
 * HeapAlloc or HeapReAlloc returned and HeapFree has not freed, of the same heap (from the table above; the page: "a pointer ... returned by an
 * earlier call to the HeapAlloc or HeapReAlloc function"). A NULL lpMem, an unknown or freed block and a block of another heap stop, as in
 * HeapSize. dwFlags: HEAP_NO_SERIALIZE, HEAP_GENERATE_EXCEPTIONS, HEAP_REALLOC_IN_PLACE_ONLY (0x10), HEAP_ZERO_MEMORY; any other bit stops.
 * The block is a host realloc block (so the contents are preserved up to the smaller size, the pointer may change, and the 16-byte
 * alignment of HeapAlloc stays); the recorded requested size becomes dwBytes (the table entry moves with the block). dwBytes 0 is
 * reallocated as 1 byte, as HeapAlloc does. HEAP_ZERO_MEMORY zeroes the bytes from the old requested size to dwBytes when the block grows.
 * Failure returns NULL, leaves the original block valid and does not call SetLastError (the page); with HEAP_GENERATE_EXCEPTIONS, from the
 * call or from HeapCreate, a failure stops (no exception model). HEAP_REALLOC_IN_PLACE_ONLY: the page says the call fails when the block
 * "cannot be resized without moving"; whether a block can grow in place is not stated, so a growth stops by name, while the same size or a
 * smaller one keeps the block where it is (a choice without a source: shrinking never needs to move a block). */
void fle_imp_HeapReAlloc(void)
{
    uint32_t flags = (uint32_t)R[2];
    uint64_t heap = R[1], old = R[8], bytes = R[9], requested = R[9], oldsize;
    heap_block *e;
    void *p;
    int exceptions;

    fle_sys_check_thread("HeapReAlloc");
    fle_heap_handle("HeapReAlloc", heap);
    if (flags & ~(FLE_HEAP_NO_SERIALIZE | FLE_HEAP_GENERATE_EXCEPTIONS | FLE_HEAP_REALLOC_IN_PLACE_ONLY | FLE_HEAP_ZERO_MEMORY))
        STOP("HeapReAlloc dwFlags %#x is not defined\n", flags);
    if (old == 0) STOP("HeapReAlloc on a NULL block (the page names only blocks returned by HeapAlloc or HeapReAlloc)\n");
    e = heap_block_find(old);
    if (e == NULL) STOP("HeapReAlloc on %#llx: not a block that HeapAlloc or HeapReAlloc returned (or already freed)\n", (unsigned long long)old);
    if (e->heap != heap) STOP("HeapReAlloc on %#llx: the block belongs to heap %#llx, not %#llx\n", (unsigned long long)old, (unsigned long long)e->heap, (unsigned long long)heap);
    oldsize = e->size;
    exceptions = (flags & FLE_HEAP_GENERATE_EXCEPTIONS) != 0 || (heap != FLE_PROCESS_HEAP && (private_heap_options[private_heap_index(heap)] & FLE_HEAP_GENERATE_EXCEPTIONS) != 0);
    if (flags & FLE_HEAP_REALLOC_IN_PLACE_ONLY) {
        if (requested > oldsize)
            STOP("HeapReAlloc(HEAP_REALLOC_IN_PLACE_ONLY) to grow %#llx from %llu to %llu bytes: whether a block can grow without moving is not stated\n", (unsigned long long)old, (unsigned long long)oldsize, (unsigned long long)requested);
        e->size = requested;
        R[0] = old;
        return;
    }
    if (bytes == 0) bytes = 1;
    p = !fle_host_size_fits(bytes) ? NULL : realloc((void *)(uintptr_t)old, (size_t)bytes);
    if (!p) {
        if (exceptions) STOP("HeapReAlloc failed with HEAP_GENERATE_EXCEPTIONS (exception not modelled)\n");
        R[0] = 0;
        return;
    }
    if ((flags & FLE_HEAP_ZERO_MEMORY) && requested > oldsize) memset((uint8_t *)p + oldsize, 0, (size_t)(requested - oldsize));
    if ((uint64_t)(uintptr_t)p == old) {
        e->size = requested;
    } else {
        e->block = UINT64_MAX;
        heap_block_add((uint64_t)(uintptr_t)p, requested, heap);
    }
    R[0] = (uint64_t)(uintptr_t)p;
}

/* HeapFree(hHeap rcx, dwFlags edx, lpMem r8). dwFlags: 0 or HEAP_NO_SERIALIZE. lpMem may be NULL
 * (MSDN); the return value for NULL is not stated and is nonzero here (assumption). A pointer that
 * was not allocated from the heap is not detected (undefined on the page; free() decides). */
void fle_imp_HeapFree(void)
{
    uint32_t flags = (uint32_t)R[2];

    fle_sys_check_thread("HeapFree");
    fle_heap_handle("HeapFree", R[1]);
    if (flags & ~FLE_HEAP_NO_SERIALIZE) STOP("HeapFree dwFlags %#x is not defined\n", flags);
    {
        heap_block *e = R[8] ? heap_block_find(R[8]) : NULL;
        if (e) e->block = UINT64_MAX;
    }
    free((void *)fle_addr_opt(R[8], 1, 1));
    R[0] = 1;
}

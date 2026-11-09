/* mt2 (decision 0234, PROVISIONAL, measurement only): census of the synchronisation calls of the guest threads. It is linked with -Wl,--wrap=fle_imp_<API> for the APIs in MT2_APIS and
 * -Wl,--wrap=fle_audio_switch_note (tools/mt2-var.sh TRACE=1). It changes nothing that the guest sees: every wrapper reads the arguments from the register file (rcx = R[1], rdx = R[2]),
 * calls the real implementation, reads the result (R[0]) and counts or logs. Not part of any shipping link; no guest function is named here.
 * Output on stderr at exit and every FLE_MT2_DUMP_VMS of virtual time: "MT2C" lines = counters. With FLE_MT2_WIN=a:b (virtual ms) one "MT2T" line per call and per thread switch inside the window.
 * Run time per thread: the wall time between two switches (the scheduler is cooperative) is charged to the thread that ran (node, one process; compare with the CPU seconds of /usr/bin/time). */
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include "fle_runtime.h"
#include "fle_thread.h"
#include "fle_sys.h"

#define MT2_APIS(X) \
    X(EnterCriticalSection) X(LeaveCriticalSection) X(TryEnterCriticalSection) X(Sleep) X(SwitchToThread) X(WaitForSingleObject) X(WaitForSingleObjectEx) \
    X(WaitForMultipleObjects) X(WaitForMultipleObjectsEx) X(MsgWaitForMultipleObjects) X(MsgWaitForMultipleObjectsEx) X(SetEvent) X(ResetEvent) X(ReleaseSemaphore) \
    X(DwmFlush) X(CreateThread) X(ResumeThread) X(PostMessageW) X(SendMessageW) X(SendMessageA) X(GetMessageW) X(PeekMessageW) X(WaitMessage) \
    X(SleepConditionVariableSRW) X(WakeConditionVariable) X(WakeAllConditionVariable) X(AcquireSRWLockExclusive) X(AcquireSRWLockShared) X(ReleaseSRWLockExclusive) X(ReleaseSRWLockShared) \
    X(InterlockedPushEntrySList) X(InterlockedPopEntrySList) X(InterlockedFlushSList) X(TlsGetValue) X(TlsSetValue) X(GetTickCount) X(timeGetTime) X(QueryPerformanceCounter) \
    X(GetLastError) X(SetLastError) X(SetThreadPriority) X(CloseHandle) X(GetCurrentThreadId) X(GetSystemTimeAsFileTime) X(GetSystemTimePreciseAsFileTime)

enum {
#define X(n) A_##n,
    MT2_APIS(X)
#undef X
    A_COUNT
};
static const char *const api_name[A_COUNT] = {
#define X(n) #n,
    MT2_APIS(X)
#undef X
};

#define MAXTH 40
typedef struct { uint64_t calls, blocked; double run_ms; } cnt_t;      /* run_ms = run-clock of the calling thread spent inside the call (a switch inside the call is not charged) */
static cnt_t C[MAXTH][A_COUNT];
static double run_ms[MAXTH]; static uint64_t slices[MAXTH]; static double longest_ms[MAXTH];
static double last_switch_ms = -1; static unsigned running = 0; static uint64_t sw_count = 0;
static uint64_t vmt_of[MAXTH], start_of[MAXTH], param_of[MAXTH];
static double t0_ms = -1;

typedef struct { uint64_t key; uint16_t api; int used; uint32_t n[MAXTH], blocked[MAXTH], depth[MAXTH]; double hold_ms[MAXTH], since[MAXTH]; } obj_t;
#define NOBJ 3000
static obj_t O[NOBJ];
static unsigned key_api(unsigned api)
{
    switch (api) {
    case A_LeaveCriticalSection: case A_TryEnterCriticalSection: return A_EnterCriticalSection;
    case A_ReleaseSRWLockExclusive: case A_ReleaseSRWLockShared: case A_AcquireSRWLockShared: return A_AcquireSRWLockExclusive;
    case A_InterlockedPopEntrySList: case A_InterlockedFlushSList: return A_InterlockedPushEntrySList;
    case A_WakeAllConditionVariable: case A_SleepConditionVariableSRW: return A_WakeConditionVariable;
    default: return api;
    }
}
static obj_t *obj_find(unsigned api, uint64_t key)
{
    api = key_api(api);
    unsigned i = (unsigned)(((key >> 3) * 0x9E3779B97F4A7C15ull + api * 131u) >> 32) % NOBJ, k;
    for (k = 0; k < NOBJ; k++, i = (i + 1) % NOBJ) {
        if (!O[i].used) { O[i].used = 1; O[i].key = key; O[i].api = (uint16_t)api; return &O[i]; }
        if (O[i].key == key && O[i].api == api) return &O[i];
    }
    return &O[0];
}

static double now_ms(void) { struct timespec ts; clock_gettime(CLOCK_MONOTONIC, &ts); return ts.tv_sec * 1e3 + ts.tv_nsec / 1e6; }
static uint64_t vms(void) { return fle_sys_clock_ticks() / FLE_TICKS_PER_MS; }
/* the run-clock of thread th: its charged run time plus the part of the slice that is going on */
static double rclock(unsigned th, double t) { double r = th < MAXTH ? run_ms[th] : 0; if (th == running && last_switch_ms >= 0) r += t - last_switch_ms; return r; }

static int win_set, dump_set; static uint64_t win_a, win_b, win_period, dump_every, next_dump;
static int in_win(uint64_t v) { if (!win_set || v < win_a) return 0; if (!win_period) return v < win_b; return ((v - win_a) % win_period) < (win_b - win_a); }
static void env_init(void)
{
    const char *e;
    if (t0_ms >= 0) return;
    t0_ms = now_ms();
    e = getenv("FLE_MT2_WIN");
    if (e && sscanf(e, "%" SCNu64 ":%" SCNu64 ":%" SCNu64, &win_a, &win_b, &win_period) >= 2) win_set = 1;   /* a:b[:period] in virtual ms: the window [a,b) of every period */
    e = getenv("FLE_MT2_DUMP_VMS");
    if (e && (dump_every = strtoull(e, NULL, 10)) > 0) { dump_set = 1; next_dump = dump_every; }
}

/* census of every other API of the runtime (generated list, tools/mt2-gen-census.py): calls per thread */
extern const char *const mt2_census_names[]; extern const unsigned mt2_census_n;
static uint32_t CEN[MAXTH][1024];
void mt2_census_hit(unsigned id) { unsigned th = fle_thread_current(); if (th >= MAXTH) th = MAXTH - 1; CEN[th][id]++; }
static void dump(const char *why)
{
    unsigned t, a, i;
    fprintf(stderr, "MT2C begin %s vms=%" PRIu64 " wall_ms=%.1f switches=%" PRIu64 "\n", why, vms(), now_ms() - t0_ms, sw_count);
    for (t = 0; t < MAXTH; t++) {
        if (!run_ms[t] && !slices[t]) continue;
        fprintf(stderr, "MT2C thread %u run_ms=%.2f slices=%" PRIu64 " longest_ms=%.2f vmt=%#" PRIx64 " start=%#" PRIx64 " param=%#" PRIx64 "\n", t, run_ms[t], slices[t], longest_ms[t], vmt_of[t], start_of[t], param_of[t]);
    }
    for (t = 0; t < MAXTH; t++) for (a = 0; a < A_COUNT; a++) if (C[t][a].calls)
        fprintf(stderr, "MT2C api th=%u %s calls=%" PRIu64 " blocked=%" PRIu64 " run_ms=%.2f\n", t, api_name[a], C[t][a].calls, C[t][a].blocked, C[t][a].run_ms);
    for (i = 0; i < NOBJ; i++) if (O[i].used)
        for (t = 0; t < MAXTH; t++) if (O[i].n[t])
            fprintf(stderr, "MT2C obj %s key=%#" PRIx64 " th=%u n=%u blocked=%u hold_ms=%.2f\n", api_name[O[i].api], O[i].key, t, O[i].n[t], O[i].blocked[t], O[i].hold_ms[t]);
    for (t = 0; t < MAXTH; t++) for (a = 0; a < mt2_census_n && a < 1024; a++) if (CEN[t][a]) fprintf(stderr, "MT2C cen th=%u %s calls=%u\n", t, mt2_census_names[a], CEN[t][a]);
    fprintf(stderr, "MT2C end\n");
    fflush(stderr);
}
static int atexit_set;
static void at_exit(void) { dump("exit"); }

void __real_fle_audio_switch_note(unsigned from, unsigned to) __attribute__((weak));
void __wrap_fle_audio_switch_note(unsigned from, unsigned to)
{
    double t = now_ms();
    env_init();
    if (!atexit_set) { atexit_set = 1; atexit(at_exit); }
    if (last_switch_ms >= 0 && from < MAXTH) {
        double d = t - last_switch_ms;
        run_ms[from] += d; slices[from]++;
        if (d > longest_ms[from]) longest_ms[from] = d;
    }
    last_switch_ms = t; running = to; sw_count++;
    { uint64_t v = win_set ? vms() : 0; if (in_win(v)) fprintf(stderr, "MT2T S %.3f %" PRIu64 " %u %u\n", t - t0_ms, v, from, to); }
    if (__real_fle_audio_switch_note) __real_fle_audio_switch_note(from, to);
}

static void call_wrapped(unsigned api, void (*real)(void))
{
    uint64_t a1, a2, a8, a9, v0, v1, ret, c0;
    double t0, t1, r0, r1;
    unsigned th, n0, th_new = 0;
    int is_obj = 0;
    obj_t *o = NULL;

    env_init();
    a1 = R[1]; a2 = R[2]; a8 = R[8]; a9 = R[9];
    th = fle_thread_current(); if (th >= MAXTH) th = MAXTH - 1;
    n0 = fle_thread_count();
    t0 = now_ms(); r0 = rclock(th, t0); v0 = vms(); c0 = sw_count;
    switch (api) {
    case A_EnterCriticalSection: case A_TryEnterCriticalSection: case A_LeaveCriticalSection:
    case A_SetEvent: case A_ResetEvent: case A_WaitForSingleObject: case A_WaitForSingleObjectEx: case A_ReleaseSemaphore: case A_CloseHandle:
    case A_AcquireSRWLockExclusive: case A_AcquireSRWLockShared: case A_ReleaseSRWLockExclusive: case A_ReleaseSRWLockShared:
    case A_WakeConditionVariable: case A_WakeAllConditionVariable: case A_SleepConditionVariableSRW:
    case A_InterlockedPushEntrySList: case A_InterlockedPopEntrySList: case A_InterlockedFlushSList:
    case A_PostMessageW: case A_SendMessageW: case A_SendMessageA: case A_ResumeThread: case A_SetThreadPriority:
        is_obj = 1; o = obj_find(api, a1); break;
    default: break;
    }
    if (api == A_LeaveCriticalSection && o) {   /* the hold time ends at Leave of the outermost level (run-clock of this thread) */
        if (o->depth[th] > 0 && --o->depth[th] == 0) o->hold_ms[th] += r0 - o->since[th];
    }
    real();
    t1 = now_ms(); r1 = rclock(th, t1); v1 = vms();
    ret = R[0];
    C[th][api].calls++;
    if (sw_count != c0) C[th][api].blocked++;
    C[th][api].run_ms += (sw_count == c0) ? (r1 - r0) : 0;
    if (o) {
        o->n[th]++;
        if (sw_count != c0) o->blocked[th]++;
        if ((api == A_EnterCriticalSection || (api == A_TryEnterCriticalSection && (uint32_t)ret)) && o->depth[th]++ == 0) o->since[th] = r1;
    }
    if (api == A_CreateThread && ret) {
        unsigned n1 = fle_thread_count();
        if (n1 == n0 + 1 && n0 < MAXTH) {
            uint64_t obj = 0, vmt = 0;
            th_new = n0; start_of[th_new] = a8; param_of[th_new] = a9;
            if (a9 >= 0x10000 && a9 < 0x7ff00000u && !(a9 & 7)) {   /* a Delphi TThread record {proc, TThread}: the VMT is the first qword of the object (a guess: only the printed class address is used) */
                obj = fle_ld(a9 + 8, 8);
                if (obj >= 0x10000 && obj < 0x7ff00000u && !(obj & 7)) { vmt = fle_ld(obj, 8); if (vmt < 0x400000 || vmt >= 0x20000000u) vmt = 0; }
            }
            vmt_of[th_new] = vmt;
        }
    }
    if (in_win(v0))
        fprintf(stderr, "MT2T C %u %s %#" PRIx64 " %#" PRIx64 " %#" PRIx64 " b=%d tr=%.3f..%.3f v=%" PRIu64 "..%" PRIu64 "\n", th, api_name[api], a1, a2, ret, sw_count != c0, r0, r1, v0, v1);
    if (dump_set && v1 >= next_dump) { next_dump = (v1 / dump_every + 1) * dump_every; dump("periodic"); }
    (void)is_obj; (void)th_new;
}

#define W(n) void __real_fle_imp_##n(void) __attribute__((weak)); void __wrap_fle_imp_##n(void) { call_wrapped(A_##n, __real_fle_imp_##n); }
#define X(n) W(n)
MT2_APIS(X)
#undef X

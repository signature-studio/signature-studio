/* A4 survey report (see a4.h). Same code for the native and the wasm harness.
 *
 * Definitions (decision 0060):
 * - runnable   = proc != 0 and LD64(proc) != 0 (the test of f_4120c0 at 0x412115..0x412124).
 * - InitCount is written by f_4120c0 as index+1 BEFORE the unit is called (also for empty and noproc units):
 *   it counts positions reached, not units completed.
 * - started    = runnable units with index < InitCount (f_4120c0 calls the runnable units in index order).
 * - in_progress = 1 when the last started unit's proc is on the call stack at the stop, else 0.
 * - completed  = started - in_progress.
 * - all_units_done = InitCount == UnitCount and the last started unit is not in progress (started == runnable follows
 *   from InitCount == UnitCount). */
#include <stdio.h>

#include "a4.h"
#include "boot.h"
#include "fle_dispatch.h"
#include "fle_runtime.h"

#define A4_CTX UINT64_C(0x15EAC90)
#define A4_COUNT UINT64_C(0x15EAC98)
#define A4_MAX_UNITS (1u << 16)
#define ULL(x) ((unsigned long long)(x))

static uint64_t img_base, img_size;

void a4_set_image(uint64_t base, uint64_t size) {
    img_base = base;
    img_size = size;
}

static int in_image(uint64_t va, uint64_t n) {
    /* va below img_base wraps to a huge offset and fails the second test */
    return img_size && va - img_base <= img_size && n <= img_size - (va - img_base);
}

static void *fn_of(uint64_t va) {
    uint64_t lo = 0, hi = fle_dispatch_count;
    while (lo < hi) {
        uint64_t mid = lo + (hi - lo) / 2;
        if (fle_dispatch_table[mid].va < va) lo = mid + 1;
        else hi = mid;
    }
    if (lo < fle_dispatch_count && fle_dispatch_table[lo].va == va) return (void *)fle_dispatch_table[lo].fn;
    return NULL;
}

/* 1 runnable, 0 empty or noproc, -1 proc outside the image */
static int runnable(uint64_t proc) {
    if (proc == 0) return 0;
    if (!in_image(proc, 8)) return -1;
    return fle_ld(proc, 8) != 0;
}

/* Decision 0113 (corrected): the program's own exit path (the program is Delphi-built, read from the compiler string in the binary: f_4128b0 zeroes
 * InitCount, f_412770 zeroes InitContext+0, then the guest's longjmp returns to the entry; the unit finalizers are not called) leaves InitContext+0
 * and InitCount zero, so a run that returns from the entry normally shows "InitContext+0 is 0" at the report although InitUnits ran to the
 * end. The trace hook (native) calls a4_sample() at every lifted function entry: it keeps the last non-zero table header and the largest InitCount
 * seen while the header was non-zero. a4_report uses them only when the header is zero at the report; otherwise nothing here is used. */
static uint64_t smp_hdr, smp_count;

void a4_sample(void) {
    uint64_t h, c;
    if (!img_size) return;
    h = fle_ld(A4_CTX, 8);
    if (h == 0) return;
    c = fle_ld(A4_COUNT, 4);
    smp_hdr = h;
    if (c > smp_count) smp_count = c;
}

void a4_report(void) {
    uint64_t hdr, units, nu, count, n_run = 0, started = 0, completed, rank = 0;
    uint64_t not_entered = 0, beyond = 0, last_idx = 0, last_proc = 0, last_rank = 0, at_count_proc = 0;
    int have_last = 0, in_progress = 0, bad = 0;

    if (!img_size) {
        printf("a4: image not loaded, nothing to read\n");
        return;
    }
    hdr = fle_ld(A4_CTX, 8);
    count = fle_ld(A4_COUNT, 4);
    if (hdr == 0 && smp_hdr != 0) {
        printf("a4: note: InitContext+0 is 0 now (the program's exit path zeroed it; the unit finalizers were not called); the table 0x%llx and init_count=%llu are the last non-zero header and the largest InitCount sampled at lifted function entries\n",
               ULL(smp_hdr), ULL(smp_count));
        hdr = smp_hdr;
        count = smp_count;
    }
    if (hdr == 0) {
        printf("a4: init_count=%llu of ?, InitContext+0 is 0 (InitUnits not reached)\n", ULL(count));
        return;
    }
    if (!in_image(hdr, 16)) {
        printf("a4: init_count=%llu of ?, InitContext+0=0x%llx is outside the image\n", ULL(count), ULL(hdr));
        return;
    }
    nu = fle_ld(hdr, 4);
    units = fle_ld(hdr + 8, 8);
    if (nu > A4_MAX_UNITS || !in_image(units, 16 * nu)) {
        printf("a4: init_count=%llu of ?, bad table header 0x%llx (count %llu, units 0x%llx)\n", ULL(count), ULL(hdr),
               ULL(nu), ULL(units));
        return;
    }
    for (uint64_t i = 0; i < nu; i++) {
        uint64_t proc = fle_ld(units + 16 * i, 8);
        int r = runnable(proc);
        void *fn;
        if (r < 0) bad++;
        if (r <= 0) continue;
        n_run++;
        rank++;
        fn = fn_of(proc);
        if (i < count) {
            started++;
            have_last = 1;
            last_idx = i;
            last_proc = proc;
            last_rank = rank;
            if (!fn || !trace_fn_seen(fn)) not_entered++;
        } else if (fn && trace_fn_seen(fn)) {
            beyond++;
        }
    }
    if (count >= 1 && count <= nu) at_count_proc = fle_ld(units + 16 * (count - 1), 8);
    if (have_last) {
        void *fn = fn_of(last_proc);
        in_progress = fn && trace_fn_on_stack(fn);
    }
    completed = started - (in_progress ? 1 : 0);
    printf("a4: init_count=%llu of %llu, last_unit_proc=0x%llx\n", ULL(count), ULL(nu), ULL(last_proc));
    printf("a4: last_unit index=%llu run_rank=%llu of %llu proc=0x%llx state=%s\n", ULL(last_idx), ULL(last_rank),
           ULL(n_run), ULL(last_proc), !have_last ? "none" : in_progress ? "in_progress" : "returned");
    printf("a4: runnable_started=%llu completed=%llu in_progress=%d of %llu runnable\n", ULL(started), ULL(completed),
           in_progress, ULL(n_run));
    printf("a4: unit[init_count-1] proc=0x%llx; units_outside_image=%d\n", ULL(at_count_proc), bad);
    printf("a4: entry_check started_but_not_entered=%llu runnable_entered_beyond_init_count=%llu\n", ULL(not_entered),
           ULL(beyond));
    printf("a4: all_units_done=%s functions_distinct=%llu function_entries=%llu\n",
           (count == nu && !in_progress) ? "yes" : "no", ULL(trace_distinct_count()),
           ULL(trace_call_count()));
}

/* Decision 0160, diagnostic only (link_b2d1.sh DIAG_MISS=1, with -Wl,--wrap=fle_win_try_thunk): an indirect call whose target is not a known function entry and not a
 * window-procedure thunk (the runtime's own answer comes first) is LOGGED ("b2d1-miss <va>") and skipped instead of stopping the run. It finds the seeds that a trimmed
 * module needs; the results of such a run are not results. */
#include <stdint.h>
#include <stdio.h>
int __real_fle_win_try_thunk(uint64_t va);
int __wrap_fle_win_try_thunk(uint64_t va) {
    if (__real_fle_win_try_thunk(va)) return 1;
    fprintf(stderr, "b2d1-miss %#llx\n", (unsigned long long)va);
    return 1;
}

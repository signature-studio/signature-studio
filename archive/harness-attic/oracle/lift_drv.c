/* A5 oracle, lift side: calls the generated C function for the same VA with the same
 * register file, memory and stack as native_drv. Linked against the all-pdata split build
 * (lift/out-pdata/parts, object files) and the runtime objects of that build. Usage: lift_drv CASES OUT PE */
#define _GNU_SOURCE
#include <stdarg.h>
#include <stdio.h>
#include <string.h>

#include "fle_dispatch.h"
#include "fle_runtime.h"
#include "fle_sse.h"
#include "oracle_common.h"

__attribute__((no_instrument_function)) void __cyg_profile_func_enter(void *fn, void *site) { (void)fn; (void)site; }
__attribute__((no_instrument_function)) void __cyg_profile_func_exit(void *fn, void *site) { (void)fn; (void)site; }

int __real_fputs(const char *s, FILE *f);
int __wrap_fputs(const char *s, FILE *f) {
    if (f == stderr) snprintf(oracle_last_msg, sizeof oracle_last_msg, "%s", s);
    return __real_fputs(s, f);
}

int __wrap_fprintf(FILE *f, const char *fmt, ...) {
    va_list ap;
    int r;
    if (f == stderr) {
        va_start(ap, fmt);
        vsnprintf(oracle_last_msg, sizeof oracle_last_msg, fmt, ap);
        va_end(ap);
    }
    va_start(ap, fmt);
    r = vfprintf(f, fmt, ap);
    va_end(ap);
    return r;
}

static void (*find_fn(uint64_t va))(void) {
    uint64_t lo = 0, hi = fle_dispatch_count;
    while (lo < hi) {
        uint64_t mid = lo + (hi - lo) / 2;
        if (fle_dispatch_table[mid].va < va) lo = mid + 1;
        else hi = mid;
    }
    return lo < fle_dispatch_count && fle_dispatch_table[lo].va == va ? fle_dispatch_table[lo].fn : NULL;
}

static int exec_lift(const oracle_case *c, const oracle_pe *pe, oracle_cpu *out) {
    void (*fn)(void) = find_fn(c->fn);
    (void)pe;
    if (!fn) return 1;
    for (int i = 0; i < 16; i++) R[i] = c->in.gpr[i];
    for (int i = 0; i < 16; i++) {
        X[i].u64[0] = c->in.xmm[i][0];
        X[i].u64[1] = c->in.xmm[i][1];
    }
    R[4] = OR_STACK_TOP - 8;
    fle_st(R[4], 8, OR_SENTINEL);
    fn();
    for (int i = 0; i < 16; i++) out->gpr[i] = R[i];
    out->gpr[4] = R[4] + 8; /* the lifted ret leaves R[4] at the return-address slot */
    for (int i = 0; i < 16; i++) {
        out->xmm[i][0] = X[i].u64[0];
        out->xmm[i][1] = X[i].u64[1];
    }
    return 0;
}

int main(int argc, char **argv) {
    return oracle_main(argc, argv, "lift", exec_lift);
}

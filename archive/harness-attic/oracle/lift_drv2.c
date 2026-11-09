/* A5 oracle v2, lift side: generated C for the same VA with the same register file, memory and
 * EFLAGS as native_drv2. Reports the runtime's FL (six status flags, FLE_UNDEF -> flundef mask),
 * fle_df, fle_mxcsr and fle_x87 (cw/sw/tw at the fnstenv offsets 0/4/8). Nothing in the lifter or
 * the runtime is changed. Usage: lift_drv2 CASES OUT PE */
#define _GNU_SOURCE
#include <stdarg.h>
#include <stdio.h>
#include <string.h>

#include "fle_dispatch.h"
#include "fle_mxcsr.h"
#include "fle_runtime.h"
#include "fle_sse.h"
#include "fle_x87ctl.h"
#include "oracle2_common.h"

extern uint8_t fle_df;

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

static const struct { uint8_t *f; unsigned bit; } flmap[] = {
    { &FL.cf, 0 }, { &FL.pf, 2 }, { &FL.af, 4 }, { &FL.zf, 6 }, { &FL.sf, 7 }, { &FL.of, 11 },
};

static int exec_lift(const oracle_case *c, const oracle_pe *pe, oracle_cpu *out) {
    void (*fn)(void) = find_fn(c->fn);
    uint64_t fl = 0, undef = 0;
    uint16_t w;
    (void)pe;
    if (!fn) return 1;
    for (int i = 0; i < 16; i++) R[i] = c->in.gpr[i];
    for (int i = 0; i < 16; i++) {
        X[i].u64[0] = c->in.xmm[i][0];
        X[i].u64[1] = c->in.xmm[i][1];
    }
    for (unsigned i = 0; i < sizeof flmap / sizeof flmap[0]; i++) *flmap[i].f = (c->in.rflags >> flmap[i].bit) & 1;
    fle_df = (uint8_t)((c->in.rflags >> 10) & 1);
    R[4] = OR_STACK_TOP - 8;
    fle_st(R[4], 8, OR_SENTINEL);
    fn();
    for (int i = 0; i < 16; i++) out->gpr[i] = R[i];
    out->gpr[4] = R[4] + 8;
    for (int i = 0; i < 16; i++) {
        out->xmm[i][0] = X[i].u64[0];
        out->xmm[i][1] = X[i].u64[1];
    }
    for (unsigned i = 0; i < sizeof flmap / sizeof flmap[0]; i++) {
        uint8_t v = *flmap[i].f;
        if (v == FLE_UNDEF) undef |= UINT64_C(1) << flmap[i].bit;
        else fl |= (uint64_t)(v & 1) << flmap[i].bit;
    }
    fl |= (uint64_t)(fle_df & 1) << 10;
    out->rflags = fl;
    out->flundef = undef;
    out->mxcsr = fle_mxcsr;
    memset(out->env, 0, sizeof out->env);
    w = fle_x87.cw;
    memcpy(out->env + 0, &w, 2);
    w = fle_x87.sw;
    memcpy(out->env + 4, &w, 2);
    w = fle_x87.tw;
    memcpy(out->env + 8, &w, 2);
    return 0;
}

int main(int argc, char **argv) {
    return oracle_main(argc, argv, "lift", exec_lift);
}

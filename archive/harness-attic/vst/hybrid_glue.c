/* Decision 0164 (the OTT pilot of the direct x86-64 -> wasm back end, hybrid): the entry point of the C path that code compiled by fe-wasmgen calls for an indirect call
 * (`c.fle_hyb_dispatch`). fle_dispatch is a static inline function of fle_dispatch.h; this makes it a real function that is exported. It is the same dispatch as the generated
 * C uses (landings, the census hook, the thunk check, the progress counter of the cooperative threads), so a call through it is the call the C path would make. */
#include <stdint.h>
#include "fle_dispatch.h"
#include "fle_sse.h"
#include "fle_vex.h"

__attribute__((used, noinline)) void fle_hyb_dispatch(uint64_t va) { fle_dispatch(va); }

/* `mov r64, gs:[off]` of compiled code: the C path does not read the TEB buffer, fle_gs_ld8 answers (and fills fle_teb[0x30], fle_teb[0x58] on the way) for the running guest thread. Reading
 * fle_teb directly (as fe-wasmgen does without a C runtime) gives 0 until the first read through the C path - found by ddmin on the OTT pilot (the TLS epoch of the thread-safe statics). */
__attribute__((used, noinline)) uint64_t fle_hyb_gs_ld8(uint64_t off) { return fle_gs_ld8(off); }

/* The slow path of the MXCSR window of the direct back end (decision 0164): code compiled by fe-wasmgen that finds an SSE operand outside the window (or MXCSR not in the
 * state the window needs) stores the operands in fle_hyb_buf[0] (dst) and [1] (src) and calls fle_hyb_mx(kind, imm); the result is read from fle_hyb_buf[2]. The functions called
 * are the ones the generated C calls for the same instruction, so the MXCSR flags, the NaN rules, denormals and rounding are the C path's own. kind = op * 2 + (double) for the
 * scalar arithmetic (op: 0 add, 1 sub, 2 mul, 3 div, 4 sqrt, 5 max, 6 min), then the numbers below. */
#include "fle_sse.h"
#include <stdio.h>
#include <stdlib.h>

fle_xmm fle_hyb_buf[4];
/* crossings between the compiled code and the C path: [0] compiled -> C function, [1] compiled -> C runtime (imports, dispatch), [2] C -> compiled function */
unsigned long long fle_hyb_cnt[4];

/* how often each kind of operation took the slow path (printed at exit when FLE_HYB_STATS is set) */
static unsigned long long fle_hyb_mx_n[40];
static void fle_hyb_mx_report(void)
{
    int k;
    unsigned long long t = 0;
    for (k = 0; k < 40; k++) t += fle_hyb_mx_n[k];
    fprintf(stderr, "hyb-cross: compiled->C function %llu, compiled->C runtime(import/dispatch) %llu, C->compiled %llu\n", fle_hyb_cnt[0], fle_hyb_cnt[1], fle_hyb_cnt[2]);
    fprintf(stderr, "hyb-mx slow path: total %llu\n", t);
    for (k = 0; k < 40; k++) if (fle_hyb_mx_n[k]) fprintf(stderr, "  kind %d: %llu\n", k, fle_hyb_mx_n[k]);
}
__attribute__((constructor)) static void fle_hyb_mx_init(void)
{
    if (getenv("FLE_HYB_STATS")) atexit(fle_hyb_mx_report);
}

__attribute__((used, noinline)) void fle_hyb_mx(int kind, int imm) {
    fle_xmm d = fle_hyb_buf[0], s = fle_hyb_buf[1], r = d;
    if (kind >= 0 && kind < 40) fle_hyb_mx_n[kind]++;
    { static int dbg; if (getenv("FLE_HYB_DBG") && dbg < 12) { dbg++; fprintf(stderr, "hyb-mx kind %d imm %d d=%016llx %016llx s=%016llx %016llx\n", kind, imm, (unsigned long long)d.u64[0], (unsigned long long)d.u64[1], (unsigned long long)s.u64[0], (unsigned long long)s.u64[1]); } }
    switch (kind) {
    case 0: r = fle_mx_addss(d, s); break;
    case 1: r = fle_mx_addsd(d, s); break;
    case 2: r = fle_mx_subss(d, s); break;
    case 3: r = fle_mx_subsd(d, s); break;
    case 4: r = fle_mx_mulss(d, s); break;
    case 5: r = fle_mx_mulsd(d, s); break;
    case 6: r = fle_mx_divss(d, s); break;
    case 7: r = fle_mx_divsd(d, s); break;
    case 8: r = fle_mx_sqrtss(d, s); break;
    case 9: r = fle_mx_sqrtsd(d, s); break;
    case 10: r = fle_mx_maxss(d, s); break;
    case 11: r = fle_mx_maxsd(d, s); break;
    case 12: r = fle_mx_minss(d, s); break;
    case 13: r = fle_mx_minsd(d, s); break;
    case 14: r = fle_mx_cvtss2sd(d, s); break;
    case 15: r = fle_mx_cvtsd2ss(d, s); break;
    case 16: r = fle_mx_cvtps2pd(d, s); break;
    case 17: r = fle_mx_cvtpd2ps(d, s); break;
    case 18: r = fle_mx_cvtdq2pd(d, s); break;
    case 19: r = fle_mx_cvttpd2dq(d, s); break;
    case 20: r = FLE_MX_CMPSD(d, s, (unsigned)imm); break;
    case 21: r.u64[0] = fle_mx_cvttsd2si32(s); break;
    case 22: r.u64[0] = fle_mx_cvttsd2si64(s); break;
    case 23: r.u64[0] = fle_mx_cvttss2si32(s); break;
    case 24: r.u64[0] = fle_mx_cvttss2si64(s); break;
    case 25: fle_mx_ucomisd(d, s); break;
    case 26: fle_mx_comisd(d, s); break;
    case 27: fle_mx_ucomiss(d, s); break;
    case 28: fle_mx_comiss(d, s); break;
    case 29: r = fle_mx_cvtsi2sd32(d, (int32_t)s.u32[0]); break;
    case 30: r = fle_mx_cvtsi2sd64(d, (int64_t)s.u64[0]); break;
    case 31: r = fle_mx_sqrtpd(d, s); break;
    case 32: r = fle_mx_cvtdq2ps(d, s); break;
    default: __builtin_trap();
    }
    fle_hyb_buf[2] = r;
}

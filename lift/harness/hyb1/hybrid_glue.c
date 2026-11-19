/* Decision 0164 (the OTT pilot of the direct x86-64 -> wasm back end, hybrid): the entry point of the C path that code compiled by fe-wasmgen calls for an indirect call
 * (`c.fle_hyb_dispatch`). fle_dispatch is a static inline function of fle_dispatch.h; this makes it a real function that is exported. It is the same dispatch as the generated
 * C uses (landings, the census hook, the thunk check, the progress counter of the cooperative threads), so a call through it is the call the C path would make. */
#include <stdint.h>
#include "fle_test_env.h"
#include "fle_dispatch.h"
#include "fle_sse.h"
#include "fle_vex.h"

__attribute__((used, noinline)) void fle_hyb_dispatch(uint64_t va) { fle_dispatch(va); }

/* Decision 0169 (inline dispatch, opt-in): the compiled code does the lookup of fle_dispatch itself and calls this when the progress count reaches the limit (the line of fle_dispatch:
 * `if (++fle_progress_count >= fle_progress_limit && fle_progress_slice) fle_progress_slice();`; the increment was done by the compiled code). */
__attribute__((used, noinline)) void fle_hyb_slice(void) { if (fle_progress_slice) fle_progress_slice(); }

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
static unsigned long long fle_hyb_mx_n[128];
static void fle_hyb_mx_report(void)
{
    int k;
    unsigned long long t = 0;
    for (k = 0; k < 128; k++) t += fle_hyb_mx_n[k];
    fprintf(stderr, "hyb-cross: compiled->C function %llu, compiled->C runtime(import/dispatch) %llu, C->compiled %llu\n", fle_hyb_cnt[0], fle_hyb_cnt[1], fle_hyb_cnt[2]);
    fprintf(stderr, "hyb-mx slow path: total %llu\n", t);
    for (k = 0; k < 128; k++) if (fle_hyb_mx_n[k]) fprintf(stderr, "  kind %d: %llu\n", k, fle_hyb_mx_n[k]);
}
__attribute__((constructor)) static void fle_hyb_mx_init(void)
{
    if (FLE_TEST_GETENV("FLE_HYB_STATS")) atexit(fle_hyb_mx_report);
}

__attribute__((used, noinline)) void fle_hyb_mx(int kind, int imm) {
    fle_xmm d = fle_hyb_buf[0], s = fle_hyb_buf[1], r = d;
    if (kind >= 0 && kind < 128) fle_hyb_mx_n[kind]++;
    { static int dbg; if (FLE_TEST_GETENV("FLE_HYB_DBG") && dbg < 12) { dbg++; fprintf(stderr, "hyb-mx kind %d imm %d d=%016llx %016llx s=%016llx %016llx\n", kind, imm, (unsigned long long)d.u64[0], (unsigned long long)d.u64[1], (unsigned long long)s.u64[0], (unsigned long long)s.u64[1]); } }
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
    case 33: r = fle_mx_cvtsi2ss32(d, (int32_t)s.u32[0]); break;   /* decision 0169 */
    case 34: r = fle_mx_cvtsi2ss64(d, (int64_t)s.u64[0]); break;
    case 35: r.u64[0] = fle_mx_cvtsd2si32(s); break;   /* decision 0169: the rounding conversions */
    case 36: r.u64[0] = fle_mx_cvtsd2si64(s); break;
    case 37: r.u64[0] = fle_mx_cvtss2si32(s); break;
    case 38: r.u64[0] = fle_mx_cvtss2si64(s); break;
    /* decision 0181 (fe-wasmgen --mx-packed): the packed arithmetic outside its window or guard */
    case 40: r = fle_mx_addps(d, s); break;
    case 41: r = fle_mx_addpd(d, s); break;
    case 42: r = fle_mx_subps(d, s); break;
    case 43: r = fle_mx_subpd(d, s); break;
    case 44: r = fle_mx_mulps(d, s); break;
    case 45: r = fle_mx_mulpd(d, s); break;
    case 46: r = fle_mx_divps(d, s); break;
    case 47: r = fle_mx_divpd(d, s); break;
    case 48: r = fle_mx_sqrtps(d, s); break;
    case 49: r = fle_mx_maxps(d, s); break;
    case 50: r = fle_mx_maxpd(d, s); break;
    case 51: r = fle_mx_minps(d, s); break;
    case 52: r = fle_mx_minpd(d, s); break;
    case 53: r = FLE_MX_CMPPS(d, s, (unsigned)imm); break;
    case 54: r = FLE_MX_CMPPD(d, s, (unsigned)imm); break;
    case 55: r = FLE_MX_CMPSS(d, s, (unsigned)imm); break;
    case 56: r = fle_mx_cvttps2dq(d, s); break;
    case 57: r = FLE_MX_ROUNDSS(d, s, (unsigned)imm); break;
    case 58: r = FLE_MX_ROUNDSD(d, s, (unsigned)imm); break;
    case 59: r = fle_mx_cvtps2dq(d, s); break;   /* decision 0252 */
    case 60: r = fle_mx_cvtpd2dq(d, s); break;   /* decision 0252 */
    case 61: r = fle_lg_rsqrtps(d, s); break;    /* decision 0252: the approximations of the hardware (tables) and the packed rounding */
    case 62: r = d; r.u32[0] = fle_lg_rsqrt_f32(s.u32[0]); break;
    case 63: r = fle_i2_rcpps(d, s); break;
    case 64: r = FLE_MX_ROUNDPS(d, s, (unsigned)imm); break;
    case 65: r = FLE_MX_ROUNDPD(d, s, (unsigned)imm); break;
    default: __builtin_trap();
    }
    fle_hyb_buf[2] = r;
}


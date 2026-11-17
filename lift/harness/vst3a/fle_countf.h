/* The opt-in per-function instruction counter of a lifted module (decision 0165). count_fn.py turns the generated units into units that call FLE_CNTF(fid, n, s, p, x, v) once at the
 * start of every basic block (and FLE_CNTCALL(fid) at the entry of every function): n = original instructions of the block, s = scalar SSE floating-point ones, p = other SSE ones
 * (a line that uses X[]), x = x87 ones (mnemonic f*), v = VEX-encoded ones (mnemonic v*; AVX/AVX2/FMA). The rest is integer. The generated C of the default build is not changed:
 * the instrumented units are separate files; without -DFLE_COUNT the macros are empty. rep-prefixed string instructions count as one per execution. */
#ifndef FLE_COUNTF_H
#define FLE_COUNTF_H
#include <stdint.h>
extern uint64_t fle_icount, fle_icount_s, fle_icount_p, fle_icount_x, fle_icount_v;
extern uint64_t fle_fn_n[], fle_fn_s[], fle_fn_p[], fle_fn_x[], fle_fn_v[], fle_fn_calls[];
#ifdef FLE_COUNT
#define FLE_CNTF(fid, n, s, p, x, v) (fle_icount += (n), fle_icount_s += (s), fle_icount_p += (p), fle_icount_x += (x), fle_icount_v += (v), \
    fle_fn_n[fid] += (n), fle_fn_s[fid] += (s), fle_fn_p[fid] += (p), fle_fn_x[fid] += (x), fle_fn_v[fid] += (v))
#define FLE_CNTCALL(fid) (fle_fn_calls[fid]++)
#else
#define FLE_CNTF(fid, n, s, p, x, v) ((void)0)
#define FLE_CNTCALL(fid) ((void)0)
#endif
#endif

/* The opt-in instruction counter of a lifted module (decision 0156). count_instrument.py turns the generated units into units that call FLE_CNT(n, s, p) once at the start of
 * every basic block: n = the number of original instructions of the block, s = how many of them are scalar SSE floating-point instructions (addsd, mulss, cvtsi2sd, ...), p = how many
 * are other SSE instructions (packed arithmetic, packed moves, SSE integer). The generated C of the default build is not changed: the instrumented copies are separate files, and
 * without -DFLE_COUNT the macro is empty. The counters are defined in main_vst.c. rep-prefixed string instructions count as one instruction per execution of the instruction
 * (not per iteration); a basic block that is left early by something other than a branch (a longjmp, a stop) is counted whole. */
#ifndef FLE_COUNT_H
#define FLE_COUNT_H
#include <stdint.h>
extern uint64_t fle_icount, fle_icount_s, fle_icount_p;
#ifdef FLE_COUNT
#define FLE_CNT(n, s, p) (fle_icount += (n), fle_icount_s += (s), fle_icount_p += (p))
#else
#define FLE_CNT(n, s, p) ((void)0)
#endif
#endif

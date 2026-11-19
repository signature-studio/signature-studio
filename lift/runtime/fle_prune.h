/* Pruned functions (decision 0168, provisional). The generated C includes this header only when it was made with `fe-lift --prune-list`: a function that the
 * environment cannot reach is replaced, where kept code still names it, by a stub `void f_<va>(void) { fle_pruned(UINT64_C(<va>)); }`. If the run ever gets
 * there, the judgement was wrong for this run: stop by name, with the VA, and never go on with a made-up value (as fle_dispatch_miss and fle_jumptab_miss).
 * For the list of tools/prune/judge.py the usual cause is that the IPP CPU selector (dsp_ippv2 variables at RVA 0x2541000, 0x25410a0, 0x254f270, 0x255d050)
 * is no longer -1/0: the CPUID answer (fle_cpuid.c, decision 0118) was changed, or the guest called ippSetCpuFeatures. */
#ifndef FLE_PRUNE_H
#define FLE_PRUNE_H

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

static inline __attribute__((noreturn, noinline, cold)) void fle_pruned(uint64_t va) {
    fprintf(stderr, "fle: pruned function %#llx was executed (prune list, decision 0168; check the CPUID answer of fle_cpuid.c and calls of ippSetCpuFeatures)\n", (unsigned long long)va);
    abort();
}

#endif

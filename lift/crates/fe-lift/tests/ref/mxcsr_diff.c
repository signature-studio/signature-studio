/* Pure-C runtime/fle_mxcsr_flags.h versus the host reference (tests/ref/fle_mxcsr_flags_host.h).
 * Build: clang -O0 -std=gnu11 -ffp-contract=off -Iruntime -Itests/ref tests/ref/mxcsr_diff.c
 * Run: ./a.out [cases-per-op, default 100000]. Exit status 0 only if there is no mismatch. */
#include "fle_runtime.h"
#include "fle_sse.h" /* includes the portable fle_mxcsr_flags.h */
#include "fle_mxcsr_flags_host.h"

#include "mxcsr_diff_ops.h"
#include "mxcsr_diff_main.c"

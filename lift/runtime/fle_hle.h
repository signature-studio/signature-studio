/* HLE: host implementations of lifted guest functions (decision 0162, PROVISIONAL; the mechanism is decision 0151's reference implementation).
 *
 * `fe-lift lift ... --hle-entries FILE` puts a guard in front of the first instruction of each listed function:
 *     { extern int fle_hle_<name>(void); if (fle_hle_<name>()) return; }
 * fle_hle_<name>() returns 1 when the host did everything the function does - the same registers (R, X), flags (FL) and memory as the lifted body
 * leaves, so the function just returns - and 0 when it does not handle this call; it has not touched anything then and the lifted body runs.
 * Without --hle-entries the generated C has no guard, and this file's code is never called.
 *
 * The run has one mode: the host implementation runs where its conditions hold, the lifted body elsewhere. The statistics go to stderr at exit (native: also on SIGABRT).
 *
 * The one implementation: the guest function at 0x33d6e70 of dsp_ippv2_x64.dll rebased to 0x2000000 (f_33d6e70), the inner loop of Intel IPP's
 * ippiWarpAffineQuad_8u_C4R (docs/decisions/0151-hle-first-step.md section W, docs/decisions/0162-dsp-hot-function-hle.md). */
#ifndef FLE_HLE_H
#define FLE_HLE_H

#include <stdint.h>

/* Counters of one guard (calls that reached it; handled by the host; given back to the lifted body). */
typedef struct {
    uint64_t calls, handled, fallback;
    uint64_t rows, pixels;
} fle_hle_stats;

/* f_33d6e70: a bilinear affine warp of a 4-channel byte image into rows of a destination, in double precision. The guard, and the same function
 * without the counters (for the tests; it answers 0 when the plan does not hold). */
int fle_hle_warp_bilin(void);
int fle_hle_warp_bilin_run(void);
const fle_hle_stats *fle_hle_warp_bilin_stats(void);

#endif

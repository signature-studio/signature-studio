/* EXPLORATION ONLY (decision 0188): print the seed of every MT19937 init_genrand(state, seed) call of the engine (f_981250: rcx = state, edx = seed), to see whether the dithering generator is seeded with a constant
 * or with something taken from the clock. Link with -Wl,--wrap=f_981250. Not part of any shipped build. */
#include <stdint.h>
#include <stdio.h>

#include "fle_runtime.h"

extern void __real_f_981250(void);
void __wrap_f_981250(void)
{
    fprintf(stderr, "mt-seed: init_genrand(state=%#llx, seed=%#x)\n", (unsigned long long)R[1], (unsigned)(uint32_t)R[2]);
    __real_f_981250();
}

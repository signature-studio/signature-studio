/* Test-only (decision 0047): the lifted RCPPS helper against the host RCPPS for all 2^32 float bit patterns, four lanes
 * at a time (fle_i2_rcpps from runtime/fle_sse.h against the host instruction, inline asm). Prints
 * "rcpps_exhaustive inputs <n> mismatches <m>" and the first mismatches. Build: clang -O2 -I runtime. */
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_sse.h"

uint64_t R[16];
fle_eflags FL;
fle_xmm X[16];

static inline void host_rcpps(const uint32_t in[4], uint32_t out[4]) {
    __asm__ volatile("movdqu (%1), %%xmm0\n\trcpps %%xmm0, %%xmm0\n\tmovdqu %%xmm0, (%0)" ::"r"(out), "r"(in) : "xmm0", "memory");
}

int main(void) {
    uint64_t bad = 0, n = 0;
    for (uint64_t b = 0; b < (1ull << 32); b += 4) {
        uint32_t in[4] = {(uint32_t)b, (uint32_t)(b + 1), (uint32_t)(b + 2), (uint32_t)(b + 3)};
        uint32_t h[4];
        fle_xmm s, z, r;
        host_rcpps(in, h);
        memcpy(&s, in, 16);
        memset(&z, 0xa5, 16);
        r = fle_i2_rcpps(z, s);
        for (int k = 0; k < 4; k++) {
            n++;
            if (r.u32[k] != h[k]) {
                if (bad < 8) printf("mismatch x=%#x host=%#x lifted=%#x\n", in[k], h[k], r.u32[k]);
                bad++;
            }
        }
    }
    printf("rcpps_exhaustive inputs %llu mismatches %llu\n", (unsigned long long)n, (unsigned long long)bad);
    return bad != 0;
}

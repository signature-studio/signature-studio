/* Test-only (decision 0075): the lifted RSQRTPS helper (fle_lg_rsqrtps from runtime/fle_sse.h) against the host RSQRTPS for
 * all 2^32 float bit patterns, four lanes at a time, once per MXCSR value given on the command line (hex; default
 * 0x1f80). The host MXCSR is set with ldmxcsr before the run and restored after; the status flags (bits 5:0) are
 * cleared before each block and checked to stay 0 (the SDM: SIMD exceptions "None"). Prints
 * "rsqrtps_exhaustive mxcsr <m> inputs <n> mismatches <k> flag_changes <f>" per MXCSR value.
 * Decision 0149: the environment variables RSQRT_FROM and RSQRT_TO (multiples of 4, 0 <= FROM <= TO <= 2^32, decimal or 0x hex) restrict the run to the
 * bit patterns FROM <= x < TO, so that the test can run slices of the 2^32 range in parallel; the line then reports the slice (inputs = TO - FROM). Without
 * them the program checks all 2^32 patterns as before.
 * Build: clang -O2 -I runtime. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <xmmintrin.h>

#include "fle_runtime.h"
#include "fle_sse.h"

uint64_t R[16];
fle_eflags FL;
fle_xmm X[16];

static inline void host_rsqrtps(const uint32_t in[4], uint32_t out[4]) {
    __asm__ volatile("movdqu (%1), %%xmm0\n\trsqrtps %%xmm0, %%xmm0\n\tmovdqu %%xmm0, (%0)" ::"r"(out), "r"(in) : "xmm0", "memory");
}

int main(int argc, char **argv) {
    int a;
    uint64_t from = 0, to = 1ull << 32;
    const char *env;
    unsigned saved = _mm_getcsr();
    if ((env = getenv("RSQRT_FROM")) != 0) from = strtoull(env, 0, 0);
    if ((env = getenv("RSQRT_TO")) != 0) to = strtoull(env, 0, 0);
    if (from % 4 != 0 || to % 4 != 0 || from > to || to > (1ull << 32)) {
        fprintf(stderr, "rsqrt_exhaustive: bad RSQRT_FROM/RSQRT_TO\n");
        return 2;
    }
    if (argc < 2) {
        static char dflt[] = "1f80";
        static char *av[] = {0, dflt};
        argc = 2;
        argv = av;
    }
    for (a = 1; a < argc; a++) {
        unsigned mx = (unsigned)strtoul(argv[a], 0, 16);
        uint64_t bad = 0, n = 0, flagbad = 0, b;
        for (b = from; b < to; b += 4) {
            uint32_t in[4] = {(uint32_t)b, (uint32_t)(b + 1), (uint32_t)(b + 2), (uint32_t)(b + 3)};
            uint32_t h[4];
            fle_xmm s, z, r;
            int k;
            _mm_setcsr(mx & ~0x3fu);
            host_rsqrtps(in, h);
            if ((_mm_getcsr() & 0x3fu) != 0) flagbad++;
            _mm_setcsr(saved);
            memcpy(&s, in, 16);
            memset(&z, 0xa5, 16);
            r = fle_lg_rsqrtps(z, s);
            for (k = 0; k < 4; k++) {
                n++;
                if (r.u32[k] != h[k]) {
                    if (bad < 8) printf("mismatch mxcsr %#x x=%#x host=%#x lifted=%#x\n", mx, in[k], h[k], r.u32[k]);
                    bad++;
                }
            }
        }
        printf("rsqrtps_exhaustive mxcsr %#x inputs %llu mismatches %llu flag_changes %llu\n", mx, (unsigned long long)n,
               (unsigned long long)bad, (unsigned long long)flagbad);
        if (bad || flagbad) return 1;
    }
    return 0;
}

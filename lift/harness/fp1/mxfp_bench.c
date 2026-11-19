/* Micro-benchmark of the fast path (decision 0143): the inner loop of the interpolation function f_351b5a0 of the
 * engine (byte -> double, multiply by a weight, add, compare with 0 and 255, +0.5, truncate), in the lifted form
 * (X[] registers, fle_mx_* wrappers). Built twice: with and without -DFLE_MX_NOFAST (the integer path only).
 * usage: mxfp_bench [iterations] */
#include <stdio.h>
#include <stdlib.h>
#include <time.h>

#include "fle_runtime.h"
#include "fle_sse.h"

uint64_t R[16];
fle_eflags FL;
fle_xmm X[16];

static double now(void) {
    struct timespec t;
    clock_gettime(CLOCK_MONOTONIC, &t);
    return (double)t.tv_sec + 1e-9 * (double)t.tv_nsec;
}

int main(int argc, char **argv) {
    unsigned long n = argc > 1 ? strtoul(argv[1], 0, 0) : 20000000ul, i;
    uint8_t px[4096];
    double w[16];
    unsigned long long sum = 0;
    double t0, t1;
    uint64_t seed = 88172645463325252ull;
    for (i = 0; i < sizeof px; i++) {
        seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17;
        px[i] = (seed >> 33) % 7 == 0 ? 0 : (uint8_t)(seed >> 20); /* many zeros as in an image with a transparent background */
    }
    for (i = 0; i < 16; i++) {
        seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17;
        w[i] = (double)(seed >> 11) / 9007199254740992.0 * 1.5 - 0.25; /* weights in -0.25 .. 1.25 */
    }
    X[0].f64[0] = 0.0;
    X[13].f64[0] = 0.0;
    X[7].f64[0] = 255.0;
    X[9].f64[0] = 0.5;
    X[6].f64[0] = 0.8;
    for (i = 0; i < 16; i++) { X[2 + (i & 3)].f64[0] = w[i]; }
    t0 = now();
    for (i = 0; i < n; i++) {
        unsigned k = (unsigned)(i * 4) & 4095u;
        uint32_t b0 = px[k], b1 = px[k + 1], b2 = px[k + 2], b3 = px[k + 3];
        X[14].u64[0] = 0; X[10].u64[0] = 0; X[15].u64[0] = 0; X[12].u64[0] = 0;
        X[14] = fle_mx_cvtsi2sd32(X[14], (int32_t)b0);
        X[10] = fle_mx_cvtsi2sd32(X[10], (int32_t)b1);
        X[15] = fle_mx_cvtsi2sd32(X[15], (int32_t)b2);
        X[12] = fle_mx_cvtsi2sd32(X[12], (int32_t)b3);
        X[14] = fle_mx_mulsd(X[14], X[2]);
        X[10] = fle_mx_mulsd(X[10], X[3]);
        X[15] = fle_mx_mulsd(X[15], X[4]);
        X[12] = fle_mx_mulsd(X[12], X[5]);
        X[14] = fle_mx_addsd(X[14], X[10]);
        X[14] = fle_mx_addsd(X[14], X[15]);
        X[14] = fle_mx_addsd(X[14], X[12]);
        X[14] = fle_mx_mulsd(X[14], X[6]);
        fle_mx_comisd(X[14], X[7]);
        if (FL.cf == 0 && FL.zf == 0) {
            sum += 255;
        } else {
            fle_mx_comisd(X[13], X[14]);
            if (FL.cf || FL.zf) {
                X[14] = fle_mx_addsd(X[14], X[9]);
                sum += (uint32_t)fle_mx_cvttsd2si32(X[14]);
            }
        }
        X[2].f64[0] += 1e-9; /* keep the weights slightly changing */
    }
    t1 = now();
    printf("%lu iterations, %.1f ns per iteration (12 fle_mx instructions + 2 comparisons), checksum %llu mxcsr %#x\n", n,
           (t1 - t0) * 1e9 / (double)n, sum, (unsigned)fle_mxcsr);
    return 0;
}

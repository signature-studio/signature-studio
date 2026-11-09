/* anim1 (decision 0225): differential test of the packed-integer SSE helpers of lift/runtime/fle_sse.h, fle_sse_int2.h and fle_sse_more.c.
 * The same file is built twice: natively by gcc (the lane loops: the reference) and by emcc -msimd128 (the WebAssembly SIMD path). Both print one FNV-1a hash per operation over
 * the same pseudo-random inputs (a byte is 0x00, 0x7f, 0x80, 0xff or random, so that every saturation edge is hit often); the two outputs must be identical.
 * usage: sse_simd_difftest [iterations=1500000] */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "fle_sse.h"
#include "fle_sse_more.h"

fle_xmm X_[1];
static uint64_t st = 0x9E3779B97F4A7C15ull;
static uint64_t rnd(void) { st ^= st << 13; st ^= st >> 7; st ^= st << 17; return st; }
static fle_xmm rx(void) {
    fle_xmm r; uint8_t b[16];
    for (int i = 0; i < 16; i++) { uint64_t v = rnd(); switch (v & 7) { case 0: b[i] = 0x00; break; case 1: b[i] = 0x7f; break; case 2: b[i] = 0x80; break; case 3: b[i] = 0xff; break; default: b[i] = (uint8_t)(v >> 8); } }
    memcpy(&r, b, 16);
    return r;
}
static uint64_t h;
static void mix(fle_xmm r) { const uint8_t *p = (const uint8_t *)&r; for (int i = 0; i < 16; i++) { h ^= p[i]; h *= 0x100000001b3ull; } }
#define OP2(NAME) { h = 0xcbf29ce484222325ull; st = 0x9E3779B97F4A7C15ull; for (long i = 0; i < n; i++) { fle_xmm a = rx(), b = rx(); mix(NAME(a, b)); } printf("%-18s %016llx\n", #NAME, (unsigned long long)h); }
#define OPS(NAME) { h = 0xcbf29ce484222325ull; st = 0x9E3779B97F4A7C15ull; for (long i = 0; i < n; i++) { fle_xmm a = rx(); uint64_t c = (i % 5 == 0) ? rnd() : (rnd() % 80); mix(NAME(a, c)); } printf("%-18s %016llx\n", #NAME, (unsigned long long)h); }
#define OPU(ESZ, HI) { h = 0xcbf29ce484222325ull; st = 0x9E3779B97F4A7C15ull; for (long i = 0; i < n; i++) { fle_xmm a = rx(), b = rx(); mix(fle_unpack(a, b, ESZ, HI)); } printf("fle_unpack_%d_%d       %016llx\n", ESZ, HI, (unsigned long long)h); }
int main(int argc, char **argv) {
    long n = argc > 1 ? atol(argv[1]) : 1500000;
#ifdef FLE_WASM_SIMD
    printf("# path: wasm simd128\n");
#else
    printf("# path: lane loops\n");
#endif
    OP2(fle_paddd) OP2(fle_psubd) OP2(fle_pcmpeqd) OP2(fle_pmulhuw) OP2(fle_packuswb) OP2(fle_paddw)
    OPU(1, 0) OPU(1, 1) OPU(2, 0) OPU(2, 1) OPU(4, 0) OPU(4, 1) OPU(8, 0) OPU(8, 1)
    OP2(fle_i2_paddsb) OP2(fle_i2_paddsw) OP2(fle_i2_psubsb) OP2(fle_i2_psubsw) OP2(fle_i2_paddusb) OP2(fle_i2_paddusw) OP2(fle_i2_psubusb) OP2(fle_i2_psubusw)
    OP2(fle_i2_pmaxsw) OP2(fle_i2_pminsw) OP2(fle_i2_pmulhw) OP2(fle_i2_pmullw) OP2(fle_i2_psubw) OP2(fle_i2_pcmpgtd) OP2(fle_i2_paddq) OP2(fle_i2_packsswb) OP2(fle_i2_packssdw)
    OP2(fle_lg_pcmpeqb) OP2(fle_lg_pcmpeqw) OP2(fle_lg_pcmpgtb) OP2(fle_lg_pcmpgtw) OP2(fle_lg_paddb) OP2(fle_lg_psubb) OP2(fle_lg_pmaxub) OP2(fle_lg_pminub) OP2(fle_lg_psubq) OP2(fle_lg_pmaddwd)
    OPS(fle_i2_psllw) OPS(fle_i2_pslld) OPS(fle_i2_psllq) OPS(fle_i2_psrlw) OPS(fle_i2_psrld) OPS(fle_i2_psrlq) OPS(fle_i2_psraw) OPS(fle_i2_psrad)
    return 0;
}

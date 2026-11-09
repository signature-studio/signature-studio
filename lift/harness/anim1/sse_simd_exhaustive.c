/* anim1 (decision 0225): the EXHAUSTIVE part of the differential test of the packed-integer SSE helpers (see sse_simd_difftest.c for the random part).
 * Built twice (gcc natively = lane loops; emcc -msimd128 = WebAssembly SIMD); both print one hash per operation; the outputs must be identical.
 *   - every 8-bit lane operation: all 65,536 (p, q) byte pairs (a = p in all lanes, b = q .. q+15 over the lanes, 16 values of q per call),
 *   - every 16-bit lane operation: all 2^32 (p, q) word pairs (8 values of q per call; 536,870,912 calls per operation),
 *   - the word packs: all 65,536 words as the signed source; PACKSSDW: 2^26 doublewords strided plus every doubleword within 40 of 0 / +-0x8000 / +-0x7fff / +-0x80000000,
 *   - PMADDWD: every combination of 27 boundary words in the four operand lanes (27^4), 32-bit lane operations and 64-bit lanes: every combination of boundary values,
 *   - the shifts: every word (all 65,536) with every count 0..130 and counts 2^32, 2^32+3, 2^63, ~0; doublewords / quadwords: boundary values x the same counts.
 * usage: sse_simd_exhaustive [quick]   (quick: the 16-bit pair loops use every 16th p, for a smoke test) */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "fle_sse.h"
#include "fle_sse_more.h"

static uint64_t h;
static inline void mix(fle_xmm r) { h = (h ^ r.u64[0]) * 0x100000001b3ull; h = (h ^ (r.u64[1] + 0x9E3779B97F4A7C15ull)) * 0x100000001b3ull; h ^= h >> 29; }
#define BEGIN h = 0xcbf29ce484222325ull
#define END(name) printf("%-22s %016llx\n", name, (unsigned long long)h)
static long pstep = 1;

#define OP8(NAME) { BEGIN; for (int p = 0; p < 256; p++) for (int q0 = 0; q0 < 256; q0 += 16) { fle_xmm a, b; uint8_t pa[16], pb[16]; for (int i = 0; i < 16; i++) { pa[i] = (uint8_t)p; pb[i] = (uint8_t)(q0 + i); } memcpy(&a, pa, 16); memcpy(&b, pb, 16); mix(NAME(a, b)); } END(#NAME); }
#define OP16(NAME) { BEGIN; for (long p = 0; p < 65536; p += pstep) for (int q0 = 0; q0 < 65536; q0 += 8) { fle_xmm a, b; uint16_t pa[8], pb[8]; for (int i = 0; i < 8; i++) { pa[i] = (uint16_t)p; pb[i] = (uint16_t)(q0 + i); } memcpy(&a, pa, 16); memcpy(&b, pb, 16); mix(NAME(a, b)); } END(#NAME); }
static const uint64_t B64[] = { 0, 1, 2, 3, 0x7f, 0x80, 0xff, 0x100, 0x7fff, 0x8000, 0x8001, 0xffff, 0x10000, 0x7ffffffe, 0x7fffffff, 0x80000000ull, 0x80000001ull, 0xfffffffeull, 0xffffffffull,
                                0x100000000ull, 0x7fffffffffffffffull, 0x8000000000000000ull, 0x8000000000000001ull, 0xfffffffffffffffeull, 0xffffffffffffffffull, 0x5555555555555555ull, 0xaaaaaaaaaaaaaaaaull };
#define NB ((int)(sizeof B64 / sizeof B64[0]))
#define OPQ(NAME) { BEGIN; for (int i = 0; i < NB; i++) for (int j = 0; j < NB; j++) for (int k = 0; k < NB; k++) for (int l = 0; l < NB; l += 1) { fle_xmm a, b; a.u64[0] = B64[i]; a.u64[1] = B64[j]; b.u64[0] = B64[k]; b.u64[1] = B64[l]; mix(NAME(a, b)); } END(#NAME); }
#define OPD(NAME) { BEGIN; for (int i = 0; i < NB; i++) for (int j = 0; j < NB; j++) for (int k = 0; k < NB; k++) for (int l = 0; l < NB; l++) { fle_xmm a, b; a.u32[0] = (uint32_t)B64[i]; a.u32[1] = (uint32_t)B64[j]; a.u32[2] = (uint32_t)B64[k]; a.u32[3] = (uint32_t)B64[l]; b.u32[0] = (uint32_t)B64[l]; b.u32[1] = (uint32_t)B64[i]; b.u32[2] = (uint32_t)B64[j]; b.u32[3] = (uint32_t)B64[k]; mix(NAME(a, b)); } END(#NAME); }
static const uint64_t CNT[] = { 0, 1, 2, 7, 8, 14, 15, 16, 17, 30, 31, 32, 33, 62, 63, 64, 65, 100, 130, 0x100000000ull, 0x100000003ull, 0x8000000000000000ull, 0xffffffffffffffffull };
#define NC ((int)(sizeof CNT / sizeof CNT[0]))
#define SHW(NAME) { BEGIN; for (long p = 0; p < 65536; p += pstep) for (int c = 0; c < NC + 131; c++) { uint64_t cnt = c < 131 ? (uint64_t)c : CNT[c - 131]; fle_xmm a; uint16_t pa[8]; for (int i = 0; i < 8; i++) pa[i] = (uint16_t)(p ^ (i * 0x1111)); memcpy(&a, pa, 16); mix(NAME(a, cnt)); } END(#NAME); }
#define SHD(NAME) { BEGIN; for (int i = 0; i < NB; i++) for (int j = 0; j < NB; j++) for (int c = 0; c < NC + 131; c++) { uint64_t cnt = c < 131 ? (uint64_t)c : CNT[c - 131]; fle_xmm a; a.u32[0] = (uint32_t)B64[i]; a.u32[1] = (uint32_t)B64[j]; a.u32[2] = (uint32_t)B64[j] * 3u; a.u32[3] = (uint32_t)B64[i] ^ 0x80000000u; mix(NAME(a, cnt)); } END(#NAME); }
#define SHQ(NAME) { BEGIN; for (int i = 0; i < NB; i++) for (int j = 0; j < NB; j++) for (int c = 0; c < NC + 131; c++) { uint64_t cnt = c < 131 ? (uint64_t)c : CNT[c - 131]; fle_xmm a; a.u64[0] = B64[i]; a.u64[1] = B64[j]; mix(NAME(a, cnt)); } END(#NAME); }

int main(int argc, char **argv) {
    if (argc > 1 && !strcmp(argv[1], "quick")) pstep = 16;
#ifdef FLE_WASM_SIMD
    printf("# path: wasm simd128\n");
#else
    printf("# path: lane loops\n");
#endif
    OP8(fle_i2_paddsb) OP8(fle_i2_psubsb) OP8(fle_i2_paddusb) OP8(fle_i2_psubusb) OP8(fle_lg_pcmpeqb) OP8(fle_lg_pcmpgtb) OP8(fle_lg_paddb) OP8(fle_lg_psubb) OP8(fle_lg_pmaxub) OP8(fle_lg_pminub)
    OP16(fle_i2_paddsw) OP16(fle_i2_psubsw) OP16(fle_i2_paddusw) OP16(fle_i2_psubusw) OP16(fle_i2_pmaxsw) OP16(fle_i2_pminsw) OP16(fle_i2_pmulhw) OP16(fle_i2_pmullw) OP16(fle_i2_psubw)
    OP16(fle_pmulhuw) OP16(fle_paddw) OP16(fle_lg_pcmpeqw) OP16(fle_lg_pcmpgtw)
    /* packs: all 65,536 words as the signed source (a = the word in all lanes, b = the next 8 words) */
    { BEGIN; for (int p = 0; p < 65536; p++) for (int q0 = 0; q0 < 65536; q0 += 8 * 256) { fle_xmm a, b; uint16_t pa[8], pb[8]; for (int i = 0; i < 8; i++) { pa[i] = (uint16_t)p; pb[i] = (uint16_t)(q0 + i * 256 + (p & 255)); } memcpy(&a, pa, 16); memcpy(&b, pb, 16); mix(fle_packuswb(a, b)); } END("fle_packuswb"); }
    { BEGIN; for (int p = 0; p < 65536; p++) for (int q0 = 0; q0 < 65536; q0 += 8 * 256) { fle_xmm a, b; uint16_t pa[8], pb[8]; for (int i = 0; i < 8; i++) { pa[i] = (uint16_t)p; pb[i] = (uint16_t)(q0 + i * 256 + (p & 255)); } memcpy(&a, pa, 16); memcpy(&b, pb, 16); mix(fle_i2_packsswb(a, b)); } END("fle_i2_packsswb"); }
    { BEGIN; for (uint64_t p = 0; p < (1ull << 32); p += 64) { fle_xmm a, b; a.u32[0] = a.u32[1] = (uint32_t)p; a.u32[2] = a.u32[3] = (uint32_t)(p ^ 0xffff0000u); b.u32[0] = (uint32_t)(p * 3); b.u32[1] = (uint32_t)(p + 1); b.u32[2] = (uint32_t)(p + 32768); b.u32[3] = (uint32_t)(p - 32769); mix(fle_i2_packssdw(a, b)); }
      static const int32_t C[] = { 0, 0x8000, -0x8000, 0x7fff, -0x7fff, 0x7fffffff, (int32_t)0x80000000 };
      for (int c = 0; c < 7; c++) for (int d = -40; d <= 40; d++) { fle_xmm a, b; for (int i = 0; i < 4; i++) { a.u32[i] = (uint32_t)(C[c] + d + i); b.u32[i] = (uint32_t)(C[c] - d - i); } mix(fle_i2_packssdw(a, b)); } END("fle_i2_packssdw"); }
    { BEGIN; static const int16_t W[27] = { 0, 1, -1, 2, -2, 0x7f, -0x80, 0x7fff, -0x8000, 0x7ffe, -0x7fff, 0x4000, -0x4000, 0x3fff, 0x1234, -0x1234, 0x100, -0x100, 255, -255, 181, -181, 32767 / 3, -32768 / 3, 3, -3, 0x5555 };
      for (int a0 = 0; a0 < 27; a0++) for (int a1 = 0; a1 < 27; a1++) for (int b0 = 0; b0 < 27; b0++) for (int b1 = 0; b1 < 27; b1++) { fle_xmm a, b; int16_t pa[8], pb[8]; for (int i = 0; i < 4; i++) { pa[2 * i] = W[(a0 + i) % 27]; pa[2 * i + 1] = W[(a1 + 2 * i) % 27]; pb[2 * i] = W[(b0 + 3 * i) % 27]; pb[2 * i + 1] = W[(b1 + i) % 27]; } memcpy(&a, pa, 16); memcpy(&b, pb, 16); mix(fle_lg_pmaddwd(a, b)); } END("fle_lg_pmaddwd"); }
    OPD(fle_paddd) OPD(fle_psubd) OPD(fle_pcmpeqd) OPD(fle_i2_pcmpgtd)
    OPQ(fle_i2_paddq) OPQ(fle_lg_psubq)
    SHW(fle_i2_psllw) SHW(fle_i2_psrlw) SHW(fle_i2_psraw) SHD(fle_i2_pslld) SHD(fle_i2_psrld) SHD(fle_i2_psrad) SHQ(fle_i2_psllq) SHQ(fle_i2_psrlq)
    /* unpacks: every byte value pair in the interleave positions (exhaustive over 65,536 byte pairs x position patterns) */
    for (int esz = 1; esz <= 8; esz *= 2) for (int hi = 0; hi < 2; hi++) { BEGIN; for (int p = 0; p < 256; p++) for (int q = 0; q < 256; q++) { fle_xmm a, b; uint8_t pa[16], pb[16]; for (int i = 0; i < 16; i++) { pa[i] = (uint8_t)(p + 17 * i); pb[i] = (uint8_t)(q + 29 * i); } memcpy(&a, pa, 16); memcpy(&b, pb, 16); mix(fle_unpack(a, b, esz, hi)); } char nm[40]; snprintf(nm, sizeof nm, "fle_unpack_%d_%d", esz, hi); END(nm); }
    return 0;
}

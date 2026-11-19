/* Decision 0195 (opt-in test tool): runs the scenes of b3scene.h against the REAL Blend2D.dll (Image-Line's, JIT enabled) under Wine and prints one hash per scene.
 * Build: x86_64-w64-mingw32-gcc -O2 -o b3_wine.exe b3_wine.c      Run (Blend2D.dll next to the exe, WINEPREFIX outside /tmp): wine b3_wine.exe OP STYLE GEOM RAMPNOISE OPAQUE SEED N [IDX0]
 * The call convention of the scan harness is reproduced with a variadic function pointer: in the Microsoft x64 convention a double passed to a variadic function goes into the integer register AND the xmm
 * register of its position, so every raw word is visible to a callee that wants it as an integer (rdx/r8/r9) or as a double (xmm1..3); words after the third go on the stack. */
#include <windows.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "b3scene.h"
typedef uint64_t (*vfn)(void *, ...);
static void *obj;
static double asd(uint64_t u) { double d; memcpy(&d, &u, 8); return d; }
static uint64_t docall(unsigned slot, int n, const uint64_t *w) {
    vfn f = (vfn)(((void ***)obj)[0][slot]); uint64_t a[8] = {0};
    int i; for (i = 0; i < n && i < 8; i++) a[i] = w[i];
    return f(obj, asd(a[0]), asd(a[1]), asd(a[2]), asd(a[3]), asd(a[4]), asd(a[5]), asd(a[6]), asd(a[7]));
}
static void ev_call(B3Env *e, unsigned slot, int n, const uint64_t *w) { (void)e; docall(slot, n, w); }
static uint64_t ev_ret(B3Env *e, unsigned slot, int n, const uint64_t *w) { (void)e; return docall(slot, n, w); }
static void ev_poke(B3Env *e, uint64_t ptr, unsigned off, uint16_t v) { (void)e; *(uint16_t *)(uintptr_t)(ptr + off) = v; }
static uint8_t *g_buf;
static void ev_mark(B3Env *e, const char *tag) { if (!strcmp(tag, "post") && getenv("B3_DUMP")) { FILE *f = fopen(getenv("B3_DUMP"), "ab"); fwrite(e->pix, 1, B3W * B3H * 4, f); fclose(f); } if (strcmp(tag, "post")) printf("mark %s %016llx\n", tag, (unsigned long long)b3_fnv(e->pix, B3W * B3H * 4)); }
int main(int argc, char **argv) {
    if (argc < 8) { fprintf(stderr, "usage: b3_wine OP STYLE GEOM RAMPNOISE OPAQUE SEED N [IDX0]\n"); return 2; }
    unsigned op = (unsigned)strtoul(argv[1], 0, 0), style = (unsigned)strtoul(argv[2], 0, 0), geom = (unsigned)strtoul(argv[3], 0, 0), rn = (unsigned)strtoul(argv[4], 0, 0), opq = (unsigned)strtoul(argv[5], 0, 0);
    uint64_t seed = strtoull(argv[6], 0, 0); long n = atol(argv[7]), i0 = argc > 8 ? atol(argv[8]) : 0, i;
    HMODULE h = LoadLibraryA("Blend2D.dll"); if (!h) { fprintf(stderr, "no Blend2D.dll\n"); return 3; }
    void *(*create)(void) = (void *(*)(void))GetProcAddress(h, "AggCreateInstance"); obj = create();
    uint8_t *buf = (uint8_t *)VirtualAlloc(0, B3W * B3H * 4, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE); memset(buf, 0, B3W * B3H * 4);
    { vfn att = (vfn)(((void ***)obj)[0][1]); att(obj, (uint64_t)(uintptr_t)buf, (uint64_t)B3W, (uint64_t)B3H, (uint64_t)(B3W * 4)); }
    B3Env e = {ev_call, ev_ret, ev_poke, buf, 0, getenv("B3_DEBUG") ? ev_mark : 0};
    for (i = i0; i < i0 + n; i++) { b3_scene(&e, op, style, geom, rn, opq, seed, i); printf("b3 %ld %016llx\n", i, (unsigned long long)b3_fnv(buf, B3W * B3H * 4)); }
    return 0;
}

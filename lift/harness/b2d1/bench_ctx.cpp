/* Decision 0160 (provisional): cost of one BLContext create + draw + destroy on a 64x64 buffer in upstream Blend2D 0.11.1 (no JIT), without any lifted code: what one
 * Agg2D::attach costs the swapped build at least. Native and wasm (node). usage: bench_ctx [N] -> process CPU seconds per cycle (clock_gettime CLOCK_PROCESS_CPUTIME_ID). */
#include <blend2d.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>
static double cpu(void) { struct timespec t; clock_gettime(CLOCK_PROCESS_CPUTIME_ID, &t); return (double)t.tv_sec + 1e-9 * (double)t.tv_nsec; }
int main(int argc, char **argv) {
    blRuntimeInit();
    int N = argc > 1 ? atoi(argv[1]) : 2000;
    static unsigned char buf[64 * 64 * 4];
    BLContextCreateInfo ci{}; ci.flags = BL_CONTEXT_CREATE_FLAG_DISABLE_JIT;
    for (int warm = 0; warm < 2; warm++) {
        double t0 = cpu();
        for (int i = 0; i < N; i++) {
            BLImage img; img.createFromData(64, 64, BL_FORMAT_PRGB32, buf, 64 * 4);
            BLContext ctx(img, ci);
            ctx.fillAll(BLRgba32(0xff202830u));
            ctx.fillRect(BLRect(5, 5, 35, 25), BLRgba32(0xffc86432u));
            BLPath p; p.moveTo(3, 60); p.lineTo(30, 10); p.lineTo(60, 50);
            ctx.setStrokeWidth(2.5); ctx.strokePath(p, BLRgba32(0xff0ac814u));
            ctx.end();
        }
        double t1 = cpu();
        if (warm) printf("create+3 draws+destroy: %.1f us per cycle (%d cycles, %.3f s process cpu)\n", 1e6 * (t1 - t0) / N, N, t1 - t0);
    }
    return 0;
}

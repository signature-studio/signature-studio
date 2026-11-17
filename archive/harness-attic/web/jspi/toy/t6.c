#include <stdio.h>
#include <stdlib.h>
#include <emscripten.h>
EM_JS(int, nop, (int x), { return x; });
EM_JS(double, now, (void), { return performance.now(); });
static long N;
__attribute__((noinline)) long loop(void) { long s = 0; for (long i = 0; i < N; i++) s += nop((int)i); return s; }
__attribute__((noinline)) long deep(int d) { volatile char pad[32]; pad[0] = (char)d; if (d == 0) return loop(); return deep(d - 1) + (pad[1] & 0); }
int main(int argc, char **argv) {
    int d = argc > 1 ? atoi(argv[1]) : 0; N = 20000000L;
    double t0 = now(); long s = deep(d); double t1 = now();
    printf("depth %d: %.2f ns per import call (sum %ld)\n", d, (t1 - t0) * 1e6 / N, s);
    return 0;
}

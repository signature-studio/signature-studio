#include <stdio.h>
#include <stdlib.h>
#include <emscripten.h>
EM_JS(int, nop, (int x), { return x; });
EM_JS(double, now, (void), { return performance.now(); });
int main(int argc, char **argv) {
    long n = argc > 1 ? atol(argv[1]) : 100000000L;
    long s = 0;
    double t0 = now();
    for (long i = 0; i < n; i++) s += nop((int)i);
    double t1 = now();
    printf("%ld imports in %.1f ms = %.2f ns each (sum %ld)\n", n, t1 - t0, (t1 - t0) * 1e6 / n, s);
    return 0;
}

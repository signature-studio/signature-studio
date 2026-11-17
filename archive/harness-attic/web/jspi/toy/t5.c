#include <stdio.h>
#include <stdlib.h>
#include <emscripten.h>
EM_ASYNC_JS(int, susp, (int k), { await Promise.resolve(); return k; });
__attribute__((noinline)) int deep(int n) { volatile char pad[64]; pad[0] = (char)n; if (n == 0) return susp(1) + pad[0]; return deep(n - 1) + 1 + (pad[1] & 0); }
int main(int argc, char **argv) { int n = argc > 1 ? atoi(argv[1]) : 100; printf("depth %d -> %d\n", n, deep(n)); return 0; }

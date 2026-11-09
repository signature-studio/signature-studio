#include <stdint.h>
#include <string.h>
#include <emscripten.h>
__attribute__((noinline)) static uint64_t ld_var(const uint8_t *p, unsigned n) { uint64_t v = 0; memcpy(&v, p, n); return v; }
__attribute__((noinline)) static uint64_t ld_sw(const uint8_t *p, unsigned n) {
    uint64_t v = 0;
    switch (n) { case 1: { uint8_t x; memcpy(&x, p, 1); return x; } case 2: { uint16_t x; memcpy(&x, p, 2); return x; } case 4: { uint32_t x; memcpy(&x, p, 4); return x; } case 8: { uint64_t x; memcpy(&x, p, 8); return x; } default: memcpy(&v, p, n); return v; }
}
static uint8_t buf[4096];
EMSCRIPTEN_KEEPALIVE uint64_t bench_var(unsigned iters) { uint64_t s = 0; for (unsigned i = 0; i < iters; i++) s += ld_var(buf + (i & 0x7f8), 8 >> (i & 3)); return s; }
EMSCRIPTEN_KEEPALIVE uint64_t bench_sw(unsigned iters) { uint64_t s = 0; for (unsigned i = 0; i < iters; i++) s += ld_sw(buf + (i & 0x7f8), 8 >> (i & 3)); return s; }
int main() { return 0; }

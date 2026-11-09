// mt1 (decision 0173) experiment (iii): what breaks when the lifted engine's state lives in ONE shared linear memory that several threads run on.
// A kernel written the way fe-lift writes code (every "register" is a load/store of a global array R[], like lift/runtime/fle_lowregs.h with R at a fixed low address)
// is run by two pthreads (emscripten pthreads = Workers on one shared WebAssembly.Memory) in four variants:
//   shared   : R is one global array (what the generated C does today)            -> the threads overwrite each other's registers
//   tls      : R is _Thread_local (wasm TLS: __tls_base + offset)                    -> correct, costs an add per access
//   ptr      : R is reached through a pointer parameter (a register block per thread) -> correct
//   plain/atomic counter: a "lock xadd" lifted as ld/add/st (what int_more.rs xadd emits today) against __atomic_fetch_add
// Output: one line per measurement, key=value.
#include <pthread.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#define NITER 60000000
uint64_t R_shared[16];
static _Thread_local uint64_t R_tls[16];
volatile uint32_t plain_counter;
volatile uint32_t atomic_counter;
volatile uint32_t go;

#define KERN(RR, seed, n) do { \
    RR[0] = (seed); RR[1] = (seed) * 7 + 1; RR[2] = 0; \
    for (long i_ = 0; i_ < (n); i_++) { \
        RR[0] = RR[0] * 3 + RR[1]; \
        RR[1] = RR[1] ^ (RR[0] >> 7); \
        RR[2] += RR[0] & 0xff; \
    } \
} while (0)

static uint64_t expect(uint64_t seed, long n) { uint64_t a = seed, b = seed * 7 + 1, c = 0; for (long i = 0; i < n; i++) { a = a * 3 + b; b = b ^ (a >> 7); c += a & 0xff; } return a ^ b ^ c; }

__attribute__((noinline)) static uint64_t run_shared(uint64_t seed, long n) { volatile uint64_t *R = R_shared; KERN(R, seed, n); return R[0] ^ R[1] ^ R[2]; }
__attribute__((noinline)) static uint64_t run_tls(uint64_t seed, long n) { volatile uint64_t *R = R_tls; KERN(R, seed, n); return R[0] ^ R[1] ^ R[2]; }
__attribute__((noinline)) static uint64_t run_ptr(volatile uint64_t *R, uint64_t seed, long n) { KERN(R, seed, n); return R[0] ^ R[1] ^ R[2]; }

typedef struct { int id; int variant; uint64_t got, want; double ms; long cnt; } targ;
static double now_ms(void) { struct timespec t; clock_gettime(CLOCK_MONOTONIC, &t); return t.tv_sec * 1e3 + t.tv_nsec / 1e6; }

static void *thr(void *p)
{
    targ *a = p;
    uint64_t seed = 12345 + 1000 * a->id;
    uint64_t block[16];
    while (!__atomic_load_n(&go, __ATOMIC_ACQUIRE)) { }
    double t0 = now_ms();
    switch (a->variant) {
    case 0: a->got = run_shared(seed, NITER); break;
    case 1: a->got = run_tls(seed, NITER); break;
    case 2: a->got = run_ptr(block, seed, NITER); break;
    case 3: for (long i = 0; i < a->cnt; i++) { uint32_t v = __atomic_load_n(&plain_counter, __ATOMIC_RELAXED); __atomic_store_n(&plain_counter, v + 1, __ATOMIC_RELAXED); } break; /* ld; add; st as fe-lift emits `lock xadd` today (int_more.rs): each half is atomic, the pair is not */
    case 4: for (long i = 0; i < a->cnt; i++) __atomic_fetch_add(&atomic_counter, 1, __ATOMIC_SEQ_CST); break;
    }
    a->ms = now_ms() - t0;
    a->want = a->variant <= 2 ? expect(seed, NITER) : 0;
    return 0;
}

static void trial(int variant, int nthreads, long cnt)
{
    pthread_t th[4]; targ a[4];
    __atomic_store_n(&go, 0, __ATOMIC_RELEASE);
    plain_counter = 0; atomic_counter = 0;
    for (int i = 0; i < nthreads; i++) { a[i] = (targ){ .id = i, .variant = variant, .cnt = cnt }; pthread_create(&th[i], 0, thr, &a[i]); }
    __atomic_store_n(&go, 1, __ATOMIC_RELEASE);
    for (int i = 0; i < nthreads; i++) pthread_join(th[i], 0);
    static const char *nm[] = { "shared", "tls", "ptr", "plain_counter", "atomic_counter" };
    int ok = 1; double ms = 0;
    for (int i = 0; i < nthreads; i++) { if (variant <= 2 && a[i].got != a[i].want) ok = 0; if (a[i].ms > ms) ms = a[i].ms; }
    if (variant == 3) printf("variant=%s threads=%d adds_each=%ld final=%u expected=%ld lost=%ld ms=%.1f\n", nm[variant], nthreads, cnt, plain_counter, cnt * nthreads, cnt * nthreads - (long)plain_counter, ms);
    else if (variant == 4) printf("variant=%s threads=%d adds_each=%ld final=%u expected=%ld lost=%ld ms=%.1f ns_per_add=%.2f\n", nm[variant], nthreads, cnt, atomic_counter, cnt * nthreads, cnt * nthreads - (long)atomic_counter, ms, ms * 1e6 / (double)cnt);
    else printf("variant=%s threads=%d iters_each=%d all_results_correct=%d ms_slowest_thread=%.1f ns_per_iter=%.2f\n", nm[variant], nthreads, NITER, ok, ms, ms * 1e6 / NITER);
    fflush(stdout);
}

int main(void)
{
    for (int rep = 0; rep < 2; rep++) {
        for (int v = 0; v <= 2; v++) { trial(v, 1, 0); trial(v, 2, 0); }
        trial(3, 1, 10000000); trial(3, 2, 10000000); trial(4, 1, 10000000); trial(4, 2, 10000000);
    }
    printf("DONE\n");
    return 0;
}

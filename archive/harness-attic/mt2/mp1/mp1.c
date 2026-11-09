/* mt2 micro-prototype 1 (decision 0234, measurement only): the smallest thing that a second Worker needs in order to run lifted-style code on the same linear memory.
 * One WebAssembly module (shared, imported memory), instantiated once per Worker, no emscripten pthread runtime:
 *   - the guest register file shape R[16] / X[32] is _Thread_local (wasm TLS: `global.get __tls_base; i64.load offset`), each instance gets its own TLS block through __wasm_init_tls;
 *   - kernel(n) is a register-file-heavy loop (the access pattern of lifted C: every step loads and stores R[] in memory), deterministic, so a result from a second Worker can be compared with a single-thread run;
 *   - helper_loop(slot) is the loop of the second Worker: wait (memory.atomic.wait32) for a job word, run kernel, store the result, notify;
 *   - job_post/job_wait are the engine side (usable from the first Worker, which may block in Workers).
 * -DGLOBAL_R builds the negative control (R / X plain globals: two Workers corrupt each other, the mt1 smem finding in this setting). */
#include <stdint.h>

#ifdef GLOBAL_R
static uint64_t R[16];
static uint64_t X[32];
#else
static _Thread_local uint64_t R[16];
static _Thread_local uint64_t X[32];
#endif

enum { ST_IDLE = 0, ST_JOB = 1, ST_DONE = 2, ST_EXIT = 3 };
typedef struct { volatile uint32_t state; uint32_t spin; uint64_t arg; uint64_t result; uint32_t kind; uint32_t pad1[11]; } slot_t;   /* 64 bytes: one slot per cache line */
__attribute__((aligned(64))) slot_t slots[8];
__attribute__((aligned(64))) uint8_t tls_blocks[8][4096];

__attribute__((noinline)) uint64_t kernel(uint64_t n)
{
    uint64_t s = 0, i;
    R[1] = 1; R[2] = 7; for (i = 0; i < 32; i++) X[i] = 0;
    for (i = 0; i < n; i++) {
        R[1] += i;
        R[2] ^= R[1] * 3u;
        R[3] = R[2] + R[1];
        s += R[3];
        X[i & 31] += s;
    }
    return s ^ X[5] ^ R[2];
}

static inline void wait_while(volatile uint32_t *p, uint32_t v) { while (*p == v) __builtin_wasm_memory_atomic_wait32((int32_t *)p, (int32_t)v, -1); }

void helper_loop(uint32_t slot_idx)
{
    slot_t *s = &slots[slot_idx];
    for (;;) {
        uint32_t st, k;
        for (k = 0; k < s->spin; k++) { st = __atomic_load_n(&s->state, __ATOMIC_ACQUIRE); if (st == ST_JOB || st == ST_EXIT) break; }
        while ((st = __atomic_load_n(&s->state, __ATOMIC_ACQUIRE)) != ST_JOB && st != ST_EXIT)
            __builtin_wasm_memory_atomic_wait32((int32_t *)&s->state, (int32_t)st, -1);
        if (st == ST_EXIT) { __atomic_store_n(&s->state, ST_DONE, __ATOMIC_RELEASE); __builtin_wasm_memory_atomic_notify((int32_t *)&s->state, 1); return; }
        s->result = s->kind == 0 ? 0 : kernel(s->arg);
        __atomic_store_n(&s->state, ST_DONE, __ATOMIC_RELEASE);
        __builtin_wasm_memory_atomic_notify((int32_t *)&s->state, 1);
        /* wait for the engine side to take the result and reset the slot */
        while (__atomic_load_n(&s->state, __ATOMIC_ACQUIRE) == ST_DONE) __builtin_wasm_memory_atomic_wait32((int32_t *)&s->state, ST_DONE, -1);
    }
}
void job_post(uint32_t slot_idx, uint32_t kind, uint64_t arg)
{
    slot_t *s = &slots[slot_idx];
    s->arg = arg; s->kind = kind;
    __atomic_store_n(&s->state, ST_JOB, __ATOMIC_RELEASE);
    __builtin_wasm_memory_atomic_notify((int32_t *)&s->state, 1);
}
uint64_t job_wait(uint32_t slot_idx)
{
    slot_t *s = &slots[slot_idx];
    uint32_t st;
    while ((st = __atomic_load_n(&s->state, __ATOMIC_ACQUIRE)) != ST_DONE) __builtin_wasm_memory_atomic_wait32((int32_t *)&s->state, (int32_t)st, -1);
    uint64_t r = s->result;
    __atomic_store_n(&s->state, ST_IDLE, __ATOMIC_RELEASE);
    __builtin_wasm_memory_atomic_notify((int32_t *)&s->state, 1);
    return r;
}
/* spin up to `spins` iterations before sleeping (the "spin then wait" variant of the engine side) */
uint64_t job_wait_spin(uint32_t slot_idx, uint32_t spins)
{
    slot_t *s = &slots[slot_idx];
    uint32_t i;
    for (i = 0; i < spins; i++) if (__atomic_load_n(&s->state, __ATOMIC_ACQUIRE) == ST_DONE) break;
    return job_wait(slot_idx);
}
void job_exit(uint32_t slot_idx)
{
    slot_t *s = &slots[slot_idx];
    __atomic_store_n(&s->state, ST_EXIT, __ATOMIC_RELEASE);
    __builtin_wasm_memory_atomic_notify((int32_t *)&s->state, 1);
}
uint32_t tls_block_addr(uint32_t i) { return (uint32_t)(uintptr_t)tls_blocks[i]; }
uint32_t tls_block_size(void) { return sizeof tls_blocks[0]; }
void set_spin(uint32_t i, uint32_t n) { slots[i].spin = n; }
uint32_t slot_state(uint32_t i) { return slots[i].state; }

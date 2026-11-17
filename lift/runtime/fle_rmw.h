/* The guest's atomic read-modify-write instructions (mt4 stage A): LOCK-prefixed instructions with a memory operand, and XCHG with a memory operand, which the processor locks implicitly
 * (Intel SDM Vol. 2, XCHG: "If a memory operand is referenced, the processor's locking protocol is automatically implemented for the duration of the exchange operation, regardless of the presence or absence of the LOCK prefix";
 * Vol. 3A 9.1: the LOCK prefix makes the read-modify-write a single atomic operation with respect to other processors). fe-lift emits calls of fle_cas, fle_xchg and fle_xadd for CMPXCHG, XCHG and XADD with a memory
 * operand (whether or not the LOCK prefix is present: a superset of the contract), a compare-and-swap loop around fle_cas for the other LOCK-prefixed instructions (ADD, ADC, SUB, SBB, AND, OR, XOR, INC, DEC, NEG, NOT, BTS, BTR,
 * BTC), and fle_cmpxchg16b for CMPXCHG16B. They act on the guest's linear memory with the compiler's __atomic operations (WebAssembly: i32/i64.atomic.rmw.* on the shared memory), so two guest threads that run at the same
 * time on different processors (Workers) are ordered as on x86. Each of the three returns the OLD value of the operand, which the lifted instruction needs for the flags and the registers.
 *
 * Misaligned operands: x86 performs a locked access that crosses a boundary (a "split lock"); WebAssembly atomics trap on one. A misaligned operand takes the global interlock below instead (a spin lock that every such operand
 * and every CMPXCHG16B takes): atomic against the other operands that take it, not against a plain store to the same bytes. CMPXCHG16B: WebAssembly has no 16-byte atomic; the same interlock is used, so a 16-byte compare-and-swap is atomic
 * against other CMPXCHG16B and not against 8-byte accesses to its halves (the lock-free lists of the guest use CMPXCHG16B for all their updates). */
#ifndef FLE_RMW_H
#define FLE_RMW_H
#include <stdint.h>
#include "fle_runtime.h"

__attribute__((weak)) volatile uint32_t fle_rmw_word = 0;   /* weak in every translation unit that includes this header (the same way as fle_dispatch_hook): the linker keeps one */
static inline void fle_rmw_begin(void)
{
    while (__builtin_expect(__atomic_exchange_n(&fle_rmw_word, 1u, __ATOMIC_ACQUIRE) != 0u, 0)) {
        while (__atomic_load_n(&fle_rmw_word, __ATOMIC_RELAXED) != 0u) {}
    }
}
static inline void fle_rmw_end(void) { __atomic_store_n(&fle_rmw_word, 0u, __ATOMIC_RELEASE); }

#define FLE_RMW_MASK(n) ((n) >= 8u ? ~UINT64_C(0) : ((UINT64_C(1) << (8u * (n))) - 1u))

/* compare-and-swap: the operand becomes `desired` when it equals `expected`; returns the value the operand had */
static inline uint64_t fle_cas(uint64_t a, unsigned n, uint64_t expected, uint64_t desired)
{
    uintptr_t p = fle_addr(a, n, 1);

    if (__builtin_expect((p & (n - 1u)) == 0, 1)) {
        switch (n) {
        case 1: { uint8_t e = (uint8_t)expected; __atomic_compare_exchange_n((uint8_t *)p, &e, (uint8_t)desired, 0, __ATOMIC_SEQ_CST, __ATOMIC_SEQ_CST); return e; }
        case 2: { uint16_t e = (uint16_t)expected; __atomic_compare_exchange_n((uint16_t *)p, &e, (uint16_t)desired, 0, __ATOMIC_SEQ_CST, __ATOMIC_SEQ_CST); return e; }
        case 4: { uint32_t e = (uint32_t)expected; __atomic_compare_exchange_n((uint32_t *)p, &e, (uint32_t)desired, 0, __ATOMIC_SEQ_CST, __ATOMIC_SEQ_CST); return e; }
        default: { uint64_t e = expected; __atomic_compare_exchange_n((uint64_t *)p, &e, desired, 0, __ATOMIC_SEQ_CST, __ATOMIC_SEQ_CST); return e; }
        }
    }
    {
        uint64_t o;

        fle_rmw_begin();
        o = fle_ld(a, n);
        if (o == (expected & FLE_RMW_MASK(n))) fle_st(a, n, desired);
        fle_rmw_end();
        return o;
    }
}

/* exchange: the operand becomes `v`; returns the value it had */
static inline uint64_t fle_xchg(uint64_t a, unsigned n, uint64_t v)
{
    uintptr_t p = fle_addr(a, n, 1);

    if (__builtin_expect((p & (n - 1u)) == 0, 1)) {
        switch (n) {
        case 1: return __atomic_exchange_n((uint8_t *)p, (uint8_t)v, __ATOMIC_SEQ_CST);
        case 2: return __atomic_exchange_n((uint16_t *)p, (uint16_t)v, __ATOMIC_SEQ_CST);
        case 4: return __atomic_exchange_n((uint32_t *)p, (uint32_t)v, __ATOMIC_SEQ_CST);
        default: return __atomic_exchange_n((uint64_t *)p, v, __ATOMIC_SEQ_CST);
        }
    }
    {
        uint64_t o;

        fle_rmw_begin();
        o = fle_ld(a, n);
        fle_st(a, n, v);
        fle_rmw_end();
        return o;
    }
}

/* fetch-and-add: the operand becomes operand + `v` (modulo its width); returns the value it had */
static inline uint64_t fle_xadd(uint64_t a, unsigned n, uint64_t v)
{
    uintptr_t p = fle_addr(a, n, 1);

    if (__builtin_expect((p & (n - 1u)) == 0, 1)) {
        switch (n) {
        case 1: return __atomic_fetch_add((uint8_t *)p, (uint8_t)v, __ATOMIC_SEQ_CST);
        case 2: return __atomic_fetch_add((uint16_t *)p, (uint16_t)v, __ATOMIC_SEQ_CST);
        case 4: return __atomic_fetch_add((uint32_t *)p, (uint32_t)v, __ATOMIC_SEQ_CST);
        default: return __atomic_fetch_add((uint64_t *)p, v, __ATOMIC_SEQ_CST);
        }
    }
    {
        uint64_t o;

        fle_rmw_begin();
        o = fle_ld(a, n);
        fle_st(a, n, o + v);
        fle_rmw_end();
        return o;
    }
}

/* CMPXCHG16B m128 (the address is 16-byte aligned: the caller stops otherwise, as the SDM's #GP): 1 and the operand becomes rcx:rbx when it equals *rdx:*rax; else 0 and *rdx:*rax becomes the operand */
static inline int fle_cmpxchg16b(uint64_t a, uint64_t *rax, uint64_t *rdx, uint64_t rbx, uint64_t rcx)
{
    uint64_t lo, hi;
    int eq;

    fle_rmw_begin();
    lo = fle_ld(a, 8);
    hi = fle_ld(a + 8, 8);
    eq = lo == *rax && hi == *rdx;
    if (eq) {
        fle_st(a, 8, rbx);
        fle_st(a + 8, 8, rcx);
    }
    fle_rmw_end();
    if (!eq) {
        *rax = lo;
        *rdx = hi;
    }
    return eq;
}
#endif

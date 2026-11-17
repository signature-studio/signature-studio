/* Runtime support for fe-lift output. Identity memory map, eager status flags. */
#ifndef FLE_RUNTIME_H
#define FLE_RUNTIME_H

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* The target is decided here and nowhere else (decision 0243): the wasm build (emscripten) keeps the guest register file R, the flags FL and the XMM file X in the thread-local block
 * fle_ctx (fle_lowregs.h, fle_ctx_tls.c); the generated C tests these two names, so they are defined here, before anything is included. A native build has the arrays R, FL, X. */
#ifdef __EMSCRIPTEN__
#define FLE_LOWREGS 1
#define FLE_LOWREGS_X 1
#endif

#include "fle_ai.h"
#include "fle_flags.h"

#ifdef FLE_LOWREGS
#include "fle_lowregs.h"
#else
extern uint64_t R[16];
#endif

/* Guest addresses are 64-bit values used as host addresses (identity map). On wasm32 (emscripten) a host address
 * is 32 bits, so (uintptr_t)a would silently drop bits 63:32 and a wild guest pointer would alias another address
 * (docs/decisions/0045-wasm-port.md). The wasm build therefore stops on an address above 4 GiB, the counterpart of
 * the SIGSEGV a native run gets. FLE_WASM32 is set by emscripten; FLE_CHECK_HI32 may also be set by hand (a native
 * non-PIE program whose data lies below 4 GiB, as the tests do).
 */
#if defined(__EMSCRIPTEN__) && !defined(FLE_WASM32)
#define FLE_WASM32 1
#endif
#if defined(FLE_WASM32) && !defined(FLE_CHECK_HI32)
#define FLE_CHECK_HI32 1
#endif

#if defined(FLE_CHECK_HI32)
/* bits 63:32 of a guest address are not zero. Never returns (fle_host.c). */
void fle_addr_fault(uint64_t a, unsigned n, int store) __attribute__((noreturn, cold));
#endif

#if defined(FLE_CHECK_HI32)
static inline FLE_AI uintptr_t fle_addr(uint64_t a, unsigned n, int store) {
    if (a >> 32) fle_addr_fault(a, n, store);
    return (uintptr_t)a;
}
/* A guest pointer argument that the API may legally receive as NULL (checked after the conversion): NULL is
 * passed through, anything else goes through fle_addr. */
static inline FLE_AI uintptr_t fle_addr_opt(uint64_t a, unsigned n, int store) {
    return a ? fle_addr(a, n, store) : (uintptr_t)0;
}
#else
/* native: exactly the original conversion, no call */
#define fle_addr(a, n, store) ((uintptr_t)(a))
#define fle_addr_opt(a, n, store) ((uintptr_t)(a))
#endif

/* fle_ld / fle_st: the little-endian n bytes at the guest address in the low bytes of a uint64_t (the rest 0) / the low n bytes of v stored. A memcpy with a run-time n is a call (a wasm
 * memory.copy) when n is not a constant at the point of use (a call that is not inlined, 4.4-5.0 ns against 0.9-1.5 ns in node 24, decision 0234); the switch makes every n in 1, 2, 4, 8 a
 * constant-size access, which is what a constant n already folded to. Any other n (3, 5, 6, 7, ...) is the memcpy of before. The address check (fle_addr) is made once, first, with the same
 * arguments as before. */
static inline FLE_AI uint64_t fle_ld(uint64_t a, unsigned n) {
    const void *p = (const void *)fle_addr(a, n, 0);
    switch (n) {
    case 1: { uint8_t x; memcpy(&x, p, 1); return x; }
    case 2: { uint16_t x; memcpy(&x, p, 2); return x; }
    case 4: { uint32_t x; memcpy(&x, p, 4); return x; }
    case 8: { uint64_t x; memcpy(&x, p, 8); return x; }
    default: { uint64_t v = 0; memcpy(&v, p, n); return v; }
    }
}

static inline FLE_AI void fle_st(uint64_t a, unsigned n, uint64_t v) {
    void *p = (void *)fle_addr(a, n, 1);
    switch (n) {
    case 1: { uint8_t x = (uint8_t)v; memcpy(p, &x, 1); break; }
    case 2: { uint16_t x = (uint16_t)v; memcpy(p, &x, 2); break; }
    case 4: { uint32_t x = (uint32_t)v; memcpy(p, &x, 4); break; }
    case 8: memcpy(p, &v, 8); break;
    default: memcpy(p, &v, n); break;
    }
}

/* Instruction outside the minimal set. Stop instead of skipping it. */
static inline FLE_AI void fle_unsupported(uint64_t va) {
    fprintf(stderr, "fle: unsupported instruction at %#llx\n", (unsigned long long)va);
    abort();
}

#include "fle_rmw.h"   /* mt4 stage A: fle_cas, fle_xchg, fle_xadd, fle_cmpxchg16b: the guest's atomic read-modify-write instructions (they use fle_addr, fle_ld and fle_st above) */

#endif

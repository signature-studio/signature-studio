/* Calling a guest function from the runtime (docs/decisions/0064-guest-callback.md). Stateless except the nesting counter.
 * Nothing here reads the host: no thread, no clock, no file, no signal. */
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_dispatch.h"
#include "fle_env.h"
#include "fle_callback.h"

/* Decision 0319: the TEB of the running guest thread (fle_vex.h; weak, so that a link without it still links: no guest stack is registered then and that check is skipped) */
extern uint8_t fle_teb[] __attribute__((weak));
extern uint8_t *fle_teb_cur __attribute__((weak));

static unsigned depth;

unsigned fle_callback_depth(void)
{
    return depth;
}

/* Decision 0122: the nesting depth belongs to the guest thread; fle_thread.c saves and loads it at every switch. */
void fle_callback_depth_set(unsigned d)
{
    depth = d;
}

/* The guest registers that a Win64 callee must preserve and that live in R[]: rbx 3, rbp 5, rsi 6, rdi 7, r12-r15 12..15 (rsp is
 * R[4], checked separately). */
static const unsigned preserved[] = { 3, 5, 6, 7, 12, 13, 14, 15 };
static const char *const preserved_name[] = { "rbx", "rbp", "rsi", "rdi", "r12", "r13", "r14", "r15" };

uint64_t fle_call_guest(uint64_t va, uint64_t rcx, uint64_t rdx, uint64_t r8, uint64_t r9)
{
    static const char api[] = "fle_call_guest";
    const unsigned n = (unsigned)(sizeof preserved / sizeof preserved[0]);
    uint64_t keep[sizeof preserved / sizeof preserved[0]];
    uint64_t entry_sp = R[4];
    uint64_t call_sp;
    uint64_t result;
    unsigned i;
    char why[160];

    if (va == 0) fle_stop(api, "the callback address is 0");
    {   /* Decision 0319: Windows limits the nesting by the stack only (see fle_callback.h) */
        const uint8_t *cur = NULL, *teb;
        uint64_t limit = 0, base = 0, reserve = FLE_CB_GUEST_RESERVE;

        if (&fle_teb_cur != NULL) cur = fle_teb_cur;   /* a weak VARIABLE: read it only when it is linked (a test that links fle_callback.c with a few files has none) */
        teb = cur ? cur : fle_teb;
        if (teb) {
            memcpy(&base, teb + 0x8, sizeof base);     /* NT_TIB.StackBase / StackLimit (fle_vex.h FLE_TEB_STACKBASE, FLE_TEB_STACKLIMIT) */
            memcpy(&limit, teb + 0x10, sizeof limit);
        }
        if (limit != 0 && base > limit && (base - limit) / 4u < reserve) reserve = (base - limit) / 4u;   /* a small stack (the tests use 64 KiB): a quarter of it */
        if (limit != 0 && entry_sp < limit + reserve) {
            snprintf(why, sizeof why, "callback nesting %u: the guest stack is used up (rsp %#" PRIx64 ", StackLimit %#" PRIx64 ", reserve %" PRIu64 "): Windows limits the nesting by the stack size only, decision 0319", depth + 1u,
                     (uint64_t)entry_sp, (uint64_t)limit, (uint64_t)reserve);
            fle_stop(api, why);
        }
    }
    if (fle_cstack_left) {   /* defined by the file that knows the host (fle_thread_wasm.c in a wasm build); not linked = unknown */
        size_t host_free = fle_cstack_left();

        if (host_free < FLE_CB_HOST_RESERVE) {
            snprintf(why, sizeof why, "callback nesting %u: the host (C) stack of the thread is used up (%" PRIu64 " bytes free, reserve %u): Windows limits the nesting by the stack size only, decision 0319", depth + 1u,
                     (uint64_t)host_free, (unsigned)FLE_CB_HOST_RESERVE);
            fle_stop(api, why);
        }
    }
    if (depth >= FLE_CB_MAX_DEPTH) {
        snprintf(why, sizeof why, "callback nesting is deeper than FLE_CB_MAX_DEPTH (%u), the backstop against a runaway recursion: decision 0319", (unsigned)FLE_CB_MAX_DEPTH);
        fle_stop(api, why);
    }

    /* The value of rsp at the call instruction: 16-byte aligned, with the shadow space between it and the caller's stack pointer.
     * Rounding down keeps everything at and above entry_sp untouched. */
    call_sp = (entry_sp - FLE_CB_SHADOW_BYTES) & ~UINT64_C(15);

    for (i = 0; i < n; i++) keep[i] = R[preserved[i]];

    R[1] = rcx;
    R[2] = rdx;
    R[8] = r8;
    R[9] = r9;
    R[4] = call_sp - 8;
    fle_st(R[4], 8, FLE_CB_RETURN_ADDRESS);

    depth++;
    fle_dispatch(va);
    depth--;

    /* The lifted `ret` is a C return and leaves R[4] at the return address slot; the caller (here) pops it, and with it the shadow
     * space and the alignment padding: R[4] goes back to the entry value below. */
    if (R[4] != call_sp - 8) fle_stop(api, "the callback returned with R[4] not at its return address: the stack is unbalanced");
    if (fle_ld(R[4], 8) != FLE_CB_RETURN_ADDRESS) fle_stop(api, "the callback changed its return address slot");
    for (i = 0; i < n; i++) {
        if (R[preserved[i]] != keep[i]) {
            snprintf(why, sizeof why, "the callback changed the callee-saved register %s", preserved_name[i]);
            fle_stop(api, why);
        }
    }
    R[4] = entry_sp;
    result = R[0];
    return result;
}

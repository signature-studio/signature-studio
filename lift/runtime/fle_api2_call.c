/* Decision 0210 (api2, PROVISIONAL, opt-in): runtime-internal call of another fle_imp_* function with the guest ABI (fle_api2.h).
 * The callee gets a fresh frame BELOW the caller's guest stack pointer (the guest stack grows down; the callee may keep structures under R[4], as fle_win.c's apply_pos does for WINDOWPOS). When R[4] is 0 (a test driver that has no guest
 * stack) a private stack is used. R[1..15] are restored afterwards. */
#include <stdint.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_api2.h"

static uint64_t private_stack[4096] __attribute__((aligned(16)));

uint64_t fle_api2_call(void (*f)(void), uint64_t a1, uint64_t a2, uint64_t a3, uint64_t a4, uint64_t a5, uint64_t a6, uint64_t a7, uint64_t a8)
{
    uint64_t saved[16], sp, r;

    memcpy(saved, R, sizeof saved);
    if (saved[4] == 0) sp = (uint64_t)(uintptr_t)&private_stack[4000];
    else sp = (saved[4] - 0x200u) & ~UINT64_C(15);
    sp -= 8;                    /* as after a `call`: [sp] is the return address */
    fle_st(sp, 8, 0);           /* the guest return address of the callee: never used */
    fle_st(sp + 0x28, 8, a5);   /* argument 5 (after the return address and the 32-byte shadow space) */
    fle_st(sp + 0x30, 8, a6);
    fle_st(sp + 0x38, 8, a7);
    fle_st(sp + 0x40, 8, a8);
    R[1] = a1;
    R[2] = a2;
    R[8] = a3;
    R[9] = a4;
    R[4] = sp;
    R[0] = 0;
    f();
    r = R[0];
    memcpy(R, saved, sizeof saved);
    R[0] = r;
    return r;
}

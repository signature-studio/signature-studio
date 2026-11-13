/* Calling a guest function from the runtime (docs/decisions/0064-guest-callback.md; the facts read from the generated C and
 * from Microsoft Learn are listed there and in docs/investigations/callback-caller.md). The first user is user32!EnumDisplayMonitors
 * (fle_monitor.c); window procedures, EnumWindows and timers will use the same function.
 *
 * The lifted code keeps all machine state in the globals R[] (fle_runtime.h) and calls a function with
 *     R[4] -= 8; fle_st(R[4], 8, <return address>); f_<va>(); R[4] += 8;
 * (the `ret` of the callee is a plain C return: it does not move R[4]). fle_call_guest does the same for a function whose VA the
 * runtime knows, through fle_dispatch (fle_dispatch.h), and follows the Microsoft x64 calling convention at the point of the
 * call: arguments in rcx, rdx, r8, r9 (R[1], R[2], R[8], R[9]); a 32-byte shadow space owned by the caller; R[4] 16-byte
 * aligned at the call instruction, so 16n+8 inside the callee; the result in rax (R[0]).
 *
 * Standard C only, so that it builds the same for native and for wasm. A stop inside a callback (fle_stop, fle_unsupported,
 * fle_dispatch_miss) aborts the whole run, as everywhere else: there is no unwinding. */
#ifndef FLE_CALLBACK_H
#define FLE_CALLBACK_H

#include <stddef.h>
#include <stdint.h>

/* Decision 0319 (O5; replaces the depth limit 16 of decision 0064, which was "a choice of this environment, not a value with a basis"): Windows limits the nesting of callbacks by the size of the
 * stack only, so the run stops when the stack of the running thread is nearly used up, by name and with the numbers:
 *   - the guest stack: rsp at the call less than FLE_CB_GUEST_RESERVE (a quarter of the stack when the stack is smaller than 4 x the reserve: the tests use 64 KiB) above the StackLimit of the thread's TEB
 *     (fle_teb_set_stack; the bounds of the memory the guest runs on);
 *   - the host (C) stack of the lifted code, in a wasm build: the shadow stack of the running thread (emscripten's STACK_SIZE for the first thread, FLE_THREAD_WASM_CSTACK_BYTES for the others) less
 *     FLE_CB_HOST_RESERVE. The most one nested level used in the measurement of decision 0319 (display change, 17 levels, wizard open) is 34,912 bytes of host stack (a window message sent from
 *     SetWindowPos) and 1,500 bytes of guest stack.
 * FLE_CB_MAX_DEPTH stays only as a backstop against a runaway recursion in a build that cannot ask its host stack (native tests); 17 levels were measured in a real run, a window tree is never near this deep. */
#define FLE_CB_MAX_DEPTH 256
#define FLE_CB_GUEST_RESERVE 65536    /* 64 KiB */
#define FLE_CB_HOST_RESERVE 524288    /* 512 KiB */
/* The bytes of the host (C) stack that the running thread has left. Defined by the file that knows the host (fle_thread_wasm.c in a wasm build); a build without it is not linked with a definition and the host check is skipped. */
size_t fle_cstack_left(void) __attribute__((weak));

/* The caller-allocated space for the four register arguments that the x64 convention requires at every call (Learn, x64 calling
 * convention: "The caller must always allocate sufficient space to store four register parameters"). */
#define FLE_CB_SHADOW_BYTES 32

/* The return address that the callee finds at [R[4]] when it starts. The lifted `ret` does not read it. It is not the address of
 * an instruction of the image or of any function entry (the image ends at 0x18C0000), so nothing can mistake it for one. After
 * the call the runtime checks that the slot still holds this value. */
#define FLE_CB_RETURN_ADDRESS UINT64_C(0xCB000000)

/* Calls the guest function whose entry VA is `va` with the four integer arguments (rcx, rdx, r8, r9) and returns rax (all 64 bits;
 * the caller takes the part that the return type of the callback has). Only functions that take at most four arguments are
 * supported (MONITORENUMPROC, WNDENUMPROC, TIMERPROC, WNDPROC all do).
 *
 * R[4] must be the guest stack pointer of the API that is running (it points at that API's return address): everything below it
 * is free and is used. On return R[4] has the value it had on entry; the volatile registers (rax, rcx, rdx, r8-r11, xmm0-5, the
 * flags) are whatever the callee left, as after any call, and the callee-saved registers (rbx, rbp, rsi, rdi, r12-r15) have their
 * entry values. The function stops the run, by name, when:
 *   - va is 0, or is not a function entry of the image (fle_dispatch_miss);
 *   - the guest stack or (wasm) the host stack of the running thread has less than FLE_CB_GUEST_RESERVE / FLE_CB_HOST_RESERVE left, or the nesting is deeper than FLE_CB_MAX_DEPTH (a backstop);
 *   - the callee returns with R[4] not where its return address is, with the return address slot changed, or with a callee-saved
 *     register changed (the callee broke the calling convention, or the lift did). xmm6-15 are not checked. */
uint64_t fle_call_guest(uint64_t va, uint64_t rcx, uint64_t rdx, uint64_t r8, uint64_t r9);

/* The current nesting (0 outside every callback). For tests and tools. */
unsigned fle_callback_depth(void);

/* Decision 0122: sets the nesting. Only fle_thread.c calls it (the depth belongs to the running guest thread and is switched with it). */
void fle_callback_depth_set(unsigned d);

#endif

/* The guest's structured exception handling (docs/decisions/0132-seh-s1.md, PROVISIONAL). Native; the wasm build compiles it and needs SJLJ=wasm + FLE_LANDING_FORCE.
 *
 * What runs: the guest's own language-specific handler (FLEngine's Delphi handler, RVA 0x112c0), called by this file as the operating system calls it (S1a):
 * RaiseException builds an EXCEPTION_RECORD, walks the guest frames with the UNWIND_INFO of the image's .pdata (virtual unwind), and calls the handler of each
 * frame that has UNW_FLAG_EHANDLER (pass 1); the handler, on a match, calls RtlUnwindEx, which walks the frames again and calls the handler of each frame that has
 * UNW_FLAG_UHANDLER with EXCEPTION_UNWINDING (pass 2; the guest's finally funclets run there), and then continues at TargetIp in the TargetFrame with the registers of
 * that frame (callee-saved ones unwound) and rax = ReturnValue. What is not modelled: the operating system's dispatcher frames (KiUserExceptionDispatcher), vectored
 * handlers, the debugger notification, hardware exceptions, an exception that crosses a host callback (fle_call_guest), a second exception while one is dispatched.
 *
 * The C side of "continue in the frame": fe-lift (--seh-handler) emits, for every function that has an except landing, an entry wrapper
 *   void f_P(void) { fle_seh_reg sr; fle_seh_enter(&sr, P); if (FLE_SEH_SETJMP(&sr) == 0) { fb_P(); } else { fle_seh_land(&sr); } fle_seh_leave(&sr); }
 * (fb_P is the body) and makes each landing L a function f_L. RtlUnwindEx finds the registration of the TargetFrame by (function, the stack address of its return
 * address), drops the registrations above it, and longjmps to it; fle_seh_land loads the registers and runs f_L inside the wrapper's C frame. */
#ifndef FLE_SEH_H
#define FLE_SEH_H

#include <stddef.h>
#include <stdint.h>

#include "fle_dispatch.h"

typedef struct fle_seh_reg {
    struct fle_seh_reg *prev;
    uint64_t func;      /* BeginAddress of the function's RUNTIME_FUNCTION as a VA = the entry VA */
    uint64_t entry_sp;  /* R[4] at the entry: the address of the return address */
    fle_landing *landing; /* fle_landing_top at the entry (decision 0100) */
    unsigned cbdepth;   /* fle_callback_depth() at the entry */
    size_t tdepth;      /* trace_depth() at the entry (the harness's shadow stack; 0 without one) */
    unsigned thread;    /* fle_thread_current() at the entry */
    uint64_t land_ip;   /* set by RtlUnwindEx before the jump */
    jmp_buf jb;
} fle_seh_reg;

#define FLE_SEH_SETJMP(r) _setjmp((r)->jb)

/* Registers the running function (entry VA `func`, the guest's R[4] at its entry). */
void fle_seh_enter(fle_seh_reg *r, uint64_t func);
static inline void fle_seh_leave(fle_seh_reg *r) { fle_seh_top = r->prev; }
/* Loads the registers of the landing (the unwound ones of the TargetFrame, rax = ReturnValue). */
void fle_seh_restore(fle_seh_reg *r);
static inline void fle_seh_land(fle_seh_reg *r) {
    fle_seh_restore(r);
    fle_dispatch(r->land_ip);
}

/* The state of the exception being dispatched on a thread (a pointer to a local of RaiseException's frame; NULL when none). Switched with the thread (fle_thread.c). */
struct fle_seh_disp;
__attribute__((weak)) struct fle_seh_disp *fle_seh_cur = 0;

/* kernel32!RaiseException / ntdll!RtlUnwindEx, reached through fle_proc.c (weak references there: a program that does not link fle_seh.c keeps the named stops of
 * decisions 0020 and 0078). Neither returns. */
void fle_seh_raise(void) __attribute__((noreturn));
void fle_seh_unwind_ex(void) __attribute__((noreturn));

/* For tests: the number of frames the last dispatch walked in pass 1, and the number of handler calls of the last pass 2. */
unsigned fle_seh_stat_frames(void);
unsigned fle_seh_stat_unwind_calls(void);

#endif

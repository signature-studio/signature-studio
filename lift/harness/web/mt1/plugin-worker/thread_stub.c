/* The JSPI thread switching of the runtime (runtime/fle_thread_wasm.c) has a JS side (harness/web/threads/fle_thread_lib.js) that the plugin worker does not have.
 * The plugin's own threads are run by the harness (vst3a_thread_entry, wraps.txt: CreateThread), so the scheduler does not switch threads here. If it ever does,
 * the run stops by name. */
#include <stdint.h>

#include "fle_env.h"

void fle_wasm_thread_init(void) { fle_stop("thread", "JSPI thread switching is not in the plugin worker"); }
void fle_wasm_thread_switch(unsigned from, unsigned to, uintptr_t sp_now, uintptr_t sp_new)
{
    (void)from; (void)to; (void)sp_now; (void)sp_new;
    fle_stop("thread", "JSPI thread switching is not in the plugin worker");
}
void fle_wasm_thread_exit(unsigned to, uintptr_t sp_new)
{
    (void)to; (void)sp_new;
    fle_stop("thread", "JSPI thread switching is not in the plugin worker");
}
void fle_wasm_thread_slot_new(unsigned idx) { (void)idx; fle_stop("thread", "JSPI thread switching is not in the plugin worker"); }

/* the trace of a raised exception (to the log only: the frame handlers of the runtime, fle_seh_raise, run after this) */
void __real_fle_imp_RaiseException(void);
#include <stdio.h>
#include "fle_runtime.h"
void __wrap_fle_imp_RaiseException(void)
{
    uint64_t args = R[9];
    printf("vst: RaiseException code %#x flags %#x nargs %u args %#llx ret %#llx\n", (unsigned)(uint32_t)R[1], (unsigned)(uint32_t)R[2], (unsigned)(uint32_t)R[8], (unsigned long long)args,
           (unsigned long long)fle_ld(R[4], 8));
    if (args && (uint32_t)R[8] >= 4) printf("vst:   info0 %#llx info1 %#llx info2 %#llx info3 %#llx\n", (unsigned long long)fle_ld(args, 8), (unsigned long long)fle_ld(args + 8, 8),
           (unsigned long long)fle_ld(args + 16, 8), (unsigned long long)fle_ld(args + 24, 8));
    __real_fle_imp_RaiseException();
}


/* Thread switching by JSPI (docs/decisions/0124-wasm-thread-switching.md, PROVISIONAL). Only the MEANS of switching of decision 0122's cooperative threads (fle_thread.c)
 * differs from native; every scheduling rule stays in fle_thread.c. Compiled on emscripten (linked with -sJSPI -sSUPPORT_LONGJMP=wasm and
 * --js-library harness/web/threads/fle_thread_lib.js); on native fle_thread.c switches threads itself.
 *
 * Every guest thread other than the first runs on its own JSPI stack (one WebAssembly.promising call of the export fle_wasm_thread_entry) and on its own region of the wasm
 * shadow stack (__stack_pointer is one global that all JSPI stacks share, so it is saved and restored at every switch). */
#ifndef FLE_THREAD_WASM_H
#define FLE_THREAD_WASM_H

#ifdef __EMSCRIPTEN__

#include <stddef.h>

/* The size of the shadow stack region of a thread other than the first (a choice without a source: it must hold the C frames of the lifted code of one thread; the first thread keeps
 * emscripten's stack, STACK_SIZE of the link). The low end holds a canary that every switch away from the thread checks. */
#define FLE_THREAD_WASM_CSTACK_BYTES (8u << 20)  /* decision 0140: 8 MiB (was 1 MiB: the C stack of guest thread 2 overflowed once the virtual clock ran with SEH landings; PM decision) */

/* Makes the shadow stack region of thread idx (1 when it succeeded, 0 when memory ran out). */
int fle_thread_wasm_stack_alloc(unsigned idx);
/* Frees it (called by the next thread after a thread finished, like free_pending on native). */
void fle_thread_wasm_stack_free(unsigned idx);
/* The thread `from` gives the processor to `to` (fle_thread.c switch_to made `to` current already). `from_done`: `from` has finished: its stacks are unwound and it never
 * comes back; otherwise this returns when some thread switches back to `from`. */
void fle_thread_wasm_transfer(unsigned from, unsigned to, int from_done);
/* The most bytes of its shadow stack region that thread idx used (measured when the thread ended and its region was freed; 0 before, and 0 unless built with FLE_THREAD_WASM_HIWATER). */
size_t fle_thread_wasm_hiwater(unsigned idx);
/* fle_thread.c: the body of the current thread on its new JSPI stack. */
void fle_thread_wasm_entry_cur(unsigned idx);

#endif
#endif

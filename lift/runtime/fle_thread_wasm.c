/* Thread switching by JSPI: the C side (docs/decisions/0124-wasm-thread-switching.md, PROVISIONAL; see fle_thread_wasm.h). The scheduling decision is made in fle_thread.c before
 * anything here is called; this file only moves the processor: it saves the shadow stack pointer, asks the JS scheduler (harness/web/threads/fle_thread_lib.js) to run thread `to`, and
 * the import suspends the calling JSPI stack. Nothing here reads the host: no clock, no real thread. Empty on native (the file is part of the glob fle_*.c of the build scripts). */
#ifdef __EMSCRIPTEN__
#include <emscripten.h>
#include <emscripten/stack.h>
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_env.h"
#include "fle_thread.h"
#include "fle_thread_wasm.h"

/* The JS library (harness/web/threads/fle_thread_lib.js). `fle_wasm_thread_switch` is an async import (WebAssembly.Suspending under -sJSPI): it returns when another thread has
 * switched back to `from`. `fle_wasm_thread_exit` throws a JS exception that ends the calling JSPI stack and hands over to `to`; it never returns. */
void fle_wasm_thread_init(void);
void fle_wasm_thread_switch(unsigned from, unsigned to, uintptr_t sp_now, uintptr_t sp_new);
void fle_wasm_thread_exit(unsigned to, uintptr_t sp_new) __attribute__((noreturn));
/* decision 0196: the table place `idx` is given to a new thread (a finished thread's place is used again once the table is full): the JS scheduler forgets that a thread was started there. */
void fle_wasm_thread_slot_new(unsigned idx);

#define CANARY UINT64_C(0x7468726561645F73) /* "thread_s" */
#define CANARY_BYTES 32u

static uint8_t *region[FLE_THREAD_MAX];   /* aligned start of the region of thread idx */
static void *raw_ptr[FLE_THREAD_MAX];     /* what malloc returned (for free) */
static size_t hiwater[FLE_THREAD_MAX];    /* the most bytes of the region any past or present user of the slot has used (measured at the end of the thread) */
#define FILL 0xA5u
/* FLE_THREAD_WASM_HIWATER (a measurement build): the region is filled at creation so that the end of the thread can tell how deep it went. Off by default: the fill touches the whole region. */
static int js_ready;

static uintptr_t top_of(unsigned idx)
{
    return ((uintptr_t)region[idx] + FLE_THREAD_WASM_CSTACK_BYTES) & ~(uintptr_t)15;
}

static void check_region(unsigned idx, uintptr_t sp_now, const char *what)
{
    uint64_t c;
    char why[200];

    if (idx == 0 || !region[idx]) return;
    memcpy(&c, region[idx], 8);
    if (c != CANARY) {
        snprintf(why, sizeof why, "the canary at the low end of the shadow stack of thread %u was overwritten (%s): the C frames of the thread are deeper than %u bytes", idx, what,
                 (unsigned)FLE_THREAD_WASM_CSTACK_BYTES);
        fle_stop("scheduler", why);
    }
    if (sp_now && (sp_now < (uintptr_t)region[idx] + CANARY_BYTES || sp_now > top_of(idx))) {
        snprintf(why, sizeof why, "the shadow stack pointer of thread %u (%#" PRIxPTR ") is outside its region [%#" PRIxPTR ", %#" PRIxPTR "] (%s)", idx, sp_now, (uintptr_t)region[idx], top_of(idx), what);
        fle_stop("scheduler", why);
    }
}

int fle_thread_wasm_stack_alloc(unsigned idx)
{
    uint8_t *p;

    if (!js_ready) {
        fle_wasm_thread_init();
        js_ready = 1;
    }
    p = (uint8_t *)malloc(FLE_THREAD_WASM_CSTACK_BYTES + 16u);
    if (!p) return 0;
    fle_wasm_thread_slot_new(idx);
    raw_ptr[idx] = p;
    p = (uint8_t *)(((uintptr_t)p + 15) & ~(uintptr_t)15);
    region[idx] = p;
    {
        uint64_t c = CANARY;
        unsigned i;

        for (i = 0; i < CANARY_BYTES; i += 8) memcpy(p + i, &c, 8);
    }
    return 1;
}

void fle_thread_wasm_stack_free(unsigned idx)
{
    if (idx == 0 || !region[idx]) return;
    check_region(idx, 0, "at the end of the thread");
    free(raw_ptr[idx]);
    raw_ptr[idx] = NULL;
    region[idx] = NULL;
}

size_t fle_thread_wasm_hiwater(unsigned idx)
{
    return idx < FLE_THREAD_MAX ? hiwater[idx] : 0;
}

void fle_thread_wasm_transfer(unsigned from, unsigned to, int from_done)
{
    uintptr_t sp_now = (uintptr_t)emscripten_stack_get_current();

    if (!js_ready) {
        fle_wasm_thread_init();
        js_ready = 1;
    }
    check_region(from, from_done ? 0 : sp_now, "at a switch away from the thread");
    if (to && !region[to]) fle_stop("scheduler", "a switch to a thread that has no shadow stack region");
    if (from_done) fle_wasm_thread_exit(to, to ? top_of(to) : 0);
    fle_wasm_thread_switch(from, to, sp_now, to ? top_of(to) : 0);
    /* The JS scheduler set __stack_pointer to the value saved above before it resumed this thread: a different value means a lost restore (the frames of the code that runs next would be
     * allocated on another thread's region). FLE_THREAD_WASM_NO_SPCHECK exists only to measure what the scenarios alone detect (mutation checks of decision 0124). */
    if ((uintptr_t)emscripten_stack_get_current() != sp_now) {
        char why[160];

        snprintf(why, sizeof why, "thread %u resumed with __stack_pointer %#" PRIxPTR " instead of the %#" PRIxPTR " it had when it switched away", from,
                 (uintptr_t)emscripten_stack_get_current(), sp_now);
        fle_stop("scheduler", why);
    }
}

/* The body of a thread: started by the JS scheduler with WebAssembly.promising on a new JSPI stack, after __stack_pointer was set to the top of the thread's region. */
EMSCRIPTEN_KEEPALIVE void fle_wasm_thread_entry(unsigned idx)
{
    fle_thread_wasm_entry_cur(idx);
}
#endif

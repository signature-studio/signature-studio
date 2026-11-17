/* Run-time dispatch for an indirect call whose target is not known statically.
 * fe-lift emits fle_dispatch_table (all function entries, sorted by VA) and
 * fle_dispatch_count into the generated translation unit. */
#ifndef FLE_DISPATCH_H
#define FLE_DISPATCH_H

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

typedef struct {
    uint64_t va;
    void (*fn)(void);
} fle_fn_entry;

extern const fle_fn_entry fle_dispatch_table[];
extern const uint64_t fle_dispatch_count;

/* Census hook (docs/decisions/0097-census-diagnostic.md). A diagnostic harness may set it to take over the call of a function that
 * fle_dispatch found: the hook gets the target VA and the function and must call the function itself. NULL (the default, and the
 * only value outside the census harness, lift/harness/census) means fn() is called directly, as before. The definition is weak in
 * every translation unit that includes this header, so that no .c file has to provide it (the same way as fle_teb in fle_vex.h). */
typedef void (*fle_dispatch_hook_fn)(uint64_t va, void (*fn)(void));
__attribute__((weak)) fle_dispatch_hook_fn fle_dispatch_hook = 0;

/* Decision 0123 (PROVISIONAL), wall 31: code that the guest wrote itself at run time (the thunk of Delphi's MakeObjectInstance in a VirtualAlloc block) is called by the guest's own `call [reg+disp]`. fle_win.c defines
 * fle_win_try_thunk (a weak reference here: a link without fle_win.c has none): it answers 1 and runs the thunk when `va` is exactly the `E8 rel32` slot of such a block (the same strict shape check as call_wndproc,
 * decision 0119), and 0 for anything else, which still stops below. The generated code is not changed: the hook is in the one function that every unresolved indirect call already ends in. */
int fle_win_try_thunk(uint64_t va) __attribute__((weak));

/* Decision 0147 (PROVISIONAL): the progress counter of the deterministic pseudo-preemption of the cooperative threads (fle_thread.c). Every indirect call that fle_dispatch finds is one unit of guest progress (an
 * indirect call is a guest instruction, so the count is the same for native, wasm, -O0 and -O2; the entry count of the instrumented builds is not). fle_thread.c arms the limit at the first CreateThread and, when the
 * count reaches it, fle_progress_slice() moves the virtual clock by one slice and lets a thread whose time has come run. Unarmed (the limit is UINT64_MAX, a link without fle_thread.c has no slice function) the
 * only effect is the increment. The definitions are weak in every translation unit that includes this header (the same way as fle_dispatch_hook above), the slice function is weak like fle_win_try_thunk. */
__attribute__((weak)) uint64_t fle_progress_count = 0;
__attribute__((weak)) uint64_t fle_progress_limit = UINT64_MAX;
void fle_progress_slice(void) __attribute__((weak));

/* A value that is not a known function entry. Stop instead of guessing (except the thunk above). */
static inline void fle_dispatch_miss(uint64_t va) {
    if (fle_win_try_thunk && fle_win_try_thunk(va)) return;
    fprintf(stderr, "fle: indirect call to %#llx is not a known function entry\n", (unsigned long long)va);
    abort();
}

/* A jump through an annotated table (fe-decode jumptab) with a selector outside the table. */
static inline void fle_jumptab_miss(uint64_t jmp_va, uint64_t sel) {
    fprintf(stderr, "fle: jump table at %#llx: selector %#llx is outside the table\n", (unsigned long long)jmp_va,
            (unsigned long long)sel);
    abort();
}

/* Landing of a setjmp of the guest (docs/decisions/0100-wall-lift-gaps.md). The guest's longjmp ends in `jmp [buf]`, which fe-lift emits as
 * fle_dispatch(<the saved return address>): the instruction after the call of setjmp, in the middle of the calling function. fe-lift makes that
 * address a function entry f_L (the code after the call) and emits the tail into f_L as
 *   { fle_landing lj; fle_landing_push(&lj, L, R[4]); if (FLE_LANDING_SETJMP(&lj) == 0) { f_L(); } else { f_L(); } fle_landing_pop(&lj); return; }
 * so that the C frame of the caller is live, registered by (L, rsp after the setjmp call returned), while f_L and everything below runs.
 * fle_dispatch(L) finds that registration (the guest's longjmp has restored rsp to the value registered), drops the registrations above it and
 * longjmps to it: the frames in between are discarded, as the guest's frames are, and f_L runs again from the restored state. The guest state is
 * all in R, FL, X and the guest memory, so nothing in a discarded C frame is needed. A function-entry hook (-finstrument-functions) does not see
 * the exits of the discarded frames, and nothing in a discarded frame is undone (the callback depth of fle_call_guest, host locks).
 *
 * Under emscripten the jump needs -sSUPPORT_LONGJMP=wasm (the build sets it, decision 0124). */
#include <setjmp.h>

/* Decision 0132: the newest registration of a function with an except landing (fle_seh.h); a decision-0100 landing remembers it, so that its longjmp drops the
 * registrations of the C frames it discards. */
struct fle_seh_reg;
__attribute__((weak)) struct fle_seh_reg *fle_seh_top = 0;

typedef struct fle_landing {
    struct fle_landing *prev;
    uint64_t va;
    uint64_t sp;
    struct fle_seh_reg *seh;
    jmp_buf jb;
} fle_landing;

#define FLE_LANDING_SETJMP(l) _setjmp((l)->jb)

#ifdef FLE_LOWREGS
#include "fle_lowregs.h"
#else
extern uint64_t R[16];
#endif
__attribute__((weak)) fle_landing *fle_landing_top = 0;

static inline void fle_landing_push(fle_landing *l, uint64_t va, uint64_t sp) {
    l->prev = fle_landing_top;
    l->va = va;
    l->sp = sp;
    l->seh = fle_seh_top;
    fle_landing_top = l;
}

static inline void fle_landing_pop(fle_landing *l) {
    fle_landing_top = l->prev;
}

/* Returns when no live landing is registered at `va`; jumps to the newest one whose rsp is R[4]; stops when there is one at `va` but none with
 * that rsp (a longjmp to a frame whose stack is not the one restored). */
static inline void fle_landing_try(uint64_t va) {
    fle_landing *l;
    int seen = 0;
    for (l = fle_landing_top; l; l = l->prev) {
        if (l->va != va) continue;
        if (l->sp == R[4]) {
            fle_landing_top = l;
            fle_seh_top = l->seh;
            _longjmp(l->jb, 1);
        }
        seen = 1;
    }
    if (seen) {
        fprintf(stderr, "fle: jump to %#llx, the landing of a setjmp of the guest, with rsp %#llx, which no live setjmp frame has\n",
                (unsigned long long)va, (unsigned long long)R[4]);
        abort();
    }
}

/* Calls the function whose entry VA is `va`. Any other value aborts.
 *
 * Decision 0163 (PROVISIONAL): the lookup of the function is a direct-mapped cache in front of the binary search. The result for every `va` is the one of the binary search alone (the same function, or the same
 * stop through fle_dispatch_miss): fle_dispatch_table is const, so a cache line is only ever a copy of a row of it. A hit is one index computation, two loads and a compare; a miss (the first call of a target, a
 * conflict, a `va` that is not an entry) runs fle_dispatch_find, the old binary search, out of line (one copy per translation unit), and fills the line. Only rows with a
 * non-null function are copied, so an all-zero line (the initial state) never hits, even for va == 0. The cache is shared by all guest threads of decision 0147/0122: they are cooperative (ucontext, or JSPI stacks
 * in the wasm build) and switch only inside the runtime's waiting/yielding calls and fle_progress_slice(), never between the two stores of a fill, so no lock is needed; real parallel threads would need one cache each.
 * FLE_DISPATCH_CACHE_BITS (12): log2 of the number of lines of 16 bytes (wasm32: also 16, the va is 64-bit).
 * The line is chosen by bits 4.. of the VA (va & 0xFFF0 is the byte offset of the line: one AND); measured on the real engine (docs/decisions/0163-dispatch-speedup.md): 84 distinct targets in the steady main window, 3,763 until the first idle, simulated miss rate of this index with 4096 lines 0.002% and 0.077%. */
#define FLE_DISPATCH_CACHE_BITS 12
#define FLE_DISPATCH_CACHE_HASH(va) (((va) >> 4) & ((1u << FLE_DISPATCH_CACHE_BITS) - 1u))
typedef struct {
    uint64_t va;
    void (*fn)(void);
} fle_dispatch_line;
__attribute__((weak)) fle_dispatch_line fle_dispatch_cache[1u << FLE_DISPATCH_CACHE_BITS];

/* The binary search of the old fle_dispatch: the row whose va is exactly `va`, or NULL. Fills the cache line of a found row that has a function. */
static inline __attribute__((noinline)) const fle_fn_entry *fle_dispatch_find(uint64_t va) {
    uint64_t lo = 0, hi = fle_dispatch_count;
    while (lo < hi) {
        uint64_t mid = lo + (hi - lo) / 2;
        if (fle_dispatch_table[mid].va < va) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    if (lo >= fle_dispatch_count || fle_dispatch_table[lo].va != va) return 0;
    if (fle_dispatch_table[lo].fn) {
        fle_dispatch_line *e = &fle_dispatch_cache[FLE_DISPATCH_CACHE_HASH(va)];
        e->fn = fle_dispatch_table[lo].fn;
        e->va = va;
    }
    return &fle_dispatch_table[lo];
}


/* The one place where every indirect call of the guest ends (the progress counter and the slice call below are counted by wall_fr20): landing, lookup of the function (cache + binary search, or the old binary search alone
 * with FLE_DISPATCH_BSEARCH), progress counter, hook, call. */
static inline void fle_dispatch(uint64_t va) {
    void (*fn)(void);
    if (__builtin_expect(fle_landing_top != 0, 0)) fle_landing_try(va);
    {
        const fle_dispatch_line *e = &fle_dispatch_cache[FLE_DISPATCH_CACHE_HASH(va)];
        if (__builtin_expect(e->va == va && e->fn != 0, 1)) {
            fn = e->fn;
        } else {
            const fle_fn_entry *row = fle_dispatch_find(va);
            if (!row) {
                fle_dispatch_miss(va);
                return;
            }
            fn = row->fn;
        }
    }
    if (__builtin_expect(++fle_progress_count >= fle_progress_limit, 0) && fle_progress_slice) fle_progress_slice();
    if (fle_dispatch_hook) {
        fle_dispatch_hook(va, fn);
        return;
    }
    fn();
}

#endif

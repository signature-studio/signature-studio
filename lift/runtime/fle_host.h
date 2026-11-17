/* Host portability layer for the runtime (docs/decisions/0045-wasm-port.md). Header only, so a test that links a
 * subset of the runtime sources needs nothing else.
 *
 * fle_host_tid(): the id the single-thread rule (decision 0017) compares. Native: the Linux tid, as before.
 * emscripten (__EMSCRIPTEN__, one thread, decision 0017): the constant FLE_HOST_WASM_TID (never 0, because the
 * owner slots use 0 for "not recorded yet"). The guest never sees this value: GetCurrentThreadId returns the
 * fixed FLE_EMULATED_TID (fle_imports.h, decision 0056) on both hosts.
 * A .c file that includes this header must define _GNU_SOURCE before its first system include (all of them do),
 * as syscall() needs it on native. */
#ifndef FLE_HOST_H
#define FLE_HOST_H

#include "fle_runtime.h"

/* FLE_VMEM_ARENA: VirtualAlloc, VirtualFree, VirtualProtect, VirtualQuery and the file-mapping objects do not use the
 * host's mmap, mprotect, madvise, memfd_create (emscripten has no usable ones: mprotect and madvise do nothing, an
 * anonymous mmap cannot be placed at an address, memfd_create is missing). Regions are cut out of one linear-memory
 * arena and the page protections are kept only in the runtime's tables (fle_sys.c, fle_vmap.c). Set automatically
 * for emscripten; a native build may set it by hand to run the same code (tests/wasm_port.rs). */
#if defined(__EMSCRIPTEN__) && !defined(FLE_VMEM_ARENA)
#define FLE_VMEM_ARENA 1
#endif

#ifdef __EMSCRIPTEN__
#define FLE_HOST_WASM_TID 1L
static inline long fle_host_tid(void)
{
    return FLE_HOST_WASM_TID;
}
#else
#include <sys/syscall.h>
#include <unistd.h>
static inline long fle_host_tid(void)
{
    return (long)syscall(SYS_gettid);
}
#endif

/* A guest size is 64 bits; the host allocators take a size_t, which is 32 bits on wasm32. Converting without this
 * test turns a 4 GiB + n request into an n-byte block (the host would not fail it, as a native run does). A size that
 * does not fit is an allocation the host cannot satisfy: the caller treats it as a failed malloc. Native: SIZE_MAX
 * is UINT64_MAX, the test is always true and the compiler drops it. */
static inline int fle_host_size_fits(uint64_t n)
{
    return n <= (uint64_t)SIZE_MAX;
}

#endif

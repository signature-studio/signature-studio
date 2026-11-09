/* Decision 0177 (srm1; decision 0243: always in the wasm build): 16-byte aligned malloc for the wasm build (an empty translation unit on native).
 *
 * Why: HeapAlloc, LocalAlloc, GlobalAlloc, CoTaskMemAlloc ... are plain malloc (fle_sys.c, fle_ole.c ...). On Windows x64 HeapAlloc returns 16-byte aligned blocks (Microsoft Learn,
 * HeapAlloc, Remarks: "The alignment of memory returned by HeapAlloc is MEMORY_ALLOCATION_ALIGNMENT in WinNT.h" = 16 for _WIN64; fetched 2026-11-03), and compiled code relies on it
 * (MOVAPS/MOVDQA on heap data; the lifted code aborts with "fle: misaligned 16-byte access", fle_sse.h). emscripten's dlmalloc returns 8-byte aligned blocks in wasm32 (a block at
 * 0x11cce758 was found with Serum2.vst3). glibc's malloc is 16-byte aligned on x86-64, so native builds are unaffected.
 *
 * How: user-defined malloc/free/calloc/realloc replace emscripten's (the documented interposition; the originals are emscripten_builtin_*). A block of
 * emscripten_builtin_memalign is freed by emscripten_builtin_free. The cost is dlmalloc's memalign path (larger blocks, a little slower than malloc). */
#ifdef __EMSCRIPTEN__
#include <stddef.h>
#include <stdlib.h>
#include <string.h>

void *emscripten_builtin_memalign(size_t align, size_t n);
void emscripten_builtin_free(void *p);
size_t malloc_usable_size(void *p);

void *malloc(size_t n) { return emscripten_builtin_memalign(16, n ? n : 1); }
void free(void *p) { emscripten_builtin_free(p); }
void *calloc(size_t a, size_t b)
{
    size_t n;
    void *p;
    if (__builtin_mul_overflow(a, b, &n)) return NULL;
    p = malloc(n);
    if (p) memset(p, 0, n);
    return p;
}
void *realloc(void *p, size_t n)
{
    void *q;
    size_t old;
    if (!p) return malloc(n);
    if (n == 0) { emscripten_builtin_free(p); return NULL; }
    old = malloc_usable_size(p);
    if (n <= old) return p;
    q = malloc(n);
    if (!q) return NULL;
    memcpy(q, p, old);
    emscripten_builtin_free(p);
    return q;
}
#else
typedef int fle_malloc16_off;
#endif

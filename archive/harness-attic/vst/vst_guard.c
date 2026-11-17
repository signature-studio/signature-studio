/* Debug allocator (decision 0156; build_vst_native.sh GUARD=1): every block of the runtime/harness gets its own pages, ends exactly at an inaccessible page, and is left
 * inaccessible after free. A guest store past the end of a block (or a use after free) faults at once, where glibc's malloc would only fail later ("corrupted top size").
 * Linked with -Wl,--wrap=malloc,calloc,realloc,free,aligned_alloc only; libc's own allocations are not affected. */
#define _GNU_SOURCE
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>

#define PG 4096u

void *__wrap_aligned_alloc(size_t al, size_t n)
{
    size_t rn = (n + 15u) & ~(size_t)15u;
    size_t data = (rn + 16u + PG - 1u) / PG * PG;
    uint8_t *base = mmap(NULL, data + PG, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    uint8_t *guard, *ret;
    if (base == MAP_FAILED) return NULL;
    guard = base + data;
    mprotect(guard, PG, PROT_NONE);
    if (al < 16) al = 16;
    ret = (uint8_t *)(((uintptr_t)(guard - rn)) & ~(uintptr_t)(al - 1u));
    if (ret - 16 < base) { munmap(base, data + PG); return NULL; }
    memcpy(ret - 16, &n, 8);
    memcpy(ret - 8, &base, 8);
    return ret;
}
void *__wrap_malloc(size_t n) { return __wrap_aligned_alloc(16, n); }
void *__wrap_calloc(size_t a, size_t b) { return __wrap_aligned_alloc(16, a * b); }
void __wrap_free(void *p)
{
    uint8_t *base;
    size_t n, rn, data;
    if (!p) return;
    memcpy(&n, (uint8_t *)p - 16, 8);
    memcpy(&base, (uint8_t *)p - 8, 8);
    rn = (n + 15u) & ~(size_t)15u;
    data = (rn + 16u + PG - 1u) / PG * PG;
    mprotect(base, data, PROT_NONE);
}
void *__wrap_realloc(void *p, size_t n)
{
    void *q;
    size_t old;
    if (!p) return __wrap_malloc(n);
    memcpy(&old, (uint8_t *)p - 16, 8);
    q = __wrap_malloc(n);
    if (!q) return NULL;
    memcpy(q, p, old < n ? old : n);
    __wrap_free(p);
    return q;
}

/* Two-module harness main (stage 2/3 of docs/investigations/two-module-design.md section 7; decisions 0080 and 0082).
 * The same source builds the native harness and the wasm harness (like lift/harness/main.c and lift/harness/wasm/main.c, which
 * are not changed). It maps FLEngine_x64.dll at its ImageBase and the rebased real module (a file made by `fe-pe rebase`, at the
 * base that the link names in fle_realmod_table) at that base, then calls the DllMain-equivalent entry of FLEngine as the
 * single-module harness does. Loading the real module is done by the guest's own LoadLibraryW (fle_realmod.c).
 * Usage: main_two <FLEngine dll> <real module file> [entry_va_hex] */
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifndef __EMSCRIPTEN__
#include <sys/mman.h>
#include <sys/resource.h>
#else
#include "fle_host.h"
#endif

#include "a4.h"
#include "boot.h"
#include "census.h"
#include "fle_dispatch.h"
#include "fle_realmod.h"
#include "fle_runtime.h"
#include "vfs_seed.h"

void fle_teb_set_stack(uint64_t lo, uint64_t hi); /* runtime/fle_vex.h (weak definition in every generated unit), decision 0076 */

static void (*find_fn(uint64_t va))(void) {
    for (uint64_t i = 0; i < fle_dispatch_count; i++) {
        if (fle_dispatch_table[i].va == va) return fle_dispatch_table[i].fn;
    }
    return NULL;
}

int main(int argc, char **argv) {
    const char *path = argc > 1 ? argv[1] : "FLEngine_x64.dll";
    const char *real = argc > 2 ? argv[2] : NULL;
    harness_image img, rimg;
    char err[256];
    uint64_t entry;
    void (*fn)(void);
    uint8_t *stk;
    /* N real modules (lift/out-three/harness-two): argv[2] is one file or several separated by ':'. One file: the output is the one of
     * the one-module main_two.c. MAIN_REAL_MAX only sizes these arrays. It is larger than the RM_MAX (8) of fle_realmod.c on purpose, as
     * in harness/two/main_two.c (decision 0106: it was 4 = RM_MAX, so a 5th real module file was refused here before the runtime saw it);
     * more files than the runtime holds are mapped and then stopped by the runtime by name. */
    enum { MAIN_REAL_MAX = 16 };
    char *rpath[MAIN_REAL_MAX], *rcopy = NULL;
    uint64_t rimg_lo[MAIN_REAL_MAX], rimg_sz[MAIN_REAL_MAX]; /* census: the images whose memory the report may read */
    int nreal = 0;
    uint64_t span_lo = 0, span_hi = 0;
    int census = 0;

    if (!real) {
        fprintf(stderr, "usage: %s <FLEngine dll> <real module file>[:<real module file>...] [entry_va_hex]\n", argv[0]);
        return 64;
    }
    rcopy = strdup(real);
    if (!rcopy) {
        perror("strdup");
        return 2;
    }
    for (char *tok = strtok(rcopy, ":"); tok; tok = strtok(NULL, ":")) {
        if (nreal == MAIN_REAL_MAX) {
            fprintf(stderr, "more than %d real module files in %s\n", MAIN_REAL_MAX, real);
            return 64;
        }
        rpath[nreal++] = tok;
    }
    if (nreal == 0) {
        fprintf(stderr, "no real module file in \"%s\"\n", real);
        return 64;
    }
#ifndef __EMSCRIPTEN__
    {
        struct rlimit rl = {HARNESS_MAX_AS_BYTES, HARNESS_MAX_AS_BYTES};
        if (setrlimit(RLIMIT_AS, &rl) != 0) {
            perror("setrlimit");
            return 2;
        }
    }
#endif
    trace_init();
    if (image_load(path, &img, err, sizeof err) != 0) {
        fprintf(stderr, "image: %s\n", err);
        return 2;
    }
    a4_set_image(img.base, img.size); /* the a4 survey reads FLEngine's InitContext only */
    span_lo = img.base;
    span_hi = img.base + img.size;
    printf("image: base=%#llx size=%#llx entry=%#llx imports=%zu exports=%zu\n",
           (unsigned long long)img.base, (unsigned long long)img.size,
           (unsigned long long)img.entry, img.nimports, img.nexports);

    for (int k = 0; k < nreal; k++) {
        if (image_load(rpath[k], &rimg, err, sizeof err) != 0) {
            fprintf(stderr, "real module image: %s (%s)\n", err, rpath[k]);
            return 2;
        }
        printf("real module image: base=%#llx size=%#llx entry=%#llx imports(cumulative)=%zu exports(cumulative)=%zu\n",
               (unsigned long long)rimg.base, (unsigned long long)rimg.size,
               (unsigned long long)rimg.entry, rimg.nimports, rimg.nexports);
        rimg_lo[k] = rimg.base;
        rimg_sz[k] = rimg.size;
        if (rimg.base < span_lo) span_lo = rimg.base;
        if (rimg.base + rimg.size > span_hi) span_hi = rimg.base + rimg.size;
        {
            int ok = 0;
            for (uint64_t i = 0; i < fle_realmod_count; i++) {
                if (fle_realmod_table[i].base == rimg.base) {
                    ok = 1;
                    printf("real module table: name=%s base=%#llx\n", fle_realmod_table[i].name,
                           (unsigned long long)fle_realmod_table[i].base);
                }
            }
            if (!ok) {
                fprintf(stderr, "the file %s is mapped at %#llx, which no entry of fle_realmod_table names\n", rpath[k],
                        (unsigned long long)rimg.base);
                return 2;
            }
        }
    }
    {
        /* every table entry must have been mapped (fle_realmod.c would otherwise stop at the first LoadLibrary of it) */
        if (fle_realmod_count != (uint64_t)nreal) {
            fprintf(stderr, "fle_realmod_table has %llu entries and %d real module files were mapped\n",
                    (unsigned long long)fle_realmod_count, nreal);
            return 2;
        }
    }
    (void)span_lo;
    (void)span_hi;

    entry = argc > 3 ? strtoull(argv[3], NULL, 0) : img.entry;
    fn = find_fn(entry);
    if (!fn) {
        printf("entry %#llx is not in fle_dispatch_table (not a lifted function entry)\n",
               (unsigned long long)entry);
        return 2;
    }

    /* FLE_VFS_SEED=<list> (decision 0110, provisional): product files of the installer written to the virtual volume before the guest runs.
     * Unset: vfs_seed_from_env returns 0 and does nothing. R[] is put back as it was. */
    if (vfs_seed_from_env() != 0) return 2;

#ifndef __EMSCRIPTEN__
    stk = mmap(NULL, HARNESS_STACK_BYTES, PROT_READ | PROT_WRITE,
               MAP_PRIVATE | MAP_ANONYMOUS | MAP_NORESERVE, -1, 0);
    if (stk == MAP_FAILED) {
        perror("mmap stack");
        return 2;
    }
#else
    stk = aligned_alloc(4096, HARNESS_STACK_BYTES); /* page-aligned lower bound: StackLimit is that bound (decision 0076) */
    if (!stk) {
        perror("aligned_alloc stack");
        return 2;
    }
#endif
    R[4] = ((uint64_t)(uintptr_t)stk + HARNESS_STACK_BYTES - 64) & ~UINT64_C(15);
    fle_teb_set_stack((uint64_t)(uintptr_t)stk, (uint64_t)(uintptr_t)stk + HARNESS_STACK_BYTES); /* decision 0076 */
    R[1] = img.base;
    R[2] = 1;
    R[8] = 0;
    printf("calling entry %s rcx=%#llx rdx=1 r8=0 rsp=%#llx\n",
           image_export_name(entry) ? image_export_name(entry) : "(no export name)",
           (unsigned long long)R[1], (unsigned long long)R[4]);
    fflush(stdout);

    /* census mode (decision 0097): FLE_CENSUS=<file>. Unset: census_begin returns 0 and nothing below is called. */
    census = census_begin(img.base, img.size);
    if (census) {
        for (int k = 0; k < nreal; k++) census_add_image(rimg_lo[k], rimg_sz[k]);
    }
    R[4] -= 8;
    fle_st(R[4], 8, HARNESS_SENTINEL_RET);
    fn();
    R[4] += 8;
    if (census) census_end("returned from entry");

    trace_report("returned from entry");
    printf("rax=%#llx\n", (unsigned long long)R[0]);
    return 0;
}

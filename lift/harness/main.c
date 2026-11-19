/* Native harness for the fe-lift output. Maps FLEngine_x64.dll at its ImageBase,
 * sets up a stack, and calls the DllMain-equivalent entry (DLL_PROCESS_ATTACH)
 * through the generated function. Usage: flengine_harness <dll> [entry_va_hex] */
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <sys/mman.h>
#include <sys/resource.h>

#include "a4.h"
#include "boot.h"
#include "fle_dispatch.h"
#include "fle_runtime.h"

void a4_set_image(uint64_t base, uint64_t size) __attribute__((weak));
/* census/census.c (decision 0097): weak, so that a build without it (harness/build.sh) is unchanged. With FLE_CENSUS unset
 * census_begin returns 0 and nothing else is called: the run is the one it always was. */
int census_begin(uint64_t base, uint64_t size) __attribute__((weak));
void census_end(const char *how) __attribute__((weak));
void fle_teb_set_stack(uint64_t lo, uint64_t hi); /* runtime/fle_vex.h (weak definition in every generated unit), decision 0076 */

static void (*find_fn(uint64_t va))(void) {
    for (uint64_t i = 0; i < fle_dispatch_count; i++) {
        if (fle_dispatch_table[i].va == va) return fle_dispatch_table[i].fn;
    }
    return NULL;
}

int main(int argc, char **argv) {
    const char *path = argc > 1 ? argv[1] : "FLEngine_x64.dll";
    struct rlimit rl = {HARNESS_MAX_AS_BYTES, HARNESS_MAX_AS_BYTES};
    harness_image img;
    char err[256];
    uint64_t entry;
    void (*fn)(void);
    uint8_t *stk;
    int census = 0;

    if (setrlimit(RLIMIT_AS, &rl) != 0) {
        perror("setrlimit");
        return 2;
    }
    trace_init();
    if (image_load(path, &img, err, sizeof err) != 0) {
        fprintf(stderr, "image: %s\n", err);
        return 2;
    }
    if (a4_set_image) a4_set_image(img.base, img.size); /* weak: the old harness/build.sh links no a4.c */
    printf("image: base=%#llx size=%#llx entry=%#llx imports=%zu exports=%zu\n",
           (unsigned long long)img.base, (unsigned long long)img.size,
           (unsigned long long)img.entry, img.nimports, img.nexports);

    entry = argc > 2 ? strtoull(argv[2], NULL, 0) : img.entry;
    fn = find_fn(entry);
    if (!fn) {
        printf("entry %#llx is not in fle_dispatch_table (not a lifted function entry)\n",
               (unsigned long long)entry);
        return 2;
    }

    stk = mmap(NULL, HARNESS_STACK_BYTES, PROT_READ | PROT_WRITE,
               MAP_PRIVATE | MAP_ANONYMOUS | MAP_NORESERVE, -1, 0);
    if (stk == MAP_FAILED) {
        perror("mmap stack");
        return 2;
    }
    R[4] = ((uint64_t)(uintptr_t)stk + HARNESS_STACK_BYTES - 64) & ~UINT64_C(15);
    fle_teb_set_stack((uint64_t)(uintptr_t)stk, (uint64_t)(uintptr_t)stk + HARNESS_STACK_BYTES); /* decision 0076 */
    if (getenv("FLE_STACK_SKEW")) R[4] -= 8; /* diagnostic only: shifts rsp by 8 */
    R[1] = img.base;
    R[2] = 1;
    R[8] = 0;
    printf("calling entry %s rcx=%#llx rdx=1 r8=0 rsp=%#llx\n",
           image_export_name(entry) ? image_export_name(entry) : "(no export name)",
           (unsigned long long)R[1], (unsigned long long)R[4]);
    fflush(stdout);

    if (census_begin) census = census_begin(img.base, img.size);
    R[4] -= 8;
    fle_st(R[4], 8, HARNESS_SENTINEL_RET);
    fn();
    R[4] += 8;
    if (census && census_end) census_end("returned from entry");

    trace_report("returned from entry");
    printf("rax=%#llx\n", (unsigned long long)R[0]);
    return 0;
}

/* wasm harness main (decision 0045; lift/harness/main.c is the native one: it uses setrlimit and mmap, which wasm
 * does not have). Loads the DLL (node, -sNODERAWFS=1) into linear memory at its ImageBase and calls the
 * DllMain-equivalent entry the same way as the native harness, then prints the same stop report (trace.c).
 * Usage (node): node run_node.cjs <dll> [entry_va_hex] */
#include <stdio.h>
#include <stdlib.h>

#include "a4.h"
#include "boot.h"
#include "fle_dispatch.h"
#include "fle_host.h"
#include "fle_runtime.h"

void fle_teb_set_stack(uint64_t lo, uint64_t hi); /* runtime/fle_vex.h (weak definition in every generated unit), decision 0076 */

static void (*find_fn(uint64_t va))(void) {
    for (uint64_t i = 0; i < fle_dispatch_count; i++) {
        if (fle_dispatch_table[i].va == va) return fle_dispatch_table[i].fn;
    }
    return NULL;
}

int main(int argc, char **argv) {
    const char *path = argc > 1 ? argv[1] : "FLEngine_x64.dll";
    harness_image img;
    char err[256];
    uint64_t entry;
    void (*fn)(void);
    uint8_t *stk;

    trace_init();
    if (image_load(path, &img, err, sizeof err) != 0) {
        fprintf(stderr, "image: %s\n", err);
        return 2;
    }
    a4_set_image(img.base, img.size);
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

    stk = aligned_alloc(4096, HARNESS_STACK_BYTES); /* page-aligned lower bound: StackLimit is that bound (decision 0076) */
    if (!stk) {
        perror("aligned_alloc stack");
        return 2;
    }
    R[4] = ((uint64_t)(uintptr_t)stk + HARNESS_STACK_BYTES - 64) & ~UINT64_C(15);
    fle_teb_set_stack((uint64_t)(uintptr_t)stk, (uint64_t)(uintptr_t)stk + HARNESS_STACK_BYTES);
    R[1] = img.base;
    R[2] = 1;
    R[8] = 0;
    printf("calling entry %s rcx=%#llx rdx=1 r8=0 rsp=%#llx\n",
           image_export_name(entry) ? image_export_name(entry) : "(no export name)",
           (unsigned long long)R[1], (unsigned long long)R[4]);
    fflush(stdout);

    R[4] -= 8;
    fle_st(R[4], 8, HARNESS_SENTINEL_RET);
    fn();
    R[4] += 8;

    trace_report("returned from entry");
    printf("rax=%#llx\n", (unsigned long long)R[0]);
    return 0;
}

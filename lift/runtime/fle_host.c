/* Host portability layer, the parts that are not header-only (docs/decisions/0045-wasm-port.md).
 * Everything here is used only by a build that sets FLE_CHECK_HI32 (wasm, or a test); a native
 * build links it and never calls it. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

#include "fle_runtime.h"
#include "fle_host.h"

/* A stop with a name that the harness report classifies (trace.c: "fle_addr_hi32"). */
void fle_addr_fault(uint64_t a, unsigned n, int store)
{
    fprintf(stderr, "fle: guest address %#llx (%u-byte %s) has bits above 32 set: not representable in wasm32 (a native run faults here) [fle_addr_hi32]\n",
            (unsigned long long)a, n, store ? "store" : "load");
    abort();
}


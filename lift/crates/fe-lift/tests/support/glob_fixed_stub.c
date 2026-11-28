/* Test-only definitions of the fixed-block functions of runtime/fle_glob.c (#1382). fle_imports.c calls them for LocalAlloc and FormatMessageW. The tests that link this file do not
 * link fle_glob.c (it needs fle_env.c, and their programs define fle_stop themselves) and run no LocalAlloc, FormatMessageW or GlobalAlloc, so no block is registered or released.
 * Linked only by those tests, never into a product build. */
#include <stdint.h>

void fle_glob_fixed_add(void *p)
{
    (void)p;
}

int fle_glob_fixed_release(uint64_t h)
{
    (void)h;
    return 0;
}

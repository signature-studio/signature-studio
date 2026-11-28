/* Test-only definitions of the page's clipboard library (#1389 and #1406, fle_clip.c). The browser build defines all three in harness/web/clip/fle_clip_lib.js (an emscripten JS library);
 * a native test has no page and no text: a SetClipboardData text is dropped, a read answers NULL, which fle_clip.c takes as "no text", and the format probe answers 0 (no text).
 * Linked by the tests that compile runtime/fle_*.c, never into a product build. */
#include <stddef.h>
#include <stdint.h>

void fle_clip_host_write(const void *units, uint32_t bytes)
{
    (void)units;
    (void)bytes;
}

void *fle_clip_host_read(void)
{
    return NULL;
}

int fle_clip_host_probe(void)
{
    return 0;
}

/* Decision 0209 (crash1): shell32's file drag-and-drop family. See fle_crash1_drag.h for the pages. */
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_win.h"
#include "fle_crash1_drag.h"

#define FLE_DROP_MAX 32u

static struct { uint64_t h; void (*release)(uint64_t); } fle_drops[FLE_DROP_MAX];

int fle_drop_register(uint64_t hdrop, void (*release)(uint64_t))
{
    unsigned i;

    if (hdrop == 0) return 0;
    for (i = 0; i < FLE_DROP_MAX; i++)
        if (fle_drops[i].h == hdrop) { fle_drops[i].release = release; return 1; }
    for (i = 0; i < FLE_DROP_MAX; i++)
        if (fle_drops[i].h == 0) { fle_drops[i].h = hdrop; fle_drops[i].release = release; return 1; }
    return 0;
}

int fle_drop_is_live(uint64_t hdrop)
{
    unsigned i;

    for (i = 0; i < FLE_DROP_MAX; i++)
        if (hdrop != 0 && fle_drops[i].h == hdrop) return 1;
    return 0;
}

/* The DROPFILES block at `hdrop` (read through fle_ld, so the guest address rules of the build apply). */
typedef struct { uint32_t pfiles; int32_t x, y; uint32_t fnc, fwide; } fle_dropfiles;

static void fle_read_dropfiles(uint64_t h, fle_dropfiles *d)
{
    d->pfiles = (uint32_t)fle_ld(h, 4);
    d->x = (int32_t)(uint32_t)fle_ld(h + 4, 4);
    d->y = (int32_t)(uint32_t)fle_ld(h + 8, 4);
    d->fnc = (uint32_t)fle_ld(h + 12, 4);
    d->fwide = (uint32_t)fle_ld(h + 16, 4);
}

/* The unit (or byte) at index k of the file list. */
static uint32_t fle_drop_unit(uint64_t h, const fle_dropfiles *d, uint64_t k)
{
    return d->fwide ? (uint32_t)fle_ld(h + d->pfiles + 2u * k, 2) : (uint32_t)fle_ld(h + d->pfiles + k, 1);
}

/* shell32!DragAcceptFiles: sets or clears WS_EX_ACCEPTFILES on the window; a handle that is not a live window changes nothing. No result, no error. */
void fle_imp_DragAcceptFiles(void)
{
    uint64_t hwnd = R[1];
    uint32_t accept = (uint32_t)R[2] != 0;

    (void)fle_win_set_exstyle_bits(hwnd, FLE_WS_EX_ACCEPTFILES, accept ? FLE_WS_EX_ACCEPTFILES : 0u);
}

void fle_imp_DragQueryFileW(void)
{
    static const char api[] = "DragQueryFileW";
    uint64_t h = R[1];
    uint32_t idx = (uint32_t)R[2], cch = (uint32_t)R[9];
    uint64_t out = R[8], k, start = 0, len = 0, count = 0;
    fle_dropfiles d;
    int found = 0;

    if (!fle_drop_is_live(h)) {
        R[0] = 0;
        return;
    }
    fle_read_dropfiles(h, &d);
    /* walk the double null-terminated list: names are runs up to a NUL; an empty run ends the list */
    for (k = 0;; ) {
        uint64_t b = k;
        uint32_t u;

        while ((u = fle_drop_unit(h, &d, k)) != 0) {
            if (!d.fwide && u >= 0x80u) fle_stop(api, "an ANSI file list with a byte of 0x80 or above: the code page conversion of the list is not done (DROPFILES.fWide is 0)");
            k++;
        }
        if (k == b) break;
        if (count == idx) { start = b; len = k - b; found = 1; }
        count++;
        k++;
    }
    if (idx == 0xFFFFFFFFu) {
        R[0] = (uint32_t)count;
        return;
    }
    if (!found) {
        R[0] = 0;
        return;
    }
    if (out == 0) {
        R[0] = (uint32_t)len;
        return;
    }
    if (cch == 0) {
        R[0] = 0;
        return;
    }
    {
        uint64_t n = len < (uint64_t)cch - 1u ? len : (uint64_t)cch - 1u, j;

        for (j = 0; j < n; j++) fle_st(out + 2u * j, 2, fle_drop_unit(h, &d, start + j));
        fle_st(out + 2u * n, 2, 0);
        R[0] = (uint32_t)n;
    }
}

void fle_imp_DragQueryPoint(void)
{
    uint64_t h = R[1], ppt = R[2];
    fle_dropfiles d;

    if (!fle_drop_is_live(h) || ppt == 0) {
        R[0] = 0;
        return;
    }
    fle_read_dropfiles(h, &d);
    fle_st(ppt, 4, (uint32_t)d.x);
    fle_st(ppt + 4, 4, (uint32_t)d.y);
    R[0] = d.fnc ? 0u : 1u;
}

void fle_imp_DragFinish(void)
{
    uint64_t h = R[1];
    unsigned i;

    for (i = 0; i < FLE_DROP_MAX; i++) {
        if (h != 0 && fle_drops[i].h == h) {
            void (*rel)(uint64_t) = fle_drops[i].release;
            fle_drops[i].h = 0;
            fle_drops[i].release = NULL;
            if (rel) rel(h);
            return;
        }
    }
}

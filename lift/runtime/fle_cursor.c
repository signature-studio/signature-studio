/* The virtual standard cursor (docs/decisions/0059-virtual-cursors.md; the pages read are listed there and in
 * docs/investigations/loadcursor-caller.md). Stateless. One identifier is answered; every other argument stops the run
 * through fle_stop (fle_env.h). Nothing here reads the host: no cursor, no theme, no mouse setting, no display, no resource
 * file, no clock, no thread. */
#include <stdint.h>
#include <stdio.h>

#include "fle_runtime.h"
#include "fle_env.h"
#include "fle_cursor.h"
#include "fle_monitor.h"

/* The thirteen identifiers of the guest's LoadImageW(NULL, ...) calls (decision 0061). */
static const uint32_t image_ids[] = {
    FLE_CURSOR_IDC_ARROW, FLE_CURSOR_IDC_IBEAM, FLE_CURSOR_IDC_WAIT, FLE_CURSOR_IDC_CROSS, FLE_CURSOR_IDC_UPARROW,
    FLE_CURSOR_IDC_SIZENWSE, FLE_CURSOR_IDC_SIZENESW, FLE_CURSOR_IDC_SIZEWE, FLE_CURSOR_IDC_SIZENS, FLE_CURSOR_IDC_SIZEALL,
    FLE_CURSOR_IDC_NO, FLE_CURSOR_IDC_APPSTARTING, FLE_CURSOR_IDC_HELP,
};

static uint64_t std_handle(uint32_t idc)
{
    return FLE_CURSOR_HANDLE_BASE + 4u * (uint64_t)(idc - FLE_CURSOR_IDC_ARROW + 1u);
}

uint64_t fle_cursor_handle_of(uint32_t idc)
{
    if (idc != FLE_CURSOR_IDC_ARROW) return 0;
    return std_handle(idc);
}

uint64_t fle_cursor_image_handle_of(uint32_t idc)
{
    size_t i;
    for (i = 0; i < sizeof image_ids / sizeof image_ids[0]; i++) {
        if (image_ids[i] == idc) return std_handle(idc);
    }
    return 0;
}

uint32_t fle_cursor_image_idc_of(uint64_t h)
{
    size_t i;
    for (i = 0; i < sizeof image_ids / sizeof image_ids[0]; i++) {
        if (h == std_handle(image_ids[i])) return image_ids[i];
    }
    return 0;
}

/* The form of the handles that LoadImageW answers for a module's cursor (decisions 0061 and 0104). This file does not use the module that makes them (the source test of tests/cursor.rs
 * forbids it), so the three numbers are written here and tests/wall_c1.rs pins them to the constants of the other module's header: base 0x7FDA0000, stride 0x1000 per real module
 * (index 0 = the lifted DLL, 1..8 = a real module), 4 * (id - 32762 + 1) with id 32762 ..= 32767. */
#define SHARED_MODULE_BASE   UINT64_C(0x7FDA0000)
#define SHARED_MODULE_STRIDE UINT64_C(0x1000)
#define SHARED_MODULE_IDS    6u

#define SHARED_NAMED_FIRST   0x100u
#define SHARED_NAMED_COUNT   0x100u

int fle_cursor_is_shared_handle(uint64_t h)
{
    uint64_t off, q;

    if (fle_cursor_image_idc_of(h) != 0) return 1;
    if (h <= SHARED_MODULE_BASE) return 0;
    off = h - SHARED_MODULE_BASE;
    if (off / SHARED_MODULE_STRIDE > 8u || off % 4u != 0) return 0;
    q = (off % SHARED_MODULE_STRIDE) / 4u;
    if (q >= SHARED_NAMED_FIRST && q < SHARED_NAMED_FIRST + SHARED_NAMED_COUNT) return 1;   /* decision 0125: a cursor of LoadCursorW(hInstance, L"name") */
    return q >= 1u && q <= SHARED_MODULE_IDS;
}

uint32_t fle_cursor_idc_of(uint64_t h)
{
    if (h == fle_cursor_handle_of(FLE_CURSOR_IDC_ARROW)) return FLE_CURSOR_IDC_ARROW;
    return 0;
}

/* user32!LoadCursorW: HCURSOR f(HINSTANCE hInstance rcx, LPCWSTR lpCursorName rdx). Learn (LoadCursorW): with hInstance NULL,
 * lpCursorName is the IDC_ identifier of a predefined system cursor; the return value is the handle to the cursor, NULL on
 * failure; "The LoadCursor function loads the cursor resource only if it has not been loaded; otherwise, it retrieves the
 * handle to the existing resource" (one handle for every call). Learn (About Cursors): IDC_ARROW is MAKEINTRESOURCE(32512);
 * LoadCursor "always uses LR_SHARED internally regardless of the instance parameter", so the handle is shared and is not
 * passed to DestroyCursor by its owner. Only IDC_ARROW is answered (the one identifier of the guest's static call sites).
 * A non-NULL hInstance needs the resources of a module (the guest's other call sites do that, with a string name); a string
 * name and another identifier are not answered: all stop. The whole 64 bits of rcx and rdx are compared (a value of rdx that
 * has bits above the low 16 is a pointer to a name, not an identifier: Learn IS_INTRESOURCE). The last error is left alone. */
/* Decision 0125 (PROVISIONAL), wall 54: a non-NULL hInstance loads the cursor of the guest's own resources by name or ordinal (fle_res.c; a weak reference: a build without it keeps the stop below). */
extern int fle_cursor_load_named(const char *api, uint64_t inst, uint64_t name, uint64_t *handle) __attribute__((weak));

void fle_imp_LoadCursorW(void)
{
    static const char api[] = "LoadCursorW";
    uint64_t inst = R[1], name = R[2], h;
    char why[160];

    if (inst != 0) {
        if (fle_cursor_load_named == NULL) fle_stop(api, "hInstance is not NULL: cursor resources of a module are not implemented (decision 0059; only IDC_ARROW is answered)");
        (void)fle_cursor_load_named(api, inst, name, &h);
        R[0] = h;
        return;
    }
    if ((name >> 16) != 0) fle_stop(api, "lpCursorName is a pointer to a name, not an integer identifier: not answered (decision 0059)");
    if (name != FLE_CURSOR_IDC_ARROW) {
        snprintf(why, sizeof why, "lpCursorName MAKEINTRESOURCE(%u) is not IDC_ARROW (32512): not answered (decision 0059)", (unsigned)name);
        fle_stop(api, why);
    }
    R[0] = fle_cursor_handle_of((uint32_t)name);
}

/* user32!GetClipCursor (decision 0098, PROVISIONAL): BOOL f(LPRECT lpRect rcx). Learn (GetClipCursor): "A pointer to a RECT structure that
 * receives the screen coordinates of the confining rectangle. The structure receives the dimensions of the screen if the cursor is not
 * confined to a rectangle"; the return value is nonzero on success, zero on failure (GetLastError). Nothing in this environment confines the
 * cursor, so the RECT is the dimensions of the screen: (0, 0, 1920, 1080), by the names of the monitor rectangle of decision 0065. Exactly the 16
 * bytes of the RECT are written. The last error is left alone (the page names none for success). A NULL lpRect has no result on the page: stop. */
void fle_imp_GetClipCursor(void)
{
    static const char api[] = "GetClipCursor";
    uint64_t p = R[1];

    if (p == 0) fle_stop(api, "lpRect is NULL: the page does not define it (decision 0098)");
    fle_st(p, 4, (uint32_t)FLE_MONITOR_LEFT);
    fle_st(p + 4, 4, (uint32_t)FLE_MONITOR_TOP);
    fle_st(p + 8, 4, (uint32_t)FLE_MONITOR_RIGHT);
    fle_st(p + 12, 4, (uint32_t)FLE_MONITOR_BOTTOM);
    R[0] = 1;
}

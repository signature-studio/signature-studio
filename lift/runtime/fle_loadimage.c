/* The virtual LoadImageW (docs/decisions/0061-virtual-load-image.md; the pages read are listed there and in
 * docs/investigations/loadimage-caller.md). Stateless. Two forms are answered; every other argument stops the run through
 * fle_stop (fle_env.h). Nothing here reads the host: no cursor, no theme, no mouse setting, no display, no file, no clock. */
#include <stdint.h>
#include <stdio.h>

#include "fle_runtime.h"
#include "fle_env.h"
#include "fle_modules.h"
#include "fle_cursor.h"
#include "fle_res.h"
#include "fle_realmod.h"
#include "fle_loadimage.h"

uint64_t fle_loadimage_module_handle_of(uint32_t id)
{
    if (id < FLE_LI_MODULE_FIRST || id > FLE_LI_MODULE_LAST) return 0;
    return FLE_LI_MODULE_HANDLE_BASE + 4u * (uint64_t)(id - FLE_LI_MODULE_FIRST + 1u);
}

uint32_t fle_loadimage_module_id_of(uint64_t h)
{
    uint32_t id;
    for (id = FLE_LI_MODULE_FIRST; id <= FLE_LI_MODULE_LAST; id++) {
        if (h == fle_loadimage_module_handle_of(id)) return id;
    }
    return 0;
}

/* user32!LoadImageW. Learn (LoadImageW): HANDLE LoadImageW(HINSTANCE hInst, LPCWSTR name, UINT type, int cx, int cy, UINT
 * fuLoad). type is IMAGE_BITMAP 0, IMAGE_ICON 1 or IMAGE_CURSOR 2. With hInst NULL, no LR_LOADFROMFILE and LR_SHARED, name is a
 * predefined image (IDC_ for cursors); with hInst non-NULL and no LR_LOADFROMFILE, name is the image resource in that module.
 * LR_SHARED (0x8000): "Shares the image handle if the image is loaded multiple times"; "When loading a system icon or cursor,
 * you must use LR_SHARED or the function will fail to load the resource". Success returns the handle, failure NULL (GetLastError;
 * the page names no value). IS_INTRESOURCE(name) true: an integer identifier; otherwise a pointer to a string (Learn: Remarks).
 *
 * Answered: the guest's form only, (type IMAGE_CURSOR, cx 32, cy 32, fuLoad exactly LR_SHARED) and
 *  (a) hInst NULL, name one of the 13 identifiers of fle_cursor.h (fle_cursor_image_handle_of): that cursor's handle, the same
 *      as LoadCursorW(NULL, IDC_ARROW) gives for IDC_ARROW (LoadCursorW "retrieves the handle to the existing resource", it
 *      "always uses LR_SHARED internally": one shared object per identifier; decision 0061);
 *  (b) hInst the lifted DLL (FLE_EXE_HANDLE), name 32762..32767, each an RT_GROUP_CURSOR of the DLL (fle_res_cursor_group_ok
 *      checks it and its RT_CURSOR components): a handle of FLE_LI_MODULE_HANDLE_BASE + 4 * (id - 32762 + 1). The cursor's
 *      image and hot spot are not read.
 * Everything else stops: another type (IMAGE_BITMAP, IMAGE_ICON, other values), another size, another fuLoad (including
 * LR_SHARED with another flag), a string name, a hInst that is neither NULL nor the DLL, an identifier outside (a) / (b), and
 * a DLL id that the DLL does not have (real Windows fails with NULL there, but the page gives no GetLastError value, so a
 * failure is not invented), except FLE_LI_MODULE_MISSING: decision 0062 (PROVISIONAL) returns NULL there, the failure value
 * Learn names, and leaves the last error alone (the page gives no value for it). All 64 bits of rcx and rdx are compared; type, cx, cy and fuLoad are 32-bit values (UINT, int,
 * int, UINT), so only the low 32 bits of their registers count (x64 convention: the upper bits of a 32-bit argument are not
 * defined). The last error is left alone. */
void fle_imp_LoadImageW(void)
{
    static const char api[] = "LoadImageW";
    uint64_t inst = R[1], name = R[2], h;
    uint32_t type = (uint32_t)R[8], fu;
    int32_t cx = (int32_t)(uint32_t)R[9], cy;
    int rm = -1, ok;
    char why[200];

    cy = (int32_t)(uint32_t)fle_arg(5);
    fu = (uint32_t)fle_arg(6);

    if (type != FLE_IMAGE_CURSOR) {
        snprintf(why, sizeof why, "type %u is not IMAGE_CURSOR (2): bitmaps and icons are not answered (decision 0061)", (unsigned)type);
        fle_stop(api, why);
    }
    if (fu != FLE_LR_SHARED) {
        snprintf(why, sizeof why, "fuLoad %#x is not exactly LR_SHARED: not answered (decision 0061)", (unsigned)fu);
        fle_stop(api, why);
    }
    if (cx != FLE_LI_SIZE || cy != FLE_LI_SIZE) {
        snprintf(why, sizeof why, "size %dx%d is not 32x32: not answered (decision 0061)", (int)cx, (int)cy);
        fle_stop(api, why);
    }
    if ((name >> 16) != 0) fle_stop(api, "name is a pointer to a string, not an integer identifier: not answered (decision 0061)");

    if (inst == 0) {
        h = fle_cursor_image_handle_of((uint32_t)name);
        if (h == 0) {
            snprintf(why, sizeof why, "hInst is NULL and name MAKEINTRESOURCE(%u) is not one of the 13 standard cursors the guest loads: not answered (decision 0061)", (unsigned)name);
            fle_stop(api, why);
        }
        R[0] = h;
        return;
    }
    if (inst != FLE_EXE_HANDLE) {
        /* decision 0104: the base of a real module selects the resources of that module's image */
        if (FLE_REALMOD_PRESENT()) rm = fle_realmod_find_handle(inst);
        if (rm < 0) fle_stop(api, "hInst is neither NULL nor the lifted DLL: not answered (decision 0061)");
    }

    ok = inst == FLE_EXE_HANDLE ? fle_res_cursor_group_ok(api, (uint32_t)name) : fle_res_cursor_group_ok_at(api, inst, (uint32_t)name);
    if (!ok) {
        if (name == FLE_LI_MODULE_MISSING) {
            R[0] = 0;
            return;
        }
        snprintf(why, sizeof why, "the lifted DLL has no cursor MAKEINTRESOURCE(%u) (RT_GROUP_CURSOR): real Windows fails with NULL, but Learn gives no GetLastError value for it, so it is not answered (decision 0061)", (unsigned)name);
        fle_stop(api, why);
    }
    h = fle_loadimage_module_handle_of((uint32_t)name);
    if (h == 0) {
        snprintf(why, sizeof why, "the DLL's cursor MAKEINTRESOURCE(%u) is not one the guest loads (32762..32767): not answered (decision 0061)", (unsigned)name);
        fle_stop(api, why);
    }
    if (rm >= 0) h += FLE_LI_REALMOD_STRIDE * (uint64_t)(rm + 1);
    R[0] = h;
}

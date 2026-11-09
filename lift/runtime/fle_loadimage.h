/* The virtual LoadImageW for the one call the guest makes: user32!LoadImageW(hInst, name, IMAGE_CURSOR, 32, 32, LR_SHARED)
 * (docs/decisions/0061-virtual-load-image.md; the pages read and the source of every number are listed there and in
 * docs/investigations/loadimage-caller.md). Same convention as fle_imports.h: arguments in R[1] (rcx), R[2] (rdx), R[8] (r8),
 * R[9] (r9), the fifth and sixth at [rsp+0x28] and [rsp+0x30] (fle_arg, fle_env.h); the Win32 return value in R[0]; the guest
 * return address stays on the stack for the caller's ret. Stateless: a handle is a pure function of the arguments.
 *
 * Nothing is read from the host: no cursor, no theme, no mouse setting, no display. The only data read is the guest's own PE
 * resource directory, to check that an RT_GROUP_CURSOR exists (fle_res.h). No cursor image or hot spot is read.
 *
 * Two forms are answered:
 *  (a) hInst NULL, name MAKEINTRESOURCE(one of the 13 standard IDC_ identifiers the guest loads): the handle of that cursor in
 *      fle_cursor.h (the same one LoadCursorW(NULL, IDC_ARROW) gives for IDC_ARROW);
 *  (b) hInst the lifted DLL (0x400000), name MAKEINTRESOURCE(32762..32767) (RT_GROUP_CURSOR ids of the DLL): a handle in the
 *      region below.
 * Every other argument stops the run through fle_stop (fle_env.h): "fle: LoadImageW: <why>" and abort(). A value is never made up. */
#ifndef FLE_LOADIMAGE_H
#define FLE_LOADIMAGE_H

#include <stdint.h>

/* Learn "LoadImageW" (OFFICIAL): the type values, and LR_SHARED = 0x00008000. */
#define FLE_IMAGE_BITMAP 0
#define FLE_IMAGE_ICON   1
#define FLE_IMAGE_CURSOR 2
#define FLE_LR_SHARED    0x00008000u

/* The only size the guest asks for (the guest's immediates cx = cy = 0x20); any other size stops. */
#define FLE_LI_SIZE 32

/* The RT_GROUP_CURSOR ids of the DLL that the guest loads through LoadImageW with hInst = the DLL: 32762 ..= 32767 (read from
 * the DLL's resource directory; decision 0061). The DLL also has 32761 (not loaded by the guest: stops) and no 32649. */
#define FLE_LI_MODULE_FIRST 32762
#define FLE_LI_MODULE_LAST  32767

/* The one id the guest loads with hInst = the DLL that the DLL does not have (IDC_HAND, 32649; the guest's esi = -21 call):
 * when the DLL's RT_GROUP_CURSOR directory has no such id, LoadImageW returns NULL and leaves the last error alone (decision 0062,
 * PROVISIONAL: Learn names NULL as the failure value and no GetLastError value). Any other id the DLL lacks still stops. */
#define FLE_LI_MODULE_MISSING 32649

/* Handles of the cursors loaded from the DLL (decision 0061, a PROVISIONAL choice; no page gives a value):
 * FLE_LI_MODULE_HANDLE_BASE + 4 * (id - FLE_LI_MODULE_FIRST + 1). The region lies directly after the one of the standard cursors
 * (fle_cursor.h: 0x7FCA0000 + 1 MiB = up to 0x7FDA0000), below 0x80000000 for the same reason as there, and does not overlap the one of the virtual
 * GDI objects (fle_gdi.h: 0x7FBA0000), nor the one of the virtual screen DCs (fle_gdc.h: 0x7FA90000). */
#define FLE_LI_MODULE_HANDLE_BASE UINT64_C(0x7FDA0000)
#define FLE_LI_REGION_SIZE        UINT64_C(0x00100000)

/* Decision 0104 (PROVISIONAL, a token like the ones above): the cursors loaded from a real module (fle_realmod.h; hInst = its base) have
 * the handle FLE_LI_MODULE_HANDLE_BASE + FLE_LI_REALMOD_STRIDE * (index + 1) + 4 * (id - FLE_LI_MODULE_FIRST + 1), index = the position of the
 * module in fle_realmod_table (at most 8 modules, RM_MAX of fle_realmod.c: the largest handle is base + 0x8018, far below the 1 MiB region), so that two modules' cursors with the same id are two handles. Inside the region. */
#define FLE_LI_REALMOD_STRIDE     UINT64_C(0x00001000)

/* Decision 0125 (PROVISIONAL, S-loadcursor-handle): the cursors that user32!LoadCursorW(hInstance, L"name") loads from the NAMED RT_GROUP_CURSOR entries of a module take the slots
 * FLE_LI_NAMED_FIRST + p (p = the position of the name in the directory, below FLE_LI_NAMED_MAX) of the same 4-byte slots as above: base + FLE_LI_REALMOD_STRIDE * k + 4 * (FLE_LI_NAMED_FIRST + p).
 * The ordinal cursors use the slots 1..6, so the two do not meet. Inside the 1 MiB region (the largest is base + 0x8000 + 4 * 0x1FF). */
#define FLE_LI_NAMED_FIRST        0x100u
#define FLE_LI_NAMED_MAX          0x100u

/* user32!LoadImageW: HANDLE f(HINSTANCE hInst rcx, LPCWSTR name rdx, UINT type r8, int cx r9, int cy [rsp+0x28], UINT fuLoad
 * [rsp+0x30]). */
void fle_imp_LoadImageW(void);

/* Inspection (for tests and tools; pure). The handle that LoadImageW answers for a DLL cursor id (0 when that id is not
 * answered), and the id of a handle of that region that LoadImageW answers (0 when h is not such a handle). */
uint64_t fle_loadimage_module_handle_of(uint32_t id);
uint32_t fle_loadimage_module_id_of(uint64_t h);

#endif

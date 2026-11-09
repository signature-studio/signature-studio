/* Resource APIs on the lifted DLL's own PE resources (docs/decisions/0025-resource-apis.md):
 * kernel32!FindResourceW, LoadResource, LockResource, SizeofResource and user32!LoadStringW.
 * The resources are read from the image that the harness maps at FLE_EXE_HANDLE (ImageBase 0x400000,
 * fle_modules.h), through the resource data directory (data directory 2) of that image. Nothing is read from
 * the host. As in fle_imports.h, each stub sets the Win32 return value in R[0] and leaves the guest return
 * address on the stack for the caller's ret. */
#ifndef FLE_RES_H
#define FLE_RES_H

#include <stdint.h>

/* System error codes set by FindResourceW (System Error Codes 1700-3999, Microsoft Learn). The API pages name
 * no code; which code goes with which failure is decision 0025. */
#define FLE_ERROR_RESOURCE_DATA_NOT_FOUND 1812u
#define FLE_ERROR_RESOURCE_TYPE_NOT_FOUND 1813u
#define FLE_ERROR_RESOURCE_NAME_NOT_FOUND 1814u
#define FLE_ERROR_RESOURCE_LANG_NOT_FOUND 1815u

/* kernel32!FindResourceW: HRSRC FindResourceW(HMODULE hModule, LPCWSTR lpName, LPCWSTR lpType).
 * rcx = hModule, rdx = lpName, r8 = lpType (note: name before type, unlike FindResourceExW). */
void fle_imp_FindResourceW(void);

/* kernel32!LoadResource: HGLOBAL LoadResource(HMODULE hModule, HRSRC hResInfo). rcx, rdx. */
void fle_imp_LoadResource(void);

/* kernel32!LockResource: LPVOID LockResource(HGLOBAL hResData). rcx. */
void fle_imp_LockResource(void);

/* kernel32!SizeofResource: DWORD SizeofResource(HMODULE hModule, HRSRC hResInfo). rcx, rdx. */
void fle_imp_SizeofResource(void);

/* user32!LoadStringW: int LoadStringW(HINSTANCE hInstance, UINT uID, LPWSTR lpBuffer, int cchBufferMax).
 * rcx = hInstance, edx = uID, r8 = lpBuffer, r9d = cchBufferMax. */
void fle_imp_LoadStringW(void);

/* Inspection used by LoadImageW (decision 0061; fle_loadimage.c): 1 when the lifted DLL has the RT_GROUP_CURSOR with integer id
 * `id` and every RT_CURSOR component it lists, 0 when one is missing. `api` names the caller in a stop message. No last error. */
int fle_res_cursor_group_ok(const char *api, uint32_t id);
/* The same for the image mapped at `base`: FLE_EXE_HANDLE, or the base of a real module (fle_realmod.h; decision 0104). */
int fle_res_cursor_group_ok_at(const char *api, uint64_t base, uint32_t id);

/* Decision 0118 wall 8 (provisional): the icon that user32!LoadIconW(hInstance, name) loads from an RT_GROUP_ICON of the lifted DLL or of a real module (the rules are
 * in fle_res.c). cx, cy: the requested size; depth: the colour depth of the display. out[0] = the VA of the group's data, out[1] = width, out[2] = height, out[3] = depth,
 * out[4] = the id of the RT_ICON. Returns 1, or 0 when there is no RT_GROUP_ICON of that name. fle_gdi.c reaches it through a weak reference. */
int fle_icon_group_pick(const char *api, uint64_t base, uint64_t name, uint32_t cx, uint32_t cy, uint32_t depth, uint64_t out[5]);

/* Decision 0123 (PROVISIONAL), wall 26: kernel32!FreeResource: BOOL f(HGLOBAL rcx): always FALSE (0), nothing is freed (Learn FreeResource: "For modern versions of Windows this function always returns FALSE"). */
void fle_imp_FreeResource(void);

/* Decision 0125 (PROVISIONAL), wall 54: user32!LoadCursorW(hInstance non-NULL, name) for a cursor of the lifted DLL or of a real module (the rules are in fle_res.c). Returns 1 with *handle set, or 0 with
 * *handle = 0 and the last error set. fle_cursor.c reaches it through a weak reference. */
int fle_cursor_load_named(const char *api, uint64_t inst, uint64_t name, uint64_t *handle);

/* Decision 0129 (PROVISIONAL, S-version-info-real-resource): the RT_VERSION resource of the image mapped at FLE_EXE_HANDLE: 1 with its VA and size, or 0 when there is none. See fle_res.c. */
int fle_res_version_block(const char *api, uint64_t *va, uint32_t *size);

#endif

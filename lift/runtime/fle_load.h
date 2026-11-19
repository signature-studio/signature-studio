/* Library-loader answers over the virtual module table (docs/decisions/0029-apis-a6-a12.md):
 * kernel32!LoadLibraryA, LoadLibraryW, LoadLibraryExW, FreeLibrary and SetDllDirectoryW. No DLL is loaded
 * and the host is never asked: a module is a name in fle_modules.c (decision 0015) and a fixed handle value.
 * LoadLibrary* of a name in that table returns its handle and counts one reference; a name that the
 * virtual environment is defined to lack (FLE_LOAD_ABSENT, decision 0015) fails with ERROR_MOD_NOT_FOUND;
 * every other name stops the run (fle_stop in fle_env.h). As in fle_imports.h, each stub sets the Win32
 * return value in R[0] and leaves the guest return address on the stack for the caller's ret. */
#ifndef FLE_LOAD_H
#define FLE_LOAD_H

#include <stdint.h>

/* kernel32!LoadLibraryA: HMODULE LoadLibraryA(LPCSTR lpLibFileName). rcx = name. */
void fle_imp_LoadLibraryA(void);

/* kernel32!LoadLibraryW: HMODULE LoadLibraryW(LPCWSTR lpLibFileName). rcx = name. */
void fle_imp_LoadLibraryW(void);

/* kernel32!LoadLibraryExW: HMODULE LoadLibraryExW(LPCWSTR lpLibFileName, HANDLE hFile, DWORD dwFlags).
 * rcx = name, rdx = hFile, r8d = dwFlags. */
void fle_imp_LoadLibraryExW(void);

/* kernel32!FreeLibrary: BOOL FreeLibrary(HMODULE hLibModule). rcx = handle. */
void fle_imp_FreeLibrary(void);

/* kernel32!SetDllDirectoryW: BOOL SetDllDirectoryW(LPCWSTR lpPathName). rcx = path or NULL. */
void fle_imp_SetDllDirectoryW(void);

/* Read-only views of the loader state (for tests and tools; not imports).
 * fle_load_refcount: reference count of virtual module `index` (the position in fle_modules.c). */
uint32_t fle_load_refcount(uint64_t index);
/* The directory the last SetDllDirectoryW stored, as a NUL-terminated UTF-16 string, or NULL when the default
 * search order is in force (never called, or called with NULL). The empty string means "remove the current
 * directory from the default order". */
const uint16_t *fle_load_dll_directory(void);

#endif

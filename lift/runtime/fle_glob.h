/* Global memory, atoms and string answers for IAT slots reached by a tail jump
 * (docs/decisions/0024-apis-a3-a5-a13.md, sources in docs/investigations/apis-a3-a5-a13-run.md). Same convention as
 * fle_imports.h: arguments in R[1] (rcx), R[2] (rdx), R[8] (r8), R[9] (r9); the Win32 return value in R[0]; the guest
 * return address stays on the stack for the caller's ret. Single guest thread only. A case the Microsoft pages do
 * not define stops the run through fle_stop (fle_env.h). */
#ifndef FLE_GLOB_H
#define FLE_GLOB_H

#include <stdint.h>

/* Handles of GMEM_MOVEABLE objects (decision 0024). The page only says "a handle to the memory object", so the
 * value is a choice: FLE_GLOBAL_HANDLE_BASE + FLE_GLOBAL_HANDLE_STEP * (i + 1) for the i-th movable object, never
 * reused. The region is above the registry key handles (fle_vfs.h) and does not overlap the other handle regions. */
#define FLE_GLOBAL_HANDLE_BASE UINT64_C(0x7FFA50000000)
#define FLE_GLOBAL_HANDLE_STEP UINT64_C(16)
#define FLE_GLOBAL_REGION_SIZE UINT64_C(0x10000000)

/* Atoms: the page gives 0xC000..0xFFFF for string atoms and 0x0001..0xBFFF for integer atoms. */
#define FLE_ATOM_STRING_FIRST 0xC000u
#define FLE_ATOM_STRING_LAST  0xFFFFu
/* The longest atom name accepted, in UTF-16 units. The pages say "no more than 255 bytes" and do not say whether the
 * terminating NUL counts, nor what happens above the limit: 126 units (252 bytes, 254 with the NUL) fit under both
 * readings; a longer name stops the run. */
#define FLE_ATOM_MAX_UNITS 126u

/* kernel32!GlobalAlloc: HGLOBAL f(UINT uFlags, SIZE_T dwBytes). GlobalFree: HGLOBAL f(HGLOBAL).
 * GlobalLock: LPVOID f(HGLOBAL). GlobalUnlock: BOOL f(HGLOBAL). */
void fle_imp_GlobalAlloc(void);
void fle_imp_GlobalFree(void);
void fle_imp_GlobalLock(void);
void fle_imp_GlobalUnlock(void);

/* kernel32!AddAtomW, FindAtomW, GlobalAddAtomW, GlobalFindAtomW: ATOM f(LPCWSTR). DeleteAtom: ATOM f(ATOM).
 * AddAtomW, FindAtomW and DeleteAtom use the local atom table; the Global ones use the global atom table. */
void fle_imp_AddAtomW(void);
void fle_imp_DeleteAtom(void);
void fle_imp_FindAtomW(void);
void fle_imp_GlobalAddAtomW(void);
void fle_imp_GlobalFindAtomW(void);

/* kernel32!MulDiv: int f(int, int, int). kernel32!lstrlenW: int f(LPCWSTR). */
void fle_imp_MulDiv(void);
void fle_imp_lstrlenW(void);

/* Number of live GMEM_MOVEABLE objects and of live fixed blocks, and of live string atoms in the local and the
 * global table (for tests and tools). */
uint64_t fle_glob_movable_count(void);
uint64_t fle_glob_fixed_count(void);
uint64_t fle_glob_atom_count(int global_table);

#endif

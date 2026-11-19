/* Decision 0210 (api2, PROVISIONAL, opt-in): the first small block of the systematic Win32 API coverage. New API answers, written from the Microsoft Learn pages and checked against Wine 10.0 with
 * lift/harness/api2/ (the pages are in the comment above each function; where a page is silent the choice is named S-api2-N and, if Wine is the only source, marked UNOFFICIAL).
 * Same convention as fle_imports.h: arguments in R[1] (rcx), R[2] (rdx), R[8] (r8), R[9] (r9) and from the fifth on at [R[4] + 0x28 + 8 * (n - 5)]; the Win32 return value in R[0].
 * Nothing here changes the default generated C: a row for these functions exists only in lift/harness/seeds/api2-imports.txt (fe-lift lift --extra-imports). */
#ifndef FLE_API2_H
#define FLE_API2_H

#include <stdint.h>

/* kernel32 NLS, A variants (converted through the code page of the emulated locale, CP_ACP, to the W functions of fle_locale.c): */
void fle_imp_CompareStringA(void);
void fle_imp_LCMapStringA(void);
void fle_imp_GetStringTypeA(void);
void fle_imp_GetLocaleInfoA(void);
/* kernel32 time zone */
void fle_imp_TzSpecificLocalTimeToSystemTime(void);

/* kernel32 system / console / links */
void fle_imp_GetLargePageMinimum(void);
void fle_imp_VirtualQueryEx(void);
void fle_imp_OutputDebugStringA(void);
void fle_imp_WriteConsoleA(void);
void fle_imp_ReadConsoleW(void);
void fle_imp_GetConsoleCP(void);
void fle_imp_CreateSymbolicLinkW(void);

/* shell32 */
void fle_imp_ShellExecuteA(void);
void fle_imp_ShellExecuteExW(void);

/* Calls another runtime function of the guest ABI from inside the runtime: a_1..a_8 are the first eight arguments (the fifth to the eighth go to a private fake stack frame), the result is R[0] of the
 * callee. R[1..15] are restored, R[0] is left to the caller. Guest pointers are host pointers (identity map, fle_runtime.h), so a buffer of the runtime can be passed. */
uint64_t fle_api2_call(void (*f)(void), uint64_t a1, uint64_t a2, uint64_t a3, uint64_t a4, uint64_t a5, uint64_t a6, uint64_t a7, uint64_t a8);

#endif

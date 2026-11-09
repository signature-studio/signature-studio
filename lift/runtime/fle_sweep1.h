/* Decision 0215 (sweep1, PROVISIONAL, opt-in): API answers written from the Microsoft Learn pages for the walls that the systematic sweep (lift/out-sweep1/tools, docs/0215-systematic-sweep.md) found. A row for these
 * functions exists only in lift/harness/seeds/sweep1-imports.txt (fe-lift lift --extra-imports); without the rows the generated C is the one of the earlier decisions. Same convention as fle_imports.h: arguments in
 * R[1] (rcx), R[2] (rdx), R[8] (r8), R[9] (r9) and from the fifth on at [R[4] + 0x28 + 8 * (n - 5)]; the Win32 return value in R[0]. */
#ifndef FLE_SWEEP1_H
#define FLE_SWEEP1_H

#include <stdint.h>

/* user32 keyboard-layout family (fle_sweep1_kbd.c) */
void fle_imp_GetKeyboardLayoutNameW(void);
void fle_imp_GetKeyboardLayoutNameA(void);
void fle_imp_GetKeyboardLayoutList(void);
void fle_imp_LoadKeyboardLayoutW(void);
void fle_imp_LoadKeyboardLayoutA(void);
void fle_imp_ActivateKeyboardLayout(void);
void fle_imp_ToUnicode(void);
void fle_imp_ToUnicodeEx(void);
void fle_imp_ToAscii(void);
void fle_imp_ToAsciiEx(void);
void fle_imp_VkKeyScanW(void);
void fle_imp_VkKeyScanExW(void);
void fle_imp_VkKeyScanA(void);
void fle_imp_VkKeyScanExA(void);
void fle_imp_OemKeyScan(void);
void fle_imp_MapVirtualKeyExW(void);
void fle_imp_MapVirtualKeyA(void);
void fle_imp_MapVirtualKeyExA(void);

/* defined in fle_input.c (the existing MapVirtualKeyW; not declared in an installed header) */
void fle_imp_MapVirtualKeyW(void);

#endif

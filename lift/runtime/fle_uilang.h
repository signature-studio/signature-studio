/* kernel32!GetThreadUILanguage (docs/decisions/0021-uilang.md). As in fle_imports.h, the stub sets the Win32
 * return value in R[0] and leaves the guest return address on the stack for the caller's ret. The language
 * comes from fle_emulated_locale (fle_locale.h); this file holds no locale value of its own. */
#ifndef FLE_UILANG_H
#define FLE_UILANG_H

#include <stdint.h>

/* kernel32!GetThreadUILanguage: LANGID GetThreadUILanguage(void). No arguments. */
void fle_imp_GetThreadUILanguage(void);

#endif

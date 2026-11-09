#include <stdint.h>

#include "fle_runtime.h"
#include "fle_locale.h"
#include "fle_uilang.h"

/* kernel32!GetThreadUILanguage: LANGID GetThreadUILanguage(void). MSDN (GetThreadUILanguage, Return value
 * and Remarks): returns the language identifier of the first user interface language of the calling
 * thread. A language explicitly associated with the thread by SetThreadUILanguage or
 * SetThreadPreferredUILanguages is returned first; "if no language has been explicitly associated with the
 * current thread, the identifier can indicate a user or system user interface language".
 * Neither setter is answered by this runtime (they are not in the response table), so no language can have
 * been associated with the thread. The answer is therefore the user/system UI language of the emulated
 * environment, the same value as GetUserDefaultUILanguage and GetSystemDefaultUILanguage (fle_locale.c):
 * the low 16 bits of fle_emulated_locale.lcid. The page's remark about supplemental-locale (LIP) languages
 * (1400 / 1000) does not apply: en-US is not a supplemental locale. If a setter is added later, this
 * function has to return the language that it set. */
void fle_imp_GetThreadUILanguage(void)
{
    R[0] = fle_emulated_locale.lcid & 0xFFFFu;
}

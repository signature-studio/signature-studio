/* kernel32!GetThreadPreferredUILanguages (docs/decisions/0022-thread-preferred-ui-languages.md). Same
 * convention as fle_imports.h: arguments in R[1] (rcx), R[2] (rdx), R[8] (r8), R[9] (r9); the BOOL result in
 * R[0]; the guest return address stays on the stack for the caller's ret. The language names come from
 * fle_emulated_locale through the GetLocaleInfoW answers (fle_locale.c); this file holds no locale value of
 * its own. A case that the MSDN pages leave open stops the run (fle_stop in fle_env.h). */
#ifndef FLE_UIPREF_H
#define FLE_UIPREF_H

#include <stdint.h>

/* kernel32!GetThreadPreferredUILanguages: BOOL f(DWORD dwFlags, PULONG pulNumLanguages,
 * PZZWSTR pwszLanguagesBuffer, PULONG pcchLanguagesBuffer). ecx, rdx, r8, r9. */
void fle_imp_GetThreadPreferredUILanguages(void);

/* kernel32!GetUserPreferredUILanguages (decision 0117, PROVISIONAL): BOOL f(DWORD dwFlags, PULONG pulNumLanguages, PZZWSTR pwszLanguagesBuffer, PULONG pcchLanguagesBuffer).
 * ecx, rdx, r8, r9. The user's preferred display languages of this environment: the one emulated locale, as a name ("en-US") or, with MUI_LANGUAGE_ID, as "0409". */
void fle_imp_GetUserPreferredUILanguages(void);

#endif

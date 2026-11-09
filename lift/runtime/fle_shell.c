/* shell32 additions of decision 0171 (PROVISIONAL, wall ia1-W6). Convention as fle_imports.h: arguments in R[1] (rcx), R[2] (rdx), R[8] (r8), R[9] (r9); the Win32 return value in R[0]. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_shell.h"

/* shell32!PathCleanupSpec: int f(PCWSTR pszDir rcx, PWSTR pszSpec rdx). Learn (shlobj_core.h, "Removes illegal characters from a path ... and truncates"): the return value is a bit mask: PCS_REPLACEDCHAR 0x1
 * (an illegal character was replaced), PCS_REMOVEDCHAR 0x2, PCS_TRUNCATED 0x4, PCS_PATHTOOLONG 0x8, PCS_FATAL 0x80000000; 0 = pszSpec is unchanged. pszDir may be NULL. The page does not list the illegal
 * characters or say what replaces them, so this answers only the case where nothing needs changing: pszSpec holds none of \ / : * ? " < > | and no control character (S-ia1-pcs-1), and dir + '\' + spec is
 * shorter than MAX_PATH (260). Anything else stops by name. NULL pszSpec stops. */
void fle_imp_PathCleanupSpec(void)
{
    static const char api[] = "PathCleanupSpec";
    uint64_t dir = R[1], spec = R[2], n = 0, dn = 0;
    uint16_t c;

    if (spec == 0) fle_stop(api, "pszSpec is NULL: the page gives no behaviour");
    for (;; n++) {
        c = (uint16_t)fle_ld(spec + 2u * n, 2);
        if (c == 0) break;
        if (c < 0x20u || c == '\\' || c == '/' || c == ':' || c == '*' || c == '?' || c == '"' || c == '<' || c == '>' || c == '|') {
            fle_stop(api, "pszSpec holds an illegal character: the page lists the illegal characters but does not say which of them are replaced, which are removed, or what replaces them (decision 0191)");
        }
        if (n > 300u) fle_stop(api, "pszSpec is longer than MAX_PATH: the truncation rule is not on the page");
    }
    if (dir != 0) {
        for (;; dn++) {
            c = (uint16_t)fle_ld(dir + 2u * dn, 2);
            if (c == 0) break;
            if (dn > 300u) fle_stop(api, "pszDir is longer than MAX_PATH");
        }
    }
    if (dn + (dir ? 1u : 0u) + n >= 260u) fle_stop(api, "the path would reach MAX_PATH: the truncation rule is not on the page");
    R[0] = 0;
}

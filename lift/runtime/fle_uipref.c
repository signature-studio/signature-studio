#include <stdint.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_locale.h"
#include "fle_uipref.h"

/* kernel32!GetThreadPreferredUILanguages. Rules used (MSDN GetThreadPreferredUILanguages, retrieved
 * 2026-10-20, and "User Interface Language Management"):
 *  - format flags MUI_LANGUAGE_ID (0x4) / MUI_LANGUAGE_NAME (0x8), mutually exclusive, default NAME. ID strings
 *    are 4 hexadecimal digits without 0x ("0409" for en-US, "0009" for en).
 *  - filter flags: the default is MUI_MERGE_USER_FALLBACK (0x20): thread preferred languages, then process
 *    preferred, then user preferred that are distinct, then the system default UI language if not already
 *    in the list. MUI_MERGE_SYSTEM_FALLBACK (0x10) only together with 0x20 (= MUI_UI_FALLBACK, 0x30): the list
 *    also holds fallback and neutral languages; the language-management page orders a language before its
 *    neutral form ("fr-FR, fr") and drops duplicates.
 *  - this environment has no thread and no process preferred languages (the setters are not answered) and one
 *    user/system language, the emulated locale. So the list is [name] for 0x20 and [name, neutral] for 0x30.
 *    The neutral form is the ISO 639 language name of the locale (name format) or the primary language id,
 *    low 10 bits (id format): a reading that matches the page examples for en-US ("0409", "0009", "en"), not
 *    a page rule for every locale.
 *  - buffer NULL with *pcch == 0 returns the required size in *pcch, which includes the two NULs. A NULL
 *    buffer with a non-zero size, or a buffer that is too small, fails with ERROR_INSUFFICIENT_BUFFER (122)
 *    and leaves the outputs alone (the page calls them undefined). On success *pcch is the size of the
 *    retrieved buffer, *pul the number of languages; the last error is not touched.
 * Stops (the page does not define the result): a flag bit other than the four above (for example
 * MUI_THREAD_LANGUAGES), both format flags, 0x10 without 0x20, a NULL pul or pcch. */

#define FLE_UIPREF_ID        0x04u
#define FLE_UIPREF_NAME      0x08u
#define FLE_UIPREF_SYSFB     0x10u
#define FLE_UIPREF_USERFB    0x20u
#define FLE_UIPREF_KNOWN     (FLE_UIPREF_ID | FLE_UIPREF_NAME | FLE_UIPREF_SYSFB | FLE_UIPREF_USERFB)

#define FLE_UIPREF_LC_SISO639LANGNAME 0x00000059u /* MSDN: LOCALE_SISO639LANGNAME */
#define FLE_UIPREF_LC_SNAME           0x0000005Cu /* MSDN: LOCALE_SNAME */

#define FLE_UIPREF_ERR_INSUFFICIENT_BUFFER 122u

#define FLE_UIPREF_LIST_MAX 64

/* One GetLocaleInfoW answer for the emulated locale, into out (cap characters). Returns the length with the NUL,
 * 0 if the answer failed. The argument registers of the caller are restored. */
static size_t fle_uipref_locale_string(uint32_t lctype, uint16_t *out, size_t cap)
{
    uint64_t s1 = R[1], s2 = R[2], s8 = R[8], s9 = R[9], n;

    R[1] = fle_emulated_locale.lcid;
    R[2] = lctype;
    R[8] = (uint64_t)(uintptr_t)out;
    R[9] = cap;
    fle_imp_GetLocaleInfoW();
    n = R[0];
    R[1] = s1;
    R[2] = s2;
    R[8] = s8;
    R[9] = s9;
    return (size_t)n;
}

static size_t fle_uipref_hex4(unsigned v, uint16_t *out)
{
    static const char digits[] = "0123456789ABCDEF";
    int i;

    for (i = 0; i < 4; i++) out[i] = (uint8_t)digits[(v >> (12 - 4 * i)) & 0xFu];
    out[4] = 0;
    return 5;
}

/* Appends the string s (n characters with its NUL) to list at *used unless it is already in the list (the
 * page: a duplicate language is retrieved only once). Counts it in *num. */
static void fle_uipref_add(uint16_t *list, size_t *used, uint32_t *num, const uint16_t *s, size_t n)
{
    size_t off = 0;
    uint32_t i;

    for (i = 0; i < *num; i++) {
        size_t len = fle_w_len(list + off) + 1;

        if (len == n && memcmp(list + off, s, n * sizeof *s) == 0) return;
        off += len;
    }
    if (*used + n + 1 > FLE_UIPREF_LIST_MAX)
        fle_stop("GetThreadPreferredUILanguages", "language list longer than the internal buffer");
    memcpy(list + *used, s, n * sizeof *s);
    *used += n;
    (*num)++;
}

void fle_imp_GetThreadPreferredUILanguages(void)
{
    uint32_t flags = (uint32_t)R[1];
    uint32_t *pnum = (uint32_t *)fle_addr_opt(R[2], 4, 1);
    uint16_t *buf = (uint16_t *)fle_addr_opt(R[8], 2, 1);
    uint32_t *pcch = (uint32_t *)fle_addr_opt(R[9], 4, 1);
    uint16_t list[FLE_UIPREF_LIST_MAX], tmp[32];
    size_t used = 0, n;
    uint32_t num = 0, filter;
    int as_id;

    if (flags & ~FLE_UIPREF_KNOWN)
        fle_stop("GetThreadPreferredUILanguages", "dwFlags bit not handled (MUI_THREAD_LANGUAGES or an unknown bit)");
    if ((flags & FLE_UIPREF_ID) && (flags & FLE_UIPREF_NAME))
        fle_stop("GetThreadPreferredUILanguages", "MUI_LANGUAGE_ID with MUI_LANGUAGE_NAME: the page says exclusive, not what fails");
    as_id = (flags & FLE_UIPREF_ID) != 0;
    filter = flags & (FLE_UIPREF_SYSFB | FLE_UIPREF_USERFB);
    if (filter == 0) filter = FLE_UIPREF_USERFB;
    if (filter == FLE_UIPREF_SYSFB)
        fle_stop("GetThreadPreferredUILanguages", "MUI_MERGE_SYSTEM_FALLBACK without MUI_MERGE_USER_FALLBACK: the page does not say what fails");
    if (pnum == NULL || pcch == NULL)
        fle_stop("GetThreadPreferredUILanguages", "NULL pulNumLanguages or pcchLanguagesBuffer: the page does not define it");

    if (as_id) {
        fle_uipref_add(list, &used, &num, tmp, fle_uipref_hex4(fle_emulated_locale.lcid & 0xFFFFu, tmp));
        if (filter == (FLE_UIPREF_SYSFB | FLE_UIPREF_USERFB))
            fle_uipref_add(list, &used, &num, tmp, fle_uipref_hex4(fle_emulated_locale.lcid & 0x3FFu, tmp));
    } else {
        n = fle_uipref_locale_string(FLE_UIPREF_LC_SNAME, tmp, sizeof tmp / sizeof tmp[0]);
        if (n == 0) fle_stop("GetThreadPreferredUILanguages", "no language name for the emulated locale");
        fle_uipref_add(list, &used, &num, tmp, n);
        if (filter == (FLE_UIPREF_SYSFB | FLE_UIPREF_USERFB)) {
            n = fle_uipref_locale_string(FLE_UIPREF_LC_SISO639LANGNAME, tmp, sizeof tmp / sizeof tmp[0]);
            if (n == 0) fle_stop("GetThreadPreferredUILanguages", "no neutral language name for the emulated locale");
            fle_uipref_add(list, &used, &num, tmp, n);
        }
    }
    list[used++] = 0; /* the second NUL that ends the list */

    if (buf == NULL && *pcch == 0) {
        *pcch = (uint32_t)used;
        *pnum = num;
        R[0] = 1;
        return;
    }
    if (buf == NULL || *pcch < used) {
        fle_set_last_error(FLE_UIPREF_ERR_INSUFFICIENT_BUFFER);
        R[0] = 0;
        return;
    }
    memcpy(buf, list, used * sizeof list[0]);
    *pcch = (uint32_t)used;
    *pnum = num;
    R[0] = 1;
}

/* kernel32!GetUserPreferredUILanguages (decision 0117, PROVISIONAL). Learn (GetUserPreferredUILanguages, last updated 2021-10-13): dwFlags "specify the language format
 * ... The flags are mutually exclusive, and the default is MUI_LANGUAGE_NAME" (MUI_LANGUAGE_ID, MUI_LANGUAGE_NAME; no filter flags are listed); pwszLanguagesBuffer
 * receives "an ordered, null-delimited display language list ... This list ends with two null characters"; "if this parameter is set to NULL and pcchLanguagesBuffer is
 * set to 0, the function retrieves the required size of the language buffer in pcchLanguagesBuffer. The required size includes the two null characters"; on return
 * pcchLanguagesBuffer holds the size of the retrieved buffer; TRUE or FALSE with ERROR_INSUFFICIENT_BUFFER "A supplied buffer size was not large enough, or it was
 * incorrectly set to NULL"; "When MUI_LANGUAGE_ID is specified, the language strings ... will be 4 characters in length. For example, en-US will be returned as "0409""; the
 * list has no duplicates. The guest (the Delphi helper f_9df190, reached through a delay-loaded GetProcAddress) calls it with MUI_LANGUAGE_NAME, first with a NULL buffer and
 * size 0, then with a buffer of that size, and uses the first name.
 * The answer is the user/system language of the emulated environment (fle_emulated_locale, the same en-US as GetUserDefaultUILanguage and GetThreadPreferredUILanguages):
 * one language. This environment has no per-user language list of its own; one entry is a choice of this environment, not a rule of the page. The same buffer rules as
 * GetThreadPreferredUILanguages. Stops: a flag bit other than the two format flags, both format flags, a NULL pulNumLanguages or pcchLanguagesBuffer. */
void fle_imp_GetUserPreferredUILanguages(void)
{
    static const char api[] = "GetUserPreferredUILanguages";
    uint32_t flags = (uint32_t)R[1];
    uint32_t *pnum = (uint32_t *)fle_addr_opt(R[2], 4, 1);
    uint16_t *buf = (uint16_t *)fle_addr_opt(R[8], 2, 1);
    uint32_t *pcch = (uint32_t *)fle_addr_opt(R[9], 4, 1);
    uint16_t list[FLE_UIPREF_LIST_MAX], tmp[32];
    size_t used = 0, n;
    uint32_t num = 0;

    if (flags & ~(FLE_UIPREF_ID | FLE_UIPREF_NAME)) fle_stop(api, "dwFlags holds a bit other than MUI_LANGUAGE_ID and MUI_LANGUAGE_NAME: the page lists no other flag (decision 0117)");
    if ((flags & FLE_UIPREF_ID) && (flags & FLE_UIPREF_NAME)) fle_stop(api, "MUI_LANGUAGE_ID with MUI_LANGUAGE_NAME: the page says exclusive, not what fails (decision 0117)");
    if (pnum == NULL || pcch == NULL) fle_stop(api, "NULL pulNumLanguages or pcchLanguagesBuffer: the page does not define it (decision 0117)");
    if (flags & FLE_UIPREF_ID) {
        fle_uipref_add(list, &used, &num, tmp, fle_uipref_hex4(fle_emulated_locale.lcid & 0xFFFFu, tmp));
    } else {
        n = fle_uipref_locale_string(FLE_UIPREF_LC_SNAME, tmp, sizeof tmp / sizeof tmp[0]);
        if (n == 0) fle_stop(api, "no language name for the emulated locale (decision 0117)");
        fle_uipref_add(list, &used, &num, tmp, n);
    }
    list[used++] = 0; /* the second NUL that ends the list */
    if (buf == NULL && *pcch == 0) {
        *pcch = (uint32_t)used;
        *pnum = num;
        R[0] = 1;
        return;
    }
    if (buf == NULL || *pcch < used) {
        fle_set_last_error(FLE_UIPREF_ERR_INSUFFICIENT_BUFFER);
        R[0] = 0;
        return;
    }
    memcpy(buf, list, used * sizeof list[0]);
    *pcch = (uint32_t)used;
    *pnum = num;
    R[0] = 1;
}

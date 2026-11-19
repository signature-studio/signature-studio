#include <stdint.h>
#include <stdlib.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_locale.h"
#include "fle_crash1_lcmap.h"
#include "fle_thread.h"

/* The one place that holds the emulated locale (docs/decisions/0016-locale.md): en-US (LCID 0x0409),
 * ANSI code page 1252, OEM code page 437. A setting of this lift environment; the MSDN pages do not
 * define these values. Everything below reads this object. The code below also treats the two code
 * pages in it as single-byte and ASCII-compatible (true for 1252 and 437, general knowledge, not an
 * MSDN statement); changing the object to a DBCS code page needs changes here. */
const fle_locale_t fle_emulated_locale = {
    .lcid = 0x0409,
    .acp = 1252,
    .oemcp = 437,
};

#define FLE_LOCALE_USER_DEFAULT   0x0400u /* MSDN: LOCALE_USER_DEFAULT */
#define FLE_LOCALE_SYSTEM_DEFAULT 0x0800u /* MSDN: LOCALE_SYSTEM_DEFAULT */
#define FLE_LOCALE_INVARIANT      0x007Fu /* MSDN: LOCALE_INVARIANT */

#define FLE_CP_UTF8 65001u
#define FLE_CP_1252 1252u

#define FLE_ERR_INVALID_PARAMETER    87u
#define FLE_ERR_INSUFFICIENT_BUFFER  122u
#define FLE_ERR_INVALID_FLAGS        1004u
#define FLE_ERR_NO_UNICODE_TRANSLATION 1113u

static void fle_cmp_context(void);   /* decision 0284 */

static void fle_locale_stop(const char *api, const char *why)
{
    fprintf(stderr, "fle: %s: %s (not defined by the MSDN page or outside the emulated locale; not implemented)\n",
            api, why);
    fle_cmp_context();
    abort();
}

static void fle_fail(uint32_t err)
{
    fle_set_last_error(err);
    R[0] = 0;
}

/* Argument n (1-based, n >= 5) of the Windows x64 call. The guest return address is still on the
 * stack at R[4], and the 32-byte shadow space follows it. */
static uint64_t fle_stack_arg(unsigned n)
{
    return fle_ld(R[4] + 8u + 8u * (n - 1u), 8);
}

/* Thread locale (MSDN: GetThreadLocale Remarks: a thread starts with the user locale; SetThreadLocale
 * changes it). 0 means not set. Decision 0122: this was `__thread` (the HOST thread); the thread is now the GUEST thread (fle_thread.c): the first guest thread keeps this variable
 * (the host thread of a program that creates no guest thread, as before), a guest thread that CreateThread made has its own slot, which starts at 0 = not set. */
static __thread uint32_t fle_thread_lcid;

static uint32_t *lcid_slot(void)
{
    uint32_t *p = fle_thread_slot_lcid ? fle_thread_slot_lcid() : NULL;

    return p ? p : &fle_thread_lcid;
}

static uint32_t fle_thread_locale(void)
{
    uint32_t v = *lcid_slot();

    return v ? v : fle_emulated_locale.lcid;
}

/* The locales this environment has: the emulated one, the two defaults (which resolve to it) and the
 * invariant locale. Anything else is not claimed. Returns 1 and the resolved LCID, or 0. */
static int fle_locale_resolve(uint32_t lcid, uint32_t *out)
{
    if (lcid == FLE_LOCALE_USER_DEFAULT || lcid == FLE_LOCALE_SYSTEM_DEFAULT ||
        lcid == fle_emulated_locale.lcid) {
        *out = fle_emulated_locale.lcid;
        return 1;
    }
    if (lcid == FLE_LOCALE_INVARIANT) {
        *out = FLE_LOCALE_INVARIANT;
        return 1;
    }
    return 0;
}

/* Locale argument of an NLS function whose data exist only for the emulated locale. */
static void fle_require_emulated(const char *api, uint32_t lcid)
{
    uint32_t r;
    if (!fle_locale_resolve(lcid, &r) || r != fle_emulated_locale.lcid)
        fle_locale_stop(api, "locale other than the emulated one");
}

/* kernel32!SetThreadLocale: BOOL SetThreadLocale(LCID Locale). The locale of the calling thread
 * becomes Locale (predefined defaults resolve to the emulated locale). TRUE on success. A locale the
 * environment does not have fails with 0 and ERROR_INVALID_PARAMETER (the page names no error code:
 * 87 is a harness choice). The page's Syntax says BOOL; its Return value text says "an LCID ... of the
 * previous thread locale". The two disagree; the BOOL reading is used (the lifted caller at 0x41ce46
 * does not read the result). */
void fle_imp_SetThreadLocale(void)
{
    uint32_t r;

    if (!fle_locale_resolve((uint32_t)R[1], &r)) {
        fle_fail(FLE_ERR_INVALID_PARAMETER);
        return;
    }
    *lcid_slot() = r;
    R[0] = 1;
}

/* kernel32!GetThreadLocale: LCID GetThreadLocale(void). */
void fle_imp_GetThreadLocale(void)
{
    R[0] = fle_thread_locale();
}

/* kernel32!IsValidLocale: BOOL IsValidLocale(LCID Locale, DWORD dwFlags). dwFlags is LCID_INSTALLED
 * (1) or LCID_SUPPORTED (2); the page lists no other value (0x39 is "do not use"), so others stop.
 * TRUE only for a locale this environment has (see fle_locale_resolve). That also answers FALSE for
 * LCID_SUPPORTED of locales Windows supports but this environment does not have; this is the
 * environment's choice, not Windows behavior. The failure case sets no last error (the page names
 * none). */
void fle_imp_IsValidLocale(void)
{
    uint32_t r, flags = (uint32_t)R[2];

    if (flags != 1u && flags != 2u)
        fle_locale_stop("IsValidLocale", "dwFlags is neither LCID_INSTALLED nor LCID_SUPPORTED");
    R[0] = fle_locale_resolve((uint32_t)R[1], &r) ? 1u : 0u;
}

/* kernel32!GetUserDefaultUILanguage and GetSystemDefaultUILanguage: LANGID f(void). Both are the
 * language identifier (low 16 bits) of the emulated locale. */
void fle_imp_GetUserDefaultUILanguage(void)
{
    R[0] = fle_emulated_locale.lcid & 0xFFFFu;
}

void fle_imp_GetSystemDefaultUILanguage(void)
{
    R[0] = fle_emulated_locale.lcid & 0xFFFFu;
}

/* kernel32!GetACP: UINT GetACP(void). */
void fle_imp_GetACP(void)
{
    R[0] = fle_emulated_locale.acp;
}

/* kernel32!GetOEMCP: UINT GetOEMCP(void) (decision 0078). The page: "the current OEM code page identifier for the operating system".
 * The emulated OEM code page (fle_emulated_locale.oemcp, 437). */
void fle_imp_GetOEMCP(void)
{
    R[0] = fle_emulated_locale.oemcp;
}

/* ---- the range D of decision 0087 ---------------------------------------------------------------------------------------------------
 * D = U+0000..U+00FF, the 27 units above U+00FF that the lifted DLL's CRT passes (docs/investigations/dsp-ippv2-ctype-locale-data.md 3.1)
 * and U+0191 (the MS8 upper case of U+0192): 284 units. GetStringTypeW(CT_CTYPE1), LCMapStringW (LCMAP_LOWERCASE or LCMAP_UPPERCASE alone)
 * and WideCharToMultiByte(1252) answer for a string of D only; a unit outside D stops by name, before anything is written. */
static const uint16_t fle_d_high[28] = {
    0x0152, 0x0153, 0x0160, 0x0161, 0x0178, 0x017D, 0x017E, 0x0191, 0x0192, 0x02C6, 0x02DC, 0x2013, 0x2014, 0x2018,
    0x2019, 0x201A, 0x201C, 0x201D, 0x201E, 0x2020, 0x2021, 0x2022, 0x2026, 0x2030, 0x2039, 0x203A, 0x20AC, 0x2122,
};

static size_t fle_wlen(const uint16_t *s);

static int fle_in_d(uint32_t c)
{
    size_t i;

    if (c < 0x100) return 1;
    for (i = 0; i < sizeof fle_d_high / sizeof fle_d_high[0]; i++)
        if (fle_d_high[i] == c) return 1;
    return 0;
}

static void fle_stop_outside_d(const char *api, uint32_t c)
{
    fprintf(stderr, "fle: %s: U+%04X is outside the range D of decision 0087 (U+0000..U+00FF and 28 units above it; not implemented)\n", api, (unsigned)c);
    abort();
}

/* C1 values of GetStringTypeW(CT_CTYPE1) for D: UNOFFICIAL, Wine 10.0 en-US (LC_ALL=en_US.UTF-8), docs/decisions/0087-ctype-locale-data.md. No
 * Microsoft page gives them (the Learn page names the C1_* bits only). UPPER (0x1) and LOWER (0x2) equal UCD 16.0.0 Lu and Lowercase on all of
 * these units; the other bits (ALPHA, DIGIT, PUNCT, SPACE, CNTRL, BLANK, XDIGIT, DEFINED) are Wine's alone. Copy of
 * lift/runtime/test/c1_wine_10_0_enus.txt (283 rows; U+0191 was not measured and has no value here). */
static const uint16_t fle_c1_latin1[256] = {
    0x0220, 0x0220, 0x0220, 0x0220, 0x0220, 0x0220, 0x0220, 0x0220, 0x0220, 0x0268, 0x0228, 0x0228, 0x0228, 0x0228, 0x0220, 0x0220,
    0x0220, 0x0220, 0x0220, 0x0220, 0x0220, 0x0220, 0x0220, 0x0220, 0x0220, 0x0220, 0x0220, 0x0220, 0x0220, 0x0220, 0x0220, 0x0220,
    0x0248, 0x0210, 0x0210, 0x0210, 0x0210, 0x0210, 0x0210, 0x0210, 0x0210, 0x0210, 0x0210, 0x0210, 0x0210, 0x0210, 0x0210, 0x0210,
    0x0284, 0x0284, 0x0284, 0x0284, 0x0284, 0x0284, 0x0284, 0x0284, 0x0284, 0x0284, 0x0210, 0x0210, 0x0210, 0x0210, 0x0210, 0x0210,
    0x0210, 0x0381, 0x0381, 0x0381, 0x0381, 0x0381, 0x0381, 0x0301, 0x0301, 0x0301, 0x0301, 0x0301, 0x0301, 0x0301, 0x0301, 0x0301,
    0x0301, 0x0301, 0x0301, 0x0301, 0x0301, 0x0301, 0x0301, 0x0301, 0x0301, 0x0301, 0x0301, 0x0210, 0x0210, 0x0210, 0x0210, 0x0210,
    0x0210, 0x0382, 0x0382, 0x0382, 0x0382, 0x0382, 0x0382, 0x0302, 0x0302, 0x0302, 0x0302, 0x0302, 0x0302, 0x0302, 0x0302, 0x0302,
    0x0302, 0x0302, 0x0302, 0x0302, 0x0302, 0x0302, 0x0302, 0x0302, 0x0302, 0x0302, 0x0302, 0x0210, 0x0210, 0x0210, 0x0210, 0x0220,
    0x0220, 0x0220, 0x0220, 0x0220, 0x0220, 0x0228, 0x0220, 0x0220, 0x0220, 0x0220, 0x0220, 0x0220, 0x0220, 0x0220, 0x0220, 0x0220,
    0x0220, 0x0220, 0x0220, 0x0220, 0x0220, 0x0220, 0x0220, 0x0220, 0x0220, 0x0220, 0x0220, 0x0220, 0x0220, 0x0220, 0x0220, 0x0220,
    0x0248, 0x0210, 0x0210, 0x0210, 0x0210, 0x0210, 0x0210, 0x0210, 0x0210, 0x0210, 0x0312, 0x0210, 0x0210, 0x0230, 0x0210, 0x0210,
    0x0210, 0x0210, 0x0214, 0x0214, 0x0210, 0x0312, 0x0210, 0x0210, 0x0210, 0x0214, 0x0312, 0x0210, 0x0210, 0x0210, 0x0210, 0x0210,
    0x0301, 0x0301, 0x0301, 0x0301, 0x0301, 0x0301, 0x0301, 0x0301, 0x0301, 0x0301, 0x0301, 0x0301, 0x0301, 0x0301, 0x0301, 0x0301,
    0x0301, 0x0301, 0x0301, 0x0301, 0x0301, 0x0301, 0x0301, 0x0210, 0x0301, 0x0301, 0x0301, 0x0301, 0x0301, 0x0301, 0x0301, 0x0302,
    0x0302, 0x0302, 0x0302, 0x0302, 0x0302, 0x0302, 0x0302, 0x0302, 0x0302, 0x0302, 0x0302, 0x0302, 0x0302, 0x0302, 0x0302, 0x0302,
    0x0302, 0x0302, 0x0302, 0x0302, 0x0302, 0x0302, 0x0302, 0x0210, 0x0302, 0x0302, 0x0302, 0x0302, 0x0302, 0x0302, 0x0302, 0x0302,
};
static const uint16_t fle_c1_high[27][2] = {
    { 0x0152, 0x0301 }, { 0x0153, 0x0302 }, { 0x0160, 0x0301 }, { 0x0161, 0x0302 }, { 0x0178, 0x0301 }, { 0x017D, 0x0301 },
    { 0x017E, 0x0302 }, { 0x0192, 0x0302 }, { 0x02C6, 0x0300 }, { 0x02DC, 0x0200 }, { 0x2013, 0x0210 }, { 0x2014, 0x0210 },
    { 0x2018, 0x0210 }, { 0x2019, 0x0210 }, { 0x201A, 0x0210 }, { 0x201C, 0x0210 }, { 0x201D, 0x0210 }, { 0x201E, 0x0210 },
    { 0x2020, 0x0210 }, { 0x2021, 0x0210 }, { 0x2022, 0x0210 }, { 0x2026, 0x0210 }, { 0x2030, 0x0210 }, { 0x2039, 0x0210 },
    { 0x203A, 0x0210 }, { 0x20AC, 0x0200 }, { 0x2122, 0x0200 },
};

/* Rows { lower, upper } of "Windows 8 Upper Case Mapping Table" ([MS-UCODEREF] 3.1.5.3, [MSDN-UCMT/Win8]; SHA-256
 * 76a66e29298eafc744fa1e8e11c80c3de28219c70539b3fb10efbccadb6ae335, 973 rows) that touch D: all 61, both ends inside D (decision 0087). A unit
 * without a row is unchanged (U+00B5, U+00DF, U+00AA, U+00BA are not in the table). The lower case map is the INVERSE of these rows: a reading
 * of the investigator, not a Microsoft statement (Microsoft gives no lower case table). Copy of lift/runtime/test/case_ms8_d.txt. */
static const uint16_t fle_ms8_d[61][2] = {
    { 0x0061, 0x0041 }, { 0x0062, 0x0042 }, { 0x0063, 0x0043 }, { 0x0064, 0x0044 }, { 0x0065, 0x0045 }, { 0x0066, 0x0046 },
    { 0x0067, 0x0047 }, { 0x0068, 0x0048 }, { 0x0069, 0x0049 }, { 0x006A, 0x004A }, { 0x006B, 0x004B }, { 0x006C, 0x004C },
    { 0x006D, 0x004D }, { 0x006E, 0x004E }, { 0x006F, 0x004F }, { 0x0070, 0x0050 }, { 0x0071, 0x0051 }, { 0x0072, 0x0052 },
    { 0x0073, 0x0053 }, { 0x0074, 0x0054 }, { 0x0075, 0x0055 }, { 0x0076, 0x0056 }, { 0x0077, 0x0057 }, { 0x0078, 0x0058 },
    { 0x0079, 0x0059 }, { 0x007A, 0x005A }, { 0x00E0, 0x00C0 }, { 0x00E1, 0x00C1 }, { 0x00E2, 0x00C2 }, { 0x00E3, 0x00C3 },
    { 0x00E4, 0x00C4 }, { 0x00E5, 0x00C5 }, { 0x00E6, 0x00C6 }, { 0x00E7, 0x00C7 }, { 0x00E8, 0x00C8 }, { 0x00E9, 0x00C9 },
    { 0x00EA, 0x00CA }, { 0x00EB, 0x00CB }, { 0x00EC, 0x00CC }, { 0x00ED, 0x00CD }, { 0x00EE, 0x00CE }, { 0x00EF, 0x00CF },
    { 0x00F0, 0x00D0 }, { 0x00F1, 0x00D1 }, { 0x00F2, 0x00D2 }, { 0x00F3, 0x00D3 }, { 0x00F4, 0x00D4 }, { 0x00F5, 0x00D5 },
    { 0x00F6, 0x00D6 }, { 0x00F8, 0x00D8 }, { 0x00F9, 0x00D9 }, { 0x00FA, 0x00DA }, { 0x00FB, 0x00DB }, { 0x00FC, 0x00DC },
    { 0x00FD, 0x00DD }, { 0x00FE, 0x00DE }, { 0x00FF, 0x0178 }, { 0x0153, 0x0152 }, { 0x0161, 0x0160 }, { 0x017E, 0x017D },
    { 0x0192, 0x0191 },
};

static uint16_t fle_case_d(uint16_t c, int upper)
{
    size_t i;

    for (i = 0; i < sizeof fle_ms8_d / sizeof fle_ms8_d[0]; i++)
        if (fle_ms8_d[i][upper ? 0 : 1] == c) return fle_ms8_d[i][upper ? 1 : 0];
    return c;
}

/* kernel32!GetStringTypeW: BOOL f(DWORD dwInfoType, LPCWCH lpSrcStr, int cchSrc, LPWORD lpCharType) (decision 0087; ecx, rdx, r8d, r9).
 * Learn rules used: lpSrcStr and lpCharType the same pointer fails with ERROR_INVALID_PARAMETER; a negative cchSrc means NUL-terminated and
 * the NUL is typed too (its character type is retrieved); otherwise exactly cchSrc units are typed; success is nonzero (1) and sets no error.
 * Only CT_CTYPE1 (1) and only a string of the range D with C1 values (283 of its 284 units; U+0191 was not measured) are answered. Stops by
 * name, before anything is written: CT_CTYPE2, CT_CTYPE3 and every other dwInfoType (the page lists no other value and no error for it),
 * a NULL pointer or cchSrc 0 (the page gives no rule), a unit outside D, U+0191. The values are unofficial (see fle_c1_latin1). */
void fle_imp_GetStringTypeW(void)
{
    static const char api[] = "GetStringTypeW";
    uint32_t ty = (uint32_t)R[1];
    const uint16_t *src = (const uint16_t *)fle_addr_opt(R[2], 2, 0);
    int32_t cch = (int32_t)R[8];
    uint16_t *out = (uint16_t *)fle_addr_opt(R[9], 2, 1);
    size_t n, i, k;
    uint16_t v;

    if (ty != 1u) fle_locale_stop(api, "dwInfoType other than CT_CTYPE1: decision 0087 gives C1 values only (CT_CTYPE2 and CT_CTYPE3 have no data here)");
    if (src == NULL || out == NULL || cch == 0)
        fle_locale_stop(api, "NULL lpSrcStr or lpCharType, or cchSrc 0: the page gives no rule for it");
    if ((const void *)src == (const void *)out) {
        fle_fail(FLE_ERR_INVALID_PARAMETER);
        return;
    }
    n = cch < 0 ? fle_wlen(src) + 1 : (size_t)cch;
    for (i = 0; i < n; i++) {
        if (!fle_in_d(src[i])) fle_stop_outside_d(api, src[i]);
        if (src[i] >= 0x100) {
            for (k = 0; k < sizeof fle_c1_high / sizeof fle_c1_high[0] && fle_c1_high[k][0] != src[i]; k++) {}
            if (k == sizeof fle_c1_high / sizeof fle_c1_high[0]) {
                fprintf(stderr, "fle: %s: U+%04X is in the range D of decision 0087 but has no C1 value (Wine was not run on it; not implemented)\n", api, (unsigned)src[i]);
                abort();
            }
        }
    }
    for (i = 0; i < n; i++) {
        if (src[i] < 0x100) {
            v = fle_c1_latin1[src[i]];
        } else {
            for (k = 0; fle_c1_high[k][0] != src[i]; k++) {}
            v = fle_c1_high[k][1];
        }
        out[i] = v;
    }
    R[0] = 1;
}

/* kernel32!IsValidCodePage: BOOL IsValidCodePage(UINT CodePage ecx) (decision 0078). The page: nonzero if the code page is valid, 0 if
 * invalid; "a code page is considered valid only if it is installed on the operating system". The page does not list which code pages
 * are installed. The two the environment itself has chosen (the ANSI and the OEM code page of fle_emulated_locale: they are the system's
 * current code pages, so they are installed) are valid; every other identifier is not decided by the page and stops the run. */
void fle_imp_IsValidCodePage(void)
{
    uint32_t cp = (uint32_t)R[1];

    if (cp == fle_emulated_locale.acp || cp == fle_emulated_locale.oemcp) {
        R[0] = 1;
        return;
    }
    fle_locale_stop("IsValidCodePage", "a code page other than the emulated ANSI and OEM code pages (the page does not list the installed ones)");   /* decision 0209: soft: FALSE ("otherwise, the return value is zero") */
}

/* CP_ACP (0), CP_OEMCP (1), CP_THREAD_ACP (3) resolved to a code page. CP_MACCP (2): the Macintosh
 * code page is not defined here and stops. CP_THREAD_ACP follows the thread locale; only the emulated
 * locale has a code page here. */
static uint32_t fle_cp_resolve(const char *api, uint32_t cp)
{
    switch (cp) {
    case 0: return fle_emulated_locale.acp;
    case 1: return fle_emulated_locale.oemcp;
    case 2: fle_locale_stop(api, "CP_MACCP"); return 0;
    case 3:
        if (fle_thread_locale() != fle_emulated_locale.lcid)
            fle_locale_stop(api, "CP_THREAD_ACP for a thread locale other than the emulated one");
        return fle_emulated_locale.acp;
    default: return cp;
    }
}

static int fle_cp_is_env_sbcs(uint32_t cp)
{
    return cp == fle_emulated_locale.acp || cp == fle_emulated_locale.oemcp;
}

/* kernel32!IsDBCSLeadByteEx: BOOL IsDBCSLeadByteEx(UINT CodePage, BYTE TestChar). The page says the
 * function "validates lead byte values only in code pages 932, 936, 949, 950, and 1361", so any other
 * code page answers 0. The lead byte ranges of those five are not on the page and stop. */
void fle_imp_IsDBCSLeadByteEx(void)
{
    uint32_t cp = fle_cp_resolve("IsDBCSLeadByteEx", (uint32_t)R[1]);

    if (cp == 932 || cp == 936 || cp == 949 || cp == 950 || cp == 1361)
        fle_locale_stop("IsDBCSLeadByteEx", "lead byte ranges of a DBCS code page are not on the MSDN page");
    R[0] = 0;
}

/* kernel32!GetCPInfo: BOOL GetCPInfo(UINT CodePage, LPCPINFO lpCPInfo). CPINFO is MaxCharSize (UINT),
 * DefaultChar[2], LeadByte[12]: 20 bytes (CPINFO page, MAX_DEFAULTCHAR 2 and MAX_LEADBYTES 12 from
 * winnls.h). For the two environment code pages (single-byte, general knowledge): MaxCharSize 1,
 * DefaultChar "?" (the page: "the default is usually '?'"), no lead bytes (page: every element 0).
 * Other code pages stop. A NULL pointer fails with ERROR_INVALID_PARAMETER (page). */
void fle_imp_GetCPInfo(void)
{
    uint32_t cp = fle_cp_resolve("GetCPInfo", (uint32_t)R[1]);
    uint8_t *info = (uint8_t *)fle_addr_opt(R[2], 20, 1);

    /* Decision 0116 (PROVISIONAL), CP_UTF8 (65001), asked by the Delphi encoding class of FLEngine (f_452960, TEncoding.UTF8). Learn (CPINFO): LeadByte "is not even
     * provided for many multi-byte encodings, for example, UTF-8 and GB18030" (so all 0), "MaxCharSize ... a value larger than 2 for other character set types" (the
     * exact UTF-8 value is NOT on the page: 4, the longest UTF-8 sequence of a Unicode scalar value, RFC 3629, is a choice), DefaultChar "usually the '?' character". */
    if (cp == 65001u) {
        if (info == NULL) {
            fle_fail(FLE_ERR_INVALID_PARAMETER);
            return;
        }
        memset(info, 0, 20);
        info[0] = 4;    /* MaxCharSize */
        info[4] = '?';  /* DefaultChar[0] */
        R[0] = 1;
        return;
    }
    if (!fle_cp_is_env_sbcs(cp))
        fle_locale_stop("GetCPInfo", "code page other than the emulated ANSI and OEM code pages");   /* decision 0209: soft: 0 and ERROR_INVALID_PARAMETER (the page) */
    if (info == NULL) {
        fle_fail(FLE_ERR_INVALID_PARAMETER);
        return;
    }
    memset(info, 0, 20);
    info[0] = 1;    /* MaxCharSize */
    info[4] = '?';  /* DefaultChar[0] */
    R[0] = 1;
}

/* GetLocaleInfoW values: only those that an MSDN page states for en-US (the LOCALE_ILANGUAGE page:
 * "0409"; LOCALE_SABBREVLANGNAME: "ENU"; LOCALE_SNAME: "en-US"; LOCALE_SISO* pages: "en", "eng", "US",
 * "USA"; LOCALE_SCURRENCY: "$" for the United States) and the two code pages from the emulated
 * locale, plus LOCALE_IOPTIONALCALENDAR "0" (unofficial source, decision 0041). Other LCTYPEs stop. Returns 1 when known; the string goes in buf, and *is_num tells that
 * the type is an integer type (LOCALE_I*, may be combined with LOCALE_RETURN_NUMBER) whose value is
 * *num. */
#define FLE_LC_ILANGUAGE            0x00000001u
#define FLE_LC_SABBREVLANGNAME      0x00000003u
#define FLE_LC_IDEFAULTCODEPAGE     0x0000000Bu
#define FLE_LC_SCURRENCY            0x00000014u
#define FLE_LC_IDEFAULTANSICODEPAGE 0x00001004u
#define FLE_LC_IOPTIONALCALENDAR    0x0000100Bu
#define FLE_LC_SISO639LANGNAME      0x00000059u
#define FLE_LC_SISO3166CTRYNAME     0x0000005Au
#define FLE_LC_SNAME                0x0000005Cu
#define FLE_LC_SISO639LANGNAME2     0x00000067u
#define FLE_LC_SISO3166CTRYNAME2    0x00000068u

/* en-US values added by docs/decisions/0042-locale-enus-values.md. NOT from MSDN (the pages give no en-US
 * value for these); each value is one on which Wine's en-US data (5.0 enu.nls and the current locale.nls)
 * and CLDR en agree; provisional, not checked on real Windows. The six LCTypes on which those sources
 * disagreed (0042 section 3) are answered by docs/decisions/0048-locale-six.md (below, after this block). */
#define FLE_LC_SLANGUAGE            0x00000002u
#define FLE_LC_SDECIMAL             0x0000000Eu
#define FLE_LC_STHOUSAND            0x0000000Fu
#define FLE_LC_ICURRDIGITS          0x00000019u
#define FLE_LC_ICURRENCY            0x0000001Bu
#define FLE_LC_SDATE                0x0000001Du
#define FLE_LC_STIME                0x0000001Eu
#define FLE_LC_ITIME                0x00000023u
#define FLE_LC_ITLZERO              0x00000025u
#define FLE_LC_S1159                0x00000028u
#define FLE_LC_S2359                0x00000029u
#define FLE_LC_SDAYNAME1            0x0000002Au /* ..SDAYNAME7 0x30 */
#define FLE_LC_SABBREVDAYNAME1      0x00000031u /* ..SABBREVDAYNAME7 0x37 */
#define FLE_LC_SMONTHNAME1          0x00000038u /* ..SMONTHNAME12 0x43 (the 13th, 0x100e, is not requested) */
#define FLE_LC_SABBREVMONTHNAME1    0x00000044u /* ..SABBREVMONTHNAME12 0x4f */
#define FLE_LC_ITIMEMARKPOSN        0x00001005u
#define FLE_LC_ICALENDARTYPE        0x00001009u

/* en-US values added by docs/decisions/0048-locale-six.md (investigation:
 * docs/investigations/locale-6-official-sources.md). Provisional, NOT checked on real Windows. The format
 * strings use the NLS notation (M/d/yyyy, tt), a plain U+0020 before "tt" (not U+202F), and are ASCII only.
 * Strength: SLIST official (Learn Excel article); SSHORTDATE, STIMEFORMAT, SSHORTTIME official examples plus
 * independent sources; SLONGDATE weak (official examples disagree on d/dd); INEGCURR weakest (unofficial
 * Wine/registry dumps and one official .NET example; ICU/CLDR says 1). */
#define FLE_LC_SLIST                0x0000000Cu
#define FLE_LC_INEGCURR             0x0000001Cu
#define FLE_LC_SSHORTDATE           0x0000001Fu
#define FLE_LC_SLONGDATE            0x00000020u
#define FLE_LC_STIMEFORMAT          0x00001003u
#define FLE_LC_SSHORTTIME           0x00000079u

#define FLE_LC_NOUSEROVERRIDE       0x80000000u
#define FLE_LC_USE_CP_ACP           0x40000000u
#define FLE_LC_RETURN_NUMBER        0x20000000u
#define FLE_LC_RETURN_GENITIVE      0x10000000u

static int fle_locale_info_value(uint32_t type, char *buf, size_t cap, int *is_num, uint32_t *num)
{
    *is_num = 0;
    *num = 0;
    switch (type) {
    case FLE_LC_ILANGUAGE:
        snprintf(buf, cap, "%04X", fle_emulated_locale.lcid & 0xFFFFu);
        return 1;
    case FLE_LC_SABBREVLANGNAME:
        if (fle_emulated_locale.lcid != 0x0409) return 0;
        snprintf(buf, cap, "ENU");
        return 1;
    case FLE_LC_SISO639LANGNAME:
    case FLE_LC_SISO639LANGNAME2:
    case FLE_LC_SISO3166CTRYNAME:
    case FLE_LC_SISO3166CTRYNAME2:
    case FLE_LC_SNAME:
    case FLE_LC_SCURRENCY:
        if (fle_emulated_locale.lcid != 0x0409) return 0;
        snprintf(buf, cap, "%s",
                 type == FLE_LC_SISO639LANGNAME ? "en" :
                 type == FLE_LC_SISO639LANGNAME2 ? "eng" :
                 type == FLE_LC_SISO3166CTRYNAME ? "US" :
                 type == FLE_LC_SISO3166CTRYNAME2 ? "USA" :
                 type == FLE_LC_SNAME ? "en-US" : "$");
        return 1;
    case FLE_LC_IDEFAULTANSICODEPAGE:
        *is_num = 1;
        *num = fle_emulated_locale.acp;
        snprintf(buf, cap, "%u", *num);
        return 1;
    case FLE_LC_IDEFAULTCODEPAGE:
        *is_num = 1;
        *num = fle_emulated_locale.oemcp;
        snprintf(buf, cap, "%u", *num);
        return 1;
    case FLE_LC_IOPTIONALCALENDAR:
        /* en-US has no optional calendar besides the default Gregorian one: "0". NOT from MSDN (the
         * LOCALE_IOPTIONALCALENDAR page gives no value); unofficial source, Wine's en-US locale data
         * (docs/decisions/0041-locale-lctype-100b.md). Provisional. */
        if (fle_emulated_locale.lcid != 0x0409) return 0;
        *is_num = 1;
        *num = 0;
        snprintf(buf, cap, "%u", *num);
        return 1;
    case FLE_LC_SLANGUAGE:
    case FLE_LC_SDECIMAL:
    case FLE_LC_STHOUSAND:
    case FLE_LC_SDATE:
    case FLE_LC_STIME:
    case FLE_LC_S1159:
    case FLE_LC_S2359:
        if (fle_emulated_locale.lcid != 0x0409) return 0;
        snprintf(buf, cap, "%s",
                 type == FLE_LC_SLANGUAGE ? "English (United States)" :
                 type == FLE_LC_SDECIMAL ? "." :
                 type == FLE_LC_STHOUSAND ? "," :
                 type == FLE_LC_SDATE ? "/" :
                 type == FLE_LC_STIME ? ":" :
                 type == FLE_LC_S1159 ? "AM" : "PM");
        return 1;
    case FLE_LC_SLIST:
    case FLE_LC_SSHORTDATE:
    case FLE_LC_SLONGDATE:
    case FLE_LC_STIMEFORMAT:
    case FLE_LC_SSHORTTIME:
        if (fle_emulated_locale.lcid != 0x0409) return 0;
        snprintf(buf, cap, "%s",
                 type == FLE_LC_SLIST ? "," :
                 type == FLE_LC_SSHORTDATE ? "M/d/yyyy" :
                 type == FLE_LC_SLONGDATE ? "dddd, MMMM d, yyyy" :
                 type == FLE_LC_STIMEFORMAT ? "h:mm:ss tt" : "h:mm tt");
        return 1;
    case FLE_LC_INEGCURR:
        if (fle_emulated_locale.lcid != 0x0409) return 0;
        *is_num = 1;
        *num = 0;
        snprintf(buf, cap, "%u", *num);
        return 1;
    case FLE_LC_ICURRDIGITS:
    case FLE_LC_ICURRENCY:
    case FLE_LC_ITIME:
    case FLE_LC_ITLZERO:
    case FLE_LC_ITIMEMARKPOSN:
    case FLE_LC_ICALENDARTYPE:
        if (fle_emulated_locale.lcid != 0x0409) return 0;
        *is_num = 1;
        *num = type == FLE_LC_ICURRDIGITS ? 2u : type == FLE_LC_ICALENDARTYPE ? 1u : 0u;
        snprintf(buf, cap, "%u", *num);
        return 1;
    default:
        break;
    }
    if (fle_emulated_locale.lcid != 0x0409) return 0;
    {
        /* LOCALE_SDAYNAME1..7 and LOCALE_SABBREVDAYNAME1..7 are Monday..Sunday (MSDN: "Native long name
         * for Monday" ... "for Sunday"); LOCALE_SMONTHNAME1..12 and LOCALE_SABBREVMONTHNAME1..12 are
         * January..December. The names are the 0042 values (Wine and CLDR agree). */
        static const char *const day[7] = { "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday" };
        static const char *const aday[7] = { "Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun" };
        static const char *const mon[12] = { "January", "February", "March", "April", "May", "June", "July",
                                             "August", "September", "October", "November", "December" };
        static const char *const amon[12] = { "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep",
                                              "Oct", "Nov", "Dec" };
        const char *s = NULL;

        if (type >= FLE_LC_SDAYNAME1 && type < FLE_LC_SDAYNAME1 + 7u) s = day[type - FLE_LC_SDAYNAME1];
        else if (type >= FLE_LC_SABBREVDAYNAME1 && type < FLE_LC_SABBREVDAYNAME1 + 7u) s = aday[type - FLE_LC_SABBREVDAYNAME1];
        else if (type >= FLE_LC_SMONTHNAME1 && type < FLE_LC_SMONTHNAME1 + 12u) s = mon[type - FLE_LC_SMONTHNAME1];
        else if (type >= FLE_LC_SABBREVMONTHNAME1 && type < FLE_LC_SABBREVMONTHNAME1 + 12u) s = amon[type - FLE_LC_SABBREVMONTHNAME1];
        if (s == NULL) return 0;
        snprintf(buf, cap, "%s", s);
        return 1;
    }
}

/* kernel32!GetLocaleInfoW: int GetLocaleInfoW(LCID Locale, LCTYPE LCType, LPWSTR lpLCData, int cchData).
 * MSDN rules used: cchData 0 returns the required size in characters including the NUL and does not
 * touch the buffer; a buffer that is too small (or NULL with cchData > 0) returns 0 with
 * ERROR_INSUFFICIENT_BUFFER; success returns the number of characters written including the NUL;
 * LOCALE_RETURN_NUMBER on an integer type writes a DWORD (2 characters) and returns 2 (the buffer must be
 * at least 2 characters); a negative cchData is an invalid parameter (87). LOCALE_NOUSEROVERRIDE is
 * accepted (the environment has no user overrides); LOCALE_USE_CP_ACP is ignored (the page: relevant
 * only for the ANSI version). Stops: a locale other than the emulated one (and the two defaults), an
 * LCType not in the value table above, LOCALE_RETURN_GENITIVE_NAMES, LOCALE_RETURN_NUMBER with a
 * non-integer type or ILANGUAGE (hexadecimal string) or with cchData 0 (the page does not define the
 * size). */
void fle_imp_GetLocaleInfoW(void)
{
    uint32_t lcid = (uint32_t)R[1], lctype = (uint32_t)R[2], base, num;
    uint16_t *out = (uint16_t *)fle_addr_opt(R[8], 2, 1);
    int32_t cch = (int32_t)R[9];
    int is_num, want_num = (lctype & FLE_LC_RETURN_NUMBER) != 0;
    char s[32];
    size_t n, i;

    fle_require_emulated("GetLocaleInfoW", lcid);
    if (lctype & FLE_LC_RETURN_GENITIVE)
        fle_locale_stop("GetLocaleInfoW", "LOCALE_RETURN_GENITIVE_NAMES");
    base = lctype & ~(FLE_LC_NOUSEROVERRIDE | FLE_LC_USE_CP_ACP | FLE_LC_RETURN_NUMBER | FLE_LC_RETURN_GENITIVE);
    if (!fle_locale_info_value(base, s, sizeof s, &is_num, &num))
        fle_locale_stop("GetLocaleInfoW", "LCType value for en-US is not on the MSDN pages");   /* decision 0209: soft: 0 and ERROR_INVALID_FLAGS (the page) */
    if (cch < 0) {
        fle_fail(FLE_ERR_INVALID_PARAMETER);
        return;
    }
    if (want_num) {
        if (!is_num) fle_locale_stop("GetLocaleInfoW", "LOCALE_RETURN_NUMBER with a type that is not an integer type here");
        if (cch == 0) fle_locale_stop("GetLocaleInfoW", "LOCALE_RETURN_NUMBER with cchData 0: size not defined by the page");
        if (cch < 2 || out == NULL) {
            fle_fail(FLE_ERR_INSUFFICIENT_BUFFER);
            return;
        }
        memcpy(out, &num, 4);
        R[0] = 2;
        return;
    }
    n = strlen(s) + 1;
    if (cch == 0) {
        R[0] = n;
        return;
    }
    if (out == NULL || (size_t)cch < n) {
        fle_fail(FLE_ERR_INSUFFICIENT_BUFFER);
        return;
    }
    for (i = 0; i < n; i++) out[i] = (uint8_t)s[i];
    R[0] = n;
}

/* ---- code page conversion ---------------------------------------------------------------------
 * UTF-8 (CP_UTF8) follows the Unicode encoding rules (MSDN: "fully conforms to the Unicode 4.1
 * specification for UTF-8 and UTF-16"). The two environment code pages are converted only for ASCII
 * (bytes below 0x80); their tables for 0x80 and above are not on the MSDN pages and stop. */

/* Decodes one UTF-8 sequence at s (n bytes left). Returns its length, or 0 if it is not well formed
 * (overlong forms, surrogates, above U+10FFFF and truncated sequences are not well formed). */
static int fle_utf8_decode(const uint8_t *s, size_t n, uint32_t *cp)
{
    uint8_t b = s[0], lo, hi;

    if (b < 0x80) {
        *cp = b;
        return 1;
    }
    if (b >= 0xC2 && b <= 0xDF) {
        if (n < 2 || (s[1] & 0xC0) != 0x80) return 0;
        *cp = ((uint32_t)(b & 0x1F) << 6) | (s[1] & 0x3F);
        return 2;
    }
    if (b >= 0xE0 && b <= 0xEF) {
        lo = b == 0xE0 ? 0xA0 : 0x80;
        hi = b == 0xED ? 0x9F : 0xBF;
        if (n < 3 || s[1] < lo || s[1] > hi || (s[2] & 0xC0) != 0x80) return 0;
        *cp = ((uint32_t)(b & 0x0F) << 12) | ((uint32_t)(s[1] & 0x3F) << 6) | (s[2] & 0x3F);
        return 3;
    }
    if (b >= 0xF0 && b <= 0xF4) {
        lo = b == 0xF0 ? 0x90 : 0x80;
        hi = b == 0xF4 ? 0x8F : 0xBF;
        if (n < 4 || s[1] < lo || s[1] > hi || (s[2] & 0xC0) != 0x80 || (s[3] & 0xC0) != 0x80) return 0;
        *cp = ((uint32_t)(b & 0x07) << 18) | ((uint32_t)(s[1] & 0x3F) << 12) |
              ((uint32_t)(s[2] & 0x3F) << 6) | (s[3] & 0x3F);
        return 4;
    }
    return 0;
}

/* Bytes 0x80..0x9F of MBTABLE in CodpageFiles/1252.txt (decision 0084; [MS-UCODEREF] 1.2.1, 2.2.2, 3.1.5.1.1.3); the other bytes map to themselves.
 * "Windows Supported Code Page Data Files.zip" SHA-256 5074e6dd253056ba61fc6c870c9a955467855129c6ad3a51761c386b301b125a, 1252.txt SHA-256 72ea23c939c5b26fae7aded0207b327e2f3902d7d3c168d7087f5cfc38ee76a9 */
static const uint16_t fle_cp1252_hi[32] = {
    0x20AC, 0x0081, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160, 0x2039, 0x0152, 0x008D, 0x017D, 0x008F,
    0x0090, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014, 0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0x009D, 0x017E, 0x0178,
};

/* Converts n bytes. dst may be NULL (count only). Returns the number of UTF-16 units, or -1 if the
 * input has an ill-formed UTF-8 sequence (*bad set). cp1252 selects the table above for a byte of 0x80 or above. */
static int64_t fle_mb_to_wide(const char *api, const uint8_t *src, size_t n, uint16_t *dst, int utf8, int cp1252)
{
    size_t i = 0;
    int64_t out = 0;
    uint32_t cp;
    int len;

    while (i < n) {
        if (!utf8) {
            if (src[i] >= 0x80 && !cp1252) fle_locale_stop(api, "byte of 0x80 or above in a single-byte code page other than 1252: table not adopted (decision 0084 covers 1252 only)");
            if (dst) dst[out] = src[i] >= 0x80 && src[i] < 0xA0 ? fle_cp1252_hi[src[i] - 0x80] : src[i];
            out++;
            i++;
            continue;
        }
        len = fle_utf8_decode(src + i, n - i, &cp);
        if (len == 0) return -1;
        if (cp >= 0x10000) {
            if (dst) {
                dst[out] = (uint16_t)(0xD800 + ((cp - 0x10000) >> 10));
                dst[out + 1] = (uint16_t)(0xDC00 + ((cp - 0x10000) & 0x3FF));
            }
            out += 2;
        } else {
            if (dst) dst[out] = (uint16_t)cp;
            out++;
        }
        i += (size_t)len;
    }
    return out;
}

/* kernel32!MultiByteToWideChar: int f(UINT CodePage, DWORD dwFlags, LPCCH lpMultiByteStr, int cbMultiByte,
 * LPWSTR lpWideCharStr, int cchWideChar). MSDN rules used: cbMultiByte -1 means NUL-terminated and the
 * terminator is converted and counted; 0 fails; cchWideChar 0 returns the required size without using the
 * buffer; a small buffer fails with ERROR_INSUFFICIENT_BUFFER; the same pointer for both strings fails with
 * ERROR_INVALID_PARAMETER; for UTF-8 dwFlags must be 0 or MB_ERR_INVALID_CHARS (else ERROR_INVALID_FLAGS);
 * with MB_ERR_INVALID_CHARS an ill-formed sequence fails with ERROR_NO_UNICODE_TRANSLATION. Not defined by
 * the page and so a stop: an ill-formed UTF-8 sequence without MB_ERR_INVALID_CHARS (the page says only
 * "replaces illegal sequences with U+FFFD", not how many), a cbMultiByte below -1, MB_COMPOSITE and
 * MB_USEGLYPHCHARS on a single-byte code page, a code page other than UTF-8 and the two environment
 * code pages, and a byte of 0x80 or above in the OEM code page. For 1252 (decision 0084) the bytes 0x80..0x9F
 * come from fle_cp1252_hi, the others map to themselves, and MB_ERR_INVALID_CHARS never makes a byte invalid.
 * Nothing is written when the buffer is too small (the page does not say what the buffer holds on failure). */
void fle_imp_MultiByteToWideChar(void)
{
    static const char api[] = "MultiByteToWideChar";
    uint32_t cp = fle_cp_resolve(api, (uint32_t)R[1]), flags = (uint32_t)R[2];
    const uint8_t *src = (const uint8_t *)fle_addr_opt(R[8], 1, 0);
    int32_t cb = (int32_t)R[9];
    uint16_t *dst = (uint16_t *)fle_addr_opt(fle_stack_arg(5), 2, 1);
    int32_t cch = (int32_t)fle_stack_arg(6);
    int utf8 = cp == FLE_CP_UTF8;
    int cp1252 = cp == FLE_CP_1252;
    size_t n;
    int64_t count;

    if (!utf8 && !fle_cp_is_env_sbcs(cp))
        fle_locale_stop(api, "code page other than UTF-8 and the emulated ANSI and OEM code pages");   /* decision 0209: soft: 0 and ERROR_INVALID_PARAMETER (the page) */
    if (utf8) {
        if (flags & ~0x8u) {
            fle_fail(FLE_ERR_INVALID_FLAGS);
            return;
        }
    } else {
        if ((flags & ~0xFu) || (flags & 3u) == 3u) {
            fle_fail(FLE_ERR_INVALID_FLAGS);
            return;
        }
        if (flags & 6u) fle_locale_stop(api, "MB_COMPOSITE or MB_USEGLYPHCHARS: effect not defined by the page for this code page");
    }
    if (cb < -1) fle_locale_stop(api, "cbMultiByte below -1");
    if (src == NULL || cb == 0 || cch < 0 || (dst != NULL && (const void *)dst == (const void *)src)) {
        fle_fail(FLE_ERR_INVALID_PARAMETER);
        return;
    }
    n = cb == -1 ? strlen((const char *)src) + 1 : (size_t)cb;
    count = fle_mb_to_wide(api, src, n, NULL, utf8, cp1252);
    if (count < 0) {
        if (!(flags & 0x8u))
            fle_locale_stop(api, "ill-formed UTF-8 without MB_ERR_INVALID_CHARS: number of U+FFFD not defined by the page");
        fle_fail(FLE_ERR_NO_UNICODE_TRANSLATION);
        return;
    }
    if (cch == 0) {
        R[0] = (uint64_t)count;
        return;
    }
    if (dst == NULL || (int64_t)cch < count) {
        fle_fail(FLE_ERR_INSUFFICIENT_BUFFER);
        return;
    }
    (void)fle_mb_to_wide(api, src, n, dst, utf8, cp1252);
    R[0] = (uint64_t)count;
}

/* Converts n UTF-16 units for UTF-8 and (ASCII only) the OEM code page. dst may be NULL (count only). Returns the number of bytes, or -1 for a lone
 * surrogate when err_invalid is set. A lone surrogate otherwise becomes U+FFFD (EF BF BD in UTF-8). */
static int64_t fle_wide_to_mb(const char *api, const uint16_t *src, size_t n, uint8_t *dst, int utf8, int err_invalid)
{
    size_t i = 0;
    int64_t out = 0;
    uint32_t c, c2;
    uint8_t tmp[4];
    int len, k;

    while (i < n) {
        c = src[i];
        if (!utf8) {
            if (c >= 0x80) fle_locale_stop(api, "UTF-16 unit of 0x80 or above for a single-byte code page: table not on the MSDN pages");
            if (dst) dst[out] = (uint8_t)c;
            out++;
            i++;
            continue;
        }
        i++;
        if (c >= 0xD800 && c <= 0xDBFF && i < n && src[i] >= 0xDC00 && src[i] <= 0xDFFF) {
            c2 = src[i++];
            c = 0x10000 + ((c - 0xD800) << 10) + (c2 - 0xDC00);
        } else if (c >= 0xD800 && c <= 0xDFFF) {
            if (err_invalid) return -1;
            c = 0xFFFD;
        }
        if (c < 0x80) {
            tmp[0] = (uint8_t)c;
            len = 1;
        } else if (c < 0x800) {
            tmp[0] = (uint8_t)(0xC0 | (c >> 6));
            tmp[1] = (uint8_t)(0x80 | (c & 0x3F));
            len = 2;
        } else if (c < 0x10000) {
            tmp[0] = (uint8_t)(0xE0 | (c >> 12));
            tmp[1] = (uint8_t)(0x80 | ((c >> 6) & 0x3F));
            tmp[2] = (uint8_t)(0x80 | (c & 0x3F));
            len = 3;
        } else {
            tmp[0] = (uint8_t)(0xF0 | (c >> 18));
            tmp[1] = (uint8_t)(0x80 | ((c >> 12) & 0x3F));
            tmp[2] = (uint8_t)(0x80 | ((c >> 6) & 0x3F));
            tmp[3] = (uint8_t)(0x80 | (c & 0x3F));
            len = 4;
        }
        if (dst) for (k = 0; k < len; k++) dst[out + k] = tmp[k];
        out += len;
    }
    return out;
}

/* WideCharToMultiByte for code page 1252 (decision 0087), a string of the range D only (checked by the caller before anything is written).
 * One unit gives one byte. WCTABLE of CodpageFiles/1252.txt (decision 0084's zip; [MS-UCODEREF] 3.1.5.1.1.2): U+0000..U+007F and
 * U+00A0..U+00FF map to themselves; of U+0080..U+009F only U+0081, U+008D, U+008F, U+0090, U+009D have a row (themselves); the 27 units above
 * U+00FF with a row are exactly the entries of fle_cp1252_hi (byte 0x80 + index); every other unit of D has no row and becomes the default
 * character, except U+0191, whose row is a best-fit one (-> 0x83). The page's WC_NO_BEST_FIT_CHARS text ("if translating from Unicode to
 * multibyte and back to Unicode again does not yield the same Unicode character, the function uses the default character") is read as: with
 * 0x400 only the exact inverse of MBTABLE (256 rows) converts, so U+0191 becomes the default character. The pseudocode has no dwFlags and no
 * lpUsedDefaultChar; *used is set when a default character was used (a best-fit row does not count). dst may be NULL (count only);
 * at most cap bytes are written. Returns the byte count, which is n. */
static int64_t fle_wide_to_mb_1252(const uint16_t *src, size_t n, uint8_t *dst, size_t cap, uint32_t flags, uint8_t defb, int *used)
{
    size_t i, k;
    uint32_t c;
    uint8_t b;
    int def;

    *used = 0;
    for (i = 0; i < n; i++) {
        c = src[i];
        def = 0;
        b = defb;
        if (c < 0x80 || (c >= 0xA0 && c < 0x100)) {
            b = (uint8_t)c;
        } else {
            for (k = 0; k < 32 && fle_cp1252_hi[k] != c; k++) {}
            if (k < 32) b = (uint8_t)(0x80 + k);
            else if (c == 0x0191 && !(flags & 0x400u)) b = 0x83;
            else def = 1;
        }
        if (def) *used = 1;
        if (dst && i < cap) dst[i] = b;
    }
    return (int64_t)n;
}

/* kernel32!WideCharToMultiByte: int f(UINT CodePage, DWORD dwFlags, LPCWCH lpWideCharStr, int cchWideChar,
 * LPSTR lpMultiByteStr, int cbMultiByte, LPCCH lpDefaultChar, LPBOOL lpUsedDefaultChar). Same size,
 * terminator and error rules as MultiByteToWideChar, plus: for UTF-8, dwFlags must be 0 or WC_ERR_INVALID_CHARS
 * (else ERROR_INVALID_FLAGS) and lpDefaultChar and lpUsedDefaultChar must be NULL (else ERROR_INVALID_PARAMETER);
 * with WC_ERR_INVALID_CHARS a lone surrogate fails with ERROR_NO_UNICODE_TRANSLATION; without it a lone
 * surrogate becomes U+FFFD (the page: "replaces illegal sequences with U+FFFD"; one per lone unit is a reading).
 * For the OEM code page (437) only dwFlags 0 or WC_NO_BEST_FIT_CHARS and only ASCII input are converted (no character is replaced, so
 * *lpUsedDefaultChar is set to FALSE); everything else stops. For code page 1252 (CP_ACP, CP_THREAD_ACP at the emulated locale included;
 * decision 0087) dwFlags 0 or 0x400 and a string of the range D are converted by fle_wide_to_mb_1252; a unit outside D stops by name before
 * anything is written. lpDefaultChar (NULL: the CPINFO default 0x3f) and lpUsedDefaultChar are honoured for 1252. A buffer smaller than the
 * result fails with ERROR_INSUFFICIENT_BUFFER; for 1252 with a positive cchWideChar the first cbMultiByte bytes are written first (the page:
 * "If cbMultiByte is less than cchWideChar, this function writes the number of characters specified by cbMultiByte"). */
void fle_imp_WideCharToMultiByte(void)
{
    static const char api[] = "WideCharToMultiByte";
    uint32_t cp = fle_cp_resolve(api, (uint32_t)R[1]), flags = (uint32_t)R[2];
    const uint16_t *src = (const uint16_t *)fle_addr_opt(R[8], 2, 0);
    int32_t cw = (int32_t)R[9];
    uint8_t *dst = (uint8_t *)fle_addr_opt(fle_stack_arg(5), 1, 1);
    int32_t cb = (int32_t)fle_stack_arg(6);
    const uint8_t *defchar = (const uint8_t *)fle_addr_opt(fle_stack_arg(7), 1, 0);
    uint32_t *used = (uint32_t *)fle_addr_opt(fle_stack_arg(8), 4, 1);
    int utf8 = cp == FLE_CP_UTF8;
    size_t n;
    int64_t count;

    if (!utf8 && !fle_cp_is_env_sbcs(cp))
        fle_locale_stop(api, "code page other than UTF-8 and the emulated ANSI and OEM code pages");   /* decision 0209: soft: 0 and ERROR_INVALID_PARAMETER (the page) */
    if (utf8) {
        if (flags & ~0x80u) {
            fle_fail(FLE_ERR_INVALID_FLAGS);
            return;
        }
        if (defchar != NULL || used != NULL) {
            fle_fail(FLE_ERR_INVALID_PARAMETER);
            return;
        }
    } else if (flags & ~0x400u) {
        fle_locale_stop(api, "dwFlags other than 0 and WC_NO_BEST_FIT_CHARS for a single-byte code page");
    }
    if (cw < -1) fle_locale_stop(api, "cchWideChar below -1");
    if (src == NULL || cw == 0 || cb < 0 || (dst != NULL && (const void *)dst == (const void *)src)) {
        fle_fail(FLE_ERR_INVALID_PARAMETER);
        return;
    }
    n = cw == -1 ? 0 : (size_t)cw;
    if (cw == -1) {
        while (src[n] != 0) n++;
        n++;
    }
    if (cp == FLE_CP_1252) {
        size_t i;
        int u;
        uint8_t defb = defchar != NULL ? *defchar : 0x3F; /* CPINFO of 1252.txt: default char 0x3f */

        for (i = 0; i < n; i++)
            if (!fle_in_d(src[i])) fle_stop_outside_d(api, src[i]);
        count = fle_wide_to_mb_1252(src, n, NULL, 0, flags, defb, &u);
        if (cb == 0) {
            if (used != NULL) *used = (uint32_t)u;
            R[0] = (uint64_t)count;
            return;
        }
        if (dst == NULL || (int64_t)cb < count) {
            if (dst != NULL && cw > 0) (void)fle_wide_to_mb_1252(src, n, dst, (size_t)cb, flags, defb, &u);
            fle_fail(FLE_ERR_INSUFFICIENT_BUFFER);
            return;
        }
        (void)fle_wide_to_mb_1252(src, n, dst, (size_t)cb, flags, defb, &u);
        if (used != NULL) *used = (uint32_t)u;
        R[0] = (uint64_t)count;
        return;
    }
    count = fle_wide_to_mb(api, src, n, NULL, utf8, (flags & 0x80u) != 0);
    if (count < 0) {
        fle_fail(FLE_ERR_NO_UNICODE_TRANSLATION);
        return;
    }
    if (cb == 0) {
        R[0] = (uint64_t)count;
        return;
    }
    if (dst == NULL || (int64_t)cb < count) {
        fle_fail(FLE_ERR_INSUFFICIENT_BUFFER);
        return;
    }
    (void)fle_wide_to_mb(api, src, n, dst, utf8, (flags & 0x80u) != 0);
    if (!utf8 && used != NULL) *used = 0;
    R[0] = (uint64_t)count;
}

/* ---- LCMapStringW and CompareStringW ----------------------------------------------------------- */

static size_t fle_wlen(const uint16_t *s)
{
    size_t n = 0;
    while (s[n] != 0) n++;
    return n;
}

/* Every flag the LCMapStringEx and CompareStringEx pages define (LCMAP_*, NORM_*, LINGUISTIC_*,
 * SORT_*); a bit outside this set is ERROR_INVALID_FLAGS. Values from winnls.h. */
#define FLE_LCMAP_LOWERCASE        0x00000100u
#define FLE_LCMAP_UPPERCASE        0x00000200u
#define FLE_LCMAP_BYTEREV          0x00000800u
#define FLE_LCMAP_LINGUISTIC_CASING 0x01000000u
#define FLE_ALL_MAP_FLAGS (0x00000001u | 0x00000002u | 0x00000004u | 0x00000008u | 0x00000010u | 0x00000020u | \
                           0x00000100u | 0x00000200u | 0x00000400u | 0x00000800u | 0x00001000u | 0x00010000u | \
                           0x00020000u | 0x00040000u | 0x00100000u | 0x00200000u | 0x00400000u | 0x00800000u | \
                           0x01000000u | 0x02000000u | 0x04000000u | 0x08000000u | 0x20000000u)
#define FLE_ALL_CMP_FLAGS (0x00000001u | 0x00000002u | 0x00000004u | 0x00000008u | 0x00000010u | 0x00000020u | \
                           0x00001000u | 0x00010000u | 0x00020000u | 0x08000000u)

/* Decision 0191 (PROVISIONAL; decision 0284: no longer opt-in): the primary sort weights of Microsoft's published Sorting Weight Table decide a pair of units where the old code stopped (fle_wall_ia2.c). Until decision 0284
 * this was on only in a build with the row "CompareStringW::ia2-sort-weights" of harness/seeds/ia2-imports.txt (every shipped configuration had it); the row and the switch are gone (docs/policies/no-options.md). */
/* ---- CompareStringW: the part decided by Microsoft's published sort weights (wall ia2-W19; decision 0284: these functions moved here from fle_wall_ia2.c, so that every link that holds fle_locale.c holds the table) ----
 * Source: "Sorting Weight Tables" (Microsoft Download Center id=10921), "Windows 10 Sorting Weight Table.txt", SHA-256 in fle_ia2_sortw.inc. Its header: "Weights are 4 bytes: SM, AW, DW & CW. Together the SM + AW are
 * 'Unicode Weight' ... For the most part, strings are sorted with all of the 'Unicode Weights' being primary, then the diacritic and case weights as being secondary and tertiary weights." Used here, and only here:
 * two code units whose primary weights (SM, then AW; compared as numbers) differ are ordered by them (smaller primary weight = less). Not used (the unit pair is left to the old code, which stops by name): equal primary
 * weights (the order then depends on the secondary and tertiary weights and on the unpublished algorithm around them), a unit that has SM 0 (no weight) or SM 1 (non-spacing mark), a unit that is not in the
 * default table, two units of the same script member of which one has no AW in fle_ia2_sortw.inc (the ideographs: 27,639 entries of SM 5 are not carried; their order among themselves is in the table but not in this
 * build), the hyphen and apostrophe (the word sort treats them apart, "Handling Sorting in Your Applications"), and everything that is not a single code unit against a single code unit at the first position
 * where the strings differ. The table is Windows 10's: which Windows version's table the guest program expects is not known (the Windows 7/8/11 tables are on the same page, not compared). S-ia2-sortw-version.
 * That the Microsoft table orders the kana (SM 3) and the ideographs (SM 5) BEFORE the symbols (SM 6-12), the digits (13) and the Latin letters (14) is the table's own number order; it is the opposite of decision 0178's
 * S-dlg1-cmp-nonascii (CHOICE WITHOUT A SOURCE: "the ASCII unit is the smaller"), which this does not replace (that rule stays opt-in under FLE_CMP_EXTEND and is only reached for pairs this code leaves undecided).
 * Whether the real CompareStringW agrees was not compared with a machine (0 comparisons). */
#include "fle_modules.h"
#include "fle_ia2_sortw.inc"
/* SM known (and AW if *has_aw): 1 when the table gives the unit a primary weight */
static int ia2_unit_weight(uint16_t c, unsigned *sm, unsigned *aw, int *has_aw)
{
    unsigned lo, hi, mid;

    if (c >= 0x20 && c <= 0x7e) {
        *sm = ia2_ascii[c - 0x20].sm;
        *aw = ia2_ascii[c - 0x20].aw;
        *has_aw = 1;
        return 1;
    }
    if (c < 0x20 || c == 0x7f) {   /* decision 0284: the ASCII control units that the default table gives a primary weight (U+0000 has none) */
        for (lo = 0; lo < sizeof ia2_ctl / sizeof ia2_ctl[0]; lo++)
            if (ia2_ctl[lo].cp == c) {
                *sm = ia2_ctl[lo].sm;
                *aw = ia2_ctl[lo].aw;
                *has_aw = 1;
                return 1;
            }
        return 0;
    }
    lo = 0;
    hi = sizeof ia2_full / sizeof ia2_full[0];
    while (lo < hi) {
        mid = (lo + hi) / 2;
        if (ia2_full[mid].cp == c) {
            *sm = ia2_full[mid].sm;
            *aw = ia2_full[mid].aw;
            *has_aw = 1;
            return 1;
        }
        if (ia2_full[mid].cp < c) lo = mid + 1; else hi = mid;
    }
    lo = 0;
    hi = sizeof ia2_smr / sizeof ia2_smr[0];
    while (lo < hi) {
        mid = (lo + hi) / 2;
        if (c < ia2_smr[mid].lo) hi = mid;
        else if (c > ia2_smr[mid].hi) lo = mid + 1;
        else {
            *sm = ia2_smr[mid].sm;
            *aw = 0;
            *has_aw = 0;
            return 1;
        }
    }
    return 0;
}

static int fle_ia2_cmp_weight(uint16_t c, unsigned *sm, unsigned *aw)
{
    int has;
    unsigned s, a;

    if (!ia2_unit_weight(c, &s, &a, &has)) return 0;
    if (sm) *sm = s;
    if (aw) *aw = a;
    return 1;
}

static int fle_ia2_cmp_primary(uint16_t a, uint16_t b, uint32_t *r)
{
    unsigned sa, wa, sb, wb;
    int ha, hb;

    if (a == '-' || a == '\'' || b == '-' || b == '\'') return 0;
    if (!ia2_unit_weight(a, &sa, &wa, &ha) || !ia2_unit_weight(b, &sb, &wb, &hb)) return 0;
    if (sa != sb) { *r = sa < sb ? 1u : 3u; return 1; }
    if (!ha || !hb || wa == wb) return 0;
    *r = wa < wb ? 1u : 3u;
    return 1;
}

/* Decision 0191 (PROVISIONAL; wall ia2-W25): the Private Use Area U+E000..U+F8FF. Microsoft's "Windows 8 Upper Case Mapping Table.txt" (the table of decision 0087, SHA-256 76a66e29...b6ae335, 973 rows) lists no
 * unit of it, neither as a lower case nor as an upper case: such a unit is mapped to itself by LCMAP_LOWERCASE and LCMAP_UPPERCASE, with or without LCMAP_LINGUISTIC_CASING (the FL Studio engine puts icon glyphs of its own font in
 * the names it lower-cases: "808 Kick\uE5A0 (Insert 1)\uE5FF", flags 0x01000100). */
static int fle_pua(uint32_t c) { return c >= 0xE000u && c <= 0xF8FFu; }
/* Decision 0209 (crash1): the case table of the Unicode Character Database for LCMapStringW (fle_crash1_lcmap.c). Weak: a link without that file behaves as before (a unit of 0x80 or above outside D stops by name). */
extern uint32_t fle_crash1_ucd_case(uint32_t cp, int upper) __attribute__((weak));
extern uint32_t fle_crash1_case(uint32_t cp, int upper, int linguistic) __attribute__((weak));

/* kernel32!LCMapStringW: int f(LCID, DWORD dwMapFlags, LPCWSTR lpSrcStr, int cchSrc, LPWSTR lpDestStr,
 * int cchDest). Implemented: dwMapFlags exactly LCMAP_LOWERCASE or LCMAP_UPPERCASE (decision 0087) on a string of the range D, with the
 * upper case from the Windows 8 Upper Case Mapping Table (fle_ms8_d) and the lower case from its inverse; Locale 0 is accepted for these two
 * flags only (a provisional choice; the Learn page does not list it). A unit outside D stops by name before anything is written.
 * LCMAP_LOWERCASE or LCMAP_UPPERCASE with LCMAP_LINGUISTIC_CASING (0x01000100, 0x01000200): the older behaviour below, ASCII letters
 * only and Locale 0 stops (decision 0087 leaves it unchanged). MSDN rules used: the source may not be NULL or of size 0 (87); a negative cchSrc
 * means NUL-terminated and the NUL is mapped and counted (reading of "null-terminates the mapped string");
 * cchDest 0 returns the required size; a small buffer fails with ERROR_INSUFFICIENT_BUFFER; a flag bit
 * the pages do not define, or LCMAP_LINGUISTIC_CASING without a case flag, is ERROR_INVALID_FLAGS; the
 * lowercase "i" maps to "I" in every locale unless LCMAP_LINGUISTIC_CASING (page), so for ASCII the
 * result does not depend on the locale (en-US, the defaults and the invariant locale are accepted).
 * Stops: any other flag (sort keys, width, kana, Chinese, title case, hash, byte reversal, NORM_*),
 * both case flags together (that is LCMAP_TITLECASE 0x300), any character of 0x80 or above (the Unicode
 * case table is not on the pages), and any other locale. */
void fle_imp_LCMapStringW(void)
{
    static const char api[] = "LCMapStringW";
    uint32_t lcid = (uint32_t)R[1], flags = (uint32_t)R[2], r, up, lo;
    const uint16_t *src = (const uint16_t *)fle_addr_opt(R[8], 2, 0);
    int32_t cs = (int32_t)R[9];
    uint16_t *dst = (uint16_t *)fle_addr_opt(fle_stack_arg(5), 2, 1);
    int32_t cd = (int32_t)fle_stack_arg(6);
    size_t n, i;
    int have_ucd = fle_crash1_ucd_case != NULL;   /* decision 0209: every unit is mapped, none stops */
    /* decision 0087: dwMapFlags exactly LCMAP_LOWERCASE or LCMAP_UPPERCASE: the range D, and Locale 0 accepted as the user default */
    int in_d = flags == FLE_LCMAP_LOWERCASE || flags == FLE_LCMAP_UPPERCASE;

    if (in_d && lcid == 0) {
        /* Locale 0 is not on the Learn page (nor in MS-LCID): a provisional choice of decision 0087 for these two flags only; it is
         * treated as LOCALE_USER_DEFAULT. fle_locale_resolve (used by the other functions) is not changed. */
        r = fle_emulated_locale.lcid;
    } else if (!fle_locale_resolve(lcid, &r)) {
        fle_locale_stop(api, "locale other than the emulated one");   /* decision 0209: soft: 0 and ERROR_INVALID_PARAMETER (the page) */
    }
    if (flags & ~FLE_ALL_MAP_FLAGS) {
        fle_fail(FLE_ERR_INVALID_FLAGS);
        return;
    }
    up = (flags & FLE_LCMAP_UPPERCASE) != 0;
    lo = (flags & FLE_LCMAP_LOWERCASE) != 0;
    if ((flags & FLE_LCMAP_LINGUISTIC_CASING) && !up && !lo) {
        fle_fail(FLE_ERR_INVALID_FLAGS);
        return;
    }
    if ((flags & ~(FLE_LCMAP_UPPERCASE | FLE_LCMAP_LOWERCASE | FLE_LCMAP_LINGUISTIC_CASING | (have_ucd ? FLE_LCMAP_BYTEREV : 0u))) || (up && lo) ||
        (!up && !lo && !((flags & FLE_LCMAP_BYTEREV) && have_ucd)))
        fle_locale_stop(api, "map flags other than a single LCMAP_LOWERCASE or LCMAP_UPPERCASE (with LCMAP_LINGUISTIC_CASING and LCMAP_BYTEREV), or LCMAP_BYTEREV alone");   /* decision 0209: soft: 0 and ERROR_INVALID_FLAGS (the page; the flag is a valid one that this runtime cannot do) */
    if (src == NULL || cs == 0 || cd < 0) {
        fle_fail(FLE_ERR_INVALID_PARAMETER);
        return;
    }
    n = cs < 0 ? fle_wlen(src) + 1 : (size_t)cs;
    for (i = 0; i < n; i++) {
        if (have_ucd) break;
        if (in_d) {
            if (!fle_in_d(src[i]) && !fle_pua(src[i])) fle_stop_outside_d(api, src[i]);
        } else if (src[i] >= 0x80 && !fle_pua(src[i])) {
            fle_locale_stop(api, "character of 0x80 or above: Unicode case table not on the MSDN pages");
        }
    }
    if (cd == 0) {
        R[0] = n;
        return;
    }
    if (dst == NULL || (size_t)cd < n) {
        fle_fail(FLE_ERR_INSUFFICIENT_BUFFER);
        return;
    }
    if (have_ucd) {
        /* decision 0209: every unit is mapped, none stops. A pair of surrogates is one code point; an unpaired surrogate and a unit without a mapping are copied. The range D answers from the rows of the Windows 8 table
         * (as before); everything else from the UCD 16.0.0 simple case mapping (fle_crash1_lcmap.c; a choice, see decision 0209). The string is read to cchSrc whatever it holds (the NUL "is considered to be unsortable
         * and always maps to itself"), so what follows a NUL inside cchSrc is mapped too. LCMAP_BYTEREV (the page: 0x3450 0x4822 becomes 0x5034 0x2248) swaps the two bytes of every unit of the result. */
        int byterev = (flags & FLE_LCMAP_BYTEREV) != 0;
        int ling = (flags & FLE_LCMAP_LINGUISTIC_CASING) != 0;

        for (i = 0; i < n; i++) {
            uint32_t c = src[i], m;
            int pair = 0;

            if (c >= 0xD800u && c <= 0xDBFFu && i + 1 < n && src[i + 1] >= 0xDC00u && src[i + 1] <= 0xDFFFu) {
                c = 0x10000u + ((c - 0xD800u) << 10) + ((uint32_t)src[i + 1] - 0xDC00u);
                pair = 1;
            }
            if (!up && !lo) m = c;
            else if (fle_pua(c) || (c >= 0xD800u && c <= 0xDFFFu)) m = c;
            else if (!ling && !pair && fle_in_d(c)) m = fle_case_d((uint16_t)c, up);   /* file system rules (the page's default): D from the rows of the Windows 8 table */
            else m = fle_crash1_case(c, (int)up, ling);   /* file system rules map one letter to another only where the second maps back (decision 0087's reading; Wine 10 agrees on all 0x110000 code points, wine/ of decision 0209): U+0130, U+0131, U+017F, U+03C2, the Kelvin sign and the like are left alone; LCMAP_LINGUISTIC_CASING maps them all */
            if (pair) {
                uint16_t hi, lw;

                m -= 0x10000u;
                hi = (uint16_t)(0xD800u + (m >> 10));
                lw = (uint16_t)(0xDC00u + (m & 0x3FFu));
                dst[i] = byterev ? (uint16_t)((hi << 8) | (hi >> 8)) : hi;
                dst[i + 1] = byterev ? (uint16_t)((lw << 8) | (lw >> 8)) : lw;
                i++;
            } else {
                dst[i] = (uint16_t)(byterev ? ((m << 8) | ((m >> 8) & 0xFFu)) : m);
            }
        }
        R[0] = n;
        return;
    }
    for (i = 0; i < n; i++) {
        uint16_t c = src[i];
        if (in_d) {
            if (!fle_pua(c)) c = fle_case_d(c, up);
        } else {
            if (up && c >= 'a' && c <= 'z') c = (uint16_t)(c - 'a' + 'A');
            if (lo && c >= 'A' && c <= 'Z') c = (uint16_t)(c - 'A' + 'a');
        }
        dst[i] = c;
    }
    R[0] = n;
}

static uint16_t fle_fold(uint16_t c, int ignore_case)
{
    return (ignore_case && c >= 'A' && c <= 'Z') ? (uint16_t)(c - 'A' + 'a') : c;
}

static int fle_is_alpha(uint16_t c) { return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z'); }
static int fle_is_ctl(uint16_t c) { return c < 0x20 || c == 0x7f; }   /* decision 0284: the ASCII control units */
static int fle_is_digit(uint16_t c) { return c >= '0' && c <= '9'; }
static int fle_is_sort_symbol(uint16_t c)
{
    return c >= 0x21 && c <= 0x7e && !fle_is_alpha(c) && !fle_is_digit(c) && c != '-' && c != '\'';
}

/* kernel32!CompareStringW: int f(LCID, DWORD dwCmpFlags, PCNZWCH lpString1, int cchCount1, PCNZWCH lpString2,
 * int cchCount2). Returns CSTR_LESS_THAN 1, CSTR_EQUAL 2 or CSTR_GREATER_THAN 3 (CompareStringEx page).
 * A negative length means NUL-terminated (page). A flag bit the page does not define is ERROR_INVALID_FLAGS;
 * a NULL string is ERROR_INVALID_PARAMETER.
 *
 * The en-US sort table is not on the MSDN pages, so only this subset is decided, only for dwCmpFlags 0 or
 * NORM_IGNORECASE and only for the emulated locale:
 *   - the two strings are equal as written (same length, same units), or, with NORM_IGNORECASE, equal after
 *     folding ASCII letters (the flag's own definition): CSTR_EQUAL;
 *   - otherwise, every unit before the first position where the (case-folded) units differ is an ASCII letter or
 *     digit (the units are equal on both sides there), and at that position both units are letters or both are
 *     digits: the order of the two units decides (alphabetical, numeric; the "billet, bills" rows of the
 *     "Handling Sorting" page show letters in alphabetical order; a primary difference outranks case, a
 *     reading). The units after that position, in either string, are not looked at (decision 0107: "Usually the
 *     sorting functions evaluate strings character by character", so the first differing character decides;
 *     before decision 0107 every unit of both strings had to be an ASCII letter or digit, which stopped
 *     CompareText("Default", "Segoe UI") of unit 283 at the space although it differs at position 0).
 *   - the first units (position 0) differ, one is a printable ASCII symbol other than the hyphen and the
 *     apostrophe and the other is an ASCII letter or digit: the symbol is the smaller (word sort, "Handling
 *     Sorting in Your Applications": all punctuation marks and other nonalphanumeric characters except the
 *     hyphen and the apostrophe come before any alphanumeric character; limited to position 0 because the page
 *     does not say the rule applies in the middle of a string, docs/decisions/0072).
 * Everything else stops: a prefix (shorter or longer), a case-only difference without NORM_IGNORECASE,
 * a letter against a digit, a symbol after position 0, a symbol against a symbol, the hyphen and the
 * apostrophe, a unit that is not an ASCII letter or digit at or before the first difference (a space
 * before it included), any other defined flag (SORT_STRINGSORT included), and any other locale
 * (LOCALE_INVARIANT included: the page says only that it is "similar to English (United States)"). */
static void fle_cmp_body(void)
{
    static const char api[] = "CompareStringW";
    uint32_t lcid = (uint32_t)R[1], flags = (uint32_t)R[2];
    const uint16_t *s1 = (const uint16_t *)fle_addr_opt(R[8], 2, 0);
    int32_t c1 = (int32_t)R[9];
    const uint16_t *s2 = (const uint16_t *)fle_addr_opt(fle_stack_arg(5), 2, 0);
    int32_t c2 = (int32_t)fle_stack_arg(6);
    int ic = (flags & 1u) != 0;
    size_t n1, n2, i, m;

    fle_require_emulated(api, lcid);
    if (flags & ~FLE_ALL_CMP_FLAGS) {
        fle_fail(FLE_ERR_INVALID_FLAGS);
        return;
    }
    if (s1 == NULL || s2 == NULL) {
        if (flags & ~1u) fle_locale_stop(api, "dwCmpFlags other than 0 and NORM_IGNORECASE");
        fle_fail(FLE_ERR_INVALID_PARAMETER);
        return;
    }
    n1 = c1 < 0 ? fle_wlen(s1) : (size_t)c1;
    n2 = c2 < 0 ? fle_wlen(s2) : (size_t)c2;
    /* Decision 0129 (PROVISIONAL), walls 5 and 6 (S-cmp-digits-flag): SORT_DIGITSASNUMBERS (0x8; Learn CompareStringEx: "Windows 7: Treat digits as numbers during sorting, for example, sort "2" before "10""; the value
     * 0x8 is winnls.h = UNOFFICIAL, FLE_ALL_CMP_FLAGS already holds it) changes the order only where a run of digits is compared with a run of digits, and the page gives that one rule (numeric order). The model: at the first
     * position where the (case-folded) units of the two strings differ, when BOTH are digits (0x30 to 0x39), the whole digit runs that contain them (back to the start of the run, forward to its end; the start is the same
     * in both strings because the units before it are equal) are compared as numbers (leading zeros do not count: fewer significant digits is the smaller, the same count compares digit by digit) and that decides; two runs
     * of the same value with different text ("01" and "1") stop by name (the page does not say which is first). When the first difference is not digit against digit the flag has no effect on the result (the primary difference
     * is decided as without the flag; a digit against a letter still stops as before) and a string without a digit is compared as without the flag. The guest compares the Browser's names "FL Studio homepage" with "FL
     * Studio tutorials" and "Layout FPC (C4)" with another "Layout ..." name (f_662950). Every other flag other than NORM_IGNORECASE still stops. Withdrawn when the digit order of the sort is modelled in full. */
    if (flags & 8u) {
        size_t f, mm = n1 < n2 ? n1 : n2;

        flags &= ~8u;
        for (f = 0; f < mm && fle_fold(s1[f], ic) == fle_fold(s2[f], ic); f++) {}
        if (f < mm && s1[f] >= '0' && s1[f] <= '9' && s2[f] >= '0' && s2[f] <= '9') {
            size_t st = f, e1, e2, z1, z2, l1, l2;

            while (st > 0 && s1[st - 1] >= '0' && s1[st - 1] <= '9') st--;
            for (e1 = f; e1 < n1 && s1[e1] >= '0' && s1[e1] <= '9'; e1++) {}
            for (e2 = f; e2 < n2 && s2[e2] >= '0' && s2[e2] <= '9'; e2++) {}
            for (z1 = st; z1 < e1 && s1[z1] == '0'; z1++) {}
            for (z2 = st; z2 < e2 && s2[z2] == '0'; z2++) {}
            l1 = e1 - z1;
            l2 = e2 - z2;
            if (l1 != l2) {
                R[0] = l1 < l2 ? 1 : 3;
                return;
            }
            for (f = 0; f < l1; f++) {
                if (s1[z1 + f] != s2[z2 + f]) {
                    R[0] = s1[z1 + f] < s2[z2 + f] ? 1 : 3;
                    return;
                }
            }
            fle_locale_stop(api, "SORT_DIGITSASNUMBERS: two digit runs of the same value with different text: the order is not on the MSDN pages");
        }
    }
    if (flags & ~1u) fle_locale_stop(api, "dwCmpFlags other than 0, NORM_IGNORECASE and SORT_DIGITSASNUMBERS");   /* decision 0209: soft: 0 and ERROR_INVALID_FLAGS (the page) */
    if (n1 == n2) {
        for (i = 0; i < n1 && fle_fold(s1[i], ic) == fle_fold(s2[i], ic); i++) {}
        if (i == n1) {
            R[0] = 2;
            return;
        }
    }
    if (n1 > 0 && n2 > 0) {
        uint16_t a = s1[0], b = s2[0];
        if (fle_is_sort_symbol(a) && (fle_is_alpha(b) || fle_is_digit(b))) {
            R[0] = 1;
            return;
        }
        if (fle_is_sort_symbol(b) && (fle_is_alpha(a) || fle_is_digit(a))) {
            R[0] = 3;
            return;
        }
    }
    m = n1 < n2 ? n1 : n2;
    for (i = 0; i < m; i++) {
        uint16_t a = fle_fold(s1[i], 1), b = fle_fold(s2[i], 1);
        if (!ic) {
            /* Without NORM_IGNORECASE a case-only difference is a tertiary difference: only a difference of
             * the folded units is a primary one. */
        }
        /* Decision 0107: only the units up to and including the first difference are looked at; what follows it, in either string, is not. */
        if (a == b) {
            /* Decision 0126 (PROVISIONAL), wall 62 (S-cmp-equal-prefix; the guest compares "Test message processing" with "Test program expiration" and the common prefix holds a space): a unit that is equal on
             * both sides at the same position is a common prefix, and a common prefix does not decide the order whichever weight the unit has: the page "Handling Sorting in Your Applications" says the sorting
             * functions "evaluate strings character by character" and that the word sort puts punctuation and other non-alphanumerics (the space included) before alphanumerics, which orders a symbol AGAINST another unit and
             * says nothing of a symbol that both strings have. The rule is limited to the ASCII printable symbols the word sort treats as "other nonalphanumeric characters" (0x20 to 0x7E except the hyphen and the apostrophe,
             * which the page treats differently) and to the case where the first difference that follows is a letter against a letter or a digit against a digit; a control character, 0x7F, a non-ASCII unit, the
             * hyphen and the apostrophe before the first difference still stop. A word sort that ignores symbols (NORM_IGNORESYMBOLS) is not requested: only flags 0 and NORM_IGNORECASE reach this code. */
            /* Wall 68 (S-cmp-equal-hyphen): the guest compares two paths ("c:\\users\\user\\documents\\image-line\\dow..." and "...\\fl s...") that share a hyphen: the hyphen and the apostrophe are the units the page says
             * "are treated differently from the other nonalphanumeric characters to ensure that words such as "coop" and "co-op" stay together in a sorted list" (their place between a letter and a digit is on the page's
             * table of examples only: "co-op" sorts between "con" and "coop"), but a unit that BOTH strings have at the same position is not compared against another unit, so it does not decide the order here either: a
             * hyphen (or apostrophe) in the common prefix is skipped like any other equal unit, and only the first different pair decides. That "co-op" and "coop" stay together is not used (the two strings here have the
             * hyphen at the same place); a pair of strings in which one has the hyphen and the other has a letter at that place still stops. */
            if (!fle_is_alpha(a) && !fle_is_digit(a) && !(a >= 0x20 && a <= 0x7e) && !((a >= 0x80 || fle_is_ctl(a)) && fle_ia2_cmp_weight(a, NULL, NULL)))
                fle_locale_stop(api, "character other than an ASCII letter, digit or printable symbol before the first difference: order not on the MSDN pages");
            continue;
        }
        if ((fle_is_alpha(a) && fle_is_alpha(b)) || (fle_is_digit(a) && fle_is_digit(b))) {
            R[0] = a < b ? 1 : 3;
            return;
        }
        /* Decision 0129 (PROVISIONAL), wall 7 (S-cmp-symbol-any-position; the guest sorts the Browser's names: "Minor Natural (Aeolian)" against "Minor Natural Chordmap", first difference '(' against 'C' at position 14): the
         * rule of "Handling Sorting in Your Applications" that wall 61 of decision 0126 limited to position 0 ("all punctuation marks and other nonalphanumeric characters except the hyphen and the apostrophe come before
         * any alphanumeric character") is applied at the first difference wherever it is: a printable ASCII symbol other than the hyphen and the apostrophe against a letter or a digit is the smaller. The page states the
         * order of the characters, not a position, so this is a reading of its sentence; the rule for the middle of a string is not on the page (S). A symbol against a symbol, the hyphen and the apostrophe, and a
         * unit that is not printable ASCII still stop. Withdrawn when the sort table is modelled. */
        if (fle_is_sort_symbol(a) && (fle_is_alpha(b) || fle_is_digit(b))) {
            R[0] = 1;
            return;
        }
        if (fle_is_sort_symbol(b) && (fle_is_alpha(a) || fle_is_digit(a))) {
            R[0] = 3;
            return;
        }
        /* Decision 0284 (A, PROVISIONAL): the rules below that order two units of the ASCII symbol classes by their CODE POINTS (S-cmp-symbol-vs-symbol, S-cmp-space-as-symbol, S-cmp-control: each of them
         * is a CHOICE WITHOUT A SOURCE and says "withdrawn when the sort table is modelled") give way to the primary weights (script member, then alphabetic weight) of Microsoft's published Sorting Weight Table (Windows10-SortingWeightTable.txt,
         * SHA-256 81fcfa1e...; fle_wall_ia2.c) when the table decides the pair (decision 0284c: the table is always on). The table decides every pair these rules can be asked about except two units of equal primary weight and U+0000
         * (no weight), which the rules below still answer as before. The code-point order is not the table's order for 116 ordered pairs of printable ASCII and white space (docs 0284b: 106 ordered pairs of two symbols, 10 of
         * TAB..CR against the space, both orders). S-cmp-general-punctuation (U+2000..U+206F against an ASCII symbol or the space) is replaced too (0284b's extension: 156 more ordered pairs). */
        if (((fle_is_sort_symbol(a) || a == 0x20) && (fle_is_sort_symbol(b) || b == 0x20)) ||
            ((a >= 0x2000 && a <= 0x206f) && (b >= 0x20 && b <= 0x7e && !fle_is_alpha(b) && !fle_is_digit(b))) ||
            ((b >= 0x2000 && b <= 0x206f) && (a >= 0x20 && a <= 0x7e && !fle_is_alpha(a) && !fle_is_digit(a))) ||
            ((a >= 0x09 && a <= 0x0d) && (b >= 0x20 && b <= 0x7e)) || ((b >= 0x09 && b <= 0x0d) && (a >= 0x20 && a <= 0x7e))) {
            uint32_t pr;
            if (fle_ia2_cmp_primary(a, b, &pr)) {
                R[0] = pr;
                return;
            }
        }
        /* Decision 0129 (PROVISIONAL), wall 8 (S-cmp-symbol-vs-symbol; "Minor Natural + Chords" against "Minor Natural (Aeolian)": '+' against '('): the order of two DIFFERENT symbols among themselves is in no Learn page read
         * (the sort table of the Windows locales is not published there; "Handling Sorting in Your Applications" gives only the classes: punctuation before alphanumerics, the hyphen and the apostrophe apart). CHOICE WITHOUT A
         * SOURCE: two different printable ASCII symbols (not the hyphen and not the apostrophe) are ordered by their code points. It is only used where a symbol meets a symbol at the first difference. Withdrawn
         * when the sort table is modelled (then the order of the Browser's lists may change; nothing else depends on it). */
        if (fle_is_sort_symbol(a) && fle_is_sort_symbol(b)) {
            R[0] = a < b ? 1 : 3;
            return;
        }
        /* Decision 0129 (PROVISIONAL), wall 13 (S-cmp-space-as-symbol; "Default.tpr" against "Default (macOS).tpr": '.' against ' ' at position 7): the space (0x20) is a symbol for the word sort like the other
         * nonalphanumeric characters ("Handling Sorting in Your Applications": punctuation and other nonalphanumeric characters come before alphanumerics): the first difference where one unit is the space and the other
         * is a symbol is ordered by the code points (the space, 0x20, is the smaller), the same CHOICE WITHOUT A SOURCE as for two different symbols (S-cmp-symbol-vs-symbol); the space against a letter or a digit is
         * the smaller (the page's rule). Withdrawn when the sort table is modelled. */
        if ((a == 0x20 && (fle_is_sort_symbol(b) || fle_is_alpha(b) || fle_is_digit(b))) || (b == 0x20 && (fle_is_sort_symbol(a) || fle_is_alpha(a) || fle_is_digit(a)))) {
            R[0] = a < b ? 1 : 3;
            return;
        }
        /* Decision 0132 (PROVISIONAL), wall 25 (S-cmp-general-punctuation; the guest compares "News - FL Studio" with "News \u2013 FL Studio": the hyphen against U+2013 EN DASH at position 5): a unit of the Unicode General
         * Punctuation block (U+2000 to U+206F: the dashes, the quotation marks, the ellipsis ...) against a printable ASCII unit that is not a letter or a digit (the space, the hyphen and the apostrophe included) is a symbol
         * against a symbol, and the order of two different symbols is the code point (the same CHOICE WITHOUT A SOURCE as S-cmp-symbol-vs-symbol and S-cmp-space-as-symbol: no Learn page reads the sort table; the page
         * "Handling Sorting in Your Applications" gives the classes only). The hyphen is not set aside here as in S-cmp-equal-hyphen, which concerns a hyphen that BOTH strings have; the page's "co-op" example is about a hyphen
         * against letters, not used. A general punctuation unit against a letter or a digit, a hyphen against a printable ASCII symbol, and every other non-ASCII unit still stop. Withdrawn when the sort table is modelled. */
        if (((a >= 0x2000 && a <= 0x206f) && (b >= 0x20 && b <= 0x7e && !fle_is_alpha(b) && !fle_is_digit(b))) ||
            ((b >= 0x2000 && b <= 0x206f) && (a >= 0x20 && a <= 0x7e && !fle_is_alpha(a) && !fle_is_digit(a)))) {
            R[0] = a < b ? 1 : 3;
            return;
        }
        /* Decision 0136 (PROVISIONAL), wall of the Enter key at the error dialog (S-cmp-control, a CHOICE WITHOUT A SOURCE; the guest compares the key character "\r" with "O", the accelerator letter of the OK button, with
         * NORM_IGNORECASE): a white-space control unit (TAB, LF, VT, FF, CR: 0x09 to 0x0d) against a printable ASCII unit that differs is ordered by the code points, so the control unit is the smaller (the same reading as
         * S-cmp-symbol-vs-symbol and S-cmp-general-punctuation: the page "Handling Sorting in Your Applications" gives the classes, punctuation and other non-alphanumerics before alphanumerics, and no weight of a control
         * character). The other control units (0x00 to 0x08, 0x0e to 0x1f, 0x7f) still stop. Withdrawn when the sort table is modelled. */
        if (((a >= 0x09 && a <= 0x0d) && (b >= 0x20 && b <= 0x7e)) || ((b >= 0x09 && b <= 0x0d) && (a >= 0x20 && a <= 0x7e))) {
            R[0] = a < b ? 1 : 3;
            return;
        }
        /* Decision 0178 (PROVISIONAL, OPT-IN: the environment variable FLE_CMP_EXTEND is set; without it the stop below is as before and the 16 snapshots and every earlier run do not reach this code). Two rules for the
         * first difference when it is not letter/letter, digit/digit, symbol/alphanumeric or symbol/symbol (the guest compares "\\Backup\\" with "-serum2\\" and "<katakana>" with "UNSORTED" while it opens the reference
         * project; see docs/0178):
         * (1) S-dlg1-cmp-hyphen-wordsort: the hyphen and the apostrophe. "Handling Sorting in Your Applications": the word sort puts punctuation and other non-alphanumerics before any alphanumeric character EXCEPT the
         * hyphen and the apostrophe, which "are treated differently ... to ensure that words such as "coop" and "co-op" stay together in a sorted list", and its table of word-sort results lists billet, bills, bill's,
         * cannot, cant, can't, con, coop, co-op. Read as: in the word sort a hyphen or an apostrophe does not take part in the comparison of the other units, and when the strings are equal without them the one that has
         * more of them is the greater (bills < bill's, cant < can't, coop < co-op). The two strings are compared again from the first difference with those units removed (same flags, this function), and a tie is
         * decided by the number removed; a tie with the same number stops. SORT_STRINGSORT (which treats them as ordinary symbols) is not in the flags that reach this code.
         * (2) S-dlg1-cmp-nonascii: a unit above U+007F against an ASCII unit (CHOICE WITHOUT A SOURCE; no Learn page gives the script order of the sort table): the ASCII unit is the smaller (an ASCII symbol, digit or
         * letter sorts before a kana or a CJK letter in the sort table of Windows, a reading from the usual order of the scripts, not a quotation), two such units by code point. */
        /* Decision 0191 (PROVISIONAL; wall ia2-W19): a pair of which at least one unit is above U+007F, decided by the primary weights (script member, alphabetic weight) of Microsoft's
         * Sorting Weight Table when they differ and both units are in the table (fle_wall_ia2.c has the exact limits); otherwise the code below as before. */
        /* Decision 0284 (PROVISIONAL): the ASCII control units too. The default table of the same file gives U+0001..U+001F and U+007F a primary weight (SM 6 or 7, an AW each; U+0000 has none: "0 0 0 0 is
         * used as a flag for no weight"), so a pair of which one is a control unit is decided the same way when both units have a weight (the guest compares the Escape character U+001B with the accelerator letter of a
         * button of its error window, decision 0136 row 6: SM 6 against the letters' SM 14). Whitespace controls against printable ASCII stay with S-cmp-control above. */
        if ((a >= 0x80 || b >= 0x80 || fle_is_ctl(a) || fle_is_ctl(b))) {
            uint32_t pr;
            if (fle_ia2_cmp_primary(a, b, &pr)) {
                R[0] = pr;
                return;
            }
        }
        /* Decision 0229 (ref1, PROVISIONAL): rule (1) below, the word sort's treatment of the hyphen and the apostrophe, is no longer behind FLE_CMP_EXTEND: it follows a sentence of Learn's "Handling Sorting in Your
         * Applications" and only decides pairs that stopped before (the first difference is a hyphen or apostrophe against a printable ASCII unit that the rules above did not decide). Rule (2), the code-point order of a unit above
         * U+007F against ASCII, has no source and stays behind FLE_CMP_EXTEND. */
        /* Decision 0284: the sentence of "Handling Sorting in Your Applications" that this rule reads (the hyphen and the apostrophe are "treated differently ... to ensure that words such as "coop" and "co-op" stay together") does
         * not depend on the class of the OTHER unit, so the printable-ASCII limit that was here is gone: a hyphen or an apostrophe against a control unit or a unit above U+007F is removed the same way, and the tails are
         * compared again (the call below stops by name when the tails cannot be decided). */
        if (a == '-' || a == '\'' || b == '-' || b == '\'') {
            {
                uint16_t *t1 = malloc((n1 - i + 1) * sizeof *t1), *t2 = malloc((n2 - i + 1) * sizeof *t2);
                uint64_t saved[16], st[8] = { 0 };
                size_t k1 = 0, k2 = 0, j, h1 = 0, h2 = 0;
                uint64_t res;

                if (t1 == NULL || t2 == NULL) fle_locale_stop(api, "out of memory");
                for (j = i; j < n1; j++) { if (s1[j] == '-' || s1[j] == '\'') h1++; else t1[k1++] = s1[j]; }
                for (j = i; j < n2; j++) { if (s2[j] == '-' || s2[j] == '\'') h2++; else t2[k2++] = s2[j]; }
                memcpy(saved, R, sizeof saved);
                R[1] = lcid; R[2] = flags; R[8] = (uint64_t)(uintptr_t)t1; R[9] = (uint64_t)(int64_t)k1;
                st[5] = (uint64_t)(uintptr_t)t2; st[6] = (uint64_t)(int64_t)k2;
                R[4] = (uint64_t)(uintptr_t)st;
                fle_cmp_body();
                res = R[0];
                memcpy(R, saved, sizeof saved);
                free(t1);
                free(t2);
                if (res == 2) {
                    if (h1 == h2) fle_locale_stop(api, "strings equal without their hyphens and apostrophes and with as many of them: the order is not on the MSDN pages");
                    res = h1 > h2 ? 3 : 1;
                }
                R[0] = res;
                return;
            }
        }
        if ((!fle_is_alpha(a) && !fle_is_digit(a)) || (!fle_is_alpha(b) && !fle_is_digit(b)))
            fle_locale_stop(api, "character other than an ASCII letter or digit in unequal strings");
        /* Decision 0171 (PROVISIONAL), wall ia1-W11 (S-ia1-cmp-digit-letter; the mixer's names "Insert 1" ... and a right click on an insert compare a letter with a digit at the first difference): the ASCII digits sort BEFORE
         * the ASCII letters. The page "Handling Sorting in Your Applications" gives the order of the classes of the word sort as punctuation and other non-alphanumerics, then alphanumerics, and shows digit-before-letter in
         * its examples of a sorted list (a reading, not a quoted sentence: the table of the Windows locales is not on the page). CHOICE WITHOUT A SOURCE for the exact rule: a digit against a letter is ordered digit first,
         * with or without NORM_IGNORECASE. Withdrawn when the sort table is modelled. The check above guarantees that both units are ASCII letters or digits and that they differ in class. */
        R[0] = fle_is_digit(a) ? 1 : 3;
        return;
    }
    /* Decision 0126 (PROVISIONAL), wall 72 (S-cmp-prefix; the guest compares ".FLMS" with ".FLM": one is a prefix of the other, the units of the shorter one being equal to the same units of the longer one): the page
     * CompareStringEx says only that CSTR_LESS_THAN means "The string indicated by lpString1 is less in lexical value than the string indicated by lpString2" and gives no rule for two strings of which one is a prefix of the
     * other. A CHOICE WITHOUT A SOURCE: the shorter string is the smaller ("lexical value": a lexicographic order puts a prefix before the strings it begins), CSTR_LESS_THAN when string 1 is the shorter, CSTR_GREATER_THAN
     * when it is the longer. It applies only when the common part is made of the units the loop above accepts (a letter, a digit or a printable symbol) and the lengths differ. A pair that is equal after the fold of
     * NORM_IGNORECASE and of the same length returned CSTR_EQUAL before. A case-only difference of strings of the same length without NORM_IGNORECASE still stops. */
    if (n1 != n2) {
        R[0] = n1 < n2 ? 1u : 3u;
        return;
    }
    fle_locale_stop(api, "case-only difference: order not on the MSDN pages");
}

/* Decision 0284 (DIAGNOSTIC ONLY: no answer changes): when CompareStringW stops by name, the second line of the message holds what was compared, so that the next stop shows the strings. The context is the OUTERMOST
 * call (the hyphen rule below calls fle_cmp_body again on copies of the tails, and that must not replace the original strings). A unit that is not printable ASCII (and the quote) is written \uXXXX; the backslash is
 * written as it is. Up to 160 units of each string are shown. */
static struct {
    int depth;
    const uint16_t *s1, *s2;
    size_t n1, n2;
    uint32_t lcid, flags;
    uint64_t ra;
    int have_ra;
} fle_cmp_ctx;

static void fle_cmp_show(const uint16_t *s, size_t n)
{
    size_t i, shown = n < 160 ? n : 160;

    if (s == NULL) {
        fputs("(null)", stderr);
        return;
    }
    fputc('"', stderr);
    for (i = 0; i < shown; i++) {
        uint16_t c = s[i];

        if (c >= 0x20 && c <= 0x7e && c != '"') fputc((int)c, stderr);
        else fprintf(stderr, "\\u%04X", (unsigned)c);
    }
    fputc('"', stderr);
    if (shown < n) fprintf(stderr, "...(+%zu units)", n - shown);
}

static void fle_cmp_context(void)
{
    size_t i, m;

    if (fle_cmp_ctx.depth <= 0) return;
    fprintf(stderr, "fle: CompareStringW: lcid=%#x flags=%#x len1=%zu len2=%zu string1=", (unsigned)fle_cmp_ctx.lcid, (unsigned)fle_cmp_ctx.flags, fle_cmp_ctx.n1, fle_cmp_ctx.n2);
    fle_cmp_show(fle_cmp_ctx.s1, fle_cmp_ctx.n1);
    fputs(" string2=", stderr);
    fle_cmp_show(fle_cmp_ctx.s2, fle_cmp_ctx.n2);
    if (fle_cmp_ctx.s1 != NULL && fle_cmp_ctx.s2 != NULL) {
        m = fle_cmp_ctx.n1 < fle_cmp_ctx.n2 ? fle_cmp_ctx.n1 : fle_cmp_ctx.n2;
        for (i = 0; i < m && fle_fold(fle_cmp_ctx.s1[i], 1) == fle_fold(fle_cmp_ctx.s2[i], 1); i++) {}
        if (i < m) fprintf(stderr, " first-difference=%zu units=U+%04X/U+%04X", i, (unsigned)fle_cmp_ctx.s1[i], (unsigned)fle_cmp_ctx.s2[i]);
        else fputs(" first-difference=none (equal after folding the case, or one string is a prefix of the other)", stderr);
    }
    if (fle_cmp_ctx.have_ra) fprintf(stderr, " guest-return-address=0x%llx", (unsigned long long)fle_cmp_ctx.ra);
    fputc('\n', stderr);
}

void fle_imp_CompareStringW(void)
{
    if (fle_cmp_ctx.depth == 0) {
        const uint64_t *rp = (const uint64_t *)fle_addr_opt(R[4], 8, 0);
        int32_t c1 = (int32_t)R[9], c2 = (int32_t)fle_stack_arg(6);

        fle_cmp_ctx.lcid = (uint32_t)R[1];
        fle_cmp_ctx.flags = (uint32_t)R[2];
        fle_cmp_ctx.s1 = (const uint16_t *)fle_addr_opt(R[8], 2, 0);
        fle_cmp_ctx.s2 = (const uint16_t *)fle_addr_opt(fle_stack_arg(5), 2, 0);
        fle_cmp_ctx.n1 = fle_cmp_ctx.s1 == NULL ? 0 : (c1 < 0 ? fle_wlen(fle_cmp_ctx.s1) : (size_t)c1);
        fle_cmp_ctx.n2 = fle_cmp_ctx.s2 == NULL ? 0 : (c2 < 0 ? fle_wlen(fle_cmp_ctx.s2) : (size_t)c2);
        fle_cmp_ctx.have_ra = rp != NULL;
        fle_cmp_ctx.ra = rp != NULL ? *rp : 0;
    }
    fle_cmp_ctx.depth++;
    fle_cmp_body();
    fle_cmp_ctx.depth--;
}

/* kernel32!GetSystemDefaultLocaleName (decision 0099, provisional): int GetSystemDefaultLocaleName(LPWSTR lpLocaleName, int cchLocaleName).
 * lpLocaleName rcx, cchLocaleName edx. Learn (winnls.h, Windows Vista and later): success returns the length of the locale name including the
 * terminating NUL (> 0); failure returns 0 with ERROR_INSUFFICIENT_BUFFER ("a supplied buffer size was not large enough, or it was incorrectly
 * set to NULL"). The system default locale is the emulated locale (decision 0016: user default = system default = en-US), and its name is the
 * LOCALE_SNAME value of GetLocaleInfoW (one source), "en-US": 6 characters with the NUL. The page does not define a negative cchLocaleName, so it
 * stops; a buffer that is NULL or smaller than the name fails and is not touched. Another emulated locale stops (fle_locale_info_value has no name). */
void fle_imp_GetSystemDefaultLocaleName(void)
{
    static const char api[] = "GetSystemDefaultLocaleName";
    uint16_t *out = (uint16_t *)fle_addr_opt(R[1], 2, 1);
    int32_t cch = (int32_t)(uint32_t)R[2];
    char s[32];
    int is_num;
    uint32_t num;
    size_t n, i;

    if (!fle_locale_info_value(FLE_LC_SNAME, s, sizeof s, &is_num, &num))
        fle_locale_stop(api, "the emulated locale has no name value");
    if (cch < 0) fle_locale_stop(api, "a negative cchLocaleName is not defined by the page");
    n = strlen(s) + 1;
    if (out == NULL || (size_t)cch < n) {
        fle_fail(FLE_ERR_INSUFFICIENT_BUFFER);
        return;
    }
    for (i = 0; i < n; i++) out[i] = (uint8_t)s[i];
    R[0] = n;
}

/* kernel32!LocaleNameToLCID (decision 0112, PROVISIONAL): LCID LocaleNameToLCID(LPCWSTR lpName rcx, DWORD dwFlags edx). Learn (LocaleNameToLCID): lpName is "a
 * locale name, or one of the following predefined values": LOCALE_NAME_INVARIANT, LOCALE_NAME_SYSTEM_DEFAULT, LOCALE_NAME_USER_DEFAULT (winnls.h, not on that page:
 * L"", L"!x-sys-default-locale", NULL); dwFlags is "Reserved; should always be 0" before Windows 7 and, from Windows 7, "Can be set to LOCALE_ALLOW_NEUTRAL_NAMES"
 * (0x08000000); "If successful, returns the locale identifier corresponding to the locale name"; "The function returns 0 if it does not succeed" (with
 * ERROR_INVALID_PARAMETER "if any of the parameter values are invalid"). The page has no table of names, so only what this environment's locale decisions already
 * hold is answered: NULL and the system-default name give the emulated locale (decision 0016: user default = system default), the emulated locale's own name
 * (the LOCALE_SNAME value that GetSystemDefaultLocaleName returns, "en-US"; compared without ASCII case: inference, locale names are language tags) gives its LCID,
 * and the empty name gives LOCALE_INVARIANT. Any other name stops: answering 0 would call a real locale name invalid, and a value from another table would be
 * invented. A flag other than the two stops, and so does a name of 85 characters or more (LOCALE_NAME_MAX_LENGTH "includes space for a terminating null
 * character") or with a character above 0x7F. The last error is not touched on success. The one FLEngine call (delay-loaded; f_446ad0 at 0x446b24) passes flags 0. */
#define FLE_LOCALE_ALLOW_NEUTRAL_NAMES 0x08000000u
#define FLE_LOCALE_NAME_MAX_LENGTH     85u

static int fle_lnl_ieq(const char *a, const char *b)
{
    for (; *a && *b; a++, b++) {
        char x = (*a >= 'A' && *a <= 'Z') ? (char)(*a + 32) : *a;
        char y = (*b >= 'A' && *b <= 'Z') ? (char)(*b + 32) : *b;
        if (x != y) return 0;
    }
    return *a == *b;
}

void fle_imp_LocaleNameToLCID(void)
{
    static const char api[] = "LocaleNameToLCID";
    uint64_t p = R[1];
    uint32_t flags = (uint32_t)R[2];
    char got[FLE_LOCALE_NAME_MAX_LENGTH + 1], want[32];
    int is_num;
    uint32_t num;
    size_t n = 0;

    if (flags != 0u && flags != FLE_LOCALE_ALLOW_NEUTRAL_NAMES)
        fle_locale_stop(api, "dwFlags is neither 0 nor LOCALE_ALLOW_NEUTRAL_NAMES: the page names no other value");
    if (p == 0) {
        R[0] = fle_emulated_locale.lcid;
        return;
    }
    for (;; n++) {
        uint64_t c;
        if (n >= FLE_LOCALE_NAME_MAX_LENGTH) fle_locale_stop(api, "a name of LOCALE_NAME_MAX_LENGTH (85) characters or more: the page does not define it");
        c = fle_ld(p + 2u * n, 2);
        if (c == 0) break;
        if (c > 0x7Fu) fle_locale_stop(api, "a name with a character above 0x7F is not a locale name of this environment");
        got[n] = (char)c;
    }
    got[n] = 0;
    if (n == 0) {
        R[0] = FLE_LOCALE_INVARIANT;
        return;
    }
    if (fle_lnl_ieq(got, "!x-sys-default-locale")) {
        R[0] = fle_emulated_locale.lcid;
        return;
    }
    if (fle_locale_info_value(FLE_LC_SNAME, want, sizeof want, &is_num, &num) && fle_lnl_ieq(got, want)) {
        R[0] = fle_emulated_locale.lcid;
        return;
    }
    fle_locale_stop(api, "a locale name that this environment does not have (only the emulated locale's name, NULL, the system-default name and the empty name are answered)");
}

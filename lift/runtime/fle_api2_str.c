/* Decision 0210 (api2, PROVISIONAL, opt-in): kernel32 NLS A variants and TzSpecificLocalTimeToSystemTime (fle_api2.h).
 *
 * The A functions convert the narrow strings with the code page of the emulated locale (CP_ACP: Learn, CompareStringA / LCMapStringA / GetStringTypeA / GetLocaleInfoA: "the code page associated with
 * the locale"; every locale that fle_locale.c answers has the one emulated ANSI code page, fle_emulated_locale.acp), call the W function of fle_locale.c with the converted text and convert the result
 * back. They therefore answer exactly the locales, flags and types that the W function answers; what the W function stops on stops here (by the W function's name). */
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_locale.h"
#include "fle_time.h"
#include "fle_api2.h"

#define ERR_INVALID_PARAMETER 87u
#define ERR_INSUFFICIENT_BUFFER 122u
#define ERR_NOT_ENOUGH_MEMORY 8u

#define LOCALE_RETURN_NUMBER 0x20000000u
#define LCMAP_SORTKEY 0x00000400u

static void fail(uint32_t err)
{
    fle_set_last_error(err);
    R[0] = 0;
}

/* The narrow string s (c bytes, -1 = NUL-terminated) as a malloc'd UTF-16 buffer; *n = the number of UTF-16 units (including the NUL for c == -1). NULL: the conversion failed (the last error is
 * set by MultiByteToWideChar) or no memory. c == 0 is an empty string (CompareStringA: "cchCount1 ... the size ... in bytes", a zero length is an empty string). */
static uint16_t *a_to_w(uint64_t s, int32_t c, int32_t *n)
{
    uint32_t k;
    uint16_t *w;

    if (c == 0) {
        *n = 0;
        return (uint16_t *)calloc(1, 2);
    }
    k = (uint32_t)fle_api2_call(fle_imp_MultiByteToWideChar, 0, 0, s, (uint64_t)(int64_t)c, 0, 0, 0, 0);
    if (k == 0) return NULL;
    w = (uint16_t *)malloc(2u * k);
    if (!w) {
        fle_set_last_error(ERR_NOT_ENOUGH_MEMORY);
        return NULL;
    }
    k = (uint32_t)fle_api2_call(fle_imp_MultiByteToWideChar, 0, 0, s, (uint64_t)(int64_t)c, (uint64_t)(uintptr_t)w, k, 0, 0);
    if (k == 0) {
        free(w);
        return NULL;
    }
    *n = (int32_t)k;
    return w;
}

/* UTF-16 text (n units) as narrow bytes in CP_ACP: the number of bytes, 0 on failure. dst may be NULL with cb 0 (size query). */
static uint32_t w_to_a(const uint16_t *w, int32_t n, uint64_t dst, uint32_t cb)
{
    return (uint32_t)fle_api2_call(fle_imp_WideCharToMultiByte, 0, 0, (uint64_t)(uintptr_t)w, (uint64_t)(int64_t)n, dst, cb, 0, 0);
}

/* kernel32!CompareStringA: int f(LCID Locale rcx, DWORD dwCmpFlags rdx, PCNZCH lpString1 r8, int cchCount1 r9, PCNZCH lpString2 [5], int cchCount2 [6]).
 * Learn (CompareStringA / CompareStringEx): returns CSTR_LESS_THAN 1, CSTR_EQUAL 2, CSTR_GREATER_THAN 3, 0 on failure (ERROR_INVALID_PARAMETER for an invalid parameter, ERROR_INVALID_FLAGS for invalid flags). A NULL string pointer is an
 * invalid parameter (87; the same reading as CompareStringW in fle_locale.c). The comparison is that of CompareStringW on the converted strings. */
void fle_imp_CompareStringA(void)
{
    uint64_t loc = R[1], fl = R[2], s1 = R[8], s2 = fle_arg(5);
    int32_t c1 = (int32_t)R[9], c2 = (int32_t)fle_arg(6), n1 = 0, n2 = 0;
    uint16_t *w1, *w2;
    uint64_t r;

    if (s1 == 0 || s2 == 0) {
        fail(ERR_INVALID_PARAMETER);
        return;
    }
    w1 = a_to_w(s1, c1, &n1);
    if (!w1) {
        R[0] = 0;
        return;
    }
    w2 = a_to_w(s2, c2, &n2);
    if (!w2) {
        free(w1);
        R[0] = 0;
        return;
    }
    r = fle_api2_call(fle_imp_CompareStringW, loc, fl, (uint64_t)(uintptr_t)w1, (uint64_t)(int64_t)(c1 == -1 ? -1 : n1), (uint64_t)(uintptr_t)w2, (uint64_t)(int64_t)(c2 == -1 ? -1 : n2), 0, 0);
    free(w1);
    free(w2);
    R[0] = r;
}

/* kernel32!LCMapStringA: int f(LCID Locale rcx, DWORD dwMapFlags rdx, PCNZCH lpSrcStr r8, int cchSrc r9, PCHAR lpDestStr [5], int cchDest [6]).
 * Learn (LCMapStringEx): "the number of characters or bytes in the translated string or sort key ... If cchDest is 0, the required size of the buffer"; ERROR_INSUFFICIENT_BUFFER when the buffer is too small; with LCMAP_SORTKEY the destination holds a
 * sort key in bytes (not text): the key of LCMapStringW is passed through unconverted. For the text maps the size is that of the narrow result (the map is done on UTF-16 and converted back with CP_ACP). */
void fle_imp_LCMapStringA(void)
{
    uint64_t loc = R[1], fl = R[2], src = R[8], dst = fle_arg(5);
    int32_t csrc = (int32_t)R[9], cdst = (int32_t)fle_arg(6), n = 0, m;
    uint16_t *w, *wo;
    uint32_t b;

    if (src == 0) {
        fail(ERR_INVALID_PARAMETER);
        return;
    }
    if (cdst < 0) {
        fail(ERR_INVALID_PARAMETER);
        return;
    }
    w = a_to_w(src, csrc, &n);
    if (!w) {
        R[0] = 0;
        return;
    }
    if (fl & LCMAP_SORTKEY) {
        uint64_t r = fle_api2_call(fle_imp_LCMapStringW, loc, fl, (uint64_t)(uintptr_t)w, (uint64_t)(int64_t)n, dst, (uint64_t)(int64_t)cdst, 0, 0);

        free(w);
        R[0] = r;
        return;
    }
    m = (int32_t)fle_api2_call(fle_imp_LCMapStringW, loc, fl, (uint64_t)(uintptr_t)w, (uint64_t)(int64_t)n, 0, 0, 0, 0);
    if (m <= 0) {
        free(w);
        R[0] = 0;
        return;
    }
    wo = (uint16_t *)malloc(2u * (size_t)m);
    if (!wo) {
        free(w);
        fail(ERR_NOT_ENOUGH_MEMORY);
        return;
    }
    m = (int32_t)fle_api2_call(fle_imp_LCMapStringW, loc, fl, (uint64_t)(uintptr_t)w, (uint64_t)(int64_t)n, (uint64_t)(uintptr_t)wo, (uint64_t)(int64_t)m, 0, 0);
    free(w);
    if (m <= 0) {
        free(wo);
        R[0] = 0;
        return;
    }
    b = w_to_a(wo, m, 0, 0);
    if (b == 0) {
        free(wo);
        R[0] = 0;
        return;
    }
    if (cdst == 0) {
        free(wo);
        R[0] = b;
        return;
    }
    if ((uint32_t)cdst < b || dst == 0) {
        free(wo);
        fail(ERR_INSUFFICIENT_BUFFER);
        return;
    }
    b = w_to_a(wo, m, dst, (uint32_t)cdst);
    free(wo);
    R[0] = b;
}

/* kernel32!GetStringTypeA: BOOL f(LCID Locale rcx, DWORD dwInfoType rdx, LPCSTR lpSrcStr r8, int cchSrc r9, LPWORD lpCharType [5]).
 * Learn (GetStringTypeA): "Retrieves character type information for the characters in the specified source string. For each character in the string, the function sets one or more bits in the corresponding 16-bit element of the output array";
 * cchSrc -1 = NUL-terminated (then the NUL is part of the string and has an element). The type bits are those of GetStringTypeW for the same character (the narrow characters of the emulated single-byte code page map one to one). */
void fle_imp_GetStringTypeA(void)
{
    uint64_t loc = R[1], info = R[2], src = R[8], out = fle_arg(5);
    int32_t c = (int32_t)R[9], n = 0;
    uint16_t *w;
    uint64_t r;

    (void)loc;
    if (src == 0 || out == 0) {
        fail(ERR_INVALID_PARAMETER);
        return;
    }
    w = a_to_w(src, c, &n);
    if (!w) {
        R[0] = 0;
        return;
    }
    r = fle_api2_call(fle_imp_GetStringTypeW, info, (uint64_t)(uintptr_t)w, (uint64_t)(int64_t)n, out, 0, 0, 0, 0);
    free(w);
    R[0] = r;
}

/* kernel32!GetLocaleInfoA: int f(LCID Locale rcx, LCTYPE LCType rdx, LPSTR lpLCData r8, int cchData r9).
 * Learn (GetLocaleInfoW / GetLocaleInfoEx): the value is a string (or, with LOCALE_RETURN_NUMBER, a DWORD stored in the buffer); "the number of characters retrieved in the locale data buffer ... If cchData is 0, the required size of the buffer, in characters,
 * including a terminating null"; ERROR_INSUFFICIENT_BUFFER when the buffer is too small. The string is the W string converted with CP_ACP (a character the code page lacks becomes '?', Learn WideCharToMultiByte default character). */
void fle_imp_GetLocaleInfoA(void)
{
    uint64_t loc = R[1], type = R[2], buf = R[8];
    int32_t cch = (int32_t)R[9], nw;
    uint16_t *w;
    uint32_t b;

    if (type & LOCALE_RETURN_NUMBER) {
        uint64_t r = fle_api2_call(fle_imp_GetLocaleInfoW, loc, type, buf, (uint64_t)(int64_t)cch, 0, 0, 0, 0);

        /* S-api2-8 (UNOFFICIAL, Wine 10.0): the number is a DWORD in the buffer, and the A function counts bytes: 4 (the W function counts 2 UTF-16 units for the same DWORD) */
        R[0] = r * 2u;
        return;
    }
    if (cch < 0) {
        fail(ERR_INVALID_PARAMETER);
        return;
    }
    nw = (int32_t)fle_api2_call(fle_imp_GetLocaleInfoW, loc, type, 0, 0, 0, 0, 0, 0);
    if (nw <= 0) {
        R[0] = 0;
        return;
    }
    w = (uint16_t *)malloc(2u * (size_t)nw);
    if (!w) {
        fail(ERR_NOT_ENOUGH_MEMORY);
        return;
    }
    nw = (int32_t)fle_api2_call(fle_imp_GetLocaleInfoW, loc, type, (uint64_t)(uintptr_t)w, (uint64_t)(int64_t)nw, 0, 0, 0, 0);
    if (nw <= 0) {
        free(w);
        R[0] = 0;
        return;
    }
    b = w_to_a(w, nw, 0, 0);
    if (b == 0) {
        free(w);
        R[0] = 0;
        return;
    }
    if (cch == 0) {
        free(w);
        R[0] = b;
        return;
    }
    if ((uint32_t)cch < b || buf == 0) {
        free(w);
        fail(ERR_INSUFFICIENT_BUFFER);
        return;
    }
    b = w_to_a(w, nw, buf, (uint32_t)cch);
    free(w);
    R[0] = b;
}

/* ---- kernel32!TzSpecificLocalTimeToSystemTime ----
 * BOOL f(const TIME_ZONE_INFORMATION *lpTimeZoneInformation rcx, const SYSTEMTIME *lpLocalTime rdx, LPSYSTEMTIME lpUniversalTime r8).
 * Learn (TzSpecificLocalTimeToSystemTime): converts "the specified local time to the corresponding time in Coordinated Universal Time (UTC)"; "takes into account whether daylight saving time (DST) is in effect for the local time"; a local time that is
 * ambiguous (the repeated hour when DST ends) or invalid (the skipped hour when DST starts) "is treated as daylight saving time and the daylight saving time bias is applied"; lpTimeZoneInformation NULL = "the currently active time zone" (the emulated one,
 * GetTimeZoneInformation: bias FLE_TZ_BIAS_MINUTES, no DST). TIME_ZONE_INFORMATION (Learn): Bias (UTC = local + Bias, minutes), StandardDate / DaylightDate (SYSTEMTIME): "the date and local time when the transition from daylight saving time to standard time
 * (StandardDate) / from standard time to daylight saving time (DaylightDate) occurs"; wMonth 0 = "not supported / no transition date"; wYear 0 = the date is relative: wMonth, wDayOfWeek and wDay 1..5 (the n-th wDayOfWeek of the month, 5 = the last) with
 * wHour / wMinute the local time; a non-zero wYear is an absolute date. StandardBias / DaylightBias are added to Bias in the respective period and "ignored if a value for the StandardDate member is not supplied".
 * Hemispheres: the transition into DST (DaylightDate) is read in local standard time and the transition out (StandardDate) in local daylight time, so a local wall-clock value L is in DST when start <= L < end (start < end) or L >= start || L < end (start > end);
 * this gives the page's rule for the ambiguous and invalid hours (DST). An invalid SYSTEMTIME: ERROR_INVALID_PARAMETER (87; the page names no code: S-api2-1, the code of SystemTimeToFileTime in fle_time.c). NULL lpLocalTime or lpUniversalTime stops. */
#define TZI_BIAS 0u
#define TZI_STD_DATE 68u
#define TZI_STD_BIAS 84u
#define TZI_DLT_DATE 152u
#define TZI_DLT_BIAS 168u

static int64_t days_from_civil(int64_t y, unsigned m, unsigned d)
{
    int64_t era;
    unsigned yoe, doy, doe;

    y -= m <= 2;
    era = (y >= 0 ? y : y - 399) / 400;
    yoe = (unsigned)(y - era * 400);
    doy = (153u * (m + (m > 2 ? -3 : 9)) + 2u) / 5u + d - 1u;
    doe = yoe * 365u + yoe / 4u - yoe / 100u + doy;
    return era * 146097 + (int64_t)doe - 719468;
}

static unsigned days_in_month(int64_t y, unsigned m)
{
    static const unsigned dm[12] = { 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31 };

    if (m == 2 && ((y % 4 == 0 && y % 100 != 0) || y % 400 == 0)) return 29;
    return dm[m - 1];
}

/* seconds since 1970-01-01 of the wall-clock time of a transition SYSTEMTIME (words: year, month, day of week, day, hour, minute, second, ms) in year y */
static int64_t transition_secs(const uint16_t st[8], int64_t y)
{
    int64_t yy = st[0] ? st[0] : y, days;
    unsigned day;

    if (st[0]) {
        day = st[3];
    } else {
        unsigned wd1 = (unsigned)((days_from_civil(yy, st[1], 1) + 4) % 7 + 7) % 7;   /* weekday of the 1st, Sunday = 0 */
        unsigned first = 1u + ((unsigned)st[2] + 7u - wd1) % 7u;

        day = first + 7u * ((unsigned)st[3] - 1u);
        while (day > days_in_month(yy, st[1])) day -= 7u;
    }
    days = days_from_civil(yy, st[1], day);
    return days * 86400 + (int64_t)st[4] * 3600 + (int64_t)st[5] * 60 + st[6];
}

void fle_imp_TzSpecificLocalTimeToSystemTime(void)
{
    static const char api[] = "TzSpecificLocalTimeToSystemTime";
    uint64_t tz = R[1], pl = R[2], pu = R[8];
    uint16_t loc[8], sd[8], dd[8], out[8];
    uint64_t lft, uft;
    int32_t bias = FLE_TZ_BIAS_MINUTES, sbias = 0, dbias = 0;
    int dst = 0, k;

    if (pl == 0) fle_stop(api, "lpLocalTime is NULL: the page gives no behaviour");
    if (pu == 0) fle_stop(api, "lpUniversalTime is NULL: the page gives no behaviour");
    for (k = 0; k < 8; k++) loc[k] = (uint16_t)fle_ld(pl + 2u * (unsigned)k, 2);
    if (!fle_time_systemtime_to_filetime(loc, &lft)) {
        fail(ERR_INVALID_PARAMETER);
        return;
    }
    if (tz) {
        bias = (int32_t)fle_ld(tz + TZI_BIAS, 4);
        for (k = 0; k < 8; k++) {
            sd[k] = (uint16_t)fle_ld(tz + TZI_STD_DATE + 2u * (unsigned)k, 2);
            dd[k] = (uint16_t)fle_ld(tz + TZI_DLT_DATE + 2u * (unsigned)k, 2);
        }
        sbias = (int32_t)fle_ld(tz + TZI_STD_BIAS, 4);
        dbias = (int32_t)fle_ld(tz + TZI_DLT_BIAS, 4);
        if (sd[1] != 0 && dd[1] != 0) {
            int64_t y = loc[0], l = (int64_t)(lft / 10000000u) - 11644473600LL;
            int64_t start = transition_secs(dd, y), end = transition_secs(sd, y);

            dst = start < end ? (l >= start && l < end) : (l >= start || l < end);
            bias += dst ? dbias : sbias;
        }
    }
    (void)dst;
    uft = (uint64_t)((int64_t)lft + (int64_t)bias * 600000000LL);
    if (!fle_time_filetime_to_systemtime(uft, out)) {
        fail(ERR_INVALID_PARAMETER);
        return;
    }
    for (k = 0; k < 8; k++) fle_st(pu + 2u * (unsigned)k, 2, out[k]);
    R[0] = 1;
}

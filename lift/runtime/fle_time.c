/* Emulated wall-clock time, time zone and the kernel32 time/date answers (docs/decisions/0020-process-and-time.md;
 * the pages read are listed in docs/investigations/import-proctime-spec.md). The host clock is never read.
 * A case the pages leave open stops the run through fle_stop (fle_env.h). */
#include <stdint.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_sys.h"
#include "fle_time.h"

/* ---- Calendar (proleptic Gregorian, from 1601-01-01 which the FILETIME epoch names) ---- */
static int fle_leap(uint32_t y) { return (y % 4 == 0 && y % 100 != 0) || y % 400 == 0; }

static const uint8_t fle_mdays[12] = { 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31 };

static uint32_t fle_month_len(uint32_t y, uint32_t m) { return fle_mdays[m - 1] + (m == 2 && fle_leap(y) ? 1u : 0u); }

/* Leap years in 1..n. */
static uint64_t fle_leaps_upto(uint64_t n) { return n / 4 - n / 100 + n / 400; }

/* Days from 1601-01-01 to the given date (the date is valid). */
static uint64_t fle_days_since_1601(uint32_t y, uint32_t m, uint32_t d)
{
    uint64_t days = (uint64_t)(y - 1601) * 365 + (fle_leaps_upto(y - 1) - fle_leaps_upto(1600));
    uint32_t i;

    for (i = 1; i < m; i++) days += fle_month_len(y, i);
    return days + (d - 1);
}

/* SYSTEMTIME member limits (MSDN: SYSTEMTIME: year 1601 through 30827, and the ranges of the other members).
 * The page also says a SYSTEMTIME is not checked to be a real date; the real-date test is ours (leap years). */
#define FLE_ST_YEAR_MIN 1601u
#define FLE_ST_YEAR_MAX 30827u

static int fle_st_valid(const uint16_t st[8])
{
    uint32_t y = st[0], m = st[1], d = st[3];

    if (y < FLE_ST_YEAR_MIN || y > FLE_ST_YEAR_MAX) return 0;
    if (m < 1 || m > 12) return 0;
    if (d < 1 || d > fle_month_len(y, m)) return 0;
    return st[4] < 24 && st[5] < 60 && st[6] < 60 && st[7] < 1000;
}

int fle_time_systemtime_to_filetime(const uint16_t st[8], uint64_t *ft)
{
    uint64_t days, secs, t;

    if (!fle_st_valid(st)) return 0;
    days = fle_days_since_1601(st[0], st[1], st[3]);
    secs = days * 86400 + (uint64_t)st[4] * 3600 + (uint64_t)st[5] * 60 + st[6];
    if (secs > (UINT64_MAX / FLE_TICKS_PER_SECOND)) return 0;
    t = secs * FLE_TICKS_PER_SECOND + (uint64_t)st[7] * 10000;
    if (t >= FLE_TIME_FILETIME_LIMIT) return 0;
    *ft = t;
    return 1;
}

int fle_time_filetime_to_systemtime(uint64_t ft, uint16_t st[8])
{
    uint64_t secs, days, rem;
    uint32_t y, m, ms;

    if (ft >= FLE_TIME_FILETIME_LIMIT) return 0;   /* MSDN FileTimeToSystemTime: must be below 0x8000000000000000 */
    ms = (uint32_t)((ft % FLE_TICKS_PER_SECOND) / 10000);
    secs = ft / FLE_TICKS_PER_SECOND;
    days = secs / 86400;
    rem = secs % 86400;
    st[2] = (uint16_t)((days + 1) % 7);            /* 1601-01-01 was a Monday (1); Sunday is 0 */
    y = 1601 + (uint32_t)(days / 146097) * 400;     /* 146097 days in 400 Gregorian years */
    days %= 146097;
    for (;;) {
        uint32_t len = fle_leap(y) ? 366u : 365u;
        if (days < len) break;
        days -= len;
        y++;
    }
    for (m = 1; days >= fle_month_len(y, m); m++) days -= fle_month_len(y, m);
    if (y > FLE_ST_YEAR_MAX) return 0;             /* above the documented wYear range (see the spec note) */
    st[0] = (uint16_t)y;
    st[1] = (uint16_t)m;
    st[3] = (uint16_t)(days + 1);
    st[4] = (uint16_t)(rem / 3600);
    st[5] = (uint16_t)((rem % 3600) / 60);
    st[6] = (uint16_t)(rem % 60);
    st[7] = (uint16_t)ms;
    return 1;
}

/* ---- The one time source ----
 * fle_imp_QueryPerformanceCounter (fle_sys.c) returns the tick count of the virtual clock and then advances it
 * by FLE_CLOCK_READ_TICKS; calling it keeps one clock and one advance rule for every time API. It touches R[0]
 * and R[1], which are saved and put back. Tick 0 is FLE_TIME_BOOT_FILETIME. */
static uint64_t fle_clock_read_ticks(void)
{
    uint64_t save0 = R[0], save1 = R[1], t = 0;

    R[1] = (uint64_t)(uintptr_t)&t;
    fle_imp_QueryPerformanceCounter();
    R[0] = save0;
    R[1] = save1;
    return t;
}

uint64_t fle_time_now_filetime(void) { return FLE_TIME_BOOT_FILETIME + fle_clock_read_ticks(); }

static int64_t fle_bias_ticks(int32_t bias_minutes) { return (int64_t)bias_minutes * (int64_t)FLE_TICKS_PER_MINUTE; }

/* UTC = local time + bias, so local = UTC - bias. Fails (0) if the result is outside the FILETIME range. */
static int fle_utc_to_local(uint64_t utc, int32_t bias_minutes, uint64_t *local)
{
    int64_t v;

    if (utc >= FLE_TIME_FILETIME_LIMIT) return 0;
    v = (int64_t)utc - fle_bias_ticks(bias_minutes);
    if (v < 0) return 0;
    *local = (uint64_t)v;
    return 1;
}

static int fle_local_to_utc(uint64_t local, int32_t bias_minutes, uint64_t *utc)
{
    int64_t v;

    if (local >= FLE_TIME_FILETIME_LIMIT) return 0;
    v = (int64_t)local + fle_bias_ticks(bias_minutes);
    if (v < 0) return 0;
    *utc = (uint64_t)v;
    return 1;
}

static void fle_put_systemtime(uint64_t dst, const uint16_t st[8])
{
    unsigned i;
    for (i = 0; i < 8; i++) fle_st(dst + 2u * i, 2, st[i]);
}

static void fle_get_systemtime(uint64_t src, uint16_t st[8])
{
    unsigned i;
    for (i = 0; i < 8; i++) st[i] = (uint16_t)fle_ld(src + 2u * i, 2);
}

static uint64_t fle_need_ptr(const char *api, uint64_t p, const char *what)
{
    if (p == 0) fle_stop(api, what);
    return p;
}

/* A time that the converter cannot represent: the pages name no error code; ERROR_INVALID_PARAMETER is a
 * runtime choice ("assumption" in the spec note). */
static void fle_fail(uint32_t code)
{
    fle_set_last_error(code);
    R[0] = 0;
}

/* ---- Current time ---- */
/* MSDN GetSystemTime: VOID; lpSystemTime must not be NULL (a NULL "will result in an access violation"). */
void fle_imp_GetSystemTime(void)
{
    uint64_t out = fle_need_ptr("GetSystemTime", R[1], "lpSystemTime is NULL (the page: access violation)");
    uint16_t st[8];
    uint64_t ft = fle_time_now_filetime();

    if (!fle_time_filetime_to_systemtime(ft, st)) fle_stop("GetSystemTime", "emulated time is outside the SYSTEMTIME range");
    fle_put_systemtime(out, st);
}

/* MSDN GetLocalTime: VOID; the local time. The page does not say what NULL does; stop. */
void fle_imp_GetLocalTime(void)
{
    uint64_t out = fle_need_ptr("GetLocalTime", R[1], "lpSystemTime is NULL (not defined by the page)");
    uint16_t st[8];
    uint64_t local;
    uint64_t ft = fle_time_now_filetime();

    if (!fle_utc_to_local(ft, FLE_TZ_BIAS_MINUTES, &local) || !fle_time_filetime_to_systemtime(local, st))
        fle_stop("GetLocalTime", "emulated local time is outside the SYSTEMTIME range");
    fle_put_systemtime(out, st);
}

/* MSDN GetSystemTimeAsFileTime: VOID; UTC. NULL is not described by the page; stop. */
void fle_imp_GetSystemTimeAsFileTime(void)
{
    uint64_t out = fle_need_ptr("GetSystemTimeAsFileTime", R[1], "lpSystemTimeAsFileTime is NULL (not defined by the page)");
    uint64_t ft = fle_time_now_filetime();

    fle_st(out, 4, ft & 0xFFFFFFFFu);
    fle_st(out + 4, 4, ft >> 32);
}

/* MSDN GetTickCount64: milliseconds since the system start, 64 bits (no wrap at 2^32). The same virtual clock
 * as GetTickCount (fle_sys.c); that both read one clock is the runtime's choice. */
void fle_imp_GetTickCount64(void)
{
    R[0] = fle_clock_read_ticks() / FLE_TICKS_PER_MS;
}

/* ---- Time zone ---- */
static void fle_put_ascii16(uint64_t dst, const char *s, unsigned units)
{
    unsigned i;
    for (i = 0; i < units; i++) fle_st(dst + 2u * i, 2, i < strlen(s) ? (uint8_t)s[i] : 0);
}

/* MSDN GetTimeZoneInformation: with no DST transition dates the return value is TIME_ZONE_ID_UNKNOWN (0); the
 * structure holds the bias (minutes) and the names; a StandardDate/DaylightDate with wMonth 0 means "no DST"
 * (TIME_ZONE_INFORMATION page). All the other members are zero. */
void fle_imp_GetTimeZoneInformation(void)
{
    uint64_t out = fle_need_ptr("GetTimeZoneInformation", R[1], "lpTimeZoneInformation is NULL (not defined by the page)");
    unsigned i;

    for (i = 0; i < FLE_TZI_SIZE; i++) fle_st(out + i, 1, 0);
    fle_st(out, 4, (uint32_t)(int32_t)FLE_TZ_BIAS_MINUTES);
    fle_put_ascii16(out + FLE_TZI_STD_NAME, FLE_TZ_STANDARD_NAME, 32);
    fle_put_ascii16(out + FLE_TZI_DLT_NAME, FLE_TZ_DAYLIGHT_NAME, 32);
    R[0] = FLE_TIME_ZONE_ID_UNKNOWN;
}

/* ---- Conversions ---- */
void fle_imp_FileTimeToSystemTime(void)
{
    uint64_t pin = fle_need_ptr("FileTimeToSystemTime", R[1], "lpFileTime is NULL (not defined by the page)");
    uint64_t pout = fle_need_ptr("FileTimeToSystemTime", R[2], "lpSystemTime is NULL (not defined by the page)");
    uint64_t ft = fle_ld(pin, 4) | (fle_ld(pin + 4, 4) << 32);
    uint16_t st[8];

    if (!fle_time_filetime_to_systemtime(ft, st)) {
        if (ft < FLE_TIME_FILETIME_LIMIT) fle_stop("FileTimeToSystemTime", "result year above 30827, the documented wYear maximum (the page defines no result)");
        fle_fail(FLE_ERR_INVALID_PARAMETER);
        return;
    }
    fle_put_systemtime(pout, st);
    R[0] = 1;
}

void fle_imp_SystemTimeToFileTime(void)
{
    uint64_t pin = fle_need_ptr("SystemTimeToFileTime", R[1], "lpSystemTime is NULL (not defined by the page)");
    uint64_t pout = fle_need_ptr("SystemTimeToFileTime", R[2], "lpFileTime is NULL (not defined by the page)");
    uint16_t st[8];
    uint64_t ft;

    fle_get_systemtime(pin, st);
    if (!fle_time_systemtime_to_filetime(st, &ft)) {
        fle_fail(FLE_ERR_INVALID_PARAMETER);
        return;
    }
    fle_st(pout, 4, ft & 0xFFFFFFFFu);
    fle_st(pout + 4, 4, ft >> 32);
    R[0] = 1;
}

/* MSDN FileTimeToLocalFileTime and LocalFileTimeToFileTime: the output "cannot be the same as" the input. */
static void fle_ft_zone(const char *api, int to_local)
{
    uint64_t pin = fle_need_ptr(api, R[1], "input pointer is NULL (not defined by the page)");
    uint64_t pout = fle_need_ptr(api, R[2], "output pointer is NULL (not defined by the page)");
    uint64_t v = fle_ld(pin, 4) | (fle_ld(pin + 4, 4) << 32), r;

    if (pin == pout) fle_stop(api, "output is the same as the input (the page forbids it)");
    if (!(to_local ? fle_utc_to_local(v, FLE_TZ_BIAS_MINUTES, &r) : fle_local_to_utc(v, FLE_TZ_BIAS_MINUTES, &r))) {
        fle_fail(FLE_ERR_INVALID_PARAMETER);
        return;
    }
    fle_st(pout, 4, r & 0xFFFFFFFFu);
    fle_st(pout + 4, 4, r >> 32);
    R[0] = 1;
}

void fle_imp_FileTimeToLocalFileTime(void) { fle_ft_zone("FileTimeToLocalFileTime", 1); }
void fle_imp_LocalFileTimeToFileTime(void) { fle_ft_zone("LocalFileTimeToFileTime", 0); }

/* MSDN SystemTimeToTzSpecificLocalTime: NULL zone means the current zone. A given structure is used for its Bias
 * only when neither transition date is supplied (wMonth 0 in both); then the bias that applies is Bias (the
 * StandardBias and DaylightBias are ignored when the dates are not supplied: TIME_ZONE_INFORMATION page).
 * Transition rules are not implemented: a structure that has them stops the run. */
void fle_imp_SystemTimeToTzSpecificLocalTime(void)
{
    static const char *api = "SystemTimeToTzSpecificLocalTime";
    uint64_t ptz = R[1];
    uint64_t pin = fle_need_ptr(api, R[2], "lpUniversalTime is NULL (not defined by the page)");
    uint64_t pout = fle_need_ptr(api, R[8], "lpLocalTime is NULL (not defined by the page)");
    int32_t bias = FLE_TZ_BIAS_MINUTES;
    uint16_t st[8], lt[8];
    uint64_t utc, local;

    if (ptz != 0) {
        uint16_t sm = (uint16_t)fle_ld(ptz + FLE_TZI_STD_DATE + 2, 2), dm = (uint16_t)fle_ld(ptz + FLE_TZI_DLT_DATE + 2, 2);

        if (sm != 0 || dm != 0)
            fle_stop(api, "a time zone with daylight-saving transition dates is not implemented");
        bias = (int32_t)(uint32_t)fle_ld(ptz, 4);
    }
    fle_get_systemtime(pin, st);
    if (!fle_time_systemtime_to_filetime(st, &utc) || !fle_utc_to_local(utc, bias, &local) ||
        !fle_time_filetime_to_systemtime(local, lt)) {
        fle_fail(FLE_ERR_INVALID_PARAMETER);
        return;
    }
    fle_put_systemtime(pout, lt);
    R[0] = 1;
}

/* ---- MS-DOS date and time (MSDN DosDateTimeToFileTime, FileTimeToDosDateTime) ----
 * date: bits 0-4 day (1-31), 5-8 month, 9-15 year - 1980. time: bits 0-4 seconds / 2, 5-10 minute, 11-15 hour.
 * The result is UTC-based (MS-DOS Date and Time page: "convert it to a UTC-based time"), so no zone is applied.
 * Fields that are not a real date or time make DosDateTimeToFileTime fail (the page names no case). */
void fle_imp_DosDateTimeToFileTime(void)
{
    uint32_t date = (uint32_t)R[1] & 0xFFFFu, time = (uint32_t)R[2] & 0xFFFFu;
    uint64_t pout = fle_need_ptr("DosDateTimeToFileTime", R[8], "lpFileTime is NULL (not defined by the page)");
    uint16_t st[8];
    uint64_t ft;

    st[0] = (uint16_t)(1980 + (date >> 9));
    st[1] = (uint16_t)((date >> 5) & 0xF);
    st[2] = 0;
    st[3] = (uint16_t)(date & 0x1F);
    st[4] = (uint16_t)(time >> 11);
    st[5] = (uint16_t)((time >> 5) & 0x3F);
    st[6] = (uint16_t)((time & 0x1F) * 2);
    st[7] = 0;
    if (!fle_time_systemtime_to_filetime(st, &ft)) {
        fle_fail(FLE_ERR_INVALID_PARAMETER);
        return;
    }
    fle_st(pout, 4, ft & 0xFFFFFFFFu);
    fle_st(pout + 4, 4, ft >> 32);
    R[0] = 1;
}

/* The page: only 1/1/1980 to 12/31/2107 can be represented; outside it the conversion fails. Seconds are stored
 * divided by 2; an odd second is truncated (the page does not say how it rounds: assumption). */
void fle_imp_FileTimeToDosDateTime(void)
{
    uint64_t pin = fle_need_ptr("FileTimeToDosDateTime", R[1], "lpFileTime is NULL (not defined by the page)");
    uint64_t pdate = fle_need_ptr("FileTimeToDosDateTime", R[2], "lpFatDate is NULL (not defined by the page)");
    uint64_t ptime = fle_need_ptr("FileTimeToDosDateTime", R[8], "lpFatTime is NULL (not defined by the page)");
    uint64_t ft = fle_ld(pin, 4) | (fle_ld(pin + 4, 4) << 32);
    uint16_t st[8];

    if (!fle_time_filetime_to_systemtime(ft, st) || st[0] < 1980 || st[0] > 2107) {
        fle_fail(FLE_ERR_INVALID_PARAMETER);
        return;
    }
    fle_st(pdate, 2, (uint32_t)(((st[0] - 1980) << 9) | (st[1] << 5) | st[3]));
    fle_st(ptime, 2, (uint32_t)((st[4] << 11) | (st[5] << 5) | (st[6] / 2)));
    R[0] = 1;
}

/* ---- GetDateFormatW ----
 * Implemented: a format picture (lpFormat not NULL), flags 0, en-US family of locales, pictures d, dd, M, MM,
 * yy, yyyy, yyyyy, quoted text and other punctuation. Everything else stops the run (spec note): names (ddd, dddd,
 * MMM, MMMM) and the era (g, gg) need data the pages read do not give; y and yyy are described differently by the
 * two pages; with lpFormat NULL the locale's short-date picture is needed; flags with a picture are not defined. */
#define FLE_LOCALE_EN_US   0x0409u
#define FLE_LOCALE_USER    0x0400u
#define FLE_LOCALE_SYSTEM  0x0800u
#define FLE_LOCALE_INVAR   0x007Fu
#define FLE_DATE_BUF       1024u

static size_t fle_put_num(uint16_t *out, size_t n, uint32_t v, unsigned width)
{
    char t[12];
    unsigned len = 0, i;

    do { t[len++] = (char)('0' + v % 10); v /= 10; } while (v);
    while (len < width) t[len++] = '0';
    for (i = 0; i < len; i++) out[n + i] = (uint16_t)(uint8_t)t[len - 1 - i];
    return n + len;
}

static size_t fle_run(const uint16_t *f, size_t fl, size_t i, uint16_t c)
{
    size_t k = 0;
    while (i + k < fl && f[i + k] == c) k++;
    return k;
}

static size_t fle_date_picture(const uint16_t *f, size_t fl, const uint16_t st[8], uint16_t *out)
{
    static const char *api = "GetDateFormatW";
    size_t i = 0, n = 0;

    while (i < fl) {
        uint16_t c = f[i];
        size_t k;

        if (c == '\'') {
            i++;
            for (;;) {
                if (i >= fl) break;
                if (f[i] == '\'') {
                    if (i + 1 < fl && f[i + 1] == '\'') { out[n++] = '\''; i += 2; continue; }
                    i++;
                    break;
                }
                out[n++] = f[i++];
            }
        } else if (c == 'd') {
            k = fle_run(f, fl, i, 'd');
            if (k > 2) fle_stop(api, "day-name pictures (ddd, dddd) need the locale's day names, which the pages read do not give");
            n = fle_put_num(out, n, st[3], (unsigned)k == 2 ? 2 : 1);
            i += k;
        } else if (c == 'M') {
            k = fle_run(f, fl, i, 'M');
            if (k > 2) fle_stop(api, "month-name pictures (MMM, MMMM) need the locale's month names, which the pages read do not give");
            n = fle_put_num(out, n, st[1], (unsigned)k == 2 ? 2 : 1);
            i += k;
        } else if (c == 'y') {
            k = fle_run(f, fl, i, 'y');
            if (k == 1 || k == 3) fle_stop(api, "y and yyy: the two pages read describe them differently (last digit / treated as yy, yyyy)");
            if (k > 5) fle_stop(api, "more than five y in a picture is not defined by the pages");
            if (k == 2) n = fle_put_num(out, n, st[0] % 100u, 2);
            else {
                if (st[0] > 9999) fle_stop(api, "a year above 9999 with yyyy is not defined for the Gregorian calendar by the pages");
                n = fle_put_num(out, n, st[0], 4);
            }
            i += k;
        } else if (c == 'g') {
            fle_stop(api, "era pictures (g, gg) need the era string (CAL_SERASTRING), which the pages read do not give");
        } else if ((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z')) {
            fle_stop(api, "an unquoted letter that is not a picture: the page leaves the result to 'the best possible string'");
        } else {
            out[n++] = c;
            i++;
        }
        if (n + 8 > FLE_DATE_BUF) fle_stop(api, "formatted date longer than the supported limit");
    }
    return n;
}

void fle_imp_GetDateFormatW(void)
{
    static const char *api = "GetDateFormatW";
    uint32_t lcid = (uint32_t)R[1], flags = (uint32_t)R[2];
    uint64_t pdate = R[8], pfmt = R[9];
    uint64_t pout = fle_arg(5);
    int32_t cch = (int32_t)(uint32_t)fle_arg(6);
    uint16_t st[8], fbuf[512], obuf[FLE_DATE_BUF];
    size_t fl, n;
    uint64_t ft, local;

    if (lcid != FLE_LOCALE_EN_US && lcid != FLE_LOCALE_USER && lcid != FLE_LOCALE_SYSTEM && lcid != FLE_LOCALE_INVAR)
        fle_stop(api, "a locale other than the emulated en-US (decision 0016)");
    if (pfmt == 0) fle_stop(api, "lpFormat is NULL: the locale's date picture is not given by the pages read");
    if (flags != 0) fle_stop(api, "flags with a format picture: the page says the flags apply when lpFormat is NULL");
    fl = fle_w_read(api, pfmt, fbuf, 512);
    if (pdate != 0) {
        fle_get_systemtime(pdate, st);
        if (st[0] < FLE_ST_YEAR_MIN || st[0] > FLE_ST_YEAR_MAX || st[1] < 1 || st[1] > 12 || st[3] < 1 ||
            st[3] > fle_month_len(st[0], st[1])) {
            fle_fail(FLE_ERR_INVALID_PARAMETER);
            return;
        }
    } else {
        ft = fle_time_now_filetime();
        if (!fle_utc_to_local(ft, FLE_TZ_BIAS_MINUTES, &local) || !fle_time_filetime_to_systemtime(local, st))
            fle_stop(api, "emulated local time is outside the SYSTEMTIME range");
    }
    if (cch < 0) { fle_fail(FLE_ERR_INVALID_PARAMETER); return; }
    n = fle_date_picture(fbuf, fl, st, obuf);
    if (cch == 0) { R[0] = (uint64_t)(n + 1); return; }
    if (pout == 0 || (size_t)cch < n + 1) { fle_fail(FLE_ERR_INSUFFICIENT_BUFFER); return; }
    obuf[n] = 0;
    fle_w_write(pout, obuf, n + 1);
    R[0] = (uint64_t)(n + 1);
}

/* Decision 0210 (api2, opt-in test tool): the same cases run against Wine 10.0's real APIs (API2_WINE: mingw build, `wine api2_wine.exe`) and against the runtime's fle_imp_* functions (native build,
 * lift/harness/api2/build.sh). Both print one line per case; the two outputs must be equal for the cases listed in `api2-diff-cases.txt`. A line starts with a case label. Nothing is printed that depends on
 * addresses, times of day or the host. The lines "native-only" at the end are the cases for which Wine is not a reference (Wine differs from the environment by design: no large pages there, symbolic links
 * can be created there, ShellExecute finds a Unix program there); the Rust test pins their expected output by hand from the Microsoft pages. */
#include <stdio.h>
#include <string.h>
#include <stdint.h>

#ifdef API2_WINE
#include <windows.h>
#define LASTERR() ((unsigned)GetLastError())
#define SETERR(e) SetLastError(e)
#define P(x) (x)
typedef WORD W16;
#else
#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_glob.h"
#include "fle_locale.h"
#include "fle_api2.h"
#include "fle_proc.h"
uint64_t R[16];
typedef unsigned int UINT, DWORD, LCID, BOOL, LCTYPE;
typedef int INT;
typedef uint16_t W16, WORD, WCHAR;
typedef void *HANDLE;
typedef struct { WORD wYear, wMonth, wDayOfWeek, wDay, wHour, wMinute, wSecond, wMilliseconds; } SYSTEMTIME;
typedef struct { int32_t Bias; WCHAR StandardName[32]; SYSTEMTIME StandardDate; int32_t StandardBias; WCHAR DaylightName[32]; SYSTEMTIME DaylightDate; int32_t DaylightBias; } TIME_ZONE_INFORMATION;
#define A(x) ((uint64_t)(uintptr_t)(x))
#define I(x) ((uint64_t)(int64_t)(x))
#define C(f, ...) fle_api2_call(fle_imp_##f, __VA_ARGS__)
static unsigned LASTERR(void) { return (unsigned)C(GetLastError, 0, 0, 0, 0, 0, 0, 0, 0); }
static void SETERR(unsigned e) { (void)C(SetLastError, e, 0, 0, 0, 0, 0, 0, 0); }
static int CompareStringA(LCID l, DWORD f, const char *a, int ca, const char *b, int cb) { return (int)C(CompareStringA, l, f, A(a), I(ca), A(b), I(cb), 0, 0); }
static int LCMapStringA(LCID l, DWORD f, const char *s, int cs, char *d, int cd) { return (int)C(LCMapStringA, l, f, A(s), I(cs), A(d), I(cd), 0, 0); }
static BOOL GetStringTypeA(LCID l, DWORD t, const char *s, int cs, WORD *o) { return (BOOL)C(GetStringTypeA, l, t, A(s), I(cs), A(o), 0, 0, 0); }
static int GetLocaleInfoA(LCID l, LCTYPE t, char *d, int cd) { return (int)C(GetLocaleInfoA, l, t, A(d), I(cd), 0, 0, 0, 0); }
static BOOL TzSpecificLocalTimeToSystemTime(const TIME_ZONE_INFORMATION *tz, const SYSTEMTIME *l, SYSTEMTIME *u) { return (BOOL)C(TzSpecificLocalTimeToSystemTime, A(tz), A(l), A(u), 0, 0, 0, 0, 0); }
static BOOL WriteConsoleA(HANDLE h, const void *b, DWORD n, DWORD *w, void *r) { return (BOOL)C(WriteConsoleA, A(h), A(b), n, A(w), A(r), 0, 0, 0); }
static BOOL ReadConsoleW(HANDLE h, void *b, DWORD n, DWORD *r, void *c) { return (BOOL)C(ReadConsoleW, A(h), A(b), n, A(r), A(c), 0, 0, 0); }
static UINT GetConsoleCP(void) { return (UINT)C(GetConsoleCP, 0, 0, 0, 0, 0, 0, 0, 0); }
static uint64_t VirtualQueryEx(HANDLE h, const void *a, void *b, uint64_t n) { return C(VirtualQueryEx, A(h), A(a), A(b), n, 0, 0, 0, 0); }
#define NORM_IGNORECASE 1u
#define LOCALE_INVARIANT 0x7fu
#define LOCALE_USER_DEFAULT 0x0400u
#define LCMAP_LOWERCASE 0x100u
#define LCMAP_UPPERCASE 0x200u
#define CT_CTYPE1 1u
#define LOCALE_RETURN_NUMBER 0x20000000u
#define INVALID_HANDLE_VALUE ((HANDLE)(intptr_t)-1)
#endif

static void cmpa(const char *label, DWORD f, const char *a, int ca, const char *b, int cb)
{
    int r;
    SETERR(0);
    r = CompareStringA(LOCALE_USER_DEFAULT, f, a, ca, b, cb);
    printf("CompareStringA %s = %d", label, r);
    if (r == 0) printf(" err=%u", LASTERR());
    printf("\n");
}

static void lmap(const char *label, DWORD f, const char *s, int cs, int cd)
{
    char out[64];
    int r;
    memset(out, 'x', sizeof out);
    SETERR(0);
    r = LCMapStringA(LOCALE_USER_DEFAULT, f, s, cs, cd ? out : NULL, cd);
    printf("LCMapStringA %s = %d", label, r);
    if (r == 0) printf(" err=%u", LASTERR());
    else if (cd) { int i; printf(" ["); for (i = 0; i < r; i++) printf("%02x", (unsigned char)out[i]); printf("]"); }
    printf("\n");
}

static void gti(const char *label, DWORD t, const char *s, int cs, int n)
{
    WORD o[16];
    int i;
    BOOL r;
    memset(o, 0xAB, sizeof o);
    SETERR(0);
    r = GetStringTypeA(LOCALE_USER_DEFAULT, t, s, cs, o);
    printf("GetStringTypeA %s = %d", label, (int)r);
    if (!r) printf(" err=%u", LASTERR());
    else for (i = 0; i < n; i++) printf(" %04x", (unsigned)o[i]);
    printf("\n");
}

static void gli(const char *label, LCTYPE t, int cd)
{
    char out[64];
    int r;
    memset(out, 'x', sizeof out);
    SETERR(0);
    r = GetLocaleInfoA(LOCALE_USER_DEFAULT, t, cd ? out : NULL, cd);
    printf("GetLocaleInfoA %s = %d", label, r);
    if (r == 0) printf(" err=%u", LASTERR());
    else if (cd && !(t & LOCALE_RETURN_NUMBER)) printf(" \"%s\"", out);
    else if (cd) { DWORD v; memcpy(&v, out, 4); printf(" #%u", (unsigned)v); }
    printf("\n");
}

static SYSTEMTIME mk(WORD y, WORD mo, WORD d, WORD h, WORD mi)
{
    SYSTEMTIME s;
    memset(&s, 0, sizeof s);
    s.wYear = y; s.wMonth = mo; s.wDay = d; s.wHour = h; s.wMinute = mi;
    return s;
}

static void tz1(const char *label, const TIME_ZONE_INFORMATION *tz, SYSTEMTIME l)
{
    SYSTEMTIME u;
    BOOL r;
    memset(&u, 0xAB, sizeof u);
    SETERR(0);
    r = TzSpecificLocalTimeToSystemTime(tz, &l, &u);
    printf("TzSpecificLocalTimeToSystemTime %s = %d", label, (int)r);
    if (!r) printf(" err=%u", LASTERR());
    else printf(" %04u-%02u-%02u dow%u %02u:%02u:%02u.%03u", u.wYear, u.wMonth, u.wDayOfWeek ? u.wDay : u.wDay, u.wDayOfWeek, u.wHour, u.wMinute, u.wSecond, u.wMilliseconds);
    printf("\n");
}

int main(void)
{
    TIME_ZONE_INFORMATION us, au, abs_, nodst;
    DWORD wr = 0x1234;
    char big[8];
    unsigned char mbi_buf[64];
#ifdef API2_WINE
    MEMORY_BASIC_INFORMATION *mbi = (MEMORY_BASIC_INFORMATION *)mbi_buf;
#else
    void *mbi = mbi_buf;
#endif

    setvbuf(stdout, NULL, _IONBF, 0);
    /* CompareStringA */
    cmpa("eq", 0, "abc", -1, "abc", -1);
    cmpa("lt", 0, "abc", -1, "abd", -1);
    cmpa("gt", 0, "abd", -1, "abc", -1);
    cmpa("icase", NORM_IGNORECASE, "ABC", -1, "abc", -1);
    cmpa("len3", 0, "abcdef", 3, "abcxyz", 3);
    cmpa("len-mixed", 0, "abc", 3, "abc", -1);
    cmpa("len-second", 0, "abcx", 3, "abcdef", 3);
    cmpa("len-first-short", 0, "ab", 2, "abc", 3);
    cmpa("prefix", 0, "ab", -1, "abc", -1);
    cmpa("empty-len0", 0, "abc", 0, "", 0);
    cmpa("empty-vs-empty", 0, "", -1, "", -1);
    cmpa("digits", 0, "a10", -1, "a9", -1);
    cmpa("null1", 0, NULL, -1, "a", -1);
    cmpa("null2", 0, "a", -1, NULL, -1);

    /* LCMapStringA */
    lmap("upper", LCMAP_UPPERCASE, "Hello, World 123", -1, 64);
    lmap("lower", LCMAP_LOWERCASE, "Hello, World 123", -1, 64);
    lmap("upper-size", LCMAP_UPPERCASE, "Hello", -1, 0);
    lmap("upper-len", LCMAP_UPPERCASE, "hello world", 5, 64);
    lmap("upper-len-size", LCMAP_UPPERCASE, "hello world", 5, 0);
    lmap("small-buffer", LCMAP_UPPERCASE, "hello", -1, 3);
    lmap("exact-buffer", LCMAP_UPPERCASE, "hello", -1, 6);
    lmap("null-src", LCMAP_UPPERCASE, NULL, -1, 8);
    lmap("empty", LCMAP_UPPERCASE, "", -1, 8);

    /* GetStringTypeA */
    gti("ctype1-ascii", CT_CTYPE1, "aA1 .\t", 6, 6);
    gti("ctype1-nul", CT_CTYPE1, "ab", -1, 3);
    gti("ctype1-cp1252", CT_CTYPE1, "\xe9\xc9\xa0", 3, 3);
    gti("null-src", CT_CTYPE1, NULL, 3, 0);

    /* GetLocaleInfoA */
    gli("ILANGUAGE", 0x1u, 16);
    gli("SDECIMAL", 0xEu, 16);
    gli("STHOUSAND", 0xFu, 16);
    gli("ANSICP", 0x1004u, 16);
    gli("ANSICP-number", 0x1004u | LOCALE_RETURN_NUMBER, 4);
    gli("ANSICP-size", 0x1004u, 0);
    gli("ANSICP-small", 0x1004u, 3);
    gli("ILANGUAGE-exact", 0x1u, 5);

    /* TzSpecificLocalTimeToSystemTime */
    memset(&us, 0, sizeof us);
    us.Bias = 300; us.StandardBias = 0; us.DaylightBias = -60;
    us.StandardDate.wMonth = 11; us.StandardDate.wDayOfWeek = 0; us.StandardDate.wDay = 1; us.StandardDate.wHour = 2;
    us.DaylightDate.wMonth = 3; us.DaylightDate.wDayOfWeek = 0; us.DaylightDate.wDay = 2; us.DaylightDate.wHour = 2;
    tz1("us-summer", &us, mk(2024, 7, 1, 12, 0));
    tz1("us-winter", &us, mk(2024, 1, 15, 12, 0));
    tz1("us-ambiguous-0130", &us, mk(2024, 11, 3, 1, 30));
    tz1("us-end-0200", &us, mk(2024, 11, 3, 2, 0));
    tz1("us-invalid-0230", &us, mk(2024, 3, 10, 2, 30));
    tz1("us-before-start-0159", &us, mk(2024, 3, 10, 1, 59));
    tz1("us-start-0200", &us, mk(2024, 3, 10, 2, 0));
    tz1("us-year-rollover", &us, mk(2024, 12, 31, 22, 0));
    tz1("us-2025-last-sun-style", &us, mk(2025, 3, 9, 3, 0));
    memset(&au, 0, sizeof au);
    au.Bias = -600; au.StandardBias = 0; au.DaylightBias = -60;
    au.StandardDate.wMonth = 4; au.StandardDate.wDayOfWeek = 0; au.StandardDate.wDay = 1; au.StandardDate.wHour = 3;
    au.DaylightDate.wMonth = 10; au.DaylightDate.wDayOfWeek = 0; au.DaylightDate.wDay = 1; au.DaylightDate.wHour = 2;
    tz1("au-january(dst)", &au, mk(2024, 1, 10, 12, 0));
    tz1("au-july(std)", &au, mk(2024, 7, 10, 12, 0));
    tz1("au-december(dst)", &au, mk(2024, 12, 25, 0, 30));
    tz1("au-ambiguous-0230", &au, mk(2024, 4, 7, 2, 30));
    memset(&abs_, 0, sizeof abs_);
    abs_.Bias = 0; abs_.DaylightBias = -60;
    abs_.StandardDate.wYear = 2024; abs_.StandardDate.wMonth = 10; abs_.StandardDate.wDay = 27; abs_.StandardDate.wHour = 2;
    abs_.DaylightDate.wYear = 2024; abs_.DaylightDate.wMonth = 3; abs_.DaylightDate.wDay = 31; abs_.DaylightDate.wHour = 1;
    tz1("absolute-summer", &abs_, mk(2024, 6, 1, 12, 0));
    tz1("absolute-winter", &abs_, mk(2024, 12, 1, 12, 0));
    memset(&nodst, 0, sizeof nodst);
    nodst.Bias = -540; nodst.StandardBias = 0; nodst.DaylightBias = -60;
    tz1("no-dst", &nodst, mk(2024, 7, 1, 12, 0));
    tz1("invalid-month", &us, mk(2024, 13, 1, 12, 0));
    tz1("invalid-day", &us, mk(2023, 2, 29, 12, 0));
    tz1("leap-day", &us, mk(2024, 2, 29, 23, 30));
    tz1("year-end-carry", &nodst, mk(2024, 12, 31, 20, 0));
    tz1("null-tz-utc", NULL, mk(2024, 7, 1, 12, 0));

    /* console / process handles (the result is read before GetLastError: the order of evaluation of printf's arguments is unspecified) */
    {
        long long r;
        unsigned e;

        SETERR(0); r = (long long)WriteConsoleA(NULL, "x", 1, &wr, NULL); e = LASTERR(); printf("WriteConsoleA null = %lld err=%u\n", r, e);
        SETERR(0); r = (long long)WriteConsoleA(INVALID_HANDLE_VALUE, "x", 1, &wr, NULL); e = LASTERR(); printf("WriteConsoleA invalid = %lld err=%u written=%u\n", r, e, (unsigned)wr);
        SETERR(0); r = (long long)WriteConsoleA((HANDLE)(intptr_t)0x1234, "x", 1, NULL, NULL); e = LASTERR(); printf("WriteConsoleA bogus = %lld err=%u\n", r, e);
        SETERR(0); r = (long long)ReadConsoleW(NULL, big, 1, &wr, NULL); e = LASTERR(); printf("ReadConsoleW null = %lld err=%u\n", r, e);
        SETERR(0); r = (long long)GetConsoleCP(); e = LASTERR(); printf("GetConsoleCP = %lld err=%u\n", r, e);
        SETERR(0); r = (long long)VirtualQueryEx((HANDLE)(intptr_t)0x1234, big, mbi, 48); e = LASTERR(); printf("VirtualQueryEx bogus-handle = %lld err=%u\n", r, e);
    }

#ifndef API2_WINE
    /* native-only cases: Wine is not a reference (it has large pages, creates symbolic links, finds Unix programs); the expected lines are written by hand from the pages in tests/api2.rs */
    {
        static const uint16_t link[] = { 'C', ':', '\\', 'l', 0 }, target[] = { 'C', ':', '\\', 't', 0 }, file[] = { 'x', '.', 'q', 'z', 0 }, verb[] = { 'o', 'p', 'e', 'n', 0 };
        uint64_t sei[14];
        unsigned e;
        uint64_t r;

        printf("NATIVE GetLargePageMinimum = %llu\n", (unsigned long long)C(GetLargePageMinimum, 0, 0, 0, 0, 0, 0, 0, 0));
        SETERR(0); r = C(CreateSymbolicLinkW, A(link), A(target), 0, 0, 0, 0, 0, 0); e = LASTERR(); printf("NATIVE CreateSymbolicLinkW = %u err=%u\n", (unsigned)(r & 0xFF), e);
        SETERR(0); r = C(CreateSymbolicLinkW, A(link), A(target), 1, 0, 0, 0, 0, 0); e = LASTERR(); printf("NATIVE CreateSymbolicLinkW dir = %u err=%u\n", (unsigned)(r & 0xFF), e);
        (void)C(OutputDebugStringA, A("hello"), 0, 0, 0, 0, 0, 0, 0);
        printf("NATIVE OutputDebugStringA done\n");
        r = C(ShellExecuteA, 0, A("open"), A("x.qz"), 0, 0, 1, 0, 0);
        printf("NATIVE ShellExecuteA = %llu\n", (unsigned long long)r);
        memset(sei, 0, sizeof sei);
        ((uint32_t *)sei)[0] = 112;
        sei[2] = A(verb); sei[3] = A(file); sei[7] = 0xFFFF;
        SETERR(0); r = C(ShellExecuteExW, A(sei), 0, 0, 0, 0, 0, 0, 0); e = LASTERR();
        printf("NATIVE ShellExecuteExW = %u err=%u hInstApp=%llu\n", (unsigned)(r & 0xFFFFFFFF), e, (unsigned long long)sei[7]);
    }
#endif
    return 0;
}

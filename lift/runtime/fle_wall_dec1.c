/* Walls that the sample-decoder DLLs of the installer reached when they were lifted and run (decision 0206, dec1, PROVISIONAL, OPT-IN: a function here is reached only through a row of
 * harness/seeds/dec1-imports.txt; a build without those rows never calls it). oggio_x64.dll, wavpackdll_x64.dll and DelZip64.dll are MSVC static-CRT DLLs: their DllMain reaches the CRT's
 * kernel32 imports. Every answer follows a Learn page; what the page does not give is a named stop or an S-dec1-N choice (listed in the decision document). */
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_vfs.h"
#include "fle_callback.h"
#include "fle_locale.h"

void fle_imp_InitializeCriticalSectionAndSpinCount(void);

#define FLE_ERR_INVALID_HANDLE 6u        /* the code of the other file functions of fle_vfs.c for an invalid or closed handle */
#define FLE_EMULATED_LCID 0x0409u        /* en-US: the emulated locale of fle_locale.c (decisions 0014, 0044) */

/* kernel32!InitializeCriticalSectionEx: BOOL f(LPCRITICAL_SECTION rcx, DWORD dwSpinCount edx, DWORD Flags r8d). Learn: Flags may be 0 or a combination of CRITICAL_SECTION_NO_DEBUG_INFO (0x01000000),
 * RTL_CRITICAL_SECTION_FLAG_DYNAMIC_SPIN, ..._STATIC_INIT, ..._RESOURCE_TYPE, ..._FORCE_DEBUG_INFO ('Reserved; do not use' for the RTL ones); 'The function returns a nonzero value' (Windows Vista and later).
 * Same object as InitializeCriticalSectionAndSpinCount (decision 0078: one processor, the spin count is not kept); the debug-info flag only changes bookkeeping that the table does not have. Any other flag stops. */
void fle_imp_InitializeCriticalSectionEx(void)
{
    uint32_t flags = (uint32_t)R[8];

    if (flags & ~0x01000000u) fle_stop("InitializeCriticalSectionEx", "Flags other than 0 and CRITICAL_SECTION_NO_DEBUG_INFO: the page marks the RTL_* flags reserved");
    fle_imp_InitializeCriticalSectionAndSpinCount();
    R[0] = 1;
}

/* kernel32!GetFileSizeEx: BOOL f(HANDLE rcx, PLARGE_INTEGER lpFileSize rdx). Learn: 'Retrieves the size of the specified file, in bytes'; nonzero on success; the handle must have FILE_READ_ATTRIBUTES access.
 * The size is the file's in the volume. An invalid or closed handle fails with ERROR_INVALID_HANDLE (6, as the other file functions). */
void fle_imp_GetFileSizeEx(void)
{
    static const char api[] = "GetFileSizeEx";
    const void *node;
    uint32_t attrs;
    uint64_t ct, at, wt, size;

    if (R[2] == 0) fle_stop(api, "lpFileSize is NULL: the page gives no behaviour");
    if (!fle_vfs_file_stat(api, R[1], &node, &attrs, &ct, &at, &wt, &size)) {
        fle_set_last_error(FLE_ERR_INVALID_HANDLE);
        R[0] = 0;
        return;
    }
    fle_st(R[2], 8, size);
    R[0] = 1;
}

/* kernel32!GetUserDefaultLCID: LCID f(void). Learn: 'Retrieves the locale identifier (LCID) for the user default locale'. The emulated user locale is en-US (0x0409). */
void fle_imp_GetUserDefaultLCID(void)
{
    R[0] = FLE_EMULATED_LCID;
}

/* The locale-name entry points (Vista and later) take a name where the older ones take an LCID. The only locale the environment emulates is en-US (decision 0044), so NULL (LOCALE_NAME_USER_DEFAULT,
 * 'the user default locale') and "en-US" mean 0x0409 and every other name stops by name. The Ex functions then run the LCID versions with the same remaining arguments (the arguments 2..6 sit in the same places). */
static void ex_locale(const char *api, uint64_t name)
{
    static const uint16_t en_us[] = {'e', 'n', '-', 'U', 'S', 0};
    unsigned i;

    if (name != 0) {
        for (i = 0; i < 6; i++)
            if ((uint16_t)fle_ld(name + 2u * i, 2) != en_us[i]) fle_stop(api, "a locale name other than NULL and \"en-US\" (the environment emulates one locale)");
    }
    R[1] = FLE_EMULATED_LCID;
}

/* kernel32!LCMapStringEx: int f(LPCWSTR name, DWORD flags, LPCWSTR src, int cchSrc, LPWSTR dest, int cchDest, LPNLSVERSIONINFO versionInformation, LPVOID lpReserved, LPARAM sortHandle). Learn: lpVersionInformation
 * 'Pointer to an NLSVERSIONINFO structure ... optional'; lpReserved 'Reserved; must be NULL'; sortHandle 'Reserved; must be NULL' (the page: 'Pointer to a sort handle ... can be NULL'). A non-NULL value of the three stops. */
void fle_imp_LCMapStringEx(void)
{
    static const char api[] = "LCMapStringEx";

    if (fle_arg(7) != 0 || fle_arg(8) != 0 || fle_arg(9) != 0) fle_stop(api, "lpVersionInformation, lpReserved or sortHandle is not NULL: the page gives no behaviour here");
    ex_locale(api, R[1]);
    fle_imp_LCMapStringW();
}

/* kernel32!CompareStringEx: int f(LPCWSTR name, DWORD flags, LPCWCH s1, int c1, LPCWCH s2, int c2, LPNLSVERSIONINFO versionInformation, LPVOID lpReserved, LPARAM lParam). The three trailing arguments must be NULL/0. */
void fle_imp_CompareStringEx(void)
{
    static const char api[] = "CompareStringEx";

    if (fle_arg(7) != 0 || fle_arg(8) != 0 || fle_arg(9) != 0) fle_stop(api, "lpVersionInformation, lpReserved or lParam is not NULL: the page gives no behaviour here");
    ex_locale(api, R[1]);
    fle_imp_CompareStringW();
}

/* kernel32!GetLocaleInfoEx: int f(LPCWSTR name, LCTYPE type, LPWSTR buf, int cch). Same as GetLocaleInfoW for the emulated locale, plus the values below that GetLocaleInfoW does not give (the MSVC C++ library's
 * std::locale::classic() initialisation asks for them while oggio's first call runs). The table is S-dec1-locale: values for en-US that no Learn page states (the LCTYPE constants and the 'return value counts the
 * terminating null; cchData 0 asks for the size; too small a buffer is ERROR_INSUFFICIENT_BUFFER (122)' rules are Learn's; the strings are the English day and month names and the en-US ordering, which are common
 * knowledge and not sourced from a page). LOCALE_RETURN_NUMBER (0x20000000) is answered for the numeric ones. */
typedef struct { uint32_t type; const char *val; int num; } gli_row;
static const gli_row gli_rows[] = {
    { 0x21, "0", 1 },   /* LOCALE_IDATE: short date order, 0 = M/D/Y */
    { 0x22, "0", 1 },   /* LOCALE_ILDATE: long date order, 0 = M D Y */
};

void fle_imp_GetLocaleInfoEx(void)
{
    static const char api[] = "GetLocaleInfoEx";
    uint32_t type = (uint32_t)R[2];
    uint32_t base = type & ~0x20000000u;
    int rn = (type & 0x20000000u) != 0;
    int32_t cch = (int32_t)R[9];
    uint64_t buf = R[8];
    unsigned i;

    ex_locale(api, R[1]);
    for (i = 0; i < sizeof gli_rows / sizeof gli_rows[0]; i++) {
        const char *v;
        int n;
        if (gli_rows[i].type != base) continue;
        if (rn && !gli_rows[i].num) fle_stop(api, "LOCALE_RETURN_NUMBER with a type that is not numeric");
        if (rn) {
            if (cch < 2) { if (cch == 0) { R[0] = 2; return; } fle_set_last_error(122); R[0] = 0; return; }
            fle_st(buf, 4, (uint32_t)atoi(gli_rows[i].val));
            R[0] = 2;
            return;
        }
        v = gli_rows[i].val;
        n = (int)strlen(v) + 1;
        if (cch == 0) { R[0] = (uint64_t)n; return; }
        if (cch < n) { fle_set_last_error(122); R[0] = 0; return; }
        for (n = 0; v[n]; n++) fle_st(buf + 2u * (unsigned)n, 2, (unsigned char)v[n]);
        fle_st(buf + 2u * (unsigned)n, 2, 0);
        R[0] = (uint64_t)n + 1;
        return;
    }
    fle_imp_GetLocaleInfoW();
}

/* kernel32!EnumSystemLocalesW: BOOL f(LOCALE_ENUMPROCW rcx, DWORD dwFlags edx). Learn: 'Enumerates the locales that are either installed on or supported by an operating system'; the callback is called with 'the locale
 * identifier ... as a Unicode string, hexadecimal' (the page's LOCALE_ENUMPROCW: 'lpLocaleString: Pointer to a buffer containing a null-terminated locale identifier string'), 'continues until the last locale is found or the
 * callback function returns FALSE'. The environment has one locale: the callback is called once with L"00000409" and the result is TRUE. A dwFlags value other than LCID_INSTALLED (1), LCID_SUPPORTED (2) or both (the page's
 * values) stops. S-dec1-enumlocales: one installed locale. */
void fle_imp_EnumSystemLocalesW(void)
{
    static const uint16_t id[] = {'0', '0', '0', '0', '0', '4', '0', '9', 0};
    static uint16_t buf[16] __attribute__((aligned(16)));
    uint64_t proc = R[1];
    uint32_t flags = (uint32_t)R[2];
    unsigned i;

    if (proc == 0) fle_stop("EnumSystemLocalesW", "lpLocaleEnumProc is NULL: the page gives no behaviour");
    if (flags == 0 || (flags & ~3u)) fle_stop("EnumSystemLocalesW", "dwFlags is not LCID_INSTALLED, LCID_SUPPORTED or both");
    for (i = 0; i < 9; i++) buf[i] = id[i];
    (void)fle_call_guest(proc, (uint64_t)(uintptr_t)buf, 0, 0, 0);
    R[0] = 1;
}

/* kernel32!CreateFileA: HANDLE f(LPCSTR lpFileName rcx, DWORD dwDesiredAccess edx, DWORD dwShareMode r8d, LPSECURITY_ATTRIBUTES r9, DWORD dwCreationDisposition [5], DWORD dwFlagsAndAttributes [6], HANDLE hTemplateFile [7]).
 * Learn: the same as CreateFileW with an ANSI name. The name is converted unit by unit; a byte above 0x7F would need the ANSI code page of the emulated system (the page does not give one: the conversion is not defined here) and
 * stops, as does a name of 260 characters or more (MAX_PATH, the limit of the stack buffer). S-dec1-createfilea. */
void fle_imp_CreateFileW(void);
void fle_imp_CreateFileA(void)
{
    static const char api[] = "CreateFileA";
    static uint16_t wbuf[264] __attribute__((aligned(16)));
    uint64_t p = R[1];
    unsigned i;

    if (p == 0) fle_stop(api, "lpFileName is NULL: the page gives no behaviour");
    for (i = 0; i < 261; i++) {
        uint8_t c = (uint8_t)fle_ld(p + i, 1);
        if (c >= 0x80) fle_stop(api, "a file name byte above 0x7F: the ANSI code page of the emulated system is not defined");
        wbuf[i] = c;
        if (c == 0) break;
    }
    if (i == 261) fle_stop(api, "a file name of 260 characters or more (MAX_PATH)");
    R[1] = (uint64_t)(uintptr_t)wbuf;
    fle_imp_CreateFileW();
}

/* kernel32!GetFileTime: BOOL f(HANDLE rcx, LPFILETIME lpCreationTime rdx, LPFILETIME lpLastAccessTime r8, LPFILETIME lpLastWriteTime r9). Learn: 'Retrieves the date and time that a file or directory was created, last accessed,
 * and last modified'; each pointer is optional ('This parameter can be NULL if the application does not require this information'); the handle needs GENERIC_READ. The times are the file's in the volume (the
 * FILETIME ticks that fle_vfs.c keeps). An invalid or closed handle fails with ERROR_INVALID_HANDLE (6, as the other file functions). DelZip64.dll asks for the times of the files it extracts. */
void fle_imp_GetFileTime(void)
{
    static const char api[] = "GetFileTime";
    const void *node;
    uint32_t attrs;
    uint64_t ct, at, wt, size;

    if (!fle_vfs_file_stat(api, R[1], &node, &attrs, &ct, &at, &wt, &size)) {
        fle_set_last_error(FLE_ERR_INVALID_HANDLE);
        R[0] = 0;
        return;
    }
    if (R[2]) fle_st(R[2], 8, ct);
    if (R[8]) fle_st(R[8], 8, at);
    if (R[9]) fle_st(R[9], 8, wt);
    R[0] = 1;
}

/* The system ANSI code page of the environment is 1252 (fle_emulated_locale.acp; en-US, decision 0044): a single-byte code page with no lead bytes (GetCPInfo of fle_locale.c says so). The two user32 string functions below follow it.
 * S-dec1-ansi: a ZIP written on a Japanese system (Shift-JIS names, as in serina1-serum2.zip) is therefore read here as CP1252 text, as a real en-US system would read it; a Japanese machine (ACP 932) would not. */

/* user32!CharNextA: LPSTR f(LPCSTR lpsz). Learn: 'Retrieves a pointer to the next character in a string. This function can handle strings consisting of either single- or multi-byte characters'; at the terminating null it
 * returns the pointer itself. With the single-byte ACP every byte is one character. */
void fle_imp_CharNextA(void)
{
    uint64_t p = R[1];

    if (p == 0) fle_stop("CharNextA", "lpsz is NULL: the page gives no behaviour");
    R[0] = (uint8_t)fle_ld(p, 1) == 0 ? p : p + 1;
}

/* user32!CharLowerBuffA: DWORD f(LPSTR lpsz, DWORD cchLength). Learn: 'Converts uppercase characters in a buffer to lowercase characters. The function converts the characters in place'; 'the return value is the number of
 * characters processed'. The mapping is the simple Unicode lower-case mapping restricted to CP1252 (not on the page; S-dec1-ansi): A-Z -> a-z, 0xC0-0xDE except 0xD7 (Latin-1 letters) -> +0x20, 0x8A -> 0x9A (S caron), 0x8C -> 0x9C (OE),
 * 0x8E -> 0x9E (Z caron), 0x9F -> 0xFF (Y diaeresis). */
void fle_imp_CharLowerBuffA(void)
{
    uint64_t p = R[1];
    uint32_t n = (uint32_t)R[2], i;

    if (p == 0 && n != 0) fle_stop("CharLowerBuffA", "lpsz is NULL: the page gives no behaviour");
    for (i = 0; i < n; i++) {
        uint8_t c = (uint8_t)fle_ld(p + i, 1), d = c;
        if (c >= 'A' && c <= 'Z') d = (uint8_t)(c + 32);
        else if (c >= 0xC0 && c <= 0xDE && c != 0xD7) d = (uint8_t)(c + 32);
        else if (c == 0x8A || c == 0x8C || c == 0x8E) d = (uint8_t)(c + 16);
        else if (c == 0x9F) d = 0xFF;
        if (d != c) fle_st(p + i, 1, d);
    }
    R[0] = n;
}

/* kernel32!InitOnceExecuteOnce: BOOL f(PINIT_ONCE rcx, PINIT_ONCE_FN rdx, PVOID Parameter r8, LPVOID *Context r9). Learn: runs InitFn 'one time'; the first caller's InitFn(InitOnce, Parameter, &Context) is called; if it
 * returns TRUE the state becomes complete and the context it stored is returned to this and every later caller (*Context, when non-NULL); if it returns FALSE 'the function fails' and the next caller runs InitFn again. The
 * INIT_ONCE object is opaque ('system use'), so the state is kept outside it, keyed by its address (as the SLIST heads of fle_sys.c). 64 objects; more stop. One thread (no waiting for a concurrent initializer). S-dec1-initonce. */
void fle_imp_InitOnceExecuteOnce(void)
{
    static uint64_t addr[64], ctx[64];
    static uint8_t done[64];
    static unsigned n;
    uint64_t once = R[1], fn = R[2], param = R[8], pctx = R[9];
    unsigned i;
    uint64_t local = 0, r;

    if (once == 0 || fn == 0) fle_stop("InitOnceExecuteOnce", "InitOnce or InitFn is NULL: the page gives no behaviour");
    for (i = 0; i < n && addr[i] != once; i++) {}
    if (i == n) {
        if (n == 64) fle_stop("InitOnceExecuteOnce", "more than 64 INIT_ONCE objects (a limit of this table)");
        addr[n] = once; ctx[n] = 0; done[n] = 0; n++;
    }
    if (!done[i]) {
        r = fle_call_guest(fn, once, param, (uint64_t)(uintptr_t)&local, 0) & 0xffffffffu;
        if (r == 0) { R[0] = 0; return; }
        done[i] = 1; ctx[i] = local;
    }
    if (pctx) fle_st(pctx, 8, ctx[i]);
    R[0] = 1;
}

/* shell32!SHChangeNotify: void f(LONG wEventId, UINT uFlags, LPCVOID dwItem1, LPCVOID dwItem2). Learn: 'Notifies the system of an event that an application has performed'; the system then tells the windows that registered with
 * SHChangeNotifyRegister (fle_ole.c). The environment has no shell and delivers no change notifications (the registrations are recorded and never called), so the call has no effect. S-dec1-shchange. DelZip64.dll calls it for
 * each file it creates (SHCNE_CREATE). */
void fle_imp_SHChangeNotify(void)
{
}

/* kernel32!CompareFileTime: LONG f(const FILETIME *rcx, const FILETIME *rdx). Learn: -1 'First file time is earlier than second', 0 'equal', +1 'later' (the 64-bit values compared as unsigned). */
void fle_imp_CompareFileTime(void)
{
    uint64_t a, b;

    if (R[1] == 0 || R[2] == 0) fle_stop("CompareFileTime", "a FILETIME pointer is NULL: the page gives no behaviour");
    a = fle_ld(R[1], 8);
    b = fle_ld(R[2], 8);
    R[0] = a < b ? 0xFFFFFFFFu : a > b ? 1u : 0u;
}

/* kernel32!GetDiskFreeSpaceW: BOOL f(LPCWSTR lpRootPathName, LPDWORD lpSectorsPerCluster, LPDWORD lpBytesPerSector, LPDWORD lpNumberOfFreeClusters, LPDWORD lpTotalNumberOfClusters). Learn: 'Retrieves information about the specified disk, including
 * the amount of free space'; lpRootPathName NULL = 'the root of the current directory'; the root must end with a backslash. The only drive is the virtual volume C: (fle_vfs.h). The sizes are a choice (S-dec1-diskspace): 8 sectors of 512 B per cluster
 * (4 KiB), 0x1000000 clusters in all (64 GiB), 0xC00000 free (48 GiB), i.e. plenty for the engine's 'is there room to unzip' check. A root other than C:\ (or NULL) stops by name. */
void fle_imp_GetDiskFreeSpaceW(void)
{
    uint64_t p = R[1];

    if (p != 0) {
        uint16_t c0 = (uint16_t)fle_ld(p, 2), c1 = (uint16_t)fle_ld(p + 2, 2), c2 = (uint16_t)fle_ld(p + 4, 2), c3 = (uint16_t)fle_ld(p + 6, 2);
        if (!((c0 == 'C' || c0 == 'c') && c1 == ':' && c2 == '\\' && c3 == 0)) fle_stop("GetDiskFreeSpaceW", "a root path other than NULL and \"C:\\\": the volume has one drive and the page defines no other form here");
    }
    if (R[2]) fle_st(R[2], 4, 8);
    if (R[8]) fle_st(R[8], 4, 512);
    if (R[9]) fle_st(R[9], 4, 0xC00000u);
    if (fle_arg(5)) fle_st(fle_arg(5), 4, 0x1000000u);
    R[0] = 1;
}

/* kernel32!MoveFileWithProgressW: BOOL f(LPCWSTR existing, LPCWSTR new, LPPROGRESS_ROUTINE lpProgressRoutine, LPVOID lpData, DWORD dwFlags [rsp+0x28]). Learn: 'Moves a file or directory ... with a callback'; MOVEFILE_REPLACE_EXISTING (1),
 * MOVEFILE_COPY_ALLOWED (2; a move inside one volume needs no copy), MOVEFILE_WRITE_THROUGH (8). Implemented for lpProgressRoutine == NULL and those flags: with REPLACE_EXISTING an existing destination FILE is deleted first, then MoveFileW. Anything else
 * stops. S-dec1-movefile. */
void fle_imp_MoveFileW(void);
void fle_imp_DeleteFileW(void);
void fle_imp_GetFileAttributesW(void);
void fle_imp_MoveFileWithProgressW(void)
{
    uint32_t flags = (uint32_t)fle_arg(5);
    uint64_t src = R[1], dst = R[2], sp = R[4];

    if (R[8] != 0) fle_stop("MoveFileWithProgressW", "lpProgressRoutine is not NULL: the callback is not modelled");
    if (flags & ~(1u | 2u | 8u)) fle_stop("MoveFileWithProgressW", "dwFlags other than MOVEFILE_REPLACE_EXISTING, COPY_ALLOWED and WRITE_THROUGH");
    if (flags & 1u) {
        uint32_t attr;
        R[1] = dst;
        fle_imp_GetFileAttributesW();
        attr = (uint32_t)R[0];
        if (attr != 0xFFFFFFFFu && !(attr & 0x10u)) {
            R[1] = dst; R[4] = sp;
            fle_imp_DeleteFileW();
            if (R[0] == 0) return;
        }
    }
    R[1] = src; R[2] = dst; R[4] = sp;
    fle_imp_MoveFileW();
}

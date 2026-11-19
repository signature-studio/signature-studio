/* Decision 0183 (wrp1, PROVISIONAL, OPT-IN: linked only into the VST3 build, rows in seeds/wrp1-vst3-imports.txt): the answers that Serum2.vst3 needed from the runtime, PORTED from harness/vst3a/vst3a_imp.c (decision 0165;
 * the S-vst3a-* names and their reasons are there) with three changes: vst3a_note is a no-op, vst3a_stop is fle_stop, and SHGetKnownFolderPath is NOT redefined (the engine's model in fle_filedlg.c answers the same GUIDs,
 * plus ProgramData added by this decision). Convention of fle_imports.h: arguments in R[1], R[2], R[8], R[9] and the stack, the result in R[0], the guest return address stays on the stack. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_env.h"
#include "fle_imports.h"
#include "fle_runtime.h"

#define vst3a_note(x) ((void)(x))
#define vst3a_stop(what, why) fle_stop((what), (why))
#define vst3a_arg(n) fle_arg(n)

/* ---- kernel32 locale: the Ex forms (name instead of LCID) ---- */
static uint32_t lcid_of_name(const char *api, uint64_t pname)
{
    char nm[32];
    size_t i;
    if (pname == 0) return 0; /* LOCALE_NAME_USER_DEFAULT: fle_imp_LCMapStringW takes LCID 0 as the user default for the two case flags (decision 0087) */
    for (i = 0; i + 1 < sizeof nm; i++) {
        uint16_t c = (uint16_t)fle_ld(pname + 2 * i, 2);
        if (!c) break;
        nm[i] = c < 0x80 ? (char)c : '?';
    }
    nm[i] = 0;
    if (!strcmp(nm, "en-US")) return 0x0409u;
    fprintf(stderr, "vst3a: %s: locale name \"%s\" is not modelled\n", api, nm);
    vst3a_stop(api, "locale name other than NULL / en-US");
}

void fle_imp_LCMapStringW(void);
/* kernel32!LCMapStringEx(LPCWSTR name rcx, DWORD flags edx, LPCWSTR src r8, int cchSrc r9d, LPWSTR dst [+0x28], int cchDst [+0x30], LPNLSVERSIONINFO, LPVOID, LPARAM): int.
 * Learn: same as LCMapStringW but the locale is a name. The last three parameters are ignored when NULL/0 (the CRT passes NULL, NULL, 0); a non-NULL one stops. The call is
 * LCMapStringW with the LCID of the name (S-vst3a-locale-ex: NULL = user default = the emulated en-US, "en-US" = 0x0409, any other name stops). */
void fle_imp_LCMapStringEx(void)
{
    uint64_t name = R[1];
    if (vst3a_arg(7) || vst3a_arg(8) || vst3a_arg(9)) vst3a_stop("LCMapStringEx", "lpVersionInformation / lpReserved / sortHandle is not NULL/0");
    R[1] = lcid_of_name("LCMapStringEx", name);
    fle_imp_LCMapStringW();
}

#include "fle_locale.h"

/* kernel32!AreFileApisANSI(): BOOL. Learn: TRUE if the file I/O functions use the ANSI character set, FALSE for OEM. A process starts with ANSI (SetFileApisToOEM is the only way
 * to change it and nothing here calls it): TRUE. (S-vst3a-fileapis-ansi: the runtime has no SetFileApisToOEM row, so the state cannot change.) */
void fle_imp_AreFileApisANSI(void) { vst3a_note("AreFileApisANSI"); R[0] = 1; }

/* kernel32!GetUserDefaultLCID(): LCID. Learn: the user's default locale identifier; here the emulated locale of the runtime (fle_emulated_locale, en-US 0x0409). */
void fle_imp_GetUserDefaultLCID(void) { vst3a_note("GetUserDefaultLCID"); R[0] = fle_emulated_locale.lcid; }

void fle_imp_CompareStringW(void);
/* kernel32!CompareStringEx(name rcx, flags edx, s1 r8, c1 r9d, s2 [+0x28], c2 [+0x30], version [+0x38], reserved [+0x40], lParam [+0x48]): CompareStringW with the LCID of the name (S-vst3a-locale-ex). */
void fle_imp_CompareStringEx(void)
{
    if (vst3a_arg(7) || vst3a_arg(8) || vst3a_arg(9)) vst3a_stop("CompareStringEx", "lpVersionInformation / lpReserved / lParam is not NULL/0");
    R[1] = lcid_of_name("CompareStringEx", R[1]);
    if (R[1] == 0) R[1] = fle_emulated_locale.lcid;
    fle_imp_CompareStringW();
}

void fle_imp_GetLocaleInfoW(void);
/* kernel32!GetLocaleInfoEx(name rcx, LCType edx, buf r8, cch r9d): GetLocaleInfoW with the LCID of the name (S-vst3a-locale-ex). */
void fle_imp_GetLocaleInfoEx(void)
{
    R[1] = lcid_of_name("GetLocaleInfoEx", R[1]);
    if (R[1] == 0) R[1] = fle_emulated_locale.lcid;
    fle_imp_GetLocaleInfoW();
}

/* kernel32!InitOnceBeginInitialize(INIT_ONCE *rcx, DWORD flags edx, BOOL *fPending r8, LPVOID *ctx r9) and InitOnceComplete(INIT_ONCE *rcx, DWORD flags edx, LPVOID ctx r8): BOOL.
 * Learn: a one-time initialisation object (a pointer-sized word, 0 at start). Synchronous use, one thread: state 0 -> Begin sets "pending" (1) and reports fPending TRUE; Complete stores
 * ctx | 2 ("complete"; the context must have its low 2 bits clear, or the page's ERROR_INVALID_PARAMETER) and Begin on a complete object reports fPending FALSE with the context;
 * Complete with INIT_ONCE_INIT_FAILED (4) resets the object to 0. Begin on a pending object would wait for another thread that cannot run: a named stop. INIT_ONCE_ASYNC (2) is not
 * modelled (stop); INIT_ONCE_CHECK_ONLY (1) reports a not-complete object as FALSE with ERROR_GEN_FAILURE (31). S-vst3a-initonce: the whole is a model for one thread. */
void fle_imp_InitOnceBeginInitialize(void)
{
    uint64_t p = R[1], pend = R[8], ctx = R[9];
    uint32_t fl = (uint32_t)R[2];
    uint64_t st = fle_ld(p, 8);
    vst3a_note("InitOnceBeginInitialize");
    if (fl & ~1u) vst3a_stop("InitOnceBeginInitialize", "flags other than 0 / INIT_ONCE_CHECK_ONLY");
    if ((st & 3) == 2) { fle_st(pend, 4, 0); if (ctx) fle_st(ctx, 8, st & ~UINT64_C(3)); R[0] = 1; return; }
    if (fl & 1) { fle_st(pend, 4, 1); R[0] = 0; fle_set_last_error(31); return; }
    if ((st & 3) == 1) vst3a_stop("InitOnceBeginInitialize", "the object is pending (another thread would be initialising it; none can run)");
    fle_st(p, 8, 1);
    fle_st(pend, 4, 1);
    R[0] = 1;
}
void fle_imp_InitOnceComplete(void)
{
    uint64_t p = R[1], ctx = R[8];
    uint32_t fl = (uint32_t)R[2];
    vst3a_note("InitOnceComplete");
    if (fl & ~4u) vst3a_stop("InitOnceComplete", "flags other than 0 / INIT_ONCE_INIT_FAILED");
    if (fle_ld(p, 8) != 1) vst3a_stop("InitOnceComplete", "the object is not pending");
    if (fl & 4) { fle_st(p, 8, 0); R[0] = 1; return; }
    if (ctx & 3) { fle_set_last_error(87); R[0] = 0; return; }
    fle_st(p, 8, ctx | 2);
    R[0] = 1;
}

/* d2d1.dll!#1 (D2D1CreateFactory(D2D1_FACTORY_TYPE factoryType ecx, REFIID riid rdx, const D2D1_FACTORY_OPTIONS *pFactoryOptions r8, void **ppIFactory r9): HRESULT). Serum 2 calls it
 * while InitDll runs (the Direct2D factory of its editor). There is no Direct2D here; the answer is a failure: E_FAIL (0x80004005) with *ppIFactory = NULL (S-vst3a-d2d-fail: the choice of
 * failure code is mine; the page lists no code for "Direct2D absent"; a plugin that cannot take a failure stops where it dereferences the NULL, which is reported). */
void fle_imp_ord_d2d1_1(void)
{
    vst3a_note("D2D1CreateFactory");
    fprintf(stdout, "vst3a: D2D1CreateFactory(type %u) -> E_FAIL, *ppIFactory = NULL\n", (unsigned)R[1]);
    if (R[9]) fle_st(R[9], 8, 0);
    R[0] = 0x80004005u;
}

/* dwrite.dll!DWriteCreateFactory(DWRITE_FACTORY_TYPE factoryType ecx, REFIID iid rdx, IUnknown **factory r8): HRESULT. Serum 2 calls it in InitDll right after D2D1CreateFactory. No DirectWrite
 * here: E_FAIL with *factory = NULL (S-vst3a-d2d-fail, same choice as for D2D1CreateFactory; the page lists no code for "absent"). */
void fle_imp_DWriteCreateFactory(void)
{
    vst3a_note("DWriteCreateFactory");
    fprintf(stdout, "vst3a: DWriteCreateFactory(type %u) -> E_FAIL, *factory = NULL\n", (unsigned)R[1]);
    if (R[8]) fle_st(R[8], 8, 0);
    R[0] = 0x80004005u;
}

/* shell32!SHCreateDirectoryExW(HWND hwnd rcx, LPCWSTR pszPath rdx, const SECURITY_ATTRIBUTES *psa r8): int. Learn: "Creates a new file system folder... creates all intermediate folders"; returns
 * ERROR_SUCCESS (0) on success, ERROR_ALREADY_EXISTS (183) if the final folder already exists, other Win32 error codes otherwise (ERROR_BAD_PATHNAME 161 for a relative path, which Learn says
 * must be fully qualified). hwnd is only for a UI that is not shown; psa must be NULL here (stop otherwise). Every prefix is made with the runtime's CreateDirectoryW on the virtual volume
 * (an existing prefix is not an error). S-vst3a-shcreatedir: the order of the checks and the 161 answer for a non-absolute path are mine. */
void fle_imp_CreateDirectoryW(void);
void fle_imp_GetLastError(void);
void fle_imp_SHCreateDirectoryExW(void)
{
    uint16_t buf[520];
    uint64_t p = R[2];
    size_t n, i;
    uint32_t err = 0;
    vst3a_note("SHCreateDirectoryExW");
    if (R[8] != 0) vst3a_stop("SHCreateDirectoryExW", "psa is not NULL");
    for (n = 0; n < 519; n++) { buf[n] = (uint16_t)fle_ld(p + 2 * n, 2); if (!buf[n]) break; }
    if (n == 519) vst3a_stop("SHCreateDirectoryExW", "path longer than 518 units");
    buf[n] = 0;
    if (n < 3 || buf[1] != ':' || buf[2] != '\\') { R[0] = 161; return; }
    for (i = 3; i <= n; i++) {
        uint16_t save;
        if (buf[i] != '\\' && buf[i] != 0) continue;
        if (i == n - 1 && buf[i] == '\\' && 0) continue;
        save = buf[i]; buf[i] = 0;
        if (i > 3 && buf[i - 1] != '\\') {
            R[1] = (uint64_t)(uintptr_t)buf; R[2] = 0;
            fle_imp_CreateDirectoryW();
            if ((uint32_t)R[0] == 0) {
                fle_imp_GetLastError();
                err = (uint32_t)R[0];
                if (err != 183) { R[0] = err; return; }
                if (save != 0) err = 0;
            } else err = 0;
        }
        buf[i] = save;
        if (save == 0) break;
    }
    R[0] = err;
}

/* kernel32!TryAcquireSRWLockExclusive(PSRWLOCK rcx): BOOLEAN. See the SRW wrappers in vst_env3a.c (S-vst3a-srw-try). */
int vst3a_srw_is_held(uint64_t a);
void fle_imp_AcquireSRWLockExclusive(void);
void fle_imp_TryAcquireSRWLockExclusive(void)
{
    vst3a_note("TryAcquireSRWLockExclusive");
    if (vst3a_srw_is_held(R[1])) { R[0] = 0; return; }
    fle_imp_AcquireSRWLockExclusive();
    R[0] = 1;
}

/* kernel32!WaitForSingleObjectEx(HANDLE rcx, DWORD ms edx, BOOL bAlertable r8d): DWORD. Learn: as WaitForSingleObject, and with bAlertable TRUE the thread also returns WAIT_IO_COMPLETION when an APC runs.
 * No APC is queued by anything here, so bAlertable only has to be accepted: the call is the runtime's WaitForSingleObject (thread-aware: it waits for a thread handle while the other threads run).
 * (S-vst3a-waitex; Serum 2 joins its worker thread at ExitDll.) */
void fle_imp_WaitForSingleObject(void);
void fle_imp_WaitForSingleObjectEx(void)
{
    vst3a_note("WaitForSingleObjectEx");
    fle_imp_WaitForSingleObject();
}

/* kernel32!FreeLibraryAndExitThread(HMODULE rcx, DWORD dwExitCode edx): VOID. Learn: FreeLibrary(hLibModule) then ExitThread(dwExitCode), "does not return". The plugin's worker thread ends this way
 * (it took a reference to its own module with GetModuleHandleExW when it started). That reference is not counted here (S-vst-getmodulehandleex answers without counting), so there is nothing to
 * release: the call is ExitThread(dwExitCode). (S-vst3a-freelib-exit) */
void fle_imp_ExitThread(void);
void fle_imp_FreeLibraryAndExitThread(void)
{
    vst3a_note("FreeLibraryAndExitThread");
    R[1] = R[2];
    fle_imp_ExitThread();
}

/* ---- ported from harness/vst3a/vst_env3a.c (decision 0165) ---- */
int wrp1_vst3_range(uint64_t *base, uint64_t *size);   /* wrp1_vst3.c */

/* S-vst3a-pctofileheader: ntdll!RtlPcToFileHeader(PVOID PcValue rcx, PVOID *BaseOfImage rdx): PVOID. Learn: the base address of the image that contains the PC; NULL (and *BaseOfImage = NULL) if none.
 * The plug-in's own image answers (Serum 2 builds EXCEPTION_RECORDs of pre-constructed std::exception_ptr objects in its static initialisers); any other PC goes to the runtime, which stops by name. */
void __real_fle_imp_RtlPcToFileHeader(void);
void __wrap_fle_imp_RtlPcToFileHeader(void)
{
    uint64_t pc = R[1], pb = R[2], b, s;
    if (wrp1_vst3_range(&b, &s) && pc >= b && pc < b + s) {
        if (pb) fle_st(pb, 8, b);
        R[0] = b;
        return;
    }
    __real_fle_imp_RtlPcToFileHeader();
}

/* S-vst3a-srw-try: TryAcquireSRWLockExclusive must answer FALSE for a held lock (Learn); the runtime stops by name when a held lock is acquired. The four Acquire/Release functions are wrapped to keep a copy of
 * "address -> mode, count"; TryAcquire answers 0 when the copy says held, otherwise it takes the lock through the runtime and answers 1. */
void __real_fle_imp_AcquireSRWLockExclusive(void);
void __real_fle_imp_ReleaseSRWLockExclusive(void);
void __real_fle_imp_AcquireSRWLockShared(void);
void __real_fle_imp_ReleaseSRWLockShared(void);
static struct { uint64_t addr; int ex; unsigned sh; } srw_held[256];
static int srw_n;
static int srw_find(uint64_t a) { for (int i = 0; i < srw_n; i++) if (srw_held[i].addr == a && (srw_held[i].ex || srw_held[i].sh)) return i; return -1; }
static int srw_slot(uint64_t a)
{
    int i = srw_find(a);
    if (i >= 0) return i;
    for (i = 0; i < srw_n; i++) if (!srw_held[i].ex && !srw_held[i].sh) { srw_held[i].addr = a; return i; }
    if (srw_n >= 256) vst3a_stop("SRW", "more than 256 SRW locks held at once");
    srw_held[srw_n].addr = a; srw_held[srw_n].ex = 0; srw_held[srw_n].sh = 0;
    return srw_n++;
}
void __wrap_fle_imp_AcquireSRWLockExclusive(void) { uint64_t a = R[1]; __real_fle_imp_AcquireSRWLockExclusive(); srw_held[srw_slot(a)].ex = 1; }
void __wrap_fle_imp_ReleaseSRWLockExclusive(void) { uint64_t a = R[1]; __real_fle_imp_ReleaseSRWLockExclusive(); { int i = srw_find(a); if (i >= 0) srw_held[i].ex = 0; } }
void __wrap_fle_imp_AcquireSRWLockShared(void) { uint64_t a = R[1]; __real_fle_imp_AcquireSRWLockShared(); srw_held[srw_slot(a)].sh++; }
void __wrap_fle_imp_ReleaseSRWLockShared(void) { uint64_t a = R[1]; __real_fle_imp_ReleaseSRWLockShared(); { int i = srw_find(a); if (i >= 0 && srw_held[i].sh) srw_held[i].sh--; } }
int vst3a_srw_is_held(uint64_t a) { return srw_find(a) >= 0; }

/* ---- LoadLibrary* in the VST3 build (ported from harness/vst3a/vst_env3a.c; S-vst-apiset, S-vst-gfx-absent, S-vst3a-loadlib-path) ----
 * Called from the --wrap of fle_imp_LoadLibraryW / ExW / A (tools/wrp1_wraps.c) with R[1] = the name; kind 0 = W, 1 = ExW, 2 = A. Returns 1 when it answered (R[0], last error set).
 *  - the API-set contract names (api-ms-win-*, ext-ms-win-*) that the static UCRT of the plug-in passes to LoadLibraryExW answer the handle of the virtual DLL that has the same functions
 *    (ntuser -> user32, -security- -> advapi32, everything else -> kernel32): a CHOICE without a source (S-vst-apiset);
 *  - d2d1 / dcomp (the graphics libraries JUCE probes) fail as a missing file, NULL + ERROR_MOD_NOT_FOUND (126): a CHOICE (S-vst-gfx-absent);
 *  - a name WITH a path that the link's path table does not map to a real module names a file the virtual volume does not hold as a module (Serum 2 looks for ...\MTS-ESP\LIBMTS.dll): NULL + 126
 *    (S-vst3a-loadlib-path). A path that the table maps is passed on to the runtime (fle_load.c, which also checks that the file is in the volume). */
#include "fle_modules.h"
#include "fle_realmod.h"
static int ieq_prefix(const uint16_t *u, const char *p)
{
    for (; *p; p++, u++) {
        uint16_t c = *u;
        if (c >= 'A' && c <= 'Z') c = (uint16_t)(c - 'A' + 'a');
        if (c != (uint16_t)(unsigned char)*p) return 0;
    }
    return 1;
}
int wrp1_vst3_loadlib(int kind)
{
    uint16_t u[300];
    size_t n;
    char nm[300];
    const char *dll = NULL;
    uint64_t p = R[1], i;
    if (p == 0) return 0;
    for (n = 0; n < 299; n++) {
        u[n] = kind == 2 ? (uint16_t)(uint8_t)fle_ld(p + n, 1) : (uint16_t)fle_ld(p + 2u * n, 2);
        if (u[n] == 0) break;
    }
    if (n == 299) return 0;
    u[n] = 0;
    for (i = 0; i < n; i++) nm[i] = u[i] < 0x80 ? (u[i] >= 'A' && u[i] <= 'Z' ? (char)(u[i] + 32) : (char)u[i]) : '?';
    nm[n] = 0;
    if (kind != 2 && (ieq_prefix(u, "api-ms-win-") || ieq_prefix(u, "ext-ms-win-"))) {
        dll = strstr(nm, "ntuser") ? "user32.dll" : strstr(nm, "-security-") ? "advapi32.dll" : "kernel32.dll";
        for (i = 0; i < fle_module_count(); i++)
            if (strcmp(fle_module_name(i), dll) == 0) { R[0] = fle_module_handle(i); return 1; }
        return 0;
    }
    if (strchr(nm, '\\') || strchr(nm, '/') || strchr(nm, ':')) {
        if (kind != 2 && fle_realmod_find_path(u, n) >= 0) return 0;
        fprintf(stdout, "wrp1: LoadLibrary of a path that no real module serves: \"%s\" -> NULL, ERROR_MOD_NOT_FOUND\n", nm);
        fle_set_last_error(FLE_ERROR_MOD_NOT_FOUND);
        R[0] = 0;
        return 1;
    }
    if (!strcmp(nm, "d2d1.dll") || !strcmp(nm, "d2d1") || !strcmp(nm, "dcomp.dll") || !strcmp(nm, "dcomp")) {
        fle_set_last_error(FLE_ERROR_MOD_NOT_FOUND);
        R[0] = 0;
        return 1;
    }
    return 0;
}

/* ---- more environment answers ported from harness/vst3a/vst_env3a.c (decisions 0156, 0165) ---- */
#include "fle_class.h"
#include "fle_vfs.h"

/* S-vst-getmodulehandleex: GetModuleHandleExW(dwFlags ecx, lpModuleName rdx, phModule r8). FROM_ADDRESS (4) with an address inside the plug-in's image -> its base; a name -> what GetModuleHandleW answers; any other address fails with 126. */
void fle_imp_GetModuleHandleW(void);
void __wrap_fle_imp_GetModuleHandleExW(void)
{
    uint32_t flags = (uint32_t)R[1];
    uint64_t name = R[2], ph = R[8], b, s;
    int have = wrp1_vst3_range(&b, &s);
    if (!ph) { fle_set_last_error(87); R[0] = 0; return; }
    if (flags & 4u) {
        if (have && name >= b && name < b + s) { fle_st(ph, 8, b); R[0] = 1; return; }
        fle_st(ph, 8, 0);
        fle_set_last_error(FLE_ERROR_MOD_NOT_FOUND);
        R[0] = 0;
        return;
    }
    R[1] = name;
    fle_imp_GetModuleHandleW();
    fle_st(ph, 8, R[0]);
    R[0] = R[0] ? 1 : 0;
}

/* S-vst-classatom: CreateWindowExW with the class given as an ATOM (MAKEINTATOM) that RegisterClassW/ExW handed out becomes the class's registered name; an unknown atom goes to the runtime unchanged. */
void __real_fle_imp_CreateWindowExW(void);
void __wrap_fle_imp_CreateWindowExW(void)
{
    static uint16_t nm[FLE_CLASS_NAME_MAX_UNITS + 2];
    uint64_t c = R[2];
    uint16_t len = 0;
    if (c != 0 && c < 0x10000u && fle_class_name_of((uint16_t)c, nm, &len)) {
        nm[len] = 0;
        R[2] = (uint64_t)(uintptr_t)nm;
    }
    __real_fle_imp_CreateWindowExW();
}

/* S-vst3a-createfile-empty and S-vst3a-dirwatch-fail (called from the CreateFileW wrap of tools/wrp1_wraps.c; 1 = answered): an empty name (Serum 2 opens "" with BACKUP_SEMANTICS|OVERLAPPED: a directory watcher of
 * a content folder it could not find) -> INVALID_HANDLE_VALUE + ERROR_PATH_NOT_FOUND (3: my recollection; Learn's CreateFile page lists no code for it); a directory opened for change notification
 * (BACKUP_SEMANTICS 0x02000000 | OVERLAPPED 0x40000000) -> INVALID_HANDLE_VALUE + ERROR_ACCESS_DENIED (5: my choice; the virtual volume has no directory handles). */
int wrp1_vst3_createfile(void)
{
    /* S-wrp1-sa-default (S-vst-secattr of decision 0156): a SECURITY_ATTRIBUTES with nLength 24 and a NULL lpSecurityDescriptor is the default security descriptor (Learn, SECURITY_ATTRIBUTES: "If this member is NULL, the object gets a
     * default security descriptor"); bInheritHandle only matters to child processes, of which there are none. The call goes on as with a NULL pointer; a real descriptor (or another size) still stops in the runtime. */
    if (R[9] != 0 && (uint32_t)fle_ld(R[9], 4) == 24u && fle_ld(R[9] + 8, 8) == 0) R[9] = 0;
    if (R[1] && fle_ld(R[1], 2) == 0) {
        printf("wrp1: CreateFileW(\"\", access %#x, share %#x, flags %#x) -> INVALID_HANDLE_VALUE, ERROR_PATH_NOT_FOUND\n", (unsigned)R[2], (unsigned)R[8], (unsigned)fle_arg(6));
        fle_set_last_error(3);
        R[0] = UINT64_C(0xffffffffffffffff);
        return 1;
    }
    if ((fle_arg(6) & 0x42000000u) == 0x42000000u) {
        char nm[200];
        size_t i;
        for (i = 0; i + 1 < sizeof nm; i++) { uint16_t c = (uint16_t)fle_ld(R[1] + 2 * i, 2); if (!c) break; nm[i] = c < 0x80 ? (char)c : '?'; }
        nm[i] = 0;
        printf("wrp1: CreateFileW(\"%s\", flags %#x: directory change notification) -> INVALID_HANDLE_VALUE, ERROR_ACCESS_DENIED\n", nm, (unsigned)fle_arg(6));
        fle_set_last_error(5);
        R[0] = UINT64_C(0xffffffffffffffff);
        return 1;
    }
    return 0;
}

/* S-vst-filetype: GetFileType(h): a handle that fle_vfs.c issued for an open virtual file is a disk file (FILE_TYPE_DISK 1; Learn); any other value goes to the runtime. */
void __real_fle_imp_GetFileType(void);
void __wrap_fle_imp_GetFileType(void)
{
    const uint8_t *d;
    uint64_t sz;
    unsigned r, w;
    uint64_t h = R[1];
    if (h != 0 && h != UINT64_MAX && fle_vfs_file_info("GetFileType", h, &d, &sz, &r, &w)) { R[0] = 1; return; }
    __real_fle_imp_GetFileType();
}

/* S-vst-setfilepointerex: SetFilePointerEx(h, liDistanceToMove, lpNewFilePointer, dwMoveMethod) = SetFilePointer with the distance split in two (Learn: nonzero on success). */
void fle_imp_SetFilePointer(void);
void fle_imp_GetLastError(void);
void __wrap_fle_imp_SetFilePointerEx(void)
{
    uint64_t h = R[1], newp = R[8];
    int64_t dist = (int64_t)R[2];
    uint32_t method = (uint32_t)R[9], hi = (uint32_t)((uint64_t)dist >> 32), lo, err;
    R[1] = h;
    R[2] = (uint32_t)dist;
    R[8] = (uint64_t)(uintptr_t)&hi;
    R[9] = method;
    fle_imp_SetFilePointer();
    lo = (uint32_t)R[0];
    fle_imp_GetLastError();
    err = (uint32_t)R[0];
    if (lo == 0xFFFFFFFFu && err != 0) { R[0] = 0; return; }
    if (newp) fle_st(newp, 8, ((uint64_t)hi << 32) | lo);
    R[0] = 1;
}

/* S-wrp1-csidl-personal: SHGetSpecialFolderLocation / SHGetPathFromIDListW for the CSIDLs that SHGetFolderPathW (fle_env.c) already answers (0x05 PERSONAL asked for by the wrapper after Serum 2 hashed the embedded data of the state in r13; 0x23 COMMON_APPDATA right after in r14; the others added together). Learn (CSIDL): "CSIDL_PERSONAL: The virtual folder that represents
 * the My Documents desktop item ... A typical path is C:\Documents and Settings\username\My Documents" (post-Vista: C:\Users\username\Documents); the path is the one fle_env.c gives SHGetFolderPathW for the same CSIDL
 * (C:\Users\user + \Documents; decisions 0103/0107/0126). Used by fle_ole.c through its weak hook fle_special_folder_extra. */
#include "fle_env.h"
const char *fle_special_folder_extra(uint32_t csidl)
{
    switch (csidl) {
    case 0x05u: return FLE_VIRTUAL_USER_PROFILE_DIR "\\Documents";             /* CSIDL_PERSONAL */
    case 0x1cu: return FLE_VIRTUAL_USER_PROFILE_DIR "\\AppData\\Local";       /* CSIDL_LOCAL_APPDATA */
    case 0x23u: return "C:\\ProgramData";                                    /* CSIDL_COMMON_APPDATA (r14: asked for after PERSONAL) */
    case 0x24u: return FLE_VIRTUAL_WINDOWS_DIR;                              /* CSIDL_WINDOWS */
    case 0x26u: case 0x2au: return "C:\\Program Files";                       /* CSIDL_PROGRAM_FILES, _FILESX86 (the choice of fle_env.c, S-programfiles) */
    case 0x28u: return FLE_VIRTUAL_USER_PROFILE_DIR;                         /* CSIDL_PROFILE */
    case 0x2bu: case 0x2cu: return "C:\\Program Files\\Common Files";        /* CSIDL_PROGRAM_FILES_COMMON, _COMMONX86 */
    default: return NULL;
    }
}

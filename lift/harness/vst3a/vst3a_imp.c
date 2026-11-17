/* Answers of the vst3a harness for the imports of Serum2.vst3 that the runtime's import table lacks (decision 0165, PROVISIONAL). Reached through rows of
 * imports-vst3a-rows.txt (mk_stubs.py): fe-lift --extra-imports. Convention of fle_imports.h: arguments in R[1], R[2], R[8], R[9] and the stack, the result in R[0],
 * the guest return address stays on the stack. A function that is not defined here is a weak stopping stub (vst3a_stubs.c). */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_env.h"
#include "fle_imports.h"
#include "fle_runtime.h"
#include "vst3a.h"

static unsigned long long g_notes_n;
void vst3a_note(const char *api) { (void)api; g_notes_n++; }

void vst3a_unanswered(const char *dll, const char *api)
{
    fflush(stdout);
    fprintf(stderr, "vst3a: UNANSWERED import %s!%s called (rcx=%#llx rdx=%#llx r8=%#llx r9=%#llx, guest return address %#llx)\n", dll, api,
            (unsigned long long)R[1], (unsigned long long)R[2], (unsigned long long)R[8], (unsigned long long)R[9], (unsigned long long)fle_ld(R[4], 8));
    exit(97);
}

void vst3a_stop(const char *what, const char *why)
{
    fflush(stdout);
    fprintf(stderr, "vst3a: STOP %s: %s\n", what, why);
    exit(98);
}

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

/* ---- shell32!SHGetKnownFolderPath(REFKNOWNFOLDERID rfid rcx, DWORD dwFlags edx, HANDLE hToken r8, PWSTR *ppszPath r9): HRESULT ----
 * Learn: "Retrieves the full path of a known folder identified by the folder's KNOWNFOLDERID"; the caller frees *ppszPath with CoTaskMemFree. The GUIDs and their CSIDL equivalents are those of
 * the Learn page KNOWNFOLDERID (fetched 2026-11-03): RoamingAppData {3EB685DB-65F9-4CF6-A03A-E3EF65729F3D} = CSIDL_APPDATA, LocalAppData {F1B32785-6FBA-4FCF-9D55-7B8E7F157091} = CSIDL_LOCAL_APPDATA,
 * Documents {FDD39AD0-238F-46AF-ADB4-6C85480369C7} = CSIDL_PERSONAL, ProgramData {62AB5D82-FDC1-4DC3-A9DD-070D1D495D97} = CSIDL_COMMON_APPDATA, Profile {5E6C858F-0E22-4760-9AFE-EA3317B67173} =
 * CSIDL_PROFILE, ProgramFilesCommon {F7F1ED05-9F6D-47A2-AAAE-29D317C6F066} = CSIDL_PROGRAM_FILES_COMMON. The path is that of fle_imp_SHGetFolderPathW (the virtual volume's profile folders, decisions 0103/0107/0126), so the two APIs agree. Another folder id stops by name with its
 * GUID; hToken must be 0; dwFlags KF_FLAG_CREATE (0x8000) is accepted and creates nothing (the folders exist in the volume's seed), any other flag stops (S-vst3a-known-folder). */
void fle_imp_SHGetFolderPathW(void);
void fle_imp_CoTaskMemAlloc(void);
void fle_imp_SHGetKnownFolderPath(void)
{
    static const struct { uint8_t g[16]; int csidl; } kf[] = {
        { { 0xDB, 0x85, 0xB6, 0x3E, 0xF9, 0x65, 0xF6, 0x4C, 0xA0, 0x3A, 0xE3, 0xEF, 0x65, 0x72, 0x9F, 0x3D }, 0x1a },
        { { 0x85, 0x27, 0xB3, 0xF1, 0xBA, 0x6F, 0xCF, 0x4F, 0x9D, 0x55, 0x7B, 0x8E, 0x7F, 0x15, 0x70, 0x91 }, 0x1c },
        { { 0xD0, 0x9A, 0xD3, 0xFD, 0x8F, 0x23, 0xAF, 0x46, 0xAD, 0xB4, 0x6C, 0x85, 0x48, 0x03, 0x69, 0xC7 }, 0x05 },
        { { 0x82, 0x5D, 0xAB, 0x62, 0xC1, 0xFD, 0xC3, 0x4D, 0xA9, 0xDD, 0x07, 0x0D, 0x1D, 0x49, 0x5D, 0x97 }, 0x23 },
        { { 0x05, 0xED, 0xF1, 0xF7, 0x6D, 0x9F, 0xA2, 0x47, 0xAA, 0xAE, 0x29, 0xD3, 0x17, 0xC6, 0xF0, 0x66 }, 0x2b },
        { { 0x8F, 0x85, 0x6C, 0x5E, 0x22, 0x0E, 0x60, 0x47, 0x9A, 0xFE, 0xEA, 0x33, 0x17, 0xB6, 0x71, 0x73 }, 0x28 },
    };
    uint64_t pg = R[1], out = R[9];
    uint32_t fl = (uint32_t)R[2];
    uint8_t g[16];
    uint16_t buf[300];
    uint64_t saved, blk;
    size_t n;
    int csidl = -1;
    vst3a_note("SHGetKnownFolderPath");
    for (int i = 0; i < 16; i++) g[i] = (uint8_t)fle_ld(pg + i, 1);
    for (size_t k = 0; k < sizeof kf / sizeof kf[0]; k++) if (!memcmp(g, kf[k].g, 16)) csidl = kf[k].csidl;
    if (csidl < 0) {
        char s[64];
        for (int i = 0; i < 16; i++) snprintf(s + 2 * i, 3, "%02X", g[i]);
        fprintf(stderr, "vst3a: SHGetKnownFolderPath: FOLDERID bytes %s\n", s);
        vst3a_stop("SHGetKnownFolderPath", "a folder id that is not modelled");
    }
    if (R[8] != 0) vst3a_stop("SHGetKnownFolderPath", "hToken is not NULL");
    if (fl & ~0x8000u) vst3a_stop("SHGetKnownFolderPath", "dwFlags other than 0 / KF_FLAG_CREATE");
    saved = fle_ld(R[4] + 0x28u, 8);
    fle_st(R[4] + 0x28u, 8, (uint64_t)(uintptr_t)buf);
    R[1] = 0; R[2] = (uint64_t)csidl; R[8] = 0; R[9] = 0;
    fle_imp_SHGetFolderPathW();
    fle_st(R[4] + 0x28u, 8, saved);
    if ((uint32_t)R[0] != 0) { fle_st(out, 8, 0); R[0] = 0x80004005u; return; }
    for (n = 0; buf[n]; n++) {}
    R[1] = 2 * (n + 1);
    fle_imp_CoTaskMemAlloc();
    blk = R[0];
    if (!blk) { fle_st(out, 8, 0); R[0] = 0x8007000Eu; return; }
    memcpy((void *)(uintptr_t)blk, buf, 2 * (n + 1));
    fle_st(out, 8, blk);
    R[0] = 0;
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

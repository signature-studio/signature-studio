/* Answers of the vst3a harness for the imports of Serum2.vst3 that the runtime's import table lacks (decision 0165, PROVISIONAL). #1367: the locale Ex forms, GetUserDefaultLCID
 * and SHGetKnownFolderPath are answered by the runtime itself now (fle_wall_dec1.c, fle_filedlg.c), so they are not defined here (a second definition does not link). Reached through rows of
 * lift/harness/vst/imports-vst.txt (the rows; fe-lift --extra-imports, gen_vst.sh). Convention of fle_imports.h: arguments in R[1], R[2], R[8], R[9] and the stack, the result in R[0],
 * the guest return address stays on the stack. A function that is not defined here is a weak stopping stub (vst3a_stubs.c). */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fle_env.h"
#include "fle_imports.h"
#include "fle_runtime.h"
#include "vst3a.h"
#include "boot.h"

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

#include "fle_locale.h"

/* ole32!CoCreateGuid(GUID *pguid rcx): HRESULT. Learn: "creates a GUID, a globally unique identifier" (S_OK; E_POINTER for NULL). No random source is modelled: the
 * GUIDs are numbered in the order of the calls, so that a run gives the same identifiers every time (#1367). */
void fle_imp_CoCreateGuid(void)
{
    static uint32_t n;
    uint64_t p = R[1];
    vst3a_note("CoCreateGuid");
    if (!p) { R[0] = 0x80004003u; return; }
    n++;
    for (int i = 0; i < 16; i++) fle_st(p + (uint64_t)i, 1, ((n >> ((i & 3) * 8)) ^ (uint32_t)i) & 0xffu);
    fle_st(p + 7, 1, 0x40u);
    fle_st(p + 8, 1, 0x80u);
    R[0] = 0;
}

void fle_imp_RegisterClassW(void);
/* user32!RegisterClassExW(const WNDCLASSEXW *lpwcx rcx): ATOM. Learn: as RegisterClassW, with cbSize and hIconSm in front of and behind the WNDCLASSW fields. The copy
 * of the fields (style at 4 and the pointers from 8 on, 64 bytes) goes to a WNDCLASSW in host memory, and RegisterClassW (runtime/fle_class.c) answers (#1367). */
void fle_imp_RegisterClassExW(void)
{
    static uint8_t wc[72];
    const uint8_t *src = (const uint8_t *)(uintptr_t)R[1];
    vst3a_note("RegisterClassExW");
    if (!src) { R[0] = 0; fle_set_last_error(87); return; }
    memset(wc, 0, sizeof wc);
    memcpy(wc, src + 4, 4);
    memcpy(wc + 8, src + 8, 64);
    R[1] = (uint64_t)(uintptr_t)wc;
    fle_imp_RegisterClassW();
}

void fle_imp_SHGetFolderPathW(void);
/* shell32!SHGetSpecialFolderPathW(HWND hwnd rcx, LPWSTR pszPath rdx, int csidl r8d, BOOL fCreate r9d): BOOL. Learn: the path of a special folder, "as
 * SHGetFolderPath". Answered by SHGetFolderPathW (its fifth argument, pszPath, goes on the stack: the call is made on a frame of its own, as main_vst3a.c seeds the
 * registry). hToken NULL and SHGFP_TYPE_CURRENT (0). fCreate is not used by the runtime's folders (they exist in the volume's seed). BOOL TRUE when the HRESULT is S_OK (#1367). */
void fle_imp_SHGetSpecialFolderPathW(void)
{
    static uint64_t stk[16];
    uint64_t save4 = R[4], hwnd = R[1], path = R[2], csidl = R[8];
    uint64_t hr;
    vst3a_note("SHGetSpecialFolderPathW");
    memset(stk, 0, sizeof stk);
    stk[0] = HARNESS_SENTINEL_RET;
    stk[5] = path;
    R[1] = hwnd; R[2] = csidl; R[8] = 0; R[9] = 0;
    R[4] = (uint64_t)(uintptr_t)stk;
    fle_imp_SHGetFolderPathW();
    hr = R[0] & 0xffffffffu;
    R[4] = save4;
    R[0] = hr == 0 ? 1 : 0;
}

void fle_imp_FindResourceW(void);
/* kernel32!FindResourceA(HMODULE rcx, LPCSTR lpName rdx, LPCSTR lpType r8): HRSRC. Learn: as FindResourceW with ANSI strings; an integer name or type (below 0x10000,
 * MAKEINTRESOURCE) is passed as it is. The strings are converted to UTF-16 (the code page of the names is the one of the ASCII names the plugin uses) and
 * FindResourceW (runtime/fle_res.c) answers (#1367). */
static uint64_t ansi_to_w_arg(uint64_t p, uint16_t *buf, size_t cap)
{
    size_t i;
    if (p < 0x10000u) return p;
    if (!p) return 0;
    for (i = 0; i + 1 < cap; i++) {
        uint8_t c = (uint8_t)fle_ld(p + i, 1);
        if (!c) break;
        buf[i] = c;
    }
    buf[i] = 0;
    return (uint64_t)(uintptr_t)buf;
}
void fle_imp_FindResourceA(void)
{
    static uint16_t wname[256], wtype[256];
    vst3a_note("FindResourceA");
    R[2] = ansi_to_w_arg(R[2], wname, sizeof wname / sizeof wname[0]);
    R[8] = ansi_to_w_arg(R[8], wtype, sizeof wtype / sizeof wtype[0]);
    fle_imp_FindResourceW();
}

/* kernel32!InitializeSRWLock(PSRWLOCK rcx): VOID, and InitializeConditionVariable(PCONDITION_VARIABLE rcx): VOID. Learn: both initialise the object to its unlocked /
 * empty state, which is the zero word (the wrappers in vst_env3a.c keep the held state of a lock by address, and the waits of a condition variable are the ones there) (#1367). */
void fle_imp_InitializeSRWLock(void)
{
    vst3a_note("InitializeSRWLock");
    if (R[1]) fle_st(R[1], 8, 0);
}
void fle_imp_InitializeConditionVariable(void)
{
    vst3a_note("InitializeConditionVariable");
    if (R[1]) fle_st(R[1], 8, 0);
}

/* gdiplus!GdiplusStartup(ULONG_PTR *token rcx, const GdiplusStartupInput *input rdx, GdiplusStartupOutput *output r8): Status. Learn: "initializes GDI+"; Ok (0) and a
 * token (non-zero). The plugin asks for it in its start-up (the resources of its UI); no drawing is modelled, so a later GDI+ drawing call stops by name (#1367). */
void fle_imp_GdiplusStartup(void)
{
    vst3a_note("GdiplusStartup");
    if (!R[1]) { R[0] = 1; return; }
    fle_st(R[1], 8, 1);
    if (R[8]) fle_st(R[8], 8, 0);
    R[0] = 0;
}
/* gdiplus!GdiplusShutdown(ULONG_PTR token rcx): VOID. Learn: "cleans up GDI+ resources" (no resources are kept here). */
void fle_imp_GdiplusShutdown(void) { vst3a_note("GdiplusShutdown"); }

/* GDI+ objects are opaque handles here: a creation answers a new handle (a counter, never a real object), a disposal answers Ok. No drawing is modelled: a drawing call
 * (GdipDrawString, GdipFillRectangle, ...) is not answered and stops by name. gdiplus!GdipCreateBitmapFromScan0(width rcx, height rdx, stride r8, format r9,
 * scan0 [+0x28], GpBitmap **bitmap [+0x30]): Status. The pixels of the bitmap are not kept (scan0 is NULL in the plugin's start-up). (#1367) */
static uint64_t gdip_next = 0x10000;
/* a bitmap made by GdipCreateBitmapFromScan0 with a NULL scan0: 32 bpp, zeroed pixels, kept here by its handle (at most 16 bitmaps) */
static struct { uint64_t h; uint32_t w, hgt; uint8_t *px; } gdip_bm[16];
static int gdip_nbm;
static void gdip_new_handle(uint64_t out)
{
    gdip_next += 0x10;
    if (out) fle_st(out, 8, gdip_next);
}
void fle_imp_GdipCreateBitmapFromScan0(void)
{
    uint32_t w = (uint32_t)R[1], h = (uint32_t)R[2];
    vst3a_note("GdipCreateBitmapFromScan0");
    if (!vst3a_arg(6)) { R[0] = 2; return; }
    if (vst3a_arg(5) || gdip_nbm >= 16 || !w || !h) vst3a_stop("GdipCreateBitmapFromScan0", "a caller's scan0 buffer, or more than 16 bitmaps, is not modelled");
    gdip_new_handle(vst3a_arg(6));
    gdip_bm[gdip_nbm].h = gdip_next; gdip_bm[gdip_nbm].w = w; gdip_bm[gdip_nbm].hgt = h;
    gdip_bm[gdip_nbm].px = calloc((size_t)w * h, 4);
    gdip_nbm++;
    R[0] = 0;
}
/* GdipBitmapLockBits(GpBitmap *bitmap rcx, const GpRect *rect rdx, UINT flags r8, PixelFormat format r9, BitmapData *lockedBitmapData [+0x28]): Status.
 * The whole bitmap (the rect is not modelled: NULL or the whole bitmap). BitmapData = { Width, Height, Stride, PixelFormat, Scan0, Reserved } (#1367) */
void fle_imp_GdipBitmapLockBits(void)
{
    uint64_t bm = R[1], bd = vst3a_arg(5);
    int i;
    vst3a_note("GdipBitmapLockBits");
    for (i = 0; i < gdip_nbm; i++) if (gdip_bm[i].h == bm) break;
    if (i == gdip_nbm || !bd) { R[0] = 2; return; }
    fle_st(bd, 4, gdip_bm[i].w);
    fle_st(bd + 4, 4, gdip_bm[i].hgt);
    fle_st(bd + 8, 4, gdip_bm[i].w * 4u);
    fle_st(bd + 12, 4, R[9]);
    fle_st(bd + 16, 8, (uint64_t)(uintptr_t)gdip_bm[i].px);
    fle_st(bd + 24, 8, 0);
    R[0] = 0;
}
void fle_imp_GdipBitmapUnlockBits(void) { vst3a_note("GdipBitmapUnlockBits"); R[0] = 0; }

void fle_imp_GdipGetImageGraphicsContext(void)
{
    vst3a_note("GdipGetImageGraphicsContext");
    if (!R[9]) { R[0] = 2; return; }
    gdip_new_handle(R[9]);
    R[0] = 0;
}
void fle_imp_GdipDeleteGraphics(void) { vst3a_note("GdipDeleteGraphics"); R[0] = 0; }
void fle_imp_GdipDisposeImage(void) { vst3a_note("GdipDisposeImage"); R[0] = 0; }

/* gdi32!AddFontResourceExW(LPCWSTR name rcx, DWORD fl rdx, PVOID res r8): int. Learn: "the number of fonts added"; a font file is not loaded in this environment, so 0
 * fonts are added and the last error is ERROR_NOT_SUPPORTED (50): the caller then uses the fonts it has (#1367). */
void fle_imp_AddFontResourceExW(void)
{
    vst3a_note("AddFontResourceExW");
    R[0] = 0;
    fle_set_last_error(50);
}
/* kernel32!AreFileApisANSI(): BOOL. Learn: TRUE if the file I/O functions use the ANSI character set, FALSE for OEM. A process starts with ANSI (SetFileApisToOEM is the only way
 * to change it and nothing here calls it): TRUE. (S-vst3a-fileapis-ansi: the runtime has no SetFileApisToOEM row, so the state cannot change.) */
void fle_imp_AreFileApisANSI(void) { vst3a_note("AreFileApisANSI"); R[0] = 1; }


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
 * while InitDll runs (the Direct2D factory of its editor). There is no Direct2D here: the answer is the failure of a machine without a hardware device, D2DERR_NO_HARDWARE_DEVICE
 * (0x8899000B, d2derr.h), with *ppIFactory = NULL. The row is in lift/harness/vst/ordinals-vst.txt (#1336). No other ordinal is answered. */
void fle_imp_ord_d2d1_1(void)
{
    vst3a_note("D2D1CreateFactory");
    fprintf(stdout, "vst3a: D2D1CreateFactory(type %u) -> D2DERR_NO_HARDWARE_DEVICE, *ppIFactory = NULL\n", (unsigned)R[1]);
    if (R[9]) fle_st(R[9], 8, 0);
    R[0] = 0x8899000Bu;
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

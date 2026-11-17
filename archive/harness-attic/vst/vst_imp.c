/* Import answers that a third-party VST3 plugin needed and that the runtime did not have (decision 0156, PROVISIONAL). They are reached only through rows that
 * fe-lift reads from `--extra-imports` (lift/harness/vst/imports-vst.txt); no row of the import table and no generated C of another module refers to them.
 * Same convention as fle_imports.h: arguments in R[1], R[2], R[8], R[9] and the stack, the result in R[0], the guest return address stays on the stack.
 * Every answer says where its value comes from; a value without a source is listed as S-vst-... in docs/decisions/0156-first-vst3-plugin.md. */
#include <stdint.h>
#include <stdio.h>

#include "fle_env.h"
#include "fle_imports.h"
#include "fle_runtime.h"

void fle_imp_InitializeCriticalSectionAndSpinCount(void);

/* kernel32!InitializeCriticalSectionEx(LPCRITICAL_SECTION rcx, DWORD dwSpinCount edx, DWORD Flags r8d): BOOL. Learn: "This function always succeeds and returns a
 * nonzero value" (Windows Vista and later). The object is the one of InitializeCriticalSectionAndSpinCount (same table entry, same stop for a section that is
 * already initialized); Flags (CRITICAL_SECTION_NO_DEBUG_INFO 0x01000000, RTL_CRITICAL_SECTION_FLAG_DYNAMIC_SPIN, STATIC_INIT) change nothing that this runtime keeps. */
void fle_imp_InitializeCriticalSectionEx(void)
{
    fle_imp_InitializeCriticalSectionAndSpinCount();
    R[0] = 1;
}

void fle_imp_UuidCreate(void);

/* ole32!CoCreateGuid(GUID *pguid rcx): HRESULT. Learn: "Creates a GUID, a unique 128-bit integer"; S_OK on success. Same value source as rpcrt4!UuidCreate of this runtime
 * (fle_proc.c: the deterministic stream of decision 0118, version 4 / variant 10 bits), so two runs give the same GUIDs; not a Windows rule (S-vst-guid). */
void fle_imp_CoCreateGuid(void)
{
    fle_imp_UuidCreate();
    R[0] = 0;
}

void fle_imp_RegisterClassW(void);

/* user32!RegisterClassExW(const WNDCLASSEXW *rcx): ATOM. The plugin registers a window class (JUCE: "JUCEWindow", the hidden window of its message queue) when its component is
 * created. WNDCLASSEXW (Learn: cbSize 0, style 4, lpfnWndProc 8, cbClsExtra 16, cbWndExtra 20, hInstance 24, hIcon 32, hCursor 40, hbrBackground 48, lpszMenuName 56,
 * lpszClassName 64, hIconSm 72) differs from the WNDCLASSW that RegisterClassW (fle_class.c) takes (style 0, then the same fields from offset 8) only in cbSize/style and the
 * trailing hIconSm, so the call is RegisterClassW on a WNDCLASSW built from it: the class goes into the one class table that CreateWindowExW reads, with the atom, the
 * duplicate and the hInstance rules of that file. A cbSize other than 80 stops (the page: cbSize "must be set to sizeof(WNDCLASSEX)"). */
void fle_imp_RegisterClassExW(void)
{
    static const char api[] = "RegisterClassExW";
    static uint64_t wc[9];
    uint64_t p = R[1];
    unsigned i;
    if (p == 0) fle_stop(api, "lpwcx is NULL (the page gives no value for the failure)");
    if ((uint32_t)fle_ld(p, 4) != 80u) fle_stop(api, "cbSize is not sizeof(WNDCLASSEXW) = 80");
    memset(wc, 0, sizeof wc);
    wc[0] = (uint32_t)fle_ld(p + 4, 4);
    for (i = 1; i < 9; i++) wc[i] = fle_ld(p + 8u * i, 8);
    R[1] = (uint64_t)(uintptr_t)wc;
    fle_imp_RegisterClassW();
}

void fle_imp_SHGetFolderPathW(void);

/* shell32!SHGetSpecialFolderPathW(HWND hwnd rcx, LPWSTR pszPath rdx, int csidl r8d, BOOL fCreate r9d): BOOL. Learn (SHGetSpecialFolderPath): "Retrieves the path of a special
 * folder, identified by its CSIDL"; TRUE on success. fCreate only asks to create a missing folder; the virtual volume has no profile folders to create, so it is ignored (a
 * choice: the same as SHGetFolderPathW, which never creates). The values are those of fle_imp_SHGetFolderPathW (fle_env.c, decisions 0103/0107/0126): the arguments are
 * rearranged to that function's (csidl in rdx, token NULL, SHGFP_TYPE_CURRENT, the path as the fifth argument at [rsp+0x28], which belongs to the caller's frame: its value is
 * put back after the call). A csidl that function does not answer stops by name there. */
void fle_imp_SHGetSpecialFolderPathW(void)
{
    uint64_t path = R[2], saved;
    uint32_t csidl = (uint32_t)R[8];
    saved = fle_ld(R[4] + 0x28u, 8);
    fle_st(R[4] + 0x28u, 8, path);
    R[2] = csidl;
    R[8] = 0;
    R[9] = 0;
    fle_imp_SHGetFolderPathW();
    fle_st(R[4] + 0x28u, 8, saved);
    R[0] = ((uint32_t)R[0] == 0) ? 1 : 0;
}

void fle_imp_FindResourceW(void);

/* kernel32!FindResourceA(HMODULE hModule rcx, LPCSTR lpName rdx, LPCSTR lpType r8): HRSRC. Learn: the ANSI form of FindResourceW; a name or type is "either a pointer to a string or an
 * integer resource identifier" (MAKEINTRESOURCE: a value below 0x10000). A string is widened byte for byte to UTF-16 (ASCII only; a byte above 0x7F stops: the code page mapping is
 * not defined here, as in GetModuleHandleA) and the call is FindResourceW (fle_res.c), whose answers and stops it keeps. */
static uint16_t *vst_widen(const char *api, uint64_t p, uint16_t *buf, size_t cap)
{
    size_t i;
    if (p < 0x10000u) return (uint16_t *)(uintptr_t)p;
    for (i = 0; i + 1 < cap; i++) {
        uint8_t c = (uint8_t)fle_ld(p + i, 1);
        if (c > 0x7f) fle_stop(api, "a byte above 0x7F in a resource name or type: the ANSI code page is not modelled");
        buf[i] = c;
        if (!c) return buf;
    }
    fle_stop(api, "a resource name or type longer than 255 characters");
}

void fle_imp_FindResourceA(void)
{
    uint16_t nb[256], tb[256];
    uint64_t pn = R[2], pt = R[8];
    R[2] = (uint64_t)(uintptr_t)vst_widen("FindResourceA", pn, nb, 256);
    R[8] = (uint64_t)(uintptr_t)vst_widen("FindResourceA", pt, tb, 256);
    fle_imp_FindResourceW();
}

/* kernel32!InitializeSRWLock(PSRWLOCK rcx) and InitializeConditionVariable(PCONDITION_VARIABLE rcx): Learn: both set the object to its initial value, which is the all-zero
 * SRWLOCK_INIT / CONDITION_VARIABLE_INIT (one pointer-sized member, Ptr = NULL). fle_sys.c keeps the state of a lock outside the object and only while it is held / waited on
 * (a lock it has no entry for is free), so the initial value is the only thing to write. */
void fle_imp_InitializeSRWLock(void)
{
    if (R[1] == 0) fle_stop("InitializeSRWLock", "the lock pointer is NULL (not defined)");
    fle_st(R[1], 8, 0);
}

void fle_imp_InitializeConditionVariable(void)
{
    if (R[1] == 0) fle_stop("InitializeConditionVariable", "the pointer is NULL (not defined)");
    fle_st(R[1], 8, 0);
}

void fle_imp_EnterCriticalSection(void);

/* kernel32!TryEnterCriticalSection(LPCRITICAL_SECTION rcx): BOOL. Learn: "If the critical section is successfully entered or the current thread already owns the critical
 * section, the return value is nonzero. If another thread already owns the critical section, the return value is zero." Entered here as EnterCriticalSection and 1. The case of
 * another thread owning the section is not modelled (S-vst-trycs): with the cooperative threads of fle_thread.c, Enter would wait where Try would return 0; a run in which the
 * plugin makes a thread that contends for a section is not what this answer is for. */
void fle_imp_TryEnterCriticalSection(void)
{
    fle_imp_EnterCriticalSection();
    R[0] = 1;
}

/* gdiplus.dll: the plugin starts GDI+ when its component is created (the editor's drawing library). The harness opens no editor, so only the start and the memory functions are
 * answered (S-vst-gdip); every other GDI+ function stays unanswered and stops the run at the call by name if an editor is ever created.
 * GdiplusStartup(ULONG_PTR *token rcx, const GdiplusStartupInput *input rdx, GdiplusStartupOutput *output r8): Status. Learn (GdiplusStartup): "Initializes Windows GDI+"; the token
 * is what GdiplusShutdown takes; Ok = 0. GdiplusStartupInput is { UINT32 GdiplusVersion; DebugEventProc; BOOL SuppressBackgroundThread; BOOL SuppressExternalCodecs } (the version
 * must be 1: else UnsupportedGdiplusVersion = 11). With SuppressBackgroundThread TRUE the caller must call the NotificationHook that the output gives, and no such function exists
 * here: that case stops. The token is a nonzero counter value (a choice: Learn says only that it is an opaque value). */
static uint64_t vst_gdip_token;

void fle_imp_GdiplusStartup(void)
{
    uint64_t ptok = R[1], in = R[2];
    if (ptok == 0 || in == 0) { R[0] = 2; return; } /* InvalidParameter */
    if ((uint32_t)fle_ld(in, 4) != 1u) { R[0] = 11; return; } /* UnsupportedGdiplusVersion */
    if ((uint32_t)fle_ld(in + 16, 4) != 0u) fle_stop("GdiplusStartup", "SuppressBackgroundThread is TRUE: the NotificationHook/NotificationUnhook functions are not provided (not answered)");
    fle_st(ptok, 8, ++vst_gdip_token);
    R[0] = 0;
}

void fle_imp_GdiplusShutdown(void)
{
    if (R[1] == 0 || R[1] > vst_gdip_token) fle_stop("GdiplusShutdown", "a token that GdiplusStartup did not hand out (not defined)");
}

/* GdipAlloc(size_t size rcx): void* and GdipFree(void *ptr rcx): the allocator behind `new` of the GDI+ classes (gdiplusbase.h). Learn lists them as flat API functions
 * without a page of semantics beyond "allocates"/"frees"; calloc/free give a zeroed block, a freeing of NULL does nothing (as free). */
void fle_imp_GdipAlloc(void)
{
    R[0] = (uint64_t)(uintptr_t)calloc(1, R[1] ? (size_t)R[1] : 1u);
}

void fle_imp_GdipFree(void)
{
    free((void *)(uintptr_t)R[1]);
}

/* gdi32!AddFontResourceExW(LPCWSTR name rcx, DWORD fl edx, PVOID res r8): int and RemoveFontResourceExW(same arguments): BOOL. The plugin writes a TrueType font that it carries in
 * its resources to a temporary file and adds it as a private font for its editor's text. This environment has no font engine for a plugin (the harness opens no editor), so the
 * answers are the success values of the Learn pages ("the number of fonts added": 1 for one file; TRUE for the removal) without a font being registered anywhere (S-vst-addfont).
 * fl must be FR_PRIVATE (0x10) or FR_NOT_ENUM (0x20) or 0 and res must be NULL ("Reserved"); another value stops. */
void fle_imp_AddFontResourceExW(void)
{
    if (R[8] != 0) fle_stop("AddFontResourceExW", "res is not NULL (reserved)");
    if ((uint32_t)R[2] & ~0x30u) fle_stop("AddFontResourceExW", "fl has bits other than FR_PRIVATE and FR_NOT_ENUM: not answered");
    if (R[1] == 0) { R[0] = 0; return; }
    R[0] = 1;
}

void fle_imp_RemoveFontResourceExW(void)
{
    if (R[8] != 0) fle_stop("RemoveFontResourceExW", "pdv is not NULL (reserved)");
    R[0] = R[1] ? 1 : 0;
}

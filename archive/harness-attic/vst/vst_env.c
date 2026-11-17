/* Harness-side answers of the environment that a third-party VST3 plugin asks for (decision 0156, PROVISIONAL). They replace nothing in the runtime:
 * the build links with -Wl,--wrap=<symbol> (build_vst_native.sh) and each wrapper either answers or calls the runtime's own function (__real_...).
 * Every choice here is listed as S-vst-... in docs/decisions/0156-first-vst3-plugin.md. */
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "fle_env.h"
#include "fle_imports.h"
#include "fle_modules.h"
#include "fle_runtime.h"

void __real_fle_imp_LoadLibraryExW(void);
void __real_fle_imp_LoadLibraryW(void);

static int ascii_ieq_prefix(const uint16_t *u, const char *p)
{
    for (; *p; p++, u++) {
        uint16_t c = *u;
        if (c >= 'A' && c <= 'Z') c = (uint16_t)(c - 'A' + 'a');
        if (c != (uint16_t)(unsigned char)*p) return 0;
    }
    return 1;
}

static uint64_t module_handle_by_name(const char *name)
{
    uint64_t i;
    for (i = 0; i < fle_module_count(); i++)
        if (strcmp(fle_module_name(i), name) == 0) return fle_module_handle(i);
    return 0;
}

/* S-vst-apiset: the API-set contract names (api-ms-win-*, ext-ms-win-*) that the static UCRT of a plugin passes to LoadLibraryExW. On Windows 10 each is a
 * forwarder to a system DLL; here a load of such a name answers the handle of the DLL that this runtime has for the same functions (the response table is keyed by that DLL
 * name, so GetProcAddress on the handle finds a row when one exists and gives NULL/127 otherwise, which sends the CRT to its own fallback). The mapping (ntuser -> user32,
 * security -> advapi32, everything else -> kernel32) is a CHOICE without a source. FreeLibrary is not wrapped: the plugin does not free these handles during the run. */
static int apiset_target(const uint16_t *u, const char **dll)
{
    if (!ascii_ieq_prefix(u, "api-ms-win-") && !ascii_ieq_prefix(u, "ext-ms-win-")) return 0;
    {
        char name[128];
        size_t n;
        for (n = 0; u[n] && n + 1 < sizeof name; n++) name[n] = u[n] < 0x80 ? (char)u[n] : '?';
        name[n] = 0;
        if (strstr(name, "ntuser")) *dll = "user32.dll";
        else if (strstr(name, "-security-")) *dll = "advapi32.dll";
        else *dll = "kernel32.dll";
    }
    return 1;
}

static int wrap_apiset(uint64_t pname)
{
    uint16_t buf[260];
    size_t k;
    const char *dll;
    uint64_t h;
    if (pname == 0) return 0;
    for (k = 0; k < 259; k++) {
        buf[k] = (uint16_t)fle_ld(pname + 2u * k, 2);
        if (buf[k] == 0) break;
    }
    buf[k] = 0;
    if (!apiset_target(buf, &dll)) return 0;
    h = module_handle_by_name(dll);
    if (!h) return 0;
    R[0] = h;
    return 1;
}

/* S-vst-gfx-absent: the graphics libraries that JUCE probes when it starts (Direct2D for its renderer). On a Windows 10 they exist; this environment has no graphics
 * hardware path (and the harness opens no editor), so a load of one of them fails as for a missing file: NULL with ERROR_MOD_NOT_FOUND (126), the answer the runtime gives for the
 * names it is "defined to lack" (fle_load.c). JUCE then uses its software renderer. A CHOICE, not what a real machine does; the list holds only names that were asked for. */
static const char *const vst_absent[] = { "d2d1.dll", "d2d1" };

static int vst_is_absent_w(uint64_t p)
{
    char nm[64];
    size_t i;
    if (!p) return 0;
    for (i = 0; i + 1 < sizeof nm; i++) { uint16_t c = (uint16_t)fle_ld(p + 2 * i, 2); if (!c) break; nm[i] = c < 0x80 ? (c >= 'A' && c <= 'Z' ? (char)(c + 32) : (char)c) : '?'; }
    nm[i] = 0;
    for (i = 0; i < sizeof vst_absent / sizeof vst_absent[0]; i++) if (!strcmp(nm, vst_absent[i])) return 1;
    return 0;
}
static int vst_is_absent_a(uint64_t p)
{
    char nm[64];
    size_t i;
    if (!p) return 0;
    for (i = 0; i + 1 < sizeof nm; i++) { uint8_t c = (uint8_t)fle_ld(p + i, 1); if (!c) break; nm[i] = (c >= 'A' && c <= 'Z') ? (char)(c + 32) : (char)c; }
    nm[i] = 0;
    for (i = 0; i < sizeof vst_absent / sizeof vst_absent[0]; i++) if (!strcmp(nm, vst_absent[i])) return 1;
    return 0;
}
static void vst_load_fail(void)
{
    fle_set_last_error(FLE_ERROR_MOD_NOT_FOUND);
    R[0] = 0;
}

void __real_fle_imp_LoadLibraryA(void);

void __wrap_fle_imp_LoadLibraryExW(void)
{
    if (wrap_apiset(R[1])) return;
    if (vst_is_absent_w(R[1])) { vst_load_fail(); return; }
    __real_fle_imp_LoadLibraryExW();
}

void __wrap_fle_imp_LoadLibraryW(void)
{
    if (wrap_apiset(R[1])) return;
    if (vst_is_absent_w(R[1])) { vst_load_fail(); return; }
    __real_fle_imp_LoadLibraryW();
}

void __wrap_fle_imp_LoadLibraryA(void)
{
    if (vst_is_absent_a(R[1])) { vst_load_fail(); return; }
    __real_fle_imp_LoadLibraryA();
}

/* S-vst-getmodulehandleex: kernel32!GetModuleHandleExW(DWORD dwFlags ecx, LPCWSTR lpModuleName rdx, HMODULE *phModule r8). The runtime stops by name (decision 0078). Learn
 * (GetModuleHandleExW): GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS (4) makes lpModuleName an address inside the module; PIN (1) and UNCHANGED_REFCOUNT (2) change reference
 * bookkeeping that this runtime does not keep for the plugin's own module. Answered here: FROM_ADDRESS with an address inside the plugin's image -> its base; a name ->
 * what GetModuleHandleW answers (NULL means the current process's exe, FLE_EXE_HANDLE). Any other address fails with ERROR_MOD_NOT_FOUND (126): the addresses of the
 * runtime's own answers are not inside any module that the plugin could know. */
uint64_t vst_module_base;
uint64_t vst_module_size;
void fle_imp_GetModuleHandleW(void);

void __wrap_fle_imp_GetModuleHandleExW(void)
{
    uint32_t flags = (uint32_t)R[1];
    uint64_t name = R[2], ph = R[8];
    if (!ph) { fle_set_last_error(FLE_W32_ERR_INVALID_PARAMETER); R[0] = 0; return; }
    if (flags & 4u) {
        if (vst_module_base && name >= vst_module_base && name < vst_module_base + vst_module_size) { fle_st(ph, 8, vst_module_base); R[0] = 1; return; }
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

/* S-vst-classatom: CreateWindowExW with the class given as an ATOM (MAKEINTATOM(atom) in lpClassName: JUCE registers its hidden window's class once and creates the window by
 * the atom). The runtime stops on an atom (tests/class.rs, window.rs pin that), so the wrapper turns an atom that RegisterClassW/ExW handed out into the class's registered name
 * (fle_class_name_of) and calls the runtime's own CreateWindowExW with it; an atom the table does not know goes to the runtime unchanged and stops there. */
#include "fle_class.h"
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

/* S-vst-secattr: CreateFileW with a SECURITY_ATTRIBUTES (JUCE passes one for the files it creates). The runtime stops on a non-NULL pointer. Learn (SECURITY_ATTRIBUTES): nLength,
 * lpSecurityDescriptor, bInheritHandle; a NULL descriptor means "the default security descriptor", and bInheritHandle only matters for child processes, of which there are none. So a
 * structure with nLength 24 and a NULL descriptor is passed on as NULL; anything else (a real descriptor) stops. With VST_TRACE set, every CreateFileW is logged. */
void __real_fle_imp_CreateFileW(void);

void __wrap_fle_imp_CreateFileW(void)
{
    uint64_t sa = R[9];
    if (getenv("VST_TRACE")) {
        char nm[200];
        size_t i;
        for (i = 0; i + 1 < sizeof nm; i++) { uint16_t c = (uint16_t)fle_ld(R[1] + 2 * i, 2); if (!c) break; nm[i] = c < 0x80 ? (char)c : '?'; }
        nm[i] = 0;
        fprintf(stderr, "vst: CreateFileW(\"%s\", access %#x, share %#x, sa %#llx, disposition %u, flags %#x)\n", nm, (unsigned)R[2], (unsigned)R[8], (unsigned long long)sa, (unsigned)fle_arg(5), (unsigned)fle_arg(6));
    }
    if (sa != 0) {
        if ((uint32_t)fle_ld(sa, 4) != 24u || fle_ld(sa + 8, 8) != 0) fle_stop("CreateFileW", "lpSecurityAttributes holds a security descriptor (or nLength is not 24): not modelled");
        R[9] = 0;
    }
    __real_fle_imp_CreateFileW();
}

/* S-vst-filetype: kernel32!GetFileType(HANDLE hFile rcx): DWORD. The runtime answers only for the standard handles (which do not exist here). The CRT calls it on every file it opens
 * (fopen/_open_osfhandle). Learn: FILE_TYPE_DISK (0x0001) "The specified file is a disk file". A handle that fle_vfs.c issued for an open virtual file (fle_vfs_file_info) is a disk file;
 * any other value goes to the runtime unchanged. */
#include "fle_vfs.h"
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

void fle_imp_SetFilePointer(void);
void fle_imp_GetLastError(void);

/* S-vst-setfilepointerex (the runtime stops by name: fle_vfs.c): kernel32!SetFilePointerEx(HANDLE hFile rcx, LARGE_INTEGER liDistanceToMove rdx, PLARGE_INTEGER lpNewFilePointer r8, DWORD dwMoveMethod r9d): BOOL. Learn: the 64-bit form of
 * SetFilePointer; "If the function succeeds, the return value is nonzero". The virtual file system answers SetFilePointer (fle_vfs.c) with the high half passed through
 * lpDistanceToMoveHigh, so this is that call with the distance split in two; the failure codes and the stops are that function's (a bad handle 6, a negative position 131). The new
 * position is written to *lpNewFilePointer when it is not NULL (the page: it "can be NULL"). */
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
    if (lo == 0xFFFFFFFFu && err != 0) {
        R[0] = 0;
        return;
    }
    if (newp) fle_st(newp, 8, ((uint64_t)hi << 32) | lo);
    R[0] = 1;
}

/* S-vst-lockresource: kernel32!LockResource(HGLOBAL hResData rcx). The runtime reads the resource tree of the exe image (0x400000) before it looks at the handle, and an
 * environment whose only module is the plugin has no such image. LoadResource of this runtime returns the VA of the data inside the (mapped) module, and its LockResource
 * returns that same VA ("the image is mapped, nothing is locked"), so for a value inside the plugin's image the answer is the value itself; anything else goes to the runtime. */
void __real_fle_imp_LockResource(void);

void __wrap_fle_imp_LockResource(void)
{
    uint64_t h = R[1];
    if (vst_module_base && h >= vst_module_base && h < vst_module_base + vst_module_size) { R[0] = h; return; }
    __real_fle_imp_LockResource();
}

/* S-vst-fmtmsg-lang: FormatMessageW with dwLanguageId 0x0400. That is MAKELANGID(LANG_NEUTRAL, SUBLANG_DEFAULT), the value the Learn page of FormatMessage uses in its examples for
 * "look the message up with the default rules"; the runtime accepts only 0 (which the page describes as the same default order: language neutral, thread, user default, system
 * default, US English) because it holds the US English texts only. The two are treated as one here. */
void __real_fle_imp_FormatMessageW(void);

void __wrap_fle_imp_FormatMessageW(void)
{
    if ((uint32_t)R[9] == 0x400u) R[9] = 0;
    __real_fle_imp_FormatMessageW();
}

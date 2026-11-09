/* vst3a copy of vst_env.c (decision 0165) with additions at the end. Harness-side answers of the environment that a third-party VST3 plugin asks for (decision 0156, PROVISIONAL). They replace nothing in the runtime:
 * the build links with -Wl,--wrap=<symbol> (build_vst_native.sh) and each wrapper either answers or calls the runtime's own function (__real_...).
 * Every choice here is listed as S-vst-... in docs/decisions/0156-first-vst3-plugin.md. */
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "fle_env.h"
#include "fle_imports.h"
#include "fle_modules.h"
#include "fle_runtime.h"
#include "vst3a.h"

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
static const char *const vst_absent[] = { "d2d1.dll", "d2d1", "dcomp.dll", "dcomp" };

static int vst_is_absent_w(uint64_t p)
{
    char nm[64], full[300];
    size_t i;
    if (!p) return 0;
    for (i = 0; i + 1 < sizeof full; i++) { uint16_t c = (uint16_t)fle_ld(p + 2 * i, 2); if (!c) break; full[i] = c < 0x80 ? (char)c : '?'; }
    full[i] = 0;
    /* S-vst3a-loadlib-path: a module name WITH a path (Serum 2 looks for C:\Program Files\Common Files\MTS-ESP\LIBMTS.dll) names a file; the virtual volume holds no module file, so
     * the load fails as for a missing file (ERROR_MOD_NOT_FOUND 126). The names are printed ("vst3a: LoadLibrary of a path"). */
    if (strchr(full, '\\') || strchr(full, '/') || strchr(full, ':')) { printf("vst3a: LoadLibrary of a path: \"%s\" -> NULL, ERROR_MOD_NOT_FOUND\n", full); return 1; }
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
    /* S-vst3a-createfile-empty: an empty file name. Serum 2 opens "" with FILE_FLAG_BACKUP_SEMANTICS|OVERLAPPED (a directory watcher on its content folder, which it could not find and so
     * left as an empty string). Win32 gives ERROR_PATH_NOT_FOUND (3) and INVALID_HANDLE_VALUE for an empty name (my recollection of the behaviour; Learn's CreateFile page lists no code for it). */
    if (R[1] && fle_ld(R[1], 2) == 0) {
        printf("vst3a: CreateFileW(\"\", access %#x, share %#x, disposition %u, flags %#x) -> INVALID_HANDLE_VALUE, ERROR_PATH_NOT_FOUND\n", (unsigned)R[2], (unsigned)R[8], (unsigned)fle_arg(5), (unsigned)fle_arg(6));
        fle_set_last_error(3);
        R[0] = UINT64_C(0xffffffffffffffff);
        return;
    }
    /* S-vst3a-dirwatch-fail: a directory opened for change notification (FILE_FLAG_BACKUP_SEMANTICS 0x02000000 | FILE_FLAG_OVERLAPPED 0x40000000, as ReadDirectoryChangesW needs) of the content
     * folder: the virtual volume has no directory handles and no change notification; the open fails as for a denied one (ERROR_ACCESS_DENIED 5; the choice of code is mine). The plugin
     * continues without its folder watcher. */
    if ((fle_arg(6) & 0x42000000u) == 0x42000000u) {
        char nm[200];
        size_t i;
        for (i = 0; i + 1 < sizeof nm; i++) { uint16_t c = (uint16_t)fle_ld(R[1] + 2 * i, 2); if (!c) break; nm[i] = c < 0x80 ? (char)c : '?'; }
        nm[i] = 0;
        printf("vst3a: CreateFileW(\"%s\", flags %#x: directory change notification) -> INVALID_HANDLE_VALUE, ERROR_ACCESS_DENIED\n", nm, (unsigned)fle_arg(6));
        fle_set_last_error(5);
        R[0] = UINT64_C(0xffffffffffffffff);
        return;
    }
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

extern uint64_t vst_module_base, vst_module_size;
void __real_fle_imp_VirtualProtect(void);

/* S-vst3a-vprotect: VirtualProtect on an address inside the plugin's own image (the CRT of Serum 2 marks the 0x100 bytes of its .fptable read-only after DllMain starts: rcx = 0x18210a000,
 * edx = 0x100, r8d = PAGE_READONLY). The runtime stops by name for memory it does not track (its image is not a VirtualAlloc region). Here the call is answered as a success; the old
 * protection is the one of the section (its Characteristics: write -> PAGE_READWRITE 4, execute -> PAGE_EXECUTE_READ 0x20, else PAGE_READONLY 2) or the last protection set by an earlier call
 * on the same page; the new protection is recorded and enforced nowhere (decision 0045: no page protection is enforced). A call outside the image goes to the runtime. */
static struct { uint64_t page; uint32_t prot; } vp_pages[64];
static int vp_n;
static uint32_t image_section_prot(uint64_t a)
{
    const uint8_t *m = (const uint8_t *)(uintptr_t)vst_module_base;
    uint32_t lfa, rva = (uint32_t)(a - vst_module_base);
    uint16_t nsec, osz;
    memcpy(&lfa, m + 0x3c, 4); memcpy(&nsec, m + lfa + 6, 2); memcpy(&osz, m + lfa + 20, 2);
    for (unsigned i = 0; i < nsec; i++) {
        const uint8_t *s = m + lfa + 24 + osz + 40u * i;
        uint32_t vs, va, ch;
        memcpy(&vs, s + 8, 4); memcpy(&va, s + 12, 4); memcpy(&ch, s + 36, 4);
        if (rva >= va && rva < va + vs) return (ch & 0x80000000u) ? 0x04u : (ch & 0x20000000u) ? 0x20u : 0x02u;
    }
    return 0x02u;
}
void __wrap_fle_imp_VirtualProtect(void)
{
    uint64_t addr = R[1], size = R[2], pold = R[9];
    uint32_t np = (uint32_t)R[8], old;
    if (vst_module_base && addr >= vst_module_base && addr + size <= vst_module_base + vst_module_size && size) {
        uint64_t pg = addr & ~UINT64_C(0xfff);
        int k;
        vst3a_note("VirtualProtect(image)");
        if (!pold) { R[0] = 0; return; }
        old = image_section_prot(addr);
        for (k = 0; k < vp_n; k++) if (vp_pages[k].page == pg) old = vp_pages[k].prot;
        fle_st(pold, 4, old);
        for (uint64_t p = pg; p < addr + size; p += 0x1000) {
            for (k = 0; k < vp_n; k++) if (vp_pages[k].page == p) break;
            if (k == vp_n) { if (vp_n >= 64) vst3a_stop("VirtualProtect", "more than 64 pages of the image changed"); vp_pages[vp_n++].page = p; }
            vp_pages[k].prot = np;
        }
        R[0] = 1;
        return;
    }
    __real_fle_imp_VirtualProtect();
}

void __real_fle_imp_RtlPcToFileHeader(void);
/* ntdll!RtlPcToFileHeader(PVOID PcValue rcx, PVOID *BaseOfImage rdx): PVOID. Documented (Learn, "RtlPcToFileHeader"): "Retrieves the base address of the image that contains the specified PC
 * value"; returns the base, and stores it in *BaseOfImage, or NULL (and *BaseOfImage = NULL) if no image contains the PC. Here (S-vst3a-pctofileheader): the plugin's own image answers; any other PC goes
 * to the runtime, which stops by name. Serum 2 calls it while it builds EXCEPTION_RECORDs of its pre-constructed std::exception_ptr objects in the static initialisers (no exception is thrown
 * by these calls; they fill ExceptionInformation[3] with the image base for the 64-bit C++ exception record). */
void __wrap_fle_imp_RtlPcToFileHeader(void)
{
    uint64_t pc = R[1], pb = R[2];
    if (vst_module_base && pc >= vst_module_base && pc < vst_module_base + vst_module_size) {
        vst3a_note("RtlPcToFileHeader");
        if (pb) fle_st(pb, 8, vst_module_base);
        R[0] = vst_module_base;
        return;
    }
    __real_fle_imp_RtlPcToFileHeader();
}

/* ---- SRW locks: TryAcquireSRWLockExclusive (S-vst3a-srw-try) ----
 * The runtime keeps the SRW state in a private table (fle_sys.c) and stops by name when a lock is acquired that is already held (no other thread could release it). TryAcquire must instead
 * answer FALSE for a held lock (Learn: "If the lock was acquired the return value is nonzero; otherwise zero"). The four Acquire/Release functions are wrapped to keep a copy of
 * "address -> mode, count" here; TryAcquireSRWLockExclusive answers 0 when the copy says held (exclusively or shared), otherwise it takes the lock through the runtime and answers 1. The
 * copy and the runtime's table move together because every Acquire/Release passes through the wrappers. */
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


/* ---- threads of the plugin (S-vst3a-thread) ----
 * The runtime's CreateThread (cooperative threads, decisions 0122/0124) gives a new thread a zeroed TEB and a zeroed TlsAlloc slot table. Serum 2's thread routine reads the PEB through the TEB
 * (gs:[0x30] -> [+0x60] -> [+0x20] -> [+8]) and, with a static CRT, would use the implicit TLS block (gs:[0x58] -> [tls_index]). The harness wraps CreateThread: the routine becomes the pseudo-VA of
 * vst3a_thread_entry (a row of the dispatch table, 0x7FFC30000010), which, on the new thread, (1) gives the TEB a PEB pointer (a zeroed PEB: ImageBaseAddress = the exe handle, ProcessParameters
 * = a zeroed block), (2) makes the module's TLS block for the thread (template + zero fill) and stores it at the TLS index, (3) calls the TLS callbacks with DLL_THREAD_ATTACH (2), (4) calls the
 * plugin's routine. DllMain(DLL_THREAD_ATTACH) is called by the runtime before all this. */
extern uint64_t tls_cb[8];
extern int ntls_cb;
extern uint64_t vst3a_tls_idx, vst3a_tls_start, vst3a_tls_len;
extern uint64_t fle_tls_values[];
extern uint64_t *fle_tls_cur;
extern uint8_t *fle_teb_cur;
extern uint8_t fle_teb[];
void __real_fle_imp_CreateThread(void);
uint64_t gcall(uint64_t va, const uint64_t a[6], const double d[4]);
typedef struct { uint64_t start, param; } thr_launch;
static uint8_t *g_peb;
void __wrap_fle_imp_CreateThread(void)
{
    thr_launch *l = malloc(sizeof *l);
    l->start = R[8]; l->param = R[9];
    vst3a_note("CreateThread");
    printf("vst3a: CreateThread(start %#llx, param %#llx)\n", (unsigned long long)l->start, (unsigned long long)l->param);
    R[8] = UINT64_C(0x7FFC30000010);
    R[9] = (uint64_t)(uintptr_t)l;
    __real_fle_imp_CreateThread();
}
void vst3a_thread_entry(void)
{
    thr_launch *l = (thr_launch *)(uintptr_t)R[1];
    uint64_t start = l->start, param = l->param, blk, peb;
    uint8_t *teb = fle_teb_cur ? fle_teb_cur : fle_teb;
    uint64_t *tbl = fle_tls_cur ? fle_tls_cur : fle_tls_values;
    size_t n = (size_t)vst3a_tls_len;
    free(l);
    if (!g_peb) {
        uint8_t *params = calloc(1, 0x400);
        g_peb = calloc(1, 0x400);
        { uint64_t pp = (uint64_t)(uintptr_t)params, exe = 0x400000; memcpy(g_peb + 0x20, &pp, 8); memcpy(g_peb + 0x10, &exe, 8); }
    }
    peb = (uint64_t)(uintptr_t)g_peb;
    memcpy(teb + 0x60, &peb, 8);
    if (vst3a_tls_len) {
        blk = (uint64_t)(uintptr_t)aligned_alloc(64, (n + 63) & ~(size_t)63);
        memset((void *)(uintptr_t)blk, 0, (n + 63) & ~(size_t)63);
        memcpy((void *)(uintptr_t)blk, (void *)(uintptr_t)vst3a_tls_start, n);
        tbl[vst3a_tls_idx] = blk;
    }
    for (int k = 0; k < ntls_cb; k++) { const uint64_t a[6] = { vst_module_base, 2, 0, 0, 0, 0 }; gcall(tls_cb[k], a, NULL); }
    { const uint64_t a[6] = { param, 0, 0, 0, 0, 0 }; R[0] = gcall(start, a, NULL); }
}

/* ---- DIAGNOSTIC ONLY (decision 0165): VST3A_DIAG_GUEST_RC0=1 clears the rounding-control bits of the guest MXCSR after every ldmxcsr, i.e. the plugin's round-toward-zero (RC = 3) becomes
 * round-to-nearest for the lifted floating point. The output is NOT the plugin's (results round differently); it only tells what the exact handling of RC != 0 costs. ---- */
extern uint32_t fle_mxcsr;
void __real_fle_mxcsr_load(uint64_t a);
void __wrap_fle_mxcsr_load(uint64_t a)
{
    static int diag = -1;
    __real_fle_mxcsr_load(a);
    if (diag < 0) diag = getenv("VST3A_DIAG_GUEST_RC0") != NULL && getenv("VST3A_DIAG_GUEST_RC0")[0] == '1';
    if (diag) fle_mxcsr &= ~0x6000u;
}

/* ---- condition variables of the plugin's worker thread (S-vst3a-cv) ----
 * The runtime's SleepConditionVariableSRW stops by name for every call (decision 0092: one thread, nobody could call Wake). With the cooperative threads of decisions 0122/0124 another thread
 * can. This is a polling model: the sleeper releases its SRW lock, gives the other threads the processor with Sleep(1) of the virtual clock until a Wake*ConditionVariable on the same variable
 * has been counted (or its time-out has passed), takes the lock again and answers TRUE (woken) or FALSE with ERROR_TIMEOUT (1460). A Wake counts for every thread that was already
 * sleeping on the variable (spurious wake-ups are allowed by the page: callers re-check their condition). The runtime's Wake functions are called as well (they do nothing). */
void __real_fle_imp_WakeConditionVariable(void);
void __real_fle_imp_WakeAllConditionVariable(void);
void fle_imp_Sleep(void);
void __wrap_fle_imp_AcquireSRWLockExclusive(void);
void __wrap_fle_imp_ReleaseSRWLockExclusive(void);
void __wrap_fle_imp_AcquireSRWLockShared(void);
void __wrap_fle_imp_ReleaseSRWLockShared(void);
static struct { uint64_t cv; unsigned long long wake; } cv_tab[64];
static int cv_n;
static unsigned long long cv_sleeps, cv_polls, cv_wakes;
static int cv_slot(uint64_t cv)
{
    for (int i = 0; i < cv_n; i++) if (cv_tab[i].cv == cv) return i;
    if (cv_n >= 64) vst3a_stop("ConditionVariable", "more than 64 condition variables");
    cv_tab[cv_n].cv = cv; cv_tab[cv_n].wake = 0;
    return cv_n++;
}
static void cv_wake(void) { cv_wakes++; for (int i = 0; i < cv_n; i++) if (cv_tab[i].cv == R[1]) { cv_tab[i].wake++; return; } }   /* a variable nobody has slept on needs no record */
void __wrap_fle_imp_WakeConditionVariable(void) { cv_wake(); }
void __wrap_fle_imp_WakeAllConditionVariable(void) { cv_wake(); }
void __wrap_fle_imp_SleepConditionVariableSRW(void)
{
    uint64_t cv = R[1], lock = R[2];
    uint32_t ms = (uint32_t)R[8], flags = (uint32_t)R[9];
    int slot = cv_slot(cv), woken = 0;
    unsigned long long seen = cv_tab[slot].wake;
    uint64_t waited = 0;
    cv_sleeps++;
    if (flags & ~1u) vst3a_stop("SleepConditionVariableSRW", "Flags other than 0 / CONDITION_VARIABLE_LOCKMODE_SHARED");
    R[1] = lock;
    if (flags & 1u) __wrap_fle_imp_ReleaseSRWLockShared(); else __wrap_fle_imp_ReleaseSRWLockExclusive();
    for (;;) {
        if (cv_tab[slot].wake != seen) { woken = 1; break; }
        if (ms != 0xffffffffu && waited >= ms) break;
        R[1] = 1;
        cv_polls++;
        fle_imp_Sleep();
        waited++;
    }
    R[1] = lock;
    if (flags & 1u) __wrap_fle_imp_AcquireSRWLockShared(); else __wrap_fle_imp_AcquireSRWLockExclusive();
    if (!woken) fle_set_last_error(1460);
    R[0] = woken ? 1 : 0;
}
void vst3a_cv_report(void) { printf("host condition variables: sleeps=%llu polls=%llu wakes=%llu\n", cv_sleeps, cv_polls, cv_wakes); }

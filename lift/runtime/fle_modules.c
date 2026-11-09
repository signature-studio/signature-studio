#include <stdint.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_modules.h"
#include "fle_realmod.h"

/* Virtual module table (decision 0015). The handle of entry i is FLE_MODULE_HANDLE_BASE +
 * FLE_MODULE_HANDLE_STEP * (i + 1), so the order below fixes the handles: append new names at the
 * end and never reorder. Names are lower-case. The first 24 are the DLLs of the PE import table
 * of FLEngine_x64.dll (lift/out-imports/imports.txt, "normal" slots); the test
 * every_pe_import_dll_is_a_module checks that list against the DLL. ntdll.dll is added because a
 * Windows process always has it loaded (the task names it); it is the only name not in the PE import
 * table. Modules that the PE only delay-loads (dwmapi, imm32, ...) are not in the table. */
static const char *const fle_modules[] = {
    "advapi32.dll",
    "api-ms-win-crt-string-l1-1-0.dll",
    "comctl32.dll",
    "comdlg32.dll",
    "crypt32.dll",
    "dsound.dll",
    "gdi32.dll",
    "imagehlp.dll",
    "kernel32.dll",
    "mpr.dll",
    "msacm32.dll",
    "oleacc.dll",
    "ole32.dll",
    "oleaut32.dll",
    "opengl32.dll",
    "shell32.dll",
    "shfolder.dll",
    "shlwapi.dll",
    "user32.dll",
    "version.dll",
    "wininet.dll",
    "winmm.dll",
    "winspool.drv",
    "wintrust.dll",
    "ntdll.dll",
    "msctf.dll", /* N = 26, decision 0055: Windows 10 always has it (decision 0014), so it is a module, not a lacking name */
    /* N = 27..30, decision 0081 (option (a), provisional): the API set names that dsp_ippv2_x64.dll passes to LoadLibraryExW. They are
     * modules (the load succeeds) but no response-table row has them in its dll column, so GetProcAddress on them gives NULL and 127;
     * they are not in fle_load_absent[] (no claim that the environment lacks them). */
    "api-ms-win-core-synch-l1-2-0.dll",
    "api-ms-win-core-fibers-l1-1-1.dll",
    "api-ms-win-core-localization-l1-2-1.dll",
    "api-ms-win-appmodel-runtime-l1-1-2.dll",
    /* N = 31, 32, decision 0088 (option (a), provisional): avrt.dll and powrprof.dll, which unit 145 of FLEngine passes to LoadLibraryW. Same shape as
     * N = 27..30: the load succeeds, no response-table row has them in its dll column (GetProcAddress gives NULL and 127), and they are not in
     * fle_load_absent[]. This is not what a real Windows 10 does (there the functions exist); it lets the guest take its "no such function" path. */
    "avrt.dll",
    "powrprof.dll",
    /* N = 33, 34, decision 0096 (provisional): the deferred modules. blend2d.dll and quickfontcache_x64.dll are installed with the product
     * (decision 0073) and are to be lifted into real modules (decision 0086, another lane), but are not lifted yet. Until then the load
     * succeeds (DllMain is not run, no image is made, the reference count is that of any virtual module), GetProcAddress gives a pseudo-VA
     * for each export name (response-table rows under these dll names), and calling that pseudo-VA stops by name (fle_imp_deferred_* below).
     * A real module registered under the same name wins everywhere (fle_realmod.c; see FLE_DEFERRED_FIRST). */
    "blend2d.dll",
    "quickfontcache_x64.dll",
    /* N = 35, decision 0099 (provisional): shcore.dll. The guest delay-loads shcore!GetProcessDpiAwareness (unit 245); with shcore.dll absent the
     * delay-load helper got NULL from LoadLibraryA and raised 0xc06d007e (VcppException ERROR_MOD_NOT_FOUND). Windows 10 has the DLL (decision 0014),
     * so it is a module; the one function the guest asks for has a response row under it (fle_proc.c). Same shape as N = 31, 32. It follows the
     * deferred modules (N = 33, 34: decision 0096), which were first; the order is only the order of arrival (decision 0015: append only). */
    "shcore.dll",
    /* N = 36, decision 0118 wall 7 (provisional): rpcrt4.dll. The guest's CreateFruityInstance helper f_11b2450 delay-loads rpcrt4!UuidCreate; with rpcrt4.dll
     * absent (fle_load_absent[], decisions 0015/0029) the delay-load helper got NULL from LoadLibraryA and raised 0xc06d007e. Windows 10 has the DLL
     * (decision 0014), so it is a module; the one function the guest asks for has a response row under it (fle_proc.c). Same shape as shcore.dll (N = 35,
     * decision 0099); appended after it (decision 0015: append only). */
    "rpcrt4.dll",
    /* N = 37, decision 0119 wall 11 (provisional): wtsapi32.dll. The guest's TApplication window creation (f_867e10) delay-loads wtsapi32!WTSRegisterSessionNotification
     * when the Windows major version is above 5; with wtsapi32.dll absent (fle_load_absent[], decisions 0015/0029) the delay-load helper got NULL from LoadLibraryA and
     * raised 0xc06d007e. Windows 10 has the DLL (decision 0014), so it is a module; the one function the guest asks for has a response row under it (fle_win.c).
     * Same shape as rpcrt4.dll (N = 36, decision 0118) and shcore.dll (N = 35, decision 0099); appended after them (decision 0015: append only). */
    "wtsapi32.dll",
    /* N = 38, decision 0119 wall 12 (provisional): uxtheme.dll. The guest's TApplication window creation (f_867e10) delay-loads uxtheme!BufferedPaintInit when the
     * Windows major version is above 6 or is 6 and the minor version is 0 or more; with uxtheme.dll absent (fle_load_absent[], decisions 0015/0029) the delay-load helper
     * got NULL from LoadLibraryA and raised 0xc06d007e. Windows 10 has the DLL (decision 0014), so it is a module; the one function the guest asks for has a response
     * row under it (fle_win.c). The other four delay-loaded names of uxtheme (BeginBufferedPaint, EndBufferedPaint, BufferedPaintUnInit, BufferedPaintSetAlpha) have no
     * row: a call reaches the delay-load failure for a missing procedure (0xc06d007f), as the shcore!GetDpiForMonitor of decision 0099 does. Appended after wtsapi32.dll
     * (decision 0015: append only). */
    "uxtheme.dll",
    /* N = 39, decision 0122 (provisional): dwmapi.dll. QuickFontCache's TTimerThread calls dwmapi!DwmFlush on every cycle through a delay-load (the helper reaches it with LoadLibraryA("DWMAPI.DLL") and
     * GetProcAddress); with dwmapi.dll absent (fle_load_absent[], decisions 0015/0029) the delay-load helper got NULL and raised 0xc06d007e. Windows 10 has the DLL (decision 0014), so it is a module;
     * the one function the thread asks for has a response row under it (fle_thread.c: DwmFlush waits for the next 60 Hz boundary of the virtual clock and returns S_OK). QuickFontCache's own
     * GetProcAddress("DwmIsCompositionEnabled") and ("DwmGetCompositionTimingInfo") find no row, so they get NULL and 127 (as before: its code then takes the EnumDisplaySettingsW branch). Appended
     * after uxtheme.dll (decision 0015: append only). */
    "dwmapi.dll",
    /* N = 40, decision 0238 (load2, provisional): kernelbase.dll. Every Windows 10 process has it loaded (kernel32.dll is a thin layer over it), and the API set schema maps most of the core contracts (api-ms-win-core-string-l1-1-0 ...) to it, so a
     * LoadLibrary / GetModuleHandle of such a contract name gives its handle (fle_apiset_resolve). Same shape as N = 31, 32: the load succeeds, no response-table row has "kernelbase.dll" in its dll column, so GetProcAddress on it gives NULL and 127
     * (S-load2-host-exports). Appended after dwmapi.dll (decision 0015: append only). */
    "kernelbase.dll",
    /* N = 41, decision 0238: combase.dll. Windows 10 loads it into every process that uses COM (ole32.dll, which FLEngine imports, forwards to it), and the schema maps api-ms-win-core-winrt-l1-1 (which the CRT of oggio_x64.dll and Elastique_x64.dll asks
     * for with LoadLibraryExW to find RoInitialize / RoUninitialize) to it. Same shape as kernelbase.dll: the load succeeds, no response-table row has "combase.dll" in its dll column, so GetProcAddress on it gives NULL and 127 (S-load2-host-exports).
     * Appended after kernelbase.dll (decision 0015: append only). */
    "combase.dll",
};
#define FLE_MODULE_N (sizeof fle_modules / sizeof fle_modules[0])
/* The deferred modules (decision 0096) are the FLE_DEFERRED_N entries from index FLE_DEFERRED_FIRST (blend2d.dll, quickfontcache_x64.dll).
 * Names appended after them later (decision 0015: append only) leave the range as it is. */
#define FLE_DEFERRED_FIRST 32u
#define FLE_DEFERRED_N     2u
_Static_assert(sizeof fle_modules / sizeof fle_modules[0] >= FLE_DEFERRED_FIRST + FLE_DEFERRED_N, "fle_modules[]: the deferred modules (decision 0096) are missing");

uint64_t fle_module_count(void) { return FLE_MODULE_N; }

/* Decision 0238 (load2, PROVISIONAL): the API set schema of this environment. Learn "Windows API sets": "an API set name identifies a contract, not a file. At run time the loader resolves that contract through the API set schema on the current device, and
 * routes the reference to the DLL that hosts the implementation"; "The loader recognizes a name that begins with api- or ext- and routes it to the API set runtime ... the .dll suffix isn't part of the contract name being resolved"; "The operation succeeds when the
 * schema on the current device maps that contract to a usable host; there isn't necessarily an actual file with that name anywhere on the PC. If the contract isn't mapped on the current device, a direct LoadLibrary fails." The pages give no table of
 * contracts and hosts: the rows are Wine 10.0's dlls/apisetschema/apisetschema.spec (UNOFFICIAL; 511 contracts, three of them without a host), written by docs/investigations/load2/tools/mk-apiset.py. The lookup follows Wine 10.0 ntdll/loader.c get_apiset_entry
 * (UNOFFICIAL): the name must be longer than four characters and begin with "api-" or "ext-" (without case); the part before the last '-' that precedes the first '.' is the key, so l1-1-0 and l1-1-3 are the same entry. */
static const struct { const char *key; const char *host; } fle_apiset_schema[] = {
#include "fle_apiset_schema.inc"
};
#define FLE_APISET_N (sizeof fle_apiset_schema / sizeof fle_apiset_schema[0])

int fle_apiset_resolve(const char *name, const char **host)
{
    char key[264];   /* a module name is at most 256 characters + the extension (FLE_MODNAME_MAX below, fle_load.c LOAD_NAME_MAX) */
    size_t len = strlen(name), i, hl = 0, lo = 0, hi = FLE_APISET_N;

    *host = NULL;
    if (len <= 4 || (strncmp(name, "api-", 4) != 0 && strncmp(name, "ext-", 4) != 0)) return FLE_APISET_NOT_A_CONTRACT;
    for (i = 0; i < len; i++) {
        if (name[i] == '.') break;
        if (name[i] == '-') hl = i;
    }
    if (hl == 0 || hl >= sizeof key) return FLE_APISET_UNKNOWN;
    memcpy(key, name, hl);
    key[hl] = 0;
    while (lo < hi) {
        size_t mid = lo + (hi - lo) / 2;
        int c = strcmp(key, fle_apiset_schema[mid].key);
        if (c == 0) {
            if (fle_apiset_schema[mid].host[0] == 0) return FLE_APISET_NO_HOST;
            *host = fle_apiset_schema[mid].host;
            return FLE_APISET_HOST;
        }
        if (c < 0) hi = mid;
        else lo = mid + 1;
    }
    return FLE_APISET_UNKNOWN;
}

int fle_module_is_deferred(uint64_t index) { return index >= FLE_DEFERRED_FIRST && index < FLE_DEFERRED_FIRST + FLE_DEFERRED_N; }

const char *fle_module_name(uint64_t index) { return index < FLE_MODULE_N ? fle_modules[index] : NULL; }

uint64_t fle_module_handle(uint64_t index)
{
    return FLE_MODULE_HANDLE_BASE + FLE_MODULE_HANDLE_STEP * (index + 1);
}

/* kernel32!GetModuleHandleW: HMODULE GetModuleHandleW(LPCWSTR lpModuleName). lpModuleName in rcx
 * (R[1]). MSDN (GetModuleHandleW, Parameters and Return value):
 *  - NULL: the handle of the file used to create the calling process. Here FLE_EXE_HANDLE.
 *  - The name is compared without case. ASCII letters are folded; a name with any character above
 *    0x7F cannot equal a table name and is treated as not found (assumption: Windows' own folding
 *    of non-ASCII letters is not modelled).
 *  - A name without an extension gets ".dll" appended. A trailing '.' means "no extension": it is
 *    removed and nothing is appended. The page does not say what an empty string does; the rule
 *    above gives ".dll", which is not a module, so it fails with ERROR_MOD_NOT_FOUND (inference).
 *  - The name does not have to include a path. A name with a path (a '\\', '/' or ':') would be
 *    compared with the full paths of the loaded modules; the virtual modules have none, so the run
 *    stops (fail-closed) instead of guessing.
 *  - Success returns the handle and does not change the reference count (nothing is counted
 *    here). Failure returns NULL. The page names no error code; ERROR_MOD_NOT_FOUND (126) is the
 *    PM's decision (0015). A name longer than FLE_MODNAME_MAX characters is not defined by the page
 *    and stops the run. Success does not touch the last error (the page does not say it does). */
#define FLE_MODNAME_MAX 256u

/* The one body of GetModuleHandleW and GetModuleHandleA (decision 0091). wide != 0: lpModuleName is UTF-16 and a unit above 0x7F
 * never equals a table name. wide == 0: lpModuleName is ANSI and a byte above 0x7F stops the run (the page does not say how the
 * ANSI code page maps or folds it; decision 0091). Everything else, the messages included, is the same for both. */
static void get_module_handle(const char *api, int wide)
{
    uint64_t p = R[1];
    char name[FLE_MODNAME_MAX + 8];
    size_t n = 0;
    int has_dot = 0;
    uint64_t i;

    if (p == 0) {
        R[0] = FLE_EXE_HANDLE;
        return;
    }
    for (;;) {
        uint16_t c = wide ? (uint16_t)fle_ld(p + 2u * n, 2) : (uint16_t)(uint8_t)fle_ld(p + n, 1);
        if (c == 0) break;
        if (c == '\\' || c == '/' || c == ':') {
            fprintf(stderr, "fle: %s with a path is not supported (virtual modules have no path)\n", api);
            abort();
        }
        if (n >= FLE_MODNAME_MAX) {
            fprintf(stderr, "fle: %s name longer than %u characters is not supported\n", api, FLE_MODNAME_MAX);
            abort();
        }
        if (c > 0x7f) {
            if (!wide) {
                fprintf(stderr, "fle: %s with a byte above 0x7F (%#x) is not supported (the page does not define the ANSI code page folding; decision 0091)\n", api, (unsigned)c);
                abort();
            }
            name[n] = 1;
        } else {
            name[n] = (c >= 'A' && c <= 'Z') ? (char)(c - 'A' + 'a') : (char)c;
            if (c == '.') has_dot = 1;
        }
        n++;
    }
    if (n > 0 && name[n - 1] == '.') {
        n--;
    } else if (!has_dot) {
        memcpy(name + n, ".dll", 4);
        n += 4;
    }
    name[n] = 0;

    for (i = 0; i < FLE_MODULE_N; i++) {
        if (strcmp(name, fle_modules[i]) == 0) {
            /* a real module registered under a deferred module's name wins (decision 0096): answer as a real module below */
            if (fle_module_is_deferred(i) && FLE_REALMOD_PRESENT() && fle_realmod_find(name) >= 0) break;
            R[0] = fle_module_handle(i);
            return;
        }
    }
    if (FLE_REALMOD_PRESENT()) {   /* a real module (decision 0082): its base once loaded, else not found */
        int rm = fle_realmod_find(name);
        if (rm >= 0) {
            R[0] = fle_realmod_module_handle(api, rm);
            if (R[0] != 0) return;
        }
    }
    {   /* Decision 0238 (PROVISIONAL): an API set contract name that is not a module of the table is the loader's redirection to the host DLL: the handle of the host when the host is a loaded (virtual) module of this environment.
         * Learn "API set loader operation": "The loader performs a run-time redirection from that contract name to the host binary that contains the implementation"; the GetModuleHandle page names no case for a contract name, so the
         * redirection is read from Wine 10.0 (UNOFFICIAL: ntdll/loader.c LdrGetDllHandleEx -> find_dll_file -> find_apiset_dll gives the host's path, and the loaded module of that path is the answer). A contract whose host is not a loaded module of this
         * environment (or that the schema lacks or maps to no host) is "not loaded": NULL and 126, as for any other name that is not a module here. */
        const char *host;
        if (fle_apiset_resolve(name, &host) == FLE_APISET_HOST) {
            for (i = 0; i < FLE_MODULE_N; i++) {
                if (strcmp(host, fle_modules[i]) == 0) {
                    R[0] = fle_module_handle(i);
                    return;
                }
            }
        }
    }
    fle_set_last_error(FLE_ERROR_MOD_NOT_FOUND);
    R[0] = 0;
}

void fle_imp_GetModuleHandleW(void) { get_module_handle("GetModuleHandleW", 1); }

/* kernel32!GetModuleHandleA: HMODULE GetModuleHandleA(LPCSTR lpModuleName), decision 0091. MSDN (GetModuleHandleA) has one page for both
 * spellings and does not say that the rules differ, so the module table, the handles, the name rules, NULL, the failure (NULL and 126, the
 * PM's decision 0015), the real modules (decision 0082, including "not loaded" during the module's own DllMain) and the stops are those of
 * GetModuleHandleW above. The one difference is the ANSI byte: a byte above 0x7F is a stop here (the page does not define the code page
 * mapping or the case folding of it). The ASCII bytes of the name widen to the same units. */
void fle_imp_GetModuleHandleA(void) { get_module_handle("GetModuleHandleA", 0); }

/* kernel32!GetModuleHandleExW stops by name (decision 0078). The page's flags (FROM_ADDRESS, PIN, UNCHANGED_REFCOUNT) change the reference
 * count and the lookup by address, which the virtual module table does not model. The lifted dsp_ippv2_x64.dll calls it only from its
 * process-exit helper, to find "CorExitProcess" in mscoree.dll, and the page gives no answer for a module that is not loaded
 * beyond "the function fails" (no code). */
void fle_imp_GetModuleHandleExW(void)
{
    fprintf(stderr, "fle: GetModuleHandleExW: not answered (reached only by the exit helper of the lifted DLL; decision 0078)\n");
    abort();
}

/* Decision 0096 (provisional): the export functions of the deferred modules. GetProcAddress hands out the pseudo-VA of a response-table row
 * (src/imports_table.rs) and fle_dispatch of that pseudo-VA ends here: the module is installed but its code is not lifted yet, so there
 * is nothing to run and no value is made up. The stop names the module and the export and shows the four register arguments (rcx, rdx,
 * r8, r9 = R[1], R[2], R[8], R[9]) of the call. <module> is the name in fle_modules[] (lower case, with the extension). */
static void deferred_stop(const char *mod, const char *name) __attribute__((noreturn));
static void deferred_stop(const char *mod, const char *name)
{
    fprintf(stderr,
            "fle: %s!%s: the module is installed but not lifted yet (deferred, decision 0096; decision 0086 lifts it later) "
            "RCX=0x%llx RDX=0x%llx R8=0x%llx R9=0x%llx\n",
            mod, name, (unsigned long long)R[1], (unsigned long long)R[2], (unsigned long long)R[8], (unsigned long long)R[9]);
    abort();
}

#define FLE_DEFERRED_STOP(fn, mod, name) void fn(void) { deferred_stop(mod, name); }
/* Blend2D.dll: the one export (Blend2D investigation 4.1; the PE export directory has nothing else). */
FLE_DEFERRED_STOP(fle_imp_deferred_blend2d_AggCreateInstance, "blend2d.dll", "AggCreateInstance")
/* QuickFontCache_x64.dll: all ten exports, in the order of the export directory's ordinals (1..10). Eight are the names FLEngine passes to
 * GetProcAddress (QFC investigation 5.1); __dbk_fcall_wrapper and dbkFCallWrapperAddr are exports that FLEngine also has itself and does
 * not look up here. dbkFCallWrapperAddr is a variable in .bss, not code: it has a row like the others (decision 0096 lists every export). */
FLE_DEFERRED_STOP(fle_imp_deferred_qfc_dbkFCallWrapperAddr, "quickfontcache_x64.dll", "dbkFCallWrapperAddr")
FLE_DEFERRED_STOP(fle_imp_deferred_qfc___dbk_fcall_wrapper, "quickfontcache_x64.dll", "__dbk_fcall_wrapper")
FLE_DEFERRED_STOP(fle_imp_deferred_qfc_InitSharedPython, "quickfontcache_x64.dll", "InitSharedPython")
FLE_DEFERRED_STOP(fle_imp_deferred_qfc_CreateVST3Instance, "quickfontcache_x64.dll", "CreateVST3Instance")
FLE_DEFERRED_STOP(fle_imp_deferred_qfc_LoadSharedElastiqueDll, "quickfontcache_x64.dll", "LoadSharedElastiqueDll")
FLE_DEFERRED_STOP(fle_imp_deferred_qfc_AcquireSharedAnimTimer, "quickfontcache_x64.dll", "AcquireSharedAnimTimer")
FLE_DEFERRED_STOP(fle_imp_deferred_qfc_AcquireSharedThreadBank, "quickfontcache_x64.dll", "AcquireSharedThreadBank")
FLE_DEFERRED_STOP(fle_imp_deferred_qfc_FontCache_GetStats, "quickfontcache_x64.dll", "FontCache_GetStats")
FLE_DEFERRED_STOP(fle_imp_deferred_qfc_FontCache_CleanUp, "quickfontcache_x64.dll", "FontCache_CleanUp")
FLE_DEFERRED_STOP(fle_imp_deferred_qfc_FontCache_AcquireFont, "quickfontcache_x64.dll", "FontCache_AcquireFont")

/* The generator emits these when the closure reaches GetProcAddress (src/imports_table.rs).
 * Weak so that a link without them still resolves: every lookup then finds nothing. */
extern const fle_resp_entry fle_resp_table[] __attribute__((weak));
extern const uint64_t fle_resp_count __attribute__((weak));

/* kernel32!GetProcAddress: FARPROC GetProcAddress(HMODULE hModule, LPCSTR lpProcName). hModule in
 * rcx (R[1]), lpProcName in rdx (R[2], ANSI; f_4099c0 passes "GetLogicalProcessorInformation").
 * MSDN (GetProcAddress, Parameters, Return value and Remarks):
 *  - The name is compared with exact spelling and case (Remarks).
 *  - A value whose bits above the low 16 are zero is an ordinal, not a name. The page says "the
 *    high-order word must be zero"; reading that as the whole pointer above bit 15 is the usual
 *    IS_INTRESOURCE convention and is an inference for 64-bit. Ordinals are not implemented: the
 *    virtual modules have no export ordinals, so the run stops. NULL is such a value and stops too.
 *  - Success returns the address; failure returns NULL. A procedure the response table does not
 *    have is reported as missing (NULL, ERROR_PROC_NOT_FOUND 127): the runtime does not claim an
 *    API exists that it cannot answer (decision 0015). The page names no error code; the code is
 *    the PM's decision and equals the System Error Codes page.
 *  - hModule must be a handle from GetModuleHandleW. A value that is not one, and the handle of
 *    the lifted DLL itself (its exports are not modelled), are not defined by the page and stop
 *    the run. Success does not touch the last error. */
#define FLE_PROCNAME_MAX 256u

/* Export ordinals of the virtual modules (decision 0099, provisional). Learn has no ordinal tables, so a virtual module has none unless a row is
 * listed here, and an ordinal that is not listed stops the run (the runtime does not say that an export exists or does not exist). The 9 rows are
 * the ordinals that the guest passes to GetProcAddress(shell32.dll, ordinal) in unit 238; the names are those of Wine's shell32.spec (UNOFFICIAL,
 * not Learn; not checked against a real Windows 10 shell32.dll). An ordinal found here is looked up by its name in the response table, so it
 * answers exactly what a lookup by that name answers. */
static const struct { const char *mod; uint16_t ord; const char *name; } fle_ordinals[] = {
    { "shell32.dll", 16, "ILFindLastID" },
    { "shell32.dll", 17, "ILRemoveLastID" },
    { "shell32.dll", 18, "ILClone" },
    { "shell32.dll", 19, "ILCloneFirst" },
    { "shell32.dll", 21, "ILIsEqual" },
    { "shell32.dll", 25, "ILCombine" },
    { "shell32.dll", 152, "ILGetSize" },
    { "shell32.dll", 153, "ILGetNext" },
    { "shell32.dll", 155, "ILFree" },
};
#define FLE_ORDINAL_N (sizeof fle_ordinals / sizeof fle_ordinals[0])

/* The nine shell32 PIDL (ITEMIDLIST) functions of the table above. The runtime has no shell namespace and no ITEMIDLIST model, so each is a
 * response-table row (GetProcAddress gives the guest a pseudo-VA, not NULL) and stops by name when it is called (decision 0099; the same shape as
 * msctf!SetInputScope, decision 0055). ILFree is also a normal import of the lifted DLL (the IAT thunk 0x593410): the row makes that thunk a named
 * stop instead of "unsupported instruction". */
static void shell32_pidl_stop(const char *api) __attribute__((noreturn));
static void shell32_pidl_stop(const char *api)
{
    fprintf(stderr, "fle: shell32!%s: not answered: a shell PIDL (ITEMIDLIST) function, and the runtime has no shell namespace or ITEMIDLIST model (decision 0099)\n", api);
    abort();
}
#define FLE_PIDL_STUB(n) void fle_imp_##n(void) { shell32_pidl_stop(#n); }
FLE_PIDL_STUB(ILFindLastID)
FLE_PIDL_STUB(ILRemoveLastID)
FLE_PIDL_STUB(ILClone)
FLE_PIDL_STUB(ILCloneFirst)
FLE_PIDL_STUB(ILIsEqual)
FLE_PIDL_STUB(ILCombine)
FLE_PIDL_STUB(ILGetSize)
FLE_PIDL_STUB(ILGetNext)
/* Decision 0126 (wall 87, PROVISIONAL): ILFree frees an ITEMIDLIST of fle_ole.c (the Learn page: the same as CoTaskMemFree on Windows 2000 and later); a build without fle_ole.c still stops by name. */
void fle_ole_il_free(const char *api, uint64_t pidl) __attribute__((weak));
void fle_imp_ILFree(void)
{
    if (fle_ole_il_free == NULL) shell32_pidl_stop("ILFree");
    fle_ole_il_free("ILFree", R[1]);
}

void fle_imp_GetProcAddress(void)
{
    uint64_t h = R[1], np = R[2], idx, i, n;
    char name[FLE_PROCNAME_MAX + 1];
    const char *mod;
    const char *ord_name = NULL;

    if ((np >> 16) == 0) {
        if (np != 0 && h >= FLE_MODULE_HANDLE_BASE + FLE_MODULE_HANDLE_STEP &&
            (h - FLE_MODULE_HANDLE_BASE) % FLE_MODULE_HANDLE_STEP == 0 &&
            (h - FLE_MODULE_HANDLE_BASE) / FLE_MODULE_HANDLE_STEP <= FLE_MODULE_N) {
            const char *m = fle_modules[(h - FLE_MODULE_HANDLE_BASE) / FLE_MODULE_HANDLE_STEP - 1];
            for (i = 0; i < FLE_ORDINAL_N; i++)
                if (fle_ordinals[i].ord == np && strcmp(fle_ordinals[i].mod, m) == 0) ord_name = fle_ordinals[i].name;
            if (ord_name == NULL) {
                fprintf(stderr, "fle: GetProcAddress by ordinal (lpProcName %#llx) is not supported: %s has no export-ordinal row for it (decision 0099)\n",
                        (unsigned long long)np, m);
                abort();
            }
        } else {
            fprintf(stderr, "fle: GetProcAddress by ordinal (lpProcName %#llx) is not supported\n", (unsigned long long)np);
            abort();
        }
    }
    if (h == 0) h = FLE_EXE_HANDLE;
    if (h == FLE_EXE_HANDLE) {
        /* Decision 0233 (load1, PROVISIONAL): hModule NULL, and the handle of the process image (GetModuleHandleW(NULL), decision 0015: the lifted DLL). The GetProcAddress page names no case for NULL (hModule is "A handle to the DLL
         * module ... The LoadLibrary, LoadLibraryEx, LoadPackagedLibrary, or GetModuleHandle function returns this handle"; "If the function fails, the return value is NULL"). Wine 10.0 reads NULL as the process's own image
         * (UNOFFICIAL: dlls/kernelbase/loader.c get_proc_address line 72: `if (!module) module = NtCurrentTeb()->Peb->ImageBaseAddress;`), and a name that the image's export directory lacks fails with
         * STATUS_PROCEDURE_NOT_FOUND (dlls/ntdll/loader.c LdrGetProcedureAddress lines 2027-2058), which dlls/ntdll/error.h line 487 maps to ERROR_PROC_NOT_FOUND (127, the code this file gives for a name a module lacks). The export
         * directory of the image here is that of FLEngine_x64.dll: three names (read from the PE file, tables/flengine-exports.txt of decision 0233). A name that is not one of them fails (NULL, 127); one that is, or an
         * ordinal, stops as before (this runtime does not hand out the engine's own addresses). The engine reaches the NULL form when LoadLibraryW("Elastique_x64.dll") failed: it does not test the handle (it shows its own
         * "Elastique DLL couldn't be opened" box and goes on). */
        static const char *const own_exports[] = { "CreateFruityInstance", "__dbk_fcall_wrapper", "dbkFCallWrapperAddr" };
        if ((np >> 16) != 0) {
            char nm[32];
            size_t q, e;
            for (q = 0; q < sizeof nm; q++) {
                nm[q] = (char)fle_ld(np + q, 1);
                if (nm[q] == 0) break;
            }
            if (q < sizeof nm) {   /* no export is as long as the longest of the three, 20 characters */
                for (e = 0; e < sizeof own_exports / sizeof own_exports[0]; e++)
                    if (strcmp(nm, own_exports[e]) == 0) break;
                if (e == sizeof own_exports / sizeof own_exports[0]) {
                    fle_set_last_error(FLE_ERROR_PROC_NOT_FOUND);
                    R[0] = 0;
                    return;
                }
            } else {
                fle_set_last_error(FLE_ERROR_PROC_NOT_FOUND);   /* longer than 31 characters: none of the three */
                R[0] = 0;
                return;
            }
        }
        fprintf(stderr, "fle: GetProcAddress on the lifted DLL itself (hModule %#llx) for one of its own exports or by ordinal is not supported\n", (unsigned long long)h);
        abort();
    }
    if (FLE_REALMOD_PRESENT()) {   /* a real module (decision 0082): the export directory of its image, by name */
        int rm = fle_realmod_find_handle(h);
        if (rm >= 0) {
            char rname[FLE_PROCNAME_MAX + 1];
            for (n = 0;; n++) {
                if (n >= FLE_PROCNAME_MAX) {
                    fprintf(stderr, "fle: GetProcAddress name longer than %u characters is not supported\n", FLE_PROCNAME_MAX);
                    abort();
                }
                rname[n] = (char)fle_ld(np + n, 1);
                if (rname[n] == 0) break;
            }
            R[0] = fle_realmod_getproc("GetProcAddress", rm, rname);
            if (R[0] == 0) fle_set_last_error(FLE_ERROR_PROC_NOT_FOUND);
            return;
        }
    }
    if (h < FLE_MODULE_HANDLE_BASE + FLE_MODULE_HANDLE_STEP ||
        (h - FLE_MODULE_HANDLE_BASE) % FLE_MODULE_HANDLE_STEP != 0 ||
        (h - FLE_MODULE_HANDLE_BASE) / FLE_MODULE_HANDLE_STEP > FLE_MODULE_N) {
        fprintf(stderr, "fle: GetProcAddress with hModule %#llx that is not a virtual module handle is not supported\n",
                (unsigned long long)h);
        abort();
    }
    idx = (h - FLE_MODULE_HANDLE_BASE) / FLE_MODULE_HANDLE_STEP - 1;
    mod = fle_modules[idx];

    if (ord_name != NULL) {   /* an ordinal of fle_ordinals[]: np is not a pointer, the name is the table's */
        strcpy(name, ord_name);
    } else {
        for (n = 0;; n++) {
            if (n >= FLE_PROCNAME_MAX) {
                fprintf(stderr, "fle: GetProcAddress name longer than %u characters is not supported\n", FLE_PROCNAME_MAX);
                abort();
            }
            name[n] = (char)fle_ld(np + n, 1);
            if (name[n] == 0) break;
        }
    }

    if (fle_resp_table != NULL) {
        for (i = 0; i < fle_resp_count; i++) {
            if (strcmp(fle_resp_table[i].dll, mod) == 0 && strcmp(fle_resp_table[i].name, name) == 0) {
                R[0] = fle_resp_table[i].va;
                return;
            }
        }
    }
    fle_set_last_error(FLE_ERROR_PROC_NOT_FOUND);
    R[0] = 0;
}

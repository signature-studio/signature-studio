#include <stdint.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_modules.h"
#include "fle_load.h"
#include "fle_realmod.h"
#include "fle_vfs.h"

/* Library-loader answers (docs/decisions/0029-apis-a6-a12.md) over the virtual module table of fle_modules.c
 * (decision 0015). Pages: Microsoft Learn LoadLibraryW, LoadLibraryExW, FreeLibrary, SetDllDirectoryW (URLs in
 * the decision). A page that does not define a case leaves it to fle_stop: the run ends with "fle: <api>: <why>".
 *
 * What a module name resolves to (the same name rules as GetModuleHandleW, decision 0015):
 *  1. a name in the virtual module table (the PE import DLLs, ntdll.dll and msctf.dll, decision 0055): the handle that GetModuleHandleW
 *     gives, and one more reference. Page: "If the string specifies a module name without a path ... the function
 *     searches for loaded modules whose base name matches ... If the name matches, the load succeeds", and "Calling
 *     LoadLibrary increments the reference count". A module that the process loaded at start has count one
 *     (FreeLibrary page);
 *  2. a name in fle_load_absent[] below: not loaded, and the virtual file system (decision 0018) holds no module
 *     file, so the search ends without a module. Return NULL, last error ERROR_MOD_NOT_FOUND (126, System Error
 *     Codes page; the LoadLibrary pages name no code, so the value is the same decision as for GetModuleHandleW);
 *  2b. an API set contract name ("api-" / "ext-" prefix; decision 0238, api_set_load below): the loader's redirection to the host DLL of the schema (fle_apiset_resolve): the host's handle and one more reference when the host is a virtual module;
 *     a contract that the schema maps to no host fails (NULL, ERROR_MOD_NOT_FOUND); a host that is not a virtual module of this environment stops by name; a contract that the schema lacks goes on like any other bare name (3.);
 *  3. anything else (the DLLs of FLEngine's own plug-ins and tools and any name nobody has measured): the standard search over the virtual volume (decision 0233, search_volume below). No handle is ever
 *     invented: a file of the volume that cannot run stops by name; a search that finds nothing fails like the pages say (NULL, ERROR_MOD_NOT_FOUND). (Before decision 0233 this case stopped, and decision
 *     0209's soft stop or decision 0178's FLE_LOADLIB_MISSING answered it in an exploration.)
 * A name with a path ('\\', '/' or ':') is load_with_path() below (decision 0233): the search of a full path, or of a relative path appended to
 * the directories of the search order, ends without a module when the virtual volume holds no file there. A name longer than
 * LOAD_NAME_MAX characters, an empty name and a NULL pointer are not defined by the pages and stop. */

#define LOAD_NAME_MAX    256u
#define LOAD_DIR_MAX     32767u
#define LOAD_MODULES_MAX 64u

/* LoadLibraryExW dwFlags (LoadLibraryExW page). Only the values below are answered. */
#define LLEX_AS_DATAFILE            0x00000002u
#define LLEX_AS_IMAGE_RESOURCE      0x00000020u
#define LLEX_AS_DATAFILE_EXCLUSIVE  0x00000040u
#define LLEX_SEARCH_APPLICATION_DIR 0x00000200u
#define LLEX_SEARCH_USER_DIRS       0x00000400u
#define LLEX_SEARCH_SYSTEM32        0x00000800u
#define LLEX_SEARCH_DEFAULT_DIRS    0x00001000u
#define LLEX_MODELLED (LLEX_AS_DATAFILE | LLEX_AS_IMAGE_RESOURCE | LLEX_AS_DATAFILE_EXCLUSIVE | \
                       LLEX_SEARCH_APPLICATION_DIR | LLEX_SEARCH_USER_DIRS | LLEX_SEARCH_SYSTEM32 | \
                       LLEX_SEARCH_DEFAULT_DIRS)

/* Names (lower-case, with extension) that are not loaded in the virtual process and that this environment is
 * defined to lack: exactly the nine DLLs that the PE only delay-loads, which decision 0015 (PM) already says are
 * "not loaded" (GetModuleHandleW gives 126 for them): dwmapi, imm32, msimg32, propsys, rpcrt4, shcore, uxtheme,
 * windowscodecs, wtsapi32. Nothing else is added here: a Windows DLL that decision 0015 does not name (ws2_32,
 * mswsock, psapi, riched20, urlmon, ...) and every plug-in or tool DLL stops the run (case 3 above), because a
 * "does not exist" answer for it would be a claim about the environment that nobody has decided.
 * (Decision 0088 made avrt.dll and powrprof.dll virtual modules, N = 31, 32; the other ten names it lists, these
 * among them, still stop by name. Decision 0099 made shcore.dll virtual module N = 35 and took it out of this list: the guest's
 * delay-load of shcore!GetProcessDpiAwareness raised 0xc06d007e while shcore.dll was absent; eight names remained. Decision 0118 wall 7 made rpcrt4.dll
 * virtual module N = 36 for the delay-load of rpcrt4!UuidCreate (CreateFruityInstance) and took it out of this list; seven names remained. Decision 0119 wall 11 made wtsapi32.dll
 * virtual module N = 37 for the delay-load of wtsapi32!WTSRegisterSessionNotification (the TApplication window) and took it out of this list; six names remained. Decision 0119 wall 12 made uxtheme.dll
 * virtual module N = 38 for the delay-load of uxtheme!BufferedPaintInit (the TApplication window) and took it out of this list; five names remain. Decision 0122 made dwmapi.dll virtual module
 * N = 39 for QuickFontCache's TTimerThread (DwmFlush) and took it out of this list; four names remain.
 * Decision 0126 wall 86 (PROVISIONAL, S-webview2-absent, a CHOICE WITHOUT A SOURCE): "webview2loader.dll" is added. FLEngine's f_be3520 calls LoadLibraryW(L"WebView2Loader.dll") and returns 0 when it gets NULL
 * (docs/investigations/loadlibrary-names-classification.md #22: "NULL を許容"); the installer ships the DLL (bundled-dll-inventory.md) but the virtual file system holds no module file for it, so "the search ends without a
 * module" (case 2): the alternative is to load it and answer CreateCoreWebView2Environment and the WebView2 COM interfaces, a COM / network model ((d') of decision 0126). The claim "this machine does not find
 * WebView2Loader.dll" differs from a real installation (where the loader is in the application directory). Withdrawn when a module lane (M2) loads the product's DLLs or a WebView2 model exists.) */
static const char *const fle_load_absent[] = {
    "imm32.dll", "msimg32.dll", "propsys.dll",
    "windowscodecs.dll", "webview2loader.dll",
};
#define LOAD_ABSENT_N (sizeof fle_load_absent / sizeof fle_load_absent[0])

/* Decision 0233 (load1, PROVISIONAL, UNOFFICIAL roster): the names of the DLLs that a Windows installation ships (fle_win_dll_roster.inc: the directory names of dlls/ in Wine 10.0 and the ten names of decision 0015's investigation).
 * A bare name that is on the roster and is not a virtual module of this environment is a DLL that exists on a real Windows, so the answer "not found" would be untrue for it, and no
 * handle is invented either: the loader stops by name, as it did before decision 0233. A name that is not on the roster (a product's or a third party's DLL) is answered by the search of the virtual volume. The roster is an input
 * of the environment (data), not a branch on a name of the product. Sorted by strcmp (tools/mk-roster.py writes it so; tests/load.rs checks). */
static const char *const fle_win_dlls[] = {
#include "fle_win_dll_roster.inc"
};
#define WIN_DLLS_N (sizeof fle_win_dlls / sizeof fle_win_dlls[0])

static int on_roster(const char *name)
{
    size_t lo = 0, hi = WIN_DLLS_N;
    while (lo < hi) {
        size_t mid = lo + (hi - lo) / 2;
        int c = strcmp(name, fle_win_dlls[mid]);
        if (c == 0) return 1;
        if (c < 0) hi = mid;
        else lo = mid + 1;
    }
    return 0;
}

static uint32_t refs[LOAD_MODULES_MAX];
static int refs_ready;
static uint16_t *dll_dir;

static void refs_init(const char *api)
{
    uint64_t i, n = fle_module_count();
    if (refs_ready) return;
    if (n > LOAD_MODULES_MAX) fle_stop(api, "the virtual module table is larger than the loader's reference table");
    for (i = 0; i < n; i++) refs[i] = 1;
    refs_ready = 1;
}

uint32_t fle_load_refcount(uint64_t index)
{
    refs_init("fle_load_refcount");
    return index < fle_module_count() ? refs[index] : 0;
}

const uint16_t *fle_load_dll_directory(void) { return dll_dir; }

/* Turns the name (n code units, any above 0x7F becomes '?', which no table name has) into the table spelling:
 * ASCII letters folded to lower case, ".dll" appended when there is no '.', a trailing '.' removed. */
extern int fle_ia2_path_absent(const uint16_t *u, size_t n) __attribute__((weak));   /* decision 0191: fle_wall_ia2.c; weak: a link without it is as before */

static void normalize(const char *api, const uint16_t *u, size_t n, char *out)
{
    size_t i, len = 0;
    int has_dot = 0;

    for (i = 0; i < n; i++) {
        uint16_t c = u[i];
        if (c == '\\' || c == '/' || c == ':') fle_stop(api, "internal: a module name with a path reached the name rules (load_with_path takes it)");
        if (c > 0x7f) {
            out[len++] = '?';
        } else {
            out[len++] = (c >= 'A' && c <= 'Z') ? (char)(c - 'A' + 'a') : (char)c;
            if (c == '.') has_dot = 1;
        }
    }
    if (len > 0 && out[len - 1] == '.') {
        len--;
    } else if (!has_dot) {
        memcpy(out + len, ".dll", 4);
        len += 4;
    }
    out[len] = 0;
}

static void read_wide_name(const char *api, uint64_t p, uint16_t *buf, size_t *n)
{
    size_t k;
    uint16_t c;
    if (p == 0) fle_stop(api, "lpLibFileName is NULL (the page does not define it)");
    for (k = 0;; k++) {
        c = (uint16_t)fle_ld(p + 2u * k, 2);
        if (c == 0) break;
        if (k >= LOAD_NAME_MAX) fle_stop(api, "a module name longer than 256 characters is not supported");
        buf[k] = c;
    }
    buf[k] = 0;
    *n = k;
}

static void read_ansi_name(const char *api, uint64_t p, uint16_t *buf, size_t *n)
{
    size_t k;
    uint8_t c;
    if (p == 0) fle_stop(api, "lpLibFileName is NULL (the page does not define it)");
    for (k = 0;; k++) {
        c = (uint8_t)fle_ld(p + k, 1);
        if (c == 0) break;
        if (k >= LOAD_NAME_MAX) fle_stop(api, "a module name longer than 256 characters is not supported");
        buf[k] = c;
    }
    buf[k] = 0;
    *n = k;
}

/* Decision 0233 (load1, PROVISIONAL): a module name WITH A PATH ('\\', '/' or ':'). Pages (Microsoft Learn): LoadLibraryW "If the string specifies a full path, the function searches only that path for the module. If the string
 * specifies a relative path or a module name without a path, the function uses a standard search strategy ... If the function cannot find the module, the function fails" and "If the function fails, the return value is
 * NULL. To get extended error information, call GetLastError"; LoadLibraryExW "If lpFileName specifies a relative path, the entire relative path is appended to every token in the DLL search path list" and "If the
 * function cannot find the module or one of its dependencies, the function fails"; "Dynamic-link library search order" (the folders of the standard order: the application folder, the system folder, the 16-bit system folder,
 * the Windows folder, the current folder, PATH; SetDllDirectory's folder). No page names the error code. ERROR_MOD_NOT_FOUND (126) is the code this file gives for every module that is not there (decision 0015); Wine 10.0 gives
 * the same for a file or a folder that does not exist (UNOFFICIAL: ntdll/loader.c open_dll_file turns STATUS_OBJECT_PATH_NOT_FOUND and STATUS_OBJECT_NAME_NOT_FOUND into STATUS_DLL_NOT_FOUND, ntdll/error.h maps
 * c0000135 to ERROR_MOD_NOT_FOUND; S-load1-code). What is searched is the virtual volume (decision 0018), which holds no module file unless a build puts one there:
 *  1. the link's path table (decision 0183, opt-in) maps the path to a real module: the file must be on the volume, else NULL / 126; on the volume the real module loads (DllMain runs before this returns);
 *  2. a file of the volume at the full path (or at the relative path under a directory of the search order) is a module file that this run cannot load: native code that was not lifted into the build does not run, and no
 *     handle is invented for it: stop by name (the boundary (b) of decision 0233);
 *  3. the base name is a virtual module (kernel32.dll ...): the module has no path in this environment (fle_modules.c), so neither a handle nor a failure is true: stop by name;
 *  4. otherwise the search ends without a module: NULL, ERROR_MOD_NOT_FOUND (the boundary (a)). A full path on another drive than C: ends the same way (the volume has the drive C: only, decision 0018).
 * Forms that the volume does not model stop by name as they did: a UNC or device prefix, a drive-relative name (C:name), a ':' elsewhere. */
extern int fle_vfs_peek_file(const uint16_t *wpath, const uint8_t **data, uint64_t *size) __attribute__((weak));

#define LOAD_PATH_MAX (LOAD_NAME_MAX + 96u)

static int is_sep_u(uint16_t c) { return c == '\\' || c == '/'; }

static int name_has_path(const uint16_t *u, size_t n)
{
    size_t i;
    for (i = 0; i < n; i++)
        if (u[i] == '\\' || u[i] == '/' || u[i] == ':') return 1;
    return 0;
}

/* 1 when the volume holds a FILE at the NUL-terminated UTF-16 path w (a relative one is the current directory's). fle_vfs.c may stop by name for a name form that it does not model (a reserved character, a device name). */
static int volume_file(const char *api, const uint16_t *w)
{
    const uint8_t *d;
    uint64_t sz;
    (void)api;
    if (!fle_vfs_peek_file) return 0;   /* a link without fle_vfs.c (the unit tests of the loader) has no volume, so no file */
    return fle_vfs_peek_file(w, &d, &sz);
}

/* dir (ASCII, no trailing backslash) + '\\' + rel -> out (NUL-terminated); 0 when it does not fit */
static int join_ascii_dir(uint16_t *out, const char *dir, const uint16_t *rel, size_t rn)
{
    size_t dl = strlen(dir), i;
    if (dl + 1 + rn + 1 > LOAD_PATH_MAX) return 0;
    for (i = 0; i < dl; i++) out[i] = (uint8_t)dir[i];
    out[dl] = '\\';
    memcpy(out + dl + 1, rel, rn * sizeof *rel);
    out[dl + 1 + rn] = 0;
    return 1;
}

static void ascii_of(const uint16_t *w, char *out, size_t cap)
{
    size_t i;
    for (i = 0; w[i] && i + 1 < cap; i++) out[i] = w[i] < 0x80 ? (char)w[i] : '?';
    out[i] = 0;
}

/* The search of the virtual volume (decision 0233). rel is the NUL-terminated name as the page searches it: a full path (full != 0) is looked at alone; a relative path or a bare name (with the extension rule applied) is
 * looked at in the current directory and appended to the folders of the standard search order that this environment has: the application folder (the folder of FLE_VIRTUAL_EXE_PATH), the system folder, the Windows folder,
 * and SetDllDirectoryW's folder when it is a full path on C:. (The 16-bit system folder and PATH name folders that the volume does not have.) Returns 1 with `shown` set to the path of a file that was found. */
static int search_volume(const char *api, const uint16_t *rel, size_t rn, int full, char *shown, size_t cap)
{
    uint16_t cand[LOAD_PATH_MAX + 1];
    static const char *const dirs[] = { FLE_VIRTUAL_SYSTEM_DIR, FLE_VIRTUAL_WINDOWS_DIR };
    char appdir[sizeof FLE_VIRTUAL_EXE_PATH];
    const uint16_t *dd;
    char *ad;
    size_t k;

    if (volume_file(api, rel)) { ascii_of(rel, shown, cap); return 1; }
    if (full) return 0;
    strcpy(appdir, FLE_VIRTUAL_EXE_PATH);
    ad = strrchr(appdir, '\\');
    if (ad) *ad = 0;
    if (join_ascii_dir(cand, appdir, rel, rn) && volume_file(api, cand)) { ascii_of(cand, shown, cap); return 1; }
    for (k = 0; k < sizeof dirs / sizeof dirs[0]; k++)
        if (join_ascii_dir(cand, dirs[k], rel, rn) && volume_file(api, cand)) { ascii_of(cand, shown, cap); return 1; }
    dd = fle_load_dll_directory();
    if (dd && dd[0] && dd[1] == ':' && (dd[0] == 'C' || dd[0] == 'c')) {
        size_t dn = fle_w_len(dd);
        if (dn + 1 + rn + 1 <= LOAD_PATH_MAX) {
            memcpy(cand, dd, dn * sizeof *dd);
            cand[dn] = '\\';
            memcpy(cand + dn + 1, rel, rn * sizeof *rel);
            cand[dn + 1 + rn] = 0;
            if (volume_file(api, cand)) { ascii_of(cand, shown, cap); return 1; }
        }
    }
    return 0;
}

/* the volume holds a module file that this build cannot run (the boundary (b)) */
static void stop_file_not_a_module(const char *api, const char *shown)
{
    char why[LOAD_PATH_MAX + 240];
    snprintf(why, sizeof why,
             "the virtual volume holds a file at \"%s\" but it is not a module of this build (native code that was not lifted does not run; no handle and no failure is invented for it; decision 0233)", shown);
    fle_stop(api, why);
}

/* 1 when the folder part of u (the units before `base`, the position after the last separator) is C:\Windows, C:\Windows\System32 or C:\Windows\SysWOW64 (the folders where a Windows installation keeps its DLLs; without case, '/' as '\') */
static int in_windows_dir(const uint16_t *u, size_t base)
{
    static const char *const dirs[] = { FLE_VIRTUAL_WINDOWS_DIR, FLE_VIRTUAL_SYSTEM_DIR, "C:\\Windows\\SysWOW64" };
    size_t k, i, dl = base;
    if (dl > 0 && is_sep_u(u[dl - 1])) dl--;
    for (k = 0; k < sizeof dirs / sizeof dirs[0]; k++) {
        size_t l = strlen(dirs[k]);
        if (l != dl) continue;
        for (i = 0; i < l; i++) {
            uint16_t c = u[i], e = (uint8_t)dirs[k][i];
            if (c >= 'A' && c <= 'Z') c = (uint16_t)(c + 32);
            if (e >= 'A' && e <= 'Z') e = (uint16_t)(e + 32);
            if (c == '/') c = '\\';
            if (c != e) break;
        }
        if (i == l) return 1;
    }
    return 0;
}

/* a name with a path: see the comment above */
static void load_with_path(const char *api, const uint16_t *u, size_t n)
{
    char name[LOAD_NAME_MAX + 8], shown[LOAD_PATH_MAX + 1], why[LOAD_PATH_MAX + 240];
    int full = 0, rooted_other = 0, rm;
    size_t i, base = 0;

    if (n >= 2 && u[1] == ':') {
        if (!((u[0] >= 'A' && u[0] <= 'Z') || (u[0] >= 'a' && u[0] <= 'z'))) fle_stop(api, "a module name with ':' in a form the volume does not model");
        if (n < 3 || !is_sep_u(u[2])) fle_stop(api, "a drive-relative module name (C:name) is not modelled");
        full = 1;
        rooted_other = (u[0] != 'C' && u[0] != 'c');
        i = 2;
    } else if (is_sep_u(u[0])) {
        if (n >= 2 && is_sep_u(u[1])) fle_stop(api, "a UNC or device module path (\\\\...) is not modelled");
        full = 1;
        i = 1;
    } else {
        i = 0;
    }
    for (; i < n; i++)
        if (u[i] == ':') fle_stop(api, "a module name with ':' in a form the volume does not model");
    for (i = 0; i < n; i++)
        if (is_sep_u(u[i])) base = i + 1;
    refs_init(api);

    if (FLE_REALMOD_PRESENT()) {   /* 1. the link's path table (decision 0183) */
        rm = fle_realmod_find_path(u, n);
        if (rm >= 0) {
            if (!fle_vfs_peek_file) fle_stop(api, "a path names a real module but fle_vfs.c is not linked (no virtual volume to check the file in)");
            if (!volume_file(api, u)) {
                fle_set_last_error(FLE_ERROR_MOD_NOT_FOUND);
                R[0] = 0;
                return;
            }
            R[0] = fle_realmod_load(api, rm);
            return;
        }
    }
    if (!rooted_other && search_volume(api, u, n, full, shown, sizeof shown)) stop_file_not_a_module(api, shown);   /* 2. */
    {   /* 3. the base name is a virtual module or a DLL on the roster, in a folder where Windows keeps its DLLs */
        size_t bn = n - base;
        if (bn > 0 && bn <= LOAD_NAME_MAX && in_windows_dir(u, base)) {
            normalize(api, u + base, bn, name);
            if (on_roster(name)) {
                snprintf(why, sizeof why, "a path to \"%s\", a DLL that a Windows installation ships (the roster of decision 0233), in a folder of Windows: this environment holds no such file and has no module for it, so neither a handle nor a failure is true (decision 0233)", name);
                fle_stop(api, why);
            }
            for (i = 0; i < fle_module_count(); i++)
                if (strcmp(name, fle_module_name(i)) == 0) {
                    snprintf(why, sizeof why, "a path to the virtual module \"%s\": the virtual modules have no path in this environment, so neither a handle nor a failure is true for it (decision 0233)", name);
                    fle_stop(api, why);
                }
        }
    }
    fle_set_last_error(FLE_ERROR_MOD_NOT_FOUND);   /* 4. */
    R[0] = 0;
}

/* Decision 0238 (load2, PROVISIONAL): an API set contract name that is not itself a module of the table. Learn "Windows API sets" / "API set loader operation": "The loader recognizes a name that begins with api- or ext- and routes it to the API set
 * runtime, an extension of the loader that resolves contracts through the schema"; "1. Your binary imports an API set contract name, or passes one to LoadLibrary. 2. The loader looks the contract up in the API set schema on the current device, and finds
 * the host binary that the schema maps it to. 3. The loader loads that host binary"; "A device that doesn't include the feature: Not mapped"; "If the contract isn't mapped on the current device, a direct LoadLibrary fails". So the handle that LoadLibrary
 * gives for a contract is the host's, with one more reference on the host (Wine 10.0, UNOFFICIAL: ntdll/loader.c load_dll -> find_dll_file -> find_apiset_dll replaces the name by the host's path and the module of that path is found / loaded; the schema
 * has no sub-version in its key). Returns 1 when it answered (R[0], the last error), 0 when the name is not a contract of the schema (an "api-" name that the schema lacks is searched as a file like any other bare name, as Wine 10.0 does: find_apiset_dll
 * gives STATUS_APISET_NOT_PRESENT and the file search goes on). A contract that is in the schema and maps to no host fails like an unmapped one: NULL and ERROR_MOD_NOT_FOUND (126 is this file's code for a module that is not there, decision 0015; the pages name
 * none; Wine 10.0 get_apiset_target gives STATUS_DLL_NOT_FOUND, which ntdll/error.h maps to 126). A host that is not a virtual module of this environment stops by name: nobody has said whether that DLL is loaded or what it exports, so neither a handle
 * nor a failure is true. The host's exports are those of the response table under the host's name (GetProcAddress, fle_modules.c). */
static int api_set_load(const char *api, const char *name)
{
    const char *host;
    uint64_t i;
    switch (fle_apiset_resolve(name, &host)) {
    case FLE_APISET_HOST:
        for (i = 0; i < fle_module_count(); i++) {
            if (strcmp(host, fle_module_name(i)) == 0) {
                refs[i]++;
                R[0] = fle_module_handle(i);
                return 1;
            }
        }
        {
            char why[LOAD_NAME_MAX + 300];
            snprintf(why, sizeof why,
                     "the API set contract \"%s\" is mapped to the host \"%s\" (the schema of decision 0238, Wine 10.0, UNOFFICIAL) and this environment has no virtual module for that host, so neither a handle nor a failure is true for it",
                     name, host);
            fle_stop(api, why);
        }
        return 1;
    case FLE_APISET_NO_HOST:
        fle_set_last_error(FLE_ERROR_MOD_NOT_FOUND);
        R[0] = 0;
        return 1;
    default:
        return 0;
    }
}

/* LoadLibrary after the arguments are read: table name -> handle and one more reference; absent name -> NULL and
 * 126; a name with a path -> load_with_path; any other name: the search of the volume (decision 0233), which fails with NULL and 126 or, for a file that is there but cannot run, stops by name. */
static void load_by_name(const char *api, const uint16_t *u, size_t n)
{
    char name[LOAD_NAME_MAX + 8];
    uint64_t i;

    if (n == 0) {
        /* Decision 0229 (ref1, PROVISIONAL; replaces the exploration switch FLE_LOADLIB_MISSING of decision 0178 for this case): an empty module name names no module, so "the function cannot find the module", and the
         * pages of LoadLibraryW / LoadLibraryExW say what happens then: "If the function cannot find the module, the function fails" and "If the function fails, the return value is NULL". The pages name no error
         * code; ERROR_MOD_NOT_FOUND (126, "The specified module could not be found") is the code this file gives for every module that is not there (S-ref1-loadlib-empty-code: ERROR_INVALID_PARAMETER (87) is the other
         * candidate; a real machine's code for an empty name was not read). The engine asks for an empty name after the file of a plugin DLL was not found on the volume (46 times while the reference project loads). */
        fle_set_last_error(FLE_ERROR_MOD_NOT_FOUND);
        R[0] = 0;
        return;
    }
    if (fle_ia2_path_absent != NULL && fle_ia2_path_absent(u, n)) {   /* decision 0191 (opt-in: the layer-4 marker row), wall ia2-W21: see fle_wall_ia2.c */
        fle_set_last_error(FLE_ERROR_MOD_NOT_FOUND);
        R[0] = 0;
        return;
    }
    if (name_has_path(u, n)) {   /* decision 0233 */
        load_with_path(api, u, n);
        return;
    }
    normalize(api, u, n, name);
    refs_init(api);
    if (FLE_REALMOD_PRESENT()) {   /* a real module (fle_realmod.h, decision 0082): DllMain runs before this returns */
        int rm = fle_realmod_find(name);
        if (rm >= 0) {
            R[0] = fle_realmod_load(api, rm);
            return;
        }
    }
    for (i = 0; i < fle_module_count(); i++) {
        if (strcmp(name, fle_module_name(i)) == 0) {
            refs[i]++;
            R[0] = fle_module_handle(i);
            return;
        }
    }
    for (i = 0; i < LOAD_ABSENT_N; i++) {
        if (strcmp(name, fle_load_absent[i]) == 0) {
            fle_set_last_error(FLE_ERROR_MOD_NOT_FOUND);
            R[0] = 0;
            return;
        }
    }
    if (api_set_load(api, name)) return;   /* decision 0238 */
    if (on_roster(name)) {   /* decision 0233: a DLL that Windows ships (roster), that this environment does not model */
        char why[LOAD_NAME_MAX + 260];
        snprintf(why, sizeof why,
                 "module \"%s\" is not a virtual module and not a name this environment is defined to lack: it is a DLL that a Windows installation ships (the roster of decision 0233), so \"not found\" would be untrue "
                 "and no handle is invented for it",
                 name);
        fle_stop(api, why);
    }
    {   /* decision 0233: no module of the build claims the bare name, so "the function uses a standard search strategy" over the volume (search_volume): a file of the volume that this run cannot load stops by name,
         * and a search that ends without a module fails ("If the function cannot find the module, the function fails"; NULL and ERROR_MOD_NOT_FOUND). Replaces decision 0178's exploration switch FLE_LOADLIB_MISSING and the
         * soft stop of decision 0209 at this place: the failure is the page's, not an exploration. */
        uint16_t rel[LOAD_NAME_MAX + 8];
        char shown[LOAD_PATH_MAX + 1];
        size_t rn = n, k;
        int dot = 0;
        memcpy(rel, u, n * sizeof *u);
        for (k = 0; k < n; k++)
            if (u[k] == '.') dot = 1;
        if (n > 0 && u[n - 1] == '.') {
            rn--;
        } else if (!dot) {
            static const uint16_t ext[4] = { '.', 'd', 'l', 'l' };
            memcpy(rel + n, ext, sizeof ext);
            rn += 4;
        }
        rel[rn] = 0;
        if (search_volume(api, rel, rn, 0, shown, sizeof shown)) stop_file_not_a_module(api, shown);
        fle_set_last_error(FLE_ERROR_MOD_NOT_FOUND);
        R[0] = 0;
    }
}

/* kernel32!LoadLibraryW: lpLibFileName in rcx. Success: the module handle, last error untouched (the page does not
 * say it is cleared). Failure: NULL (page) and ERROR_MOD_NOT_FOUND (see above). */
void fle_imp_LoadLibraryW(void)
{
    uint16_t buf[LOAD_NAME_MAX + 1];
    size_t n;
    read_wide_name("LoadLibraryW", R[1], buf, &n);
    load_by_name("LoadLibraryW", buf, n);
}

/* kernel32!LoadLibraryA: the same with an ANSI name. A byte above 0x7F cannot equal a table name. */
void fle_imp_LoadLibraryA(void)
{
    uint16_t buf[LOAD_NAME_MAX + 1];
    size_t n;
    size_t i;
    read_ansi_name("LoadLibraryA", R[1], buf, &n);
    if (name_has_path(buf, n))
        for (i = 0; i < n; i++)
            if (buf[i] > 0x7f) fle_stop("LoadLibraryA", "a module path with a byte above 0x7F (the page does not define the ANSI code page; decision 0233)");
    load_by_name("LoadLibraryA", buf, n);
}

/* kernel32!LoadLibraryExW: lpLibFileName rcx, hFile rdx, dwFlags r8d.
 *  - hFile "is reserved for future use. It must be NULL": anything else stops.
 *  - dwFlags 0 is "identical to LoadLibrary". The data and image flags (0x2, 0x20, 0x40): for a module that is
 *    already loaded, the page says LoadLibraryEx "returns a handle to the DLL and increments the DLL reference
 *    count", so a table name gives the same handle and one more reference. The search flags (0x200, 0x400, 0x800,
 *    0x1000) steer the file search, which the page says happens when "a module of the same name is not already
 *    loaded"; for a loaded module they change nothing (inference from that sentence). The page forbids
 *    LOAD_LIBRARY_AS_DATAFILE with ..._EXCLUSIVE in one call (Security Remarks).
 *  - Any other bit (DONT_RESOLVE_DLL_REFERENCES 0x1, LOAD_WITH_ALTERED_SEARCH_PATH 0x8, IGNORE_CODE_AUTHZ_LEVEL
 *    0x10, SEARCH_DLL_LOAD_DIR 0x100, REQUIRE_SIGNED_TARGET 0x80, SAFE_CURRENT_DIRS 0x2000, unknown bits) stops:
 *    the page defines them for a file load, not for a loaded virtual module. */
void fle_imp_LoadLibraryExW(void)
{
    static const char api[] = "LoadLibraryExW";
    uint16_t buf[LOAD_NAME_MAX + 1];
    size_t n;
    uint32_t flags = (uint32_t)R[8];

    if (R[2] != 0) fle_stop(api, "hFile is not NULL (the page: reserved, must be NULL)");
    if (flags & ~LLEX_MODELLED) fle_stop(api, "dwFlags has a bit that is not modelled (see fle_load.c)");
    if ((flags & LLEX_AS_DATAFILE) && (flags & LLEX_AS_DATAFILE_EXCLUSIVE))
        fle_stop(api, "LOAD_LIBRARY_AS_DATAFILE with LOAD_LIBRARY_AS_DATAFILE_EXCLUSIVE (the page says not to)");
    read_wide_name(api, R[1], buf, &n);
    load_by_name(api, buf, n);
}

/* kernel32!FreeLibrary: hLibModule in rcx. A virtual module handle with a count above one: the count goes down and
 * the result is nonzero, last error untouched. The page says the module is unloaded when the count reaches zero;
 * unloading a module that the virtual process itself imports is not modelled, so a count of one stops (that is
 * also the case of a handle taken from GetModuleHandleW, which the page warns about). NULL, the lifted DLL's own
 * handle (decision 0015: 0x400000) and a value that is not a module handle are not defined by the page (it names no
 * error code) and stop. */
void fle_imp_FreeLibrary(void)
{
    static const char api[] = "FreeLibrary";
    uint64_t h = R[1], idx;

    refs_init(api);
    if (h == FLE_EXE_HANDLE) fle_stop(api, "hLibModule is the lifted DLL itself (not modelled)");
    if (FLE_REALMOD_PRESENT()) {
        int rm = fle_realmod_find_handle(h);
        if (rm >= 0) {
            R[0] = fle_realmod_free(api, rm);
            return;
        }
    }
    if (h < FLE_MODULE_HANDLE_BASE + FLE_MODULE_HANDLE_STEP || (h - FLE_MODULE_HANDLE_BASE) % FLE_MODULE_HANDLE_STEP != 0 ||
        (h - FLE_MODULE_HANDLE_BASE) / FLE_MODULE_HANDLE_STEP > fle_module_count())
        fle_stop(api, "hLibModule is not a virtual module handle (the page names no error code for it)");
    idx = (h - FLE_MODULE_HANDLE_BASE) / FLE_MODULE_HANDLE_STEP - 1;
    if (refs[idx] <= 1) fle_stop(api, "the reference count would reach zero, which would unload a module the process imports (not modelled)");
    refs[idx]--;
    R[0] = 1;
}

/* kernel32!SetDllDirectoryW: lpPathName in rcx. Page: NULL "restores the default search order"; "" "removes the
 * current directory from the default DLL search order"; any other string is "added to the search path", and each
 * call replaces the directory of the previous call. The page lists no failing input, so every call succeeds
 * (nonzero) and the last error is untouched. The directory is only stored (fle_load_dll_directory): the virtual
 * file system holds no module file for a search to find. A path longer than LOAD_DIR_MAX characters is not
 * defined by the page (assumption: the 32,767-character limit of the Unicode path form) and stops. */
void fle_imp_SetDllDirectoryW(void)
{
    static const char api[] = "SetDllDirectoryW";
    uint16_t *copy = NULL;
    size_t len;

    if (R[1] != 0) copy = fle_w_read_alloc(api, R[1], LOAD_DIR_MAX, &len);
    free(dll_dir);
    dll_dir = copy;
    R[0] = 1;
}

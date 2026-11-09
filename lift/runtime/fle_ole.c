/* ole32 answers: COM initialization state, task memory, CLSID strings, IsEqualGUID
 * (docs/decisions/0037-apis-oleaut-ole.md; the pages read are listed in docs/investigations/apis-oleaut-ole-run.md).
 * Single guest thread, which is one apartment. A case the Microsoft pages leave open stops the run through fle_stop
 * (fle_env.h); where a page says only "fails" or names no value, the choice made here is marked "setting" or
 * "inference". Nothing is read from the host. */
#define _GNU_SOURCE
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

#include "fle_runtime.h"
#include "fle_host.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_ptrmap.h"
#include "fle_reg.h"
#include "fle_ole.h"
#include "fle_modules.h"
#include "fle_win.h"
#include "fle_filedlg.h"

/* decision 0178: weak, so that a program that links fle_ole.c without fle_filedlg.c (many tests) is unchanged */
extern int fle_filedlg_cocreate(uint64_t rclsid, uint64_t punkouter, uint32_t clsctx, uint64_t riid, uint64_t ppv, uint32_t *hr) __attribute__((weak));

static long owner_tid;

static void check_thread(const char *api)
{
    long tid = fle_host_tid();

    if (owner_tid == 0) owner_tid = tid;
    if (tid != owner_tid) fle_stop(api, "called from a second thread, not supported");
}

/* ---- COM initialization ----
 * CoInitialize page: initializes COM on the current thread as a single-thread apartment (STA); a call on an initialized
 * thread succeeds with S_FALSE as long as it does not change the concurrency model; every successful call, S_FALSE ones
 * included, must be balanced by CoUninitialize; a call on a thread initialized as multithreaded returns
 * RPC_E_CHANGED_MODE. OleInitialize page: calls CoInitializeEx internally and specifies STA; S_FALSE when COM is already
 * initialized on the apartment; each successful call, S_FALSE ones included, must be balanced by OleUninitialize.
 * OleUninitialize page: calls CoUninitialize internally.
 * Decision 0037: the guest has one thread and the closure imports no CoInitializeEx, so the apartment is always an STA
 * and RPC_E_CHANGED_MODE cannot occur; it is not produced. com_depth counts the not yet balanced successful calls of
 * CoInitialize and OleInitialize; ole_depth counts the OleInitialize ones among them. */
static uint32_t com_depth, ole_depth;

uint32_t fle_ole_com_depth(void) { return com_depth; }
uint32_t fle_ole_ole_depth(void) { return ole_depth; }

/* Both initializers: S_OK for the call that initializes, S_FALSE for a later one. pvReserved must be NULL (page: "reserved
 * and must be NULL"); the page names no result for a non-NULL one, so it stops. A counter at its maximum stops. */
static uint32_t com_init(const char *api, int ole)
{
    uint32_t hr;

    check_thread(api);
    if (R[1] != 0) fle_stop(api, "pvReserved is not NULL (the page: must be NULL)");
    if (com_depth == UINT32_MAX) fle_stop(api, "initialization count overflow");
    hr = com_depth == 0 ? FLE_HR_S_OK : FLE_HR_S_FALSE;
    com_depth++;
    if (ole) ole_depth++;
    return hr;
}

void fle_imp_CoInitialize(void) { R[0] = com_init("CoInitialize", 0); }
void fle_imp_OleInitialize(void) { R[0] = com_init("OleInitialize", 1); }

/* CoUninitialize page: one call for each successful CoInitialize/CoInitializeEx. A call with nothing to balance, and a
 * call that would take away the count of an OleInitialize (the OleInitialize page wants it balanced by OleUninitialize),
 * are not defined by the pages and stop. */
void fle_imp_CoUninitialize(void)
{
    static const char api[] = "CoUninitialize";

    check_thread(api);
    if (com_depth == 0) fle_stop(api, "no successful CoInitialize/OleInitialize to balance");
    if (com_depth == ole_depth) fle_stop(api, "the remaining initializations come from OleInitialize and need OleUninitialize");
    com_depth--;
}

/* OleUninitialize page: balances OleInitialize and calls CoUninitialize internally. A call with no OleInitialize to
 * balance stops. */
void fle_imp_OleUninitialize(void)
{
    static const char api[] = "OleUninitialize";

    check_thread(api);
    if (ole_depth == 0) fle_stop(api, "no successful OleInitialize to balance");
    ole_depth--;
    com_depth--;
}

/* ---- Task memory ----
 * CoTaskMemAlloc page: a block of at least cb bytes, undefined contents (zero here, setting: determinism); cb 0 gives a
 * valid pointer to a zero-length item; NULL when memory is insufficient. CoTaskMemFree page: NULL has no effect; the
 * block must come from CoTaskMemAlloc/CoTaskMemRealloc. The blocks are host calloc blocks (16-byte aligned, like the
 * GlobalAlloc fixed blocks in fle_glob.c), recorded so that a pointer that is not live stops the run. */
static fle_pmap tasks;

uint64_t fle_ole_task_live(void) { return tasks.live; }

static uint64_t task_alloc(const char *api, uint64_t cb)
{
    void *p;

    if (cb > FLE_COTASK_MAX) return 0;
    p = calloc(1, cb ? (size_t)cb : 1u);
    if (!p) return 0;
    fle_pm_put(&tasks, api, (uint64_t)(uintptr_t)p, cb);
    return (uint64_t)(uintptr_t)p;
}

void fle_imp_CoTaskMemAlloc(void)
{
    check_thread("CoTaskMemAlloc");
    R[0] = task_alloc("CoTaskMemAlloc", R[1]);
}

void fle_imp_CoTaskMemFree(void)
{
    uint64_t p = R[1];

    check_thread("CoTaskMemFree");
    if (p == 0) return;
    if (!fle_pm_del(&tasks, p)) fle_stop("CoTaskMemFree", "pointer is not a live CoTaskMemAlloc block (freed, from another allocator, or stray)");
    free((void *)(uintptr_t)p);
}

/* ---- shell32!SHGetSpecialFolderLocation and SHGetPathFromIDListW (decision 0116, PROVISIONAL) ----
 * They live here, not in fle_env.c, because the ITEMIDLIST they hand out is a CoTaskMemAlloc block (this file's task map) and fle_env.c is linked
 * without this file by the tests of the virtual file system and registry.
 * The Delphi helper f_63ab90 of FLEngine calls SHGetSpecialFolderLocation(NULL, csidl, &pidl); if the HRESULT is not negative it calls
 * SHGetPathFromIDListW(pidl, buf), appends a backslash, and frees the PIDL with CoTaskMemFree (reading of the helper, 0x63ab90..0x63ac27).
 * Learn, SHGetSpecialFolderLocation: hwnd "Reserved"; csidl "A CSIDL value that identifies the folder of interest"; ppidl "A PIDL specifying the folder's
 * location relative to the root of the namespace (the desktop). It is the responsibility of the calling application to free the returned IDList by using
 * CoTaskMemFree"; "If this function succeeds, it returns S_OK. Otherwise, it returns an HRESULT error code." Learn, SHGetPathFromIDListW: "Converts an item
 * identifier list to a file system path"; pidl "The address of an item identifier list that specifies a file or directory location relative to the root of the
 * namespace (the desktop)"; pszPath "The address of a buffer to receive the file system path. This buffer must be at least MAX_PATH characters in size";
 * "Returns TRUE if successful; otherwise, FALSE"; "If the location specified by the pidl parameter is not part of the file system, this function will fail."
 * The same two csidl values as SHGetFolderPathW (fle_env.c) are answered, with the same virtual paths (FLE_VIRTUAL_WINDOWS_DIR + "\\Fonts" for CSIDL_FONTS 0x14,
 * FLE_VIRTUAL_USER_PROFILE_DIR + "\\AppData\\Roaming" for CSIDL_APPDATA 0x1a; the tests of decision 0116 compare the constants with fle_env.c); any other csidl, a
 * non-NULL hwnd (the page says Reserved) or a NULL ppidl stops by name. The ITEMIDLIST is opaque on the pages. The one this runtime issues is a live
 * CoTaskMemAlloc block of 12 bytes: one SHITEMID { USHORT cb = 10; BYTE abID[8] = csidl (4 bytes, little endian) + the marker "PIDL" } and the terminating cb 0
 * (the SHITEMID form is the documented shape of an ITEMIDLIST; the content of abID is this runtime's own and is a choice). SHGetPathFromIDListW accepts only such a
 * block (a pointer that is not a live block of it, or a NULL pidl, stops by name: the page does not define them) and writes the path as SHGetFolderPathW does (no
 * trailing backslash). The last error is not touched. */
#define FLE_PIDL_BYTES  12u
#define FLE_PIDL_CB     10u
#define FLE_PIDL_MARKER UINT64_C(0x4c444950) /* "PIDL" */
#define FLE_SPECIAL_CSIDL_FONTS   0x0014u
#define FLE_SPECIAL_CSIDL_APPDATA 0x001au
/* Decision 0132 (PROVISIONAL), wall 17: CSIDL_SYSTEM (0x0025, shlobj.h: UNOFFICIAL, the Learn page CSIDL lists the name with FOLDERID_System and no number). Learn (CSIDL): "CSIDL_SYSTEM: Version 5.0. The Windows System folder.
 * A typical path is C:\Windows\System32." A real folder (the page's list). Path FLE_VIRTUAL_SYSTEM_DIR (the directory GetSystemDirectoryW answers), no trailing backslash. The caller is the Delphi helper f_63ab90
 * (called from the window code of CreateFruityInstance, return 0x63abb7 in the stop of the run). SHGetFolderPathW (fle_env.c) does NOT answer it yet (the tests of decision 0126 pin that). */
#define FLE_SPECIAL_CSIDL_SYSTEM  0x0025u

/* Decision 0183 (wrp1, PROVISIONAL, OPT-IN): a link may supply more CSIDLs (harness/two/wrp1_vst3_imp.c: CSIDL_PERSONAL 0x05, which Serum 2 asks for); the default (no definition) stops as before, with the same message. */
const char *fle_special_folder_extra(uint32_t csidl) __attribute__((weak));

static const char *special_folder_text(const char *api, uint32_t csidl)
{
    char why[140];

    if (csidl == FLE_SPECIAL_CSIDL_FONTS) return FLE_VIRTUAL_WINDOWS_DIR "\\Fonts";
    if (csidl == FLE_SPECIAL_CSIDL_APPDATA) return FLE_VIRTUAL_USER_PROFILE_DIR "\\AppData\\Roaming";
    if (csidl == FLE_SPECIAL_CSIDL_SYSTEM) return FLE_VIRTUAL_SYSTEM_DIR;
    if (fle_special_folder_extra) {
        const char *t = fle_special_folder_extra(csidl);
        if (t != NULL) return t;
    }
    snprintf(why, sizeof why, "csidl %#x is not answered (only CSIDL_FONTS 0x14, CSIDL_APPDATA 0x1a and CSIDL_SYSTEM 0x25 are)", csidl);
    fle_stop(api, why);
}

/* shell32!SHGetSpecialFolderLocation: HRESULT f(HWND hwnd rcx, int csidl edx, PIDLIST_ABSOLUTE *ppidl r8). */
void fle_imp_SHGetSpecialFolderLocation(void)
{
    static const char api[] = "SHGetSpecialFolderLocation";
    uint32_t csidl = (uint32_t)R[2];
    uint64_t pout = R[8], blk;

    if (R[1] != 0) fle_stop(api, "hwnd is not NULL (the page says Reserved)");
    (void)special_folder_text(api, csidl);
    if (pout == 0) fle_stop(api, "ppidl is NULL: the page does not define it");
    blk = task_alloc(api, FLE_PIDL_BYTES);
    if (blk == 0) {
        R[0] = 0x8007000Eu; /* E_OUTOFMEMORY: the page says "an HRESULT error code" and names none (inference); the task allocator never fails below its limit */
        return;
    }
    fle_st(blk, 2, FLE_PIDL_CB);
    fle_st(blk + 2, 4, csidl);
    fle_st(blk + 6, 4, FLE_PIDL_MARKER);
    fle_st(blk + 10, 2, 0);
    fle_st(pout, 8, blk);
    R[0] = 0;
}

/* shell32!SHGetPathFromIDListW: BOOL f(PCIDLIST_ABSOLUTE pidl rcx, LPWSTR pszPath rdx). */
void fle_imp_SHGetPathFromIDListW(void)
{
    static const char api[] = "SHGetPathFromIDListW";
    uint64_t pidl = R[1], path = R[2], size = 0;
    const char *text;
    uint16_t *w;
    size_t len;

    if (pidl == 0) fle_stop(api, "pidl is NULL: the page does not define it");
    if (path == 0) fle_stop(api, "pszPath is NULL: the page does not define it");
    if (pidl < 2 || !fle_pm_get(&tasks, pidl, &size) || size != FLE_PIDL_BYTES || fle_ld(pidl, 2) != FLE_PIDL_CB || fle_ld(pidl + 6, 4) != FLE_PIDL_MARKER ||
        fle_ld(pidl + 10, 2) != 0)
        fle_stop(api, "pidl is not an ITEMIDLIST that SHGetSpecialFolderLocation of this runtime issued (and that is still live): other item identifier lists are not modelled");
    text = special_folder_text(api, (uint32_t)fle_ld(pidl + 2, 4));
    w = fle_w_from_ascii(text);
    len = fle_w_len(w);
    fle_w_write(path, w, len + 1);
    free(w);
    R[0] = 1;
}

/* ---- shell32!ILCreateFromPathW, ILFree, SHChangeNotifyRegister, SHChangeNotifyDeregister (decision 0126, PROVISIONAL, walls 87 to 89) ----
 * Learn ILCreateFromPathW: "Returns the ITEMIDLIST structure associated with a specified file path"; pszPath "no more than MAX_PATH characters in length, including the terminating null character"; "Call ILFree to release
 * the ITEMIDLIST when you are finished with it". The page names no failure value. A path of the virtual file system gives an ITEMIDLIST that is a live CoTaskMemAlloc block (Learn ILFree: "ITEMIDLIST structures are always
 * allocated with the Component Object Model (COM) task allocator on those platforms"): one SHITEMID { USHORT cb; BYTE abID = the marker "PATH" + the path as UTF-16 with its NUL } and the terminating cb 0; the content of abID is
 * this runtime's own (S-il-path-form: the pages say the ITEMIDLIST is opaque). A path that the virtual file system does not hold gives NULL (S-il-missing-null: a CHOICE WITHOUT A SOURCE; the page names no failure value;
 * withdrawn when a measurement on a real Windows gives another answer). A NULL pszPath, an empty path, a path of MAX_PATH characters or more, and a build without the virtual file system stop. SHGetPathFromIDListW does
 * not accept these blocks (it stops, as for any other list that SHGetSpecialFolderLocation of this runtime did not issue).
 * Learn ILFree: "Frees an ITEMIDLIST structure allocated by the Shell"; pidl "can be NULL"; no return value: the same as CoTaskMemFree here (a block that is not live stops: the page does not define it).
 * Learn SHChangeNotifyRegister: "Registers a window to receive notifications from the file system or Shell"; fSources is a union of SHCNRF_InterruptLevel 0x1, SHCNRF_ShellLevel 0x2, SHCNRF_RecursiveInterrupt 0x1000 (with the
 * interrupt level), SHCNRF_NewDelivery 0x8000; "This array should always be set to one": cEntries other than 1 and a NULL pshcne stop (the page leaves them open); each entry's PIDL must be a live task block of this runtime
 * (anything else stops); returns "a positive integer registration ID" and "0 if out of memory or in response to invalid parameters": a hwnd that is not NULL and not a live window gives 0 (an invalid parameter), as do
 * fSources with no level bit, with a bit that the page does not list, or SHCNRF_RecursiveInterrupt without SHCNRF_InterruptLevel (the page: "must be combined"). The IDs are 1, 2, 3, ... (S-shchange-id: only "positive" is
 * on the page). S-shchange-no-events: a registration is recorded and NO notification is ever posted: the virtual file system has no writer other than the guest, and the guest's own writes are not turned into shell
 * notifications (withdrawn when a notification model exists). At most 64 live registrations (a limit of the virtual machine: the 65th returns 0, "out of memory"). SHChangeNotifyDeregister: "Returns TRUE if the specified
 * client was found and removed; otherwise FALSE". */
#define FLE_SHCN_MAX      64u
#define FLE_SHCNRF_KNOWN  (0x0001u | 0x0002u | 0x1000u | 0x8000u)
#define FLE_IL_PATH_MARKER UINT64_C(0x48544150) /* "PATH" */

int fle_vfs_path_exists(const char *api, uint64_t guest_path) __attribute__((weak));
int fle_win_get(uint64_t hwnd, fle_win_view *v) __attribute__((weak));

static struct {
    uint32_t id;
    uint64_t hwnd;
    uint32_t sources, events, msg;
    uint64_t pidl;
    uint32_t recursive;
} shcn[FLE_SHCN_MAX];
static uint32_t shcn_next = 1;

/* shell32!ILCreateFromPathW: PIDLIST_ABSOLUTE f(PCWSTR pszPath rcx). */
void fle_imp_ILCreateFromPathW(void)
{
    static const char api[] = "ILCreateFromPathW";
    uint64_t blk;
    uint16_t *w;
    size_t len;
    uint32_t cb;

    check_thread(api);
    if (R[1] == 0) fle_stop(api, "pszPath is NULL: the page does not define it");
    if (fle_ld(R[1], 2) == 0) fle_stop(api, "pszPath is empty: the page does not define it");
    if (fle_vfs_path_exists == NULL) fle_stop(api, "no virtual file system is linked: the path cannot be looked up");
    if (!fle_vfs_path_exists(api, R[1])) {
        R[0] = 0;
        return;
    }
    w = fle_w_read_alloc(api, R[1], 259, &len);
    cb = (uint32_t)(2 + 4 + 2 * (len + 1));
    blk = task_alloc(api, (uint64_t)cb + 2u);
    if (blk == 0) {
        free(w);
        R[0] = 0;
        return;
    }
    fle_st(blk, 2, cb);
    fle_st(blk + 2, 4, FLE_IL_PATH_MARKER);
    fle_w_write(blk + 6, w, len + 1);
    fle_st(blk + cb, 2, 0);
    free(w);
    R[0] = blk;
}

/* shell32!ILFree: void f(PIDLIST_RELATIVE pidl rcx). Called by fle_modules.c's ILFree row through a weak reference (decision 0126). */
void fle_ole_il_free(const char *api, uint64_t pidl)
{
    check_thread(api);
    if (pidl == 0) return;
    if (!fle_pm_del(&tasks, pidl)) fle_stop(api, "pidl is not a live ITEMIDLIST of this runtime (freed, from another allocator, or stray): the page does not define it");
    free((void *)(uintptr_t)pidl);
}

/* shell32!SHChangeNotifyRegister: ULONG f(HWND hwnd rcx, int fSources edx, LONG fEvents r8d, UINT wMsg r9d, int cEntries [5], const SHChangeNotifyEntry *pshcne [6]). */
void fle_imp_SHChangeNotifyRegister(void)
{
    static const char api[] = "SHChangeNotifyRegister";
    uint64_t hwnd = R[1], pe;
    uint32_t src = (uint32_t)R[2], ev = (uint32_t)R[8], msg = (uint32_t)R[9];
    int32_t n = (int32_t)(uint32_t)fle_arg(5);
    uint64_t pidl, size = 0;
    unsigned k;
    fle_win_view v;

    check_thread(api);
    pe = fle_arg(6);
    if (n != 1) fle_stop(api, "cEntries is not 1: the page says the array should always be set to one and gives no behaviour for another count");
    if (pe == 0) fle_stop(api, "pshcne is NULL: the page does not define it");
    pidl = fle_ld(pe, 8);
    /* anim1 (decision 0225): a NULL pidl is a normal registration, whatever rows the build has (it no longer depends on the layer-4 marker row of decision 0191). Learn (SHChangeNotifyEntry.pidl: "PIDL for which to receive
     * notifications"; SHChangeNotifyRegister: "Returns a positive integer registration ID. Returns 0 if out of memory or in response to invalid parameters") says nothing about a NULL pidl. Wine 10.0
     * (dlls/shell32/changenotify.c, SHChangeNotifyRegister) stores ILClone(NULL) = NULL and returns the new positive ID, and SHChangeNotify treats a NULL pidl of a registration as "all" (`if( !pidl )` with the comment "all ?", then `notify = TRUE`):
     * so the ID is positive and the entry never delivers here (S-shchange-no-events). The engine registers a second folder with the NULL pidl that ILCreateFromPathW answered for a folder the volume does not hold (wall ia2-W26). */
    pidl = fle_ld(pe, 8);
    if (pidl != 0 && (pidl < 2 || !fle_pm_get(&tasks, pidl, &size) || size < 4 || fle_ld(pidl, 2) + 2 > size))
        fle_stop(api, "the entry's pidl is not an ITEMIDLIST that a shell function of this runtime issued and that is still live: other lists are not modelled");
    if (src & ~FLE_SHCNRF_KNOWN) {
        R[0] = 0;
        return;
    }
    if ((src & 0x0003u) == 0 || ((src & 0x1000u) && !(src & 0x0001u))) {
        R[0] = 0;
        return;
    }
    if (hwnd != 0) {
        if (fle_win_get == NULL) fle_stop(api, "no window table is linked: the window cannot be checked");
        if (!fle_win_get(hwnd, &v) || !v.live) {
            R[0] = 0;
            return;
        }
    }
    for (k = 0; k < FLE_SHCN_MAX; k++)
        if (shcn[k].id == 0) break;
    if (k == FLE_SHCN_MAX) {
        R[0] = 0;
        return;
    }
    shcn[k].id = shcn_next++;
    shcn[k].hwnd = hwnd;
    shcn[k].sources = src;
    shcn[k].events = ev;
    shcn[k].msg = msg;
    shcn[k].pidl = pidl;
    shcn[k].recursive = (uint32_t)fle_ld(pe + 8, 4);
    R[0] = shcn[k].id;
}

/* shell32!SHChangeNotifyDeregister: BOOL f(ULONG ulID ecx). */
void fle_imp_SHChangeNotifyDeregister(void)
{
    uint32_t id = (uint32_t)R[1];
    unsigned k;

    check_thread("SHChangeNotifyDeregister");
    for (k = 0; k < FLE_SHCN_MAX; k++) {
        if (id != 0 && shcn[k].id == id) {
            memset(&shcn[k], 0, sizeof shcn[k]);
            R[0] = 1;
            return;
        }
    }
    R[0] = 0;
}

uint32_t fle_ole_shchange_live(void)
{
    uint32_t n = 0;
    unsigned k;

    for (k = 0; k < FLE_SHCN_MAX; k++)
        if (shcn[k].id) n++;
    return n;
}

/* ---- ole32!RegisterDragDrop and RevokeDragDrop (decision 0126, PROVISIONAL, wall 90) ----
 * Learn RegisterDragDrop: "Registers the specified window as one that can be the target of an OLE drag-and-drop operation and specifies the IDropTarget instance to use"; S_OK on success; DRAGDROP_E_INVALIDHWND ("Invalid
 * handle returned in the hwnd parameter"), DRAGDROP_E_ALREADYREGISTERED ("The specified window has already been registered as a drop target"), E_OUTOFMEMORY ("Insufficient memory for the operation"); "If you use CoInitialize
 * or CoInitializeEx instead of OleInitialize, RegisterDragDrop will always return an E_OUTOFMEMORY error"; "The RegisterDragDrop function also calls the IUnknown::AddRef method on the IDropTarget pointer". Learn
 * RevokeDragDrop: S_OK; DRAGDROP_E_NOTREGISTERED, DRAGDROP_E_INVALIDHWND, E_OUTOFMEMORY; "This function calls the IUnknown::Release method for your drop target interface". The HRESULT numbers (DRAGDROP_E_NOTREGISTERED
 * 0x80040100, DRAGDROP_E_ALREADYREGISTERED 0x80040101, DRAGDROP_E_INVALIDHWND 0x80040104; E_OUTOFMEMORY 0x8007000E) are from the SDK header winerror.h as mirrored by mingw-w64: the pages name the codes without numbers (an
 * unofficial source). The apartment state is one for the whole process here (decision 0037: com_depth, ole_depth), so "OleInitialize was called" is ole_depth != 0 (S-dragdrop-ole-global: a reading; the page says per thread).
 * The checks run in the order invalid window, E_OUTOFMEMORY for a COM-only initialisation, already registered (S-dragdrop-order: the pages give no order). A window is valid when it is a live window of the window table. A NULL
 * pDropTarget is not defined by the page: it stops. AddRef is vtable slot 1 and Release slot 2 of the IUnknown part of the interface (the order of IUnknown); both are called in the guest through fle_call_guest, and their
 * return values are not used. The registration is kept until RevokeDragDrop: a window that is destroyed without a revoke keeps its entry (handles are never reused, so no later window finds it). */
#define FLE_DRAGDROP_E_NOTREGISTERED     0x80040100u
#define FLE_DRAGDROP_E_ALREADYREGISTERED 0x80040101u
#define FLE_DRAGDROP_E_INVALIDHWND       0x80040104u
#define FLE_E_OUTOFMEMORY_HR             0x8007000Eu
#define FLE_DRAGDROP_MAX                 256u

extern uint64_t fle_call_guest(uint64_t va, uint64_t rcx, uint64_t rdx, uint64_t r8, uint64_t r9) __attribute__((weak));

static struct {
    uint64_t hwnd, target;
} dragdrop[FLE_DRAGDROP_MAX];

static int dragdrop_find(uint64_t hwnd)
{
    unsigned k;

    for (k = 0; k < FLE_DRAGDROP_MAX; k++)
        if (dragdrop[k].hwnd == hwnd && hwnd != 0) return (int)k;
    return -1;
}

static void dragdrop_unknown(const char *api, uint64_t obj, unsigned slot)
{
    uint64_t vt, fn;

    if (fle_call_guest == NULL) fle_stop(api, "the guest callback mechanism is not linked: IUnknown cannot be called");
    vt = fle_ld(obj, 8);
    fn = fle_ld(vt + 8u * slot, 8);
    (void)fle_call_guest(fn, obj, 0, 0, 0);
}

/* ole32!RegisterDragDrop: HRESULT f(HWND hwnd rcx, LPDROPTARGET pDropTarget rdx). */
void fle_imp_RegisterDragDrop(void)
{
    static const char api[] = "RegisterDragDrop";
    uint64_t hwnd = R[1], target = R[2];
    fle_win_view v;
    unsigned k;

    check_thread(api);
    if (target == 0) fle_stop(api, "pDropTarget is NULL: the page does not define it");
    if (fle_win_get == NULL) fle_stop(api, "no window table is linked: the window cannot be checked");
    if (hwnd == 0 || !fle_win_get(hwnd, &v) || !v.live) {
        R[0] = FLE_DRAGDROP_E_INVALIDHWND;
        return;
    }
    if (ole_depth == 0) {
        R[0] = FLE_E_OUTOFMEMORY_HR;
        return;
    }
    if (dragdrop_find(hwnd) >= 0) {
        R[0] = FLE_DRAGDROP_E_ALREADYREGISTERED;
        return;
    }
    for (k = 0; k < FLE_DRAGDROP_MAX; k++)
        if (dragdrop[k].hwnd == 0) break;
    if (k == FLE_DRAGDROP_MAX) {
        R[0] = FLE_E_OUTOFMEMORY_HR;
        return;
    }
    dragdrop[k].hwnd = hwnd;
    dragdrop[k].target = target;
    dragdrop_unknown(api, target, 1);
    R[0] = 0;
}

/* ole32!RevokeDragDrop: HRESULT f(HWND hwnd rcx). */
void fle_imp_RevokeDragDrop(void)
{
    static const char api[] = "RevokeDragDrop";
    uint64_t hwnd = R[1], target;
    fle_win_view v;
    int k;

    check_thread(api);
    if (fle_win_get == NULL) fle_stop(api, "no window table is linked: the window cannot be checked");
    if (hwnd == 0 || !fle_win_get(hwnd, &v)) {
        R[0] = FLE_DRAGDROP_E_INVALIDHWND;
        return;
    }
    k = dragdrop_find(hwnd);
    if (k < 0) {
        R[0] = FLE_DRAGDROP_E_NOTREGISTERED;
        return;
    }
    target = dragdrop[k].target;
    dragdrop[k].hwnd = 0;
    dragdrop[k].target = 0;
    dragdrop_unknown(api, target, 2);
    R[0] = 0;
}

uint32_t fle_ole_dragdrop_live(void)
{
    uint32_t n = 0;
    unsigned k;

    for (k = 0; k < FLE_DRAGDROP_MAX; k++)
        if (dragdrop[k].hwnd) n++;
    return n;
}

/* ---- GUID text ----
 * GUID page: Data1 (4 bytes), Data2, Data3 (2 bytes each), Data4[8]; the groups are 8, 4, 4, 4, 12 hex digits.
 * StringFromGUID2/StringFromCLSID/CLSIDFromString pages: the string has the form {xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx}
 * with enclosing braces (38 characters, 39 with the NUL). The case of the hex digits is not on those pages; the GUID
 * page's example is upper case and so is the output here (setting); the input accepts both cases (inference). */
#define GUID_CHARS 38u

static void guid_text(uint64_t g, uint16_t *out)
{
    char s[GUID_CHARS + 1];
    uint32_t d1 = (uint32_t)fle_ld(g, 4);
    uint32_t d2 = (uint32_t)fle_ld(g + 4, 2), d3 = (uint32_t)fle_ld(g + 6, 2);
    uint8_t b[8];
    unsigned i;

    for (i = 0; i < 8; i++) b[i] = (uint8_t)fle_ld(g + 8 + i, 1);
    snprintf(s, sizeof s, "{%08X-%04X-%04X-%02X%02X-%02X%02X%02X%02X%02X%02X}", d1, d2, d3, b[0], b[1], b[2], b[3], b[4],
             b[5], b[6], b[7]);
    for (i = 0; i <= GUID_CHARS; i++) out[i] = (uint8_t)s[i];
}

static int hexv(uint16_t c)
{
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    if (c >= 'A' && c <= 'F') return c - 'A' + 10;
    return -1;
}

/* 1 and the 16 bytes of the GUID when s (38 units) is {8-4-4-4-12} hex; else 0. */
static int guid_parse(const uint16_t *s, uint8_t out[16])
{
    static const unsigned dash[4] = { 9, 14, 19, 24 };
    unsigned i, k;
    uint8_t raw[16];

    if (s[0] != '{' || s[37] != '}') return 0;
    for (i = 0; i < 4; i++)
        if (s[dash[i]] != '-') return 0;
    for (i = 1, k = 0; i < 37;) {
        int hi, lo;

        if (i == 9 || i == 14 || i == 19 || i == 24) {
            i++;
            continue;
        }
        hi = hexv(s[i]);
        lo = hexv(s[i + 1]);
        if (hi < 0 || lo < 0) return 0;
        raw[k++] = (uint8_t)(hi * 16 + lo);
        i += 2;
    }
    /* raw is the text order: Data1 (big-endian text) Data2 Data3 then 8 bytes. Fields are stored little-endian. */
    out[0] = raw[3]; out[1] = raw[2]; out[2] = raw[1]; out[3] = raw[0];
    out[4] = raw[5]; out[5] = raw[4];
    out[6] = raw[7]; out[7] = raw[6];
    memcpy(out + 8, raw + 8, 8);
    return 1;
}

/* ole32!StringFromCLSID: HRESULT f(REFCLSID rclsid rcx, LPOLESTR *lplpsz rdx). Page: the string includes the braces and is
 * freed by the caller with CoTaskMemFree (so it is a CoTaskMemAlloc block); S_OK, or E_OUTOFMEMORY. NULL arguments are
 * not defined by the page and stop. */
void fle_imp_StringFromCLSID(void)
{
    static const char api[] = "StringFromCLSID";
    uint64_t g = R[1], pp = R[2], p;
    uint16_t txt[GUID_CHARS + 1];

    check_thread(api);
    if (g == 0 || pp == 0) fle_stop(api, "NULL argument (the page does not define it)");
    p = task_alloc(api, (GUID_CHARS + 1u) * 2u);
    if (p == 0) {
        R[0] = FLE_HR_E_OUTOFMEMORY;
        return;
    }
    guid_text(g, txt);
    fle_w_write(p, txt, GUID_CHARS + 1u);
    fle_st(pp, 8, p);
    R[0] = FLE_HR_S_OK;
}

/* ole32!CLSIDFromString: HRESULT f(LPCOLESTR lpsz rcx, LPCLSID pclsid rdx). Page: converts {xxxxxxxx-...} to the CLSID;
 * NULL gives GUID_NULL; a ProgID string is looked up (registry); NOERROR on success, CO_E_CLASSSTRING when the string is
 * improperly formatted, E_INVALIDARG is a standard value. pclsid NULL is E_INVALIDARG (inference). A string that does
 * not start with '{' is a ProgID, and the virtual registry has no ProgID resolution, so it stops. A string that starts
 * with '{' and is not exactly the 38-character form is CO_E_CLASSSTRING. pclsid is not written on a failure (setting). */
void fle_imp_CLSIDFromString(void)
{
    static const char api[] = "CLSIDFromString";
    uint64_t sp = R[1], out = R[2];
    uint16_t s[GUID_CHARS + 1];
    uint8_t g[16];
    unsigned n;

    check_thread(api);
    if (out == 0) {
        R[0] = FLE_HR_E_INVALIDARG;
        return;
    }
    if (sp == 0) {
        memset(g, 0, sizeof g);
        memcpy((void *)fle_addr(out, 16, 1), g, 16);
        R[0] = FLE_HR_S_OK;
        return;
    }
    if ((uint16_t)fle_ld(sp, 2) != '{') fle_stop(api, "a string that does not start with '{' is a ProgID; ProgID resolution is not modelled");
    for (n = 0; n <= GUID_CHARS; n++) {
        s[n] = (uint16_t)fle_ld(sp + 2u * n, 2);
        if (s[n] == 0) break;
    }
    if (n != GUID_CHARS || s[GUID_CHARS] != 0 || !guid_parse(s, g)) {
        R[0] = FLE_HR_CO_E_CLASSSTRING;
        return;
    }
    memcpy((void *)fle_addr(out, 16, 1), g, 16);
    R[0] = FLE_HR_S_OK;
}

/* ole32!IsEqualGUID: BOOL f(REFGUID rguid1 rcx, REFGUID rguid2 rdx). Page: TRUE when the two GUIDs are equal, else FALSE.
 * The 16 bytes are compared. A NULL argument is not defined and stops. */
void fle_imp_IsEqualGUID(void)
{
    uint64_t a = R[1], b = R[2];

    check_thread("IsEqualGUID");
    if (a == 0 || b == 0) fle_stop("IsEqualGUID", "NULL argument (the page does not define it)");
    R[0] = memcmp((const void *)fle_addr(a, 16, 0), (const void *)fle_addr(b, 16, 0), 16) == 0 ? 1u : 0u;
}

/* ---- ProgIDFromCLSID ----
 * Page: converts a CLSID to its ProgID using the registry; S_OK, REGDB_E_CLASSNOTREG (class not registered),
 * REGDB_E_READREGDB; the ProgID is freed with CoTaskMemFree. CLSID Key page: HKEY_LOCAL_MACHINE\SOFTWARE\Classes\CLSID\{CLSID}
 * holds the class and corresponds to HKEY_CLASSES_ROOT; HKEY_CLASSES_ROOT page: it merges HKLM\Software\Classes with
 * HKCU\Software\Classes. The virtual registry (fle_reg.c) is empty at start and has no merged view, so the class is
 * "not registered" when none of the three locations holds the key. When the guest has created one of them, the page does
 * not fix how the ProgID value is read, so that case stops. */
static int reg_has_key(const char *api, uint64_t root, const char *path)
{
    uint64_t save[6] = { R[0], R[1], R[2], R[4], R[8], R[9] };
    uint64_t stk[8] = { 0 };
    uint64_t hout = 0;
    uint16_t *w = fle_w_from_ascii(path);
    uint32_t st;
    int found = 0;

    stk[5] = (uint64_t)(uintptr_t)&hout;
    R[1] = root;
    R[2] = (uint64_t)(uintptr_t)w;
    R[8] = 0;
    R[9] = FLE_KEY_QUERY_VALUE;
    R[4] = (uint64_t)(uintptr_t)stk;
    fle_imp_RegOpenKeyExW();
    st = (uint32_t)R[0];
    if (st == 0) {
        R[1] = hout;
        fle_imp_RegCloseKey();
        found = 1;
    } else if (st != FLE_W32_ERR_FILE_NOT_FOUND) {
        fle_stop(api, "the virtual registry answered with an unexpected status");
    }
    R[0] = save[0]; R[1] = save[1]; R[2] = save[2]; R[4] = save[3]; R[8] = save[4]; R[9] = save[5];
    free(w);
    return found;
}

void fle_imp_ProgIDFromCLSID(void)
{
    static const char api[] = "ProgIDFromCLSID";
    uint64_t g = R[1], pp = R[2];
    uint16_t txt[GUID_CHARS + 1];
    char a[GUID_CHARS + 1], path[96];
    unsigned i;

    check_thread(api);
    if (g == 0 || pp == 0) fle_stop(api, "NULL argument (the page does not define it)");
    guid_text(g, txt);
    for (i = 0; i <= GUID_CHARS; i++) a[i] = (char)txt[i];
    snprintf(path, sizeof path, "CLSID\\%s", a);
    if (reg_has_key(api, FLE_HKEY_CLASSES_ROOT, path)) fle_stop(api, "the CLSID key exists in the virtual registry; reading its ProgID is not defined by the page");
    snprintf(path, sizeof path, "SOFTWARE\\Classes\\CLSID\\%s", a);
    if (reg_has_key(api, FLE_HKEY_LOCAL_MACHINE, path)) fle_stop(api, "the CLSID key exists in the virtual registry; reading its ProgID is not defined by the page");
    snprintf(path, sizeof path, "Software\\Classes\\CLSID\\%s", a);
    if (reg_has_key(api, FLE_HKEY_CURRENT_USER, path)) fle_stop(api, "the CLSID key exists in the virtual registry; reading its ProgID is not defined by the page");
    R[0] = FLE_HR_REGDB_E_CLASSNOTREG;
}

/* ---- ole32!CoCreateInstance (decision 0129, PROVISIONAL; PM's decision, S-com-registry-empty) ----
 * Learn (CoCreateInstance): HRESULT f(REFCLSID rclsid rcx, LPUNKNOWN pUnkOuter rdx, DWORD dwClsContext r8d, REFIID riid r9, LPVOID *ppv [5]); "Upon successful return, *ppv contains the requested interface
 * pointer. Upon failure, *ppv contains NULL."; return values: S_OK, REGDB_E_CLASSNOTREG "A specified class is not registered in the registration database", CLASS_E_NOAGGREGATION, E_NOINTERFACE, E_POINTER
 * "The ppv parameter is NULL". The answer here depends on the REGISTRATION of the class only (the same test as ProgIDFromCLSID above: HKCR CLSID\{...} or HKLM\SOFTWARE\Classes\CLSID\{...} or
 * HKCU\Software\Classes\CLSID\{...} in the virtual registry fle_reg.c, which the harness fills from the seed lists of decisions 0116 and 0117): a CLSID that none of the three keys holds is
 * "not registered": *ppv = NULL and REGDB_E_CLASSNOTREG (0x80040154: the value is the one decision 0037 took from Learn's COM Error Codes). That the virtual environment registers no shell class (the shell's
 * TaskbarList, CLSID 56FDF344-FD6D-11D0-958A-006097C9A090, in particular) is a definition of THE ENVIRONMENT, a CHOICE WITHOUT A SOURCE (S-com-registry-empty): Windows registers it, and a measurement on a real Windows
 * would show an ITaskbarList. Withdrawn when that is shown, or when a shell COM model is made. A CLSID that IS registered in the virtual registry stops (no class is implemented: a different decision), and a
 * NULL ppv stops (E_POINTER's value 0x80004003 is winerror.h, not a Learn page: UNOFFICIAL, and the guest never passes NULL). pUnkOuter and dwClsContext do not change the answer (the page's class lookup
 * is by registration; a context with no server type of that class is also REGDB_E_CLASSNOTREG on the page). */
void fle_imp_CoCreateInstance(void)
{
    static const char api[] = "CoCreateInstance";
    uint64_t g = R[1], ppv = fle_arg(5);
    uint16_t txt[GUID_CHARS + 1];
    char a[GUID_CHARS + 1], path[96];
    unsigned i;

    check_thread(api);
    if (g == 0) fle_stop(api, "rclsid is NULL: the page does not define it");
    if (ppv == 0) fle_stop(api, "ppv is NULL: the page's E_POINTER (0x80004003) is winerror.h, not a Learn page (decision 0129)");
    guid_text(g, txt);
    for (i = 0; i <= GUID_CHARS; i++) a[i] = (char)txt[i];
    fle_st(ppv, 8, 0);
    {   /* decision 0178 (opt-in): the common item dialog model answers CLSID_FileOpenDialog when the build holds its rows (fle_filedlg.c); every other case is as before */
        uint32_t hr;

        if (fle_filedlg_cocreate != NULL && fle_filedlg_cocreate(g, R[2], (uint32_t)R[8], R[9], ppv, &hr)) { R[0] = hr; return; }
    }
    snprintf(path, sizeof path, "CLSID\\%s", a);
    if (reg_has_key(api, FLE_HKEY_CLASSES_ROOT, path)) fle_stop(api, "the CLSID is registered in the virtual registry: no class of this virtual machine is implemented (decision 0129)");
    snprintf(path, sizeof path, "SOFTWARE\\Classes\\CLSID\\%s", a);
    if (reg_has_key(api, FLE_HKEY_LOCAL_MACHINE, path)) fle_stop(api, "the CLSID is registered in the virtual registry: no class of this virtual machine is implemented (decision 0129)");
    snprintf(path, sizeof path, "Software\\Classes\\CLSID\\%s", a);
    if (reg_has_key(api, FLE_HKEY_CURRENT_USER, path)) fle_stop(api, "the CLSID is registered in the virtual registry: no class of this virtual machine is implemented (decision 0129)");
    R[0] = FLE_HR_REGDB_E_CLASSNOTREG;
}

/* ---- shell32!SHGetMalloc and the IMalloc it returns (decision 0098, PROVISIONAL) ----
 * Learn (SHGetMalloc): "Retrieves a pointer to the Shell's IMalloc interface"; ppMalloc is "The address of a pointer that receives the Shell's IMalloc
 * interface pointer"; "If this function succeeds, it returns S_OK. Otherwise, it returns an HRESULT error code." Remarks: "as of Windows 2000 it is
 * no longer necessary. In its place, programs can call the equivalent (and easier to use) CoTaskMemAlloc and CoTaskMemFree". So the allocator behind
 * the interface is the task allocator above (the `tasks` table). The caller releases the pointer (the guest does: f_41a770 calls vtable[2]).
 * Learn (IMalloc::Free): "If this parameter is NULL, this method has no effect"; (IMalloc::DidAlloc): 1 "allocated by this allocator", 0 "was not
 * allocated by this allocator", -1 "cannot determine"; "If this parameter is a NULL pointer, -1 is returned". */
extern const fle_resp_entry fle_resp_table[] __attribute__((weak));
extern const uint64_t fle_resp_count __attribute__((weak));

#define IMALLOC_SLOTS 9u
static uint64_t imalloc_obj;        /* host address of { vtable pointer, slot[9] }; never freed */
static uint32_t imalloc_refs;

uint32_t fle_ole_shell_malloc_refs(void) { return imalloc_refs; }

static uint64_t imalloc_slot_va(const char *api, const char *name)
{
    uint64_t i;

    if (fle_resp_table != NULL)
        for (i = 0; i < fle_resp_count; i++)
            if (strcmp(fle_resp_table[i].dll, "shell32.dll") == 0 && strcmp(fle_resp_table[i].name, name) == 0) return fle_resp_table[i].va;
    fle_stop(api, "the response table has no row for an IMalloc method (decision 0098)");
}

void fle_imp_SHGetMalloc(void)
{
    static const char api[] = "SHGetMalloc";
    uint64_t pp = R[1];

    check_thread(api);
    if (pp == 0) fle_stop(api, "ppMalloc is NULL: the page does not define it (decision 0098)");
    if (imalloc_obj == 0) {
        uint64_t *o = calloc(1, (1u + IMALLOC_SLOTS) * sizeof *o);
        uint64_t unimpl = imalloc_slot_va(api, "IMalloc::unimplemented");
        unsigned i;

        if (!o) fle_stop(api, "out of memory");
        o[0] = (uint64_t)(uintptr_t)&o[1];
        for (i = 0; i < IMALLOC_SLOTS; i++) o[1 + i] = unimpl;
        o[1 + 2] = imalloc_slot_va(api, "IMalloc::Release");
        o[1 + 5] = imalloc_slot_va(api, "IMalloc::Free");
        o[1 + 7] = imalloc_slot_va(api, "IMalloc::DidAlloc");
        imalloc_obj = (uint64_t)(uintptr_t)o;
    }
    if (imalloc_refs == UINT32_MAX) fle_stop(api, "reference count overflow");
    imalloc_refs++;
    fle_st(pp, 8, imalloc_obj);
    R[0] = FLE_HR_S_OK;
}

static void imalloc_this(const char *api)
{
    check_thread(api);
    if (imalloc_obj == 0 || R[1] != imalloc_obj) fle_stop(api, "the object is not the one SHGetMalloc returned (decision 0098)");
}

/* IUnknown::Release (Learn): the return value is the new reference count. A Release with no reference outstanding is not defined and stops. */
void fle_imp_IMalloc_Release(void)
{
    static const char api[] = "IMalloc::Release";

    imalloc_this(api);
    if (imalloc_refs == 0) fle_stop(api, "no reference outstanding: the page does not define it (decision 0098)");
    imalloc_refs--;
    R[0] = imalloc_refs;
}

void fle_imp_IMalloc_Free(void)
{
    static const char api[] = "IMalloc::Free";
    uint64_t p = R[2];

    imalloc_this(api);
    if (p == 0) return;
    if (!fle_pm_del(&tasks, p)) fle_stop(api, "pointer is not a live task-allocator block (freed, from another allocator, or stray)");
    free((void *)(uintptr_t)p);
}

void fle_imp_IMalloc_DidAlloc(void)
{
    static const char api[] = "IMalloc::DidAlloc";
    uint64_t p = R[2];

    imalloc_this(api);
    if (p == 0) R[0] = 0xFFFFFFFFu;
    else R[0] = fle_pm_get(&tasks, p, NULL) ? 1u : 0u;
}

void fle_imp_IMalloc_unimplemented(void)
{
    fle_stop("IMalloc", "a method other than Release, Free and DidAlloc was called through the SHGetMalloc object: the guest's call sites use no other (decision 0098)");
}

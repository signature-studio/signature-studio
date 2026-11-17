/* Volume, standard-handle and DuplicateHandle answers (docs/decisions/0024-apis-a3-a5-a13.md; the pages read are
 * listed in docs/investigations/apis-a3-a5-a13-run.md). Single guest thread. A case the Microsoft pages leave open
 * stops the run through fle_stop (fle_env.h); where a page says "the function fails" without naming the error code,
 * the code set here is a runtime choice and is marked "assumption". The host's drives and handles are never read. */
#define _GNU_SOURCE
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

#include "fle_runtime.h"
#include "fle_host.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_proc.h"
#include "fle_vfs.h"
#include "fle_vmap.h"
#include "fle_vol.h"

static long fle_vol_owner_tid;

static void fle_vol_check_thread(const char *api)
{
    long tid = fle_host_tid();

    if (fle_vol_owner_tid == 0) fle_vol_owner_tid = tid;
    if (tid != fle_vol_owner_tid) fle_stop(api, "called from a second thread, not supported");
}

static void fail(uint32_t err, uint64_t ret)
{
    fle_set_last_error(err);
    R[0] = ret;
}

/* MAX_PATH + 1: the page's maximum buffer size for the two name buffers of GetVolumeInformationW. */
#define FLE_VOL_MAX_NAME_BUF 261u
#define FLE_ERR_PATH_NOT_FOUND_A 3u /* assumption */
#define FLE_ERR_FILE_NOT_FOUND_A 2u /* assumption */

/* ---- GetStdHandle ----
 * MSDN (GetStdHandle): nStdHandle is STD_INPUT_HANDLE ((DWORD)-10), STD_OUTPUT_HANDLE ((DWORD)-11) or
 * STD_ERROR_HANDLE ((DWORD)-12); the notes say the values are unsigned numbers cast from a signed one, so only the
 * DWORD (ecx) counts. "If an application does not have associated standard handles ... and has not redirected them,
 * the return value is NULL." The process has none here (decision 0018: GetStartupInfoW reports no standard handles,
 * and SetStdHandle is not answered), so every valid value returns NULL and the last error is left alone. The page does
 * not say when the function fails (INVALID_HANDLE_VALUE) nor with which code, so any other value stops the run. */
void fle_imp_GetStdHandle(void)
{
    uint32_t n = (uint32_t)R[1];

    fle_vol_check_thread("GetStdHandle");
    if (n != 0xFFFFFFF6u && n != 0xFFFFFFF5u && n != 0xFFFFFFF4u)
        fle_stop("GetStdHandle", "nStdHandle is not STD_INPUT_HANDLE, STD_OUTPUT_HANDLE or STD_ERROR_HANDLE");
    R[0] = 0;
}

/* ---- Console and file calls that stop by name (decision 0078; GetFileType is answered since decision 0238, below, and FlushFileBuffers, SetFilePointerEx and FindFirstFileExW in fle_vfs.c) ----
 * dsp_ippv2_x64.dll's statically linked UCRT calls these from its stdio, console, file-descriptor and wildcard-expansion code
 * (WriteConsoleW 0x17da7e4, GetConsoleMode and GetConsoleOutputCP 0x17d49f8, FlushFileBuffers 0x17d8908, SetFilePointerEx 0x17d2ce4,
 * SetStdHandle 0x17d7ecc, FindFirstFileExW 0x17d5ecc, GetFileType 0x17d34b4 and 0x17d35a4). Read, not run: none is on the DLL_PROCESS_ATTACH
 * line or on the line of an IPP function; GetFileType is in the start-up code but is skipped there because GetStdHandle above returns
 * NULL for every standard handle (0x17d35a4 tests the handle first) and GetStartupInfoW reports no inherited handles
 * (0x17d34b4 tests cbReserved2 first). There is no console, no standard handle and no file position beyond fle_vfs.c to answer with,
 * so reaching one ends the run, naming the call; no value is made up. */
#define FLE_VOL_NAMED_STOP(api, why) \
    void fle_imp_##api(void) { fle_stop(#api, why); }

/* Decision 0238 (load2, PROVISIONAL): kernel32!GetFileType, DWORD f(HANDLE hFile rcx). It stopped by name (decision 0078) while no code reached it; the CRT of oggio_x64.dll and Elastique_x64.dll (module start-up, F2xH6) does.
 * Learn (GetFileType): FILE_TYPE_UNKNOWN 0x0000 "Either the type of the specified file is unknown, or the function failed"; FILE_TYPE_DISK 0x0001 "The specified file is a disk file"; FILE_TYPE_CHAR 0x0002 and
 * FILE_TYPE_PIPE 0x0003 are a character file and a socket or pipe; "You can distinguish between a 'valid' return of FILE_TYPE_UNKNOWN and its return due to a calling function error (for example, passing an invalid handle to GetFileType)
 * by calling GetLastError. If the function worked properly and FILE_TYPE_UNKNOWN was returned, a call to GetLastError will return NO_ERROR. If the function call fails, GetLastError will return a different code than NO_ERROR."
 * The files of this environment are the files of the virtual volume (fle_vfs.c): a handle that one issued and that is open is a disk file. NULL, INVALID_HANDLE_VALUE, a closed handle and a find handle are invalid handles: FILE_TYPE_UNKNOWN
 * and ERROR_INVALID_HANDLE (6: the page names no code; "The handle is invalid", System Error Codes; Wine 10.0 gives it, UNOFFICIAL). There is no console, pipe or character device here, so CHAR and PIPE are never answered. A value that
 * the virtual file system did not issue (another kind of handle of this runtime) stops by name, as in every other call on a file handle. */
void fle_imp_GetFileType(void)
{
    const uint8_t *data;
    uint64_t size;
    unsigned rd, wr;

    fle_vol_check_thread("GetFileType");
    if (fle_vfs_file_info("GetFileType", R[1], &data, &size, &rd, &wr)) {
        R[0] = 1u;   /* FILE_TYPE_DISK */
        return;
    }
    fail(6u, 0u);   /* FILE_TYPE_UNKNOWN with an error */
}
FLE_VOL_NAMED_STOP(SetStdHandle, "standard handles are not modelled (GetStdHandle returns NULL for all three)")
FLE_VOL_NAMED_STOP(GetConsoleMode, "there is no console")
FLE_VOL_NAMED_STOP(GetConsoleOutputCP, "there is no console")
FLE_VOL_NAMED_STOP(WriteConsoleW, "there is no console")

/* ---- Drives ----
 * The only drive is the virtual volume C: (fle_vfs.h: "one empty volume root C:\"). */

/* The drive letter (upper case) of a root path "X:\" given by the guest, or 0 for NULL (the root of the current
 * directory; the current directory is always on C:). Any other form (no trailing backslash, a longer path, UNC,
 * relative) is not defined by the pages and stops the run. */
static uint16_t root_letter(const char *api, uint64_t p)
{
    size_t n;
    uint16_t *s;
    uint16_t c;

    if (p == 0) return 'C';
    s = fle_w_read_alloc(api, p, 259, &n);
    if (n != 3 || s[1] != ':' || s[2] != '\\') fle_stop(api, "root path is not of the form X:\\ (a trailing backslash is required; other forms are not implemented)");
    c = fle_w_fold(s[0]);
    if (c < 'a' || c > 'z') fle_stop(api, "root path does not start with a drive letter (other forms are not implemented)");
    free(s);
    return (uint16_t)(c - 32);
}

/* kernel32!GetDriveTypeW: UINT GetDriveTypeW(LPCWSTR lpRootPathName rcx). MSDN: DRIVE_NO_ROOT_DIR (1) when "no volume
 * mounted at the specified path"; DRIVE_FIXED (3) for fixed media. C: is the one volume and it is fixed (a choice);
 * every other letter has no volume. NULL means the root of the current directory (page). */
void fle_imp_GetDriveTypeW(void)
{
    uint16_t c;

    fle_vol_check_thread("GetDriveTypeW");
    c = root_letter("GetDriveTypeW", R[1]);
    R[0] = c == 'C' ? FLE_VOL_DRIVE_TYPE_FIXED : 1u;
}

/* kernel32!GetLogicalDrives: DWORD f(void). MSDN: bit 0 is drive A, bit 1 is B, bit 2 is C. Only C: exists: 0x4. */
void fle_imp_GetLogicalDrives(void)
{
    fle_vol_check_thread("GetLogicalDrives");
    R[0] = 1u << ('C' - 'A');
}

/* kernel32!GetLogicalDriveStringsW: DWORD f(DWORD nBufferLength ecx, LPWSTR lpBuffer rdx). MSDN: the buffer receives
 * one NUL-terminated string per drive plus an additional NUL ("C:\" NUL NUL: 5 units). Success returns the length of
 * the strings copied "not including the terminating null character": 4 (the string with its NUL, without the
 * additional final NUL; this is the reading of the page's sentence, which gives no example). A buffer that is not
 * large enough returns a value above nBufferLength: "the size of the buffer required", 5 units including the final NUL;
 * the buffer is left alone. nBufferLength 0: lpBuffer is not used (page) and the required size is returned. */
void fle_imp_GetLogicalDriveStringsW(void)
{
    static const uint16_t strings[5] = { 'C', ':', '\\', 0, 0 };
    uint32_t n = (uint32_t)R[1];
    uint64_t buf = R[2];

    fle_vol_check_thread("GetLogicalDriveStringsW");
    if (n < 5u) {
        R[0] = 5u;
        return;
    }
    if (buf == 0) fle_stop("GetLogicalDriveStringsW", "lpBuffer is NULL with a non-zero nBufferLength");
    fle_w_write(buf, strings, 5);
    R[0] = 4u;
}

/* Writes the NUL-terminated ASCII `text` to the optional guest buffer when it is given. A buffer of 0 units, or one
 * too short, has no behavior on the page and stops. */
static void put_name(const char *api, uint64_t buf, uint32_t size, const char *text)
{
    uint16_t *w;
    size_t len;

    if (buf == 0) return;
    if (size > FLE_VOL_MAX_NAME_BUF) fle_stop(api, "buffer size above the page's maximum of MAX_PATH + 1");
    w = fle_w_from_ascii(text);
    len = fle_w_len(w);
    if (size < len + 1) fle_stop(api, "buffer too short for the answer (the page gives no behavior)");
    fle_w_write(buf, w, len + 1);
    free(w);
}

/* kernel32!GetVolumeInformationW: BOOL f(lpRootPathName rcx, lpVolumeNameBuffer rdx, nVolumeNameSize r8d,
 * lpVolumeSerialNumber r9, lpMaximumComponentLength [5], lpFileSystemFlags [6], lpFileSystemNameBuffer [7],
 * nFileSystemNameSize [8]). MSDN: a trailing backslash is required; NULL means the root of the current directory;
 * every output is optional; the size of an unsupplied name buffer is ignored; the maximum buffer size is MAX_PATH + 1;
 * the return value is nonzero when everything asked for was retrieved. C: answers with the values of fle_vol.h (all of
 * them choices; the page gives only the shape). A root that has no volume retrieves nothing: 0, with the last error
 * ERROR_PATH_NOT_FOUND (3, assumption: the page names no code). */
void fle_imp_GetVolumeInformationW(void)
{
    uint64_t root = R[1], vbuf = R[2], pserial = R[9];
    uint32_t vsize = (uint32_t)R[8];
    uint64_t pmax = fle_arg(5), pflags = fle_arg(6), fbuf = fle_arg(7);
    uint32_t fsize = (uint32_t)fle_arg(8);
    uint16_t c;

    fle_vol_check_thread("GetVolumeInformationW");
    c = root_letter("GetVolumeInformationW", root);
    if (c != 'C') {
        fail(FLE_ERR_PATH_NOT_FOUND_A, 0);
        return;
    }
    put_name("GetVolumeInformationW", vbuf, vsize, FLE_VOL_LABEL);
    put_name("GetVolumeInformationW", fbuf, fsize, FLE_VOL_FS_NAME);
    if (pserial) fle_st(pserial, 4, FLE_VOL_SERIAL);
    if (pmax) fle_st(pmax, 4, FLE_VOL_MAX_COMPONENT);
    if (pflags) fle_st(pflags, 4, FLE_VOL_FS_FLAGS);
    R[0] = 1;
}

/* kernel32!QueryDosDeviceW: DWORD f(lpDeviceName rcx, lpTargetPath rdx, ucchMax r8d). MSDN: the name has no trailing
 * backslash ("C:", not "C:\"); the buffer gets one or more NUL-terminated strings, the first is the current mapping,
 * the last is followed by an additional NUL; success returns the number of TCHARs stored; a buffer that is too small
 * fails with ERROR_INSUFFICIENT_BUFFER (122, named on the page). C: maps to FLE_VOL_DOS_DEVICE_TARGET (the page's
 * example form \Device\HarddiskVolume1; the number is a choice) with no earlier mappings. Another drive letter has no
 * mapping: 0, with the last error ERROR_FILE_NOT_FOUND (2, assumption: the page names no code). The listing of all device
 * names (lpDeviceName NULL) and device names that are not "X:" are not implemented and stop the run. */
void fle_imp_QueryDosDeviceW(void)
{
    uint64_t name = R[1], out = R[2];
    uint32_t max = (uint32_t)R[8];
    uint16_t *s, *w;
    size_t n, len;
    uint16_t c;

    fle_vol_check_thread("QueryDosDeviceW");
    if (name == 0) fle_stop("QueryDosDeviceW", "lpDeviceName NULL (list of all device names) is not implemented");
    s = fle_w_read_alloc("QueryDosDeviceW", name, 259, &n);
    if (n != 2 || s[1] != ':') fle_stop("QueryDosDeviceW", "device name is not of the form X: (other names are not implemented)");
    c = fle_w_fold(s[0]);
    free(s);
    if (c < 'a' || c > 'z') fle_stop("QueryDosDeviceW", "device name is not of the form X: (other names are not implemented)");
    if (c != 'c') {
        fail(FLE_ERR_FILE_NOT_FOUND_A, 0);
        return;
    }
    w = fle_w_from_ascii(FLE_VOL_DOS_DEVICE_TARGET);
    len = fle_w_len(w);
    if (max < len + 2) {
        free(w);
        fail(FLE_W32_ERR_INSUFFICIENT_BUFFER, 0);
        return;
    }
    if (out == 0) fle_stop("QueryDosDeviceW", "lpTargetPath is NULL");
    fle_w_write(out, w, len + 1);
    fle_st(out + 2u * (len + 1), 2, 0);
    free(w);
    R[0] = (uint32_t)(len + 2);
}

/* ---- DuplicateHandle ----
 * MSDN (DuplicateHandle). The handles it can make here: for the current process (the pseudo handle of GetCurrentProcess,
 * -1, or a real process handle it issued) as source and as target. A pseudo handle of GetCurrentProcess or
 * GetCurrentThread is converted into a real handle of that kind (page). dwOptions is a combination of
 * DUPLICATE_CLOSE_SOURCE (1) and DUPLICATE_SAME_ACCESS (2); any other bit stops. The access rights are not modelled
 * (nothing here checks a handle's access), so dwDesiredAccess and bInheritHandle are accepted and have no effect. When
 * hTargetProcessHandle is NULL (allowed with DUPLICATE_CLOSE_SOURCE) the source is only closed and lpTargetHandle is
 * ignored (page). lpTargetHandle NULL duplicates without returning the value (page). A source that is a file handle of
 * the virtual file system, or any value not issued by this runtime, stops the run. A closed handle of a region that
 * this runtime issues fails with 0 and ERROR_INVALID_HANDLE (6, assumption: the page names no code). */
#define FLE_DUP_CLOSE_SOURCE 1u
#define FLE_DUP_SAME_ACCESS  2u
#define FLE_VOL_PROCESS 1u
#define FLE_VOL_THREAD  2u

typedef struct {
    uint8_t kind;
    uint8_t live;
} vol_ent;

static vol_ent *vh;
static uint64_t vh_n, vh_cap, vh_live;
static int hook_registered;

uint64_t fle_vol_handle_count(void) { return vh_live; }

static int in_vol_region(uint64_t h)
{
    return h > FLE_VOL_HANDLE_BASE && h < FLE_VOL_HANDLE_BASE + FLE_VOL_REGION_SIZE;
}

/* The live entry of an issued handle, NULL when it was closed. A value in the region that was never issued stops. */
static vol_ent *vol_get(const char *api, uint64_t h)
{
    uint64_t off = h - FLE_VOL_HANDLE_BASE, i;

    if (off % 4u) fle_stop(api, "handle value was not issued by this runtime");
    i = off / 4u - 1u;
    if (i >= vh_n) fle_stop(api, "handle value was not issued by this runtime");
    return vh[i].live ? &vh[i] : NULL;
}

static uint64_t vol_new(uint8_t kind)
{
    if (vh_n == vh_cap) {
        uint64_t nc = vh_cap ? vh_cap * 2 : 16;
        vol_ent *n = realloc(vh, nc * sizeof *n);

        if (!n) fle_stop("DuplicateHandle", "out of memory");
        vh = n;
        vh_cap = nc;
    }
    if (vh_n + 2 >= FLE_VOL_REGION_SIZE / 4u) fle_stop("DuplicateHandle", "handle region exhausted");
    vh[vh_n].kind = kind;
    vh[vh_n].live = 1;
    vh_n++;
    vh_live++;
    return FLE_VOL_HANDLE_BASE + 4u * vh_n;
}

/* CloseHandle for the handles issued here (called by fle_imp_CloseHandle through fle_vfs_add_close_hook). */
static int vol_close_hook(uint64_t h)
{
    vol_ent *e;

    if (!in_vol_region(h)) return 0;
    e = vol_get("CloseHandle", h);
    if (!e) {
        fail(FLE_W32_ERR_INVALID_HANDLE, 0);
        return 1;
    }
    e->live = 0;
    vh_live--;
    R[0] = 1;
    return 1;
}

static int in_vfs_file_region(uint64_t h)
{
    return h > FLE_VFS_FILE_BASE && h < FLE_VFS_FILE_BASE + FLE_HANDLE_REGION_SIZE;
}

/* The kind (FLE_VOL_PROCESS, FLE_VOL_THREAD) of a source handle, or 0 when it is a file mapping handle, and the entry
 * to close when CLOSE_SOURCE asks for it. *bad is set to 1 for a handle that was closed. */
static void fle_vol_source(uint64_t h, uint32_t opts, unsigned *kind, int *is_map, vol_ent **ent, int *bad)
{
    *is_map = 0;
    *ent = NULL;
    *bad = 0;
    if (h == FLE_PSEUDO_PROCESS_HANDLE) {
        if (opts & FLE_DUP_CLOSE_SOURCE) fle_stop("DuplicateHandle", "DUPLICATE_CLOSE_SOURCE on a pseudo handle is not defined");
        *kind = FLE_VOL_PROCESS;
    } else if (h == FLE_PSEUDO_THREAD_HANDLE) {
        if (opts & FLE_DUP_CLOSE_SOURCE) fle_stop("DuplicateHandle", "DUPLICATE_CLOSE_SOURCE on a pseudo handle is not defined");
        *kind = FLE_VOL_THREAD;
    } else if (h == 0) {
        *kind = 0;
        *bad = 1;
    } else if (in_vol_region(h)) {
        *ent = vol_get("DuplicateHandle", h);
        if (!*ent) {
            *kind = 0;
            *bad = 1;
        } else {
            *kind = (*ent)->kind;
        }
    } else if (fle_vmap_is_handle(h)) {
        *kind = 0;
        *is_map = 1;
    } else if (in_vfs_file_region(h)) {
        fle_stop("DuplicateHandle", "duplicating a file handle is not implemented (it needs the file system's handle table)");
    } else {
        fle_stop("DuplicateHandle", "source handle was not issued by this runtime");
    }
}

static void fle_vol_hook_once(void)
{
    if (hook_registered) return;
    fle_vfs_add_close_hook(vol_close_hook);
    hook_registered = 1;
}

/* BOOL DuplicateHandle(hSourceProcessHandle rcx, hSourceHandle rdx, hTargetProcessHandle r8, lpTargetHandle r9,
 * dwDesiredAccess [5], bInheritHandle [6], dwOptions [7]). */
void fle_imp_DuplicateHandle(void)
{
    static const char api[] = "DuplicateHandle";
    uint64_t sproc = R[1], src = R[2], tproc = R[8], pout = R[9];
    uint32_t opts = (uint32_t)fle_arg(7);
    unsigned kind;
    int is_map, bad;
    vol_ent *ent;
    uint64_t nh = 0;

    fle_vol_check_thread(api);
    if (opts & ~(FLE_DUP_CLOSE_SOURCE | FLE_DUP_SAME_ACCESS)) fle_stop(api, "dwOptions has a bit that the page does not list");
    if (sproc != FLE_PSEUDO_PROCESS_HANDLE) {
        vol_ent *p = in_vol_region(sproc) ? vol_get(api, sproc) : NULL;

        if (!p || p->kind != FLE_VOL_PROCESS) fle_stop(api, "hSourceProcessHandle is not the current process (no other process exists)");
    }
    if (tproc == 0) {
        if (!(opts & FLE_DUP_CLOSE_SOURCE)) fle_stop(api, "hTargetProcessHandle NULL without DUPLICATE_CLOSE_SOURCE is not defined");
    } else if (tproc != FLE_PSEUDO_PROCESS_HANDLE) {
        vol_ent *p = in_vol_region(tproc) ? vol_get(api, tproc) : NULL;

        if (!p || p->kind != FLE_VOL_PROCESS) fle_stop(api, "hTargetProcessHandle is not the current process (no other process exists)");
    }
    fle_vol_hook_once();
    fle_vol_source(src, opts, &kind, &is_map, &ent, &bad);
    if (bad) {
        fail(FLE_W32_ERR_INVALID_HANDLE, 0);
        return;
    }
    if (tproc != 0) {
        if (is_map) {
            if (!fle_vmap_dup_handle(src, &nh)) {
                fail(FLE_W32_ERR_INVALID_HANDLE, 0);
                return;
            }
        } else {
            nh = vol_new((uint8_t)kind);
        }
        if (pout) fle_st(pout, 8, nh);
    }
    if ((opts & FLE_DUP_CLOSE_SOURCE) && ent) {
        vol_ent *again = vol_get(api, src); /* vol_new may have moved the table: look the source up again */

        again->live = 0;
        vh_live--;
    } else if ((opts & FLE_DUP_CLOSE_SOURCE) && is_map) {
        if (!fle_vmap_close_handle(src)) {
            fail(FLE_W32_ERR_INVALID_HANDLE, 0);
            return;
        }
    }
    R[0] = 1;
}

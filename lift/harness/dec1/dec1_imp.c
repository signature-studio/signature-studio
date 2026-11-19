/* Decision 0206 (dec1, PROVISIONAL, OPT-IN, harness code): answers for kernel32 calls that the lifted sample decoders make on files of the virtual volume and that the runtime stops on by name (fle_vol.c, decision 0078:
 * "not answered for the virtual file system"). Linked with -Wl,--wrap=fle_imp_<API> exactly as harness/two/wrp1_vst3_imp.c does for GetFileType and SetFilePointerEx (decision 0183, wrp1, which only the VST3 link has);
 * a link without the wraps keeps the runtime's named stops. Handles that fle_vfs.c did not issue go to the runtime (__real_*). */
#include <stdint.h>
#include <stdio.h>
#include "fle_runtime.h"
#include "fle_env.h"
#include "fle_imports.h"
#include "fle_vfs.h"

void fle_imp_SetFilePointer(void);
void fle_imp_GetLastError(void);
int fle_vfs_file_info(const char *api, uint64_t h, const uint8_t **data, uint64_t *size, unsigned *can_read, unsigned *can_write);

static int is_vfs_file(const char *api, uint64_t h)
{
    const uint8_t *d;
    uint64_t sz;
    unsigned r, w;
    return h != 0 && h != UINT64_MAX && fle_vfs_file_info(api, h, &d, &sz, &r, &w);
}

/* S-dec1-filetype: GetFileType of a handle that fle_vfs.c issued for an open virtual file is FILE_TYPE_DISK (1; Learn GetFileType: 'The specified file is a disk file'). */
void __real_fle_imp_GetFileType(void);
void __wrap_fle_imp_GetFileType(void)
{
    if (is_vfs_file("GetFileType", R[1])) { R[0] = 1; return; }
    __real_fle_imp_GetFileType();
}

/* S-dec1-setfilepointerex: BOOL SetFilePointerEx(h, LARGE_INTEGER liDistanceToMove, PLARGE_INTEGER lpNewFilePointer, DWORD dwMoveMethod) = SetFilePointer with the distance split in two (Learn: nonzero on success). */
void __real_fle_imp_SetFilePointerEx(void);
void __wrap_fle_imp_SetFilePointerEx(void)
{
    uint64_t h = R[1], newp = R[8];
    int64_t dist = (int64_t)R[2];
    uint32_t method = (uint32_t)R[9], hi = (uint32_t)((uint64_t)dist >> 32), lo, err;
    if (!is_vfs_file("SetFilePointerEx", h)) { __real_fle_imp_SetFilePointerEx(); return; }
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

/* S-dec1-flush: FlushFileBuffers of an open virtual file: the volume holds the bytes in memory, nothing to write out; Learn: 'If the function succeeds, the return value is nonzero'. */
void __real_fle_imp_FlushFileBuffers(void);
void __wrap_fle_imp_FlushFileBuffers(void)
{
    if (is_vfs_file("FlushFileBuffers", R[1])) { R[0] = 1; return; }
    __real_fle_imp_FlushFileBuffers();
}

/* S-dec1-findfirstex: FindFirstFileExW(lpFileName, fInfoLevelId, lpFindFileData, fSearchOp, lpSearchFilter, dwAdditionalFlags) = FindFirstFileW(lpFileName, lpFindFileData) for the cases Learn defines as a plain name search:
 * fInfoLevelId FindExInfoStandard (0) or FindExInfoBasic (1; the data structure is the same, the alternate name is just not required), fSearchOp FindExSearchNameMatch (0) or FindExSearchLimitToDirectories (1: Learn calls it
 * 'a hint ... if the file system does not support, the caller must do the check itself' so returning every match is allowed), lpSearchFilter NULL, dwAdditionalFlags 0 or FIND_FIRST_EX_LARGE_FETCH (2; a performance hint). Anything
 * else (device search, a filter, FIND_FIRST_EX_CASE_SENSITIVE) goes to the runtime's named stop. DelZip64.dll's static CRT and the extraction code reach it (decision 0206, wall Z-FindFirstFileExW). */
void fle_imp_FindFirstFileW(void);
void __real_fle_imp_FindFirstFileExW(void);
void __wrap_fle_imp_FindFirstFileExW(void)
{
    uint32_t level = (uint32_t)R[2], op = (uint32_t)R[9], flags = (uint32_t)fle_arg(6);
    if ((level != 0 && level != 1) || (op != 0 && op != 1) || fle_arg(5) != 0 || (flags & ~2u)) { __real_fle_imp_FindFirstFileExW(); return; }
    R[2] = R[8];
    fle_imp_FindFirstFileW();
}

/* Walls that opening a project through File -> Open... reached (decision 0178, PROVISIONAL). See fle_wall_dlg1.h. */
#include <stdint.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_vfs.h"
#include "fle_wall_dlg1.h"

#define ERROR_NOT_CONNECTED 2250u   /* winerror.h; the WNetGetUniversalNameW page names the code and its meaning, not the number */
#define UNIVERSAL_NAME_INFO_LEVEL 1u
#define REMOTE_NAME_INFO_LEVEL 2u

/* WNetGetUniversalNameW. Learn: 'takes a drive-based path for a network resource and returns ... a more universal form of the name'; ERROR_NOT_CONNECTED 'The device specified by the lpLocalPath parameter is not
 * redirected'; ERROR_BAD_DEVICE 'The string pointed to by the lpLocalPath parameter is invalid'. The virtual volume has one drive, C:, a local disk that no network provider redirects (decision 0018), so a
 * valid drive-based path gets ERROR_NOT_CONNECTED (2250). The engine reached this call while it unpacked a project .zip (decision 0178). A path that is not 'X:' + backslash, or an info level other than the two
 * the page names, stops by name (the page gives no result for them); the buffer is not touched (the page: the structure is stored on success only). */
void fle_imp_WNetGetUniversalNameW(void)
{
    static const char api[] = "WNetGetUniversalNameW";
    uint64_t path = R[1];
    uint32_t level = (uint32_t)R[2];
    uint16_t c0, c1, c2;

    if (path == 0) fle_stop(api, "lpLocalPath is NULL: the page gives no behaviour");
    if (level != UNIVERSAL_NAME_INFO_LEVEL && level != REMOTE_NAME_INFO_LEVEL) fle_stop(api, "dwInfoLevel is neither UNIVERSAL_NAME_INFO_LEVEL (1) nor REMOTE_NAME_INFO_LEVEL (2): the page names no result");
    c0 = (uint16_t)fle_ld(path, 2);
    c1 = (uint16_t)fle_ld(path + 2, 2);
    c2 = (uint16_t)fle_ld(path + 4, 2);
    if (!((c0 >= 'A' && c0 <= 'Z') || (c0 >= 'a' && c0 <= 'z')) || c1 != ':' || (c2 != '\\' && c2 != 0)) fle_stop(api, "lpLocalPath is not a drive-based path: the page names no result (ERROR_BAD_DEVICE is 'the string is invalid', not defined further)");
    if (c0 != 'C' && c0 != 'c') fle_stop(api, "a drive other than C: (the virtual volume has one drive)");
    R[0] = ERROR_NOT_CONNECTED;
}

#define FLE_ERR_INVALID_HANDLE 6u        /* the code of the other file functions of fle_vfs.c for an invalid or closed handle (inference there: the pages name no code) */

/* kernel32!GetFileInformationByHandle. Learn: 'Retrieves file information for the specified file'; the BY_HANDLE_FILE_INFORMATION structure (52 bytes): dwFileAttributes, ftCreationTime, ftLastAccessTime,
 * ftLastWriteTime (FILETIME each), dwVolumeSerialNumber, nFileSizeHigh, nFileSizeLow, nNumberOfLinks, nFileIndexHigh, nFileIndexLow; 'If the function succeeds, the return value is nonzero'. The attributes,
 * the three times and the size are the file's in the volume. The page does not give the serial number, the link count or the file id of this environment: S-dlg1-fileinfo (a CHOICE WITHOUT A SOURCE): serial number
 * 0x464C5601 (one volume), one link, and a file id that is the order in which the files were first asked about (1, 2, ...; high half 0), stable for a file while it exists. An invalid or closed handle fails
 * with ERROR_INVALID_HANDLE (6, as the other file functions: the page names no code). The engine's zip reader calls it before it reads the archive (decision 0178). */
void fle_imp_GetFileInformationByHandle(void)
{
    static const char api[] = "GetFileInformationByHandle";
    static const void *seen[256];
    static unsigned nseen;
    uint64_t out = R[2], ct, at, wt, size;
    const void *node;
    uint32_t attrs;
    unsigned i;

    if (out == 0) fle_stop(api, "lpFileInformation is NULL: the page gives no behaviour");
    if (!fle_vfs_file_stat(api, R[1], &node, &attrs, &ct, &at, &wt, &size)) {
        fle_set_last_error(FLE_ERR_INVALID_HANDLE);
        R[0] = 0;
        return;
    }
    for (i = 0; i < nseen && seen[i] != node; i++) {}
    if (i == nseen) {
        if (nseen == 256) fle_stop(api, "more than 256 files asked about (a limit of the file id table)");
        seen[nseen++] = node;
    }
    fle_st(out + 0, 4, attrs);
    fle_st(out + 4, 8, ct);
    fle_st(out + 12, 8, at);
    fle_st(out + 20, 8, wt);
    fle_st(out + 28, 4, 0x464C5601u);
    fle_st(out + 32, 4, size >> 32);
    fle_st(out + 36, 4, (uint32_t)size);
    fle_st(out + 40, 4, 1);
    fle_st(out + 44, 4, 0);
    fle_st(out + 48, 4, i + 1);
    R[0] = 1;
}

/* kernel32!SetEndOfFile. Learn: 'Sets the physical file size for the specified file to the current position of the file pointer'; the handle must have GENERIC_WRITE access. The file is cut or (zero-filled)
 * extended at the file pointer. An invalid or closed handle fails with ERROR_INVALID_HANDLE (6); a handle opened without write access: the page names no code, the call stops by name. */
void fle_imp_SetEndOfFile(void)
{
    static const char api[] = "SetEndOfFile";
    int r = fle_vfs_file_set_end(api, R[1]);

    if (r == 0) {
        fle_set_last_error(FLE_ERR_INVALID_HANDLE);
        R[0] = 0;
        return;
    }
    if (r < 0) fle_stop(api, "the handle was opened without write access: the page names no error code for it");
    R[0] = 1;
}

/* ---- EXPLORATION ONLY: msacm32!acmDriverEnum ----
 * The engine enumerates the ACM drivers when it meets an .mp3 sample of the reference project (decision 0178: the first audio clip, channel after the plugin channels). Learn: 'enumerates the available ACM drivers,
 * continuing until there are no more drivers or the callback function returns FALSE'; the return value is zero on success, otherwise an error number (MMSYSERR_INVALFLAG, MMSYSERR_INVALPARAM). This environment
 * has NO ACM driver (no Microsoft MPEG Layer-3 codec, l3codecp.acm): the callback is never called and the call returns 0 (S-dlg1-acm-none, a CHOICE: the real machine has drivers). A NULL callback is
 * MMSYSERR_INVALPARAM (11) and a flag outside ACM_DRIVERENUMF_DISABLED (0x80000000) | ACM_DRIVERENUMF_NOLOCAL (0x40000000) is MMSYSERR_INVALFLAG (10): the codes are named on the page, the numbers are mmsystem.h's. */
void fle_imp_acmDriverEnum(void)
{
    if (R[1] == 0) { R[0] = 11; return; }
    if ((uint32_t)R[8] & ~0xC0000000u) { R[0] = 10; return; }
    R[0] = 0;
}

/* ---- EXPLORATION ONLY: msacm32!acmStreamOpen ----
 * After acmDriverEnum found no driver the engine calls acmStreamOpen for the .mp3 sample (decision 0178: f_6729d0, from the sample loader f_a5ee30). With no ACM driver no conversion stream can be opened:
 * Learn names ACMERR_NOTPOSSIBLE among the return values ('The requested operation cannot be performed'); its number, ACMERR_BASE (512) + 0, is msacm.h's (S-dlg1-acm-none). *phas is left as it is (the page
 * writes it on success). A NULL phas is the 'query' form of the call (ACM_STREAMOPENF_QUERY) and is answered the same way. */
void fle_imp_acmStreamOpen(void)
{
    R[0] = 512;
}

/* ---- EXPLORATION ONLY: msacm32!acmFormatSuggest ----
 * After acmStreamOpen failed the engine asks which PCM format the .mp3 source would convert to (decision 0178: f_673f40). With no ACM driver nothing can be suggested: ACMERR_NOTPOSSIBLE (512, as acmStreamOpen
 * above; S-dlg1-acm-none). The destination format is left untouched. */
void fle_imp_acmFormatSuggest(void)
{
    R[0] = 512;
}

/* ---- EXPLORATION ONLY: msacm32!acmDriverClose ----
 * The engine closes the driver handle after acmFormatSuggest failed (decision 0178: f_a5f380). No driver was ever opened here, so every handle is invalid: MMSYSERR_INVALHANDLE (5; the page lists it among the
 * return values, the number is mmsystem.h's). fdwClose is reserved and must be zero (the page): a non-zero value is MMSYSERR_INVALFLAG (10). */
void fle_imp_acmDriverClose(void)
{
    R[0] = ((uint32_t)R[2] != 0) ? 10 : 5;
}

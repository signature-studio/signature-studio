/* Volume, standard-handle and DuplicateHandle answers for IAT slots reached by a tail jump
 * (docs/decisions/0024-apis-a3-a5-a13.md). Same convention as fle_imports.h: arguments in R[1] (rcx), R[2] (rdx),
 * R[8] (r8), R[9] (r9), then [rsp + 0x28 + 8 * (n - 5)]; the Win32 return value in R[0]; the guest return address stays
 * on the stack for the caller's ret. Single guest thread only. A case the Microsoft pages do not define stops the run
 * through fle_stop (fle_env.h). The host's drives, volumes and handles are never read: the one drive is the virtual
 * volume "C:" of fle_vfs.h. */
#ifndef FLE_VOL_H
#define FLE_VOL_H

#include <stdint.h>

/* The values this environment reports for the virtual volume C:. The pages give the shape of every answer and none of
 * these values (decision 0024: all of them are choices). */
#define FLE_VOL_DRIVE_TYPE_FIXED   3u                      /* DRIVE_FIXED */
#define FLE_VOL_SERIAL             UINT32_C(0x464C4531)    /* "FLE1" */
#define FLE_VOL_MAX_COMPONENT      255u                    /* page: 255 for a file system with long names (= fle_vfs.c) */
#define FLE_VOL_FS_FLAGS           UINT32_C(0x00000006)    /* FILE_CASE_PRESERVED_NAMES | FILE_UNICODE_ON_DISK */
#define FLE_VOL_FS_NAME            "FAT32"
#define FLE_VOL_LABEL              ""
#define FLE_VOL_DOS_DEVICE_TARGET  "\\Device\\HarddiskVolume1" /* the example on the QueryDosDevice page */

/* Handles issued by DuplicateHandle for the current process and thread (decision 0024): FLE_VOL_HANDLE_BASE + 4 * (i + 1),
 * never reused, in a region of FLE_VOL_REGION_SIZE bytes above the global-memory handles (fle_glob.h). */
#define FLE_VOL_HANDLE_BASE UINT64_C(0x7FFA60000000)
#define FLE_VOL_REGION_SIZE UINT64_C(0x10000000)

/* kernel32!GetStdHandle: HANDLE f(DWORD). The process has no standard handles (decision 0018: GetStartupInfoW gives
 * none), so each of the three valid values returns NULL; the page names that case. */
void fle_imp_GetStdHandle(void);

/* kernel32!GetFileType (decision 0238, provisional; it stopped by name before): FILE_TYPE_DISK for an open file of the virtual volume, else FILE_TYPE_UNKNOWN and ERROR_INVALID_HANDLE. */
void fle_imp_GetFileType(void);

/* Stop by name (decision 0078): kernel32!SetStdHandle, GetConsoleMode, GetConsoleOutputCP, WriteConsoleW. None returns.
 * (FlushFileBuffers, SetFilePointerEx and FindFirstFileExW are answered for the files of the virtual volume since decision 0238: fle_vfs.h.) */
void fle_imp_SetStdHandle(void) __attribute__((noreturn));
void fle_imp_GetConsoleMode(void) __attribute__((noreturn));
void fle_imp_GetConsoleOutputCP(void) __attribute__((noreturn));
void fle_imp_WriteConsoleW(void) __attribute__((noreturn));

/* kernel32!GetDriveTypeW: UINT f(LPCWSTR root). GetLogicalDrives: DWORD f(void).
 * GetLogicalDriveStringsW: DWORD f(DWORD nBufferLength, LPWSTR lpBuffer).
 * GetVolumeInformationW: BOOL f(root, volNameBuf, volNameSize, serial*, maxComponent*, flags*, fsNameBuf, fsNameSize).
 * QueryDosDeviceW: DWORD f(LPCWSTR name, LPWSTR target, DWORD ucchMax). */
void fle_imp_GetDriveTypeW(void);
void fle_imp_GetLogicalDrives(void);
void fle_imp_GetLogicalDriveStringsW(void);
void fle_imp_GetVolumeInformationW(void);
void fle_imp_QueryDosDeviceW(void);

/* kernel32!DuplicateHandle: BOOL f(hSrcProc, hSrc, hTgtProc, LPHANDLE out, DWORD access, BOOL inherit, DWORD options).
 * Sources it can duplicate: the pseudo handles of GetCurrentProcess and GetCurrentThread (they become real handles,
 * as the page says), the handles it issued itself, and file-mapping handles (fle_vmap.h). Any other handle stops. */
void fle_imp_DuplicateHandle(void);

/* Number of live process/thread handles issued by DuplicateHandle (for tests and tools). */
uint64_t fle_vol_handle_count(void);

#endif

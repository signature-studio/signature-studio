/* In-memory virtual file system and the kernel32 file answers (docs/decisions/0018-virtual-fs-reg-env.md).
 * The host file system is never touched: the tree starts with one empty volume root "C:\" and only what the
 * guest creates exists. As in fle_imports.h, each stub sets the Win32 return value in R[0] and leaves the guest
 * return address on the stack for the caller's ret. The shared error numbers and helpers are in fle_env.h. */
#ifndef FLE_VFS_H
#define FLE_VFS_H

#include <stdint.h>

#include "fle_env.h"

/* Handle regions (decision 0018). They are above the process heap token (fle_sys.h 0x7FF700000000), the module
 * handles (0x7FF800000000 + 0x10000 * N, N <= 25), and the response pseudo-VAs (0x7FF900000000 + 0x10 * k), and
 * they do not overlap each other. Handle i (0-based) of a region is BASE + 4 * (i + 1): a multiple of 4 like a
 * Windows handle. A handle is only a token; nothing is mapped there. Values are never reused. */
#define FLE_VFS_FILE_BASE UINT64_C(0x7FFA00000000)
#define FLE_VFS_FIND_BASE UINT64_C(0x7FFA10000000)
#define FLE_REG_KEY_BASE  UINT64_C(0x7FFA20000000)
#define FLE_HANDLE_REGION_SIZE UINT64_C(0x10000000)

#define FLE_INVALID_HANDLE_VALUE UINT64_C(0xFFFFFFFFFFFFFFFF)
#define FLE_INVALID_FILE_ATTRIBUTES 0xFFFFFFFFu
#define FLE_INVALID_FILE_SIZE 0xFFFFFFFFu
#define FLE_INVALID_SET_FILE_POINTER 0xFFFFFFFFu

/* Layout sizes (MSDN: WIN32_FIND_DATAW, WIN32_FILE_ATTRIBUTE_DATA). Offsets follow from the member types. */
#define FLE_FIND_DATA_SIZE 592u
#define FLE_FIND_DATA_NAME_OFF 44u
#define FLE_FILE_ATTR_DATA_SIZE 36u

/* The virtual clock: FILETIME of 2021-01-01T00:00:00Z, advanced by one second per change (no host clock). */
#define FLE_VFS_CLOCK_START UINT64_C(132539328000000000)

void fle_imp_CreateFileW(void);
void fle_imp_ReadFile(void);
void fle_imp_WriteFile(void);
void fle_imp_CloseHandle(void);
void fle_imp_SetFilePointer(void);
/* Decision 0238 (load2, PROVISIONAL): answered for the files of the virtual volume (they stopped by name in fle_vol.c, decision 0078, and were answered only by the opt-in harness wraps of decision 0206). */
void fle_imp_SetFilePointerEx(void);
void fle_imp_FlushFileBuffers(void);
void fle_imp_FindFirstFileExW(void);
void fle_imp_GetFileSize(void);
void fle_imp_SetFileTime(void);
void fle_imp_FindFirstFileW(void);
void fle_imp_FindNextFileW(void);
void fle_imp_FindClose(void);
void fle_imp_GetFileAttributesW(void);
void fle_imp_GetFileAttributesExW(void);
void fle_imp_SetFileAttributesW(void);
void fle_imp_CreateDirectoryW(void);
void fle_imp_RemoveDirectoryW(void);
void fle_imp_DeleteFileW(void);
void fle_imp_CopyFileW(void);
void fle_imp_MoveFileW(void);
void fle_imp_GetFullPathNameW(void);
void fle_imp_GetCurrentDirectoryW(void);
void fle_imp_SetCurrentDirectoryW(void);
void fle_imp_GetTempPathW(void);
/* Decision 0132 (PROVISIONAL), wall 22: kernel32!GetTempFileNameW: UINT f(LPCWSTR, LPCWSTR, UINT, LPWSTR); only a nonzero uUnique (fle_vfs.c). */
void fle_imp_GetTempFileNameW(void);
void fle_imp_GetModuleFileNameW(void);
/* kernel32!GetModuleFileNameA: the ANSI form (decision 0102 W7). */
void fle_imp_GetModuleFileNameA(void);
void fle_imp_GetWindowsDirectoryW(void);
void fle_imp_GetSystemDirectoryW(void);

/* version!GetFileVersionInfoSizeW (decision 0098, PROVISIONAL): DWORD f(LPCWSTR lptstrFilename rcx, LPDWORD lpdwHandle rdx). A path that is not in the
 * virtual file system returns 0 with the file-system error (2 or 3) and *lpdwHandle = 0; a path that is stops by name. */
void fle_imp_GetFileVersionInfoSizeW(void);

/* version!GetFileVersionInfoW (decision 0112, PROVISIONAL): BOOL f(LPCWSTR lptstrFilename rcx, DWORD dwHandle edx, DWORD dwLen r8d, LPVOID lpData r9). The failure of
 * GetFileVersionInfoSizeW for the same path (FALSE with the file-system error 2 or 3, lpData untouched); the same stops. version!VerQueryValueW: BOOL f(LPCVOID pBlock
 * rcx, LPCWSTR lpSubBlock rdx, LPVOID *lplpBuffer r8, PUINT puLen r9) stops in every case (no block of this environment exists). */
void fle_imp_GetFileVersionInfoW(void);
void fle_imp_VerQueryValueW(void);

/* Handles that other runtime files issue (decision 0024: duplicated process/thread handles in fle_vol.c, file mapping
 * objects in fle_vmap.c). CloseHandle asks each registered hook first; a hook returns 1 when the value is one of its
 * handles (it has then set R[0] and the last error as CloseHandle must) and 0 otherwise. Up to 4 hooks; registering
 * the same function twice is a no-op. */
void fle_vfs_add_close_hook(int (*hook)(uint64_t h));

/* For fle_vmap.c (CreateFileMappingW): the content and access of an open virtual file. Returns 1 and the data
 * pointer (valid until the next write to the file), the size and whether the handle was opened with read and with
 * write access; returns 0 for NULL, INVALID_HANDLE_VALUE, a closed handle or a find handle (the caller fails with
 * ERROR_INVALID_HANDLE). A value that this file system did not issue stops the run. A file of size 0 has data NULL. */
int fle_vfs_file_info(const char *api, uint64_t h, const uint8_t **data, uint64_t *size, unsigned *can_read, unsigned *can_write);

/* Decision 0178 (PROVISIONAL), for fle_wall_dlg1.c: the node (opaque, stable while the file exists), attributes, creation / last access / last write time (FILETIME ticks) and size of an open file, 0 for an
 * invalid handle; and SetEndOfFile's effect on the file pointer's position (1 done, 0 invalid handle, -1 not opened for writing). */
int fle_vfs_file_stat(const char *api, uint64_t h, const void **node, uint32_t *attrs, uint64_t *ct, uint64_t *at, uint64_t *wt, uint64_t *size);
int fle_vfs_file_set_end(const char *api, uint64_t h);

/* Decision 0183 (wrp1, PROVISIONAL), for fle_load.c: 1 when the UTF-16 path at the guest address names a FILE of the virtual volume (a directory, a missing file or a missing directory: 0). The same path rules as GetFileAttributesW. */
int fle_vfs_guest_is_file(const char *api, uint64_t guest);

/* wintrust!WinVerifyTrust and the crypt32 calls behind the engine's signer-name check (decision 0117, PROVISIONAL; the notes are in fle_vfs.c).
 * The runtime vouches for a file only through a table of CLAIMS that the harness fills before the guest runs (harness/authenticode_claims.h, from
 * harness/seeds/authenticode-claims.tsv, written by tools/authenticode_check.py); a build whose harness registers nothing has an empty table and every
 * call stops by name, as before the decision. A claim is keyed by the guest's path of the file (compared without case). `established` is the
 * tool's result: the digest of the file and the signature value were found to match the embedded signer certificate. It is NOT a statement that the
 * certificate chain, revocation or time were verified (they were not). All strings and byte strings are copied. */
typedef struct {
    const char *guest_path;       /* ASCII, 1..259 characters */
    const char *row;              /* the file name of the table row, for messages */
    int established;              /* 1: digest and signature matched; 0: the row exists but does not hold */
    const char *why;              /* when not established: why (shown in the stop) */
    const char *signer_cn;        /* printable ASCII: what CertGetNameStringW(CERT_NAME_SIMPLE_DISPLAY_TYPE) answers */
    const uint8_t *issuer_der;    /* the signer certificate's issuer Name, DER */
    size_t issuer_len;
    const uint8_t *serial_be;     /* the signer certificate's serial number, unsigned big-endian */
    size_t serial_len;
} fle_trust_claim;

/* Handle region of the WinVerifyTrust state handles, certificate store handles, message handles (tokens: values are never reused). */
#define FLE_TRUST_HANDLE_BASE UINT64_C(0x7FFA90000000)

/* Adds a claim. Returns 0, or -1 when a field is invalid, the path is already in the table, or the table (8 claims) is full. */
int fle_trust_claim_add(const fle_trust_claim *c);
/* Number of claims, number of live WinVerifyTrust states, crypt32 objects (stores + messages + contexts) not yet closed (for tests and tools). */
size_t fle_trust_claim_count(void);
unsigned fle_trust_states_live(void);
unsigned fle_trust_objects_live(void);

/* wintrust.dll!WinVerifyTrust: LONG f(HWND hwnd rcx, GUID *pgActionID rdx, LPVOID pWVTData r8).
 * crypt32.dll: CryptQueryObject (11 arguments), CryptMsgGetParam (CMSG_SIGNER_INFO_PARAM only), CertFindCertificateInStore (CERT_FIND_SUBJECT_CERT only),
 * CertGetNameStringW (CERT_NAME_SIMPLE_DISPLAY_TYPE only), CertFreeCertificateContext, CertCloseStore, CryptMsgClose. */
void fle_imp_WinVerifyTrust(void);
void fle_imp_CryptQueryObject(void);
void fle_imp_CryptMsgGetParam(void);
void fle_imp_CertFindCertificateInStore(void);
void fle_imp_CertGetNameStringW(void);
void fle_imp_CertFreeCertificateContext(void);
void fle_imp_CertCloseStore(void);
void fle_imp_CryptMsgClose(void);

/* Decision 0188 (PROVISIONAL), for the G3 script command `vfsdump`: the bytes of a file of the virtual volume by its UTF-16 guest path (1 = a file, *data valid until the next write to it, NULL for size 0; 0 = not a file). */
int fle_vfs_peek_file(const uint16_t *wpath, const uint8_t **data, uint64_t *size);

/* Decision 0193 (fpick1, PROVISIONAL), for the browser file dialog bridge (fle_filedlg.c): the user's file is put on the volume from the host side, and a file the engine saved is looked at.
 * fle_vfs_make_dirs: 0 done, 1 invalid path, 5 a component is a file. fle_vfs_put_file: `data` is malloc'd and the volume TAKES IT OVER (also on failure); 0 done, 1 invalid path, 2 the target is a directory, 3 it is open,
 * 4 over the 256 MiB limit, 5 a directory component is a file. fle_vfs_file_state: 1 with size, open handles, last-write FILETIME ticks and the bytes (valid until the next write), 0 not a file. */
int fle_vfs_make_dirs(const uint16_t *wdir);
int fle_vfs_put_file(const uint16_t *wpath, uint8_t *data, uint64_t size);
int fle_vfs_file_state(const uint16_t *wpath, uint64_t *size, unsigned *opens, uint64_t *wt, const uint8_t **data);

#endif

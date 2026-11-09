/* Virtual process environment: environment block, command line, startup info
 * (docs/decisions/0018-virtual-fs-reg-env.md). Nothing is read from the host: the environment block
 * starts empty and only SetEnvironmentVariableW fills it.
 *
 * This header is also the common base of fle_vfs.c and fle_reg.c: Win32 error numbers, the stop helper,
 * the wide-string helpers and the stack-argument reader live here so that the three files agree.
 * As in fle_imports.h, the stubs set the Win32 return value in R[0] and leave the guest return address
 * on the stack for the caller's ret. */
#ifndef FLE_ENV_H
#define FLE_ENV_H

#include <stddef.h>
#include <stdint.h>

#include "fle_runtime.h"

/* System error codes (MSDN: System Error Codes 0-499, 500-999 for none of these). */
#define FLE_W32_ERR_SUCCESS             0u
#define FLE_W32_ERR_FILE_NOT_FOUND      2u
#define FLE_W32_ERR_PATH_NOT_FOUND      3u
#define FLE_W32_ERR_ACCESS_DENIED       5u
#define FLE_W32_ERR_INVALID_HANDLE      6u
#define FLE_W32_ERR_NO_MORE_FILES       18u
#define FLE_W32_ERR_SHARING_VIOLATION   32u
#define FLE_W32_ERR_HANDLE_EOF          38u
#define FLE_W32_ERR_FILE_EXISTS         80u
#define FLE_W32_ERR_INVALID_PARAMETER   87u
#define FLE_W32_ERR_BUFFER_OVERFLOW     111u
#define FLE_W32_ERR_INSUFFICIENT_BUFFER 122u
#define FLE_W32_ERR_INVALID_NAME        123u
#define FLE_W32_ERR_NEGATIVE_SEEK       131u
#define FLE_W32_ERR_DIR_NOT_EMPTY       145u
#define FLE_W32_ERR_ALREADY_EXISTS      183u
#define FLE_W32_ERR_ENVVAR_NOT_FOUND    203u
#define FLE_W32_ERR_MORE_DATA           234u
#define FLE_W32_ERR_NO_MORE_ITEMS       259u
#define FLE_W32_ERR_DIRECTORY           267u

/* The virtual paths (decision 0018). FLE_VIRTUAL_DLL_PATH is the PM's value and is not a Windows rule. */
#define FLE_VIRTUAL_DLL_PATH    "C:\\Program Files\\Image-Line\\FL Studio 21\\FLEngine_x64.dll"
/* Decision 0118 (PROVISIONAL): the path of the process's executable file, which GetModuleFileNameW/A(NULL) answers (Learn: "If this parameter is NULL,
 * GetModuleFileName retrieves the path of the executable file of the current process"). The host is FL64.exe (docs/investigations/host-engine-boundary.md);
 * the value is the PM's: the directory of FLE_VIRTUAL_DLL_PATH (decision 0018) and the name of the host exe, the same string as the FL64.exe row of
 * harness/seeds/authenticode-claims.tsv and the registry seed (decision 0117). It is not a Windows rule. */
#define FLE_VIRTUAL_EXE_PATH    "C:\\Program Files\\Image-Line\\FL Studio 21\\FL64.exe"
#define FLE_VIRTUAL_WINDOWS_DIR "C:\\Windows"
#define FLE_VIRTUAL_SYSTEM_DIR  "C:\\Windows\\System32"
/* Decision 0107 (PROVISIONAL): the profile directory of the virtual user, for SHGetFolderPathW(CSIDL_APPDATA). The Learn pages give the form
 * "C:\\Users\\user name\\..." and no user name; the name "user" is a choice of this environment (nothing else in it names a user). */
#define FLE_VIRTUAL_USER_PROFILE_DIR "C:\\Users\\user"
/* Decision 0112 (PROVISIONAL): the user name and the computer name of the virtual process. No Microsoft page gives a value for either; both are choices of
 * this environment. The user name is the one inside FLE_VIRTUAL_USER_PROFILE_DIR (decision 0107). The computer name has the shape of the default name of a
 * Windows 10 installation ("DESKTOP-" and seven characters; the shape is not on a Learn page, see the decision), is upper case and at most
 * MAX_COMPUTERNAME_LENGTH (15) characters, so that the NetBIOS name and the DNS host name are the same string and nothing is truncated or case-folded. */
#define FLE_VIRTUAL_USER_NAME     "user"
#define FLE_VIRTUAL_COMPUTER_NAME "DESKTOP-0000001"
#define FLE_COMPUTERNAME_NETBIOS_MAX 15u

/* Stops the run: prints "fle: <api>: <why>" and aborts. Used for every input that the Microsoft pages
 * leave undefined (nothing is silently turned into a success). */
void fle_stop(const char *api, const char *why) __attribute__((noreturn));

/* Argument n (1-based) of the Microsoft x64 call. Arguments 1-4 are in rcx, rdx, r8, r9; argument n >= 5 is at
 * [rsp + 0x28 + 8 * (n - 5)] when the stub starts, because [rsp] holds the guest return address and
 * [rsp + 8, rsp + 0x28) is the caller's 32-byte shadow space. */
static inline uint64_t fle_arg(unsigned n)
{
    static const unsigned reg[4] = { 1, 2, 8, 9 };
    if (n >= 1 && n <= 4) return R[reg[n - 1]];
    return fle_ld(R[4] + 0x28u + 8u * (n - 5u), 8);
}

/* Wide-string helpers (UTF-16 code units). Case folding is ASCII only: a unit above 0x7F is compared exactly
 * (assumption: Windows' own folding of other letters is not modelled). */
size_t fle_w_len(const uint16_t *s);
uint16_t fle_w_fold(uint16_t c);
/* -1, 0, 1: ordering of the two names with ASCII case folding (equal names give 0). */
int fle_w_icmp(const uint16_t *a, size_t an, const uint16_t *b, size_t bn);
/* Reads the guest string at `guest` into buf (capacity `cap` units including the NUL). Returns the length.
 * A string of cap or more units stops the run. */
size_t fle_w_read(const char *api, uint64_t guest, uint16_t *buf, size_t cap);
/* Same, into a malloc'd copy. A string longer than maxlen units stops the run. Free with free(). */
uint16_t *fle_w_read_alloc(const char *api, uint64_t guest, size_t maxlen, size_t *len);
/* Copies `units` code units to the guest address `dst`. */
void fle_w_write(uint64_t dst, const uint16_t *src, size_t units);
/* malloc'd UTF-16 copy of an ASCII string (all characters below 0x80). */
uint16_t *fle_w_from_ascii(const char *s);

/* The value of an environment variable, or NULL. `name` is NUL-terminated; names are compared without case. */
const uint16_t *fle_env_get(const uint16_t *name);

/* kernel32!GetEnvironmentVariableW: DWORD GetEnvironmentVariableW(LPCWSTR lpName, LPWSTR lpBuffer, DWORD nSize). */
void fle_imp_GetEnvironmentVariableW(void);
/* kernel32!SetEnvironmentVariableW: BOOL SetEnvironmentVariableW(LPCWSTR lpName, LPCWSTR lpValue). */
void fle_imp_SetEnvironmentVariableW(void);
/* kernel32!ExpandEnvironmentStringsW: DWORD ExpandEnvironmentStringsW(LPCWSTR lpSrc, LPWSTR lpDst, DWORD nSize). */
void fle_imp_ExpandEnvironmentStringsW(void);
/* kernel32!GetCommandLineW: LPWSTR GetCommandLineW(void). */
void fle_imp_GetCommandLineW(void);
/* kernel32!GetStartupInfoW: void GetStartupInfoW(LPSTARTUPINFOW lpStartupInfo). */
void fle_imp_GetStartupInfoW(void);
/* kernel32!GetCommandLineA: LPSTR GetCommandLineA(void). The ASCII text of GetCommandLineW (decision 0078). */
void fle_imp_GetCommandLineA(void);
/* kernel32!GetEnvironmentStringsW: LPWCH f(void). kernel32!FreeEnvironmentStringsW: BOOL f(LPWCH penv rcx) (decision 0078). */
void fle_imp_GetEnvironmentStringsW(void);
void fle_imp_FreeEnvironmentStringsW(void);
/* kernel32!GetStartupInfoA: void f(LPSTARTUPINFOA rcx). kernel32!GetEnvironmentStrings (the ANSI export): LPCH f(void).
 * kernel32!FreeEnvironmentStringsA: BOOL f(LPCH penv rcx) (decision 0102 W7). */
void fle_imp_GetStartupInfoA(void);
void fle_imp_GetEnvironmentStrings(void);
void fle_imp_FreeEnvironmentStringsA(void);
/* shell32!SHGetFolderPathW and shfolder!SHGetFolderPathW (one function, two response-table rows): HRESULT f(HWND hwnd rcx, int csidl edx,
 * HANDLE hToken r8, DWORD dwFlags r9d, LPWSTR pszPath [rsp+0x28] on entry). Answers CSIDL_FONTS (0x14, decision 0103) and CSIDL_APPDATA (0x1a,
 * decision 0107), both provisional; every other csidl stops. */
void fle_imp_SHGetFolderPathW(void);

/* Computer name and user name (decision 0112, PROVISIONAL). All four take an in/out size pointer; the name is FLE_VIRTUAL_COMPUTER_NAME or FLE_VIRTUAL_USER_NAME.
 * kernel32!GetComputerNameExW: BOOL f(COMPUTER_NAME_FORMAT NameType ecx, LPWSTR lpBuffer rdx, LPDWORD nSize r8). Formats 0 (NetBIOS), 4 (PhysicalNetBIOS), 1 (DnsHostname)
 * and 5 (PhysicalDnsHostname) are answered; 2, 3, 6, 7 (domain and fully qualified), 8 (ComputerNameMax) and anything else stop by name. Too small: FALSE,
 * ERROR_MORE_DATA (234), *nSize = the size needed in TCHARs including the NUL. kernel32!GetComputerNameExA: the same with 8-bit characters.
 * kernel32!GetComputerNameW: BOOL f(LPWSTR lpBuffer rcx, LPDWORD nSize rdx), the NetBIOS name; too small: ERROR_BUFFER_OVERFLOW (111).
 * advapi32!GetUserNameW: BOOL f(LPWSTR lpBuffer rcx, LPDWORD pcbBuffer rdx); on success *pcbBuffer counts the NUL; too small: ERROR_INSUFFICIENT_BUFFER (122). */
void fle_imp_GetComputerNameExW(void);
void fle_imp_GetComputerNameExA(void);
void fle_imp_GetComputerNameW(void);
void fle_imp_GetUserNameW(void);

#endif

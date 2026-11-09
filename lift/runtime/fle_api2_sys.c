/* Decision 0210 (api2, PROVISIONAL, opt-in): kernel32 system, console and link answers (fle_api2.h). A call whose page defines a failure for this environment returns that failure (no process is created, no console exists); a form the page leaves open stops
 * by name (fle_stop). Nothing is answered with an invented success. */
#include <stdint.h>
#include <stdio.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_vmap.h"
#include "fle_api2.h"

#define ERR_INVALID_HANDLE 6u
#define ERR_INVALID_PARAMETER 87u
#define ERR_PRIVILEGE_NOT_HELD 1314u

static void fail(uint32_t err)
{
    fle_set_last_error(err);
    R[0] = 0;
}

/* kernel32!GetLargePageMinimum: SIZE_T f(void). Learn: "Retrieves the minimum size of a large page. ... If the processor does not support large pages, the return value is zero." S-api2-2 (an environment setting, not a page value): the virtual
 * machine has no large pages (its memory is a linear memory in which VirtualAlloc(MEM_LARGE_PAGES) cannot be honoured), so the answer is zero; the callers (Blend2D's allocator) then do not ask for MEM_LARGE_PAGES. Wine answers 2 MiB. */
void fle_imp_GetLargePageMinimum(void)
{
    R[0] = 0;
}

/* kernel32!VirtualQueryEx: SIZE_T f(HANDLE hProcess rcx, LPCVOID lpAddress rdx, PMEMORY_BASIC_INFORMATION lpBuffer r8, SIZE_T dwLength r9). Learn: "Retrieves information about a range of pages within the virtual address space of a specified process";
 * the handle must have PROCESS_QUERY_INFORMATION. The process of the virtual machine is the only one and its handle is the pseudo handle of GetCurrentProcess ((HANDLE)-1, Learn GetCurrentProcess): then the answer is VirtualQuery's. Any other value is
 * not a process handle of this machine (no other process exists, OpenProcess is not answered): 0 with ERROR_INVALID_HANDLE (6; the page names no code, System Error Codes: "The handle is invalid", S-api2-3). */
void fle_imp_VirtualQueryEx(void)
{
    uint64_t h = R[1], addr = R[2], buf = R[8], len = R[9];

    if (h != UINT64_C(0xFFFFFFFFFFFFFFFF)) {
        fail(ERR_INVALID_HANDLE);
        return;
    }
    R[0] = fle_api2_call(fle_imp_VirtualQuery, addr, buf, len, 0, 0, 0, 0, 0);
}

/* kernel32!OutputDebugStringA: void f(LPCSTR lpOutputString rcx). Learn: "If the application has no debugger and the system debugger is not active, OutputDebugString does nothing." The virtual machine has no debugger: nothing happens (the text is not kept). */
void fle_imp_OutputDebugStringA(void)
{
}

/* kernel32!WriteConsoleA: BOOL f(HANDLE hConsoleOutput rcx, const VOID *lpBuffer rdx, DWORD nNumberOfCharsToWrite r8, LPDWORD lpNumberOfCharsWritten r9, LPVOID lpReserved [5]).
 * Learn: hConsoleOutput is "A handle to the console screen buffer"; "If the function fails, the return value is zero. To get extended error information, call GetLastError". The process has no console (GetStdHandle answers NULL, fle_vol.c) and no handle of
 * the virtual machine is a console screen buffer, so every handle is not one: 0 with ERROR_INVALID_HANDLE (6; the page names no code, S-api2-3). *lpNumberOfCharsWritten is not touched (the page: set on success). */
void fle_imp_WriteConsoleA(void)
{
    fail(ERR_INVALID_HANDLE);
}

/* kernel32!ReadConsoleW: BOOL f(HANDLE hConsoleInput rcx, LPVOID lpBuffer rdx, DWORD nNumberOfCharsToRead r8, LPDWORD lpNumberOfCharsRead r9, PCONSOLE_READCONSOLE_CONTROL pInputControl [5]). Same reading as WriteConsoleA. */
void fle_imp_ReadConsoleW(void)
{
    fail(ERR_INVALID_HANDLE);
}

/* kernel32!GetConsoleCP: UINT f(void). Learn: "Retrieves the input code page used by the console associated with the calling process"; "If the function fails, the return value is zero". The process has no console: 0 with ERROR_INVALID_HANDLE (6; the page names
 * no code and does not say what a process without a console gets: S-api2-4, a CHOICE; Wine 10.0 was run for a process without a console and gives the same, UNOFFICIAL). */
void fle_imp_GetConsoleCP(void)
{
    fail(ERR_INVALID_HANDLE);
}

/* kernel32!CreateSymbolicLinkW: BOOLEAN f(LPCWSTR lpSymlinkFileName rcx, LPCWSTR lpTargetFileName rdx, DWORD dwFlags r8d). Learn: "Creates a symbolic link"; "If the function succeeds, the return value is nonzero. ... If the function fails, the return value is zero";
 * "Symbolic link creation requires the SE_CREATE_SYMBOLIC_LINK_NAME privilege" (a standard token does not hold it; Developer Mode and SYMBOLIC_LINK_FLAG_ALLOW_UNPRIVILEGED_CREATE 0x2 lift that, and the virtual machine is not in Developer Mode) and the System Error Codes
 * page names ERROR_PRIVILEGE_NOT_HELD (1314, "A required privilege is not held by the client"). The process token of the virtual machine is not elevated (fle_token.c, S-token-not-elevated: a choice of that file, not a page value), which is the token that lacks the privilege (an inference from general knowledge, not from the page), so the call fails with 1314 without touching the file system. Flags other than 0 and SYMBOLIC_LINK_FLAG_DIRECTORY
 * (0x1) stop (the page describes 0x2 only for Developer Mode). NULL names stop (the page gives no behaviour). */
void fle_imp_CreateSymbolicLinkW(void)
{
    static const char api[] = "CreateSymbolicLinkW";
    uint32_t fl = (uint32_t)R[8];

    if (R[1] == 0) fle_stop(api, "lpSymlinkFileName is NULL: the page gives no behaviour");
    if (R[2] == 0) fle_stop(api, "lpTargetFileName is NULL: the page gives no behaviour");
    if (fl & ~1u) fle_stop(api, "dwFlags has a bit other than SYMBOLIC_LINK_FLAG_DIRECTORY: the page ties 0x2 to Developer Mode, which the virtual machine does not model");
    fail(ERR_PRIVILEGE_NOT_HELD);
}

/* Process and thread answers for IAT slots reached by a tail jump (docs/decisions/0020-process-and-time.md,
 * sources in docs/investigations/import-proctime-spec.md). Same convention as fle_imports.h: arguments in
 * R[1] (rcx), R[2] (rdx), R[8] (r8), R[9] (r9); the Win32 return value in R[0]; the guest return address
 * stays on the stack for the caller's ret. A function that returns VOID leaves R[0] alone.
 * Single guest thread only. A case that the Microsoft pages do not define, and every request to end the
 * process, stops the run with abort() and a message that starts with "fle:" (fle_stop in fle_env.h). */
#ifndef FLE_PROC_H
#define FLE_PROC_H

#include <stdint.h>

/* Pseudo handles. GetCurrentProcess: the page says "currently (HANDLE)-1". GetCurrentThread: the page says
 * only "a special constant"; -2 is the commonly known value and is NOT confirmed by the pages read. */
#define FLE_PSEUDO_PROCESS_HANDLE UINT64_C(0xFFFFFFFFFFFFFFFF)
#define FLE_PSEUDO_THREAD_HANDLE  UINT64_C(0xFFFFFFFFFFFFFFFE)

/* The emulated process id: a setting of this lift environment (no page defines a value). */
#define FLE_EMULATED_PID 0x1000u

/* Error-mode bits (MSDN: SetErrorMode, SetThreadErrorMode). */
#define FLE_SEM_FAILCRITICALERRORS     0x0001u
#define FLE_SEM_NOGPFAULTERRORBOX      0x0002u
#define FLE_SEM_NOALIGNMENTFAULTEXCEPT 0x0004u
#define FLE_SEM_NOOPENFILEERRORBOX     0x8000u

/* Priority classes and modes (MSDN: SetPriorityClass, SetThreadPriority). */
#define FLE_NORMAL_PRIORITY_CLASS        0x00000020u
#define FLE_IDLE_PRIORITY_CLASS          0x00000040u
#define FLE_HIGH_PRIORITY_CLASS          0x00000080u
#define FLE_REALTIME_PRIORITY_CLASS      0x00000100u
#define FLE_BELOW_NORMAL_PRIORITY_CLASS  0x00004000u
#define FLE_ABOVE_NORMAL_PRIORITY_CLASS  0x00008000u
#define FLE_PROCESS_MODE_BACKGROUND_BEGIN 0x00100000u
#define FLE_PROCESS_MODE_BACKGROUND_END   0x00200000u
#define FLE_THREAD_MODE_BACKGROUND_BEGIN  0x00010000
#define FLE_THREAD_MODE_BACKGROUND_END    0x00020000

/* System error codes used (MSDN: System Error Codes 0-499). */
#define FLE_ERR_THREAD_MODE_ALREADY_BACKGROUND  400u
#define FLE_ERR_THREAD_MODE_NOT_BACKGROUND      401u
#define FLE_ERR_PROCESS_MODE_ALREADY_BACKGROUND 402u
#define FLE_ERR_PROCESS_MODE_NOT_BACKGROUND     403u

/* kernel32!GetCurrentProcess: HANDLE f(void). GetCurrentProcessId: DWORD f(void). GetCurrentThread: HANDLE f(void). */
void fle_imp_GetCurrentProcess(void);
void fle_imp_GetCurrentProcessId(void);
void fle_imp_GetCurrentThread(void);

/* kernel32!SetLastError: VOID SetLastError(DWORD). ecx. Sets the value that GetLastError returns. */
void fle_imp_SetLastError(void);

/* kernel32!SetErrorMode: UINT SetErrorMode(UINT). ecx. Returns the previous mode.
 * kernel32!SetThreadErrorMode: BOOL SetThreadErrorMode(DWORD dwNewMode, LPDWORD lpOldMode). ecx, rdx. */
void fle_imp_SetErrorMode(void);
void fle_imp_SetThreadErrorMode(void);

/* kernel32!SetPriorityClass: BOOL f(HANDLE, DWORD). rcx, edx. kernel32!SetThreadPriority: BOOL f(HANDLE, int).
 * rcx, edx. Only the pseudo handles are accepted. */
void fle_imp_SetPriorityClass(void);
void fle_imp_SetThreadPriority(void);
/* Decision 0129 (wall 10): kernel32!GetThreadPriority: int f(HANDLE rcx); see fle_proc.c. */
void fle_imp_GetThreadPriority(void);

/* kernel32!SwitchToThread: BOOL f(void). Returns 0: there is no other thread to run. */
void fle_imp_SwitchToThread(void);

/* kernel32!OutputDebugStringW: VOID f(LPCWSTR). The run has no debugger: does nothing (the page says so). */
void fle_imp_OutputDebugStringW(void);

/* kernel32!ExitProcess: VOID f(UINT). kernel32!TerminateProcess: BOOL f(HANDLE, UINT).
 * kernel32!RaiseException: VOID f(DWORD, DWORD, DWORD, const ULONG_PTR *).
 * None of the three returns: each stops the run with abort() and a "fle:" message that names the request. */
void fle_imp_ExitProcess(void) __attribute__((noreturn));
void fle_imp_TerminateProcess(void) __attribute__((noreturn));
void fle_imp_RaiseException(void) __attribute__((noreturn));

/* Stop by name (decision 0078): kernel32!RtlCaptureContext, RtlLookupFunctionEntry, RtlVirtualUnwind, RtlUnwindEx, RtlPcToFileHeader,
 * UnhandledExceptionFilter, SetUnhandledExceptionFilter, IsDebuggerPresent (and EncodePointer until decision 0102). The lifted dsp_ippv2_x64.dll calls them
 * only on fatal-error and exception paths. None returns: each stops the run with abort() and a "fle:" message that names the call. */
void fle_imp_RtlCaptureContext(void) __attribute__((noreturn));
void fle_imp_RtlLookupFunctionEntry(void) __attribute__((noreturn));
void fle_imp_RtlVirtualUnwind(void) __attribute__((noreturn));
void fle_imp_RtlUnwindEx(void) __attribute__((noreturn));
void fle_imp_RtlPcToFileHeader(void) __attribute__((noreturn));
void fle_imp_UnhandledExceptionFilter(void) __attribute__((noreturn));
void fle_imp_SetUnhandledExceptionFilter(void) __attribute__((noreturn));
void fle_imp_IsDebuggerPresent(void); /* decision 0126 wall 99: returns 0 (no debugger: S-no-debugger); it stopped by name in decision 0078 */
/* kernel32!SetHandleCount: UINT SetHandleCount(UINT uNumber ecx) (decision 0102 W10). */
void fle_imp_SetHandleCount(void);
/* kernel32!EncodePointer / DecodePointer: PVOID f(PVOID rcx) (decision 0102 W8; EncodePointer was a named stop in decision 0078). */
void fle_imp_EncodePointer(void);
void fle_imp_DecodePointer(void);

/* shcore!GetProcessDpiAwareness: HRESULT f(HANDLE hprocess, PROCESS_DPI_AWARENESS *value). rcx, rdx. Decision 0099 (provisional): the current process
 * (NULL or the GetCurrentProcess pseudo handle) is PROCESS_DPI_UNAWARE (0); a NULL pointer or any other handle is E_INVALIDARG. */
#define FLE_PROCESS_DPI_UNAWARE 0u
#define FLE_PROCESS_PER_MONITOR_DPI_AWARE 2u   /* shellscalingapi.h PROCESS_PER_MONITOR_DPI_AWARE; decision 0212: reported instead when FLE_SCREEN_DPI (fle_screen.h) is above 96 */
void fle_imp_GetProcessDpiAwareness(void);

/* rpcrt4!UuidCreate: RPC_STATUS f(UUID *Uuid rcx). Decision 0118 wall 7 (provisional): writes a version 4 (RFC 4122 section 4.4) UUID made from a fixed-seed
 * pseudo-random sequence (SplitMix64, seed FLE_UUID_SEED) and returns RPC_S_OK (0). The sequence is the same in every run; the n-th call gets the n-th value. */
#define FLE_UUID_SEED UINT64_C(0x5555555555555555)
void fle_imp_UuidCreate(void);

#endif

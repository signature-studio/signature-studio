/* Import stubs for IAT slots reached by a tail jump (jmp [slot]). Each stub
 * sets the Win32 return value in R[0]; the guest return address is left on the
 * stack for the caller's ret to consume. */
#ifndef FLE_IMPORTS_H
#define FLE_IMPORTS_H

#include <stdint.h>

/* The thread id the guest sees (docs/decisions/0056-virtual-thread-id.md): one fixed value, the same on every host.
 * A setting of this lift environment, not a value from a Microsoft page. Never 0. The host's real tid
 * (fle_host_tid) is not shown to the guest. */
#define FLE_EMULATED_TID 0x1001u

/* kernel32!GetCurrentThreadId: DWORD GetCurrentThreadId(void). */
void fle_imp_GetCurrentThreadId(void);

/* kernel32!TlsAlloc: DWORD TlsAlloc(void). */
void fle_imp_TlsAlloc(void);

/* kernel32!LocalAlloc: HLOCAL LocalAlloc(UINT uFlags, SIZE_T uBytes). */
void fle_imp_LocalAlloc(void);

/* kernel32!TlsSetValue: BOOL TlsSetValue(DWORD dwTlsIndex, LPVOID lpTlsValue). */
void fle_imp_TlsSetValue(void);

/* kernel32!TlsGetValue: LPVOID TlsGetValue(DWORD dwTlsIndex). */
void fle_imp_TlsGetValue(void);

/* kernel32!GetLastError: DWORD GetLastError(void). */
void fle_imp_GetLastError(void);

/* kernel32!TlsFree: BOOL TlsFree(DWORD dwTlsIndex). ecx. Frees an index that TlsAlloc handed out (decision 0078). */
void fle_imp_TlsFree(void);

/* Fiber local storage (decision 0078): kernel32!FlsAlloc: DWORD f(PFLS_CALLBACK_FUNCTION rcx). FlsFree: BOOL f(DWORD ecx).
 * FlsGetValue: PVOID f(DWORD ecx). FlsSetValue: BOOL f(DWORD ecx, PVOID rdx). One thread, one set of slots. */
void fle_imp_FlsAlloc(void);
void fle_imp_FlsFree(void);
void fle_imp_FlsGetValue(void);
void fle_imp_FlsSetValue(void);

/* kernel32!VerSetConditionMask: ULONGLONG VerSetConditionMask(ULONGLONG ConditionMask,
 * DWORD TypeMask, BYTE Condition). Builds the mask only; reads no OS state. */
void fle_imp_VerSetConditionMask(void);

/* kernel32!LocalFree: HLOCAL LocalFree(HLOCAL hMem). */
void fle_imp_LocalFree(void);

/* Sets the value that kernel32!GetLastError returns (runtime-internal, not an import). */
void fle_set_last_error(uint32_t code);
/* Decision 0126 (wall 75): kernel32!FormatMessageW for FORMAT_MESSAGE_FROM_SYSTEM | IGNORE_INSERTS (the 241 messages of the System Error Codes page 0 to 499). */
void fle_imp_FormatMessageW(void);

#endif

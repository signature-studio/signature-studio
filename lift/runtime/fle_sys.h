/* Synchronisation, time, memory and system-information answers for IAT slots reached by a tail jump
 * (docs/decisions/0017-sync-time-memory.md). Same convention as fle_imports.h: arguments in
 * R[1] (rcx), R[2] (rdx), R[8] (r8), R[9] (r9); the Win32 return value in R[0]; the guest return
 * address stays on the stack for the caller's ret. A function that returns VOID leaves R[0] alone.
 * Single guest thread only. A case that would wait forever, or that the Microsoft pages do not
 * define, stops the run with abort() and a message that starts with "fle:". */
#ifndef FLE_SYS_H
#define FLE_SYS_H

#include <stdint.h>

/* ---- Virtual clock (one for every time API; decision 0017) ----
 * The clock counts 100 ns ticks since a fictitious boot. QueryPerformanceFrequency reports
 * FLE_QPF_HZ, so QueryPerformanceCounter returns the tick count itself. GetTickCount and
 * timeGetTime return the tick count in milliseconds, modulo 2^32. Time moves only in two cases:
 * Sleep(ms) adds ms * 10000 ticks, and each read of the clock (QueryPerformanceCounter,
 * GetTickCount, timeGetTime) adds FLE_CLOCK_READ_TICKS after it returns its value, so a loop that
 * polls the clock ends. */
#define FLE_QPF_HZ            UINT64_C(10000000)
#define FLE_TICKS_PER_MS      UINT64_C(10000)
#define FLE_CLOCK_START_MS    UINT64_C(3600000)
#define FLE_CLOCK_READ_TICKS  UINT64_C(10)

/* The current tick count, without advancing the clock (for tests and tools). */
uint64_t fle_sys_clock_ticks(void);

/* ---- Memory ---- */
#define FLE_PAGE_SIZE        UINT64_C(4096)
#define FLE_ALLOC_GRANULARITY UINT64_C(65536)

/* The application address range that GetSystemInfo reports (lpMinimumApplicationAddress, lpMaximumApplicationAddress).
 * The values are not on the Microsoft pages (see fle_sys.c, GetSystemInfo); VirtualQuery uses the upper one. */
#define FLE_MIN_APP_ADDR UINT64_C(0x10000)
#define FLE_MAX_APP_ADDR UINT64_C(0x7FFFFFFEFFFF)

/* Token returned by GetProcessHeap. Nothing is mapped there. */
#define FLE_PROCESS_HEAP     UINT64_C(0x7FF700000000)

/* Number of live VirtualAlloc regions (for tests and tools). */
uint64_t fle_sys_region_count(void);

/* Read access to the VirtualAlloc region holding `addr`, for fle_vmap.c (VirtualQuery, VirtualProtect;
 * decision 0024). pages[i] is 0 while page i is only reserved, else the PAGE_* value of the committed page.
 * alloc_protect is the flProtect of the call that reserved the region. Returns 0 when `addr` is in no region. */
typedef struct {
    uint64_t base;
    uint64_t size;
    uint32_t alloc_protect;
    const uint8_t *pages;
} fle_sys_region_view;
int fle_sys_region_at(uint64_t addr, fle_sys_region_view *out);
/* Changes the protection of the committed pages [from, to) of one region (page aligned; the caller has checked the
 * range). win_prot is PAGE_NOACCESS, READONLY, READWRITE, EXECUTE, EXECUTE_READ or EXECUTE_READWRITE. */
void fle_sys_set_protection(uint64_t from, uint64_t to, uint32_t win_prot);

#ifdef FLE_VMEM_ARENA
/* The arena of decision 0045 (wasm; fle_host.h): ranges of one linear-memory block, used instead of the host's mmap by
 * VirtualAlloc (fle_sys.c) and by the file-mapping objects (fle_vmap.c). reserve claims `size` bytes (a multiple of the
 * page size) at `want`, or, with want 0, at a FLE_ALLOC_GRANULARITY aligned place; it returns the base or 0. The memory
 * is not zeroed. release gives back the range that starts at `base`. */
uint64_t fle_sys_host_reserve(uint64_t want, uint64_t size);
void fle_sys_host_release(uint64_t base);
#endif

/* kernel32!Sleep: VOID Sleep(DWORD dwMilliseconds). rcx = ms. INFINITE stops the run. */
void fle_imp_Sleep(void);

/* kernel32 critical sections: VOID f(LPCRITICAL_SECTION). rcx = the object. */
void fle_imp_InitializeCriticalSection(void);
void fle_imp_EnterCriticalSection(void);
void fle_imp_LeaveCriticalSection(void);
void fle_imp_DeleteCriticalSection(void);

/* kernel32!InitializeCriticalSectionAndSpinCount: BOOL f(LPCRITICAL_SECTION rcx, DWORD dwSpinCount edx). Always 1 (decision 0078).
 * kernel32!InitializeSListHead: VOID f(PSLIST_HEADER rcx). kernel32!InterlockedFlushSList: PSLIST_ENTRY f(PSLIST_HEADER rcx);
 * an initialized head is always empty here, so the result is NULL (decision 0078). */
void fle_imp_InitializeCriticalSectionAndSpinCount(void);
void fle_imp_InitializeSListHead(void);
void fle_imp_InterlockedFlushSList(void);
/* Decision 0126 (wall 82): kernel32!InterlockedPushEntrySList and InterlockedPopEntrySList: PSLIST_ENTRY f(PSLIST_HEADER, PSLIST_ENTRY) / f(PSLIST_HEADER); the list state is kept outside the header (fle_sys.c). */
void fle_imp_InterlockedPushEntrySList(void);
void fle_imp_InterlockedPopEntrySList(void);

/* kernel32 slim reader/writer locks and condition variables (decision 0092, provisional). All VOID except Sleep. The lock and the
 * condition variable are never read or written: the lock state is kept in a table keyed by the lock's address (an address with no
 * entry is an unlocked lock). rcx = the PSRWLOCK (Acquire/Release) or the PCONDITION_VARIABLE (Wake/WakeAll).
 * Acquire: an unlocked lock is taken; exclusive over exclusive or shared, and shared over exclusive, stop the run (the page: deadlock);
 * shared over shared adds one holder. Release: the same mode as the acquisition, else the run stops (also for a lock not held).
 * Wake, WakeAll: nothing waits, nothing happens, R[0] is not touched.
 * kernel32!SleepConditionVariableSRW: BOOL f(PCONDITION_VARIABLE rcx, PSRWLOCK rdx, DWORD dwMilliseconds r8d, ULONG Flags r9d).
 * It stops the run for every input (INFINITE, finite, 0, and a lock that is not held); the message holds the two values. */
void fle_imp_AcquireSRWLockExclusive(void);
void fle_imp_AcquireSRWLockShared(void);
void fle_imp_ReleaseSRWLockExclusive(void);
void fle_imp_ReleaseSRWLockShared(void);
void fle_imp_WakeConditionVariable(void);
void fle_imp_WakeAllConditionVariable(void);
void fle_imp_SleepConditionVariableSRW(void) __attribute__((noreturn));

/* kernel32!GetTickCount: DWORD GetTickCount(void). winmm!timeGetTime: DWORD timeGetTime(void). */
void fle_imp_GetTickCount(void);
void fle_imp_timeGetTime(void);

/* kernel32!QueryPerformanceCounter(LARGE_INTEGER*), QueryPerformanceFrequency(LARGE_INTEGER*):
 * BOOL. rcx = the output. */
void fle_imp_QueryPerformanceCounter(void);
void fle_imp_QueryPerformanceFrequency(void);

/* kernel32!GetSystemInfo: VOID GetSystemInfo(LPSYSTEM_INFO). rcx = the output (48 bytes). */
void fle_imp_GetSystemInfo(void);

/* kernel32!VirtualAlloc: LPVOID VirtualAlloc(LPVOID lpAddress, SIZE_T dwSize, DWORD flAllocationType,
 * DWORD flProtect). rcx, rdx, r8, r9. kernel32!VirtualFree: BOOL VirtualFree(LPVOID lpAddress,
 * SIZE_T dwSize, DWORD dwFreeType). rcx, rdx, r8. */
void fle_imp_VirtualAlloc(void);
void fle_imp_VirtualFree(void);

/* kernel32!GetProcessHeap: HANDLE GetProcessHeap(void). kernel32!HeapAlloc: LPVOID HeapAlloc(HANDLE,
 * DWORD dwFlags, SIZE_T dwBytes). kernel32!HeapFree: BOOL HeapFree(HANDLE, DWORD dwFlags, LPVOID). */
void fle_imp_GetProcessHeap(void);
void fle_imp_HeapAlloc(void);
void fle_imp_HeapFree(void);
/* kernel32!HeapCreate: HANDLE HeapCreate(DWORD flOptions, SIZE_T dwInitialSize, SIZE_T dwMaximumSize). kernel32!HeapSetInformation:
 * BOOL HeapSetInformation(HANDLE, HEAP_INFORMATION_CLASS, PVOID, SIZE_T) (decision 0102 W6). */
void fle_imp_HeapCreate(void);
void fle_imp_HeapSetInformation(void);
/* kernel32!HeapSize: SIZE_T HeapSize(HANDLE, DWORD dwFlags, LPCVOID lpMem) (decision 0102 W11; a named stop in decision 0078).
 * kernel32!HeapReAlloc: LPVOID HeapReAlloc(HANDLE, DWORD dwFlags, LPVOID lpMem, SIZE_T dwBytes) (decision 0113 wall 1; a named stop in decision 0078). */
void fle_imp_HeapSize(void);
void fle_imp_HeapReAlloc(void);

#endif

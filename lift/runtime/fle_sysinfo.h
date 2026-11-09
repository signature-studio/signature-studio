/* System-information answers beyond GetSystemInfo (docs/decisions/0034-sysinfo.md; the pages read are listed in
 * docs/investigations/sysinfo-run.md). Same convention as fle_imports.h: arguments in R[1] (rcx), R[2] (rdx);
 * the Win32 return value in R[0]; the guest return address stays on the stack for the caller's ret. A function
 * that returns VOID leaves R[0] alone. Single guest thread only (decision 0017). A case that the Microsoft pages
 * do not define stops the run with abort() and a message that starts with "fle:" (fle_stop in fle_env.h).
 *
 * Every number marked "setting" below is a value of this headless, virtual environment. No page fixes it and it
 * is not the value of any real machine. */
#ifndef FLE_SYSINFO_H
#define FLE_SYSINFO_H

#include <stdint.h>

#include "fle_sys.h"

/* Setting: the one logical processor of the environment (decision 0017: one guest thread). It is the system
 * affinity mask here, the initial process affinity mask, and the processor mask of every record that
 * GetLogicalProcessorInformation returns; fle_sys.c reports the same bit as dwActiveProcessorMask. */
#define FLE_SYSINFO_PROC_MASK UINT64_C(1)

/* MEMORYSTATUS on x64 (MSDN: MEMORYSTATUS): DWORD dwLength, DWORD dwMemoryLoad and six SIZE_T members, 56 bytes.
 * The guest sets dwLength to 0x38 itself before calling (0x12622a2). */
#define FLE_MEMSTATUS_SIZE 56u

/* Settings: nothing is tracked; the values are fixed. 4 GiB of physical memory, half of it available, a commit
 * limit of 8 GiB with 6 GiB available, and the whole user range (FLE_MIN_APP_ADDR .. FLE_MAX_APP_ADDR, the
 * values GetSystemInfo reports) as total and available virtual memory. dwMemoryLoad is 50 (half of the physical
 * memory in use, as the two physical values imply). */
#define FLE_MEMSTATUS_TOTAL_PHYS      (UINT64_C(4) << 30)
#define FLE_MEMSTATUS_AVAIL_PHYS      (UINT64_C(2) << 30)
#define FLE_MEMSTATUS_TOTAL_PAGEFILE  (UINT64_C(8) << 30)
#define FLE_MEMSTATUS_AVAIL_PAGEFILE  (UINT64_C(6) << 30)
#define FLE_MEMSTATUS_TOTAL_VIRTUAL   (FLE_MAX_APP_ADDR - FLE_MIN_APP_ADDR + UINT64_C(1))
#define FLE_MEMSTATUS_AVAIL_VIRTUAL   FLE_MEMSTATUS_TOTAL_VIRTUAL
#define FLE_MEMSTATUS_LOAD            50u

/* SYSTEM_LOGICAL_PROCESSOR_INFORMATION on x64 (MSDN: SYSTEM_LOGICAL_PROCESSOR_INFORMATION, CACHE_DESCRIPTOR):
 * ULONG_PTR ProcessorMask at 0, the relationship (a 4-byte enum) at 8, the 16-byte union at 16; 32 bytes.
 * The records returned, in this order (the page lets the order change between calls; here it never does):
 *   0 RelationProcessorCore (0)    mask 1, Flags 0 (no shared functional units)
 *   1 RelationNumaNode (1)         mask 1, NodeNumber 0 (the page: a non-NUMA system reports one node)
 *   2 RelationCache (2)            L1, CacheData (2),        32 KiB,  8-way, 64-byte lines   (setting)
 *   3 RelationCache (2)            L1, CacheInstruction (1), 32 KiB,  8-way, 64-byte lines   (setting)
 *   4 RelationCache (2)            L2, CacheUnified (0),     256 KiB, 8-way, 64-byte lines   (setting)
 *   5 RelationCache (2)            L3, CacheUnified (0),     8 MiB,   16-way, 64-byte lines  (setting)
 *   6 RelationProcessorPackage (3) mask 1 */
#define FLE_GLPI_RECORD_SIZE  32u
#define FLE_GLPI_RECORD_COUNT 7u

/* kernel32!IsProcessorFeaturePresent: BOOL f(DWORD ecx) (decision 0078). Answers the PF_* numbers that the emulated processor
 * (CPUID, decisions 0009 and 0047) and the Learn page decide; any other number stops by name. NOT in the response table: the only
 * calls in dsp_ippv2_x64.dll are on its fatal-error paths (PF_FASTFAIL_AVAILABLE), and tests/sysinfo.rs asserts that GetProcAddress
 * does not find it (decision 0034). */
void fle_imp_IsProcessorFeaturePresent(void);

/* kernel32!GetNativeSystemInfo: VOID f(LPSYSTEM_INFO). rcx = the output. For a 64-bit application the page says
 * it is equivalent to GetSystemInfo, so it fills the structure through fle_imp_GetSystemInfo (fle_sys.c). */
void fle_imp_GetNativeSystemInfo(void);

/* kernel32!GetLogicalProcessorInformation: BOOL f(PSYSTEM_LOGICAL_PROCESSOR_INFORMATION Buffer, PDWORD ReturnedLength).
 * rcx = Buffer, rdx = ReturnedLength. */
void fle_imp_GetLogicalProcessorInformation(void);

/* kernel32!IsWow64Process: BOOL f(HANDLE hProcess, PBOOL Wow64Process). rcx, rdx. Only the GetCurrentProcess pseudo
 * handle is accepted; the answer is FALSE (a 64-bit process on 64-bit Windows, as the page states). */
void fle_imp_IsWow64Process(void);

/* kernel32!GlobalMemoryStatus: VOID f(LPMEMORYSTATUS). rcx = the output (56 bytes). */
void fle_imp_GlobalMemoryStatus(void);

/* kernel32!GetProcessAffinityMask: BOOL f(HANDLE, PDWORD_PTR process, PDWORD_PTR system). rcx, rdx, r8.
 * kernel32!SetProcessAffinityMask: BOOL f(HANDLE, DWORD_PTR mask). rcx, rdx. Only the pseudo handle is accepted. */
void fle_imp_GetProcessAffinityMask(void);
void fle_imp_SetProcessAffinityMask(void);

#endif

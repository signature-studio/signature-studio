/* System-information answers beyond GetSystemInfo (docs/decisions/0034-sysinfo.md). The numbers that no
 * Microsoft page fixes are settings of this virtual environment (fle_sysinfo.h). */
#define _GNU_SOURCE
#include <stdint.h>
#include <string.h>
#include <unistd.h>

#include "fle_runtime.h"
#include "fle_host.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_proc.h"
#include "fle_sys.h"
#include "fle_sysinfo.h"

/* Same rule as the other runtime files: the first caller owns the state, another thread stops the run. */
static long fle_sysinfo_owner_tid;

static void fle_sysinfo_check_thread(const char *api)
{
    long tid = fle_host_tid();

    if (fle_sysinfo_owner_tid == 0) fle_sysinfo_owner_tid = tid;
    if (tid != fle_sysinfo_owner_tid) fle_stop(api, "called from a second thread, not supported");
}

/* MSDN GetNativeSystemInfo: "If the function is called from a 64-bit application, it is equivalent to the
 * GetSystemInfo function." The structure is therefore the one GetSystemInfo writes. The page does not say what a
 * NULL pointer does: the run stops. */
void fle_imp_GetNativeSystemInfo(void)
{
    fle_sysinfo_check_thread("GetNativeSystemInfo");
    if (R[1] == 0) fle_stop("GetNativeSystemInfo", "lpSystemInfo is NULL (the page does not define it)");
    fle_imp_GetSystemInfo();
}

/* kernel32!IsProcessorFeaturePresent: BOOL f(DWORD ProcessorFeature ecx) (decision 0078; Learn page IsProcessorFeaturePresent).
 * The page: nonzero when the feature is supported, zero when it is not "or if the HAL does not support detection". It lists the
 * feature numbers but gives no value for any of them: every answer below follows from a choice that this environment already made,
 * and the numbers the choices do not decide stop the run by name.
 *  - decided by the emulated processor (runtime CPUID answer, decisions 0009, 0047 and 0118: leaf 1 EDX has FPU (bit 0), CX8 (8), CMOV (15), MMX (23), FXSR (24),
 *    SSE (25) and SSE2 (26) = 0x07808101; ECX is 0, so no SSE3, SSSE3, SSE4.1, SSE4.2, CMPXCHG16B, XSAVE or AVX; basic leaf 7 and the extended leaves are not
 *    reported, so no AVX2, AVX-512F, BMI2, RDFSBASE or 3DNow!; EDX bit 4 (TSC) is 0): 2 (PF_COMPARE_EXCHANGE_DOUBLE, CX8 = CMPXCHG8B), 3 (PF_MMX_INSTRUCTIONS_AVAILABLE),
 *    6 (SSE) and 10 (SSE2) are nonzero; 7, 8, 14, 13, 36, 37, 38, 39, 40, 41, 60, 17, 22 are 0. A feature that the CPUID answer does not report is not reported here
 *    either, and a feature the CPUID answer reports is reported here, so the two cannot disagree in either direction. (A real x64 Windows reports some of the 0 ones
 *    as present, for example 8, 13, 14 and 17 on a processor that has them; that is not modelled.)
 *    DECISION 0119 (PROVISIONAL, the PM's decision 2) changed PF 2 and PF 3 from 0 to 1: decision 0118 had made the CPUID answer report CX8 and MMX and had left
 *    this function as it was ("the rule above holds in one direction only"). The Learn table (retrieved 2026-10-30): "PF_COMPARE_EXCHANGE_DOUBLE 2: The atomic compare
 *    and exchange operation (cmpxchg) is available", "PF_MMX_INSTRUCTIONS_AVAILABLE 3: The MMX instruction set is available". The correspondence of PF 2 to CPUID
 *    EDX bit 8 (CMPXCHG8B) is a reading of the page's "atomic compare and exchange operation (cmpxchg)" (the page does not name the CPUID bit; PF_COMPARE_EXCHANGE128 (14)
 *    is the separate cmpxchg16b entry).
 *  - PF_PAE_ENABLED (9): the page says "All x64 processors always return a nonzero value for this feature".
 *  - PF_FASTFAIL_AVAILABLE (23): the page says "_fastfail() is available"; the __fastfail page says native fast fail support began
 *    in Windows 8, and the emulated OS is Windows 10 (decision 0014): nonzero (derived, not stated for this OS).
 * Every other value (the other x86 features, the Arm features, a number the page does not list) is not decided and stops. */
#define FLE_PF_FLOATING_POINT_PRECISION_ERRATA 0u
#define FLE_PF_FLOATING_POINT_EMULATED         1u
#define FLE_PF_COMPARE_EXCHANGE_DOUBLE         2u
#define FLE_PF_MMX_INSTRUCTIONS_AVAILABLE      3u
#define FLE_PF_XMMI_INSTRUCTIONS_AVAILABLE     6u
#define FLE_PF_3DNOW_INSTRUCTIONS_AVAILABLE    7u
#define FLE_PF_RDTSC_INSTRUCTION_AVAILABLE     8u
#define FLE_PF_PAE_ENABLED                     9u
#define FLE_PF_XMMI64_INSTRUCTIONS_AVAILABLE   10u
#define FLE_PF_SSE3_INSTRUCTIONS_AVAILABLE     13u
#define FLE_PF_COMPARE_EXCHANGE128             14u
#define FLE_PF_XSAVE_ENABLED                   17u
#define FLE_PF_RDWRFSGSBASE_AVAILABLE          22u
#define FLE_PF_FASTFAIL_AVAILABLE              23u
#define FLE_PF_SSSE3_INSTRUCTIONS_AVAILABLE    36u
#define FLE_PF_SSE4_1_INSTRUCTIONS_AVAILABLE   37u
#define FLE_PF_SSE4_2_INSTRUCTIONS_AVAILABLE   38u
#define FLE_PF_AVX_INSTRUCTIONS_AVAILABLE      39u
#define FLE_PF_AVX2_INSTRUCTIONS_AVAILABLE     40u
#define FLE_PF_AVX512F_INSTRUCTIONS_AVAILABLE  41u
#define FLE_PF_BMI2_INSTRUCTIONS_AVAILABLE     60u

void fle_imp_IsProcessorFeaturePresent(void)
{
    uint32_t f = (uint32_t)R[1];
    char why[120];

    fle_sysinfo_check_thread("IsProcessorFeaturePresent");
    switch (f) {
    case FLE_PF_COMPARE_EXCHANGE_DOUBLE:
    case FLE_PF_MMX_INSTRUCTIONS_AVAILABLE:
    case FLE_PF_XMMI_INSTRUCTIONS_AVAILABLE:
    case FLE_PF_XMMI64_INSTRUCTIONS_AVAILABLE:
    case FLE_PF_PAE_ENABLED:
    case FLE_PF_FASTFAIL_AVAILABLE:
        R[0] = 1;
        return;
    case FLE_PF_3DNOW_INSTRUCTIONS_AVAILABLE:
    case FLE_PF_RDTSC_INSTRUCTION_AVAILABLE:
    case FLE_PF_SSE3_INSTRUCTIONS_AVAILABLE:
    case FLE_PF_COMPARE_EXCHANGE128:
    case FLE_PF_XSAVE_ENABLED:
    case FLE_PF_RDWRFSGSBASE_AVAILABLE:
    case FLE_PF_SSSE3_INSTRUCTIONS_AVAILABLE:
    case FLE_PF_SSE4_1_INSTRUCTIONS_AVAILABLE:
    case FLE_PF_SSE4_2_INSTRUCTIONS_AVAILABLE:
    case FLE_PF_AVX_INSTRUCTIONS_AVAILABLE:
    case FLE_PF_AVX2_INSTRUCTIONS_AVAILABLE:
    case FLE_PF_AVX512F_INSTRUCTIONS_AVAILABLE:
    case FLE_PF_BMI2_INSTRUCTIONS_AVAILABLE:
        R[0] = 0;
        return;
    default:
        snprintf(why, sizeof why, "feature %u: the page gives no value and no decision of this runtime covers it", f);
        fle_stop("IsProcessorFeaturePresent", why);
    }
}

/* ---- GetLogicalProcessorInformation ---- */
static void fle_glpi_record(uint8_t *r, uint32_t relationship)
{
    uint64_t mask = FLE_SYSINFO_PROC_MASK;

    memset(r, 0, FLE_GLPI_RECORD_SIZE);
    memcpy(r + 0, &mask, 8);
    memcpy(r + 8, &relationship, 4);
}

static void fle_glpi_cache(uint8_t *r, uint8_t level, uint8_t assoc, uint32_t size, uint32_t type)
{
    uint16_t line = 64;

    fle_glpi_record(r, 2);
    r[16] = level;
    r[17] = assoc;
    memcpy(r + 18, &line, 2);
    memcpy(r + 20, &size, 4);
    memcpy(r + 24, &type, 4);
}

/* MSDN GetLogicalProcessorInformation (Parameters, Return value): the buffer receives an array of records.
 * If *ReturnedLength is large enough for all of them the call succeeds (TRUE) and *ReturnedLength is the number of
 * bytes returned. If it is too small the call fails (FALSE), GetLastError is ERROR_INSUFFICIENT_BUFFER (122) and
 * *ReturnedLength is the length required. Buffer may be NULL on the size-query call. The page names no behaviour
 * for a NULL ReturnedLength, nor for a NULL Buffer when the length would be enough: the run stops. A success does
 * not touch the last error (the page does not say it does). */
void fle_imp_GetLogicalProcessorInformation(void)
{
    uint64_t buf = R[1], plen = R[2];
    uint8_t rec[FLE_GLPI_RECORD_COUNT * FLE_GLPI_RECORD_SIZE];
    uint32_t total = FLE_GLPI_RECORD_COUNT * FLE_GLPI_RECORD_SIZE, have;
    uint32_t node = 0;

    fle_sysinfo_check_thread("GetLogicalProcessorInformation");
    if (plen == 0) fle_stop("GetLogicalProcessorInformation", "ReturnedLength is NULL (the page does not define it)");
    have = (uint32_t)fle_ld(plen, 4);
    if (have < total) {
        fle_st(plen, 4, total);
        fle_set_last_error(FLE_W32_ERR_INSUFFICIENT_BUFFER);
        R[0] = 0;
        return;
    }
    if (buf == 0) fle_stop("GetLogicalProcessorInformation", "Buffer is NULL but the length is enough (the page does not define it)");

    fle_glpi_record(rec + 0 * FLE_GLPI_RECORD_SIZE, 0);            /* RelationProcessorCore, Flags 0 */
    fle_glpi_record(rec + 1 * FLE_GLPI_RECORD_SIZE, 1);            /* RelationNumaNode */
    memcpy(rec + 1 * FLE_GLPI_RECORD_SIZE + 16, &node, 4);
    fle_glpi_cache(rec + 2 * FLE_GLPI_RECORD_SIZE, 1, 8, 32u << 10, 2);    /* L1 data */
    fle_glpi_cache(rec + 3 * FLE_GLPI_RECORD_SIZE, 1, 8, 32u << 10, 1);    /* L1 instruction */
    fle_glpi_cache(rec + 4 * FLE_GLPI_RECORD_SIZE, 2, 8, 256u << 10, 0);   /* L2 unified */
    fle_glpi_cache(rec + 5 * FLE_GLPI_RECORD_SIZE, 3, 16, 8u << 20, 0);    /* L3 unified */
    fle_glpi_record(rec + 6 * FLE_GLPI_RECORD_SIZE, 3);            /* RelationProcessorPackage */

    memcpy((void *)fle_addr(buf, (unsigned)total, 1), rec, total);
    fle_st(plen, 4, total);
    R[0] = 1;
}

/* MSDN IsWow64Process (Parameters, Return value): *Wow64Process is TRUE for a process under WOW64 and FALSE
 * "if the process is a 64-bit application running under 64-bit Windows"; the return value is nonzero on success. The
 * lifted DLL runs as a 64-bit process, so the answer is FALSE. The handle needs query rights; the only handle this
 * environment hands out for a process is the GetCurrentProcess pseudo handle, so any other handle, and a NULL
 * output pointer (not defined by the page), stop the run. */
void fle_imp_IsWow64Process(void)
{
    uint64_t h = R[1], out = R[2];

    fle_sysinfo_check_thread("IsWow64Process");
    if (h != FLE_PSEUDO_PROCESS_HANDLE) fle_stop("IsWow64Process", "a handle that GetCurrentProcess did not return");
    if (out == 0) fle_stop("IsWow64Process", "Wow64Process is NULL (the page does not define it)");
    fle_st(out, 4, 0);
    R[0] = 1;
}

/* MSDN GlobalMemoryStatus and MEMORYSTATUS: the function stores the values into the structure and sets dwLength
 * itself; dwMemoryLoad is 0..100. No return value. The values are the settings in fle_sysinfo.h. A NULL pointer is
 * not defined by the page: the run stops. */
void fle_imp_GlobalMemoryStatus(void)
{
    uint64_t out = R[1];
    uint8_t ms[FLE_MEMSTATUS_SIZE];
    uint32_t len = FLE_MEMSTATUS_SIZE, load = FLE_MEMSTATUS_LOAD;
    uint64_t v[6] = { FLE_MEMSTATUS_TOTAL_PHYS, FLE_MEMSTATUS_AVAIL_PHYS, FLE_MEMSTATUS_TOTAL_PAGEFILE,
                      FLE_MEMSTATUS_AVAIL_PAGEFILE, FLE_MEMSTATUS_TOTAL_VIRTUAL, FLE_MEMSTATUS_AVAIL_VIRTUAL };

    fle_sysinfo_check_thread("GlobalMemoryStatus");
    if (out == 0) fle_stop("GlobalMemoryStatus", "lpBuffer is NULL (the page does not define it)");
    memset(ms, 0, sizeof ms);
    memcpy(ms + 0, &len, 4);
    memcpy(ms + 4, &load, 4);
    memcpy(ms + 8, v, sizeof v);
    memcpy((void *)fle_addr(out, (unsigned)sizeof ms, 1), ms, sizeof ms);
}

/* ---- Process affinity ----
 * MSDN GetProcessAffinityMask: the process mask and the system mask are bit vectors; the process mask is a subset
 * of the system mask; nonzero on success. MSDN SetProcessAffinityMask: "If the process affinity mask requests a
 * processor that is not configured in the system, the last error code is ERROR_INVALID_PARAMETER" (87); the mask
 * must be a subset of the system mask; nonzero on success. The page does not say what an empty mask does, so a
 * zero mask stops the run. The system mask is FLE_SYSINFO_PROC_MASK; the process mask starts equal to it. */
static uint64_t fle_proc_affinity = FLE_SYSINFO_PROC_MASK;

void fle_imp_GetProcessAffinityMask(void)
{
    uint64_t h = R[1], pp = R[2], ps = R[8];

    fle_sysinfo_check_thread("GetProcessAffinityMask");
    if (h != FLE_PSEUDO_PROCESS_HANDLE) fle_stop("GetProcessAffinityMask", "a handle that GetCurrentProcess did not return");
    if (pp == 0 || ps == 0) fle_stop("GetProcessAffinityMask", "an output pointer is NULL (the page does not define it)");
    fle_st(pp, 8, fle_proc_affinity);
    fle_st(ps, 8, FLE_SYSINFO_PROC_MASK);
    R[0] = 1;
}

void fle_imp_SetProcessAffinityMask(void)
{
    uint64_t h = R[1], mask = R[2];

    fle_sysinfo_check_thread("SetProcessAffinityMask");
    if (h != FLE_PSEUDO_PROCESS_HANDLE) fle_stop("SetProcessAffinityMask", "a handle that GetCurrentProcess did not return");
    if (mask == 0) fle_stop("SetProcessAffinityMask", "an empty mask (the page does not define it)");
    if (mask & ~FLE_SYSINFO_PROC_MASK) {
        fle_set_last_error(FLE_W32_ERR_INVALID_PARAMETER);
        R[0] = 0;
        return;
    }
    fle_proc_affinity = mask;
    R[0] = 1;
}

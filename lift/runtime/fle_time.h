/* Emulated wall-clock time, time zone and the kernel32 time/date answers
 * (docs/decisions/0020-process-and-time.md; sources in docs/investigations/import-proctime-spec.md).
 * Same convention as fle_imports.h: arguments in R[1] (rcx), R[2] (rdx), R[8] (r8), R[9] (r9), stack
 * arguments through fle_arg (fle_env.h); the Win32 return value in R[0]; the guest return address stays on
 * the stack for the caller's ret. A function that returns VOID leaves R[0] alone.
 *
 * ONE time source. The virtual clock of fle_sys.c counts 100 ns ticks since a fictitious system start
 * (decision 0017). Here the instant of tick 0 is FLE_TIME_BOOT_FILETIME; the current UTC time is
 * FLE_TIME_BOOT_FILETIME + ticks. The host clock is never read. Every read of the current time goes
 * through the clock-read of fle_sys.c (fle_imp_QueryPerformanceCounter), so it advances the clock by
 * FLE_CLOCK_READ_TICKS exactly like GetTickCount does and a loop that polls the time ends.
 *
 * The boot instant and the time zone are SETTINGS OF THIS LIFT ENVIRONMENT, not values from any
 * specification. Single guest thread only. A case the pages leave open stops the run (fle_stop). */
#ifndef FLE_TIME_H
#define FLE_TIME_H

#include <stdint.h>

/* FILETIME (100 ns intervals since 1601-01-01T00:00:00Z) of 2020-12-31T23:00:00Z: the instant of clock tick 0.
 * The clock of fle_sys.c starts one hour after tick 0 (FLE_CLOCK_START_MS = 3,600,000 ms), so the first time
 * read is 2021-01-01T00:00:00Z, the start value of the virtual file clock (FLE_VFS_CLOCK_START, fle_vfs.h). */
#define FLE_TIME_BOOT_FILETIME UINT64_C(132539292000000000)

/* The emulated time zone: UTC. Bias is "the difference, in minutes, between UTC and local time"
 * (UTC = local time + bias), so 0 here and local time equals UTC. No daylight saving time. */
#define FLE_TZ_BIAS_MINUTES 0
#define FLE_TZ_STANDARD_NAME "UTC"
#define FLE_TZ_DAYLIGHT_NAME ""

#define FLE_TICKS_PER_SECOND   UINT64_C(10000000)
#define FLE_TICKS_PER_MINUTE   (FLE_TICKS_PER_SECOND * 60)
#define FLE_TIME_FILETIME_LIMIT UINT64_C(0x8000000000000000)

/* Sizes (MSDN: SYSTEMTIME, FILETIME, TIME_ZONE_INFORMATION; the offsets follow from the member types). */
#define FLE_SYSTEMTIME_SIZE 16u
#define FLE_TZI_SIZE        172u
#define FLE_TZI_STD_NAME    4u
#define FLE_TZI_STD_DATE    68u
#define FLE_TZI_STD_BIAS    84u
#define FLE_TZI_DLT_NAME    88u
#define FLE_TZI_DLT_DATE    152u
#define FLE_TZI_DLT_BIAS    168u

#define FLE_TIME_ZONE_ID_UNKNOWN 0u

/* System error codes used (MSDN: System Error Codes 0-499). */
#define FLE_ERR_INVALID_PARAMETER   87u
#define FLE_ERR_INSUFFICIENT_BUFFER 122u

/* SYSTEMTIME as eight 16-bit members in declaration order: year, month, day of week, day, hour, minute,
 * second, milliseconds. The pure conversions return 1 on success and 0 when the page says the call fails. */
int fle_time_filetime_to_systemtime(uint64_t ft, uint16_t st[8]);
int fle_time_systemtime_to_filetime(const uint16_t st[8], uint64_t *ft);

/* The current UTC time as a FILETIME. Advances the clock like a clock read. */
uint64_t fle_time_now_filetime(void);

/* kernel32!GetSystemTime, GetLocalTime: VOID f(LPSYSTEMTIME). rcx. GetSystemTimeAsFileTime: VOID f(LPFILETIME).
 * rcx. GetTickCount64: ULONGLONG f(void). */
void fle_imp_GetSystemTime(void);
void fle_imp_GetLocalTime(void);
void fle_imp_GetSystemTimeAsFileTime(void);
void fle_imp_GetTickCount64(void);

/* kernel32!GetTimeZoneInformation: DWORD f(LPTIME_ZONE_INFORMATION). rcx. */
void fle_imp_GetTimeZoneInformation(void);

/* kernel32 conversions: BOOL f(const FILETIME *, LPSYSTEMTIME) and so on. */
void fle_imp_FileTimeToSystemTime(void);
void fle_imp_SystemTimeToFileTime(void);
void fle_imp_FileTimeToLocalFileTime(void);
void fle_imp_LocalFileTimeToFileTime(void);
void fle_imp_SystemTimeToTzSpecificLocalTime(void);
void fle_imp_DosDateTimeToFileTime(void);
void fle_imp_FileTimeToDosDateTime(void);

/* kernel32!GetDateFormatW: int f(LCID, DWORD, const SYSTEMTIME *, LPCWSTR, LPWSTR, int). rcx, rdx, r8, r9,
 * then lpDateStr and cchDate on the stack. */
void fle_imp_GetDateFormatW(void);

#endif

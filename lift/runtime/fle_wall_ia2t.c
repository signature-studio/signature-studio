/* kernel32!GetTimeZoneInformationForYear (wall ia2-W21: the channel rack's settings icon; a DELAY-LOADED import that the virtual kernel32 did not answer: the delay-load helper raised ERROR_PROC_NOT_FOUND 0xC06D007F). Decision 0191, PROVISIONAL.
 * BOOL f(USHORT wYear ecx, PDYNAMIC_TIME_ZONE_INFORMATION pdtzi rdx, LPTIME_ZONE_INFORMATION ptzi r8). Learn (timezoneapi.h, "GetTimeZoneInformationForYear function"): "Retrieves the time zone settings for the specified year and
 * time zone"; pdtzi "If this parameter is NULL, the current time zone is used"; the return value is nonzero on success, 0 with GetLastError otherwise. The virtual machine has ONE time zone with no daylight saving transition
 * (fle_time.c: GetTimeZoneInformation), so the settings of every year are the current zone's: ptzi gets what GetTimeZoneInformation gives. A non-NULL pdtzi (a named zone from EnumDynamicTimeZoneInformation) stops by name:
 * the virtual machine has no other zone. A NULL ptzi stops (the page gives no behaviour). */
#include <stdint.h>
#include <string.h>

#include "fle_runtime.h"
#include "fle_imports.h"
#include "fle_env.h"
#include "fle_time.h"
#include "fle_wall_ia2.h"

void fle_imp_GetTimeZoneInformationForYear(void)
{
    uint64_t pdtzi = R[2], ptzi = R[8], saved[16];

    if (pdtzi != 0) fle_stop("GetTimeZoneInformationForYear", "pdtzi is not NULL: a named time zone is not modelled (the virtual machine has one zone)");
    if (ptzi == 0) fle_stop("GetTimeZoneInformationForYear", "ptzi is NULL: the page gives no behaviour");
    memcpy(saved, R, sizeof saved);
    R[1] = ptzi;
    fle_imp_GetTimeZoneInformation();
    memcpy(R, saved, sizeof saved);
    R[0] = 1;
}

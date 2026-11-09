/* The virtual display monitor (docs/decisions/0065-virtual-monitor.md). Stateless. EnumDisplayMonitors(NULL, NULL, proc, data) and
 * GetMonitorInfoW(the virtual monitor, MONITORINFO) are answered; every other form stops the run through fle_stop (fle_env.h).
 * Nothing here reads the host: no display, no monitor, no EDID, no display server, no registry, no clock, no thread. */
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>

#include "fle_runtime.h"
#include "fle_env.h"
#include "fle_callback.h"
#include "fle_gdc.h"
#include "fle_sysmetrics.h"
#include "fle_monitor.h"

/* decision 0212: FLE_MONITOR_RIGHT / FLE_MONITOR_BOTTOM and FLE_GDC_HORZRES / FLE_GDC_VERTRES are run-time values of one function (fle_screen.h), so they cannot differ; the defaults are checked in the headers */
_Static_assert(FLE_MONITOR_LEFT == FLE_SM_VIRTUALSCREEN_ORIGIN && FLE_MONITOR_TOP == FLE_SM_VIRTUALSCREEN_ORIGIN,
               "the monitor holds the origin of the virtual screen (decision 0057)");
_Static_assert(FLE_MONITOR_HANDLE < UINT64_C(0x80000000), "the handle region is below 2 GiB");
_Static_assert(FLE_MONITOR_HANDLE_BASE >= UINT64_C(0x7FEA0000) && FLE_MONITOR_HANDLE_BASE + FLE_MONITOR_REGION_SIZE <= UINT64_C(0x80000000),
               "the region lies after the cursor regions and below 2 GiB");

/* RECT: LONG left, top, right, bottom (16 bytes). */
static void write_rect(uint64_t at, int32_t l, int32_t t, int32_t r, int32_t b)
{
    fle_st(at, 4, (uint32_t)l);
    fle_st(at + 4, 4, (uint32_t)t);
    fle_st(at + 8, 4, (uint32_t)r);
    fle_st(at + 12, 4, (uint32_t)b);
}

/* user32!EnumDisplayMonitors: BOOL f(HDC hdc rcx, LPCRECT lprcClip rdx, MONITORENUMPROC lpfnEnum r8, LPARAM dwData r9). Learn
 * (EnumDisplayMonitors): with hdc NULL and lprcClip NULL it "Enumerates all display monitors. The callback function receives a
 * NULL HDC"; the region of interest is the virtual screen. Learn (MONITORENUMPROC): the callback is called once for each monitor,
 * as BOOL f(HMONITOR hMonitor, HDC hdcMonitor, LPRECT lprcMonitor, LPARAM dwData): hMonitor "will always be non-NULL", hdcMonitor
 * is NULL when hdc was NULL, lprcMonitor is then "the display monitor rectangle ... virtual-screen coordinates", dwData is passed
 * through unchanged; "To continue the enumeration, return TRUE. To stop the enumeration, return FALSE."
 * One monitor, so one call. The RECT that lprcMonitor points at lives on the guest stack below the caller's frame (16 bytes,
 * 16-byte aligned), written before the call, valid for the duration of the callback only (as a real RECT would be).
 * Return value: Learn says nonzero on success and zero on failure and does not say what a stop by the callback returns. The result
 * here is the callback's own BOOL (1 for TRUE, 0 for FALSE): a PROVISIONAL choice (decision 0065; the guest ignores the value).
 * The callback's result is read as `al` (the PM's rule, decision 0065): Learn types it BOOL (32 bits) and says TRUE / FALSE only; the
 * guest's callback (0x85df9f: `mov al, 1`) writes al alone, so the rest of eax is whatever the callee left. For the values a
 * callback that returns TRUE / FALSE produces (0 or 1) al and eax agree; a BOOL that is nonzero with al == 0 (0x100) counts as FALSE
 * here and as TRUE for a Windows that tests eax (not met by the guest). The last error is left alone (the page names none).
 * Stops: hdc not NULL (needs the DC of a monitor), lprcClip not NULL (clipping), lpfnEnum NULL. */
void fle_imp_EnumDisplayMonitors(void)
{
    static const char api[] = "EnumDisplayMonitors";
    uint64_t hdc = R[1], clip = R[2], proc = R[8], data = R[9];
    uint64_t sp = R[4];
    uint64_t rect = (sp - 16) & ~UINT64_C(15);
    uint64_t cb;

    if (hdc != 0) fle_stop(api, "hdc is not NULL: the DC of a monitor is not implemented, only (NULL, NULL) is answered (decision 0065)");
    if (clip != 0) fle_stop(api, "lprcClip is not NULL: clipping is not implemented, only (NULL, NULL) is answered (decision 0065)");
    if (proc == 0) fle_stop(api, "lpfnEnum is NULL: the page does not define it (decision 0065)");

    write_rect(rect, FLE_MONITOR_LEFT, FLE_MONITOR_TOP, FLE_MONITOR_RIGHT, FLE_MONITOR_BOTTOM);
    R[4] = rect;
    cb = fle_call_guest(proc, FLE_MONITOR_HANDLE, 0, rect, data);
    R[4] = sp;
    R[0] = (cb & 0xFFu) != 0 ? 1 : 0;
}


/* user32!MonitorFromWindow, MonitorFromPoint, MonitorFromRect (decision 0126, PROVISIONAL, walls 100). The environment has ONE monitor (decision 0065), the rectangle (0, 0, 1920, 1080).
 * Learn MonitorFromWindow: "retrieves a handle to the display monitor that has the largest area of intersection with the bounding rectangle of a specified window"; "If the window intersects one or more display monitor rectangles,
 * the return value is an HMONITOR handle to the display monitor that has the largest area of intersection with the window. If the window does not intersect a display monitor, the return value depends on the value of dwFlags":
 * MONITOR_DEFAULTTONULL 0 "Returns NULL", MONITOR_DEFAULTTOPRIMARY 1 "Returns a handle to the primary display monitor", MONITOR_DEFAULTTONEAREST 2 "Returns a handle to the display monitor that is nearest" (the numbers are printed on the
 * MonitorFromPoint page); "If the window is currently minimized, MonitorFromWindow uses the rectangle of the window before it was minimized". With one monitor the primary and the nearest monitor are that monitor. A dwFlags that is not
 * 0, 1 or 2 stops (the pages list no other value). A rectangle intersects the monitor when the area of the intersection is not zero (S-monitor-area: the page says "intersects"; an empty rectangle has no area and so does not intersect:
 * a reading). MonitorFromPoint: the point is contained in the monitor when left <= x < right and top <= y < bottom (S-monitor-point-halfopen: the page says "contains" and no edge rule; the half-open rule is that of PtInRect and of
 * RECT in general), POINT by value in rcx (x low dword, y high dword). MonitorFromRect: lprc NULL stops (not described). A window handle that is not a live window of the table gives the no-intersection answer by dwFlags (S-monitor-invalid-window:
 * the page says nothing about an invalid window; the last error is left alone). */
#define FLE_MON_DEFAULTTONULL     0u
#define FLE_MON_DEFAULTTOPRIMARY  1u
#define FLE_MON_DEFAULTTONEAREST  2u

uint64_t fle_monitor_resolve(const char *api, int intersects, uint32_t flags)
{
    char why[160];

    if (flags != FLE_MON_DEFAULTTONULL && flags != FLE_MON_DEFAULTTOPRIMARY && flags != FLE_MON_DEFAULTTONEAREST) {
        snprintf(why, sizeof why, "dwFlags %#x is not MONITOR_DEFAULTTONULL, MONITOR_DEFAULTTOPRIMARY or MONITOR_DEFAULTTONEAREST: the pages list no other value (decision 0126)", (unsigned)flags);
        fle_stop(api, why);
    }
    if (intersects) return FLE_MONITOR_HANDLE;
    return flags == FLE_MON_DEFAULTTONULL ? 0u : FLE_MONITOR_HANDLE;
}

int fle_monitor_rect_intersects(int32_t l, int32_t t, int32_t r, int32_t b)
{
    int64_t il = l > FLE_MONITOR_LEFT ? l : FLE_MONITOR_LEFT, it = t > FLE_MONITOR_TOP ? t : FLE_MONITOR_TOP;
    int64_t ir = r < FLE_MONITOR_RIGHT ? r : FLE_MONITOR_RIGHT, ib = b < FLE_MONITOR_BOTTOM ? b : FLE_MONITOR_BOTTOM;

    return r > l && b > t && ir > il && ib > it;
}

void fle_imp_MonitorFromPoint(void)
{
    int32_t x = (int32_t)(uint32_t)R[1], y = (int32_t)(uint32_t)(R[1] >> 32);
    int in = x >= FLE_MONITOR_LEFT && x < FLE_MONITOR_RIGHT && y >= FLE_MONITOR_TOP && y < FLE_MONITOR_BOTTOM;

    R[0] = fle_monitor_resolve("MonitorFromPoint", in, (uint32_t)R[2]);
}

void fle_imp_MonitorFromRect(void)
{
    static const char api[] = "MonitorFromRect";
    uint64_t p = R[1];
    int32_t l, t, r, b;

    if (p == 0) fle_stop(api, "lprc is NULL: the page does not define it");
    l = (int32_t)(uint32_t)fle_ld(p, 4);
    t = (int32_t)(uint32_t)fle_ld(p + 4, 4);
    r = (int32_t)(uint32_t)fle_ld(p + 8, 4);
    b = (int32_t)(uint32_t)fle_ld(p + 12, 4);
    R[0] = fle_monitor_resolve(api, fle_monitor_rect_intersects(l, t, r, b), (uint32_t)R[2]);
}

/* user32!GetMonitorInfoW: BOOL f(HMONITOR hMonitor rcx, LPMONITORINFO lpmi rdx). Learn (GetMonitorInfoW): "You must set the cbSize
 * member of the structure to sizeof(MONITORINFO) or sizeof(MONITORINFOEX) before calling"; nonzero on success, zero on failure.
 * Learn (MONITORINFO): { DWORD cbSize; RECT rcMonitor ("expressed in virtual-screen coordinates"); RECT rcWork ("the work area
 * rectangle of the display monitor"); DWORD dwFlags } and the one flag MONITORINFOF_PRIMARY, "This is the primary display monitor".
 * Answered: hMonitor the virtual monitor, lpmi with cbSize 0x28: rcMonitor = the virtual display, rcWork = rcMonitor (decision
 * 0065, provisional), dwFlags = MONITORINFOF_PRIMARY. Exactly the 40 bytes of the structure are written. Only the low DWORD of
 * cbSize is read. The last error is left alone (the page names none).
 * Stops: any other hMonitor (NULL included: the page gives no failure value for a bad handle), lpmi NULL, cbSize 0x48
 * (MONITORINFOEX: the device name is not decided) or any other cbSize. */
void fle_imp_GetMonitorInfoW(void)
{
    static const char api[] = "GetMonitorInfoW";
    uint64_t h = R[1], mi = R[2];
    uint32_t cb;
    char why[200];

    if (h != FLE_MONITOR_HANDLE) {
        snprintf(why, sizeof why, "hMonitor 0x%" PRIx64 " is not the virtual monitor 0x%" PRIx64 ": the page gives no failure value for a bad handle (decision 0065)",
                 h, (uint64_t)FLE_MONITOR_HANDLE);
        fle_stop(api, why);
    }
    if (mi == 0) fle_stop(api, "lpmi is NULL: the page does not define it (decision 0065)");
    cb = (uint32_t)fle_ld(mi, 4);
    if (cb == FLE_MONITORINFOEXW_SIZE) fle_stop(api, "cbSize is sizeof(MONITORINFOEXW) 0x68: the device name is not decided, only MONITORINFO (0x28) is answered (decision 0065)");
    if (cb == FLE_MONITORINFOEXA_SIZE) fle_stop(api, "cbSize is 0x48 = sizeof(MONITORINFOEXA), not a form of the Unicode function: only MONITORINFO (0x28) is answered (decision 0065)");
    if (cb != FLE_MONITORINFO_SIZE) {
        snprintf(why, sizeof why, "cbSize 0x%x is not sizeof(MONITORINFO) 0x28: the page gives no failure value for it (decision 0065)", (unsigned)cb);
        fle_stop(api, why);
    }
    write_rect(mi + FLE_MONITORINFO_OFF_MONITOR, FLE_MONITOR_LEFT, FLE_MONITOR_TOP, FLE_MONITOR_RIGHT, FLE_MONITOR_BOTTOM);
    write_rect(mi + FLE_MONITORINFO_OFF_WORK, FLE_MONITOR_WORK_LEFT, FLE_MONITOR_WORK_TOP, FLE_MONITOR_WORK_RIGHT, FLE_MONITOR_WORK_BOTTOM);
    fle_st(mi + FLE_MONITORINFO_OFF_FLAGS, 4, FLE_MONITORINFOF_PRIMARY);
    R[0] = 1;
}

/* ui1 (decision 0166): the reported refresh rate comes from fle_thread.c (FLE_DWM_HZ / FLE_DISPLAYSETTINGS_FREQUENCY in the environment, default FLE_DISPLAYSETTINGS_FREQUENCY); weak: a program that links no fle_thread.c reports the default. */
uint32_t fle_display_settings_hz(void) __attribute__((weak));

/* user32!EnumDisplaySettingsW (decision 0098, PROVISIONAL). Learn (EnumDisplaySettingsW): lpszDeviceName "is either NULL or a
 * DISPLAY_DEVICE.DeviceName ... A NULL value specifies the current display device on the computer on which the calling thread is running";
 * iModeNum is a graphics mode index or ENUM_CURRENT_SETTINGS ("Retrieve the current settings for the display device") or
 * ENUM_REGISTRY_SETTINGS; "Before calling EnumDisplaySettings, set the dmSize member to sizeof(DEVMODE)"; "The EnumDisplaySettings function
 * sets values for the following five DEVMODE members: dmBitsPerPel, dmPelsWidth, dmPelsHeight, dmDisplayFlags, dmDisplayFrequency"; the
 * return value is nonzero on success and zero on failure ("The function fails if iModeNum is greater than the index of the display device's
 * last graphics mode"). The last error is left alone (the page names none).
 * Answered: (NULL, ENUM_CURRENT_SETTINGS, DEVMODEW with dmSize 0xDC): the whole 0xDC bytes are written as 0 except dmSize and dmDriverExtra (the
 * caller's inputs, kept), dmFields (the five DM_ bits), dmBitsPerPel 32 (FLE_GDC_BITSPIXEL), dmPelsWidth, dmPelsHeight (the virtual display),
 * dmDisplayFlags 0 (progressive colour: neither flag), dmDisplayFrequency 60. dmDeviceName, dmPosition and the rest are not decided and stay 0
 * (the guest reads dmDisplayFrequency only). Returns 1.
 * Stops: a device name, any iModeNum other than ENUM_CURRENT_SETTINGS (ENUM_REGISTRY_SETTINGS and the mode indexes included: the guest does not
 * use them), lpDevMode NULL, dmSize other than 0xDC (the page says to set it and defines no failure for a different value). */
void fle_imp_EnumDisplaySettingsW(void)
{
    static const char api[] = "EnumDisplaySettingsW";
    uint64_t name = R[1], dm = R[8];
    uint32_t mode = (uint32_t)R[2];
    uint32_t i, size, extra;
    char why[200];

    if (name != 0) fle_stop(api, "lpszDeviceName is not NULL: only the current display device is answered (decision 0098)");
    if (mode != FLE_ENUM_CURRENT_SETTINGS) {
        snprintf(why, sizeof why, "iModeNum 0x%x is not ENUM_CURRENT_SETTINGS: no other mode is answered (decision 0098)", (unsigned)mode);
        fle_stop(api, why);
    }
    if (dm == 0) fle_stop(api, "lpDevMode is NULL: the page does not define it (decision 0098)");
    size = (uint32_t)fle_ld(dm + FLE_DEVMODEW_OFF_SIZE, 2);
    if (size != FLE_DEVMODEW_SIZE) {
        snprintf(why, sizeof why, "dmSize 0x%x is not sizeof(DEVMODEW) 0xdc: the page defines no failure value for it (decision 0098)", (unsigned)size);
        fle_stop(api, why);
    }
    extra = (uint32_t)fle_ld(dm + FLE_DEVMODEW_OFF_DRIVEREXTRA, 2);
    for (i = 0; i < FLE_DEVMODEW_SIZE; i += 4) fle_st(dm + i, 4, 0);
    fle_st(dm + FLE_DEVMODEW_OFF_SIZE, 2, size);
    fle_st(dm + FLE_DEVMODEW_OFF_DRIVEREXTRA, 2, extra);
    fle_st(dm + FLE_DEVMODEW_OFF_FIELDS, 4, FLE_DM_BITSPERPEL | FLE_DM_PELSWIDTH | FLE_DM_PELSHEIGHT | FLE_DM_DISPLAYFLAGS | FLE_DM_DISPLAYFREQUENCY);
    fle_st(dm + FLE_DEVMODEW_OFF_BITSPERPEL, 4, FLE_GDC_BITSPIXEL);
    fle_st(dm + FLE_DEVMODEW_OFF_PELSWIDTH, 4, FLE_GDC_HORZRES);
    fle_st(dm + FLE_DEVMODEW_OFF_PELSHEIGHT, 4, FLE_GDC_VERTRES);
    fle_st(dm + FLE_DEVMODEW_OFF_DISPLAYFLAGS, 4, 0);
    fle_st(dm + FLE_DEVMODEW_OFF_FREQUENCY, 4, fle_display_settings_hz ? fle_display_settings_hz() : FLE_DISPLAYSETTINGS_FREQUENCY);
    R[0] = 1;
}

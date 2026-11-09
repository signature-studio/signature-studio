/* The virtual display monitor: user32!EnumDisplayMonitors and user32!GetMonitorInfoW (docs/decisions/0065-virtual-monitor.md;
 * the pages read and the source of every number are listed there and in docs/investigations/callback-caller.md). Same convention
 * as fle_imports.h: arguments in R[1] (rcx), R[2] (rdx), R[8] (r8), R[9] (r9); the Win32 return value in R[0]; the guest return
 * address stays on the stack for the caller's ret. Single guest thread only (decision 0017). EnumDisplayMonitors calls the guest's
 * MONITORENUMPROC through fle_call_guest (fle_callback.h, decision 0064).
 *
 * Nothing is read from the host: no display, no monitor, no EDID, no DPI setting, no X11 or Wayland. The one monitor is a value of
 * this lift environment, fixed, so that a run does not depend on the machine it runs on. Its rectangle is the virtual display of
 * decision 0053 (fle_gdc.h) with the origin of decision 0057 (fle_sysmetrics.h), by name, so the three cannot drift apart.
 *
 * Only the forms the guest uses are answered: EnumDisplayMonitors(NULL, NULL, proc, data), GetMonitorInfoW(the virtual monitor,
 * MONITORINFO with cbSize 0x28). Everything else stops the run through fle_stop (fle_env.h): "fle: <api>: <why>" and abort(). */
#ifndef FLE_MONITOR_H
#define FLE_MONITOR_H

#include <stdint.h>

/* The handle of the one virtual monitor (decision 0065, a PROVISIONAL choice): FLE_MONITOR_HANDLE_BASE + 4. No page gives a value
 * for an HMONITOR. The region (1 MiB) lies directly after the one of the cursors loaded from the DLL (fle_loadimage.h: 0x7FDA0000 +
 * 1 MiB = up to 0x7FEA0000) and does not overlap the regions of the virtual GDI objects (fle_gdi.h: 0x7FBA0000), the standard
 * cursors (fle_cursor.h: 0x7FCA0000) or the screen DCs (fle_gdc.h: 0x7FA90000). Below 0x80000000 like those, so that truncation to
 * 32 bits and sign extension give the same 64-bit value (Learn "Interprocess communication" lists windows and GDI objects, not
 * monitors: the same rule is applied by choice). Never 0 (MONITORENUMPROC: "This value will always be non-NULL"). */
#define FLE_MONITOR_HANDLE_BASE UINT64_C(0x7FEA0000)
#define FLE_MONITOR_REGION_SIZE UINT64_C(0x00100000)
#define FLE_MONITOR_HANDLE      (FLE_MONITOR_HANDLE_BASE + 4)

/* The monitor rectangle, in virtual-screen coordinates (decision 0065). Learn (The Virtual Screen): the primary monitor contains
 * the origin (0,0). The size is the one of the virtual display (decision 0053: FLE_GDC_HORZRES x FLE_GDC_VERTRES, a choice). */
#define FLE_MONITOR_LEFT   0
#define FLE_MONITOR_TOP    0
#define FLE_MONITOR_RIGHT  1920   /* = FLE_GDC_HORZRES, checked by a static assertion in fle_monitor.c */
#define FLE_MONITOR_BOTTOM 1080   /* = FLE_GDC_VERTRES */
/* Decision 0212 (screen1, PROVISIONAL, opt-in): the right and bottom edges above are the DEFAULTS; fle_screen.h makes them the values of the run (FLE_MONITOR_W / FLE_MONITOR_H in the engine's environment;
 * unset: 1920 x 1080, so nothing differs). They stay equal to FLE_GDC_HORZRES / FLE_GDC_VERTRES because both are the same function. */
#include "fle_screen.h"
_Static_assert(FLE_MONITOR_RIGHT == FLE_SCREEN_DEFAULT_W && FLE_MONITOR_BOTTOM == FLE_SCREEN_DEFAULT_H, "the defaults of fle_screen.h are the constants above");
#undef FLE_MONITOR_RIGHT
#undef FLE_MONITOR_BOTTOM
#define FLE_MONITOR_RIGHT  FLE_SCREEN_PX_W
#define FLE_MONITOR_BOTTOM FLE_SCREEN_PX_H

/* The work area (decision 0065, a PROVISIONAL choice): the whole monitor, because this environment has no task bar. Learn gives
 * the member and no value. SPI_GETWORKAREA (decision 0058) is the same quantity and is not answered yet: see decision 0065 for the
 * condition under which it must return exactly this rectangle. */
#define FLE_MONITOR_WORK_LEFT   FLE_MONITOR_LEFT
#define FLE_MONITOR_WORK_TOP    FLE_MONITOR_TOP
#define FLE_MONITOR_WORK_RIGHT  FLE_MONITOR_RIGHT
#define FLE_MONITOR_WORK_BOTTOM FLE_MONITOR_BOTTOM

/* MONITORINFO (Learn, MONITORINFO): cbSize, rcMonitor, rcWork, dwFlags. The flag value is the Windows SDK header's
 * (mingw-w64 winuser.h: 0x00000001); the Learn page names the flag and does not print its value. */
#define FLE_MONITORINFO_SIZE          0x28   /* sizeof(MONITORINFO): 4 + 16 + 16 + 4 */
#define FLE_MONITORINFOEXA_SIZE       0x48   /* sizeof(MONITORINFOEXA): + CHAR szDevice[CCHDEVICENAME = 32] (not answered) */
#define FLE_MONITORINFOEXW_SIZE       0x68   /* sizeof(MONITORINFOEXW): + WCHAR szDevice[32] = 64 bytes (not answered) */
#define FLE_MONITORINFOF_PRIMARY      0x00000001u
#define FLE_MONITORINFO_OFF_MONITOR   4
#define FLE_MONITORINFO_OFF_WORK      20
#define FLE_MONITORINFO_OFF_FLAGS     36

/* user32!EnumDisplayMonitors: BOOL f(HDC hdc rcx, LPCRECT lprcClip rdx, MONITORENUMPROC lpfnEnum r8, LPARAM dwData r9).
 * user32!GetMonitorInfoW: BOOL f(HMONITOR hMonitor rcx, LPMONITORINFO lpmi rdx). */
void fle_imp_EnumDisplayMonitors(void);
void fle_imp_GetMonitorInfoW(void);

/* Decision 0126 (wall 100, PROVISIONAL; the notes are in fle_monitor.c): user32!MonitorFromPoint: HMONITOR f(POINT pt rcx, DWORD dwFlags edx), user32!MonitorFromRect: HMONITOR f(LPCRECT lprc rcx, DWORD dwFlags edx).
 * user32!MonitorFromWindow (fle_win.c): HMONITOR f(HWND, DWORD). fle_monitor_resolve: the handle (or NULL) for "the shape intersects the monitor" and dwFlags; fle_monitor_rect_intersects: the rectangle
 * has a non-zero area of intersection with the monitor. */
void fle_imp_MonitorFromPoint(void);
void fle_imp_MonitorFromRect(void);
void fle_imp_MonitorFromWindow(void);
uint64_t fle_monitor_resolve(const char *api, int intersects, uint32_t flags);
int fle_monitor_rect_intersects(int32_t l, int32_t t, int32_t r, int32_t b);

/* user32!EnumDisplaySettingsW (decision 0098, PROVISIONAL): BOOL f(LPCWSTR lpszDeviceName rcx, DWORD iModeNum edx, DEVMODEW *lpDevMode r8).
 * Only (NULL, ENUM_CURRENT_SETTINGS, a DEVMODEW whose dmSize is 0xDC) is answered: the current mode of the one virtual display, 32 bits per
 * pixel, FLE_GDC_HORZRES x FLE_GDC_VERTRES (decision 0053), FLE_DISPLAYSETTINGS_FREQUENCY hertz; the five members the page lists are
 * set and the rest of the structure is written as 0. Every other form stops by name.
 * The numbers: ENUM_CURRENT_SETTINGS ((DWORD)-1), the DM_* bits and the DEVMODEW offsets are from the Windows SDK headers as mirrored by
 * mingw-w64 (winuser.h, wingdi.h) and from the member order and types of the Learn page (the sums are ours); Learn names them and does not
 * print the values. */
#define FLE_ENUM_CURRENT_SETTINGS     0xFFFFFFFFu
#define FLE_DEVMODEW_SIZE             0xDC
#define FLE_DEVMODEW_OFF_SIZE         68     /* WCHAR dmDeviceName[32] (64) + dmSpecVersion, dmDriverVersion (2 + 2) */
#define FLE_DEVMODEW_OFF_DRIVEREXTRA  70
#define FLE_DEVMODEW_OFF_FIELDS       72
#define FLE_DEVMODEW_OFF_BITSPERPEL   168
#define FLE_DEVMODEW_OFF_PELSWIDTH    172
#define FLE_DEVMODEW_OFF_PELSHEIGHT   176
#define FLE_DEVMODEW_OFF_DISPLAYFLAGS 180
#define FLE_DEVMODEW_OFF_FREQUENCY    184
#define FLE_DM_BITSPERPEL             0x00040000u
#define FLE_DM_PELSWIDTH              0x00080000u
#define FLE_DM_PELSHEIGHT             0x00100000u
#define FLE_DM_DISPLAYFLAGS           0x00200000u
#define FLE_DM_DISPLAYFREQUENCY       0x00400000u
/* The refresh rate of the virtual display: a CHOICE of this environment (no page gives one). 60 Hz is also the rate the guest assumes when
 * the call fails (f_666d50 returns -60 then). */
/* ui1 (decision 0166, PROVISIONAL): FLE_DISPLAYSETTINGS_FREQUENCY is the DEFAULT (60); the number EnumDisplaySettingsW reports is fle_display_settings_hz() of fle_thread.c:
 * the environment variable FLE_DISPLAYSETTINGS_FREQUENCY if set (24..1000), else the same value as DwmFlush's FLE_DWM_HZ (fle_display_hz()), else this default.
 * If both variables are set and differ, a line goes to stderr: the guest reads the refresh rate from this call and is paced by DwmFlush, and nothing says what it does when the two disagree. */
#define FLE_DISPLAYSETTINGS_FREQUENCY 60
void fle_imp_EnumDisplaySettingsW(void);

#endif
